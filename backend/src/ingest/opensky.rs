use chrono::Utc;
use redis::AsyncCommands;
use serde::Deserialize;
use std::sync::Arc;
use tokio::time::{interval, Duration};

use crate::geo::geofence::ZoneHit;
use crate::models::anomaly::{Anomaly, AnomalyEntity, AnomalyReason, AnomalySeverity, EntityKind};
use crate::models::Flight;
use crate::state::AppState;
use crate::ws::WsMessage;

pub fn spawn(state: Arc<AppState>) {
    tokio::spawn(async move {
        run(state).await;
    });
}

async fn run(state: Arc<AppState>) {
    let cfg = &state.config;
    let poll_secs = cfg.quiet_poll_seconds.max(10); // OpenSky minimum 10s anonymous
    let mut ticker = interval(Duration::from_secs(poll_secs));

    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let client = &state.http;
    let mut token: Option<(String, std::time::Instant)> = None;

    loop {
        ticker.tick().await;

        let bbox = &cfg.opensky_bbox;
        let url = format!(
            "https://opensky-network.org/api/states/all?lamin={}&lamax={}&lomin={}&lomax={}",
            bbox.lamin, bbox.lamax, bbox.lomin, bbox.lomax
        );

        let mut req = client.get(&url);
        if let (Some(id), Some(secret)) = (&cfg.opensky_client_id, &cfg.opensky_client_secret) {
            if token
                .as_ref()
                .is_none_or(|(_, expiry)| std::time::Instant::now() >= *expiry)
            {
                let response = client.post("https://auth.opensky-network.org/auth/realms/opensky-network/protocol/openid-connect/token").form(&[("grant_type", "client_credentials"), ("client_id", id.as_str()), ("client_secret", secret.as_str())]).send().await;
                if let Ok(resp) = response {
                    if resp.status().is_success() {
                        if let Ok(data) = resp.json::<serde_json::Value>().await {
                            if let Some(value) = data["access_token"].as_str() {
                                token = Some((
                                    value.into(),
                                    std::time::Instant::now()
                                        + Duration::from_secs(
                                            data["expires_in"]
                                                .as_u64()
                                                .unwrap_or(300)
                                                .saturating_sub(30),
                                        ),
                                ));
                            }
                        }
                    }
                }
            }
            if let Some((value, _)) = &token {
                req = req.bearer_auth(value);
            } else {
                state
                    .provider_result("opensky", false, "OAuth token request failed")
                    .await;
                continue;
            }
        }

        match req.send().await {
            Ok(resp) if resp.status().is_success() => match resp.json::<OpenSkyResponse>().await {
                Ok(data) => {
                    let flights = parse_flights(data);
                    state
                        .provider_result(
                            "opensky",
                            true,
                            &format!("{} positions received", flights.len()),
                        )
                        .await;
                    tracing::debug!("OpenSky: {} flights", flights.len());
                    for flight in flights {
                        store_and_broadcast(&state, flight).await;
                    }
                }
                Err(e) => tracing::warn!("OpenSky parse error: {}", e),
            },
            Ok(resp) => {
                state
                    .provider_result("opensky", false, &format!("HTTP {}", resp.status()))
                    .await;
                if resp.status().as_u16() == 429 {
                    tokio::time::sleep(Duration::from_secs(300)).await;
                }
            }
            Err(e) => {
                let detail = if e.is_timeout() {
                    "Request timed out"
                } else if e.is_connect() {
                    "Connection failed (DNS, TCP or TLS); see server logs"
                } else {
                    "Request transport failed; see server logs"
                };
                tracing::warn!("OpenSky request failed: {:?}", e.without_url());
                state.provider_result("opensky", false, detail).await;
            }
        }
    }
}

async fn store_and_broadcast(state: &Arc<AppState>, flight: Flight) {
    let key = format!("entity:flight:{}", flight.icao24);
    let json = match serde_json::to_string(&flight) {
        Ok(j) => j,
        Err(_) => return,
    };

    let mut redis = state.redis();
    let _: Result<(), _> = redis.set_ex(&key, &json, 120).await;

    // Geofence check
    let hits = state.geofence.check(flight.lat, flight.lon);
    if !hits.is_empty() {
        check_anomaly(
            state,
            &flight.icao24,
            flight.callsign.as_deref(),
            flight.lat,
            flight.lon,
            EntityKind::Flight,
            &hits,
        )
        .await;
    }

    let _ = state.ws_tx.send(WsMessage::FlightUpdate(flight));
}

