use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnomalySeverity {
    Critical,
    High,
    Elevated,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnomalyReason {
    MissingIdentity,
    NotOnAllowlist,
    CorridorCrossing,
    AisDark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Flight,
    Vessel,
    Vehicle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyEntity {
    pub kind: EntityKind,
    pub id: String,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub id: String,
    pub severity: AnomalySeverity,
    pub reasons: Vec<AnomalyReason>,
    pub entity: AnomalyEntity,
    pub zone_id: String,
    pub ts: DateTime<Utc>,
}
