/// PRIORITY 7 — Elevation preload on boot + 6-hour refresh.
/// PRIORITY 6 — Emit ops log on every fetch.
use crate::models::{GenericFallbackResponse, GeospatialThreat};
use crate::services::geo_resolver::GeoPoint;
use crate::services::ops_log::emit_log;
use chrono::Utc;
use redis::AsyncCommands;
use serde::Deserialize;
use std::env;
use tokio::time::Duration;

#[derive(Deserialize)]
struct OpenTopoDataLocation {
    lat: f64,
    lng: f64,
}

#[derive(Deserialize)]
struct OpenTopoDataResult {
    elevation: f64,
    location: OpenTopoDataLocation,
}

#[derive(Deserialize)]
struct OpenTopoDataResponse {
    results: Vec<OpenTopoDataResult>,
}

async fn read_cache(redis_url: &str, key: &str) -> Option<Vec<GeospatialThreat>> {
    let client = redis::Client::open(redis_url).ok()?;
    let mut conn = client.get_multiplexed_async_connection().await.ok()?;
    let payload: Option<String> = conn.get(key).await.ok()?;
    payload.and_then(|raw| serde_json::from_str::<Vec<GeospatialThreat>>(&raw).ok())
}

async fn write_cache(redis_url: &str, key: &str, rows: &[GeospatialThreat]) {
    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            if let Ok(payload) = serde_json::to_string(rows) {
                // 6-hour TTL for preloaded elevation data
                let _: Result<(), _> = conn.set_ex(key, payload, 21600).await;
            }
        }
    }
}

pub async fn get_elevation_batch(
    locs: Vec<(f64, f64)>,
) -> Result<Vec<f64>, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let locations_str: Vec<String> = locs
        .iter()
        .map(|(lat, lon)| format!("{:.6},{:.6}", lat, lon))
        .collect();

    let url = format!(
        "https://api.opentopodata.org/v1/srtm90m?locations={}",
        locations_str.join("|")
    );

    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(15))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_text = resp.text().await.unwrap_or_default();
        return Err(format!("OpenTopoData HTTP {}: {}", status, err_text).into());
    }

    let json_text = resp
        .text()
        .await
        .map_err(|e| format!("Failed to get text: {}", e))?;
    let data: OpenTopoDataResponse = serde_json::from_str(&json_text)
        .map_err(|e| format!("Failed to decode OpenTopoData JSON: {} - Raw: {}", e, json_text))?;

    Ok(data.results.iter().map(|r| r.elevation).collect())
}

/// PRIORITY 7 — Preload elevation for all configured zones.
/// Called at boot and every 6 hours.
pub async fn fetch_elevation_for_all_zones(redis_url: &str) {
    let locations_env = env::var("ELEVATION_LOCATIONS")
        .unwrap_or_else(|_| "34.1526,77.5770|Ladakh;34.5553,76.1340|KargilSector".to_string());

    let mut query_locs: Vec<(f64, f64)> = Vec::new();
    let mut names: Vec<String> = Vec::new();

    for item in locations_env.split(';') {
        let parts: Vec<&str> = item.split('|').collect();
        if parts.len() == 2 {
            let coords: Vec<&str> = parts[0].split(',').collect();
            if coords.len() == 2 {
                if let (Ok(lat), Ok(lon)) = (coords[0].parse::<f64>(), coords[1].parse::<f64>()) {
                    query_locs.push((lat, lon));
                    names.push(parts[1].trim().to_string());
                }
            }
        }
    }

    if query_locs.is_empty() {
        log::warn!("[TERRAIN] No elevation zones configured");
        return;
    }

    let ts = Utc::now().to_rfc3339();
    match get_elevation_batch(query_locs).await {
        Ok(elevations) => {
            let mut results = Vec::new();
            for (idx, &elev) in elevations.iter().enumerate() {
                results.push(GeospatialThreat {
                    area: names.get(idx).cloned().unwrap_or_else(|| "unknown".to_string()),
                    elevation_m: elev,
                    nearby_incidents: 0,
                    terrain_score: (elev / 5000.0).min(1.0),
                    timestamp: ts.clone(),
                });
            }
            write_cache(redis_url, "cache:geo:latest", &results).await;
            emit_log(
                redis_url,
                "TERRAIN",
                "PRELOAD_COMPLETE",
                &format!("{} ZONES LOADED", results.len()),
            )
            .await;
            log::info!("[TERRAIN] Elevation preloaded for {} zones", results.len());
        }
        Err(e) => {
            emit_log(redis_url, "TERRAIN", "PRELOAD_ERROR", &e.to_string()).await;
            log::error!("[TERRAIN] Elevation preload failed: {}", e);
        }
    }
}

