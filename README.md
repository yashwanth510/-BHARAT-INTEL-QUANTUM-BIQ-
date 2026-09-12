# BHARAT INTEL QUANTUM (BIQ)

Map-first research application: Next.js + MapLibre/deck.gl frontend, Rust/Axum backend, Redis, and live public data providers. The previous product has been replaced in this working tree; the original rebuild specification is preserved in [docs/ORIGINAL_PLAN.md](docs/ORIGINAL_PLAN.md).

**Status:** runnable prototype, with live provider integrations and automated API/browser checks. **The original plan is not fully implemented.** The supplied land/maritime coordinates are illustrative, not authoritative legal boundaries. A Geoapify key provides place names, not live GPS vehicle positions. See [the audit](docs/AUDIT.md) for the remaining gaps and verification evidence.

## Run locally without Docker

Prerequisites: Node **22+** (24 recommended; `frontend/.nvmrc`), npm, current stable Rust/Cargo, and a running Redis instance. On Linux, native TLS compilation also needs `pkg-config` and OpenSSL development libraries. Python is used only by the optional integration test; the application backend is Rust.

1. Copy `.env.example` to `.env` **only if `.env` does not already exist**. Configure `REDIS_URL` for your existing Redis. Provider keys stay here, server-side.
2. Copy `frontend/.env.example` to `frontend/.env.local`. Next.js reads its own directory, not the repository-root `.env`.
3. Install and build:

```bash
cd frontend
nvm use  # optional, if using nvm; otherwise use an installed Node 22+
npm ci
cd ..
cargo build --locked --manifest-path backend/Cargo.toml
./scripts/dev.sh start
```

Open **http://localhost:3000**. Backend health is **http://localhost:8000/health**. `scripts/dev.sh status` and `scripts/dev.sh stop` manage the local processes; logs are in `.runtime/`. The script uses existing Redis and does not start Docker or reset your database.

For foreground processes in separate terminals:

```bash
# Terminal 1, repository root
cargo run --locked --manifest-path backend/Cargo.toml
```

```bash
# Terminal 2
cd frontend
npm run dev
```

Production builds without Docker:

```bash
cargo build --release --locked --manifest-path backend/Cargo.toml
cd frontend
npm run build
npm start
```

The MapLibre worker and shared module are copied from the pinned package to `frontend/public/maplibre/` by npm lifecycle scripts. These generated assets are ignored in Git and regenerated on `npm ci`, `npm run dev`, and `npm run build`.

## Current flows

- OpenSky and AISstream ingest live positions → Redis → WebSocket → map. REST snapshots refresh every 30 seconds to remove expired tracks.
- Track click opens identity details. Layer buttons toggle borders, aircraft, vessels, GPS vehicles, and anomalies.
- Empty-map click requests weather at that exact coordinate; Geoapify optionally resolves its place name. Request failures are displayed, and old responses cannot replace newer selections.
- Anomaly click requests Sentinel imagery and Tavily citations. Each provider can fail independently. Acquired time is omitted because the image response does not supply it.
- Street/satellite basemap toggle, responsive controls, connection indicator, and reconnect cleanup are implemented.
- GPS webhook supports a generic position or array; no vehicle locations are invented.

Example GPS payload for a compatible device feed:

```json
{"device_id":"device-123","lat":22.0,"lon":80.0,"speed":10,"heading":90,"label":"Research vehicle"}
```

Send it to `POST /ingest/gps` with `Authorization: Bearer <GPS_WEBHOOK_TOKEN>`. `speed` is km/h. GPS polling expects `GPS_API_BASE_URL/positions`, Bearer authentication, and the same payload format (also accepts a `positions` or `devices` array). Provider-specific timestamps/units need an adapter before using another schema.

## Configuration

`.env.example` is the full public template. Never commit `.env`, `.env.local`, Redis passwords, or provider keys.

| Variable | Purpose |
| --- | --- |
| `REDIS_URL`, `PORT` | Redis connection and HTTP port; Render supplies its own `PORT` |
| `CORS_ORIGINS` | Comma-separated exact browser origins, without trailing slashes |
| `ADMIN_API_TOKEN` | Bearer token for both allowlist read/write; disabled if unset |
| `GPS_WEBHOOK_TOKEN` | Separate Bearer token for GPS writes; disabled if unset |
| `OPENSKY_CLIENT_ID/SECRET` | Optional OAuth2 client; anonymous requests are quota-limited |
| `AISSTREAM_API_KEY`, `AISSTREAM_BBOXES` | AIS subscription; boxes are `[[[lat_min,lon_min],[lat_max,lon_max]]]` |
| `GPS_API_KEY`, `GPS_API_BASE_URL` | Actual live-device provider; Geoapify URLs disable polling |
| `GEOAPIFY_API_KEY` | Reverse geocoding; legacy `GPS_API_KEY` is reused only when its URL explicitly names Geoapify |
| `OPENWEATHER_API_KEY` | Click-point weather, cached for 10 minutes |
| `SENTINEL_CLIENT_ID/SECRET` | Satellite credentials |
| `SENTINEL_TOKEN_URL`, `SENTINEL_PROCESS_URL` | Select matching commercial Sentinel Hub or Copernicus Data Space endpoints |
| `TAVILY_API_KEY`, `TAVILY_MAX_PER_DAY` | Enrichment and atomic global daily request cap |
| `BORDER_CORRIDOR_KM`, `MARITIME_CORRIDOR_NM` | Distance thresholds around the supplied illustrative lines |
| `QUIET_POLL_SECONDS`, `GPS_POLL_SECONDS` | Ingest cadence; separate hotspot scheduling remains unimplemented |
| `BORDER_DATA_DIR` | Optional explicit geometry directory; bundled assets otherwise travel with the Rust binary |
| `INGEST_ENABLED=false` | Disable all external ingest for isolated tests |
| `NEXT_PUBLIC_API_URL`, `NEXT_PUBLIC_WS_URL` | Public frontend endpoints; set before building on Vercel |

