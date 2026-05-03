use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GlobalThreat {
    pub actor: String,           // "Hafiz Saeed"
    pub country: String,         // "PK"
    pub confidence: f64,         // 0.92
    pub sources: Vec<String>,    // ["X: @pakdefence", "Dawn: article123"]
    pub location: Option<String>, // "Bahawalpur"
    pub timestamp: String,
}

/// Crypto wallet threat information
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CryptoThreat {
    pub wallet_address: String,
    pub risk_score: f64,
    pub sanctions_list: String,
    pub source: String,
    pub country: String,
    pub timestamp: String,
}

/// Wallet screening result from OFAC + MistTrack
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScreeningResult {
    pub wallet_address: String,
    pub risk_level: String,     // "CRITICAL", "Low", "Medium", "High", "Severe"
    pub risk_score: f64,        // 0.0 - 1.0
    pub source: String,         // "OFAC_SDN" or "MISTTRACK"
    pub entity: String,         // Entity name or labels
    pub category: String,       // Programs or category
    pub timestamp: String,
}

/// Flight threat information
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FlightThreat {
    pub flight_id: String,
    pub origin: String,
    pub destination: String,
    pub risk_score: f64,
    pub source: String,
    pub timestamp: String,
}

/// Satellite alert information
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SatelliteAlert {
    pub alert_id: String,
    pub region: String,
    pub alert_type: String,
    pub confidence: f64,
    pub source: String,
    pub timestamp: String,
}

/// Generic fallback response for any provider
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GenericFallbackResponse<T> {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub results: Vec<T>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MaritimeThreat {
    pub vessel_id: String,
    pub vessel_name: String,
    pub lat: f64,
    pub lon: f64,
    pub risk_score: f64,
    pub port: String,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NewsThreat {
    pub title: String,
    pub source: String,
    pub severity: String,
    pub keywords: Vec<String>,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WeatherThreat {
    pub zone: String,
    pub wind_speed: f64,
    pub visibility: f64,
    pub rain_1h: f64,
    pub risk_score: f64,
    pub risk_level: String, // PART 11: Weather to Risk
    pub operational_impact: String, // Operational risk description
    pub timestamp: String,
}

impl WeatherThreat {
    /// Convert raw weather to operational risk level
    pub fn calculate_operational_risk(&mut self) {
        self.risk_level = if self.risk_score > 0.8 {
            "CRITICAL".to_string()
        } else if self.risk_score > 0.6 {
            "HIGH".to_string()
        } else if self.risk_score > 0.3 {
            "MODERATE".to_string()
        } else {
            "LOW".to_string()
        };

        self.operational_impact = match self.risk_level.as_str() {
            "CRITICAL" => "Operations suspended. High wind/rain danger.".to_string(),
            "HIGH" => "Limited visibility. Caution required for patrols.".to_string(),
            "MODERATE" => "Normal ops with weather monitoring.".to_string(),
            _ => "Favorable conditions for operations.".to_string(),
        };
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeospatialThreat {
    pub area: String,
    pub elevation_m: f64,
    pub nearby_incidents: u32,
    pub terrain_score: f64,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CorrelationResult {
    pub correlation_id: String,
    pub risk_score: f64,
    pub explanation: String,
    pub top_contributors: Vec<String>,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AnomalyResult {
    pub item_id: String,
    pub anomaly_score: f64,
    pub is_flagged: bool,
    pub factors: Vec<String>,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProviderStatus {
    pub provider: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HealthResponse {
    pub status: String,
    pub services: u8,
    pub integrity: String,
    pub providers: Vec<ProviderStatus>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FusionResult {
    pub score: f64,
    pub risk: String,
    pub recommendations: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UnifiedIntelligenceResponse {
    pub correlation_id: String,
    pub location: String,
    pub news: serde_json::Value,
    pub maritime: serde_json::Value,
    pub weather: serde_json::Value,
    pub satellite: serde_json::Value,
    pub fusion: FusionResult,
    pub strategic_synthesis: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QuantumHealth {
    pub kyber1024: String,
    pub public_key: String,
    pub neo4j: String,
}
