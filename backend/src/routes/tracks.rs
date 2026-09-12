use axum::{extract::State, Json};
use redis::AsyncCommands;
use serde_json::Value;
use std::sync::Arc;

use crate::error::Result;
use crate::state::AppState;

pub async fn flights(State(state): State<Arc<AppState>>) -> Result<Json<Vec<Value>>> {
    list_entities(&state, "entity:flight:*").await
}

pub async fn vessels(State(state): State<Arc<AppState>>) -> Result<Json<Vec<Value>>> {
    list_entities(&state, "entity:vessel:*").await
}

pub async fn vehicles(State(state): State<Arc<AppState>>) -> Result<Json<Vec<Value>>> {
    list_entities(&state, "entity:vehicle:*").await
}

async fn list_entities(state: &Arc<AppState>, pattern: &str) -> Result<Json<Vec<Value>>> {
    let mut redis = state.redis();
    let keys: Vec<String> = redis.keys(pattern).await?;
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