Your existing Sentinel credentials were verified against **Copernicus Data Space**. Its URLs are in `.env.example`. Commercial Sentinel Hub credentials require `https://services.sentinel-hub.com/oauth/token` and `https://services.sentinel-hub.com/api/v1/process` instead.

`/health` checks Redis readiness and reports configured providers separately from observed runtime results. A configured key does not by itself mean the provider has succeeded.

## Test

```bash
cargo test --locked --manifest-path backend/Cargo.toml
cargo fmt --check --manifest-path backend/Cargo.toml
# Optional test dependency only:
python3 -m pip install websockets
python3 scripts/test_backend.py
cd frontend
npm run typecheck
npm run build
npm audit --omit=dev
# Start the app, then run browser tests (Google Chrome installed locally):
npm run test:e2e
```

The Python integration test starts a temporary native Redis on a random port and a separate backend with providers disabled. It does not modify the running app's Redis. Browser tests cover live connections/weather plus explicit fixture data for deterministic anomaly enrichment tests. `E2E_BASE_URL` can select another local server; `E2E_BROWSER_CHANNEL` defaults to `chrome`.

## Deploy: Render backend + existing Vercel frontend

Repository: https://github.com/yashwanth510/-BHARAT-INTEL-QUANTUM-BIQ-

Existing frontend URL: **https://bharat-intel-quantum-biq.vercel.app**. Keep the existing Vercel project and domain. Railway is no longer used. Pushing code does not create Render resources or change Vercel dashboard environment variables automatically: complete the one-time setup below.

### 1. Create the free Render services

1. In Render, select **New → Blueprint**, connect this GitHub repository, and select branch `main`. Render reads the root `render.yaml`.
2. Confirm both resources are on the **Free** plan: `biq-backend` (native Rust) and `biq-cache` (Redis-compatible Key Value), in Frankfurt.
3. Fill the prompted provider credentials from your local `.env`. For `GEOAPIFY_API_KEY`, use the existing Geoapify key currently stored under `GPS_API_KEY`. Do not configure Geoapify as a GPS position feed. Missing provider keys leave their features unavailable; they never generate fake data.
4. Apply the Blueprint. Redis connection settings and separate admin/GPS webhook tokens are generated automatically. The Redis service accepts private-network connections only.
5. After the backend deploys, **copy its actual `https://…onrender.com` URL from the Render dashboard**. Service names can receive suffixes, so do not assume a hostname from `biq-backend`.
6. Open `<actual-backend-url>/health`; expect HTTP 200 and `redis: true`.

The Blueprint uses root directory `backend`, build `bash build.sh`, start `./bin/biq-backend`, and health path `/health`. Rust is pinned in `backend/rust-toolchain.toml`. Render provides `PORT`; do not hardcode Railway's port. All builds are native, without a Dockerfile.

If creating services manually instead of using the Blueprint, create Free Key Value first, then a Free Rust Web Service with those commands. Set `REDIS_URL` to Key Value's **internal connection URL**, and copy the remaining variables from `render.yaml`/`.env.example`.

### 2. Point the existing Vercel project at Render

In the **existing** Vercel project's Settings:

| Setting | Value |
| --- | --- |
| Git repository / production branch | This repository / `main` |
| Root Directory | `frontend` |
| Framework | Next.js |
| Node.js | 24.x |
| Install Command | `npm ci` |
| Build Command | `npm run build` |
| Output Directory | Default Next.js output (`.next`) |
| `NEXT_PUBLIC_API_URL` | Actual Render HTTPS origin, e.g. `https://your-assigned-host.onrender.com`, without `/api` |
| `NEXT_PUBLIC_WS_URL` | **Delete the old Railway value.** Leave unset to derive `wss://<API host>/ws/live`, or set that exact URL |

Set these variables for **Production** (and Preview if used), then **Redeploy the latest commit**. Next.js embeds public variables at build time: changing them requires a redeploy. A Vercel build now fails clearly if its API URL is missing/local, or its explicit WebSocket URL points at a different host.

