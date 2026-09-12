mod auth;
mod config;
mod error;
mod geo;
mod ingest;
mod models;
mod providers;
mod routes;
mod state;
mod validation;
mod ws;

use axum::{
    routing::{get, post, put},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use config::Config;
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match dotenvy::dotenv() {
        Ok(_) => {}
        Err(dotenvy::Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => anyhow::bail!(
            "Invalid .env file; quote values containing spaces and check dotenv syntax"
        ),
    }

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cfg = Config::from_env()?;
    tracing::info!("BIQ backend starting on port {}", cfg.port);

    let state = AppState::new(cfg.clone()).await?;
    let state = Arc::new(state);

    // Spawn ingest workers
    if cfg.ingest_enabled {
        ingest::opensky::spawn(Arc::clone(&state));
        ingest::aisstream::spawn(Arc::clone(&state));
        ingest::gps::spawn(Arc::clone(&state));
    }

    let cors = CorsLayer::new()
        .allow_origin(
            cfg.cors_origins
                .iter()
                .map(|origin| origin.parse::<axum::http::HeaderValue>())
                .collect::<std::result::Result<Vec<_>, _>>()?,
        )
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(routes::health::handler))
        .route("/api/borders", get(routes::borders::handler))
        .route("/api/flights", get(routes::tracks::flights))
        .route("/api/vessels", get(routes::tracks::vessels))
        .route("/api/vehicles", get(routes::tracks::vehicles))
        .route("/api/anomalies", get(routes::anomalies::list))
        .route("/api/location", get(routes::location::handler))
        .route("/api/weather", get(routes::weather::handler))
        .route("/api/satellite/snapshot", post(routes::satellite::handler))
        .route("/api/osint/enrich", post(routes::osint::handler))
        .route("/api/allowlist", get(routes::allowlist::list))
        .route("/api/allowlist", put(routes::allowlist::upsert))
        .route("/ingest/gps", post(ingest::gps::webhook_handler))
        .route("/ws/live", get(ws::handler))
        .with_state(Arc::clone(&state))
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = format!("0.0.0.0:{}", cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
