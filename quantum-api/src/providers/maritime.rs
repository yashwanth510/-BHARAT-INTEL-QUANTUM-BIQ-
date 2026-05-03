/// PRIORITY 5 — AISStream persistence + dark vessel detection.
use crate::models::{GenericFallbackResponse, MaritimeThreat};
use crate::services::geo_resolver::{GeoPoint, to_bounding_box};
use crate::services::neo4j::{upsert_vessel, upsert_dark_vessel_event};
use crate::services::ops_log::emit_log;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use log::{error, info};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use tokio::time::{timeout, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};

const DARK_VESSEL_THRESHOLD_SECS: i64 = 1800; // 30 minutes

/// AIS anomaly detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VesselAnomaly {
    pub mmsi: String,
    pub anomaly_type: AnomalyType,
    pub severity: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyType {
    StationaryInOpenSea,
    RouteDeviation,
    MissingSignal,
    HighSpeedInRestricted,
    UnknownNavStatus,
    DarkVessel,
}

impl std::fmt::Display for AnomalyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnomalyType::StationaryInOpenSea => write!(f, "StationaryInOpenSea"),
            AnomalyType::RouteDeviation => write!(f, "RouteDeviation"),
            AnomalyType::MissingSignal => write!(f, "MissingSignal"),
            AnomalyType::HighSpeedInRestricted => write!(f, "HighSpeedInRestricted"),
            AnomalyType::UnknownNavStatus => write!(f, "UnknownNavStatus"),
            AnomalyType::DarkVessel => write!(f, "DarkVessel"),
        }
    }
}

/// Detect anomalies in vessel behavior.
pub fn detect_anomalies(vessel: &MaritimeThreat) -> Option<VesselAnomaly> {
    if vessel.risk_score > 0.8 {
        return Some(VesselAnomaly {
            mmsi: vessel.vessel_id.clone(),
            anomaly_type: AnomalyType::UnknownNavStatus,
            severity: vessel.risk_score,
            description: format!(
                "High risk vessel {} at {:.4},{:.4}",
                vessel.vessel_name, vessel.lat, vessel.lon
            ),
        });
    }
    None
}

fn cache_ttl() -> u64 {
    env::var("MARITIME_POLL_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300)
}

async fn read_cache(redis_url: &str, key: &str) -> Option<Vec<MaritimeThreat>> {
    let client = redis::Client::open(redis_url).ok()?;
    let mut conn = client.get_multiplexed_async_connection().await.ok()?;
    let payload: Option<String> = conn.get(key).await.ok()?;
    payload.and_then(|raw| serde_json::from_str::<Vec<MaritimeThreat>>(&raw).ok())
}

async fn write_cache(redis_url: &str, key: &str, rows: &[MaritimeThreat]) {
    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            if let Ok(payload) = serde_json::to_string(rows) {
                let _: Result<(), _> = conn.set_ex(key, payload, cache_ttl()).await;
            }
        }
    }
}

/// PRIORITY 5 — Persist vessel to Redis with TTL 3600s.
/// Key: ais:vessel:{mmsi}:last_seen
async fn persist_vessel_to_redis(redis_url: &str, vessel: &MaritimeThreat) {
    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            let mmsi = vessel.vessel_id.replace("MMSI-", "");
            let key = format!("ais:vessel:{}:last_seen", mmsi);
            let payload = serde_json::json!({
                "vessel_id": vessel.vessel_id,
                "vessel_name": vessel.vessel_name,
                "lat": vessel.lat,
                "lon": vessel.lon,
                "risk_score": vessel.risk_score,
                "port": vessel.port,
                "timestamp": vessel.timestamp,
                "seen_at": Utc::now().timestamp(),
            });
            if let Ok(val) = serde_json::to_string(&payload) {
                let _: Result<(), _> = conn.set_ex(key, val, 3600).await;
            }
        }
    }
}

