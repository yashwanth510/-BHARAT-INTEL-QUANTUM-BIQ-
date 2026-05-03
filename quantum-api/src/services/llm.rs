/// PRIORITY 1 — True Multi-Source Mistral Context
/// Constructs a real intelligence context block from ALL providers before every Mistral request.
use crate::providers::osint::search_threats;
use crate::services::ops_log::emit_log;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use tokio::time::Duration;
use uuid::Uuid;

pub fn llm_enabled_for_event(is_new_event: bool) -> bool {
    let enabled = env::var("LLM_ENABLED").unwrap_or_else(|_| "true".to_string()) == "true";
    let only_on_new =
        env::var("LLM_ONLY_ON_NEW_EVENTS").unwrap_or_else(|_| "true".to_string()) == "true";
    enabled && (!only_on_new || is_new_event)
}

/// Multi-source intelligence context passed to Mistral.
#[derive(Debug, Default)]
pub struct IntelContext {
    pub correlation_id: String,
    pub query: String,
    pub tavily_results: String,
    pub news_headlines: String,
    pub terrain_summary: String,
    pub weather_summary: String,
    pub maritime_summary: String,
    pub financial_summary: String,
    pub satellite_summary: String,
    pub sentiment_summary: String,
    // availability flags for confidence scoring
    pub tavily_count: usize,
    pub news_count: usize,
    pub weather_ok: bool,
    pub terrain_ok: bool,
    pub maritime_ok: bool,
    pub financial_ok: bool,
    pub satellite_ok: bool,
}

impl IntelContext {
    pub fn new(query: &str) -> Self {
        Self {
            correlation_id: Uuid::new_v4().to_string(),
            query: query.to_string(),
            ..Default::default()
        }
    }

    /// Build the structured prompt block for Mistral.
    pub fn build_prompt(&self) -> String {
        format!(
            r#"CORRELATION_ID: {correlation_id}
QUERY: {query}

OSINT_SEARCH: {tavily}
NEWS_SIGNALS: {news}
TERRAIN: {terrain}
WEATHER: {weather}
MARITIME: {maritime}
FINANCIAL: {financial}
SATELLITE: {satellite}
SENTIMENT: {sentiment}

You are a senior intelligence analyst for Bharat Intel Quantum (BIQ).
Based on the multi-source intelligence context above, compute a threat assessment.
Respond ONLY with valid JSON in this exact format:
{{
  "score": <float 0.0-1.0>,
  "level": "<NOMINAL|MONITORED|ELEVATED|HIGH|CRITICAL>",
  "explanation": "<2-3 sentence assessment>",
  "key_actors": ["<actor1>", "<actor2>"],
  "key_locations": ["<loc1>", "<loc2>"],
  "recommended_action": "<single actionable recommendation>",
  "sources_used": {{
    "tavily": {tavily_count},
    "news": {news_count},
    "weather": {weather_ok},
    "terrain": {terrain_ok},
    "maritime": {maritime_ok},
    "financial": {financial_ok},
    "satellite": {satellite_ok}
  }}
}}"#,
            correlation_id = self.correlation_id,
            query = self.query,
            tavily = self.tavily_results,
            news = self.news_headlines,
            terrain = self.terrain_summary,
            weather = self.weather_summary,
            maritime = self.maritime_summary,
            financial = self.financial_summary,
            satellite = self.satellite_summary,
            sentiment = self.sentiment_summary,
            tavily_count = self.tavily_count,
            news_count = self.news_count,
            weather_ok = self.weather_ok,
            terrain_ok = self.terrain_ok,
            maritime_ok = self.maritime_ok,
            financial_ok = self.financial_ok,
            satellite_ok = self.satellite_ok,
        )
    }
}

