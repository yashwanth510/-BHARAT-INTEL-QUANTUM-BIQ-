use axum::{
    extract::{Query, State},
    Json,
};
use redis::AsyncCommands;
use serde::Deserialize;
use std::sync::Arc;

use crate::error::{AppError, Result};
use crate::models::Weather;
use crate::providers::weather as wx;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct WeatherQuery {
    pub lat: f64,
    pub lon: f64,
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<WeatherQuery>,
) -> Result<Json<Weather>> {
    crate::validation::coordinates(q.lat, q.lon)?;
    let api_key = state.config.openweather_api_key.as_ref().ok_or_else(|| {
        AppError::ProviderUnavailable("OPENWEATHER_API_KEY not configured".into())
    })?;

    // Cache key quantized to ~1km
    let cache_key = format!("wx:{:.2}:{:.2}", q.lat, q.lon);
    let mut redis = state.redis();

    if let Ok(Some(cached)) = redis.get::<_, Option<String>>(&cache_key).await {
        if let Ok(w) = serde_json::from_str::<Weather>(&cached) {
            return Ok(Json(w));
        }
    }

    let client = &state.http;
    let weather = wx::fetch(&client, api_key, q.lat, q.lon)
        .await
        .map_err(|_| {
            AppError::ProviderUnavailable(
                "openweather request failed; check credentials, quota, and endpoint".into(),
            )
        })?;

    if let Ok(json) = serde_json::to_string(&weather) {
        let _: std::result::Result<(), _> = redis.set_ex(&cache_key, &json, 600).await;
    }

    state
        .provider_result("openweather", true, "Response received")
        .await;
    Ok(Json(weather))
}
