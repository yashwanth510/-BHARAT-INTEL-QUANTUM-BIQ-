# BHARAT INTEL QUANTUM (BIQ) — Map-First Border Surveillance

Fresh greenfield rebuild. **Map-only frontend.** **Rust-only backend.** Real border geometry for **all Indian land borders and maritime approaches**. Live flights, vessels, and GPS-tracked ground units. Anomalies pop at exact coordinates; click opens satellite imagery + Tavily OSINT cross-check + click-point weather.

> Status: **design / implementation plan.** Code will be scaffolded after GPS provider credentials are supplied.

---

## 1. Product goals

| Goal | Behavior |
|------|----------|
| Map-first UI | Single full-screen map. No multi-panel “site” chrome (news dashboard, system tabs, etc.). |
| Real borders | Draw **exact land + maritime border polylines** from authoritative GeoJSON (not point markers). |
| Live tracks | Show **flights**, **vessels**, and **GPS vehicles** on the map continuously. |
| Anomalies | Popup at **exact lat/lon** for unregistered / dark / border-corridor intrusions. |
| Click anomaly | Fetch **satellite imagery** for that location + **Tavily** enrichment. |
| Click map | Fetch **live weather** for that lat/lon (never hardcoded zones). |
| Border watch | Monitor **entire** Indian land frontier + maritime EEZ/coastal approaches via corridor + hotspots (see §5). |

---

## 2. Chosen decisions

### Backend language
**Rust only** (Actix Web or Axum — default **Axum** for clearer async + WebSocket story).

### Border focus
**All Indian land borders + maritime** end to end:
- Land: India–Pakistan, India–China (LAC), India–Nepal, India–Bhutan, India–Bangladesh, India–Myanmar, and related disputed/sensitive sectors as separate GeoJSON layers.
- Maritime: coastal outline + EEZ / territorial-sea buffer corridors (Arabian Sea, Bay of Bengal, Andaman approaches).

### Unregistered / anomaly rule (recommended — chosen)
**Hybrid identity + allowlist + corridor** (best signal/noise):

1. **Missing identity** → anomaly candidate
   - Flight: no ICAO24 / no callsign when squawk expected
   - Vessel: no MMSI or AIS “dark” (position known from other means, or AIS gap)
   - Vehicle: GPS device id missing or not authenticated
2. **Not on allowlist** while inside a **border corridor** → anomaly
   - Allowlists: registered aircraft ICAO/callsign, vessel MMSI, vehicle device_id (configurable via env/Redis)
3. **Corridor crossing** → anomaly even if identified, if track crosses from outside→inside sensitive buffer without an authorized crossing point
4. **Severity**
   - Critical: unidentified + inside hotspot corridor
   - High: identified but unauthorized crossing
   - Elevated: identity incomplete near corridor
   - Info: known track inside corridor on authorized path

Tavily is used **only for enrichment** after a position-based anomaly exists. It never invents coordinates.

### GPS
You will provide a **free GPS tracking API key**. Backend will poll or webhook-ingest positions into Redis and broadcast over WebSocket. Until the key + docs arrive, GPS ingest is stubbed behind `GPS_API_KEY` / `GPS_API_BASE_URL`.

---

## 3. High-level architecture

```text
┌──────────────────────────── Frontend (Next.js) ────────────────────────────┐
│  Full-screen MapLibre GL + deck.gl                                          │
│  Layers: borders · flights · vessels · vehicles · anomalies · weather pin   │
│  Click map → weather · Click anomaly → satellite + Tavily panel             │
└───────────────────────────────┬────────────────────────────────────────────┘
                                │ REST + WebSocket
┌──────────────────────────── Backend (Rust) ────────────────────────────────┐
│  Axum HTTP API · WS hub · ingest workers · geofence engine · Redis cache    │
└───┬──────────┬──────────┬──────────┬──────────┬──────────┬─────────────────┘
    │          │          │          │          │          │
 OpenSky    AISstream   GPS API   OpenWeather  Sentinel   Tavily
 (flights)  (vessels)  (vehicles) (click wx)   (imagery)  (OSINT)
```

### Data plane
- **Redis**: latest entity positions, track tails, allowlists, rate counters, anomaly dedupe keys.
- Optional later: Neo4j / Kafka (out of v1 scope).

---

## 4. External APIs (validated for use)