/// Low-level Mistral call with caching and timeout protection.
pub async fn summarize_threat(
    redis_url: &str,
    cache_key: &str,
    prompt: &str,
) -> Result<String, String> {
    if !llm_enabled_for_event(true) {
        return Ok("LLM disabled by policy".to_string());
    }

    // Cache check
    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            let cached: Option<String> = conn.get(cache_key).await.unwrap_or(None);
            if let Some(hit) = cached {
                return Ok(hit);
            }
        }
    }

    let api_key =
        env::var("MISTRAL_API_KEY").map_err(|_| "MISTRAL_API_KEY missing".to_string())?;
    let base_url = env::var("MISTRAL_BASE_URL")
        .unwrap_or_else(|_| "https://api.mistral.ai/v1".to_string());
    let model =
        env::var("LLM_MODEL").unwrap_or_else(|_| "mistral-small-latest".to_string());

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_default();

    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0.2
    });

    let resp = client
        .post(format!("{}/chat/completions", base_url.trim_end_matches('/')))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("LLM request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_text = resp.text().await.unwrap_or_default();
        return Err(format!("LLM HTTP {}: {}", status, err_text));
    }

    let json = resp
        .json::<Value>()
        .await
        .map_err(|e| format!("LLM parse failed: {}", e))?;
    let summary = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("No summary")
        .to_string();

    // Cache result
    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            let ttl = env::var("LLM_CACHE_TTL_SECONDS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600_u64);
            let _: Result<(), _> = conn.set_ex(cache_key, summary.clone(), ttl).await;
        }
    }

    Ok(summary)
}

