use crate::error::{AppError, Result};
use axum::http::HeaderMap;

pub fn require_token(headers: &HeaderMap, configured: Option<&str>) -> Result<()> {
    let expected = configured.ok_or_else(|| {
        AppError::ProviderUnavailable(
            "This endpoint is disabled until its server token is configured".into(),
        )
    })?;
    let supplied = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    let mismatch = supplied
        .as_bytes()
        .iter()
        .zip(expected.as_bytes())
        .fold(0u8, |diff, (a, b)| diff | (a ^ b));
    if supplied.len() != expected.len() || mismatch != 0 {
        return Err(AppError::Unauthorized);
    }
    Ok(())
}