| Domain | Provider | Endpoint / protocol | Auth | Notes |
|--------|----------|---------------------|------|-------|
| Flights | **OpenSky Network** | `GET https://opensky-network.org/api/states/all?lamin=&lamax=&lomin=&lomax=` | Optional OAuth2 client (higher limits); anonymous OK with limits | Live ADS-B state vectors; bbox India+buffer |
| Vessels | **AISstream** | `wss://stream.aisstream.io/v0/stream` | API key in subscribe JSON | Server-side only; bbox India seas |
| Vehicles | **Your GPS API** (TBD) | Poll REST or webhook | `GPS_API_KEY` | You supply key + sample payload |
| Weather | **OpenWeather** | `GET …/data/2.5/weather?lat=&lon=` | `OPENWEATHER_API_KEY` | On map click; cache ~5–10 min |
| Satellite | **Copernicus Sentinel Hub** | OAuth + Process API | `SENTINEL_CLIENT_ID` / `SECRET` | On anomaly click; true-color snapshot |
| OSINT | **Tavily** | `POST https://api.tavily.com/search` | `TAVILY_API_KEY` | Enrich anomaly only |
| Borders | **Natural Earth / OSM-derived GeoJSON** | Static files in repo `data/borders/` | None | Land admin + maritime EEZ layers |

**Not used in v1:** NewsAPI dashboards, MistTrack, Mistral LLM fusion panels, hardcoded weather zones.

**Aviationstack:** keep env slot optional; OpenSky is primary live layer (better for continuous map).

---

## 5. Border monitoring strategy (large area)

Watching thousands of km as a dense pixel fence is noisy and expensive. Design:

### 5.1 Geometry layers
1. **Border polylines** — exact lines drawn on map (user-facing “real map”).
2. **Corridor polygons** — buffer around every land border segment (default **15 km** inland/outward, configurable).
3. **Maritime corridors** — territorial sea + approach belts (default **24 nm** conceptual buffer as polygon; tune via env).
4. **Hotspots** — denser polling / lower anomaly threshold for LoC, LAC (Ladakh/Arunachal), Sir Creek, Andaman approaches, Gujarat sector, etc.

### 5.2 Detection algorithm
For each live track point `(lat, lon, t)`:
1. Point-in-polygon against corridor set (R-tree spatial index).
2. If entering corridor from outside → emit `BorderApproach` / `BorderCrossing` event.
3. Apply unregistered rule (§2).
4. Dedupe in Redis: `anomaly:{entity_id}:{zone_id}` TTL 15–30 min.
5. Broadcast WS `anomaly` with exact coordinates.

### 5.3 Keeping an eye without drowning ops
- Map always shows **full border lines**.
- Alert popups only for **corridor + rule hits**.
- Hotspots refresh every **5–15 s**; quiet sectors **30–60 s**.
- Cluster anomaly icons at low zoom; expand at high zoom.

---

## 6. Frontend design (map only)

### Layout
- Full viewport map.
- Thin top bar: brand **BIQ**, live counts (flights / vessels / vehicles / anomalies), connection status.
- Layer toggles (borders, air, sea, land GPS, anomalies) — compact, not a multi-page app.
- No sidebar modules for news / system / multi-dashboards.

### Layers (deck.gl / MapLibre)
| Layer | Source |
|-------|--------|
| Basemap | MapLibre (OSM / Esri imagery toggle) |
| Borders | GeoJSON LineString / MultiLineString |
| Corridor (optional ghost) | GeoJSON Polygon, low opacity |
| Flights | Scatter / icon + heading |
| Vessels | Scatter / icon + COG |
| Vehicles | Scatter from GPS ingest |
| Anomalies | Pulsing marker at exact lat/lon |

### Interactions
1. **Click empty map** → weather card at pin (OpenWeather via backend).
2. **Click flight/vessel/vehicle** → identity sheet (MMSI/ICAO/device_id, speed, course).
3. **Click anomaly popup** → detail drawer: why flagged, Sentinel image, Tavily sources (titles + URLs).

### Removed from old product
Multi-panel OpsSidebar routes (`overview`, `news`, `system`, …), hardcoded weather zones, synthetic air/sea path decorations, orphan intelligence panels.

---

## 7. Backend design (Rust)

