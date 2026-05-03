use crate::models::{GenericFallbackResponse, WeatherThreat};
use crate::services::geo_resolver::GeoPoint;
use crate::services::ops_log::emit_log;
use chrono::Utc;
use redis::AsyncCommands;
use serde_json::Value;
use std::env;
use tokio::time::Duration;

async fn read_cache(redis_url: &str, key: &str) -> Option<Vec<WeatherThreat>> {
    let client = redis::Client::open(redis_url).ok()?;
    let mut conn = client.get_multiplexed_async_connection().await.ok()?;
    let payload: Option<String> = conn.get(key).await.ok()?;
    payload.and_then(|raw| serde_json::from_str::<Vec<WeatherThreat>>(&raw).ok())
}

async fn write_cache(redis_url: &str, key: &str, rows: &[WeatherThreat]) {
    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            if let Ok(payload) = serde_json::to_string(rows) {
                let ttl = env::var("OPENWEATHER_POLL_SECONDS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(300);
                let _: Result<(), _> = conn.set_ex(key, payload, ttl).await;
            }
        }
    }
}

fn risk_score(wind: f64, visibility: f64, rain_1h: f64) -> f64 {
    let wind_part = (wind / 25.0).min(1.0) * 0.4;
    let vis_part = ((10.0 - visibility.max(0.0).min(10.0)) / 10.0) * 0.4;
    let rain_part = (rain_1h / 10.0).min(1.0) * 0.2;
    (wind_part + vis_part + rain_part).min(1.0)
}

pub async fn fetch_weather_threats(redis_url: &str, location: Option<&GeoPoint>) -> GenericFallbackResponse<WeatherThreat> {
    let ts = Utc::now().to_rfc3339();
    let cache_key = match location {
        Some(loc) => format!("cache:weather:{:.4}:{:.4}", loc.lat, loc.lon),
        None => "cache:weather:latest".to_string(),
    };
    
    if let Some(rows) = read_cache(redis_url, &cache_key).await {
        return GenericFallbackResponse {
            status: "ok".to_string(),
            provider_path: Some("redis_cache".to_string()),
            error: None,
            results: rows,
        };
    }

    let api_key = env::var("OPENWEATHER_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return GenericFallbackResponse {
            status: "provider_missing".to_string(),
            provider_path: Some("openweather".to_string()),
            error: Some("OPENWEATHER_API_KEY missing".to_string()),
            results: vec![],
        };
    }

    let client = reqwest::Client::builder()
        .user_agent("BharatIntelQuantum/1.0")
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    // Use provided location or fall back to environment zones
    let locations: Vec<(f64, f64, String)> = if let Some(loc) = location {
        vec![(loc.lat, loc.lon, loc.name.clone())]
    } else {
        let zones = env::var("WEATHER_ZONES")
            .unwrap_or_else(|_| "34.1526,77.5770|Ladakh;27.1767,78.0081|Agra".to_string());
        zones.split(';')
            .filter_map(|zone| {
                let parts: Vec<&str> = zone.split('|').collect();
                if parts.len() != 2 { return None; }
                let coord: Vec<&str> = parts[0].split(',').collect();
                if coord.len() != 2 { return None; }
                let lat = coord[0].trim().parse::<f64>().ok()?;
                let lon = coord[1].trim().parse::<f64>().ok()?;
                Some((lat, lon, parts[1].trim().to_string()))
            })
            .collect()
    };

    let mut output: Vec<WeatherThreat> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for (lat, lon, name) in locations {
        let lat_str = format!("{:.6}", lat);
        let lon_str = format!("{:.6}", lon);
        match client
            .get("https://api.openweathermap.org/data/2.5/weather")
            .query(&[
                ("lat", lat_str.as_str()),
                ("lon", lon_str.as_str()),
                ("appid", api_key.as_str()),
                ("units", "metric"),
            ])
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<Value>().await {
                    Ok(json) => {
                        let wind = json["wind"]["speed"].as_f64().unwrap_or(0.0);
                        let visibility = json["visibility"].as_f64().unwrap_or(10000.0) / 1000.0;
                        let rain = json["rain"]["1h"].as_f64().unwrap_or(0.0);
                        let rs = risk_score(wind, visibility, rain);
                        let mut wt = WeatherThreat {
                            zone: name.to_string(),
                            wind_speed: wind,
                            visibility,
                            rain_1h: rain,
                            risk_score: rs,
                            risk_level: "LOW".to_string(),
                            operational_impact: "Normal conditions".to_string(),
                            timestamp: ts.clone(),
                        };
                        wt.calculate_operational_risk();
                        output.push(wt);
                    }
                    Err(e) => errors.push(format!("OpenWeather parse failed for {}: {}", name, e)),
                }
            }
            Ok(resp) => errors.push(format!("OpenWeather HTTP {} for {}", resp.status(), name)),
            Err(e) => errors.push(format!("OpenWeather request failed for {}: {}", name, e)),
        }
    }

    if !output.is_empty() {
        write_cache(redis_url, &cache_key, &output).await;
        emit_log(redis_url, "WEATHER", "FETCH_COMPLETE", &format!("{} ZONES", output.len())).await;
        GenericFallbackResponse {
            status: "ok".to_string(),
            provider_path: Some("openweather".to_string()),
            error: None,
            results: output,
        }
    } else {
        emit_log(redis_url, "WEATHER", "FETCH_ERROR", &errors.join("; ")).await;
        GenericFallbackResponse {
            status: "provider_missing".to_string(),
            provider_path: Some("openweather".to_string()),
            error: Some(errors.join("; ")),
            results: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::risk_score;

    #[test]
    fn risk_score_stays_between_zero_and_one() {
        let s = risk_score(20.0, 2.0, 4.0);
        assert!(s > 0.0 && s <= 1.0);
    }
}
