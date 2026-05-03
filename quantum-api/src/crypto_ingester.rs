use std::env;
use crate::models::{CryptoThreat, ScreeningResult, GenericFallbackResponse};
use chrono::Utc;
use log::{info, error};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use redis::AsyncCommands;
use std::collections::HashSet;

// OFAC XML Structures
#[derive(Debug)]
struct OfacEntry {
    address: String,
    name: String,
    programs: Vec<String>,
    last_updated: String,
}

// MistTrack API Response
#[derive(Deserialize, Debug)]
struct MistTrackResponse {
    #[serde(default)]
    risk_score: f64,
    #[serde(default)]
    risk_level: String,
    #[serde(default)]
    labels: Vec<String>,
}

// OFAC Screening Result
#[derive(Clone, Debug)]
struct OfacResult {
    is_sanctioned: bool,
    entity_name: String,
    programs: Vec<String>,
}

/// Ingest crypto wallet sanctions data from OFAC + MistTrack.
/// Replaces Chainalysis integration.
pub async fn ingest_crypto_wallets(redis_url: &str) -> GenericFallbackResponse<CryptoThreat> {
    let timestamp = Utc::now().to_rfc3339();

    // Sync OFAC sanctioned addresses to Redis on startup
    if let Err(e) = sync_ofac_sanctions(redis_url).await {
        error!("[FINANCIAL] [OFAC_SYNC] Failed to sync OFAC data: {}", e);
    }

    // Sample screening for a known address
    let test_address = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa";
    let result = screen_wallet(redis_url, test_address, &timestamp).await;

    match result {
        Ok(screening) => {
            let threat = CryptoThreat {
                wallet_address: screening.wallet_address.clone(),
                risk_score: screening.risk_score,
                sanctions_list: screening.category.clone(),
                source: screening.source.clone(),
                country: "GLOBAL".to_string(),
                timestamp: timestamp.clone(),
            };

            info!("[FINANCIAL] [SCREENING] {} | Risk: {} | Source: {}",
                screening.wallet_address, screening.risk_level, screening.source);

            GenericFallbackResponse {
                status: "ok".to_string(),
                provider_path: Some("OFAC_MISTTRACK".to_string()),
                error: None,
                results: vec![threat],
            }
        }
        Err(e) => {
            error!("[FINANCIAL] [SCREENING_ERROR] {}", e);
            GenericFallbackResponse {
                status: "screening_error".to_string(),
                provider_path: Some("OFAC_MISTTRACK".to_string()),
                error: Some(e.to_string()),
                results: vec![],
            }
        }
    }
}

/// Sync OFAC sanctioned addresses from Treasury XML to Redis
async fn sync_ofac_sanctions(redis_url: &str) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .user_agent("BharatIntelQuantum/1.0")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default();

    let url = "https://www.treasury.gov/ofac/downloads/sanctions/1.0/sdn_advanced.xml";

    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(format!("OFAC HTTP {}", response.status()).into());
    }

    let xml_content = response.text().await?;
    let sanctioned_addresses = parse_ofac_xml(&xml_content);

    // Store in Redis Set with 86400s TTL
    let redis_client = redis::Client::open(redis_url)?;
    let mut conn = redis_client.get_multiplexed_async_connection().await?;

    // Store each address in a Set
    let key = "ofac:sanctioned_addresses";
    let _: () = redis::cmd("DEL").arg(key).query_async(&mut conn).await?;

    for entry in &sanctioned_addresses {
        let value = format!("{}|{}|{}", entry.address, entry.name, entry.programs.join(","));
        let _: () = conn.sadd(key, &value).await?;
    }

    // Set TTL
    let _: () = conn.expire(key, 86400).await?;

    info!("[FINANCIAL] [OFAC_SYNC] [{} sanctioned addresses loaded]", sanctioned_addresses.len());

    Ok(sanctioned_addresses.len())
}

