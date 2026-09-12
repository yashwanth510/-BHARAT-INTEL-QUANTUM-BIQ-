use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Daily quota exhausted")]
    RateLimited,
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("HTTP client error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Provider unavailable: {0}")]
    ProviderUnavailable(String),
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".into()),
            AppError::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "Daily provider quota exhausted".into(),
            ),
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            AppError::ProviderUnavailable(m) => (StatusCode::SERVICE_UNAVAILABLE, m.clone()),
            AppError::Reqwest(_) | AppError::Internal(_) => (
                StatusCode::BAD_GATEWAY,
                "Upstream provider request failed".into(),
            ),
            _ => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Service temporarily unavailable".into(),
            ),
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
