/// P9 — WebSocket hub broadcast and message format tests.
use crate::services::ws::{WsHub, WsMessage};
use chrono::Utc;

fn make_ws_message(priority: &str, source: &str) -> WsMessage {
    WsMessage {
        r#type: "update".to_string(),
        priority: priority.to_string(),
        source: source.to_string(),
        location: "Karachi".to_string(),
        message: "Dark vessel anomaly detected".to_string(),
        timestamp: Utc::now().to_rfc3339(),
    }
}

// ── Hub creation ──────────────────────────────────────────────────────────────

#[test]
fn ws_hub_creates_without_panic() {
    let hub = WsHub::new();
    // Hub must be created without panicking
    drop(hub);
}

#[test]
fn ws_hub_broadcast_with_no_subscribers_does_not_panic() {
    let hub = WsHub::new();
    // Broadcasting with no subscribers must not panic
    hub.broadcast(make_ws_message("high", "fusion"));
    hub.broadcast(make_ws_message("medium", "news"));
    hub.broadcast(make_ws_message("low", "weather"));
}

#[test]
fn ws_hub_subscriber_receives_message() {
    let hub = WsHub::new();
    let mut rx = hub.tx.subscribe();

    let msg = make_ws_message("high", "maritime");
    hub.broadcast(msg.clone());

    let received = rx.try_recv().expect("subscriber should receive message");
    assert_eq!(received.priority, "high");
    assert_eq!(received.source, "maritime");
    assert_eq!(received.location, "Karachi");
}

#[test]
fn ws_hub_multiple_subscribers() {
    let hub = WsHub::new();
    let mut rx1 = hub.tx.subscribe();
    let mut rx2 = hub.tx.subscribe();

    hub.broadcast(make_ws_message("high", "fusion"));

    assert!(rx1.try_recv().is_ok(), "subscriber 1 should receive");
    assert!(rx2.try_recv().is_ok(), "subscriber 2 should receive");
}

// ── Message format ────────────────────────────────────────────────────────────

#[test]
fn ws_message_serializes_correctly() {
    let msg = make_ws_message("HIGH", "fusion");
    let json = serde_json::to_string(&msg).expect("must serialize");
    assert!(json.contains("\"type\""));
    assert!(json.contains("\"priority\""));
    assert!(json.contains("\"source\""));
    assert!(json.contains("\"location\""));
    assert!(json.contains("\"message\""));
    assert!(json.contains("\"timestamp\""));
}

#[test]
fn ws_message_type_field_correct() {
    // The field is `r#type` in Rust but must serialize as "type"
    let msg = WsMessage {
        r#type: "alert".to_string(),
        priority: "high".to_string(),
        source: "fusion".to_string(),
        location: "Ladakh".to_string(),
        message: "test".to_string(),
        timestamp: "2026-05-01T00:00:00Z".to_string(),
    };
    let json = serde_json::to_value(&msg).unwrap();
    assert_eq!(json["type"].as_str().unwrap(), "alert");
}

// ── Channel capacity ──────────────────────────────────────────────────────────

#[test]
fn ws_hub_channel_capacity_256() {
    // Broadcast 256 messages without any subscriber draining — must not panic
    let hub = WsHub::new();
    let _rx = hub.tx.subscribe(); // keep one subscriber to prevent lagged errors
    for i in 0..256 {
        hub.broadcast(WsMessage {
            r#type: "update".to_string(),
            priority: "low".to_string(),
            source: "test".to_string(),
            location: "test".to_string(),
            message: format!("msg {}", i),
            timestamp: Utc::now().to_rfc3339(),
        });
    }
}
