use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Weather {
    pub lat: f64,
    pub lon: f64,
    pub description: String,
    pub temp_c: f64,
    pub feels_like_c: f64,
    pub humidity_pct: u32,
    pub wind_speed_ms: f64,
    pub wind_deg: u32,
    pub visibility_m: Option<u32>,
    pub icon: String,
}
