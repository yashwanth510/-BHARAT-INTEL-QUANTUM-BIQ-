use crate::error::{AppError, Result};
pub fn coordinates(lat: f64, lon: f64) -> Result<()> {
    if !lat.is_finite()
        || !lon.is_finite()
        || !(-90.0..=90.0).contains(&lat)
        || !(-180.0..=180.0).contains(&lon)
    {
        return Err(AppError::BadRequest(
            "lat must be -90..90 and lon -180..180".into(),
        ));
    }
    Ok(())
}
pub fn identifier(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_coordinates() {
        for (lat, lon) in [
            (91.0, 0.0),
            (0.0, 181.0),
            (f64::NAN, 0.0),
            (0.0, f64::INFINITY),
        ] {
            assert!(coordinates(lat, lon).is_err());
        }
        assert!(coordinates(0.0, 0.0).is_ok());
    }
    #[test]
    fn rejects_redis_key_injection() {
        assert!(!identifier("device:*"));
        assert!(!identifier(""));
        assert!(identifier("vehicle-123"));
    }
}
