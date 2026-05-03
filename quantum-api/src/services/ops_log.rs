/// PRIORITY 6 — Live Operations Log
/// All providers emit standardized operational events stored in Redis.
use chrono::Utc;
use redis::AsyncCommands;

const OPS_LOG_KEY: &str = "ops:log";
const OPS_LOG_MAX: isize = 200;

/// Emit a structured operational log entry and store in Redis.
/// Format: [HH:MM:SS] [CATEGORY] [ACTION] [RESULT]
pub async fn emit_log(redis_url: &str, category: &str, action: &str, result: &str) {
    let now = Utc::now();
    let entry = format!(
        "[{}] [{}] [{}] [{}]",
        now.format("%H:%M:%S"),
        category,
        action,
        result
    );

    log::info!("{}", entry);

    // Store in Redis list, keep last 200
    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            let _: Result<(), _> = conn.lpush(OPS_LOG_KEY, &entry).await;
            let _: Result<(), _> = conn.ltrim(OPS_LOG_KEY, 0, OPS_LOG_MAX - 1).await;
        }
    }
}

/// Retrieve the last N log entries from Redis.
pub async fn get_ops_log(redis_url: &str, count: isize) -> Vec<String> {
    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            return conn
                .lrange(OPS_LOG_KEY, 0, count - 1)
                .await
                .unwrap_or_default();
        }
    }
    vec![]
}
