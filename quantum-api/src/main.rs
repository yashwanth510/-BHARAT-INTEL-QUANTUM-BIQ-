use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::env;
use dotenvy::dotenv;
use log::info;

mod lib;
mod models;
mod ingester;
mod china_ingester;
mod predictor;

use crate::lib::*;
use crate::ingester::*;
use crate::china_ingester::*;
use crate::predictor::*;

#[derive(Serialize)]
struct QuantumHealth {
    kyber1024: String,
    public_key: String,
    neo4j: String,
}

#[derive(Serialize)]
struct GeneralHealth {
    services: u8,
    status: String,
}

#[derive(Deserialize, Serialize)]
struct ThreatData {
    id: String,
    content: String,
    public_key: String,
}

#[derive(Serialize)]
struct ThreatResponse {
    status: String,
    encrypted_signal: String,
    ciphertext: String,
}

#[get("/quantum-health")]
async fn quantum_health() -> impl Responder {
    let keys = generate_quantum_keys();
    HttpResponse::Ok().json(QuantumHealth {
        kyber1024: "active".to_string(),
        public_key: keys.public_key,
        neo4j: "connected".to_string(), // In real app, check actual connection
    })
}

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().json(GeneralHealth {
        services: 4,
        status: "iDEX-ready".to_string(),
    })
}

#[post("/ingest-threat")]
async fn ingest_threat(threat: web::Json<ThreatData>) -> impl Responder {
    let (ss, ct) = encrypt_with_kyber(&threat.public_key);
    info!("Ingested threat: {} with quantum encryption", threat.id);
    
    // Kafka/Redis producer stubs here
    
    HttpResponse::Ok().json(ThreatResponse {
        status: "encrypted_and_queued".to_string(),
        encrypted_signal: ss,
        ciphertext: ct,
    })
}

#[get("/pakistan-threats")]
async fn pakistan_threats() -> impl Responder {
    let results = get_stored_threats().await;
    HttpResponse::Ok().json(results)
}

#[post("/ingest-pakistan")]
async fn trigger_ingest_pakistan() -> impl Responder {
    let results = ingest_pakistan().await;
    HttpResponse::Ok().json(results)
}

#[get("/china-threats")]
async fn china_threats() -> impl Responder {
    let results = get_stored_china_threats().await;
    HttpResponse::Ok().json(results)
}

#[post("/ingest-china")]
async fn trigger_ingest_china() -> impl Responder {
    let results = ingest_china().await;
    HttpResponse::Ok().json(results)
}

#[get("/predict")]
async fn predict() -> impl Responder {
    let threats = get_stored_china_threats().await;
    let prediction = predict_attack(&threats);
    HttpResponse::Ok().json(prediction)
}

#[get("/cross-border")]
async fn cross_border() -> impl Responder {
    let pak = get_stored_threats().await;
    let china = get_stored_china_threats().await;
    let mut fused = pak;
    fused.extend(china);
    HttpResponse::Ok().json(fused)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    let port = env::var("PORT").unwrap_or_else(|_| "8000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    
    info!("Starting Bharat Intel Quantum API on {}", addr);
    
    HttpServer::new(|| {
        App::new()
            .service(quantum_health)
            .service(health)
            .service(ingest_threat)
            .service(pakistan_threats)
            .service(trigger_ingest_pakistan)
            .service(china_threats)
            .service(trigger_ingest_china)
            .service(predict)
            .service(cross_border)
    })
    .bind(addr)?
    .run()
    .await
}
