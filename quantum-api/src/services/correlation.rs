use chrono::Utc;
use crate::models::{CorrelationResult, MaritimeThreat, NewsThreat, WeatherThreat};
use redis::AsyncCommands;

pub async fn compute_correlation(redis_url: &str) -> CorrelationResult {
    let mut risk_score = 0.5;
    let mut contributors = Vec::new();

    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            let maritime: Option<String> = conn.get("cache:maritime:latest").await.unwrap_or(None);
            if let Some(m) = maritime {
                if let Ok(v) = serde_json::from_str::<Vec<MaritimeThreat>>(&m) {
                    if v.iter().any(|x| x.risk_score > 0.7) {
                        risk_score += 0.15;
                        contributors.push("high_risk_vessel_detected".to_string());
                    }
                }
            }
            let news: Option<String> = conn.get("cache:news:latest").await.unwrap_or(None);
            if let Some(n) = news {
                if let Ok(v) = serde_json::from_str::<Vec<NewsThreat>>(&n) {
                    if v.iter().any(|x| x.severity == "high") {
                        risk_score += 0.2;
                        contributors.push("high_severity_news".to_string());
                    }
                }
            }
            let weather: Option<String> = conn.get("cache:weather:latest").await.unwrap_or(None);
            if let Some(w) = weather {
                if let Ok(v) = serde_json::from_str::<Vec<WeatherThreat>>(&w) {
                    if v.iter().any(|x| x.risk_score > 0.8) {
                        risk_score += 0.1;
                        contributors.push("adverse_weather_conditions".to_string());
                    }
                }
            }
        }
    }

    if contributors.is_empty() {
        contributors.push("baseline_activity".to_string());
    }

    let prompt = format!(
        "As an intelligence analyst, explain the current threat correlation for Bharat (India). Contributors: {}. Total risk score: {:.2}. Provide a 1-sentence executive summary.",
        contributors.join(", "),
        risk_score
    );

    let explanation = crate::services::llm::summarize_threat(
        redis_url,
        &format!("cache:llm:corr:{}", Utc::now().format("%Y%m%d%H")),
        &prompt
    ).await.unwrap_or_else(|e| {
        log::error!("LLM Correlation failed: {}", e);
        format!("Calculated correlation from {} factors.", contributors.len())
    });

    CorrelationResult {
        correlation_id: format!("corr-{}", Utc::now().timestamp()),
        risk_score: (risk_score as f64).min(1.0),
        explanation,
        top_contributors: contributors,
        timestamp: Utc::now().to_rfc3339(),
    }
}
