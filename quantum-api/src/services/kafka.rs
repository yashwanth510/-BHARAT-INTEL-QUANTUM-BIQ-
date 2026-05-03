#[cfg(feature = "kafka")]
use rdkafka::producer::{FutureProducer, FutureRecord};
#[cfg(feature = "kafka")]
use rdkafka::ClientConfig;
use serde::Serialize;
use std::time::Duration;

#[cfg(feature = "kafka")]
pub fn build_producer(brokers: &str) -> Result<FutureProducer, String> {
    ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .create()
        .map_err(|e| e.to_string())
}

#[cfg(not(feature = "kafka"))]
pub fn build_producer(_brokers: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(feature = "kafka")]
pub type Producer = FutureProducer;

#[cfg(not(feature = "kafka"))]
pub type Producer = ();

#[cfg(feature = "kafka")]
pub async fn publish_json<T: Serialize>(
    producer: Option<&FutureProducer>,
    topic: &str,
    key: &str,
    payload: &T,
) -> Result<(), String> {
    let Some(p) = producer else {
        return Ok(());
    };
    let body = serde_json::to_string(payload).map_err(|e| e.to_string())?;
    p.send(
        FutureRecord::to(topic).key(key).payload(&body),
        Duration::from_secs(2),
    )
    .await
    .map_err(|(e, _)| e.to_string())?;
    Ok(())
}

#[cfg(not(feature = "kafka"))]
pub async fn publish_json<T: Serialize>(
    _producer: Option<&()>,
    _topic: &str,
    _key: &str,
    _payload: &T,
) -> Result<(), String> {
    Ok(())
}
