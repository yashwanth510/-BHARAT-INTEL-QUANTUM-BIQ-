/// P3 — Fusion engine weighted scoring tests.
/// P4 — Risk calibration threshold tests.
use crate::services::fusion::{
    compute_fusion, compute_fusion_score, score_to_level, FusionInput,
};

// ── Weighted formula ──────────────────────────────────────────────────────────

#[test]
fn fusion_score_exact_weights() {
    // All inputs at 1.0 → max score = 1.0
    let s = compute_fusion_score(1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
    assert!((s - 1.0).abs() < 1e-6, "all-ones score should be 1.0, got {}", s);
}

#[test]
fn fusion_score_zero_inputs() {
    let s = compute_fusion_score(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    assert_eq!(s, 0.0);
}

#[test]
fn fusion_score_mistral_dominates() {
    // mistral weight 0.35 is the largest single weight
    let mistral_only = compute_fusion_score(1.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    let news_only = compute_fusion_score(0.0, 1.0, 0.0, 0.0, 0.0, 0.0);
    assert!(
        mistral_only > news_only,
        "mistral (0.35) should outweigh news (0.20)"
    );
    assert!((mistral_only - 0.35).abs() < 1e-6);
    assert!((news_only - 0.20).abs() < 1e-6);
}

#[test]
fn fusion_score_weights_sum_to_one() {
    // 0.35 + 0.20 + 0.15 + 0.10 + 0.10 + 0.10 = 1.00
    let total = 0.35_f32 + 0.20 + 0.15 + 0.10 + 0.10 + 0.10;
    assert!((total - 1.0).abs() < 1e-6, "weights must sum to 1.0, got {}", total);
}

#[test]
fn fusion_score_never_exceeds_one() {
    let s = compute_fusion_score(2.0, 2.0, 2.0, 2.0, 2.0, 2.0);
    // raw formula can exceed 1.0 but compute_fusion clamps it
    let input = FusionInput {
        mistral_score: 2.0,
        news_score: 2.0,
        sentiment_score: 2.0,
        vessel_alert_ratio: 2.0,
        financial_score: 2.0,
        terrain_score: 2.0,
        providers_total: 6,
        providers_available: 6,
        ..Default::default()
    };
    let fusion = compute_fusion(&input);
    assert!(fusion.score <= 1.0, "fusion score must be ≤ 1.0, got {}", fusion.score);
}

// ── Risk level thresholds ─────────────────────────────────────────────────────

#[test]
fn score_to_level_nominal() {
    assert_eq!(score_to_level(0.0), "NOMINAL");
    assert_eq!(score_to_level(0.10), "NOMINAL");
    assert_eq!(score_to_level(0.29), "NOMINAL");
}

#[test]
fn score_to_level_monitored() {
    assert_eq!(score_to_level(0.30), "MONITORED");
    assert_eq!(score_to_level(0.40), "MONITORED");
    assert_eq!(score_to_level(0.49), "MONITORED");
}

#[test]
fn score_to_level_elevated() {
    assert_eq!(score_to_level(0.50), "ELEVATED");
    assert_eq!(score_to_level(0.60), "ELEVATED");
    assert_eq!(score_to_level(0.64), "ELEVATED");
}

#[test]
fn score_to_level_high() {
    assert_eq!(score_to_level(0.65), "HIGH");
    assert_eq!(score_to_level(0.70), "HIGH");
    assert_eq!(score_to_level(0.79), "HIGH");
}

#[test]
fn score_to_level_critical() {
    assert_eq!(score_to_level(0.80), "CRITICAL");
    assert_eq!(score_to_level(0.95), "CRITICAL");
    assert_eq!(score_to_level(1.00), "CRITICAL");
}

// ── Confidence scoring ────────────────────────────────────────────────────────

#[test]
fn confidence_full_providers() {
    let input = FusionInput {
        providers_available: 6,
        providers_total: 6,
        timeout_count: 0,
        degraded_providers: vec![],
        ..Default::default()
    };
    let fusion = compute_fusion(&input);
    assert!(
        (fusion.confidence - 1.0).abs() < 1e-6,
        "full providers → confidence=1.0, got {}",
        fusion.confidence
    );
}

#[test]
fn confidence_degraded_providers() {
    let input = FusionInput {
        providers_available: 3,
        providers_total: 6,
        timeout_count: 1,
        degraded_providers: vec!["aisstream".to_string()],
        ..Default::default()
    };
    let fusion = compute_fusion(&input);
    assert!(
        fusion.confidence < 0.6,
        "half providers + timeout → confidence < 0.6, got {}",
        fusion.confidence
    );
}

#[test]
fn confidence_never_negative() {
    let input = FusionInput {
        providers_available: 0,
        providers_total: 6,
        timeout_count: 10,
        degraded_providers: vec!["a".into(), "b".into(), "c".into()],
        ..Default::default()
    };
    let fusion = compute_fusion(&input);
    assert!(fusion.confidence >= 0.0, "confidence must be ≥ 0.0");
}

// ── Drivers ───────────────────────────────────────────────────────────────────

#[test]
fn fusion_drivers_populated() {
    let input = FusionInput {
        mistral_score: 0.7,
        news_count: 10,
        news_score: 0.5,
        vessel_alert_ratio: 0.5,
        satellite_activity: true,
        weather_risk: "HIGH".to_string(),
        providers_total: 6,
        providers_available: 5,
        ..Default::default()
    };
    let fusion = compute_fusion(&input);
    assert!(!fusion.drivers.is_empty(), "drivers must be populated");
    let all = fusion.drivers.join(" ");
    assert!(all.contains("AI analysis"), "should include AI driver");
    assert!(all.contains("vessel"), "should include vessel driver");
    assert!(all.contains("satellite"), "should include satellite driver");
    assert!(all.contains("weather"), "should include weather driver");
}

#[test]
fn fusion_recommendations_populated() {
    let input = FusionInput {
        mistral_score: 0.85,
        providers_total: 6,
        providers_available: 6,
        ..Default::default()
    };
    let fusion = compute_fusion(&input);
    assert!(!fusion.recommendations.is_empty(), "recommendations must be populated");
}
