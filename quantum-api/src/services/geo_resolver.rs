use log::info;
use serde::{Deserialize, Serialize};
use std::env;
use tokio::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoPoint {
    pub lat: f64,
    pub lon: f64,
    pub name: String,
    pub country: Option<String>,
}

#[derive(Deserialize, Debug)]
struct NominatimResponse {
    lat: String,
    lon: String,
    display_name: String,
    #[serde(default)]
    address: Option<NominatimAddress>,
}

#[derive(Deserialize, Debug)]
struct NominatimAddress {
    #[serde(default)]
    country: Option<String>,
}

/// Resolve a location query to lat/lon using OpenStreetMap Nominatim API
pub async fn resolve_location(query: &str) -> Option<GeoPoint> {
    if query.trim().is_empty() {
        return None;
    }

    info!("GeoResolver Query: {}", query);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("BIQ-Intel-System/1.0 (Research Project)")
        .build()
        .ok()?;

    let encoded = urlencoding::encode(query);
    let url = format!(
        "https://nominatim.openstreetmap.org/search?q={}&format=json&limit=1",
        encoded
    );

    info!("GeoResolver URL: {}", url);

    let resp = match client.get(&url).send().await {
        Ok(r) => {
            info!("GeoResolver Response Status: {}", r.status());
            r
        }
        Err(e) => {
            info!("GeoResolver Request Failed: {}", e);
            return None;
        }
    };
    
    if !resp.status().is_success() {
        info!("GeoResolver HTTP Error: {}", resp.status());
        return None;
    }

    let body = match resp.text().await {
        Ok(b) => b,
        Err(e) => {
            info!("GeoResolver Body Read Failed: {}", e);
            return None;
        }
    };
    info!("GeoResolver Response Body: {}", &body[..body.len().min(200)]);

    let results: Vec<NominatimResponse> = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            info!("GeoResolver JSON Parse Failed: {}", e);
            return None;
        }
    };
    
    if results.is_empty() {
        info!("GeoResolver: No results found");
        return None;
    }

    let result = &results[0];
    let lat = result.lat.parse::<f64>().ok()?;
    let lon = result.lon.parse::<f64>().ok()?;
    
    // Extract just the name part (before first comma) for cleaner display
    let name = result.display_name.split(',').next().unwrap_or(&result.display_name).trim().to_string();

    info!("GeoResolver SUCCESS: {} at {:.4}, {:.4}", name, lat, lon);

    Some(GeoPoint {
        lat,
        lon,
        name,
        country: result.address.as_ref().and_then(|a| a.country.clone()),
    })
}

/// Parse coordinates from query string like "34.1526,77.5770" or "karachi"
pub fn parse_coordinates(query: &str) -> Option<GeoPoint> {
    // Try parsing as "lat,lon" format
    let parts: Vec<&str> = query.split(',').collect();
    if parts.len() == 2 {
        if let (Ok(lat), Ok(lon)) = (parts[0].trim().parse::<f64>(), parts[1].trim().parse::<f64>()) {
            if (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon) {
                return Some(GeoPoint {
                    lat,
                    lon,
                    name: format!("{:.4},{:.4}", lat, lon),
                    country: None,
                });
            }
        }
    }
    None
}

/// Get location from query or resolve via Nominatim
pub async fn get_location(query: &str) -> Option<GeoPoint> {
    // First try parsing as coordinates
    if let Some(coord) = parse_coordinates(query) {
        return Some(coord);
    }

    // Then try Nominatim geocoding
    resolve_location(query).await
}

/// Get default zones from environment
pub fn get_default_zones() -> Vec<GeoPoint> {
    let zones_str = env::var("WEATHER_ZONES")
        .or_else(|_| env::var("ELEVATION_LOCATIONS"))
        .unwrap_or_else(|_| "34.1526,77.5770|Ladakh;34.5553,76.1340|KargilSector".to_string());

    zones_str
        .split(';')
        .filter_map(|zone| {
            let parts: Vec<&str> = zone.split('|').collect();
            if parts.len() != 2 {
                return None;
            }
            let coord = parse_coordinates(parts[0])?;
            Some(GeoPoint {
                lat: coord.lat,
                lon: coord.lon,
                name: parts[1].trim().to_string(),
                country: None,
            })
        })
        .collect()
}

/// Convert lat/lon to bounding box for AIS/stream queries
pub fn to_bounding_box(lat: f64, lon: f64, radius_km: f64) -> ((f64, f64), (f64, f64)) {
    // Rough approximation: 1 degree ≈ 111 km
    let delta = radius_km / 111.0;
    (
        ((lat - delta).max(-90.0), (lon - delta).max(-180.0)),
        ((lat + delta).min(90.0), (lon + delta).min(180.0)),
    )
}