/// PRIORITY 7 — Start background elevation refresh loop (every 6 hours).
pub fn start_elevation_refresh(redis_url: String) {
    tokio::spawn(async move {
        loop {
            fetch_elevation_for_all_zones(&redis_url).await;
            tokio::time::sleep(Duration::from_secs(6 * 3600)).await;
        }
    });
}

pub async fn fetch_geospatial_threats(
    redis_url: &str,
    location: Option<&GeoPoint>,
) -> GenericFallbackResponse<GeospatialThreat> {
    let ts = Utc::now().to_rfc3339();
    let cache_key = match location {
        Some(loc) => format!("cache:geo:{:.4}:{:.4}", loc.lat, loc.lon),
        None => "cache:geo:latest".to_string(),
    };

    if let Some(rows) = read_cache(redis_url, &cache_key).await {
        emit_log(
            redis_url,
            "TERRAIN",
            "CACHE_HIT",
            &format!("{} zones", rows.len()),
        )
        .await;
        return GenericFallbackResponse {
            status: "ok".to_string(),
            provider_path: Some("redis_cache".to_string()),
            error: None,
            results: rows,
        };
    }

    let (query_locs, names): (Vec<(f64, f64)>, Vec<String>) = if let Some(loc) = location {
        (vec![(loc.lat, loc.lon)], vec![loc.name.clone()])
    } else {
        let locations_env = env::var("ELEVATION_LOCATIONS")
            .unwrap_or_else(|_| "34.1526,77.5770|Ladakh;34.5553,76.1340|KargilSector".to_string());

        let mut ql = Vec::new();
        let mut nl = Vec::new();

        for item in locations_env.split(';') {
            let parts: Vec<&str> = item.split('|').collect();
            if parts.len() == 2 {
                let coords: Vec<&str> = parts[0].split(',').collect();
                if coords.len() == 2 {
                    if let (Ok(lat), Ok(lon)) =
                        (coords[0].parse::<f64>(), coords[1].parse::<f64>())
                    {
                        ql.push((lat, lon));
                        nl.push(parts[1].trim().to_string());
                    }
                }
            }
        }
        (ql, nl)
    };

    if query_locs.is_empty() {
        return GenericFallbackResponse {
            status: "ok".to_string(),
            provider_path: Some("opentopodata".to_string()),
            error: None,
            results: vec![],
        };
    }

    match get_elevation_batch(query_locs).await {
        Ok(elevations) => {
            let mut results = Vec::new();
            for (idx, &elev) in elevations.iter().enumerate() {
                results.push(GeospatialThreat {
                    area: names.get(idx).cloned().unwrap_or_else(|| "unknown".to_string()),
                    elevation_m: elev,
                    nearby_incidents: 0,
                    terrain_score: (elev / 5000.0).min(1.0),
                    timestamp: ts.clone(),
                });
            }
            write_cache(redis_url, &cache_key, &results).await;
            emit_log(
                redis_url,
                "TERRAIN",
                "FETCH_COMPLETE",
                &format!("{} ZONES", results.len()),
            )
            .await;
            GenericFallbackResponse {
                status: "ok".to_string(),
                provider_path: Some("opentopodata".to_string()),
                error: None,
                results,
            }
        }
        Err(e) => {
            emit_log(redis_url, "TERRAIN", "FETCH_ERROR", &e.to_string()).await;
            GenericFallbackResponse {
                status: "provider_error".to_string(),
                provider_path: Some("opentopodata".to_string()),
                error: Some(format!("{}", e)),
                results: vec![],
            }
        }
    }
}
