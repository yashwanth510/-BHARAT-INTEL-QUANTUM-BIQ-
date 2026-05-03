use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;
use redis::AsyncCommands;

async fn read_cache(redis_url: &str, key: &str) -> Option<TavilyResponse> {
    let client = redis::Client::open(redis_url).ok()?;
    let mut conn = client.get_multiplexed_async_connection().await.ok()?;
    let payload: Option<String> = conn.get(format!("cache:tavily:{}", key)).await.ok()?;
    payload.and_then(|raw| serde_json::from_str::<TavilyResponse>(&raw).ok())
}

async fn write_cache(redis_url: &str, key: &str, data: &TavilyResponse) {
    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            if let Ok(payload) = serde_json::to_string(data) {
                let _: Result<(), _> = conn.set_ex(format!("cache:tavily:{}", key), payload, 3600).await;
            }
        }
    }
}

#[derive(Serialize)]
struct TavilyRequest {
    api_key: String,
    query: String,
    search_depth: String,
    include_answer: bool,
    include_raw_content: bool,
    max_results: usize,
    include_domains: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TavilyResult {
    pub title: String,
    pub url: String,
    pub content: String,
    pub score: f64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TavilyResponse {
    pub results: Vec<TavilyResult>,
    pub answer: Option<String>,
    pub expanded_queries: Option<Vec<String>>,
}

/// Expand query with military/intelligence keywords
pub fn expand_query(query: &str) -> Vec<String> {
    let base_query = query.to_string();
    let mut expanded = vec![base_query.clone()];
    
    let lower = query.to_lowercase();
    
    // Add context-specific expansions
    if lower.contains("border") || lower.contains("ladakh") || lower.contains("kargil") {
        expanded.push(format!("{} military activity", base_query));
        expanded.push(format!("{} conflict", base_query));
    }
    
    if lower.contains("vessel") || lower.contains("ship") || lower.contains("maritime") {
        expanded.push(format!("{} naval exercise", base_query));
        expanded.push(format!("{} shipping route", base_query));
    }
    
    if lower.contains("satellite") || lower.contains("imagery") {
        expanded.push(format!("{} satellite activity", base_query));
        expanded.push(format!("{} infrastructure changes", base_query));
    }
    
    // Always add security context
    expanded.push(format!("{} security threat", base_query));
    expanded.push(format!("{} intelligence report", base_query));
    
    // Limit to 5 queries to avoid API overuse
    expanded.into_iter().take(5).collect()
}

/// Search with expanded queries and merge results
pub async fn search_threats_expanded(query: &str) -> Result<TavilyResponse, Box<dyn std::error::Error + Send + Sync>> {
    let queries = expand_query(query);
    let api_key = env::var("TAVILY_API_KEY")?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    
    let mut all_results: Vec<TavilyResult> = Vec::new();
    let mut answers: Vec<String> = Vec::new();
    
    // Run searches concurrently
    for q in &queries {
        let req_body = TavilyRequest {
            api_key: api_key.clone(),
            query: q.clone(),
            search_depth: "advanced".to_string(),
            include_answer: true,
            include_raw_content: false,
            max_results: 3,
            include_domains: vec![
                "gov.in".to_string(),
                "mod.gov.in".to_string(),
                "ndtv.com".to_string(),
                "timesofindia.indiatimes.com".to_string(),
                "reuters.com".to_string(),
                "bbc.com".to_string(),
            ],
        };
        
        match client.post("https://api.tavily.com/search")
            .json(&req_body)
            .send()
            .await {
            Ok(resp) => {
                if let Ok(tavily_resp) = resp.json::<TavilyResponse>().await {
                    all_results.extend(tavily_resp.results);
                    if let Some(ans) = tavily_resp.answer {
                        answers.push(ans);
                    }
                }
            }
            Err(e) => log::warn!("Tavily query '{}' failed: {}", q, e),
        }
    }
    
    // Deduplicate by URL
    let mut seen_urls = std::collections::HashSet::new();
    all_results.retain(|r| seen_urls.insert(r.url.clone()));
    
    // Sort by score
    all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    
    // Limit total results
    all_results.truncate(10);
    
    Ok(TavilyResponse {
        results: all_results,
        answer: if answers.is_empty() { None } else { Some(answers.join(" ")) },
        expanded_queries: Some(queries),
    })
}

/// Simple search without expansion
pub async fn search_threats(redis_url: &str, query: &str) -> Result<TavilyResponse, Box<dyn std::error::Error + Send + Sync>> {
    let cache_key = query.to_lowercase().replace(' ', "_");
    if let Some(cached) = read_cache(redis_url, &cache_key).await {
        return Ok(cached);
    }

    let api_key = env::var("TAVILY_API_KEY")?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?;
    
    let req_body = TavilyRequest {
        api_key,
        query: query.to_string(),
        search_depth: "basic".to_string(),
        include_answer: false,
        include_raw_content: false,
        max_results: 5,
        include_domains: vec![],
    };
    
    let resp = client.post("https://api.tavily.com/search")
        .json(&req_body)
        .send()
        .await?;
        
    let data = resp.json::<TavilyResponse>().await?;
    write_cache(redis_url, &cache_key, &data).await;
    Ok(data)
}
