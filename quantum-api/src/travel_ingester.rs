use std::env;
use crate::models::{FlightThreat, GenericFallbackResponse};
use chrono::Utc;
use log::{info, error};
use serde_json::Value;

/// Ingest flight data from Aviationstack API.
/// Falls back gracefully if the key is missing or the request fails.
pub async fn ingest_flights() -> GenericFallbackResponse<FlightThreat> {
    let timestamp = Utc::now().to_rfc3339();

    let api_key = match env::var("AVIATIONSTACK_API_KEY") {
        Ok(key) if !key.is_empty() => key,
        _ => {
            info!("AVIATIONSTACK_API_KEY not set. Returning fallback.");
            return GenericFallbackResponse {
                status: "aviationstack_pending".to_string(),
                provider_path: None,
                error: Some("AVIATIONSTACK_API_KEY missing".to_string()),
                results: vec![],
            };
        }
    };

    let client = reqwest::Client::builder()
        .user_agent("BharatIntelQuantum/1.0")
        .build()
        .unwrap_or_default();

    // Free-tier accounts commonly use the HTTP endpoint; use tight result size.
    let url = format!(
        "http://api.aviationstack.com/v1/flights?access_key={}&limit=20",
        api_key
    );

    match client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            if !status.is_success() {
                error!("Aviationstack returned status={} body={}", status, body);
                return GenericFallbackResponse {
                    status: "aviationstack_pending".to_string(),
                    provider_path: Some(url),
                    error: Some(format!("HTTP {} from provider", status)),
                    results: vec![],
                };
            }

            match serde_json::from_str::<Value>(&body) {
                Ok(json) => {
                    if json.get("error").is_some() {
                        error!("Aviationstack API error body={}", body);
                        return GenericFallbackResponse {
                            status: "aviationstack_pending".to_string(),
                            provider_path: Some(url),
                            error: Some("Aviationstack returned error payload".to_string()),
                            results: vec![],
                        };
                    }

                    let mut threats = Vec::new();
                    if let Some(flights) = json["data"].as_array() {
                        for flight in flights {
                            threats.push(FlightThreat {
                                flight_id: flight["flight"]["iata"]
                                    .as_str()
                                    .or_else(|| flight["flight"]["icao"].as_str())
                                    .or_else(|| flight["flight"]["number"].as_str())
                                    .unwrap_or("UNK")
                                    .to_string(),
                                origin: flight["departure"]["iata"]
                                    .as_str()
                                    .or_else(|| flight["departure"]["icao"].as_str())
                                    .unwrap_or("UNK")
                                    .to_string(),
                                destination: flight["arrival"]["iata"]
                                    .as_str()
                                    .or_else(|| flight["arrival"]["icao"].as_str())
                                    .unwrap_or("UNK")
                                    .to_string(),
                                risk_score: 0.5,
                                source: "Aviationstack Live".to_string(),
                                timestamp: timestamp.clone(),
                            });
                        }
                    }

                    if threats.is_empty() {
                        info!("Aviationstack returned empty data array.");
                        return GenericFallbackResponse {
                            status: "aviationstack_pending".to_string(),
                            provider_path: Some(url),
                            error: Some("Aviationstack returned no flight rows".to_string()),
                            results: vec![],
                        };
                    }

                    info!("Aviationstack returned {} flight(s)", threats.len());
                    GenericFallbackResponse {
                        status: "ok".to_string(),
                        provider_path: Some(url),
                        error: None,
                        results: threats,
                    }
                }
                Err(e) => {
                    error!("Aviationstack parse failed: {} body={}", e, body);
                    GenericFallbackResponse {
                        status: "aviationstack_pending".to_string(),
                        provider_path: Some(url),
                        error: Some(format!("JSON parse failed: {}", e)),
                        results: vec![],
                    }
                }
            }
        }
        Err(e) => {
            error!("Aviationstack request failed: {}", e);
            GenericFallbackResponse {
                status: "aviationstack_pending".to_string(),
                provider_path: Some("http://api.aviationstack.com/v1/flights".to_string()),
                error: Some(e.to_string()),
                results: vec![],
            }
        }
    }
}