/// PRIORITY 1 — Full multi-source intelligence fusion with Mistral.
/// Builds context from ALL providers, degrades gracefully on failures.
pub async fn correlate_osint_threats(
    redis_url: &str,
    query: &str,
) -> Result<Value, String> {
    // 1. Check cache
    let cache_key = format!("cache:llm:fusion:{}", query.to_lowercase().replace(' ', "_"));
    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            let cached: Option<String> = conn.get(&cache_key).await.unwrap_or(None);
            if let Some(hit) = cached {
                if let Ok(json) = serde_json::from_str::<Value>(&hit) {
                    let mut val = json.clone();
                    // Mark as cached
                    val["provider_path"] = serde_json::json!("redis_cache");
                    return Ok(val);
                }
            }
        }
    }

    let mut ctx = IntelContext::new(query);

    // --- TAVILY OSINT ---
    match tokio::time::timeout(Duration::from_secs(15), search_threats(redis_url, query)).await {
        Ok(Ok(tavily)) => {
            ctx.tavily_count = tavily.results.len();
            let snippets: Vec<String> = tavily
                .results
                .iter()
                .take(5)
                .map(|r| format!("[{}] {}", r.title, r.content.chars().take(200).collect::<String>()))
                .collect();
            ctx.tavily_results = snippets.join(" | ");
            if let Some(ans) = &tavily.answer {
                ctx.tavily_results = format!("ANSWER: {} | SOURCES: {}", ans, ctx.tavily_results);
            }
            emit_log(redis_url, "LLM", "TAVILY_FETCH", &format!("{} results", ctx.tavily_count)).await;
        }
        Ok(Err(e)) => {
            ctx.tavily_results = format!("UNAVAILABLE: {}", e);
            emit_log(redis_url, "LLM", "TAVILY_ERROR", &e.to_string()).await;
        }
        Err(_) => {
            ctx.tavily_results = "UNAVAILABLE: timeout".to_string();
            emit_log(redis_url, "LLM", "TAVILY_TIMEOUT", "15s exceeded").await;
        }
    }

    // --- NEWS SIGNALS ---
    {
        let news_raw: Option<String> = if let Ok(client) = redis::Client::open(redis_url) {
            if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
                conn.get("cache:news:latest").await.unwrap_or(None)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(raw) = news_raw {
            if let Ok(articles) = serde_json::from_str::<Vec<Value>>(&raw) {
                ctx.news_count = articles.len();
                let headlines: Vec<String> = articles
                    .iter()
                    .take(5)
                    .filter_map(|a| a["title"].as_str().map(|t| t.to_string()))
                    .collect();
                ctx.news_headlines = headlines.join(" | ");
                emit_log(redis_url, "LLM", "NEWS_CONTEXT", &format!("{} headlines", ctx.news_count)).await;
            }
        } else {
            ctx.news_headlines = "UNAVAILABLE: no cached news".to_string();
        }
    }

    // --- TERRAIN ---
    {
        let geo_raw: Option<String> = if let Ok(client) = redis::Client::open(redis_url) {
            if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
                conn.get("cache:geo:latest").await.unwrap_or(None)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(raw) = geo_raw {
            if let Ok(zones) = serde_json::from_str::<Vec<Value>>(&raw) {
                if let Some(first) = zones.first() {
                    let elev = first["elevation_m"].as_f64().unwrap_or(0.0);
                    let area = first["area"].as_str().unwrap_or("unknown");
                    let score = first["terrain_score"].as_f64().unwrap_or(0.0);
                    ctx.terrain_summary = format!(
                        "elevation={:.0}m zone={} terrain_score={:.2}",
                        elev, area, score
                    );
                    ctx.terrain_ok = true;
                    emit_log(redis_url, "LLM", "TERRAIN_CONTEXT", &ctx.terrain_summary).await;
                }
            }
        } else {
            ctx.terrain_summary = "UNAVAILABLE: no cached terrain".to_string();
        }
    }

    // --- WEATHER ---
    {
        let wx_raw: Option<String> = if let Ok(client) = redis::Client::open(redis_url) {
            if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
                conn.get("cache:weather:latest").await.unwrap_or(None)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(raw) = wx_raw {
            if let Ok(zones) = serde_json::from_str::<Vec<Value>>(&raw) {
                if let Some(first) = zones.first() {
                    let wind = first["wind_speed"].as_f64().unwrap_or(0.0);
                    let vis = first["visibility"].as_f64().unwrap_or(10.0);
                    let rain = first["rain_1h"].as_f64().unwrap_or(0.0);
                    let level = first["risk_level"].as_str().unwrap_or("LOW");
                    ctx.weather_summary = format!(
                        "wind={:.0}km/h visibility={:.1}km rain={:.1}mm condition={}",
                        wind * 3.6, vis, rain, level
                    );
                    ctx.weather_ok = true;
                    emit_log(redis_url, "LLM", "WEATHER_CONTEXT", &ctx.weather_summary).await;
                }
            }
        } else {
            ctx.weather_summary = "UNAVAILABLE: no cached weather".to_string();
        }
    }

    // --- MARITIME ---
    {
        let ais_raw: Option<String> = if let Ok(client) = redis::Client::open(redis_url) {
            if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
                conn.get("cache:maritime:latest").await.unwrap_or(None)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(raw) = ais_raw {
            if let Ok(vessels) = serde_json::from_str::<Vec<Value>>(&raw) {
                let total = vessels.len();
                let dark_count: usize = vessels
                    .iter()
                    .filter(|v| v["dark_vessel"].as_bool().unwrap_or(false))
                    .count();
                let high_risk: usize = vessels
                    .iter()
                    .filter(|v| v["risk_score"].as_f64().unwrap_or(0.0) > 0.7)
                    .count();
                ctx.maritime_summary = format!(
                    "{} vessels tracked {} dark vessel(s) {} high-risk",
                    total, dark_count, high_risk
                );
                ctx.maritime_ok = true;
                emit_log(redis_url, "LLM", "MARITIME_CONTEXT", &ctx.maritime_summary).await;
            }
        } else {
            ctx.maritime_summary = "UNAVAILABLE: no cached AIS data".to_string();
        }
    }

    // --- FINANCIAL ---
    {
        let fin_raw: Option<String> = if let Ok(client) = redis::Client::open(redis_url) {
            if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
                conn.get("cache:crypto:latest").await.unwrap_or(None)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(raw) = fin_raw {
            if let Ok(wallets) = serde_json::from_str::<Vec<Value>>(&raw) {
                let ofac_hits: usize = wallets
                    .iter()
                    .filter(|w| {
                        w["source"].as_str().unwrap_or("").contains("OFAC")
                            || w["risk_score"].as_f64().unwrap_or(0.0) >= 1.0
                    })
                    .count();
                ctx.financial_summary = format!("{} OFAC-linked wallet(s) detected", ofac_hits);
                ctx.financial_ok = true;
                emit_log(redis_url, "LLM", "FINANCIAL_CONTEXT", &ctx.financial_summary).await;
            }
        } else {
            ctx.financial_summary = "UNAVAILABLE: no cached financial data".to_string();
        }
    }

    // --- SATELLITE ---
    {
        let sat_raw: Option<String> = if let Ok(client) = redis::Client::open(redis_url) {
            if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
                conn.get("cache:satellite:latest").await.unwrap_or(None)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(raw) = sat_raw {
            if let Ok(alerts) = serde_json::from_str::<Vec<Value>>(&raw) {
                if alerts.is_empty() {
                    ctx.satellite_summary = "no recent activity detected".to_string();
                } else {
                    let types: Vec<String> = alerts
                        .iter()
                        .take(3)
                        .filter_map(|a| a["alert_type"].as_str().map(|s| s.to_string()))
                        .collect();
                    ctx.satellite_summary = format!("activity detected: {}", types.join(", "));
                    ctx.satellite_ok = true;
                }
                emit_log(redis_url, "LLM", "SATELLITE_CONTEXT", &ctx.satellite_summary).await;
            }
        } else {
            ctx.satellite_summary = "UNAVAILABLE: no cached satellite data".to_string();
        }
    }

    // --- SENTIMENT (derived from news/social) ---
    ctx.sentiment_summary = derive_sentiment(redis_url).await;

    // --- BUILD PROMPT & CALL MISTRAL ---
    let prompt = ctx.build_prompt();
    let cache_key = format!(
        "cache:llm:fusion:{}",
        query.replace(' ', "_").to_lowercase()
    );

    emit_log(redis_url, "LLM", "MISTRAL_REQUEST", &format!("correlation_id={}", ctx.correlation_id)).await;

    let raw_response = match summarize_threat(redis_url, &cache_key, &prompt).await {
        Ok(r) => r,
        Err(e) => {
            emit_log(redis_url, "LLM", "MISTRAL_ERROR", &e).await;
            return Err(e);
        }
    };

    // Strip markdown fences if present
    let cleaned = raw_response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let mut parsed: Value = serde_json::from_str(cleaned)
        .map_err(|e| format!("LLM output not valid JSON: {} — Raw: {}", e, cleaned))?;

    // Cache the successful assessment (final processed version)
    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            let ttl = env::var("LLM_CACHE_TTL_SECONDS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600_u64);
            let _: Result<(), _> = conn.set_ex(&cache_key, cleaned, ttl).await;
        }
    }

    // Inject correlation_id into response
    if let Some(obj) = parsed.as_object_mut() {
        obj.insert(
            "correlation_id".to_string(),
            Value::String(ctx.correlation_id.clone()),
        );
        obj.insert("query".to_string(), Value::String(query.to_string()));
    }

    // Apply risk calibration (P4)
    parsed = apply_risk_calibration(parsed);

    emit_log(
        redis_url,
        "LLM",
        "MISTRAL_COMPLETE",
        &format!(
            "score={} level={}",
            parsed["score"].as_f64().unwrap_or(0.0),
            parsed["level"].as_str().unwrap_or("UNKNOWN")
        ),
    )
    .await;

    Ok(parsed)
}

/// PRIORITY 4 — Risk calibration: downgrade if explanation contains low-threat phrases.
pub fn apply_risk_calibration(mut val: Value) -> Value {
    let low_threat_phrases = [
        "no direct indicators",
        "no immediate security concerns",
        "no active threats",
        "normal activity",
        "no significant",
    ];

    let explanation = val["explanation"]
        .as_str()
        .unwrap_or("")
        .to_lowercase();

    let should_downgrade = low_threat_phrases
        .iter()
        .any(|phrase| explanation.contains(phrase));

    if should_downgrade {
        if let Some(score) = val["score"].as_f64() {
            let capped = score.min(0.45);
            val["score"] = serde_json::json!(capped);
            val["level"] = serde_json::json!(score_to_level(capped as f32));
        }
    } else if let Some(score) = val["score"].as_f64() {
        val["level"] = serde_json::json!(score_to_level(score as f32));
    }

    val
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

/// Derive sentiment from cached social/news data.
async fn derive_sentiment(redis_url: &str) -> String {
    // Check Pakistan cache for threat keywords
    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            let pak: Option<String> = conn.get("cache:pakistan:latest").await.unwrap_or(None);
            if let Some(raw) = pak {
                if let Ok(threats) = serde_json::from_str::<Vec<Value>>(&raw) {
                    if threats.len() > 3 {
                        return "ELEVATED".to_string();
                    }
                }
            }
        }
    }
    "STABLE".to_string()
}
