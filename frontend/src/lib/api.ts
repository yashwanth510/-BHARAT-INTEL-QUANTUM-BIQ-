export const BASE = (process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000').replace(/\/$/, '');

async function request(url: string, options?: RequestInit) {
  const response = await fetch(url, { ...options, signal: AbortSignal.timeout(120000) });
  if (!response.ok) {
    const body = await response.json().catch(() => ({}));
    throw new Error(body.error || `Request failed (${response.status})`);
  }
  return response;
}

export async function fetchFlights() {
  const r = await request(`${BASE}/api/flights`);
  if (!r.ok) throw new Error('flights fetch failed');
  return r.json();
}

export async function fetchVessels() {
  const r = await request(`${BASE}/api/vessels`);
  if (!r.ok) throw new Error('vessels fetch failed');
  return r.json();
}

export async function fetchVehicles() {
  const r = await request(`${BASE}/api/vehicles`);
  if (!r.ok) throw new Error('vehicles fetch failed');
  return r.json();
}

export async function fetchAnomalies() {
  const r = await request(`${BASE}/api/anomalies`);
  if (!r.ok) throw new Error('anomalies fetch failed');
  return r.json();
}

export async function fetchBorders() {
  const r = await request(`${BASE}/api/borders`);
  if (!r.ok) throw new Error('borders fetch failed');
  return r.json();
}

export async function fetchWeather(lat: number, lon: number) {
  const r = await request(`${BASE}/api/weather?lat=${lat}&lon=${lon}`);
  if (!r.ok) throw new Error('weather fetch failed');
  return r.json();
}

export async function fetchSatellite(lat: number, lon: number) {
  const r = await request(`${BASE}/api/satellite/snapshot`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ lat, lon }),
  });
  if (!r.ok) throw new Error('satellite fetch failed');
  return r.json();
}

export async function fetchOsint(lat: number, lon: number, entity?: string) {
  const r = await request(`${BASE}/api/osint/enrich`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ lat, lon, entity }),
  });
  if (!r.ok) throw new Error('osint fetch failed');
  return r.json();
}

export async function fetchLocation(lat: number, lon: number) {
  return (await request(`${BASE}/api/location?lat=${lat}&lon=${lon}`)).json();
}