The backend always allows `https://bharat-intel-quantum-biq.vercel.app` and `https://bharat-intel-quantum-biq-frontend.vercel.app`, including when Render has an older `CORS_ORIGINS` value. Additional production/custom domains can be added through `CORS_ORIGINS`, comma-separated. Removing either built-in domain requires a code change. Preview domains also need explicit CORS entries. Provider keys and write tokens belong only in Render, never in `NEXT_PUBLIC_*` variables.

### 3. Verify the live connection

- Backend `/health`: HTTP 200, Redis ready; inspect provider runtime results separately from configured flags.
- Backend `/api/borders`, `/api/flights`, `/api/vessels`: successful JSON responses.
- Open the existing Vercel site: map tiles load and the connection indicator turns green.
- Browser Network: API requests target Render; `/ws/live` uses `wss://` and upgrades successfully. No requests should target Railway or localhost.
- Click the map for weather/place names and an anomaly for imagery/citations. Empty live layers can reflect provider coverage or quota, not necessarily a deployment fault.

Once this setup is complete, pushes to `main` trigger Render and Vercel deployments when their Git integrations/autodeploy are enabled. Their deployments run independently and can finish at different times. Disconnect/disable the old Railway service integration in Railway to avoid obsolete deployment attempts; no Railway resources are deleted by this code.

### Free-tier behavior

Render's free web service sleeps after 15 minutes without inbound traffic and can take about a minute to wake. Ingest stops while asleep, and the first request can be slow. The frontend permits up to two minutes for API responses and retries snapshots/WebSocket connections; it does not send artificial keep-alive traffic.

Free Key Value is in-memory: restarts lose tracks, cached weather, anomalies, allowlists and quota counters. Reapply allowlists after a restart. Provider-side quotas still apply. Free compute/build/bandwidth limits and possible high-outbound-traffic suspension make this a **demo/prototype**, not guaranteed 24/7 monitoring. Use one backend instance; WebSocket broadcasting is process-local.

### Local Git workflow

The rewritten folder tracks the original repository history. Deleted `quantum-api`, Docker and old frontend files are intentional replacements. `.env`, frontend local env files, build outputs and `.runtime` are ignored.

```bash
git status --short
git add -A
git diff --cached --check
git commit -m "Configure BIQ for Render backend and Vercel frontend"
git push origin main
```

References: [Render Blueprint specification](https://render.com/docs/blueprint-spec), [free-plan limits](https://render.com/docs/free), [Render WebSockets](https://render.com/docs/websocket), [Vercel monorepos](https://vercel.com/docs/monorepos), [Copernicus authentication](https://documentation.dataspace.copernicus.eu/APIs/SentinelHub/Overview/Authentication.html).

### Live deployment: empty flight or vessel lists

The deployed backend origin is `https://biq-backend-qwb4.onrender.com`.
Set Vercel's `NEXT_PUBLIC_API_URL` to that origin and redeploy after changing it.
If explicitly setting `NEXT_PUBLIC_WS_URL`, use
`wss://biq-backend-qwb4.onrender.com/ws/live`.

A successful `/health` response confirms service availability; inspect
`providers.*.runtime` separately for upstream ingestion failures. Empty track
arrays can mean ingestion failed even when Redis, CORS and the frontend work.

In Render → **biq-backend → Logs**, inspect `AISstream disconnected` and
`OpenSky request failed`. AISstream logs distinguish handshake failures from
failures after subscription. Subscription error payloads are deliberately not
logged because they may contain credentials. Reconnects back off up to five
minutes plus jitter. OpenSky transport failures now identify timeouts versus
connection failures, with underlying diagnostics in server logs.

AISstream permits three subscribed connections per account and three open
connections per originating IP. Stop unused local or old hosted consumers when
investigating limits. A reset alone does not establish that a key is invalid or
that Render is blocked. See [AISstream's limits and troubleshooting](https://aisstream.io/documentation).

### Provider diagnosis and local dotenv syntax

Quote values containing spaces in local `.env` files, especially an OpenSky
client ID. The backend now stops with a safe syntax error if dotenv parsing
fails, rather than silently starting with only some provider settings loaded.
Render dashboard values should be entered directly, without dotenv quotes.

Sentinel, Tavily, Geoapify and OpenSky OAuth failures now report sanitized HTTP
status codes or timeout/connection categories in the logs and provider health.
Provider response bodies, tokens and URLs containing keys are not included.
A status of 401/403 indicates an upstream rejection; a timeout alone does not
prove that credentials are invalid. Geoapify is also included in `/health`.

On 2026-09-12, local checks after correcting dotenv syntax returned aircraft,
weather, reverse-geocoded location, a Sentinel image and five Tavily results.
The concurrent Render checks returned weather successfully, reported an
AISstream position success, but failed OpenSky OAuth, Geoapify, Sentinel and
Tavily. These observations do not establish a single shared root cause for the
remote failures; deploy the diagnostics and inspect each provider separately.
