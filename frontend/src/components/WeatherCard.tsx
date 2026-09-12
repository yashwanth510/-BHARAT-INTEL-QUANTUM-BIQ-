'use client';

import type { WeatherData } from '@/lib/types';

interface Props {
  weather: WeatherData | null;
  error?: string | null;
  locationLabel?: string | null;
  lat: number;
  lon: number;
  onClose: () => void;
}

export default function WeatherCard({ weather, error, locationLabel, lat, lon, onClose }: Props) {
  return (
    <div className="weather-card" role="region" aria-label="Weather at selected location">
      <button className="weather-close" onClick={onClose} aria-label="Close weather card">×</button>
      <h3>Weather</h3>
      <div className="weather-row">
        <span>Location</span>
        <span>{lat.toFixed(3)}, {lon.toFixed(3)}</span>
      </div>
      {locationLabel && <p className="location-label">{locationLabel} <a href="https://www.geoapify.com/" target="_blank" rel="noopener noreferrer">Geoapify</a></p>}
      {error ? <p role="alert">{error}</p> : !weather ? (
        <div className="weather-row"><span>Loading…</span></div>
      ) : (
        <>
          <div className="weather-row">
            <span>Condition</span>
            <span>{weather.description}</span>
          </div>
          <div className="weather-row">
            <span>Temp</span>
            <span>{weather.temp_c.toFixed(1)}°C</span>
          </div>
          <div className="weather-row">
            <span>Feels like</span>
            <span>{weather.feels_like_c.toFixed(1)}°C</span>
          </div>
          <div className="weather-row">
            <span>Humidity</span>
            <span>{weather.humidity_pct}%</span>
          </div>
          <div className="weather-row">
            <span>Wind</span>
            <span>{weather.wind_speed_ms.toFixed(1)} m/s {weather.wind_deg}°</span>
          </div>
          {weather.visibility_m && (
            <div className="weather-row">
              <span>Visibility</span>
              <span>{(weather.visibility_m / 1000).toFixed(1)} km</span>
            </div>
          )}
        </>
      )}
    </div>
  );
}
