"use client";

/**
 * BiqDataProvider
 *
 * Wraps the entire application. Responsibilities:
 * 1. Open WebSocket to /ws/stream/global (with auto-reconnect)
 * 2. Poll /health every 30s → update provider status
 * 3. Poll /api/intelligence every 120s → update fusion score
 * 4. Poll /ops-log every 20s → push log entries
 * 5. Poll /maritime-threats every 60s → update vessel map layer
 */

import { useEffect, useRef } from "react";
import { useBiqWebSocket } from "@/hooks/useBiqWebSocket";
import { api } from "@/lib/api";
import { useIntelligenceStore } from "@/lib/store/useIntelligenceStore";

const WS_URL =
  (typeof window !== "undefined"
    ? window.location.origin.replace(/^http/, "ws")
    : (process.env.NEXT_PUBLIC_WS_URL ?? "ws://localhost:8000")) +
  "/ws/stream/global";

function usePoll(fn: () => Promise<void>, intervalMs: number) {
  const fnRef = useRef(fn);
  
  useEffect(() => {
    fnRef.current = fn;
  }, [fn]);

  useEffect(() => {
    // Run immediately, then on interval
    fnRef.current().catch(() => {});
    const id = setInterval(() => fnRef.current().catch(() => {}), intervalMs);
    return () => clearInterval(id);
  }, [intervalMs]);
}

export function BiqDataProvider({ children }: { children: React.ReactNode }) {
  const store = useIntelligenceStore();

  // ── WebSocket ──────────────────────────────────────────────────────────────
  useBiqWebSocket({ url: WS_URL });

  // ── Health / Provider Matrix ───────────────────────────────────────────────
  usePoll(async () => {
    try {
      const data = await api.health();
      store.setProviders(data.providers ?? []);
      store.setApiHealthy(true);
    } catch {
      store.setApiHealthy(false);
    }
  }, 30_000);

  // ── Intelligence Fusion ────────────────────────────────────────────────────
  usePoll(async () => {
    try {
      const data = await api.intelligence();
      const { fusion } = data;
      const confidence = Math.round((data.metadata?.confidence ?? 0) * 100);
      store.updateFusion(
        parseFloat(fusion.score.toFixed(2)),
        fusion.risk,
        confidence,
        fusion.recommendations ?? []
      );
      store.setLastRefresh(new Date().toISOString());

      // Surface as feed item if high priority
      if (fusion.risk === "CRITICAL" || fusion.risk === "HIGH") {
        store.prependFeedItem({
          id: `intel-${Date.now()}`,
          type: fusion.risk as "CRITICAL" | "HIGH",
          title: `Threat Level: ${fusion.risk}`,
          desc: data.strategic_synthesis?.slice(0, 120) ?? "Multi-source fusion update",
          location: data.location ?? "Global",
          time: new Date().toLocaleTimeString("en-IN", { hour12: false }),
          source: "fusion",
        });
        store.incrementAlerts();
      }
    } catch {
      // Non-fatal — keep previous values
    }
  }, 120_000);

  // ── Operations Log ─────────────────────────────────────────────────────────
  usePoll(async () => {
    try {
      const { entries } = await api.opsLog();
      if (!Array.isArray(entries)) return;
      entries.slice(0, 20).forEach((e) => {
        store.addLog({
          id: `ops-${e.timestamp}-${Math.random().toString(36).slice(2, 6)}`,
          timestamp: new Date(e.timestamp).toLocaleTimeString("en-IN", {
            hour12: false,
          }),
          source: e.category ?? "SYSTEM",
          event: e.event ?? "UPDATE",
          location: e.detail?.slice(0, 40) ?? "—",
          severity:
            e.event?.includes("CRITICAL") || e.event?.includes("ALERT")
              ? "CRITICAL"
              : e.event?.includes("HIGH")
                ? "HIGH"
                : e.event?.includes("ELEVATED")
                  ? "ELEVATED"
                  : "INFO",
        });
      });
    } catch {
      // Non-fatal
    }
  }, 20_000);

  // ── Maritime Vessels ────────────────────────────────────────────────────────
  usePoll(async () => {
    try {
      const { results } = await api.maritimeThreats();
      store.setVessels(results as any[]);

      // Count dark vessels and surface as alerts
      const darkCount = (results as any[]).filter((v: any) => v.dark).length;
      if (darkCount > 0) {
        store.prependFeedItem({
          id: `maritime-${Date.now()}`,
          type: "HIGH",
          title: `${darkCount} Dark Vessel${darkCount > 1 ? "s" : ""} Detected`,
          desc: `AIS transponder disabled — possible covert maritime activity`,
          location: "Indian Ocean / Arabian Sea",
          time: new Date().toLocaleTimeString("en-IN", { hour12: false }),
          source: "aisstream",
        });
        store.incrementAlerts();
      }
    } catch {
      // Non-fatal
    }
  }, 60_000);

  // ── Satellite Alerts ────────────────────────────────────────────────────────
  usePoll(async () => {
    try {
      const { results } = await api.satelliteAlerts();
      if (Array.isArray(results) && results.length > 0) {
        const alert = results[0] as any;
        store.prependFeedItem({
          id: `sat-${Date.now()}`,
          type:
            alert.confidence > 0.8
              ? "CRITICAL"
              : alert.confidence > 0.6
                ? "HIGH"
                : "ELEVATED",
          title: `Sentinel Anomaly: ${alert.alert_type ?? "Unknown"}`,
          desc: `Region: ${alert.region ?? "Unknown"} — Confidence: ${((alert.confidence ?? 0) * 100).toFixed(0)}%`,
          location: alert.region ?? "Unknown",
          time: new Date().toLocaleTimeString("en-IN", { hour12: false }),
          source: "sentinel",
        });
        store.incrementAlerts();
      }
    } catch {
      // Non-fatal
    }
  }, 180_000);

  return <>{children}</>;
}
