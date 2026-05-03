/// P5 — Dark vessel detection and vessel_alert_ratio tests.
use crate::providers::maritime::{compute_vessel_alert_ratio, detect_anomalies};
use crate::models::MaritimeThreat;

fn make_vessel(id: &str, risk: f64) -> MaritimeThreat {
    MaritimeThreat {
        vessel_id: id.to_string(),
        vessel_name: format!("VESSEL-{}", id),
        lat: 23.5,
        lon: 67.0,
        risk_score: risk,
        port: "Indian-Waters".to_string(),
        timestamp: "2026-05-01T00:00:00Z".to_string(),
    }
}

// ── vessel_alert_ratio ────────────────────────────────────────────────────────

#[test]
fn vessel_alert_ratio_zero_dark() {
    let ratio = compute_vessel_alert_ratio(0, 50);
    assert_eq!(ratio, 0.0);
}

#[test]
fn vessel_alert_ratio_normalized_at_20() {
    // 20 dark vessels → ratio = 1.0
    let ratio = compute_vessel_alert_ratio(20, 100);
    assert_eq!(ratio, 1.0);
}

#[test]
fn vessel_alert_ratio_capped_at_one() {
    // More than 20 dark vessels still caps at 1.0
    let ratio = compute_vessel_alert_ratio(50, 100);
    assert_eq!(ratio, 1.0);
}

#[test]
fn vessel_alert_ratio_partial() {
    // 10 dark vessels → 10/20 = 0.5
    let ratio = compute_vessel_alert_ratio(10, 100);
    assert!((ratio - 0.5).abs() < 1e-6, "expected 0.5, got {}", ratio);
}

#[test]
fn vessel_alert_ratio_never_negative() {
    let ratio = compute_vessel_alert_ratio(0, 0);
    assert!(ratio >= 0.0);
}

// ── Anomaly detection ─────────────────────────────────────────────────────────

#[test]
fn anomaly_detected_for_high_risk_vessel() {
    let vessel = make_vessel("MMSI-123456", 0.9);
    let anomaly = detect_anomalies(&vessel);
    assert!(anomaly.is_some(), "high risk vessel should trigger anomaly");
    let a = anomaly.unwrap();
    assert_eq!(a.mmsi, "MMSI-123456");
    assert!(a.severity >= 0.8);
}

#[test]
fn no_anomaly_for_low_risk_vessel() {
    let vessel = make_vessel("MMSI-999999", 0.3);
    let anomaly = detect_anomalies(&vessel);
    assert!(anomaly.is_none(), "low risk vessel should not trigger anomaly");
}

#[test]
fn anomaly_severity_matches_risk_score() {
    let vessel = make_vessel("MMSI-777777", 0.95);
    let anomaly = detect_anomalies(&vessel).unwrap();
    assert!(
        (anomaly.severity - 0.95).abs() < 1e-6,
        "severity should match risk_score"
    );
}

// ── Fusion integration: dark vessels raise score ──────────────────────────────

#[test]
fn dark_vessels_raise_fusion_score() {
    use crate::services::fusion::{compute_fusion, FusionInput};

    let no_dark = compute_fusion(&FusionInput {
        vessel_alert_ratio: 0.0,
        providers_total: 6,
        providers_available: 3,
        ..Default::default()
    });

    let with_dark = compute_fusion(&FusionInput {
        vessel_alert_ratio: 1.0,
        providers_total: 6,
        providers_available: 3,
        ..Default::default()
    });

    assert!(
        with_dark.score > no_dark.score,
        "dark vessels should raise fusion score: {} vs {}",
        with_dark.score,
        no_dark.score
    );
}
