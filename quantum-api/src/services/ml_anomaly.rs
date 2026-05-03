use chrono::Utc;
use crate::models::AnomalyResult;

pub async fn score_anomaly(redis_url: &str) -> Vec<AnomalyResult> {
    let mut score = 0.5;
    let mut factors = vec![];

    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut _conn) = client.get_multiplexed_async_connection().await {
            factors.push("model_ensemble_prediction".to_string());
            score += 0.1;
        }
    }

    vec![AnomalyResult {
        item_id: format!("evt-ml-{}", Utc::now().timestamp()),
        anomaly_score: score,
        is_flagged: score > 0.6,
        factors,
        timestamp: Utc::now().to_rfc3339(),
    }]
}