/// PRIORITY 5 — Check for dark vessels: disappeared >30 min from monitored zone.
/// Returns count of dark vessels detected.
pub async fn detect_dark_vessels(
    redis_url: &str,
    graph: Option<&neo4rs::Graph>,
    current_vessels: &[MaritimeThreat],
) -> usize {
    let current_mmsis: std::collections::HashSet<String> = current_vessels
        .iter()
        .map(|v| v.vessel_id.replace("MMSI-", ""))
        .collect();

    let mut dark_count = 0usize;

    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            // Scan all ais:vessel:*:last_seen keys
            let pattern = "ais:vessel:*:last_seen";
            let keys: Vec<String> = match redis::cmd("KEYS")
                .arg(pattern)
                .query_async(&mut conn)
                .await
            {
                Ok(k) => k,
                Err(_) => return 0,
            };

            let now = Utc::now().timestamp();

            for key in keys {
                // Extract MMSI from key
                let parts: Vec<&str> = key.split(':').collect();
                if parts.len() < 4 {
                    continue;
                }
                let mmsi = parts[2];

                // Skip if vessel is currently visible
                if current_mmsis.contains(mmsi) {
                    continue;
                }

                // Get last seen data
                let raw: Option<String> = conn.get(&key).await.unwrap_or(None);
                if let Some(raw) = raw {
                    if let Ok(data) = serde_json::from_str::<Value>(&raw) {
                        let seen_at = data["seen_at"].as_i64().unwrap_or(0);
                        let elapsed = now - seen_at;

                        if elapsed > DARK_VESSEL_THRESHOLD_SECS {
                            dark_count += 1;
                            let vessel_name =
                                data["vessel_name"].as_str().unwrap_or("UNKNOWN").to_string();
                            let last_seen = data["timestamp"].as_str().unwrap_or("").to_string();
                            let zone = data["port"].as_str().unwrap_or("Indian-Waters").to_string();

                            info!(
                                "[MARITIME] DARK_VESSEL detected: MMSI={} name={} last_seen={}s ago",
                                mmsi, vessel_name, elapsed
                            );

                            // Write to Neo4j
                            upsert_dark_vessel_event(
                                graph,
                                mmsi,
                                &zone,
                                &last_seen,
                                &Utc::now().to_rfc3339(),
                            )
                            .await;

                            // Mark as dark in Redis
                            let dark_key = format!("ais:vessel:{}:dark", mmsi);
                            let _: Result<(), _> = conn
                                .set_ex(dark_key, "true", 7200)
                                .await;
                        }
                    }
                }
            }
        }
    }

    dark_count
}

/// PRIORITY 5 — Compute vessel_alert_ratio for fusion engine.
/// Normalized: (dark_count as f32 / 20.0).min(1.0)
pub fn compute_vessel_alert_ratio(dark_count: usize, total_vessels: usize) -> f32 {
    let dark_ratio = (dark_count as f32 / 20.0).min(1.0);
    let high_risk_ratio = if total_vessels > 0 {
        (dark_count as f32 / total_vessels as f32).min(1.0)
    } else {
        0.0
    };
    dark_ratio.max(high_risk_ratio * 0.5)
}

/// Compute a simple risk score based on AIS data.
fn compute_risk(sog: f64, nav_status: i64) -> f64 {
    let speed_risk = (sog / 25.0).min(1.0) * 0.6;
    let status_risk = if nav_status == 15 { 0.3 } else { 0.1 };
    (speed_risk + status_risk).min(1.0)
}

/// Connect to AISstream.io WebSocket, collect vessel positions.
async fn fetch_from_aisstream(
    api_key: &str,
    max_vessels: usize,
) -> Result<Vec<MaritimeThreat>, String> {
    let ws_url = "wss://stream.aisstream.io/v0/stream";
    let ts = Utc::now().to_rfc3339();

    let subscribe_msg = serde_json::json!({
        "APIKey": api_key,
        "BoundingBoxes": [
            [[5.0, 55.0], [35.0, 100.0]]
        ],
        "FilterMessageTypes": ["PositionReport"]
    });

    info!("[MARITIME] Connecting to AISstream.io...");
    let (ws_stream, _) = timeout(Duration::from_secs(15), connect_async(ws_url))
        .await
        .map_err(|_| "AISstream WebSocket connection timeout".to_string())?
        .map_err(|e| format!("AISstream WebSocket connect error: {}", e))?;

    info!("[MARITIME] AISstream connected. Sending subscription...");
    let (mut write, mut read) = ws_stream.split();

    write
        .send(Message::Text(subscribe_msg.to_string()))
        .await
        .map_err(|e| format!("AISstream subscribe error: {}", e))?;

    let mut vessels: Vec<MaritimeThreat> = Vec::new();
    let mut seen_mmsi = std::collections::HashSet::new();
    let collect_timeout = Duration::from_secs(30);

    let result = timeout(collect_timeout, async {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(json) = serde_json::from_str::<Value>(&text) {
                        if let Some(err) = json.get("error").and_then(|e| e.as_str()) {
                            return Err(format!("AISstream error: {}", err));
                        }

                        let msg_type = json["MessageType"].as_str().unwrap_or("Unknown");
                        if msg_type == "PositionReport" {
                            let meta = &json["MetaData"];
                            let msg_body = &json["Message"]["PositionReport"];

                            let mmsi = meta["MMSI"].as_i64().unwrap_or(0);
                            if mmsi == 0 || seen_mmsi.contains(&mmsi) {
                                continue;
                            }
                            seen_mmsi.insert(mmsi);

                            let ship_name = meta["ShipName"]
                                .as_str()
                                .unwrap_or("UNKNOWN")
                                .trim()
                                .to_string();

                            let lat = meta["latitude"].as_f64().unwrap_or(0.0);
                            let lon = meta["longitude"].as_f64().unwrap_or(0.0);
                            let sog = msg_body["Sog"].as_f64().unwrap_or(0.0);
                            let nav_status =
                                msg_body["NavigationalStatus"].as_i64().unwrap_or(15);

                            vessels.push(MaritimeThreat {
                                vessel_id: format!("MMSI-{}", mmsi),
                                vessel_name: ship_name,
                                lat,
                                lon,
                                risk_score: compute_risk(sog, nav_status),
                                port: "Indian-Waters".to_string(),
                                timestamp: ts.clone(),
                            });

                            if vessels.len() >= max_vessels {
                                break;
                            }
                        }
                    }
                }
                Ok(Message::Close(c)) => {
                    info!("[MARITIME] AISstream closed: {:?}", c);
                    break;
                }
                Err(e) => {
                    error!("[MARITIME] AISstream read error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        Ok(())
    })
    .await;

    match result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return Err(e),
        Err(_) => {
            info!(
                "[MARITIME] Collection period ended with {} vessels",
                vessels.len()
            );
        }
    }

    if vessels.is_empty() {
        Err("No live vessels detected in the monitored region within the collection window"
            .to_string())
    } else {
        Ok(vessels)
    }
}

