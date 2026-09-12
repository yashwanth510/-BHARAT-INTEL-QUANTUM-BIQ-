export interface Flight {
  icao24: string;
  callsign?: string;
  lat: number;
  lon: number;
  altitude_m?: number;
  velocity_ms?: number;
  heading?: number;
  on_ground: boolean;
  updated_at: string;
}

export interface Vessel {
  mmsi: string;
  name?: string;
  lat: number;
  lon: number;
  speed_knots?: number;
  course?: number;
  ship_type?: number;
  updated_at: string;
}

export interface Vehicle {
  device_id: string;
  label?: string;
  lat: number;
  lon: number;
  speed_kmh?: number;
  heading?: number;
  updated_at: string;
}

export type AnomalySeverity = 'critical' | 'high' | 'elevated' | 'info';

export interface AnomalyEntity {
  kind: 'flight' | 'vessel' | 'vehicle';
  id: string;
  lat: number;
  lon: number;
}

export interface Anomaly {
  id: string;
  severity: AnomalySeverity;
  reasons: string[];
  entity: AnomalyEntity;
  zone_id: string;
  ts: string;
}

export interface WeatherData {
  lat: number;
  lon: number;
  description: string;
  temp_c: number;
  feels_like_c: number;
  humidity_pct: number;
  wind_speed_ms: number;
  wind_deg: number;
  visibility_m?: number;
  icon: string;
}

export interface TavilyResult {
  title: string;
  url: string;
  snippet: string;
}

export interface SentinelSnapshot {
  image_b64?: string;
  image_url?: string;
  bbox: [number, number, number, number];
  acquired?: string;
}

export type WsMessage =
  | { type: 'flight_update' } & Flight
  | { type: 'vessel_update' } & Vessel
  | { type: 'vehicle_update' } & Vehicle
  | { type: 'anomaly' } & Anomaly
  | { type: 'heartbeat'; ts: string };
