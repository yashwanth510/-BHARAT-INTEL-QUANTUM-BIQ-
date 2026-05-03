/// PRIORITY 8 — Graceful degradation: scheduler NEVER crashes.
/// All provider failures are caught, logged, and the loop continues.
use crate::providers::{
    geospatial::fetch_geospatial_threats, maritime::fetch_maritime_threats,
    news::fetch_news_threats, weather::fetch_weather_threats,
};
use crate::services::ops_log::emit_log;
use crate::services::ws::WsMessage;
use chrono::Utc;
use std::env;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::broadcast;
use tokio::time::{Duration, sleep};

#[derive(Clone)]
pub struct SchedulerPolicy {
    pub newsapi_poll_seconds: u64,
    pub openweather_poll_seconds: u64,
    pub maritime_poll_seconds: u64,
    pub active_zone_multiplier: u64,
}

#[derive(Clone)]
pub struct SchedulerStats {
    pub ticks: Arc<AtomicU64>,
    pub quota_denied: Arc<AtomicU64>,
    pub cache_hits: Arc<AtomicU64>,
    pub backoff_events: Arc<AtomicU64>,
}

impl SchedulerStats {
    pub fn new() -> Self {
        Self {
            ticks: Arc::new(AtomicU64::new(0)),
            quota_denied: Arc::new(AtomicU64::new(0)),
            cache_hits: Arc::new(AtomicU64::new(0)),
            backoff_events: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl SchedulerPolicy {
    pub fn from_env() -> Self {
        Self {
            newsapi_poll_seconds: env::var("NEWSAPI_POLL_SECONDS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1800),
            openweather_poll_seconds: env::var("OPENWEATHER_POLL_SECONDS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
            maritime_poll_seconds: env::var("MARITIME_POLL_SECONDS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
            active_zone_multiplier: env::var("ACTIVE_ZONE_MULTIPLIER")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2),
        }
    }
}

fn compute_backoff(error: &str, attempt: u64) -> Option<Duration> {
    let e = error.to_lowercase();
    if e.contains("402") || e.contains("creditsdepleted") || e.contains("quota") {
        return Some(Duration::from_secs(900)); // 15 minutes
    }
    if e.contains("429") || e.contains("401") || e.contains("403") || e.contains("timeout") {
        let jitter = (attempt * 137) % 900;
        Some(Duration::from_millis(800 + attempt * 500 + jitter))
    } else {
        None
    }
}

/// PRIORITY 8 — Provider loop that NEVER panics.
/// Wraps every fetch in a catch-all, logs failures, continues.
async fn run_provider_loop(
    redis_url: String,
    base_interval: u64,
    active_interval: u64,
    tx: broadcast::Sender<WsMessage>,
    stats: SchedulerStats,
    provider_name: &'static str,
) {
    let mut attempt = 0_u64;

    loop {
        stats.ticks.fetch_add(1, Ordering::Relaxed);
        let active_mode =
            env::var("ACTIVE_ALERT_MODE").unwrap_or_else(|_| "false".to_string()) == "true";
        let wait_seconds = if active_mode {
            active_interval.max(10)
        } else {
            base_interval.max(10)
        };

        // PRIORITY 8 — Wrap fetch in catch-all; never let a panic escape
        let response = std::panic::AssertUnwindSafe(async {
            match provider_name {
                "news" => {
                    let r = fetch_news_threats(&redis_url).await;
                    emit_log(
                        &redis_url,
                        "NEWS",
                        "FETCH_COMPLETE",
                        &format!("{} ARTICLES", r.results.len()),
                    )
                    .await;
                    serde_json::to_string(&r).unwrap_or_default()
                }
                "weather" => {
                    let r = fetch_weather_threats(&redis_url, None).await;
                    emit_log(
                        &redis_url,
                        "WEATHER",
                        "FETCH_COMPLETE",
                        &format!("{} ZONES", r.results.len()),
                    )
                    .await;
                    serde_json::to_string(&r).unwrap_or_default()
                }
                "maritime" => {
                    let r = fetch_maritime_threats(&redis_url, None).await;
                    emit_log(
                        &redis_url,
                        "AIS",
                        "SCHEDULED_FETCH",
                        &format!("{} VESSELS", r.results.len()),
                    )
                    .await;
                    serde_json::to_string(&r).unwrap_or_default()
                }
                _ => {
                    let r = fetch_geospatial_threats(&redis_url, None).await;
                    emit_log(
                        &redis_url,
                        "TERRAIN",
                        "FETCH_COMPLETE",
                        &format!("{} ZONES", r.results.len()),
                    )
                    .await;
                    serde_json::to_string(&r).unwrap_or_default()
                }
            }
        });

        // Catch panics — log and continue
        let response = match futures::FutureExt::catch_unwind(response).await {
            Ok(r) => r,
            Err(e) => {
                let msg = format!("PANIC in {} scheduler: {:?}", provider_name, e);
                log::error!("{}", msg);
                emit_log(&redis_url, provider_name, "SCHEDULER_PANIC", &msg).await;
                sleep(Duration::from_secs(30)).await;
                continue;
            }
        };

        if response.contains("redis_cache") {
            stats.cache_hits.fetch_add(1, Ordering::Relaxed);
        }
        if response.contains("provider_limited") || response.contains("quota") {
            stats.quota_denied.fetch_add(1, Ordering::Relaxed);
        }

        if provider_name == "maritime" {
             if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response) {
                 if let Some(results) = json["results"].as_array() {
                     for vessel in results {
                         let ws_msg = WsMessage::VesselUpdate {
                             r#type: "vessel_update".to_string(),
                             mmsi: vessel["vessel_id"].as_str().unwrap_or("").replace("MMSI-", ""),
                             lat: vessel["lat"].as_f64().unwrap_or(0.0),
                             lon: vessel["lon"].as_f64().unwrap_or(0.0),
                             vessel_name: vessel["vessel_name"].as_str().unwrap_or("UNKNOWN").to_string(),
                             risk_score: vessel["risk_score"].as_f64().unwrap_or(0.0),
                             timestamp: Utc::now().to_rfc3339(),
                         };
                         let _ = tx.send(ws_msg);
                     }
                 }
             }
        }

        let ws_msg = WsMessage::Standard {
            r#type: "update".to_string(),
            priority: "medium".to_string(),
            source: provider_name.to_string(),
            location: "scheduled".to_string(),
            message: format!("Scheduled {} update", provider_name),
            timestamp: Utc::now().to_rfc3339(),
        };
        let _ = tx.send(ws_msg);

        if let Some(delay) = compute_backoff(&response, attempt) {
            stats.backoff_events.fetch_add(1, Ordering::Relaxed);
            sleep(delay).await;
            attempt = attempt.saturating_add(1);
        } else {
            attempt = 0;
            sleep(Duration::from_secs(wait_seconds)).await;
        }
    }
}

pub fn start_scheduler(
    redis_url: String,
    tx: broadcast::Sender<WsMessage>,
    stats: SchedulerStats,
) {
    let policy = SchedulerPolicy::from_env();
    let active_news = policy.newsapi_poll_seconds / policy.active_zone_multiplier.max(1);
    let active_weather = policy.openweather_poll_seconds / policy.active_zone_multiplier.max(1);
    let active_maritime = policy.maritime_poll_seconds / policy.active_zone_multiplier.max(1);

    let stats_news = stats.clone();
    let tx_news = tx.clone();
    let redis_news = redis_url.clone();
    tokio::spawn(async move {
        run_provider_loop(
            redis_news,
            policy.newsapi_poll_seconds,
            active_news,
            tx_news,
            stats_news,
            "news",
        )
        .await;
    });

    let stats_weather = stats.clone();
    let tx_weather = tx.clone();
    let redis_weather = redis_url.clone();
    tokio::spawn(async move {
        run_provider_loop(
            redis_weather,
            policy.openweather_poll_seconds,
            active_weather,
            tx_weather,
            stats_weather,
            "weather",
        )
        .await;
    });

    tokio::spawn(async move {
        run_provider_loop(
            redis_url,
            policy.maritime_poll_seconds,
            active_maritime,
            tx,
            stats,
            "maritime",
        )
        .await;
    });
}

#[cfg(test)]
mod tests {
    use super::compute_backoff;

    #[test]
    fn backoff_is_applied_for_rate_limits() {
        let delay = compute_backoff("HTTP 429 from provider", 2);
        assert!(delay.is_some());
    }

    #[test]
    fn backoff_applied_for_quota() {
        let delay = compute_backoff("quota exceeded", 0);
        assert_eq!(delay.unwrap().as_secs(), 900);
    }

    #[test]
    fn no_backoff_for_success() {
        let delay = compute_backoff("ok", 0);
        assert!(delay.is_none());
    }
}
