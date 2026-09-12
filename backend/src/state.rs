use anyhow::Result;
use redis::aio::ConnectionManager;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::config::Config;
use crate::geo::{BorderStore, GeofenceEngine};
use crate::ws::WsMessage;

pub struct AppState {
    pub config: Config,
    pub http: reqwest::Client,
    pub provider_status: tokio::sync::RwLock<std::collections::HashMap<String, serde_json::Value>>,
    pub redis: ConnectionManager,
    pub borders: BorderStore,
    pub geofence: Arc<GeofenceEngine>,
    pub ws_tx: broadcast::Sender<WsMessage>,
}

impl AppState {
    pub async fn new(cfg: Config) -> Result<Self> {
        let client = redis::Client::open(cfg.redis_url.clone())?;
        let redis = ConnectionManager::new(client).await?;

        let borders = BorderStore::load(&cfg.border_data_dir)?;
        let geofence = Arc::new(GeofenceEngine::build(
            &cfg.border_data_dir,
            cfg.border_corridor_km,
            cfg.maritime_corridor_nm,
        ));

        let (ws_tx, _) = broadcast::channel(2048);

        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(40))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()?,
            provider_status: Default::default(),
            config: cfg,
            redis,
            borders,
            geofence,
            ws_tx,
        })
    }

    pub async fn provider_result(&self, name: &str, ok: bool, detail: &str) {
        let mut statuses = self.provider_status.write().await;
        let status = statuses
            .entry(name.into())
            .or_insert_with(|| serde_json::json!({}));
        status["status"] = serde_json::json!(if ok { "ok" } else { "error" });
        status["last_attempt"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
        status["detail"] = serde_json::json!(detail);
        if ok {
            status["last_success"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
        }
    }

    pub fn redis(&self) -> ConnectionManager {
        self.redis.clone()
    }
}