async fn check_anomaly(
    state: &Arc<AppState>,
    id: &str,
    identity: Option<&str>,
    lat: f64,
    lon: f64,
    kind: EntityKind,
    hits: &[ZoneHit],
) {
    let mut reasons = Vec::new();
    let missing_identity = identity.map(|s| s.trim().is_empty()).unwrap_or(true);

    if missing_identity {
        reasons.push(AnomalyReason::MissingIdentity);
    }

    // Check allowlist
    let allowlisted = is_allowlisted(state, id).await;
    if !allowlisted && !missing_identity {
        reasons.push(AnomalyReason::NotOnAllowlist);
    }

    if reasons.is_empty() {
        return;
    }

    for hit in hits {
        let dedup_key = format!("dedupe:flight:{}:{}", id, hit.zone_id);
        let mut redis = state.redis();
        let claimed: std::result::Result<Option<String>, _> = redis::cmd("SET")
            .arg(&dedup_key)
            .arg("1")
            .arg("NX")
            .arg("EX")
            .arg(1800)
            .query_async(&mut redis)
            .await;
        if !matches!(claimed, Ok(Some(_))) {
            continue;
        }

        let severity = if hit.is_hotspot && missing_identity {
            AnomalySeverity::Critical
        } else if !allowlisted {
            AnomalySeverity::High
        } else {
            AnomalySeverity::Elevated
        };

        let anomaly = Anomaly {
            id: format!("anom_{}", uuid::Uuid::new_v4()),
            severity,
            reasons: reasons.clone(),
            entity: AnomalyEntity {
                kind: kind.clone(),
                id: id.to_string(),
                lat,
                lon,
            },
            zone_id: hit.zone_id.clone(),
            ts: Utc::now(),
        };

        // Persist anomaly
        let akey = format!("anomaly:{}", anomaly.id);
        if let Ok(json) = serde_json::to_string(&anomaly) {
            let mut redis = state.redis();
            let _: Result<(), _> = redis.set_ex(&akey, &json, 3600).await;
        }

        let _ = state.ws_tx.send(WsMessage::Anomaly(anomaly));
    }
}

async fn is_allowlisted(state: &Arc<AppState>, id: &str) -> bool {
    let key = format!("allowlist:flight:{}", id);
    let mut redis = state.redis();
    redis.exists::<_, bool>(&key).await.unwrap_or(false)
}

// OpenSky raw response types
#[derive(Debug, Deserialize)]
struct OpenSkyResponse {
    states: Option<Vec<Vec<serde_json::Value>>>,
}

fn parse_flights(resp: OpenSkyResponse) -> Vec<Flight> {
    let mut out = Vec::new();
    let states = match resp.states {
        Some(s) => s,
        None => return out,
    };
    for row in states {
        // OpenSky state vector columns:
        // 0: icao24, 1: callsign, 5: lon, 6: lat, 7: baro_altitude,
        // 9: velocity, 10: heading, 8: on_ground
        let icao24 = row
            .get(0)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if icao24.is_empty() {
            continue;
        }

        let lat = match row.get(6).and_then(|v| v.as_f64()) {
            Some(v) => v,
            None => continue,
        };
        let lon = match row.get(5).and_then(|v| v.as_f64()) {
            Some(v) => v,
            None => continue,
        };

        if crate::validation::coordinates(lat, lon).is_err() {
            continue;
        }
        let callsign = row
            .get(1)
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        out.push(Flight {
            icao24,
            callsign,
            lat,
            lon,
            altitude_m: row.get(7).and_then(|v| v.as_f64()),
            velocity_ms: row.get(9).and_then(|v| v.as_f64()),
            heading: row.get(10).and_then(|v| v.as_f64()),
            on_ground: row.get(8).and_then(|v| v.as_bool()).unwrap_or(false),
            updated_at: Utc::now(),
        });
    }
    out
}
