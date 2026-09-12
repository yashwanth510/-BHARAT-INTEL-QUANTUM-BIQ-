use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vessel {
    pub mmsi: String,
    pub name: Option<String>,
    pub lat: f64,
    pub lon: f64,
    pub speed_knots: Option<f64>,
    pub course: Option<f64>,
    pub ship_type: Option<i32>,
    pub updated_at: DateTime<Utc>,
}
