use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GlobalThreat {
    pub actor: String,           // "Hafiz Saeed"
    pub country: String,         // "PK"
    pub confidence: f64,         // 0.92
    pub sources: Vec<String>,    // ["X: @pakdefence", "Dawn: article123"]
    pub location: Option<String>, // "Bahawalpur"
    pub timestamp: String,
}
