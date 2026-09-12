use anyhow::{Context, Result};
use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub admin_api_token: Option<String>,
    pub gps_webhook_token: Option<String>,
    pub cors_origins: Vec<String>,
    pub ingest_enabled: bool,
    pub border_data_dir: String,
    pub redis_url: String,

    // OpenSky
    pub opensky_client_id: Option<String>,
    pub opensky_client_secret: Option<String>,
    pub opensky_bbox: OpenSkyBbox,

    // AISstream
    pub aisstream_api_key: Option<String>,
    pub aisstream_bboxes: String,

    // GPS
    pub geoapify_api_key: Option<String>,
    pub gps_api_key: Option<String>,
    pub gps_api_base_url: Option<String>,
    pub gps_poll_seconds: u64,

    // Weather
    pub openweather_api_key: Option<String>,

    // Sentinel
    pub sentinel_token_url: String,
    pub sentinel_process_url: String,
    pub sentinel_client_id: Option<String>,
    pub sentinel_client_secret: Option<String>,

    // Tavily
    pub tavily_api_key: Option<String>,
    pub tavily_max_per_day: u32,

    // Geofence
    pub border_corridor_km: f64,
    pub maritime_corridor_nm: f64,
    pub quiet_poll_seconds: u64,
}

#[derive(Clone, Debug)]
pub struct OpenSkyBbox {
    pub lamin: f64,
    pub lamax: f64,
    pub lomin: f64,
    pub lomax: f64,
}

impl OpenSkyBbox {
    fn parse(s: &str) -> Result<Self> {
        let parts: Vec<f64> = s
            .split(',')
            .map(|p| p.trim().parse::<f64>())
            .collect::<std::result::Result<_, _>>()
            .context("OPENSKY_BBOX must be lamin,lamax,lomin,lomax")?;
        if parts.len() != 4 {
            anyhow::bail!("OPENSKY_BBOX must have exactly 4 values");
        }
        if parts.iter().any(|v| !v.is_finite())
            || parts[0] < -90.0
            || parts[1] > 90.0
            || parts[2] < -180.0
            || parts[3] > 180.0
            || parts[0] >= parts[1]
            || parts[2] >= parts[3]
        {
            anyhow::bail!("OPENSKY_BBOX must be ordered, finite latitude/longitude bounds");
        }
        Ok(Self {
            lamin: parts[0],
            lamax: parts[1],
            lomin: parts[2],
            lomax: parts[3],
        })
    }
}

fn opt_env(key: &str) -> Option<String> {
    env::var(key).ok().filter(|v| !v.trim().is_empty())
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            admin_api_token: opt_env("ADMIN_API_TOKEN"),
            gps_webhook_token: opt_env("GPS_WEBHOOK_TOKEN"),
            cors_origins: env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:3000,http://127.0.0.1:3000".into())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            ingest_enabled: env::var("INGEST_ENABLED").unwrap_or_else(|_| "true".into()) != "false",
            border_data_dir: env::var("BORDER_DATA_DIR").unwrap_or_else(|_| {
                if std::path::Path::new("data/borders").is_dir() {
                    "data/borders".into()
                } else {
                    "backend/data/borders".into()
                }
            }),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8000".into())
                .parse()
                .context("PORT must be a number")?,
            redis_url: env::var("REDIS_URL").context("REDIS_URL is required")?,

            opensky_client_id: opt_env("OPENSKY_CLIENT_ID"),
            opensky_client_secret: opt_env("OPENSKY_CLIENT_SECRET"),
            opensky_bbox: OpenSkyBbox::parse(
                &env::var("OPENSKY_BBOX").unwrap_or_else(|_| "5.0,38.0,65.0,100.0".into()),
            )?,

            aisstream_api_key: opt_env("AISSTREAM_API_KEY"),
            aisstream_bboxes: env::var("AISSTREAM_BBOXES")
                .unwrap_or_else(|_| "[[[5.0,65.0],[38.0,100.0]]]".into()),

            geoapify_api_key: opt_env("GEOAPIFY_API_KEY").or_else(|| {
                let url = opt_env("GPS_API_BASE_URL")?;
                let host = reqwest::Url::parse(&url).ok()?.host_str()?.to_owned();
                if host == "geoapify.com" || host.ends_with(".geoapify.com") {
                    opt_env("GPS_API_KEY")
                } else {
                    None
                }
            }),
            gps_api_key: opt_env("GPS_API_KEY"),
            gps_api_base_url: opt_env("GPS_API_BASE_URL"),
            gps_poll_seconds: env::var("GPS_POLL_SECONDS")
                .unwrap_or_else(|_| "15".into())
                .parse::<u64>()
                .unwrap_or(15)
                .max(1),

            openweather_api_key: opt_env("OPENWEATHER_API_KEY"),

            sentinel_token_url: env::var("SENTINEL_TOKEN_URL")
                .unwrap_or_else(|_| "https://services.sentinel-hub.com/oauth/token".into()),
            sentinel_process_url: env::var("SENTINEL_PROCESS_URL")
                .unwrap_or_else(|_| "https://services.sentinel-hub.com/api/v1/process".into()),
            sentinel_client_id: opt_env("SENTINEL_CLIENT_ID"),
            sentinel_client_secret: opt_env("SENTINEL_CLIENT_SECRET"),

            tavily_api_key: opt_env("TAVILY_API_KEY"),
            tavily_max_per_day: env::var("TAVILY_MAX_PER_DAY")
                .unwrap_or_else(|_| "33".into())
                .parse()
                .unwrap_or(33),

            border_corridor_km: env::var("BORDER_CORRIDOR_KM")
                .unwrap_or_else(|_| "15".into())
                .parse()
                .unwrap_or(15.0),
            maritime_corridor_nm: env::var("MARITIME_CORRIDOR_NM")
                .unwrap_or_else(|_| "24".into())
                .parse()
                .unwrap_or(24.0),
            quiet_poll_seconds: env::var("QUIET_POLL_SECONDS")
                .unwrap_or_else(|_| "45".into())
                .parse()
                .unwrap_or(45),
        })
    }
}
