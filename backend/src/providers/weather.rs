use crate::models::Weather;
use anyhow::Result;
use serde::Deserialize;

pub async fn fetch(client: &reqwest::Client, api_key: &str, lat: f64, lon: f64) -> Result<Weather> {
    let url = format!(
        "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&appid={}&units=metric",
        lat, lon, api_key
    );
    let resp: OWResponse = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let w = resp.weather.first().cloned().unwrap_or_default();
    Ok(Weather {
        lat,
        lon,
        description: w.description,
        temp_c: resp.main.temp,
        feels_like_c: resp.main.feels_like,
        humidity_pct: resp.main.humidity,
        wind_speed_ms: resp.wind.speed,
        wind_deg: resp.wind.deg,
        visibility_m: resp.visibility,
        icon: w.icon,
    })
}

#[derive(Deserialize)]
struct OWResponse {
    weather: Vec<OWWeather>,
    main: OWMain,
    wind: OWWind,
    visibility: Option<u32>,
}

#[derive(Deserialize, Default, Clone)]
struct OWWeather {
    description: String,
    icon: String,
}

#[derive(Deserialize)]
struct OWMain {
    temp: f64,
    feels_like: f64,
    humidity: u32,
}

#[derive(Deserialize)]
struct OWWind {
    speed: f64,
    #[serde(default)]
    deg: u32,
}
