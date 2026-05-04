use serde::Serialize;
use log::{info, warn};

pub trait EventBus: Send + Sync {
    fn publish(&self, topic: &str, key: &str, payload: &str);
}

#[cfg(feature = "kafka")]
pub struct KafkaBus {
    producer: crate::services::kafka::Producer,
}

#[cfg(feature = "kafka")]
impl KafkaBus {
    pub fn new(brokers: &str) -> Result<Self, String> {
        info!("Initializing KafkaBus with brokers: {}", brokers);
        let producer = crate::services::kafka::build_producer(brokers)?;
        Ok(Self { producer })
    }
}

#[cfg(feature = "kafka")]
impl EventBus for KafkaBus {
    fn publish(&self, topic: &str, key: &str, payload: &str) {
        let producer = self.producer.clone();
        let topic = topic.to_string();
        let key = key.to_string();
        let payload = payload.to_string();
        tokio::spawn(async move {
            let _ = crate::services::kafka::publish_json(Some(&producer), &topic, &key, &payload).await;
        });
    }
}

pub struct NoopBus;

impl NoopBus {
    pub fn new() -> Self {
        Self
    }
}

impl EventBus for NoopBus {
    fn publish(&self, topic: &str, _key: &str, payload: &str) {
        warn!("[EventBus-OFF] Would publish to {}: {}", topic, payload);
    }
}
