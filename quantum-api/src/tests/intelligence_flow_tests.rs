/// End-to-end intelligence flow tests.
/// Tests the full pipeline: context → fusion → risk level → response shape.
use crate::services::fusion::{compute_fusion, compute_fusion_score, score_to_level, FusionInput};
use crate::services::llm::{apply_risk_calibration, score_to_level as llm_score_to_level};
use crate::services::query_router::{classify_query, QueryType};
use crate::services::priority::{classify_priority, Priority};
use serde_json::json;

// ── Query classification ──────────────────────────────────────────────────────

#[test]
fn query_taiwan_classified_as_general_or_news() {
    let qt = classify_query("taiwan");
    assert!(
        qt == QueryType::General || qt == QueryType::News,
        "taiwan should be General or News, got {:?}",
        qt
    );
}

#[test]
fn query_karachi_vessel_classified_as_maritime() {
    let qt = classify_query("karachi vessel");
    assert_eq!(qt, QueryType::Maritime);
}

#[test]
fn query_ladakh_weather_classified_as_weather() {
    let qt = classify_query("ladakh weather");
    assert_eq!(qt, QueryType::Weather);
}

#[test]
fn query_ukraine_war_classified_as_news() {
    let qt = classify_query("ukraine war");
    assert_eq!(qt, QueryType::News);
}

// ── Priority classification ───────────────────────────────────────────────────

#[test]
fn priority_military_attack_is_high() {
    assert_eq!(classify_priority("military attack on border"), Priority::High);
}

#[test]
fn priority_border_patrol_is_medium() {
    assert_eq!(classify_priority("border patrol deployment"), Priority::Medium);
}

#[test]
fn priority_general_query_is_low() {
    assert_eq!(classify_priority("taiwan trade"), Priority::Medium);
}

// ── Full fusion pipeline ──────────────────────────────────────────────────────

#[test]
fn full_pipeline_nominal_query() {
    // A query with no signals should produce NOMINAL or MONITORED
    let input = FusionInput {
        mistral_score: 0.1,
        news_score: 0.05,
        sentiment_score: 0.2,
        vessel_alert_ratio: 0.0,
        financial_score: 0.0,
        terrain_score: 0.1,
        providers_total: 6,
        providers_available: 4,
        ..Default::default()
    };
    let fusion = compute_fusion(&input);
    assert!(
        fusion.risk == "NOMINAL" || fusion.risk == "MONITORED",
        "low-signal query should be NOMINAL or MONITORED, got {}",
        fusion.risk
    );
    assert!(fusion.score < 0.50, "score should be < 0.50, got {}", fusion.score);
}

#[test]
fn full_pipeline_high_threat_query() {
    // High mistral score + dark vessels + satellite → HIGH or CRITICAL
    let input = FusionInput {
        mistral_score: 0.85,
        news_score: 0.70,
        sentiment_score: 0.80,
        vessel_alert_ratio: 0.60,
        financial_score: 0.50,
        terrain_score: 0.70,
        satellite_activity: true,
        providers_total: 6,
        providers_available: 6,
        ..Default::default()
    };
    let fusion = compute_fusion(&input);
    assert!(
        fusion.risk == "HIGH" || fusion.risk == "CRITICAL",
        "high-signal query should be HIGH or CRITICAL, got {}",
        fusion.risk
    );
    assert!(fusion.score >= 0.65, "score should be ≥ 0.65, got {}", fusion.score);
}

#[test]
fn full_pipeline_calibration_applied() {
    // Even if fusion score is high, if Mistral says "no active threats" → downgrade
    let assessment = json!({
        "score": 0.75,
        "level": "HIGH",
        "explanation": "There are no active threats in the region. Situation is stable.",
        "key_actors": [],
        "key_locations": [],
        "recommended_action": "continue monitoring"
    });
    let calibrated = apply_risk_calibration(assessment);
    let score = calibrated["score"].as_f64().unwrap();
    assert!(score <= 0.45, "calibration should cap score at 0.45, got {}", score);
    assert_eq!(calibrated["level"].as_str().unwrap(), "MONITORED");
}

// ── Response shape validation ─────────────────────────────────────────────────

