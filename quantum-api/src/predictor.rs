use crate::models::GlobalThreat;
use serde::Serialize;
use chrono::{Utc, Duration};

#[derive(Serialize)]
pub struct Prediction {
    pub likelihood: f64,
    pub target: String,
    pub date: String,
}

pub fn predict_attack(threats: &Vec<GlobalThreat>) -> Prediction {
    let base_score = if threats.is_empty() { 15.0 } else { 
        let mut score = 40.0;
        for t in threats {
            score += t.confidence * 10.0;
        }
        score.min(99.0)
    };
    
    let target = if threats.iter().any(|t| t.country == "CN") {
        "Ladakh border".to_string()
    } else {
        "Siachen border".to_string()
    };
    
    Prediction {
        likelihood: base_score,
        date: (Utc::now() + Duration::days(5)).format("%Y-%m-%d").to_string(),
        target,
    }
}
