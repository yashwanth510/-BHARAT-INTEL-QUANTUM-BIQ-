use crate::{
    error::{AppError, Result},
    routes::weather::WeatherQuery,
    state::AppState,
};
use axum::{
    extract::{Query, State},
    Json,
};
use redis::AsyncCommands;
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Query(q): Query<WeatherQuery>,
) -> Result<Json<Value>> {
    crate::validation::coordinates(q.lat, q.lon)?;
    let key = state
        .config
        .geoapify_api_key
        .as_deref()
        .ok_or_else(|| AppError::ProviderUnavailable("Geoapify not configured".into()))?;
    let cache_key = format!("location:{:.4}:{:.4}", q.lat, q.lon);
    let mut redis = state.redis();
    if let Some(value) = redis.get::<_, Option<String>>(&cache_key).await? {
        return Ok(Json(serde_json::from_str(&value)?));
    }
    let outcome = async {
        state
            .http
            .get("https://api.geoapify.com/v1/geocode/reverse")
            .query(&[
                ("lat", q.lat.to_string()),
                ("lon", q.lon.to_string()),
                ("format", "json".into()),
                ("apiKey", key.into()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await
    }
    .await;
    let response = match outcome {
        Ok(value) => value,
        Err(error) => {
            let detail = crate::error::http_detail(&error);
            tracing::warn!(provider = "geoapify", %detail, "Provider request failed");
            state.provider_result("geoapify", false, &detail).await;
            return Err(AppError::ProviderUnavailable(format!("geoapify: {detail}")));
        }
    };
    let result = json!({"lat": q.lat, "lon": q.lon, "label": response["results"][0]["formatted"], "attribution":"Powered by Geoapify"});
    let _: () = redis.set_ex(cache_key, result.to_string(), 86400).await?;
    state
        .provider_result("geoapify", true, "Location resolved")
        .await;
    Ok(Json(result))
}
