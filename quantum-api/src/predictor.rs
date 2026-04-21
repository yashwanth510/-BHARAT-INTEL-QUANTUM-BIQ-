use crate::models::GlobalThreat;
use serde::Serialize;

#[derive(Serialize)]
pub struct Prediction {
    pub likelihood: f64,
    pub target: String,
    pub date: String,
}

pub fn predict_attack(threats: &Vec<GlobalThreat>) -> Prediction {
    // ML Stub: Likelihood based on active threats
    let base_score = if threats.is_empty() { 45.0 } else { 87.0 };
    
    Prediction {
        likelihood: base_score,
        date: "2026-05-15".to_string(),
        target: "Siachen border".to_string(),
    }
}
