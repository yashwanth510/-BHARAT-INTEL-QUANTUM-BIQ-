use std::cmp::Ordering;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Medium = 1,
    High = 2,
}

impl Priority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Priority::High => "high",
            Priority::Medium => "medium",
            Priority::Low => "low",
        }
    }

    pub fn from_score(score: f64) -> Self {
        if score >= 0.7 {
            Priority::High
        } else if score >= 0.4 {
            Priority::Medium
        } else {
            Priority::Low
        }
    }
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Low
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Classify query priority based on keywords
pub fn classify_priority(query: &str) -> Priority {
    let lower = query.to_lowercase();

    // HIGH priority keywords
    let high_keywords = [
        "military", "attack", "incursion", "invasion", "war", "conflict",
        "terror", "terrorist", "terrorism", "bomb", "explosion", "casualty",
        "casualties", "killed", "dead", "hostage", "kidnap", "abduction",
        "missile", "rocket", "artillery", "artillery fire", "gunfire",
        "nuclear", "chemical", "biological", "weapon", "weapons",
        "anomaly", "suspicious", "unidentified", "unknown",
        "emergency", "urgent", "critical", "alert",
    ];

    for keyword in &high_keywords {
        if lower.contains(keyword) {
            return Priority::High;
        }
    }

    // MEDIUM priority keywords
    let medium_keywords = [
        "border", "patrol", "deployment", "mobilization", "exercise",
        "drill", "surveillance", "reconnaissance", "intelligence",
        "economic", "trade", "sanction", "embargo",
        "weather", "storm", "cyclone", "flood",
    ];

    for keyword in &medium_keywords {
        if lower.contains(keyword) {
            return Priority::Medium;
        }
    }

    Priority::Low
}

/// Get priority boost for anomaly detection
pub fn anomaly_priority_boost(anomaly_score: f64) -> Priority {
    if anomaly_score > 0.8 {
        Priority::High
    } else if anomaly_score > 0.5 {
        Priority::Medium
    } else {
        Priority::Low
    }
}

/// Check if priority triggers LLM processing
pub fn should_trigger_llm(priority: Priority, is_new: bool) -> bool {
    match priority {
        Priority::High => true,  // Always trigger for high priority
        Priority::Medium => is_new, // Only for new events
        Priority::Low => false,   // Skip for low priority
    }
}

/// Get UI color for priority
pub fn priority_color(priority: Priority) -> &'static str {
    match priority {
        Priority::High => "#DC2626",    // Red
        Priority::Medium => "#F59E0B",  // Amber
        Priority::Low => "#10B981",     // Green
    }
}

/// Convert priority to numeric score for fusion
pub fn priority_to_score(priority: Priority) -> f64 {
    match priority {
        Priority::High => 1.0,
        Priority::Medium => 0.5,
        Priority::Low => 0.25,
    }
}
