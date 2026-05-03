/// PRIORITY 9 — WebSocket streaming stabilization.
/// Heartbeat/ping, idle client disconnect, memory-safe broadcast.
use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use serde::Serialize;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum WsMessage {
    Standard {
        r#type: String,
        priority: String,
        source: String,
        location: String,
        message: String,
        timestamp: String,
    },
    VesselUpdate {
        r#type: String,
        mmsi: String,
        lat: f64,
        lon: f64,
        vessel_name: String,
        risk_score: f64,
        timestamp: String,
    },
}

#[derive(Clone)]
pub struct WsHub {
    pub tx: broadcast::Sender<WsMessage>,
}

impl WsHub {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self { tx }
    }

    pub fn broadcast(&self, msg: WsMessage) {
        // Silently drop if no subscribers — never panic
        let _ = self.tx.send(msg);
    }
}

pub struct WsSession {
    pub rx: broadcast::Receiver<WsMessage>,
    pub filter_high_priority: bool,
    pub last_heartbeat: Instant,
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // Heartbeat loop
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // Check for idle timeout
            if Instant::now().duration_since(act.last_heartbeat) > CLIENT_TIMEOUT {
                log::info!("[WS] Client idle timeout — disconnecting");
                ctx.stop();
                return;
            }
            ctx.ping(b"biq-ping");
        });

        // Message delivery loop (100ms polling)
        ctx.run_interval(Duration::from_millis(100), |act, ctx| {
            while let Ok(msg) = act.rx.try_recv() {
                if act.filter_high_priority {
                    match &msg {
                        WsMessage::Standard { priority, .. } if priority != "high" => continue,
                        WsMessage::VesselUpdate { risk_score, .. } if *risk_score < 0.7 => continue,
                        _ => {}
                    }
                }
                if let Ok(json) = serde_json::to_string(&msg) {
                    ctx.text(json);
                }
            }
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(ws::Message::Ping(msg)) => {
                self.last_heartbeat = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.last_heartbeat = Instant::now();
            }
            Ok(ws::Message::Close(_)) => ctx.stop(),
            Ok(ws::Message::Text(_))
            | Ok(ws::Message::Binary(_))
            | Ok(ws::Message::Continuation(_)) => {}
            Ok(ws::Message::Nop) => {}
            Err(_) => ctx.stop(),
        }
    }
}

pub async fn ws_stream_global(
    req: HttpRequest,
    stream: web::Payload,
    hub: web::Data<WsHub>,
) -> Result<HttpResponse, Error> {
    let rx = hub.tx.subscribe();
    ws::start(
        WsSession {
            rx,
            filter_high_priority: false,
            last_heartbeat: Instant::now(),
        },
        &req,
        stream,
    )
}

pub async fn ws_alerts_high(
    req: HttpRequest,
    stream: web::Payload,
    hub: web::Data<WsHub>,
) -> Result<HttpResponse, Error> {
    let rx = hub.tx.subscribe();
    ws::start(
        WsSession {
            rx,
            filter_high_priority: true,
            last_heartbeat: Instant::now(),
        },
        &req,
        stream,
    )
}

pub async fn ws_threats(
    req: HttpRequest,
    stream: web::Payload,
    hub: web::Data<WsHub>,
) -> Result<HttpResponse, Error> {
    let rx = hub.tx.subscribe();
    ws::start(
        WsSession {
            rx,
            filter_high_priority: false,
            last_heartbeat: Instant::now(),
        },
        &req,
        stream,
    )
}
