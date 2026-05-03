use std::env;
use std::sync::atomic::{AtomicBool, Ordering};

static ALERT_MODE_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Check if auto alert mode is enabled
pub fn auto_alert_enabled() -> bool {
    env::var("AUTO_ALERT_MODE")
        .unwrap_or_else(|_| "false".to_string())
        == "true"
}

/// Get alert threshold from environment
pub fn get_alert_threshold() -> f64 {
    env::var("ALERT_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.7)
}

/// Check if fusion score triggers alert mode
pub fn should_trigger_alert(fusion_score: f64) -> bool {
    if !auto_alert_enabled() {
        return false;
    }
    fusion_score >= get_alert_threshold()
}

/// Activate alert mode
pub fn activate_alert_mode() {
    ALERT_MODE_ACTIVE.store(true, Ordering::Relaxed);
    log::warn!("[ALERT_MODE] Activated - increasing polling frequency");
}

/// Deactivate alert mode
pub fn deactivate_alert_mode() {
    ALERT_MODE_ACTIVE.store(false, Ordering::Relaxed);
    log::info!("[ALERT_MODE] Deactivated - normal polling resumed");
}

/// Check current alert mode status
pub fn is_alert_mode_active() -> bool {
    ALERT_MODE_ACTIVE.load(Ordering::Relaxed)
}

/// Get polling multiplier based on alert mode
pub fn get_polling_multiplier() -> u64 {
    if is_alert_mode_active() {
        env::var("ACTIVE_ZONE_MULTIPLIER")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2)
    } else {
        1
    }
}

/// Calculate adjusted poll interval
pub fn adjusted_poll_interval(base_seconds: u64) -> u64 {
    if is_alert_mode_active() {
        base_seconds / get_polling_multiplier().max(1)
    } else {
        base_seconds
    }
}

/// Alert mode configuration
#[derive(Debug, Clone)]
pub struct AlertConfig {
    pub enabled: bool,
    pub threshold: f64,
    pub multiplier: u64,
}

impl AlertConfig {
    pub fn from_env() -> Self {
        Self {
            enabled: auto_alert_enabled(),
            threshold: get_alert_threshold(),
            multiplier: env::var("ACTIVE_ZONE_MULTIPLIER")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2),
        }
    }

    pub fn check_and_update(&self, fusion_score: f64) {
        if !self.enabled {
            return;
        }

        let currently_active = is_alert_mode_active();
        let should_activate = fusion_score >= self.threshold;

        match (currently_active, should_activate) {
            (false, true) => activate_alert_mode(),
            (true, false) => deactivate_alert_mode(),
            _ => {}
        }
    }
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
