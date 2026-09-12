'use client';

import { useEffect, useRef, useState, useCallback } from 'react';
import * as maplibregl from 'maplibre-gl';
import 'maplibre-gl/dist/maplibre-gl.css';

import { MapboxOverlay } from '@deck.gl/mapbox';
import { GeoJsonLayer, ScatterplotLayer } from '@deck.gl/layers';

import type {
  Flight, Vessel, Vehicle, Anomaly, WeatherData, TavilyResult, SentinelSnapshot,
} from '@/lib/types';
import {
  fetchFlights, fetchVessels, fetchVehicles, fetchAnomalies, fetchBorders,
  fetchWeather, fetchSatellite, fetchOsint, fetchLocation,
} from '@/lib/api';
import { useWebSocket } from '@/lib/useWebSocket';
import WeatherCard from './WeatherCard';
import AnomalyDrawer from './AnomalyDrawer';

const INITIAL_VIEW = {
  longitude: 80.0,
  latitude: 22.0,
  zoom: 4.5,
  pitch: 0,
  bearing: 0,
};

const SEVERITY_COLOR: Record<string, [number, number, number, number]> = {
  critical:  [239, 68, 68, 255],
  high:      [249, 115, 22, 255],
  elevated:  [234, 179, 8, 255],
  info:      [148, 163, 184, 200],
};

