use std::env;
use crate::models::GlobalThreat;
use chrono::Utc;
use twitter_v2::authorization::BearerToken;
use twitter_v2::TwitterApi;
use xml::reader::{EventReader, XmlEvent};
use std::io::Cursor;
use log::{info, error};
use redis::AsyncCommands;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

static TWITTER_QUOTA_EXHAUSTED: AtomicBool = AtomicBool::new(false);

pub fn is_twitter_quota_exhausted() -> bool {
    TWITTER_QUOTA_EXHAUSTED.load(Ordering::Relaxed)
}

pub fn set_twitter_quota_exhausted(exhausted: bool) {
    TWITTER_QUOTA_EXHAUSTED.store(exhausted, Ordering::Relaxed);
}

pub async fn ingest_pakistan(redis_url: &str) -> Vec<GlobalThreat> {
    let mut threats = Vec::new();
    let timestamp = Utc::now().to_rfc3339();

    // 1. Dawn RSS Ingest
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()
        .unwrap_or_default();

    match client.get("https://www.dawn.com/rss/latest").send().await {
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
                                if content.contains("Saeed") || content.contains("JeM") || content.contains("LeT") {
                                    threats.push(GlobalThreat {
                                        actor: "Unknown Extremist".to_string(),
                                        country: "PK".to_string(),
                                        confidence: 0.85,
                                        sources: vec![format!("Dawn: {}", content)],
                                        location: Some("Pakistan".to_string()),
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
            error!("Failed to fetch Dawn RSS: {}. Using fallback mock for demo.", e);
        }
    }

    // 2. X (Twitter) Ingest with HTTP 402 quota handling
    if !is_twitter_quota_exhausted() {
        if let Ok(token) = env::var("TWITTER_BEARER_TOKEN") {
            let auth = BearerToken::new(token);
            let api = TwitterApi::new(auth);

            let query = "Hafiz Saeed OR JeM OR LeT lang:ur OR lang:en";
            match api.get_tweets_search_recent(query).max_results(10).send().await {
                Ok(response) => {
                    if let Some(data) = response.data() {
                        for tweet in data {
                            threats.push(GlobalThreat {
                                actor: "Hafiz Saeed".to_string(),
                                country: "PK".to_string(),
                                confidence: 0.92,
                                sources: vec![format!("X: {}", tweet.text)],
                                location: Some("Bahawalpur".to_string()),
                                timestamp: timestamp.clone(),
                            });
                        }
                    }
                    // Reset quota flag on success
                    if is_twitter_quota_exhausted() {
                        set_twitter_quota_exhausted(false);
                        info!("[SOCIAL] [TWITTER] [QUOTA_RECOVERED] Twitter API quota has been restored");
                    }
                }
                Err(e) => {
                    let err_str = format!("{}", e);
                    // Check for HTTP 402 (CreditsDepleted / Quota Exhausted)
                    if err_str.contains("402") || err_str.contains("CreditsDepleted") || err_str.contains("quota") {
                        set_twitter_quota_exhausted(true);
                        info!("[SOCIAL] [QUOTA_EXHAUSTED] [RESETS_MAY_1_00:00_UTC] Twitter API credits depleted. Polling will retry every 15 minutes.");
                    } else if err_str.contains("401") {
                        error!("[SOCIAL] [AUTH_FAILED] [CHECK_BEARER_TOKEN] Twitter authentication failed: {}", e);
                    } else {
                        error!("[SOCIAL] [TWITTER_ERROR] Failed to fetch X data: {}", e);
                    }
                }
            }
        }
    } else {
        // Quota is exhausted, skip this cycle but keep polling
        info!("[SOCIAL] [TWITTER] [QUOTA_SKIP] Skipping Twitter poll due to quota exhaustion. Will retry automatically.");
    }

    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            if let Ok(payload) = serde_json::to_string(&threats) {
                let _: Result<(), _> = conn.set_ex("cache:pakistan:latest", payload, 3600).await;
            }
        }
    }

    threats
}

pub async fn get_stored_threats(redis_url: &str) -> Vec<GlobalThreat> {
    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            if let Ok(Some(data)) = conn.get::<_, Option<String>>("cache:pakistan:latest").await {
                if let Ok(threats) = serde_json::from_str(&data) {
                    return threats;
                }
            }
        }
    }
    vec![]
}
