# BIQ implementation audit — 12 September 2026

The rewritten application now builds and runs natively. It is **not a completed implementation of every item in the original specification**, and its Render/Vercel cloud rollout requires the one-time dashboard setup in the README.

## Verified

| Check | Result |
| --- | --- |
| Clean frontend dependency installation | `npm ci` passed using Node 24 |
| Frontend production build | Next.js 15.5.25 build, TypeScript checks, and prerendering passed |
| Backend production build | Rust release build passed; `backend/build.sh` produces `bin/biq-backend` |
| Rust regression tests | 4 passed: invalid coordinates, identifiers, line-distance false positives, degenerate segments |
| Rust formatting | `cargo fmt --check` passed |
| Isolated integration test | Passed against both debug and release binaries, with temporary Redis and external providers disabled |
| Browser tests | 5 passed against both development and production servers, including the vector tile load check |
| Production dependency audit | `npm audit --omit=dev`: zero reported vulnerabilities |
| Secret handling | Provider secrets remain in ignored local env files; source checked for configured secret values |
| Git | Local `main` tracks original `origin/main`; See Git history for publication status |

The integration test exercises health/readiness, borders, track snapshots, weather/location/satellite/OSINT missing-key behavior, allowlist authentication and validation, GPS authentication and invalid coordinates, CORS origin matching, WebSocket heartbeat, GPS → Redis → WebSocket messages, anomaly coordinates, duplicate suppression, allowlisted-device suppression, and Redis outage responses. It does not touch the user's existing Redis.

Browser tests exercise map rendering and **actual vector tile loading**, layer toggles, click-coordinate weather requests, weather failure UI, mobile controls, exact-coordinate anomaly requests with fixture satellite/citation responses, closing the drawer, and WebSocket disconnection status. The deterministic anomaly fixture is test-only; production contains no synthetic track generator.

## Live provider observations

These are point-in-time requests with the existing `.env`, not a claim of continuous provider availability or complete geographical coverage.

| Provider | Observation |
| --- | --- |
| OpenSky | Successful snapshots, roughly 300–310 aircraft during the audit |
| AISstream | Actual vessel position received and stored; coverage/update frequency can be sparse; no fabricated fallback |
| OpenWeather | HTTP 200 with current weather for the clicked/requested coordinates |
| Geoapify | HTTP 200 with reverse-geocoded location; reuses the existing key previously entered as GPS credentials |
| Tavily | HTTP 200 with source titles, URLs, and snippets |
| Copernicus Sentinel | Commercial endpoint rejected this client; Data Space OAuth accepted it. Corrected endpoints returned a valid 97,671-byte JPEG at the tested coordinates |
| Live GPS devices | **Not available.** The configured service is Geoapify, not a device tracking feed. Authenticated generic webhook verified only with isolated test positions |

## Material fixes

- Replaced invalid map imports/React construction; fixed MapLibre v6 namespace and self-hosted ES-module worker dependencies. Blank basemaps are now covered by a tile-load check.
- Upgraded and pinned Next.js, deck.gl, MapLibre and PostCSS; removed unused React map adapters and generated dependency lockfile.
- Fixed frontend environment loading, backend URL-derived WebSocket URL, disconnection status, reconnect cleanup, stale selection responses, periodic snapshot reconciliation, and visible provider failures.
- Replaced blocking alert dialogs with map-overlay identity cards; added weather marker, optional Geoapify label, mobile styling and basemap toggle.
- Corrected OpenSky OAuth2 and added request timeouts/429 backoff; normalized AIS corner-pair bounding boxes and handled binary frames.
- Separated Geoapify reverse geocoding from GPS polling; authenticated GPS webhook and allowlist operations with separate server tokens; restricted CORS and validated coordinates/identifiers.
- Replaced whole-border bounding-box geofences with distance-to-segment checks; applied the maritime threshold in nautical miles. Dedupe uses atomic Redis `SET NX EX` with entity-kind-specific keys.
- Added atomic Tavily daily quota. Weather/location are cached. Internal errors no longer expose provider request URLs or secret-bearing responses.
- Corrected Copernicus endpoints; removed the false claim that the request time was the image acquisition time.
- Made Redis failure visible in readiness and REST errors; bundled geometry travels with the executable.
- Added native launch/stop/status script, separate frontend env template, native CI, Render Blueprint and Vercel configs. Removed Docker build files from the active rewrite.

## Original specification: remaining gaps

| Requirement | Current state / limitation |
| --- | --- |
| Exact all-India land borders, LAC/LoC/disputed sectors and maritime EEZ | **Unfulfilled.** Bundled files are sparse, manually drawn prototype lines. They are labeled illustrative in data, UI and health. An approved, sourced and licensed dataset is needed; no legal accuracy or complete frontier coverage is claimed |
| GPS provider integration | Generic `/positions` adapter and authenticated webhook exist; a genuine device feed and its units/timestamps/schema are still required |
| Hybrid crossing rule | Current detection considers missing identity/allowlist membership inside a corridor. It does **not** retain previous-zone state to classify outside→inside crossings or model authorized crossing points |
| AIS-dark detection | Not implemented. Missing AIS alone cannot supply an unseen vessel's location; no coordinates are invented |
| Scalable geofence index | Uses linear checks against short supplied line segments; no R-tree, geodesic polygon buffer generation, or high-resolution geometry performance validation |
| Track histories and trails | Only latest positions with TTLs; no persisted tails/history |
| Hotspot-specific polling | One OpenSky region/cadence; no separate hot/quiet-sector scheduler. `HOTSPOT_POLL_SECONDS` is not an active setting |
| Advanced map styling | Basic point markers and anomaly rings; no heading icons, pulses, low-zoom anomaly clustering or corridor polygon overlay |
| Operations hardening | Tokens protect writes, but there is no user login/RBAC/audit trail. Public read/enrichment APIs are not a full authenticated operations system. Only Tavily has a daily quota; no comprehensive request limiter or Sentinel quota |
| Multiple backend replicas | Redis data is shared, but WebSocket broadcast is process-local; use one backend instance until pub/sub/fan-out is added |
| Provider adapters | GPS uses arrival time and an assumed km/h `speed` field; provider-specific timestamp/staleness semantics need verification before operational use |
| Performance/load test | Functional checks passed; no sustained load/soak or production GPU/frame-rate claim |

## Deployment boundary

The deployment target is now **Render Free Web Service + Free Key Value**, with the existing Vercel frontend retained. The root `render.yaml` provisions the backend/cache on free plans when imported as a Blueprint. Railway configuration has been removed.

The README explains how to copy Render's actual assigned HTTPS URL into Vercel, remove the old Railway WebSocket URL, configure CORS, and redeploy. Cloud account settings and deployment success cannot be established by a local build. A Git push alone does not create a Render account/service or configure Vercel variables.

The local folder initially lacked `.git`; it now follows the original repository history without replacing the rewritten source. Deleted old paths are intentional replacements.
