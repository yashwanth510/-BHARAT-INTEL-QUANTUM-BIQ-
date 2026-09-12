'use client';

import type { Anomaly, TavilyResult, SentinelSnapshot } from '@/lib/types';

interface Props {
  anomaly: Anomaly;
  data: {
    osint: TavilyResult[];
    sentinel: SentinelSnapshot | null;
    loading: boolean;
    osintError?: string;
    sentinelError?: string;
  };
  onClose: () => void;
}

const REASON_LABEL: Record<string, string> = {
  missing_identity: 'Missing identity',
  not_on_allowlist: 'Not on allowlist',
  corridor_crossing: 'Corridor crossing',
  ais_dark: 'AIS dark',
};

export default function AnomalyDrawer({ anomaly, data, onClose }: Props) {
  return (
    <aside className="anomaly-drawer" role="complementary" aria-label="Anomaly details">
      <button className="drawer-close" onClick={onClose} aria-label="Close anomaly drawer">×</button>
      <h2>Anomaly</h2>

      <div className="anomaly-section">
        <h3>Details</h3>
        <div className="weather-row">
          <span>Severity</span>
          <span className={`severity-${anomaly.severity}`}>{anomaly.severity.toUpperCase()}</span>
        </div>
        <div className="weather-row">
          <span>Entity</span>
          <span>{anomaly.entity.kind} / {anomaly.entity.id}</span>
        </div>
        <div className="weather-row">
          <span>Zone</span>
          <span>{anomaly.zone_id}</span>
        </div>
        <div className="weather-row">
          <span>Position</span>
          <span>{anomaly.entity.lat.toFixed(4)}, {anomaly.entity.lon.toFixed(4)}</span>
        </div>
        <div className="weather-row">
          <span>Time</span>
          <span>{new Date(anomaly.ts).toLocaleTimeString()}</span>
        </div>
        <div style={{ marginTop: 6, fontSize: 11, color: '#94a3b8' }}>
          Reasons: {anomaly.reasons.map(r => REASON_LABEL[r] ?? r).join(', ')}
        </div>
      </div>

      {data.loading && (
        <div className="weather-row"><span>Loading enrichment…</span></div>
      )}

      {!data.loading && data.sentinelError && <p role="alert">Satellite: {data.sentinelError}</p>}
      {!data.loading && data.osintError && <p role="alert">OSINT: {data.osintError}</p>}
      {!data.loading && data.sentinel && (
        <div className="anomaly-section">
          <h3>Satellite Imagery</h3>
          {data.sentinel.image_b64 ? (
            <img
              className="sentinel-img"
              src={`data:image/jpeg;base64,${data.sentinel.image_b64}`}
              alt="Sentinel satellite image"
            />
          ) : data.sentinel.image_url ? (
            <img className="sentinel-img" src={data.sentinel.image_url} alt="Sentinel satellite image" />
          ) : (
            <div style={{ fontSize: 11, color: '#64748b' }}>No imagery available</div>
          )}
          {data.sentinel.acquired && (
            <div style={{ fontSize: 10, color: '#475569', marginTop: 4 }}>
              Acquired: {new Date(data.sentinel.acquired).toLocaleDateString()}
            </div>
          )}
        </div>
      )}

      {!data.loading && data.osint.length > 0 && (
        <div className="anomaly-section">
          <h3>OSINT Sources</h3>
          {data.osint.map((r, i) => (
            <div key={i} className="osint-item">
              <a href={/^https?:\/\//i.test(r.url) ? r.url : undefined} target="_blank" rel="noopener noreferrer">{r.title}</a>
              {r.snippet && <div className="osint-snippet">{r.snippet.slice(0, 160)}…</div>}
            </div>
          ))}
        </div>
      )}

      {!data.loading && data.osint.length === 0 && !data.sentinel && (
        <div style={{ fontSize: 11, color: '#475569' }}>No enrichment data available.</div>
      )}
    </aside>
  );
}
