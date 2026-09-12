use axum::{extract::State, Json};
use serde::Deserialize;
use std::sync::Arc;

use crate::error::{AppError, Result};
use crate::providers::sentinel::{self, SentinelSnapshot};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct SnapshotRequest {
    pub lat: f64,
    pub lon: f64,
    /// Bounding box side in degrees (default 0.1 ≈ 11km)
    pub bbox_deg: Option<f64>,
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SnapshotRequest>,
) -> Result<Json<SentinelSnapshot>> {
    crate::validation::coordinates(req.lat, req.lon)?;
    let size = req.bbox_deg.unwrap_or(0.1);
    if !size.is_finite()
        || !(0.001..=1.0).contains(&size)
        || req.lat.abs() + size / 2.0 > 90.0
        || req.lon.abs() + size / 2.0 > 180.0
    {
        return Err(AppError::BadRequest("Invalid snapshot bounding box".into()));
    }
    let client_id =
        state.config.sentinel_client_id.as_ref().ok_or_else(|| {
            AppError::ProviderUnavailable("SENTINEL_CLIENT_ID not configured".into())
        })?;
    let client_secret = state
        .config
        .sentinel_client_secret
        .as_ref()
        .ok_or_else(|| {
            AppError::ProviderUnavailable("SENTINEL_CLIENT_SECRET not configured".into())
        })?;

    let client = &state.http;
    let snap = sentinel::fetch_snapshot(
        &client,
        client_id,
        client_secret,
        &state.config.sentinel_token_url,
        &state.config.sentinel_process_url,
        req.lat,
        req.lon,
        req.bbox_deg.unwrap_or(0.1),
    )
    .await
    .map_err(|_| {
        AppError::ProviderUnavailable(
            "sentinel request failed; check credentials, quota, and endpoint".into(),
        )
    })?;

    state
        .provider_result("sentinel", true, "Response received")
        .await;
    Ok(Json(snap))
}
