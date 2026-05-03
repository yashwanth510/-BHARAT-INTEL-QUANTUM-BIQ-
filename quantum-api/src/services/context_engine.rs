use super::geo_resolver::{GeoPoint, get_location, get_default_zones};
use super::priority::{Priority, classify_priority};
use super::query_router::{QueryType, classify_query, routing_enabled};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub location: Option<GeoPoint>,
    pub intent: String,
    pub priority: Priority,
    pub query_type: QueryType,
    pub keywords: Vec<String>,
    pub radius_km: f64,
}

impl Context {
    pub fn new() -> Self {
        Self {
            location: None,
            intent: String::new(),
            priority: Priority::Low,
            query_type: QueryType::General,
            keywords: Vec::new(),
            radius_km: 50.0,
        }
    }

    pub fn with_location(mut self, location: GeoPoint) -> Self {
        self.location = Some(location);
        self
    }

    pub fn with_intent(mut self, intent: &str) -> Self {
        self.intent = intent.to_string();
        self
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_radius(mut self, radius: f64) -> Self {
        self.radius_km = radius;
        self
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

/// Build context from user query and background data
pub async fn build_context(query: &str, _background_data: Option<&serde_json::Value>) -> Context {
    let mut context = Context::new();

    // Extract location from query
    let location_keywords = extract_location_keywords(query);
    
    // Try to resolve location
    if let Some(loc) = get_location(query).await {
        context.location = Some(loc);
    } else if let Some(first_loc) = location_keywords.first() {
        if let Some(loc) = get_location(first_loc).await {
            context.location = Some(loc);
        }
    }

    // If no location found and not coordinates, use default zones
    if context.location.is_none() && !query.contains(',') {
        let defaults = get_default_zones();
        if let Some(first) = defaults.first() {
            context.location = Some(first.clone());
        }
    }

    // Classify query type
    context.query_type = classify_query(query);
    context.intent = format!("{:?}", context.query_type);

    // Classify priority
    context.priority = classify_priority(query);

    // Extract keywords
    context.keywords = extract_keywords(query);

    context
}

/// Extract location keywords from query
fn extract_location_keywords(query: &str) -> Vec<String> {
    let known_locations: HashMap<&str, &str> = [
        ("ladakh", "Ladakh"),
        ("kargil", "Kargil"),
        ("siachen", "Siachen"),
        ("karachi", "Karachi"),
        ("lahore", "Lahore"),
        ("islamabad", "Islamabad"),
        ("beijing", "Beijing"),
        ("taiwan", "Taiwan"),
        ("taipei", "Taipei"),
        ("mumbai", "Mumbai"),
        ("delhi", "Delhi"),
        ("srinagar", "Srinagar"),
        ("leh", "Leh"),
        ("gilgit", "Gilgit"),
        ("skardu", "Skardu"),
    ]
    .into_iter()
    .collect();

    let lower = query.to_lowercase();
    let mut found = Vec::new();

    for (key, value) in known_locations {
        if lower.contains(key) {
            found.push(value.to_string());
        }
    }

    found
}

/// Extract relevant keywords from query
fn extract_keywords(query: &str) -> Vec<String> {
    let stop_words = ["the", "a", "an", "in", "on", "at", "to", "for", "of", "and", "or", "is", "are"];
    
    query
        .split_whitespace()
        .map(|w| w.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|w| !w.is_empty() && !stop_words.contains(&w.as_str()) && w.len() > 2)
        .take(10)
        .collect()
}

/// Merge query context with background data from Kafka
pub fn merge_with_background(
    context: &mut Context,
    background: &serde_json::Value,
) {
    // Extract location from background if query didn't have one
    if context.location.is_none() {
        if let Some(loc) = background.get("location").and_then(|v| v.as_str()) {
            // This would need async, so we'll just store the intent
            context.intent = format!("{} (from background)", context.intent);
        }
    }

    // Elevate priority if background shows high-risk events
    if let Some(risk) = background.get("risk_score").and_then(|v| v.as_f64()) {
        if risk > 0.7 && context.priority < Priority::High {
            context.priority = Priority::High;
        }
    }
}

/// Get search radius based on query intent
pub fn get_search_radius(query_type: &QueryType) -> f64 {
    match query_type {
        QueryType::Maritime => 200.0,   // 200km for maritime
        QueryType::Satellite => 100.0,  // 100km for satellite
        QueryType::Weather => 50.0,     // 50km for weather
        QueryType::News => 100.0,       // 100km for news
        QueryType::Crypto => 0.0,       // No radius for crypto
        QueryType::General => 50.0,     // Default
    }
}
