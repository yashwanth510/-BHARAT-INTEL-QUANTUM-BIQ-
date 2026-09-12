use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vehicle {
    pub device_id: String,
    pub label: Option<String>,
    pub lat: f64,
    pub lon: f64,
    pub speed_kmh: Option<f64>,
    pub heading: Option<f64>,
    pub updated_at: DateTime<Utc>,
}