### Suggested crate layout
```text
backend/
  Cargo.toml
  src/
    main.rs              # Axum router, config, spawn workers
    config.rs            # env loading
    error.rs
    models/              # Flight, Vessel, Vehicle, Anomaly, Weather, Border
    geo/
      borders.rs         # load GeoJSON
      geofence.rs        # R-tree / point-in-polygon / crossing
    ingest/
      opensky.rs
      aisstream.rs
      gps.rs             # your GPS provider
    providers/
      weather.rs
      sentinel.rs
      tavily.rs
    state.rs             # AppState: Redis, WS broadcast, spatial index
    ws.rs                # /ws/live
    routes/
      health.rs
      tracks.rs          # GET /api/flights|/vessels|/vehicles
      weather.rs         # GET /api/weather?lat=&lon=
      anomalies.rs
      satellite.rs       # POST /api/satellite/snapshot
      osint.rs           # POST /api/osint/enrich
      borders.rs         # GET /api/borders/geojson
  data/borders/          # GeoJSON assets
```

### Core REST API

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/health` | Liveness + provider flags (key present / last success) |
| GET | `/api/borders` | Border + corridor GeoJSON for map |
| GET | `/api/flights` | Latest OpenSky snapshot (cached) |
| GET | `/api/vessels` | Latest AIS positions (cached) |
| GET | `/api/vehicles` | Latest GPS positions (cached) |
| GET | `/api/anomalies` | Active anomalies |
| GET | `/api/weather?lat=&lon=` | Click-point weather |
| POST | `/api/satellite/snapshot` | `{lat,lon,bbox?}` → imagery URL/bytes meta |
| POST | `/api/osint/enrich` | `{lat,lon,entity,query?}` → Tavily results |
| GET | `/api/allowlist` | (ops) list registered ids |
| PUT | `/api/allowlist` | (ops) upsert allowlist entries |
| WS | `/ws/live` | Stream `flight_update`, `vessel_update`, `vehicle_update`, `anomaly`, `heartbeat` |

### WebSocket message shapes (illustrative)
```json
{
  "type": "anomaly",
  "id": "anom_…",
  "severity": "high",
  "reason": ["missing_identity", "corridor_entry"],
  "entity": { "kind": "vessel", "id": "MMSI…", "lat": 22.1, "lon": 68.9 },
  "zone_id": "gujarat_maritime_corridor",
  "ts": "2026-09-11T07:00:00Z"
}
```

### Ingest workers
| Worker | Cadence | Action |
|--------|---------|--------|
| OpenSky | 10–15 s (hot bbox) / 30–60 s (wide) | Fetch states → Redis → WS |
| AISstream | Persistent WS | Parse PositionReport → Redis → WS |
| GPS | Poll N s **or** webhook `POST /ingest/gps` | Normalize → Redis → WS |
| Geofence | On each position upsert | Evaluate corridor + rules → anomaly |

---

## 8. End-to-end flows

### 8.1 Live tracks on map
```text
Provider → Rust ingest → Redis SET entity:{kind}:{id} → broadcast WS
Frontend hydrates GET /api/* once → keeps WS updates → deck.gl layers
```

### 8.2 Border anomaly
```text
Position upsert → geofence hit → unregistered/crossing rule →
dedupe Redis → persist anomaly → WS anomaly →
frontend popup at exact lat/lon
```

### 8.3 Anomaly click → satellite + Tavily
```text
User clicks popup →
  POST /api/satellite/snapshot {lat,lon}
  POST /api/osint/enrich {lat,lon, entity, query}
Backend calls Sentinel + Tavily → returns image meta + citations
Frontend drawer shows both
```

### 8.4 Map click → weather
```text
User clicks map → GET /api/weather?lat=&lon=
OpenWeather → cache → weather card at pin
```

---

## 9. Environment variables

Existing `.env` will be **trimmed and updated** during scaffold. Planned keys:

```bash
# Server
PORT=8000
RUST_LOG=info
REDIS_URL=redis://:…@localhost:6379

# Flights
OPENSKY_CLIENT_ID=          # optional
OPENSKY_CLIENT_SECRET=      # optional
OPENSKY_BBOX=5.0,35.0,65.0,100.0   # lamin,lamax,lomin,lomax (tune)

# Vessels
AISSTREAM_API_KEY=
AISSTREAM_BBOXES=…          # JSON or compact form

# Vehicles (YOU PROVIDE)
GPS_API_KEY=
GPS_API_BASE_URL=
GPS_POLL_SECONDS=15

# Weather
OPENWEATHER_API_KEY=

# Satellite
SENTINEL_CLIENT_ID=
SENTINEL_CLIENT_SECRET=

# OSINT
TAVILY_API_KEY=
TAVILY_MAX_PER_DAY=33

# Geofence
BORDER_CORRIDOR_KM=15
MARITIME_CORRIDOR_NM=24
HOTSPOT_POLL_SECONDS=10
QUIET_POLL_SECONDS=45

# Frontend
NEXT_PUBLIC_API_URL=http://localhost:8000
NEXT_PUBLIC_WS_URL=ws://localhost:8000/ws/live
```

Remove from active use (optional keep commented): NewsAPI, Mistral, MistTrack, hardcoded `WEATHER_ZONES` / `LOCATION_CATALOG` as primary weather source.

---

## 10. Implementation plan (phased)

### Phase 0 — Docs & env contract (this README)
- [x] Product scope, APIs, flows, anomaly rule
- [ ] Receive GPS API key + base URL + sample JSON
- [ ] Rewrite `.env` to new contract (preserve secrets you already have)

### Phase 1 — Rust backend skeleton
- [ ] Cargo workspace / `backend` crate (Axum, tokio, redis, serde, geo, rstar)
- [ ] Config + `/health`
- [ ] Redis client + WS hub
- [ ] Static GeoJSON loader for India land + maritime borders
- [ ] `GET /api/borders`

### Phase 2 — Live ingest
- [ ] OpenSky worker → `/api/flights` + WS
- [ ] AISstream worker → `/api/vessels` + WS
- [ ] GPS worker/webhook → `/api/vehicles` + WS
- [ ] Fail closed when keys missing (empty layers, clear health flags — no fake tracks in prod mode)

### Phase 3 — Geofence + anomalies
- [ ] Corridor generation from border lines
- [ ] Hotspot overlays
- [ ] Unregistered hybrid rule + Redis dedupe
- [ ] `/api/anomalies` + WS `anomaly`

### Phase 4 — Enrichment APIs
- [ ] Click weather
- [ ] Sentinel snapshot
- [ ] Tavily enrich

### Phase 5 — Map-only frontend
- [ ] Next.js app, MapLibre + deck.gl
- [ ] Border / track / anomaly layers
- [ ] Weather pin + anomaly drawer (satellite + OSINT)
- [ ] Layer toggles + live counts only

### Phase 6 — Hardening
- [ ] Rate limits / quotas
- [ ] docker-compose (Redis + backend + frontend)
- [ ] Basic auth or API token for allowlist ops (minimal)
- [ ] README quickstart verified

---

## 11. Acceptance criteria

1. Opening the app shows **only the map** with **full India land + maritime border lines**.
2. Flights and vessels move/update in near-real time when keys are set.
3. GPS vehicles appear when `GPS_API_*` is configured.
4. Unregistered / corridor events popup at **exact** coordinates.
5. Clicking an anomaly shows satellite imagery attempt + Tavily citations.
6. Clicking the map shows weather for **that** lat/lon.
7. No hardcoded weather list drives the UI.
8. Backend is **Rust**; no Python/Node API server.

---

## 12. Security & ops notes

- Never expose provider keys to the browser; frontend talks only to BIQ backend.
- AISstream and GPS must be server-side.
- Treat this as a **simulation / research cockpit** unless you add formal auth, auditing, and classified-data handling.
- Do not commit `.env`. Provide `.env.example` with empty values only.

---

## 13. Blocked on you — GPS provider

Please reply with:

1. **Provider name** (e.g. Traccar, GPSAPI, custom)
2. **`GPS_API_BASE_URL`**
3. **`GPS_API_KEY`** (or say “add placeholder, I’ll paste into `.env` myself”)
4. **Sample response JSON** for one device position (or docs link)
5. Poll vs webhook preference (if known)

Once that arrives, implementation starts at **Phase 0 env rewrite → Phase 1 Rust skeleton**.

---

## 14. Quick mental model

```text
Borders drawn exactly → tracks stream live → corridor watches the line →
unregistered/crossing pops at the point → click for satellite + Tavily →
click empty map for real weather.
```

**Stack:** Next.js (map UI) + **Rust/Axum** (API + ingest + geofence) + Redis + OpenSky + AISstream + your GPS + OpenWeather + Sentinel + Tavily.
