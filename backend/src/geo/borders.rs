use anyhow::{Context, Result};
use serde_json::Value;

/// Holds the raw GeoJSON FeatureCollections for map serving.
#[derive(Clone)]
pub struct BorderStore {
    /// Combined FeatureCollection (land + maritime) as raw JSON string for /api/borders
    pub geojson_str: String,
}

impl BorderStore {
    pub fn load(data_dir: &str) -> Result<Self> {
        let land_path = format!("{}/india_land_borders.geojson", data_dir);
        let maritime_path = format!("{}/india_maritime_borders.geojson", data_dir);

        let mut features: Vec<Value> = Vec::new();

        for path in &[&land_path, &maritime_path] {
            match read_geometry(path) {
                Ok(content) => {
                    let gj: Value = serde_json::from_str(&content)
                        .with_context(|| format!("Failed to parse {}", path))?;
                    if let Some(feats) = gj.get("features").and_then(|f| f.as_array()) {
                        features.extend(feats.iter().cloned());
                    }
                }
                Err(e) => {
                    return Err(e).with_context(|| format!("Cannot load border file {}", path));
                }
            }
        }

        let combined = serde_json::json!({
            "type": "FeatureCollection",
            "features": features
        });

        Ok(Self {
            geojson_str: serde_json::to_string(&combined)?,
        })
    }
}

/// Bundled fallback keeps deployments independent of the process working directory.
pub fn read_geometry(path: &str) -> std::io::Result<String> {
    match std::fs::read_to_string(path) {
        Ok(value) => Ok(value),
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound
                && std::env::var("BORDER_DATA_DIR").is_err() =>
        {
            match std::path::Path::new(path)
                .file_name()
                .and_then(|s| s.to_str())
            {
                Some("india_land_borders.geojson") => {
                    Ok(include_str!("../../data/borders/india_land_borders.geojson").into())
                }
                Some("india_maritime_borders.geojson") => {
                    Ok(include_str!("../../data/borders/india_maritime_borders.geojson").into())
                }
                _ => Err(error),
            }
        }
        Err(error) => Err(error),
    }
}
