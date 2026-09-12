use geo::{Contains, LineString, Point, Polygon};
use serde_json::Value;

/// Research corridors around the supplied geometry. Accuracy is limited by the input data.
pub struct GeofenceEngine {
    zones: Vec<Zone>,
}
struct Zone {
    id: String,
    hotspot: bool,
    width_km: f64,
    lines: Vec<Vec<[f64; 2]>>,
    polygon: Option<Polygon<f64>>,
}
#[derive(Clone, Debug)]
pub struct ZoneHit {
    pub zone_id: String,
    pub is_hotspot: bool,
}

impl GeofenceEngine {
    pub fn build(data_dir: &str, land_km: f64, maritime_nm: f64) -> Self {
        let mut zones = Vec::new();
        for (file, maritime) in [
            ("india_land_borders.geojson", false),
            ("india_maritime_borders.geojson", true),
        ] {
            let content = match super::borders::read_geometry(&format!("{data_dir}/{file}")) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let data: Value = match serde_json::from_str(&content) {
                Ok(d) => d,
                Err(_) => continue,
            };
            for feature in data["features"].as_array().into_iter().flatten() {
                let id = feature["properties"]["id"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_owned();
                let geometry = &feature["geometry"];
                let coords = &geometry["coordinates"];
                let points = |value: &Value| -> Vec<[f64; 2]> {
                    value
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|v| Some([v[0].as_f64()?, v[1].as_f64()?]))
                        .collect()
                };
                let (lines, polygon) = match geometry["type"].as_str() {
                    Some("LineString") => (vec![points(coords)], None),
                    Some("MultiLineString") => (
                        coords
                            .as_array()
                            .into_iter()
                            .flatten()
                            .map(points)
                            .collect(),
                        None,
                    ),
                    Some("Polygon") => {
                        let rings: Vec<LineString<f64>> = coords
                            .as_array()
                            .into_iter()
                            .flatten()
                            .map(|r| {
                                LineString::from(
                                    points(r).iter().map(|p| (p[0], p[1])).collect::<Vec<_>>(),
                                )
                            })
                            .collect();
                        (
                            vec![],
                            rings
                                .first()
                                .cloned()
                                .map(|outer| Polygon::new(outer, rings[1..].to_vec())),
                        )
                    }
                    _ => continue,
                };
                let hotspot = [
                    "india_china_lac",
                    "india_pakistan_loc",
                    "india_pakistan_sir_creek",
                    "india_maritime_andaman",
                    "india_maritime_gujarat",
                ]
                .contains(&id.as_str());
                zones.push(Zone {
                    id,
                    hotspot,
                    width_km: if maritime {
                        maritime_nm * 1.852
                    } else {
                        land_km
                    },
                    lines,
                    polygon,
                });
            }
        }
        tracing::info!("Loaded {} illustrative corridor zones", zones.len());
        Self { zones }
    }
    pub fn check(&self, lat: f64, lon: f64) -> Vec<ZoneHit> {
        self.zones
            .iter()
            .filter(|zone| {
                zone.polygon
                    .as_ref()
                    .is_some_and(|p| p.contains(&Point::new(lon, lat)))
                    || zone.lines.iter().any(|line| {
                        line.windows(2).any(|segment| {
                            distance_km([lon, lat], segment[0], segment[1]) <= zone.width_km
                        })
                    })
            })
            .map(|zone| ZoneHit {
                zone_id: zone.id.clone(),
                is_hotspot: zone.hotspot,
            })
            .collect()
    }
}

// Local equirectangular projection: suitable for short segments at Indian latitudes.
// Crucially, it measures distance to each line, not the rectangle enclosing an entire border.
fn distance_km(point: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let scale = point[1].to_radians().cos();
    let project = |v: [f64; 2]| {
        [
            (v[0] - point[0]) * 111.195 * scale,
            (v[1] - point[1]) * 111.195,
        ]
    };
    let a = project(a);
    let b = project(b);
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let length = dx * dx + dy * dy;
    let t = if length == 0.0 {
        0.0
    } else {
        (-(a[0] * dx + a[1] * dy) / length).clamp(0.0, 1.0)
    };
    (a[0] + t * dx).hypot(a[1] + t * dy)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn excludes_points_inside_bbox_but_far_from_line() {
        assert!(distance_km([0.0, 1.0], [0.0, 0.0], [1.0, 1.0]) > 50.0);
    }
    #[test]
    fn includes_points_on_line_and_handles_single_vertex() {
        assert!(distance_km([0.5, 0.5], [0.0, 0.0], [1.0, 1.0]) < 0.01);
        assert_eq!(distance_km([1.0, 1.0], [1.0, 1.0], [1.0, 1.0]), 0.0);
    }
}
