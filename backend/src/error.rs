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

/// Report only transport categories/status codes; never provider bodies or URLs.
pub fn provider_detail(error: &anyhow::Error) -> String {
    for cause in error.chain() {
        if let Some(error) = cause.downcast_ref::<reqwest::Error>() {
            return http_detail(error);
        }
    }
    "Invalid upstream response".into()
}

pub fn http_detail(error: &reqwest::Error) -> String {
    if let Some(status) = error.status() {
        format!("Upstream HTTP {}", status.as_u16())
    } else if error.is_timeout() {
        "Upstream request timed out".into()
    } else if error.is_connect() {
        "Upstream connection failed (DNS, TCP or TLS)".into()
    } else if error.is_decode() {
        "Invalid upstream response".into()
    } else {
        "Upstream transport failed".into()
    }
}

#[cfg(test)]
mod diagnostic_tests {
    #[test]
    fn reports_status_without_response_body_or_url_secrets() {
        let response = axum::http::Response::builder()
            .status(401)
            .body("secret-provider-response")
            .unwrap();
        let error = reqwest::Response::from(response)
            .error_for_status()
            .unwrap_err()
            .with_url(reqwest::Url::parse("https://example.com/?apiKey=secret").unwrap());
        assert_eq!(super::http_detail(&error), "Upstream HTTP 401");
        assert_eq!(
            super::provider_detail(&anyhow::Error::from(error)),
            "Upstream HTTP 401"
        );
    }
}
