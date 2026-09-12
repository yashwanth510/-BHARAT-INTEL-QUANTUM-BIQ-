use anyhow::Context;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use redis::AsyncCommands;
use serde_json::Value;
use std::sync::Arc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::models::anomaly::{Anomaly, AnomalyEntity, AnomalyReason, AnomalySeverity, EntityKind};
use crate::models::Vessel;
use crate::state::AppState;
use crate::ws::WsMessage;

pub fn spawn(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut retry_secs = 15_u64;
        loop {
            if state.config.aisstream_api_key.is_none() {
                tracing::info!("AISstream: no API key, skipping");
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                continue;
            }
            tracing::info!("AISstream: connecting...");
            let started = std::time::Instant::now();
            if let Err(e) = run(Arc::clone(&state)).await {
                state
                    .provider_result("aisstream", false, &e.to_string())
                    .await;
                tracing::warn!("AISstream disconnected: {:#}", e);
            }
            if started.elapsed().as_secs() >= 300 {
                retry_secs = 15;
            }
            let jitter = (uuid::Uuid::new_v4().as_u128() % 10) as u64;
            tracing::info!("AISstream: retrying in {}s", retry_secs + jitter);
            tokio::time::sleep(tokio::time::Duration::from_secs(retry_secs + jitter)).await;
            retry_secs = (retry_secs * 2).min(300);
        }
    });
}

async fn run(state: Arc<AppState>) -> anyhow::Result<()> {
    let api_key = state.config.aisstream_api_key.as_ref().unwrap();

    // Subscribe with bounding boxes
    let mut bboxes: Value = serde_json::from_str(&state.config.aisstream_bboxes)
        .context("Invalid AISSTREAM_BBOXES JSON")?;
    // Accept the old flat configuration, normalize to AISstream's corner-pair format.
    if let Some(boxes) = bboxes.as_array_mut() {
        for bbox in boxes {
            if bbox
                .as_array()
                .is_some_and(|a| a.len() == 4 && a[0].is_number())
            {
                *bbox = serde_json::json!([[bbox[0], bbox[1]], [bbox[2], bbox[3]]]);
            }
        }
    }

    let sub = serde_json::json!({
        "APIKey": api_key,
        "BoundingBoxes": bboxes,
        "FilterMessageTypes": ["PositionReport"]
    });
    let (ws_stream, _) = tokio::time::timeout(
        tokio::time::Duration::from_secs(20),
        connect_async("wss://stream.aisstream.io/v0/stream"),
    )
    .await
    .context("WebSocket handshake timed out")?
    .context("WebSocket handshake failed before subscription")?;
    let (mut write, mut read) = ws_stream.split();
    write
        .send(Message::Text(sub.to_string()))
        .await
        .context("Failed to send AISstream subscription")?;

    while let Some(msg) = read.next().await {
        match msg.context("AISstream connection failed after subscription was sent")? {
            Message::Text(text) => {
                if let Ok(val) = serde_json::from_str::<Value>(&text) {
                    check_subscription_error(&val)?;
                    if let Some(vessel) = parse_ais_position(&val) {
                        state
                            .provider_result("aisstream", true, "Position received")
                            .await;
                        store_and_broadcast(&state, vessel).await;
                    }
                }
            }
            Message::Binary(bytes) => {
                if let Ok(val) = serde_json::from_slice::<Value>(&bytes) {
                    check_subscription_error(&val)?;
                    if let Some(vessel) = parse_ais_position(&val) {
                        state
                            .provider_result("aisstream", true, "Position received")
                            .await;
                        store_and_broadcast(&state, vessel).await;
                    }
                }
            }
            Message::Ping(p) => {
                write.send(Message::Pong(p)).await?;
            }
            Message::Close(_) => anyhow::bail!("AISstream closed the connection"),
            _ => {}
        }
    }
    anyhow::bail!("AISstream connection ended")
}

fn check_subscription_error(val: &Value) -> anyhow::Result<()> {
    if val.get("error").is_some() || val.get("Error").is_some() {
        // Do not expose provider payloads, which may echo subscription credentials.
        anyhow::bail!(
            "AISstream rejected subscription; check API key, bounding boxes and connection limits"
        );
    }
    Ok(())
}

fn parse_ais_position(val: &Value) -> Option<Vessel> {
    let msg = val.get("Message")?.get("PositionReport")?;
    let mmsi = msg.get("UserID")?.as_i64()?.to_string();
    let lat = msg.get("Latitude")?.as_f64()?;
    let lon = msg.get("Longitude")?.as_f64()?;
    if crate::validation::coordinates(lat, lon).is_err() {
        return None;
    }

    let meta = val.get("MetaData");
    let name = meta
        .and_then(|m| m.get("ShipName"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    Some(Vessel {
        mmsi,
        name,
        lat,
        lon,
        speed_knots: msg.get("Sog").and_then(|v| v.as_f64()),
        course: msg.get("Cog").and_then(|v| v.as_f64()),
        ship_type: None,
        updated_at: Utc::now(),
    })
}

async fn store_and_broadcast(state: &Arc<AppState>, vessel: Vessel) {
    let key = format!("entity:vessel:{}", vessel.mmsi);
    if let Ok(json) = serde_json::to_string(&vessel) {
        let mut redis = state.redis();
        let _: Result<(), _> = redis.set_ex(&key, &json, 300).await;
    }

    // Geofence
    let hits = state.geofence.check(vessel.lat, vessel.lon);
    if !hits.is_empty() {
        check_vessel_anomaly(state, &vessel).await;
    }

    let _ = state.ws_tx.send(WsMessage::VesselUpdate(vessel));
}

async fn check_vessel_anomaly(state: &Arc<AppState>, vessel: &Vessel) {
    let mut reasons = Vec::new();

    let missing_identity = vessel.mmsi.trim().is_empty();
    if missing_identity {
        reasons.push(AnomalyReason::MissingIdentity);
    }

    let key = format!("allowlist:vessel:{}", vessel.mmsi);
    let mut redis = state.redis();
    let allowlisted: bool = redis.exists(&key).await.unwrap_or(false);
    if !allowlisted && !missing_identity {
        reasons.push(AnomalyReason::NotOnAllowlist);
    }

    if reasons.is_empty() {
        return;
    }

    let hits = state.geofence.check(vessel.lat, vessel.lon);
    for hit in &hits {
        let dedup_key = format!("dedupe:vessel:{}:{}", vessel.mmsi, hit.zone_id);
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
                kind: EntityKind::Vessel,
                id: vessel.mmsi.clone(),
                lat: vessel.lat,
                lon: vessel.lon,
            },
            zone_id: hit.zone_id.clone(),
            ts: Utc::now(),
        };

        if let Ok(json) = serde_json::to_string(&anomaly) {
            let akey = format!("anomaly:{}", anomaly.id);
            let _: Result<(), _> = redis.set_ex(&akey, &json, 3600).await;
        }
        let _ = state.ws_tx.send(WsMessage::Anomaly(anomaly));
    }
}

#[cfg(test)]
mod subscription_tests {
    use super::check_subscription_error;

    #[test]
    fn rejects_provider_errors_without_echoing_payload() {
        for field in ["error", "Error"] {
            let value = serde_json::json!({field: "rejected sensitive-subscription-value"});
            let error = check_subscription_error(&value).unwrap_err().to_string();
            assert!(error.contains("rejected subscription"));
            assert!(!error.contains("sensitive-subscription-value"));
        }
        assert!(check_subscription_error(&serde_json::json!({
            "MessageType": "SubscriptionConfirmation"
        }))
        .is_ok());
    }
}
