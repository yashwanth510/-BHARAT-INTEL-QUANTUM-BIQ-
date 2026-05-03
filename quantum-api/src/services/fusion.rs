/// PRIORITY 3 — Fusion Engine Rewrite: deterministic weighted scoring.
/// PRIORITY 4 — Risk calibration with strict thresholds.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fusion {
    pub score: f64,
    pub risk: String,
    pub drivers: Vec<String>,
    pub recommendations: Vec<String>,
    pub confidence: f64,
    pub confidence_detail: ConfidenceDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceDetail {
    pub providers_available: u8,
    pub providers_total: u8,
    pub timeout_count: u8,
    pub degraded_providers: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct FusionInput {
    // Weighted inputs (P3 exact weights)
    pub mistral_score: f32,       // weight 0.35
    pub news_score: f32,          // weight 0.20
    pub sentiment_score: f32,     // weight 0.15
    pub vessel_alert_ratio: f32,  // weight 0.10
    pub financial_score: f32,     // weight 0.10
    pub terrain_score: f32,       // weight 0.10

    // Legacy fields kept for backward compat with existing callers
    pub news_count: usize,
    pub maritime_anomaly: bool,
    pub weather_risk: String,
    pub satellite_activity: bool,
    pub llm_insights: bool,

    // Confidence tracking
    pub providers_available: u8,
    pub providers_total: u8,
    pub timeout_count: u8,
    pub degraded_providers: Vec<String>,
}

impl FusionInput {
    pub fn new() -> Self {
        Self {
            providers_total: 6,
            ..Default::default()
        }
    }
}

/// PRIORITY 3 — Exact weighted fusion formula.
pub fn compute_fusion_score(
    mistral_score: f32,
    news_score: f32,
    sentiment_score: f32,
    vessel_alert_ratio: f32,
    financial_score: f32,
    terrain_score: f32,
) -> f32 {
    (mistral_score * 0.35)
        + (news_score * 0.20)
        + (sentiment_score * 0.15)
        + (vessel_alert_ratio * 0.10)
        + (financial_score * 0.10)
        + (terrain_score * 0.10)
}

/// PRIORITY 4 — Strict threshold mapping.
pub fn score_to_level(score: f32) -> &'static str {
    match score {
        s if s < 0.30 => "NOMINAL",
        s if s < 0.50 => "MONITORED",
        s if s < 0.65 => "ELEVATED",
        s if s < 0.80 => "HIGH",
        _ => "CRITICAL",
    }
}

/// Compute confidence score based on provider availability.
pub fn compute_confidence(input: &FusionInput) -> f64 {
    let total = input.providers_total.max(1) as f64;
    let available = input.providers_available as f64;
    let timeout_penalty = input.timeout_count as f64 * 0.05;
    let degraded_penalty = input.degraded_providers.len() as f64 * 0.03;

    ((available / total) - timeout_penalty - degraded_penalty).max(0.0).min(1.0)
}

pub fn compute_fusion(input: &FusionInput) -> Fusion {
    let mut drivers: Vec<String> = Vec::new();

    // --- Derive scores from legacy fields if weighted fields are zero ---
    let mistral_score = if input.mistral_score > 0.0 {
        input.mistral_score
    } else if input.llm_insights {
        0.5_f32
    } else {
        0.0_f32
    };

    let news_score = if input.news_score > 0.0 {
        input.news_score
    } else {
        (input.news_count as f32 * 0.05).min(1.0)
    };

    let sentiment_score = if input.sentiment_score > 0.0 {
        input.sentiment_score
    } else {
        0.3_f32 // neutral baseline
    };

    let vessel_alert_ratio = if input.vessel_alert_ratio > 0.0 {
        input.vessel_alert_ratio
    } else if input.maritime_anomaly {
        1.0_f32
    } else {
        0.0_f32
    };

    let financial_score = if input.financial_score > 0.0 {
        input.financial_score
    } else {
        0.0_f32
    };

    let terrain_score = if input.terrain_score > 0.0 {
        input.terrain_score
    } else {
        // derive from weather_risk as proxy
        match input.weather_risk.as_str() {
            "HIGH" | "CRITICAL" => 0.7_f32,
            "MEDIUM" | "MODERATE" => 0.4_f32,
            _ => 0.1_f32,
        }
    };

    // --- Collect drivers ---
    if mistral_score > 0.0 {
        drivers.push(format!("AI analysis (score={:.2})", mistral_score));
    }
    if news_score > 0.2 {
        drivers.push(format!("news activity ({} articles)", input.news_count));
    }
    if vessel_alert_ratio > 0.0 {
        drivers.push("vessel anomaly".to_string());
    }
    if financial_score > 0.3 {
        drivers.push("financial threat detected".to_string());
    }
    if terrain_score > 0.5 {
        drivers.push("terrain sensitivity".to_string());
    }
    if input.satellite_activity {
        drivers.push("satellite imagery activity".to_string());
    }
    match input.weather_risk.as_str() {
        "HIGH" | "CRITICAL" => drivers.push("extreme weather conditions".to_string()),
        "MEDIUM" | "MODERATE" => drivers.push("moderate weather conditions".to_string()),
        _ => {}
    }

    // --- Weighted score ---
    let raw_score = compute_fusion_score(
        mistral_score,
        news_score,
        sentiment_score,
        vessel_alert_ratio,
        financial_score,
        terrain_score,
    );
    let score = raw_score.min(1.0) as f64;

    // --- Risk level ---
    let risk = score_to_level(score as f32).to_string();

    // --- Confidence ---
    let confidence = compute_confidence(input);
    let confidence_detail = ConfidenceDetail {
        providers_available: input.providers_available,
        providers_total: input.providers_total,
        timeout_count: input.timeout_count,
        degraded_providers: input.degraded_providers.clone(),
    };

    // --- Recommendations ---
    let recommendations = generate_recommendations(&risk, &drivers);

    Fusion {
        score,
        risk,
        drivers,
        recommendations,
        confidence,
        confidence_detail,
    }
}

fn generate_recommendations(risk: &str, drivers: &[String]) -> Vec<String> {
    let mut recs = Vec::new();

    match risk {
        "CRITICAL" => {
            recs.push("Immediate escalation to command required".to_string());
            recs.push("Activate full alert mode monitoring".to_string());
            recs.push("Deploy rapid response assets".to_string());
        }
        "HIGH" => {
            recs.push("Immediate attention required".to_string());
            recs.push("Activate alert mode monitoring".to_string());
            recs.push("Increase surveillance frequency".to_string());
        }
        "ELEVATED" => {
            recs.push("Monitor situation closely".to_string());
            recs.push("Prepare contingency plans".to_string());
            recs.push("Brief command on developing situation".to_string());
        }
        "MONITORED" => {
            recs.push("Continue enhanced monitoring".to_string());
            recs.push("Review available intelligence sources".to_string());
        }
        _ => {
            recs.push("Continue routine monitoring".to_string());
        }
    }

    // Driver-specific recommendations
    for driver in drivers {
        if driver.contains("vessel anomaly") {
            recs.push("Track vessel movement patterns".to_string());
        }
        if driver.contains("weather") {
            recs.push("Review operational weather thresholds".to_string());
        }
        if driver.contains("satellite") {
            recs.push("Analyze recent imagery changes".to_string());
        }
        if driver.contains("financial") {
            recs.push("Escalate to financial intelligence unit".to_string());
        }
        if driver.contains("cross-border") {
            recs.push("Coordinate with border security forces".to_string());
        }
    }

    recs
}
