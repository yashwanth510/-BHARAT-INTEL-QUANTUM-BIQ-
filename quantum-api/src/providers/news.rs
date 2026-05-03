use crate::models::{GenericFallbackResponse, NewsThreat};
use crate::services::ops_log::emit_log;
use chrono::Utc;
use redis::AsyncCommands;
use serde_json::Value;
use std::env;
use std::io::Cursor;
use tokio::time::Duration;
use xml::reader::{EventReader, XmlEvent};

fn keywords() -> Vec<String> {
    env::var("THREAT_KEYWORDS")
        .unwrap_or_else(|_| "border,incursion,terror,weapon,vessel".to_string())
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .collect()
}

fn severity_from_title(title: &str) -> String {
    let lowered = title.to_lowercase();
    if lowered.contains("attack") || lowered.contains("terror") {
        "high".to_string()
    } else if lowered.contains("incursion") || lowered.contains("conflict") {
        "medium".to_string()
    } else {
        "low".to_string()
    }
}

fn extract_keywords(title: &str, keys: &[String]) -> Vec<String> {
    let lowered = title.to_lowercase();
    keys.iter().filter(|k| lowered.contains(k.as_str())).cloned().collect()
}

async fn read_cache(redis_url: &str, key: &str) -> Option<Vec<NewsThreat>> {
    let client = redis::Client::open(redis_url).ok()?;
    let mut conn = client.get_multiplexed_async_connection().await.ok()?;
    let payload: Option<String> = conn.get(key).await.ok()?;
    payload.and_then(|raw| serde_json::from_str::<Vec<NewsThreat>>(&raw).ok())
}

async fn write_cache(redis_url: &str, key: &str, rows: &[NewsThreat]) {
    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            if let Ok(payload) = serde_json::to_string(rows) {
                let ttl = env::var("NEWSAPI_POLL_SECONDS").ok().and_then(|v| v.parse().ok()).unwrap_or(1800);
                let _: Result<(), _> = conn.set_ex(key, payload, ttl).await;
            }
        }
    }
}

pub async fn fetch_news_threats(redis_url: &str) -> GenericFallbackResponse<NewsThreat> {
    let ts = Utc::now().to_rfc3339();
    let cache_key = "cache:news:latest";
    if let Some(rows) = read_cache(redis_url, cache_key).await {
        return GenericFallbackResponse {
            status: "ok".to_string(),
            provider_path: Some("redis_cache".to_string()),
            error: None,
            results: rows,
        };
    }

    let key_words = keywords();
    let client = reqwest::Client::builder()
        .user_agent("BharatIntelQuantum/1.0")
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_default();
    let mut output: Vec<NewsThreat> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    let news_api_key = env::var("NEWSAPI_KEY").unwrap_or_default();
    if !news_api_key.is_empty() {
        let query = key_words.join(" OR ");
        match client
            .get("https://newsapi.org/v2/everything")
            .query(&[("q", query.as_str()), ("language", "en"), ("pageSize", "20")])
            .header("X-Api-Key", news_api_key)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(json) = resp.json::<Value>().await {
                    if let Some(rows) = json["articles"].as_array() {
                        for article in rows {
                            if let Some(title) = article["title"].as_str() {
                                let hits = extract_keywords(title, &key_words);
                                if !hits.is_empty() {
                                    output.push(NewsThreat {
                                        title: title.to_string(),
                                        source: article["source"]["name"].as_str().unwrap_or("NewsAPI").to_string(),
                                        severity: severity_from_title(title),
                                        keywords: hits,
                                        timestamp: ts.clone(),
                                    });
                                }
                            }
                        }
                    }
                } else {
                    errors.push("NewsAPI parse failed".to_string());
                }
            }
            Ok(resp) => errors.push(format!("NewsAPI HTTP {}", resp.status())),
            Err(e) => errors.push(format!("NewsAPI error: {}", e)),
        }
    } else {
        errors.push("NEWSAPI_KEY missing".to_string());
    }

    let rss_feeds = env::var("RSS_FEEDS").unwrap_or_else(|_| {
        "https://www.dawn.com/rss/latest,https://www.globaltimes.cn/rss".to_string()
    });
    for feed in rss_feeds.split(',').map(|x| x.trim()).filter(|x| !x.is_empty()) {
        match client.get(feed).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(body) = resp.text().await {
                    let parser = EventReader::new(Cursor::new(body));
                    let mut in_item = false;
                    let mut in_title = false;
                    for e in parser {
                        match e {
                            Ok(XmlEvent::StartElement { name, .. }) if name.local_name == "item" => in_item = true,
                            Ok(XmlEvent::StartElement { name, .. }) if in_item && name.local_name == "title" => in_title = true,
                            Ok(XmlEvent::EndElement { name }) if name.local_name == "item" => in_item = false,
                            Ok(XmlEvent::EndElement { name }) if name.local_name == "title" => in_title = false,
                            Ok(XmlEvent::Characters(title)) if in_title => {
                                let hits = extract_keywords(&title, &key_words);
                                if !hits.is_empty() {
                                    output.push(NewsThreat {
                                        title: title.clone(),
                                        source: format!("RSS: {}", feed),
                                        severity: severity_from_title(&title),
                                        keywords: hits,
                                        timestamp: ts.clone(),
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                } else {
                    errors.push(format!("RSS parse failed for {}", feed));
                }
            }
            Ok(resp) => errors.push(format!("RSS HTTP {} for {}", resp.status(), feed)),
            Err(e) => errors.push(format!("RSS error {} for {}", e, feed)),
        }
    }

    if !output.is_empty() {
        write_cache(redis_url, cache_key, &output).await;
        emit_log(redis_url, "NEWS", "FETCH_COMPLETE", &format!("{} ARTICLES", output.len())).await;
        GenericFallbackResponse {
            status: "ok".to_string(),
            provider_path: Some("newsapi+rss".to_string()),
            error: None,
            results: output,
        }
    } else {
        emit_log(redis_url, "NEWS", "FETCH_ERROR", &errors.join("; ")).await;
        GenericFallbackResponse {
            status: "provider_missing".to_string(),
            provider_path: Some("newsapi+rss".to_string()),
            error: Some(errors.join("; ")),
            results: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::extract_keywords;

    #[test]
    fn keywords_are_extracted() {
        let keys = vec!["border".to_string(), "terror".to_string()];
        let hits = extract_keywords("Border terror update", &keys);
        assert_eq!(hits.len(), 2);
    }
}
