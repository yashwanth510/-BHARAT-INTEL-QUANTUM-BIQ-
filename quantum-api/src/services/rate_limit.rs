use chrono::Datelike;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Clone)]
pub struct ProviderLimits {
    pub mistral_max_per_day: u32,        // 50
    pub tavily_max_per_day: u32,         // 33 (1000/month)
    pub tavily_monthly_limit: u32,       // 1000
    pub newsapi_max_per_day: u32,        // 100 (free tier)
    pub weather_max_per_day: u32,        // 1000 (free tier)
    pub maritime_max_per_hour: u32,      // 100 (AISStream free tier)
    pub elevation_max_per_day: u32,      // 10000 (OpenTopoData fair use)
    pub misttrack_max_per_day: u32,      // 100
    pub sentinel_hub_monthly_limit: u32, // 30000
    pub twitter_monthly_limit: u32,      // 1000 (basic tier)
}

impl ProviderLimits {
    pub fn from_env() -> Self {
        Self {
            mistral_max_per_day: env::var("MISTRAL_MAX_PER_DAY").ok().and_then(|v| v.parse().ok()).unwrap_or(50),
            tavily_max_per_day: env::var("TAVILY_MAX_PER_DAY").ok().and_then(|v| v.parse().ok()).unwrap_or(33),
            tavily_monthly_limit: env::var("TAVILY_MONTHLY_LIMIT").ok().and_then(|v| v.parse().ok()).unwrap_or(1000),
            newsapi_max_per_day: env::var("NEWSAPI_MAX_PER_DAY").ok().and_then(|v| v.parse().ok()).unwrap_or(100),
            weather_max_per_day: env::var("OPENWEATHER_MAX_PER_DAY").ok().and_then(|v| v.parse().ok()).unwrap_or(1000),
            maritime_max_per_hour: env::var("MARITIME_MAX_PER_HOUR").ok().and_then(|v| v.parse().ok()).unwrap_or(100),
            elevation_max_per_day: env::var("ELEVATION_MAX_PER_DAY").ok().and_then(|v| v.parse().ok()).unwrap_or(10000),
            misttrack_max_per_day: env::var("MISTTRACK_MAX_PER_DAY").ok().and_then(|v| v.parse().ok()).unwrap_or(100),
            sentinel_hub_monthly_limit: env::var("SENTINEL_HUB_MAX_UNITS_PER_MONTH").ok().and_then(|v| v.parse().ok()).unwrap_or(30000),
            twitter_monthly_limit: env::var("TWITTER_MAX_TWEETS_PER_MONTH").ok().and_then(|v| v.parse().ok()).unwrap_or(1000),
        }
    }
}

pub async fn allow_with_redis(redis_url: &str, key: &str, limit: u32, ttl_seconds: u64) -> Result<bool, String> {
    let client = redis::Client::open(redis_url).map_err(|e| e.to_string())?;
    let mut conn = client.get_multiplexed_async_connection().await.map_err(|e| e.to_string())?;
    let count: u32 = conn.incr(key, 1_u32).await.map_err(|e| e.to_string())?;
    if count == 1 {
        let _: bool = conn.expire(key, ttl_seconds as i64).await.map_err(|e| e.to_string())?;
    }
    Ok(count <= limit)
}

pub async fn peek_quota(redis_url: &str, key: &str, limit: u32) -> Result<bool, String> {
    let client = redis::Client::open(redis_url).map_err(|e| e.to_string())?;
    let mut conn = client.get_multiplexed_async_connection().await.map_err(|e| e.to_string())?;
    let count: u32 = conn.get(key).await.unwrap_or(0);
    Ok(count < limit)
}

pub async fn increment_quota(redis_url: &str, key: &str, ttl_seconds: u64) -> Result<(), String> {
    let client = redis::Client::open(redis_url).map_err(|e| e.to_string())?;
    let mut conn = client.get_multiplexed_async_connection().await.map_err(|e| e.to_string())?;
    let count: u32 = conn.incr(key, 1_u32).await.map_err(|e| e.to_string())?;
    if count == 1 {
        let _: bool = conn.expire(key, ttl_seconds as i64).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Check monthly limit (resets on first day of month)
pub async fn allow_monthly_limit(redis_url: &str, key: &str, limit: u32) -> Result<bool, String> {
    // TEST_MODE: bypass all quotas
    let testing_mode = env::var("TEST_MODE").unwrap_or_else(|_| "false".to_string()) == "true";
    if testing_mode {
        return Ok(true);
    }

    let client = redis::Client::open(redis_url).map_err(|e| e.to_string())?;
    let mut conn = client.get_multiplexed_async_connection().await.map_err(|e| e.to_string())?;
    
    let current_month = chrono::Utc::now().format("%Y-%m").to_string();
    let monthly_key = format!("{}:{}", key, current_month);
    
    // Check if we need to reset (new month) - simplified: just use month key in counter
    // Keys will expire naturally at end of month
    
    // Increment the counter
    let count: u32 = conn.incr(&monthly_key, 1_u32).await.map_err(|e| e.to_string())?;
    
    // Set expiry to end of month
    let days_in_month = days_in_month_current();
    let ttl = (days_in_month * 86400) - ((chrono::Utc::now().day() as u64 - 1) * 86400);
    if count == 1 {
        let _: bool = conn.expire(monthly_key, ttl as i64).await.map_err(|e| e.to_string())?;
    }
    
    Ok(count <= limit)
}

fn days_in_month_current() -> u64 {
    let now = chrono::Utc::now();
    let year = now.year();
    let month = now.month();
    
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) { 29 } else { 28 },
        _ => 30,
    }
}

/// PART 13: Redis optimization - Query cache (temporarily disabled for compilation)
#[derive(Serialize, Deserialize, Clone)]
pub struct CachedQuery {
    pub query: String,
    pub result: serde_json::Value,
    pub timestamp: i64,
}

/// Cache query result in Redis
pub async fn cache_query(_redis_url: &str, _query: &str, _result: &serde_json::Value) -> Result<(), String> {
    Ok(())
}

/// Get cached query result
pub async fn get_cached_query(_redis_url: &str, _query: &str) -> Option<serde_json::Value> {
    None
}

/// Cache LLM response
pub async fn cache_llm_response(redis_url: &str, cache_key: &str, response: &str) -> Result<(), String> {
    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            let ttl = env::var("LLM_CACHE_TTL_SECONDS").ok().and_then(|v| v.parse().ok()).unwrap_or(3600);
            let _: Result<(), _> = conn.set_ex(format!("cache:llm:{}", cache_key), response, ttl).await;
        }
    }
    Ok(())
}

/// Get cached LLM response
pub async fn get_cached_llm(redis_url: &str, cache_key: &str) -> Option<String> {
    let client = redis::Client::open(redis_url).ok()?;
    let mut conn = client.get_multiplexed_async_connection().await.ok()?;
    conn.get(format!("cache:llm:{}", cache_key)).await.ok()
}

/// Store context memory for recent queries
pub async fn store_context_memory(_redis_url: &str, _user_id: &str, _context: &serde_json::Value) -> Result<(), String> {
    Ok(())
}

/// Get context memory for recent queries
pub async fn get_context_memory(_redis_url: &str, _user_id: &str) -> Option<serde_json::Value> {
    None
}
