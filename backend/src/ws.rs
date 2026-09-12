use axum::extract::ws::{Message, WebSocket};
use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    FlightUpdate(crate::models::Flight),
    VesselUpdate(crate::models::Vessel),
    VehicleUpdate(crate::models::Vehicle),
    Anomaly(crate::models::Anomaly),
    Heartbeat { ts: String },
}

pub async fn handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state.ws_tx.subscribe()))
}

async fn handle_socket(socket: WebSocket, mut rx: broadcast::Receiver<WsMessage>) {
    let (mut sender, mut receiver) = socket.split();

    // Send heartbeats + broadcast messages
    let mut send_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(15));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let hb = WsMessage::Heartbeat {
                        ts: chrono::Utc::now().to_rfc3339(),
                    };
                    let json = serde_json::to_string(&hb).unwrap_or_default();
                    if sender.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
                msg = rx.recv() => {
                    match msg {
                        Ok(m) => {
                            let json = serde_json::to_string(&m).unwrap_or_default();
                            if sender.send(Message::Text(json)).await.is_err() {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(_) => break,
                    }
                }
            }
        }
    });

    // Drain client messages (we don't need them but must keep reading to detect close)
    let receive_task = async {
        while let Some(message) = receiver.next().await {
            if matches!(message, Ok(Message::Close(_)) | Err(_)) {
                break;
            }
        }
    };
    tokio::select! { _ = &mut send_task => {}, _ = receive_task => {} }
    send_task.abort();
}
