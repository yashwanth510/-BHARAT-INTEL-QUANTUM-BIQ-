use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryType {
    Maritime,
    Weather,
    Satellite,
    News,
    Crypto,
    General,
}

/// Classify query intent based on keywords
pub fn classify_query(q: &str) -> QueryType {
    let lower = q.to_lowercase();
    
    // Maritime indicators
    if lower.contains("vessel") 
        || lower.contains("ship") 
        || lower.contains("navy") 
        || lower.contains("carrier")
        || lower.contains("submarine")
        || lower.contains("fleet")
        || lower.contains("maritime")
        || lower.contains("ais")
        || lower.contains("port")
        || lower.contains("harbor")
        || lower.contains("sea")
        || lower.contains("ocean")
    {
        return QueryType::Maritime;
    }

    // Weather indicators
    if lower.contains("weather") 
        || lower.contains("storm") 
        || lower.contains("cyclone")
        || lower.contains("typhoon")
        || lower.contains("flood")
        || lower.contains(" rain ") || lower.starts_with("rain ") || lower.ends_with(" rain") || lower == "rain"
        || lower.contains(" wind ") || lower.starts_with("wind ") || lower.ends_with(" wind") || lower == "wind"
        || lower.contains("temperature")
        || lower.contains("climate")
        || lower.contains("visibility")
    {
        return QueryType::Weather;
    }

    // Satellite indicators
    if lower.contains("satellite")
        || lower.contains("imagery")
        || lower.contains("copernicus")
        || lower.contains("sentinel")
        || lower.contains("radar")
        || lower.contains("reconnaissance")
        || lower.contains("surveillance")
        || lower.contains("thermal")
        || lower.contains("infrared")
    {
        return QueryType::Satellite;
    }

    // Crypto/financial indicators
    if lower.contains("crypto")
        || lower.contains("wallet")
        || lower.contains("bitcoin")
        || lower.contains("sanction")
        || lower.contains("ofac")
        || lower.contains("blockchain")
        || lower.contains("transaction")
        || lower.contains("terror financing")
    {
        return QueryType::Crypto;
    }

    // News indicators (default for border, military, etc)
    if lower.contains("border")
        || lower.contains("incursion")
        || lower.contains("attack")
        || lower.contains("conflict")
        || lower.contains("war")
        || lower.contains("military")
        || lower.contains("army")
        || lower.contains("terror")
        || lower.contains("threat")
        || lower.contains("news")
        || lower.contains("latest")
    {
        return QueryType::News;
    }

    // Default
    QueryType::General
}

/// Check if query routing is enabled
pub fn routing_enabled() -> bool {
    env::var("QUERY_ROUTING_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        == "true"
}

/// Get relevant providers for query type
pub fn get_providers_for_query(query_type: QueryType) -> Vec<&'static str> {
    match query_type {
        QueryType::Maritime => vec!["aisstream", "maritime"],
        QueryType::Weather => vec!["openweather", "weather"],
        QueryType::Satellite => vec!["sentinel", "satellite"],
        QueryType::News => vec!["newsapi", "tavily", "osint"],
        QueryType::Crypto => vec!["ofac", "misttrack", "crypto"],
        QueryType::General => vec!["newsapi", "tavily", "openweather"],
    }
}