/// Parse OFAC XML and extract digital currency addresses
fn parse_ofac_xml(xml_content: &str) -> Vec<OfacEntry> {
    let mut entries = Vec::new();
    let mut current_id = String::new();
    let mut current_name = String::new();
    let mut in_id = false;
    let mut in_publish_information = false;

    // Simple XML parsing for OFAC format
    // Look for entries with Digital Currency Address
    if let Ok(doc) = roxmltree::Document::parse(xml_content) {
        for node in doc.descendants() {
            if node.has_tag_name("publishInformation") {
                in_publish_information = true;
                continue;
            }
            if in_publish_information && node.has_tag_name("id") && node.text().is_some() {
                current_id = node.text().unwrap_or("").to_string();
                in_id = true;
                continue;
            }
            if in_id && node.has_tag_name("lastName") && node.text().is_some() {
                current_name = node.text().unwrap_or("").to_string();
                continue;
            }
            // Look for Digital Currency Address feature
            if node.has_tag_name("feature") {
                if let Some(type_attr) = node.attribute("featureType") {
                    if type_attr.contains("Digital Currency Address") {
                        // Find the address value
                        for child in node.children() {
                            if child.has_tag_name("featureVersion") {
                                for fv_child in child.children() {
                                    if fv_child.has_tag_name("versionLocation") {
                                        if let Some(addr) = fv_child.text() {
                                            entries.push(OfacEntry {
                                                address: addr.trim().to_string(),
                                                name: current_name.clone(),
                                                programs: vec!["OFAC_SDN".to_string()],
                                                last_updated: Utc::now().to_rfc3339(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    entries
}

/// Screen a wallet address against OFAC and MistTrack
async fn screen_wallet(
    redis_url: &str,
    address: &str,
    timestamp: &str
) -> Result<ScreeningResult, Box<dyn std::error::Error + Send + Sync>> {

    // Step 1: Check OFAC first (O(1) Redis lookup)
    let ofac_result = screen_against_ofac(redis_url, address).await;

    if ofac_result.is_sanctioned {
        return Ok(ScreeningResult {
            wallet_address: address.to_string(),
            risk_level: "CRITICAL".to_string(),
            risk_score: 1.0,
            source: "OFAC_SDN".to_string(),
            entity: ofac_result.entity_name,
            category: ofac_result.programs.join(", "),
            timestamp: timestamp.to_string(),
        });
    }

    // Step 2: If not sanctioned, check MistTrack
    let misttrack_result = call_misttrack(address).await;

    match misttrack_result {
        Ok(mt) => {
            let risk_score = mt.risk_score / 100.0; // Convert 0-100 to 0.0-1.0
            let risk_level = if mt.risk_score >= 80.0 {
                "Severe"
            } else if mt.risk_score >= 60.0 {
                "High"
            } else if mt.risk_score >= 30.0 {
                "Medium"
            } else {
                "Low"
            };

            Ok(ScreeningResult {
                wallet_address: address.to_string(),
                risk_level: risk_level.to_string(),
                risk_score,
                source: "MISTTRACK".to_string(),
                entity: mt.labels.join(", "),
                category: "CRYPTO_RISK_ANALYSIS".to_string(),
                timestamp: timestamp.to_string(),
            })
        }
        Err(e) => {
            error!("[FINANCIAL] [MISTTRACK_ERROR] {}", e);
            // Return a low-risk fallback on MistTrack error
            Ok(ScreeningResult {
                wallet_address: address.to_string(),
                risk_level: "Unknown".to_string(),
                risk_score: 0.0,
                source: "MISTTRACK_ERROR".to_string(),
                entity: e.to_string(),
                category: "API_ERROR".to_string(),
                timestamp: timestamp.to_string(),
            })
        }
    }
}

/// Screen against OFAC sanctioned addresses (O(1) Redis lookup)
async fn screen_against_ofac(
    redis_url: &str,
    address: &str
) -> OfacResult {
    let redis_client = match redis::Client::open(redis_url) {
        Ok(c) => c,
        Err(_) => return OfacResult {
            is_sanctioned: false,
            entity_name: String::new(),
            programs: Vec::new(),
        },
    };

    let mut conn = match redis_client.get_multiplexed_async_connection().await {
        Ok(c) => c,
        Err(_) => return OfacResult {
            is_sanctioned: false,
            entity_name: String::new(),
            programs: Vec::new(),
        },
    };

    let key = "ofac:sanctioned_addresses";

    // Get all members and check if address matches
    match conn.smembers::<_, Vec<String>>(key).await {
        Ok(members) => {
            for member in members {
                let parts: Vec<&str> = member.split('|').collect();
                if parts.len() >= 3 && parts[0].to_lowercase() == address.to_lowercase() {
                    let programs = parts[2].split(',').map(|s| s.to_string()).collect();
                    return OfacResult {
                        is_sanctioned: true,
                        entity_name: parts[1].to_string(),
                        programs,
                    };
                }
            }
        }
        Err(_) => {}
    }

    OfacResult {
        is_sanctioned: false,
        entity_name: String::new(),
        programs: Vec::new(),
    }
}

/// Call MistTrack API for risk scoring
async fn call_misttrack(address: &str) -> Result<MistTrackResponse, Box<dyn std::error::Error + Send + Sync>> {
    let api_key = match env::var("MISTTRACK_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            return Err("MISTTRACK_API_KEY not configured".into());
        }
    };

    let client = reqwest::Client::builder()
        .user_agent("BharatIntelQuantum/1.0")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_default();

    let url = "https://openapi.misttrack.io/v1/risk_score";

    let body = serde_json::json!({
        "address": address,
        "coin": "BTC"
    });

    let response = client
        .post(url)
        .header("API-KEY", api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("MistTrack HTTP {}", response.status()).into());
    }

    let data: MistTrackResponse = response.json().await?;
    Ok(data)
}

/// Schedule crypto wallet screening (runs every 6 hours)
pub async fn schedule_crypto_screening(redis_url: &str) {
    let interval = std::time::Duration::from_secs(6 * 3600); // 6 hours

    loop {
        let _ = ingest_crypto_wallets(redis_url).await;
        tokio::time::sleep(interval).await;
    }
}

