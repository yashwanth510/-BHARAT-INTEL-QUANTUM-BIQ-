/// P4 — Risk calibration: downgrade logic and threshold correctness.
use crate::services::llm::{apply_risk_calibration, score_to_level};
use serde_json::json;

// ── Downgrade phrases ─────────────────────────────────────────────────────────

fn make_assessment(score: f64, explanation: &str) -> serde_json::Value {
    json!({
        "score": score,
        "level": score_to_level(score as f32),
        "explanation": explanation,
        "key_actors": [],
        "key_locations": [],
        "recommended_action": "monitor"
    })
}

#[test]
fn downgrade_no_direct_indicators() {
    let val = make_assessment(0.70, "There are no direct indicators of military activity.");
    let result = apply_risk_calibration(val);
    let score = result["score"].as_f64().unwrap();
    assert!(score <= 0.45, "score should be capped at 0.45, got {}", score);
    assert_eq!(result["level"].as_str().unwrap(), "MONITORED");
}

#[test]
fn downgrade_no_immediate_security_concerns() {
    let val = make_assessment(0.65, "Analysis shows no immediate security concerns in the region.");
    let result = apply_risk_calibration(val);
    let score = result["score"].as_f64().unwrap();
    assert!(score <= 0.45, "score should be capped at 0.45, got {}", score);
}

#[test]
fn downgrade_no_active_threats() {
    let val = make_assessment(0.75, "Currently there are no active threats detected.");
    let result = apply_risk_calibration(val);
    let score = result["score"].as_f64().unwrap();
    assert!(score <= 0.45);
}

#[test]
fn downgrade_normal_activity() {
    let val = make_assessment(0.60, "Situation reflects normal activity with no escalation.");
    let result = apply_risk_calibration(val);
    let score = result["score"].as_f64().unwrap();
    assert!(score <= 0.45);
}

#[test]
fn downgrade_no_significant() {
    let val = make_assessment(0.55, "No significant developments observed in the past 24 hours.");
    let result = apply_risk_calibration(val);
    let score = result["score"].as_f64().unwrap();
    assert!(score <= 0.45);
}

#[test]
fn no_downgrade_for_real_threat() {
    let val = make_assessment(0.75, "Military buildup detected near border with cross-border incursion risk.");
    let result = apply_risk_calibration(val);
    let score = result["score"].as_f64().unwrap();
    assert!(
        (score - 0.75).abs() < 1e-6,
        "real threat should not be downgraded, got {}",
        score
    );
    assert_eq!(result["level"].as_str().unwrap(), "HIGH");
}

#[test]
fn no_downgrade_for_critical_threat() {
    let val = make_assessment(0.90, "Active missile launch detected. Immediate response required.");
    let result = apply_risk_calibration(val);
    let score = result["score"].as_f64().unwrap();
    assert!(score >= 0.80, "critical threat must stay CRITICAL, got {}", score);
    assert_eq!(result["level"].as_str().unwrap(), "CRITICAL");
}

// ── Level consistency after calibration ──────────────────────────────────────

#[test]
fn level_matches_score_after_calibration() {
    // After calibration, level must always match the score
    let cases = vec![
        (0.80, "no direct indicators"),  // should downgrade to MONITORED
        (0.90, "active military threat"), // should stay CRITICAL
        (0.50, "normal activity"),        // should downgrade to MONITORED
    ];

    for (score, explanation) in cases {
        let val = make_assessment(score, explanation);
        let result = apply_risk_calibration(val);
        let final_score = result["score"].as_f64().unwrap() as f32;
        let final_level = result["level"].as_str().unwrap();
        let expected_level = score_to_level(final_score);
        assert_eq!(
            final_level, expected_level,
            "level mismatch for score={}: expected {} got {}",
            final_score, expected_level, final_level
        );
    }
}

// ── Boundary conditions ───────────────────────────────────────────────────────

#[test]
fn score_to_level_boundary_0_30() {
    // 0.30 is the boundary between NOMINAL and MONITORED
    assert_eq!(score_to_level(0.299), "NOMINAL");
    assert_eq!(score_to_level(0.300), "MONITORED");
}

#[test]
fn score_to_level_boundary_0_50() {
    assert_eq!(score_to_level(0.499), "MONITORED");
    assert_eq!(score_to_level(0.500), "ELEVATED");
}

#[test]
fn score_to_level_boundary_0_65() {
    assert_eq!(score_to_level(0.649), "ELEVATED");
    assert_eq!(score_to_level(0.650), "HIGH");
}

#[test]
fn score_to_level_boundary_0_80() {
    assert_eq!(score_to_level(0.799), "HIGH");
    assert_eq!(score_to_level(0.800), "CRITICAL");
}
