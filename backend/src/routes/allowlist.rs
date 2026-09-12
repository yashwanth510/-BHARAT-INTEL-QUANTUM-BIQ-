use axum::{extract::State, Json};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use crate::error::Result;
use crate::state::AppState;

#[derive(Deserialize, Serialize)]
pub struct AllowlistEntry {
    /// "flight" | "vessel" | "vehicle"
    pub kind: String,
    pub id: String,
    pub label: Option<String>,
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<Value>>> {
    crate::auth::require_token(&headers, state.config.admin_api_token.as_deref())?;
    let mut redis = state.redis();
    let keys: Vec<String> = redis.keys("allowlist:*").await?;
    let mut out = Vec::new();
    for key in &keys {
        // key format: allowlist:{kind}:{id}
        let parts: Vec<&str> = key.splitn(3, ':').collect();
        if parts.len() == 3 {
            let label: Option<String> = redis.get(key).await?;
            out.push(serde_json::json!({
                "kind": parts[1],
                "id": parts[2],
                "label": label
            }));
        }
    }
    Ok(Json(out))
}

pub async fn upsert(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(entry): Json<AllowlistEntry>,
) -> Result<Json<serde_json::Value>> {
    if !["flight", "vessel", "vehicle"].contains(&entry.kind.as_str())
        || !crate::validation::identifier(&entry.id)
    {
        return Err(crate::error::AppError::BadRequest(
            "Invalid entity kind or id".into(),
        ));
    }
    let key = format!("allowlist:{}:{}", entry.kind, entry.id);
    let label = entry.label.clone().unwrap_or_else(|| entry.id.clone());
    crate::auth::require_token(&headers, state.config.admin_api_token.as_deref())?;
    let mut redis = state.redis();
    let _: () = redis.set(&key, &label).await?;
    Ok(Json(serde_json::json!({ "ok": true, "key": key })))
}