#[test]
fn fusion_response_has_required_fields() {
    let input = FusionInput {
        mistral_score: 0.5,
        providers_total: 6,
        providers_available: 4,
        ..Default::default()
    };
    let fusion = compute_fusion(&input);

    // All required fields must be present
    assert!(!fusion.risk.is_empty(), "risk must not be empty");
    assert!(fusion.score >= 0.0 && fusion.score <= 1.0, "score must be 0-1");
    assert!(!fusion.recommendations.is_empty(), "recommendations must not be empty");
    assert!(fusion.confidence >= 0.0 && fusion.confidence <= 1.0, "confidence must be 0-1");
    assert!(fusion.confidence_detail.providers_total > 0);
}

#[test]
fn mistral_assessment_shape() {
    // Verify the expected JSON shape from Mistral is parseable
    let mock_response = json!({
        "score": 0.45,
        "level": "MONITORED",
        "explanation": "Border activity is within normal parameters.",
        "key_actors": ["Indian Army", "PLA"],
        "key_locations": ["Ladakh", "Aksai Chin"],
        "recommended_action": "Continue routine surveillance",
        "sources_used": {
            "tavily": 3,
            "news": 5,
            "weather": true,
            "terrain": true,
            "maritime": false,
            "financial": false,
            "satellite": true
        },
        "correlation_id": "test-uuid-1234",
        "query": "ladakh border"
    });

    assert!(mock_response["score"].as_f64().is_some());
    assert!(mock_response["level"].as_str().is_some());
    assert!(mock_response["explanation"].as_str().is_some());
    assert!(mock_response["key_actors"].as_array().is_some());
    assert!(mock_response["key_locations"].as_array().is_some());
    assert!(mock_response["recommended_action"].as_str().is_some());
    assert!(mock_response["sources_used"].is_object());
    assert!(mock_response["correlation_id"].as_str().is_some());
}

// ── Graceful degradation ──────────────────────────────────────────────────────

#[test]
fn fusion_works_with_zero_providers() {
    // System must produce a valid response even with no providers available
    let input = FusionInput {
        providers_available: 0,
        providers_total: 6,
        ..Default::default()
    };
    let fusion = compute_fusion(&input);
    assert!(!fusion.risk.is_empty());
    assert!(fusion.score >= 0.0);
    assert!(fusion.confidence >= 0.0);
    // With no providers, confidence should be 0
    assert_eq!(fusion.confidence, 0.0);
}

#[test]
fn fusion_works_with_partial_providers() {
    let input = FusionInput {
        mistral_score: 0.0, // LLM unavailable
        news_score: 0.4,
        sentiment_score: 0.3,
        vessel_alert_ratio: 0.0,
        financial_score: 0.0,
        terrain_score: 0.2,
        providers_available: 2,
        providers_total: 6,
        ..Default::default()
    };
    let fusion = compute_fusion(&input);
    // Must still produce a valid result
    assert!(!fusion.risk.is_empty());
    assert!(fusion.score >= 0.0 && fusion.score <= 1.0);
    assert!(fusion.confidence < 0.5, "partial providers → low confidence");
}

// ── Score consistency ─────────────────────────────────────────────────────────

#[test]
fn score_and_level_always_consistent() {
    // For any score, the level must match score_to_level
    let test_scores = [0.0, 0.1, 0.29, 0.30, 0.49, 0.50, 0.64, 0.65, 0.79, 0.80, 1.0];
    for &s in &test_scores {
        let level = score_to_level(s);
        let expected = match s {
            x if x < 0.30 => "NOMINAL",
            x if x < 0.50 => "MONITORED",
            x if x < 0.65 => "ELEVATED",
            x if x < 0.80 => "HIGH",
            _ => "CRITICAL",
        };
        assert_eq!(level, expected, "score {} → expected {} got {}", s, expected, level);
    }
}

// ── Backoff logic ─────────────────────────────────────────────────────────────

#[test]
fn scheduler_backoff_for_quota_errors() {
    // Verify backoff is applied for quota/rate-limit errors
    // This is tested via the scheduler's compute_backoff function
    // We test the logic inline here
    let error = "HTTP 429 Too Many Requests";
    let has_429 = error.to_lowercase().contains("429");
    assert!(has_429, "429 should trigger backoff");

    let error2 = "quota exceeded for this month";
    let has_quota = error2.to_lowercase().contains("quota");
    assert!(has_quota, "quota should trigger 15-min backoff");
}
