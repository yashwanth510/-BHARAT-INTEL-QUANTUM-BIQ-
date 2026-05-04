/**
 * BIQ API Client — typed wrappers around all backend endpoints.
 * Uses Next.js rewrites in dev; nginx proxy in production.
 * All calls include timeout + error normalisation.
 */

const BASE = process.env.NEXT_PUBLIC_API_URL ?? "https://bharat-intel-quantum-biq-production.up.railway.app";
const TIMEOUT_MS = 15_000;

async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const controller = new AbortController();
  const id = setTimeout(() => controller.abort(), TIMEOUT_MS);
  try {
    const res = await fetch(`${BASE}${path}`, {
      ...init,
      signal: controller.signal,
    });
    clearTimeout(id);
    if (!res.ok) {
      const body = await res.text().catch(() => "");
      throw new Error(`[BIQ API] ${path} → HTTP ${res.status}: ${body}`);
    }
    return res.json() as Promise<T>;
  } catch (err) {
    clearTimeout(id);
    throw err;
  }
}

// ── Types ────────────────────────────────────────────────────────────────────

export interface ProviderStatus {
  provider: string;
  status: string;
  details?: string;
}

export interface HealthResponse {
  status: string;
  integrity: string;
  services: number;
  providers: ProviderStatus[];
}

export interface FusionResult {
  score: number;
  risk: string;
  recommendations: string[];
}

export interface UnifiedIntelligenceResponse {
  correlation_id: string;
  location: string;
  news: unknown;
  maritime: unknown;
  weather: unknown;
  satellite: unknown;
  fusion: FusionResult;
  strategic_synthesis?: string;
  metadata: {
    freshness: string;
    confidence: number;
    integrity_score: number;
  };
}

export interface OpsLogEntry {
  timestamp: string;
  category: string;
  event: string;
  detail: string;
}

export interface GraphData {
  nodes: Array<{
    id: string;
    label: string;
    type: string;
    name: string;
    properties?: Record<string, unknown>;
  }>;
  edges: Array<{
    source: string;
    target: string;
    label: string;
  }>;
}

export interface MaritimeThreat {
  mmsi: string;
  vessel_name: string;
  lat: number;
  lon: number;
  status: string;
  dark: boolean;
  timestamp: string;
}

export interface ThreatCorrelation {
  correlation_id: string;
  score: number;
  risk_score: number;
  level: string;
  explanation: string;
  key_actors: string[];
  key_locations: string[];
}

// ── Endpoints ────────────────────────────────────────────────────────────────

export const api = {
  /** System health + provider reachability */
  health: () => apiFetch<HealthResponse>("/health"),

  /** Quantum system status */
  quantumHealth: () => apiFetch<Record<string, string>>("/quantum-health"),

  /** Scheduler & quota metrics */
  metrics: () => apiFetch<Record<string, unknown>>("/metrics"),

  /** Operations log (last N entries) */
  opsLog: () =>
    apiFetch<{ entries: OpsLogEntry[]; count: number }>("/ops-log"),

  /** Unified AI intelligence (multi-source fusion) */
  intelligence: (query = "latest border activity") =>
    apiFetch<UnifiedIntelligenceResponse>(
      `/api/intelligence?query=${encodeURIComponent(query)}`
    ),

  /** OSINT threat correlation (Mistral) */
  threatCorrelation: (query = "latest border activity") =>
    apiFetch<ThreatCorrelation>(
      `/api/threat-correlation?query=${encodeURIComponent(query)}`
    ),

  /** Maritime domain awareness */
  maritimeThreats: () =>
    apiFetch<{ results: MaritimeThreat[]; status: string }>(
      "/maritime-threats"
    ),

  /** News-based threat signals */
  newsThreats: () => apiFetch<{ results: unknown[]; status: string }>("/news-threats"),

  /** Weather tactical layer */
  weatherThreats: () =>
    apiFetch<{ results: unknown[]; status: string }>("/weather-threats"),

  /** Sentinel / satellite alerts */
  satelliteAlerts: () =>
    apiFetch<{ results: unknown[]; status: string }>("/satellite-alerts"),

  /** ML anomaly scoring */
  mlAnomaly: () => apiFetch<Record<string, unknown>>("/ml-anomaly"),

  /** Pakistan border threat cache */
  pakistanThreats: () => apiFetch<unknown[]>("/pakistan-threats"),

  /** China border threat cache */
  chinaThreats: () => apiFetch<unknown[]>("/china-threats"),

  /** Crypto / financial intelligence */
  cryptoThreats: () =>
    apiFetch<{ results: unknown[]; status: string }>("/crypto-threats"),

  /** Neo4j knowledge graph export */
  graphData: () => apiFetch<GraphData>("/api/graph/data"),

  /** Strike analysis */
  strikeAnalysis: (target: string) =>
    apiFetch<Record<string, unknown>>(
      `/api/tactical/strike-analysis?target=${encodeURIComponent(target)}`
    ),

  /** Border penetration risk */
  borderPenetration: () =>
    apiFetch<Record<string, unknown>>("/api/tactical/border-penetration"),
};
