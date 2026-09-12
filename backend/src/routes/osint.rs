use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::error::{AppError, Result};
use crate::providers::tavily::{self, TavilyResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct EnrichRequest {
    pub lat: f64,
    pub lon: f64,
    pub entity: Option<String>,
    pub query: Option<String>,
}

#[derive(Serialize)]
pub struct EnrichResponse {
    pub results: Vec<TavilyResult>,
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EnrichRequest>,
) -> Result<Json<EnrichResponse>> {
    crate::validation::coordinates(req.lat, req.lon)?;
    let api_key = state
        .config
        .tavily_api_key
        .as_ref()
        .ok_or_else(|| AppError::ProviderUnavailable("TAVILY_API_KEY not configured".into()))?;

    let query = req.query.unwrap_or_else(|| {
        let entity = req.entity.as_deref().unwrap_or("unknown entity");
        format!(
            "border incident surveillance India lat {} lon {} {}",
            req.lat, req.lon, entity
        )
    });

    if query.len() > 500 {
        return Err(AppError::BadRequest("Query is too long".into()));
    }
    let mut redis = state.redis();
    let key = format!("quota:tavily:{}", chrono::Utc::now().format("%Y-%m-%d"));
    let count: i64 = redis::Script::new("local n = redis.call('INCR', KEYS[1]); if n == 1 then redis.call('EXPIRE', KEYS[1], 86400) end; return n").key(key).invoke_async(&mut redis).await?;
    if count > state.config.tavily_max_per_day as i64 {
        return Err(AppError::RateLimited);
    }
    let client = &state.http;
    let outcome = tavily::search(&client, api_key, &query, 5).await;
    let results = match outcome {
        Ok(value) => value,
        Err(error) => {
            let detail = crate::error::provider_detail(&error);
            tracing::warn!(provider = "tavily", %detail, "Provider request failed");
            state.provider_result("tavily", false, &detail).await;
            return Err(AppError::ProviderUnavailable(format!("tavily: {detail}")));
        }
    };

    state
        .provider_result("tavily", true, "Response received")
        .await;
    Ok(Json(EnrichResponse { results }))
}
