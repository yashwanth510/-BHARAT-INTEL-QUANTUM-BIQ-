use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SentinelSnapshot {
    pub image_b64: Option<String>,
    pub image_url: Option<String>,
    pub bbox: [f64; 4],
    pub acquired: Option<String>,
}

pub async fn fetch_snapshot(
    client: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    token_url: &str,
    process_url: &str,
    lat: f64,
    lon: f64,
    bbox_deg: f64,
) -> Result<SentinelSnapshot> {
    // 1. Get OAuth token
    let token = get_token(client, client_id, client_secret, token_url).await?;

    let delta = bbox_deg / 2.0;
    let bbox = [lon - delta, lat - delta, lon + delta, lat + delta];

    // 2. Process API request for true-color image
    let body = serde_json::json!({
        "input": {
            "bounds": {
                "bbox": bbox,
                "properties": { "crs": "http://www.opengis.net/def/crs/EPSG/0/4326" }
            },
            "data": [{
                "dataFilter": { "timeRange": {
                    "from": recent_date_from(),
                    "to": chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
                }},
                "type": "sentinel-2-l2a"
            }]
        },
        "output": {
            "width": 512,
            "height": 512,
            "responses": [{ "identifier": "default", "format": { "type": "image/jpeg" } }]
        },
        "evalscript": TRUE_COLOR_EVALSCRIPT
    });

    let resp = client
        .post(process_url)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await?;

    let resp = resp.error_for_status()?;

    let bytes = resp.bytes().await?;
    let image_b64 = Some(B64.encode(&bytes));

    Ok(SentinelSnapshot {
        image_b64,
        image_url: None,
        bbox,
        acquired: None, // Process API does not return an acquisition timestamp.
    })
}

async fn get_token(
    client: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    token_url: &str,
) -> Result<String> {
    #[derive(Deserialize)]
    struct TokenResp {
        access_token: String,
    }

    let resp: TokenResp = client
        .post(token_url)
        .form(&[
            ("grant_type", "client_credentials"),
            ("client_id", client_id),
            ("client_secret", client_secret),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(resp.access_token)
}

fn recent_date_from() -> String {
    let thirty_days_ago = chrono::Utc::now() - chrono::Duration::days(30);
    thirty_days_ago.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

const TRUE_COLOR_EVALSCRIPT: &str = r#"
//VERSION=3
function setup() {
  return { input: [{ bands: ["B02","B03","B04"] }], output: { bands: 3 } };
}
function evaluatePixel(sample) {
  return [sample.B04 * 3.5, sample.B03 * 3.5, sample.B02 * 3.5];
}
"#;
