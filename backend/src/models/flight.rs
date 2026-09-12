use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flight {
    pub icao24: String,
    pub callsign: Option<String>,
    pub lat: f64,
    pub lon: f64,
    pub altitude_m: Option<f64>,
    pub velocity_ms: Option<f64>,
    pub heading: Option<f64>,
    pub on_ground: bool,
    pub updated_at: DateTime<Utc>,
}
