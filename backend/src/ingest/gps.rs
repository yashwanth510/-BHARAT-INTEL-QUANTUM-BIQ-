use axum::{extract::State, response::IntoResponse, Json};
use chrono::Utc;
use redis::AsyncCommands;
use serde_json::Value;
use std::sync::Arc;
use tokio::time::{interval, Duration};

use crate::error::{AppError, Result};
use crate::models::anomaly::{Anomaly, AnomalyEntity, AnomalyReason, AnomalySeverity, EntityKind};
use crate::models::Vehicle;
use crate::state::AppState;
use crate::ws::WsMessage;

pub fn spawn(state: Arc<AppState>) {
    tokio::spawn(async move {
        if state.config.gps_api_base_url.is_none() || state.config.gps_api_key.is_none() {
            tracing::info!(
                "GPS: no API key/URL configured, poll disabled (webhook /ingest/gps active)"
            );
            return;
        }
        let host = reqwest::Url::parse(state.config.gps_api_base_url.as_deref().unwrap())
            .ok()
            .and_then(|u| u.host_str().map(str::to_owned))
            .unwrap_or_default();
        if host == "geoapify.com" || host.ends_with(".geoapify.com") {
            state.provider_result("gps", false, "Geoapify is not a live device position feed; configure a GPS tracking provider").await;
            return;
        }
        run_poll(state).await;
    });
}

async fn run_poll(state: Arc<AppState>) {
    let poll_secs = state.config.gps_poll_seconds;
    let mut ticker = interval(Duration::from_secs(poll_secs));
    let client = &state.http;

    loop {
        ticker.tick().await;
        let base_url = state.config.gps_api_base_url.as_ref().unwrap();
        let api_key = state.config.gps_api_key.as_ref().unwrap();

        let url = format!("{}/positions", base_url);
        match client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
                Ok(data) => {
                    let vehicles = parse_gps_response(data);
                    state
                        .provider_result("gps", true, "Position response received")
                        .await;
                    tracing::debug!("GPS poll: {} vehicles", vehicles.len());
                    for v in vehicles {
                        store_and_broadcast(&state, v).await;
                    }
                }
                Err(e) => tracing::warn!("GPS parse error: {}", e),
            },
            Ok(resp) => tracing::warn!("GPS HTTP {}", resp.status()),
            Err(e) => tracing::warn!("GPS request failed: {}", e),
        }
    }
}

/// Webhook endpoint — GPS provider POSTs positions here.
pub async fn webhook_handler(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse> {
    crate::auth::require_token(&headers, state.config.gps_webhook_token.as_deref())?;
    let vehicles = parse_gps_response(payload);
    if vehicles.is_empty() {
        return Err(AppError::BadRequest(
            "No valid GPS positions supplied".into(),
        ));
    }
    for v in vehicles {
        store_and_broadcast(&state, v).await;
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// Parse generic GPS response — supports array of positions or single position.
/// Expected shape (flexible): [{device_id, lat, lon, speed?, heading?, label?}]
fn parse_gps_response(val: Value) -> Vec<Vehicle> {
    let mut out = Vec::new();

    let items = if val.is_array() {
        val.as_array().cloned().unwrap_or_default()
    } else if let Some(arr) = val.get("positions").and_then(|v| v.as_array()) {
        arr.clone()
    } else if let Some(arr) = val.get("devices").and_then(|v| v.as_array()) {
        arr.clone()
    } else {
        vec![val]
    };

    for item in items {
        let device_id = item
            .get("device_id")
            .or_else(|| item.get("id"))
            .or_else(|| item.get("deviceId"))
            .and_then(|v| {
                v.as_str()
                    .map(|s| s.to_string())
                    .or_else(|| v.as_i64().map(|i| i.to_string()))
            })
            .unwrap_or_default();
        if !crate::validation::identifier(&device_id) {
            continue;
        }

        let lat = item
            .get("lat")
            .or_else(|| item.get("latitude"))
            .and_then(|v| v.as_f64());
        let lon = item
            .get("lon")
            .or_else(|| item.get("lng"))
            .or_else(|| item.get("longitude"))
            .and_then(|v| v.as_f64());

        let (lat, lon) = match (lat, lon) {
            (Some(la), Some(lo)) => (la, lo),
            _ => continue,
        };

        if crate::validation::coordinates(lat, lon).is_err() {
            continue;
        }
        out.push(Vehicle {
            device_id,
            label: item
                .get("label")
                .or_else(|| item.get("name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            lat,
            lon,
            speed_kmh: item.get("speed").and_then(|v| v.as_f64()),
            heading: item
                .get("heading")
                .or_else(|| item.get("course"))
                .and_then(|v| v.as_f64()),
            updated_at: Utc::now(),
        });
    }
    out
}

async fn store_and_broadcast(state: &Arc<AppState>, vehicle: Vehicle) {
    let key = format!("entity:vehicle:{}", vehicle.device_id);
    if let Ok(json) = serde_json::to_string(&vehicle) {
        let mut redis = state.redis();
        let _: std::result::Result<(), _> = redis.set_ex(&key, &json, 120).await;
    }

    let hits = state.geofence.check(vehicle.lat, vehicle.lon);
    if !hits.is_empty() {
        check_vehicle_anomaly(state, &vehicle).await;
    }

    let _ = state.ws_tx.send(WsMessage::VehicleUpdate(vehicle));
}

async fn check_vehicle_anomaly(state: &Arc<AppState>, vehicle: &Vehicle) {
    let mut reasons = Vec::new();

    let key = format!("allowlist:vehicle:{}", vehicle.device_id);
    let mut redis = state.redis();
    let allowlisted: bool = redis.exists(&key).await.unwrap_or(false);
    if !allowlisted {
        reasons.push(AnomalyReason::NotOnAllowlist);
    }
    if reasons.is_empty() {
        return;
    }

    let hits = state.geofence.check(vehicle.lat, vehicle.lon);
    for hit in &hits {
        let dedup_key = format!("dedupe:vehicle:{}:{}", vehicle.device_id, hit.zone_id);
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

        let anomaly = Anomaly {
            id: format!("anom_{}", uuid::Uuid::new_v4()),
            severity: if hit.is_hotspot {
                AnomalySeverity::High
            } else {
                AnomalySeverity::Elevated
            },
            reasons: reasons.clone(),
            entity: AnomalyEntity {
                kind: EntityKind::Vehicle,
                id: vehicle.device_id.clone(),
                lat: vehicle.lat,
                lon: vehicle.lon,
            },
            zone_id: hit.zone_id.clone(),
            ts: Utc::now(),
        };

        if let Ok(json) = serde_json::to_string(&anomaly) {
            let akey = format!("anomaly:{}", anomaly.id);
            let _: std::result::Result<(), _> = redis.set_ex(&akey, &json, 3600).await;
        }
        let _ = state.ws_tx.send(WsMessage::Anomaly(anomaly));
    }
}
