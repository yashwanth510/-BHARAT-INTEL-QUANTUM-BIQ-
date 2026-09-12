use axum::{extract::State, Json};
use redis::AsyncCommands;
use serde_json::Value;
use std::sync::Arc;

use crate::error::Result;
use crate::state::AppState;

pub async fn list(State(state): State<Arc<AppState>>) -> Result<Json<Vec<Value>>> {
    let mut redis = state.redis();
    let keys: Vec<String> = redis.keys("anomaly:anom_*").await?;
    let mut out = Vec::with_capacity(keys.len());
    for key in keys {
        let val: Option<String> = redis.get(&key).await?;
        if let Some(s) = val {
            if let Ok(v) = serde_json::from_str::<Value>(&s) {
                out.push(v);
            }
        }
    }
    Ok(Json(out))
}