export default function BiqMap() {
  const containerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<maplibregl.Map | null>(null);
  const overlayRef = useRef<MapboxOverlay | null>(null);

  const [flights, setFlights] = useState<Flight[]>([]);
  const [vessels, setVessels] = useState<Vessel[]>([]);
  const [vehicles, setVehicles] = useState<Vehicle[]>([]);
  const [anomalies, setAnomalies] = useState<Anomaly[]>([]);
  const [borders, setBorders] = useState<any>(null);

  const [layers, setLayers] = useState({
    borders: true,
    flights: true,
    vessels: true,
    vehicles: true,
    anomalies: true,
  });

  const [weatherError, setWeatherError] = useState<string | null>(null);
  const [locationLabel, setLocationLabel] = useState<string | null>(null);
  const weatherRequest = useRef(0);
  const anomalyRequest = useRef(0);
  const [dataError, setDataError] = useState<string | null>(null);
  const [selectedTrack, setSelectedTrack] = useState<string | null>(null);
  const [imagery, setImagery] = useState(false);
  const [weather, setWeather] = useState<WeatherData | null>(null);
  const [weatherPin, setWeatherPin] = useState<{ lat: number; lon: number } | null>(null);
  const [selectedAnomaly, setSelectedAnomaly] = useState<Anomaly | null>(null);
  const [drawerData, setDrawerData] = useState<{
    osint: TavilyResult[];
    sentinel: SentinelSnapshot | null;
    loading: boolean;
    osintError?: string;
    sentinelError?: string;
  } | null>(null);



  // Refresh snapshots as well as streaming: stale Redis entities disappear after TTL.
  useEffect(() => {
    let stopped = false;
    const refresh = () => {
      Promise.all([fetchFlights(), fetchVessels(), fetchVehicles(), fetchAnomalies(), fetchBorders()])
        .then(([air, sea, land, alerts, geometry]) => {
          if (stopped) return;
          setFlights(air); setVessels(sea); setVehicles(land); setAnomalies(alerts); setBorders(geometry); setDataError(null);
        }).catch(() => { if (!stopped) setDataError('Live data unavailable. Retrying…'); });
    };
    refresh();
    const timer = setInterval(refresh, 30000);
    return () => { stopped = true; clearInterval(timer); };
  }, []);

  // WebSocket live updates
  const handleWsMessage = useCallback((msg: any) => {
    if (msg.type === 'flight_update') {
      setFlights(prev => upsert(prev, msg, 'icao24'));
    } else if (msg.type === 'vessel_update') {
      setVessels(prev => upsert(prev, msg, 'mmsi'));
    } else if (msg.type === 'vehicle_update') {
      setVehicles(prev => upsert(prev, msg, 'device_id'));
    } else if (msg.type === 'anomaly') {
      setAnomalies(prev => [msg, ...prev.filter(a => a.id !== msg.id)].slice(0, 200));
    }
  }, []);

  const connected = useWebSocket(handleWsMessage);

  // Map click → weather
  const handleMapClick = useCallback(async (event: any) => {
    // If click was on a deck.gl object, don't trigger weather
    if (event.object) return;

    const { coordinate } = event;
    if (!coordinate) return;
    const [lon, lat] = coordinate;

    const request = ++weatherRequest.current;
    setWeatherPin({ lat, lon }); setWeather(null); setWeatherError(null); setLocationLabel(null);
    fetchLocation(lat, lon).then(result => { if (request === weatherRequest.current) setLocationLabel(result.label); }).catch(() => {});
    try {
      const result = await fetchWeather(lat, lon);
      if (request === weatherRequest.current) setWeather(result);
    } catch (error) {
      if (request === weatherRequest.current) setWeatherError(error instanceof Error ? error.message : 'Weather unavailable');
    }
  }, []);

  // Anomaly click → drawer
  const handleAnomalyClick = useCallback(async (anomaly: Anomaly) => {
    const request = ++anomalyRequest.current;
    setSelectedAnomaly(anomaly);
    setDrawerData({ osint: [], sentinel: null, loading: true });

    const [osintRes, sentinelRes] = await Promise.allSettled([
      fetchOsint(anomaly.entity.lat, anomaly.entity.lon, anomaly.entity.id),
      fetchSatellite(anomaly.entity.lat, anomaly.entity.lon),
    ]);

    if (request !== anomalyRequest.current) return;
    setDrawerData({
      osint: osintRes.status === 'fulfilled' ? osintRes.value.results ?? [] : [],
      sentinel: sentinelRes.status === 'fulfilled' ? sentinelRes.value : null,
      loading: false,
      osintError: osintRes.status === 'rejected' ? String(osintRes.reason.message) : undefined,
      sentinelError: sentinelRes.status === 'rejected' ? String(sentinelRes.reason.message) : undefined,
    });
  }, []);

  // Build deck.gl layers
  const deckLayers = [
    weatherPin && new ScatterplotLayer({ id: 'weather-pin', data: [weatherPin], getPosition: (d: {lon:number;lat:number}) => [d.lon,d.lat], getRadius: 7, radiusUnits: 'pixels', getFillColor: [255,255,255,240], stroked: true, getLineColor: [56,189,248], lineWidthMinPixels: 2 }),
    layers.borders && borders && new GeoJsonLayer({
      id: 'borders',
      data: borders,
      pickable: false,
      stroked: true,
      filled: false,
      getLineColor: (f: any) => {
        const t = f.properties?.type;
        if (t === 'maritime') return [56, 189, 248, 200];
        return [239, 68, 68, 220];
      },
      getLineWidth: 2,
      lineWidthUnits: 'pixels',
    }),

    layers.flights && new ScatterplotLayer<Flight>({
      id: 'flights',
      data: flights,
      pickable: true,
      getPosition: (d) => [d.lon, d.lat, (d.altitude_m ?? 0)],
      getRadius: 4,
      radiusUnits: 'pixels',
      getFillColor: [125, 211, 252, 220],
      onClick: (info) => {
        if (info.object) {
          const f = info.object;
          setSelectedTrack(`Flight: ${f.callsign ?? f.icao24}\nAlt: ${f.altitude_m?.toFixed(0) ?? 'N/A'} m\nSpeed: ${f.velocity_ms?.toFixed(0) ?? 'N/A'} m/s\nHeading: ${f.heading?.toFixed(0) ?? 'N/A'}°`);
        }
      },
    }),

    layers.vessels && new ScatterplotLayer<Vessel>({
      id: 'vessels',
      data: vessels,
      pickable: true,
      getPosition: (d) => [d.lon, d.lat, 0],
      getRadius: 5,
      radiusUnits: 'pixels',
      getFillColor: [34, 197, 94, 220],
      onClick: (info) => {
        if (info.object) {
          const v = info.object;
          setSelectedTrack(`Vessel: ${v.name ?? v.mmsi}\nSpeed: ${v.speed_knots?.toFixed(1) ?? 'N/A'} kn\nCourse: ${v.course?.toFixed(0) ?? 'N/A'}°`);
        }
      },
    }),

    layers.vehicles && new ScatterplotLayer<Vehicle>({
      id: 'vehicles',
      data: vehicles,
      pickable: true,
      getPosition: (d) => [d.lon, d.lat, 0],
      getRadius: 6,
      radiusUnits: 'pixels',
      getFillColor: [251, 191, 36, 220],
      onClick: (info) => {
        if (info.object) {
          const v = info.object;
          setSelectedTrack(`Vehicle: ${v.label ?? v.device_id}\nSpeed: ${v.speed_kmh?.toFixed(0) ?? 'N/A'} km/h`);
        }
      },
    }),

    layers.anomalies && new ScatterplotLayer<Anomaly>({
      id: 'anomalies',
      data: anomalies,
      pickable: true,
      getPosition: (d) => [d.entity.lon, d.entity.lat, 0],
      getRadius: 9,
      radiusUnits: 'pixels',
      getFillColor: (d) => SEVERITY_COLOR[d.severity] ?? [239, 68, 68, 255],
      stroked: true,
      getLineColor: [255, 255, 255, 180],
      getLineWidth: 2,
      lineWidthUnits: 'pixels',
      onClick: (info) => {
        if (info.object) handleAnomalyClick(info.object);
      },
    }),
  ].filter(Boolean) as any[];

  useEffect(() => {
    if (!containerRef.current) return;
    maplibregl.setWorkerUrl('/maplibre/maplibre-gl-worker.mjs');
    const map = new maplibregl.Map({
      container: containerRef.current,
      style: 'https://basemaps.cartocdn.com/gl/dark-matter-gl-style/style.json',
      center: [INITIAL_VIEW.longitude, INITIAL_VIEW.latitude],
      zoom: INITIAL_VIEW.zoom,
      pitch: 0,
      bearing: 0,
    });
    const overlay = new MapboxOverlay({interleaved: false, layers: []});
    map.addControl(overlay);
    mapRef.current = map;
    overlayRef.current = overlay;
    map.on('click', event => {
      const picked = overlay.pickObject({x: event.point.x, y: event.point.y, radius: 3});
      if (!picked?.object) handleMapClick({coordinate: [event.lngLat.lng, event.lngLat.lat]});
    });
    map.on('error', () => setDataError('Some map tiles could not load. Try switching the basemap.'));
    return () => { map.remove(); mapRef.current = null; overlayRef.current = null; };
  }, [handleMapClick]);

  useEffect(() => { overlayRef.current?.setProps({layers: deckLayers}); });
  useEffect(() => {
    mapRef.current?.setStyle(imagery ? {
      version: 8,
      sources: { satellite: { type: 'raster', tiles: ['https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}'], tileSize: 256, attribution: 'Tiles © Esri — Source: Esri, Maxar, Earthstar Geographics, and the GIS User Community' } },
      layers: [{ id: 'satellite', type: 'raster', source: 'satellite' }],
    } : 'https://basemaps.cartocdn.com/gl/dark-matter-gl-style/style.json');
  }, [imagery]);

  const toggleLayer = (key: keyof typeof layers) => {
    setLayers(prev => ({ ...prev, [key]: !prev[key] }));
  };

  return (
    <div className="map-root">
      {/* Top bar */}
      <div className="topbar">
        <span className="topbar-brand">BIQ</span>
        <span className="topbar-stat">Flights<span>{flights.length}</span></span>
        <span className="topbar-stat">Vessels<span>{vessels.length}</span></span>
        <span className="topbar-stat">Vehicles<span>{vehicles.length}</span></span>
        <span className="topbar-stat">Anomalies<span>{anomalies.length}</span></span>
        <div className={`conn-dot${connected ? '' : ' disconnected'}`} title={connected ? 'Live' : 'Connecting…'} />
      </div>

      <div className="map-notice">Illustrative borders · research view{dataError && <span role="alert"> · {dataError}</span>}</div>
      {selectedTrack && <section className="track-card" aria-label="Track details"><button onClick={() => setSelectedTrack(null)} aria-label="Close track details">×</button><pre>{selectedTrack}</pre></section>}
      {/* Layer toggles */}
      <div className="layer-controls">
        {(Object.keys(layers) as (keyof typeof layers)[]).map(key => (
          <button
            key={key}
            className={`layer-btn${layers[key] ? ' active' : ''}`}
            aria-pressed={layers[key]}
            onClick={() => toggleLayer(key)}
          >
            {layers[key] ? '●' : '○'} {key}
          </button>
        ))}
        <button className="layer-btn" onClick={() => setImagery(value => !value)}>{imagery ? 'Street basemap' : 'Satellite basemap'}</button>
      </div>

      {/* Map */}
      <div ref={containerRef} style={{position: 'absolute', inset: 0}} aria-label="Live track map" />

      {/* Weather card */}
      {weatherPin && (
        <WeatherCard
          weather={weather}
          error={weatherError}
          locationLabel={locationLabel}
          lat={weatherPin.lat}
          lon={weatherPin.lon}
          onClose={() => { ++weatherRequest.current; setWeatherPin(null); setWeather(null); }}
        />
      )}

      {/* Anomaly drawer */}
      {selectedAnomaly && drawerData && (
        <AnomalyDrawer
          anomaly={selectedAnomaly}
          data={drawerData}
          onClose={() => { ++anomalyRequest.current; setSelectedAnomaly(null); setDrawerData(null); }}
        />
      )}
    </div>
  );
}

// Upsert helper — replace entity by key or prepend
function upsert<T>(arr: T[], item: T, key: keyof T): T[] {
  const idx = arr.findIndex(a => a[key] === (item as any)[key]);
  if (idx >= 0) {
    const next = [...arr];
    next[idx] = item;
    return next;
  }
  return [item, ...arr].slice(0, 2000);
}
