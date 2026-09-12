use crate::state::AppState;
use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn handler(State(state): State<Arc<AppState>>) -> (StatusCode, Json<Value>) {
    let mut redis = state.redis();
    let ready = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        redis::cmd("PING").query_async::<_, String>(&mut redis),
    )
    .await
    .is_ok_and(|r| r.is_ok());
    let status = state.provider_status.read().await;
    let cfg = &state.config;
    let providers = [
        ("opensky", cfg.ingest_enabled),
        ("aisstream", cfg.aisstream_api_key.is_some()),
        (
            "gps",
            cfg.gps_api_key.is_some() && cfg.gps_api_base_url.is_some(),
        ),
        ("openweather", cfg.openweather_api_key.is_some()),
        (
            "sentinel",
            cfg.sentinel_client_id.is_some() && cfg.sentinel_client_secret.is_some(),
        ),
        ("tavily", cfg.tavily_api_key.is_some()),
    ]
    .into_iter()
    .map(|(name, configured)| {
        (
            name.to_string(),
            json!({"configured":configured,"runtime":status.get(name)}),
        )
    })
    .collect::<serde_json::Map<_, _>>();
    (
        if ready {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        Json(
            json!({"status": if ready {"ok"} else {"degraded"}, "redis":ready, "ingest_enabled":cfg.ingest_enabled, "geometry":"illustrative; not authoritative borders", "providers":providers}),
        ),
    )
}
