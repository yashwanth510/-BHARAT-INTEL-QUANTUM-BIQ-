use std::env;
use crate::models::GlobalThreat;
use chrono::Utc;
use xml::reader::{EventReader, XmlEvent};
use std::io::Cursor;
use log::error;
use redis::AsyncCommands;

pub async fn ingest_china(redis_url: &str) -> Vec<GlobalThreat> {
    let mut threats = Vec::new();
    let timestamp = Utc::now().to_rfc3339();

    // 1. Global Times RSS Ingest
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()
        .unwrap_or_default();

    match client.get("https://www.globaltimes.cn/rss").send().await {
        Ok(response) => {
            if let Ok(body) = response.text().await {
                let parser = EventReader::new(Cursor::new(body));
                let mut in_item = false;
                let mut in_title = false;
                for e in parser {
                    match e {
                        Ok(XmlEvent::StartElement { name, .. }) => {
                            if name.local_name == "item" { in_item = true; }
                            if in_item && name.local_name == "title" { in_title = true; }
                        }
                        Ok(XmlEvent::Characters(content)) => {
                            if in_title {
                                if content.contains("PLA") || content.contains("Ladakh") || content.contains("Xinjiang") {
                                    threats.push(GlobalThreat {
                                        actor: "PLA Ladakh".to_string(),
                                        country: "CN".to_string(),
                                        confidence: 0.88,
                                        sources: vec![format!("Global Times: {}", content)],
                                        location: Some("Ladakh Border".to_string()),
                                        timestamp: timestamp.clone(),
                                    });
                                }
                            }
                        }
                        Ok(XmlEvent::EndElement { name }) => {
                            if name.local_name == "item" { in_item = false; }
                            if name.local_name == "title" { in_title = false; }
                        }
                        _ => {}
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to fetch Global Times RSS: {}.", e);
        }
    }

    // 2. NewsAPI China Ingest
    if let Ok(api_key) = env::var("NEWSAPI_KEY") {
        let url = format!(
            "https://newsapi.org/v2/everything?q=China PLA OR Xinjiang OR Ladakh&sortBy=relevancy&apiKey={}",
            api_key
        );

        match client.get(&url).send().await {
            Ok(response) => {
                if let Ok(json) = response.json::<serde_json::Value>().await {
                    if let Some(articles) = json["articles"].as_array() {
                        for article in articles {
                            if let Some(title) = article["title"].as_str() {
                                threats.push(GlobalThreat {
                                    actor: "China Intel".to_string(),
                                    country: "CN".to_string(),
                                    confidence: 0.82,
                                    sources: vec![format!("NewsAPI: {}", title)],
                                    location: Some("Mainland China".to_string()),
                                    timestamp: timestamp.clone(),
                                });
                            }
                        }
                    }
                }
            }
            Err(e) => error!("Failed to fetch NewsAPI China: {:?}", e),
        }
    }

    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            if let Ok(payload) = serde_json::to_string(&threats) {
                let _: Result<(), _> = conn.set_ex("cache:china:latest", payload, 3600).await;
            }
        }
    }

    threats
}

pub async fn get_stored_china_threats(redis_url: &str) -> Vec<GlobalThreat> {
    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            if let Ok(Some(data)) = conn.get::<_, Option<String>>("cache:china:latest").await {
                if let Ok(threats) = serde_json::from_str(&data) {
                    return threats;
                }
            }
        }
    }
    vec![]
}