pub async fn fetch_maritime_threats(
    redis_url: &str,
    location: Option<&GeoPoint>,
) -> GenericFallbackResponse<MaritimeThreat> {
    let cache_key = match location {
        Some(loc) => format!("cache:maritime:{:.4}:{:.4}", loc.lat, loc.lon),
        None => "cache:maritime:latest".to_string(),
    };

    // 1. Try Redis cache first
    if let Some(rows) = read_cache(redis_url, &cache_key).await {
        emit_log(redis_url, "MARITIME", "CACHE_HIT", &format!("{} vessels", rows.len())).await;
        return GenericFallbackResponse {
            status: "ok".to_string(),
            provider_path: Some("redis_cache".to_string()),
            error: None,
            results: rows,
        };
    }

    // 2. Try AISstream.io WebSocket
    let aisstream_key = env::var("AISSTREAM_API_KEY").unwrap_or_default();
    if !aisstream_key.is_empty() {
        let max_vessels = env::var("AISSTREAM_MAX_VESSELS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(20);

        match fetch_from_aisstream(&aisstream_key, max_vessels).await {
            Ok(rows) => {
                info!("[MARITIME] AISstream returned {} vessels", rows.len());

                // PRIORITY 5 — Persist each vessel to Redis
                for vessel in &rows {
                    persist_vessel_to_redis(redis_url, vessel).await;
                }

                write_cache(redis_url, &cache_key, &rows).await;

                emit_log(
                    redis_url,
                    "AIS",
                    "FETCH_COMPLETE",
                    &format!("{} VESSELS", rows.len()),
                )
                .await;

                return GenericFallbackResponse {
                    status: "ok".to_string(),
                    provider_path: Some("aisstream".to_string()),
                    error: None,
                    results: rows,
                };
            }
            Err(e) => {
                error!("[MARITIME] AISstream failed: {}", e);
                emit_log(redis_url, "AIS", "FETCH_ERROR", &e).await;
                return GenericFallbackResponse {
                    status: "provider_error".to_string(),
                    provider_path: Some("aisstream".to_string()),
                    error: Some(e),
                    results: vec![],
                };
            }
        }
    }

    emit_log(redis_url, "AIS", "PROVIDER_MISSING", "AISSTREAM_API_KEY not set").await;
    GenericFallbackResponse {
        status: "provider_missing".to_string(),
        provider_path: Some("aisstream".to_string()),
        error: Some("AISSTREAM_API_KEY missing".to_string()),
        results: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::{compute_risk, compute_vessel_alert_ratio};

    #[test]
    fn risk_score_reasonable() {
        let fast = compute_risk(20.0, 0);
        let slow = compute_risk(2.0, 0);
        assert!(fast > slow);
        assert!(fast <= 1.0);
        assert!(slow >= 0.0);
    }

    #[test]
    fn unknown_nav_status_increases_risk() {
        let known = compute_risk(10.0, 0);
        let unknown = compute_risk(10.0, 15);
        assert!(unknown > known);
    }

    #[test]
    fn vessel_alert_ratio_normalized() {
        let ratio = compute_vessel_alert_ratio(20, 100);
        assert!(ratio <= 1.0);
        let zero = compute_vessel_alert_ratio(0, 100);
        assert_eq!(zero, 0.0);
    }
}
