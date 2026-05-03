"use client";

import { useEffect, useRef, useCallback } from "react";
import { useIntelligenceStore } from "@/lib/store/useIntelligenceStore";

interface WsMessage {
  type: string;
  priority?: string;
  source?: string;
  location?: string;
  message?: string;
  timestamp: string;
  // Vessel fields
  mmsi?: string;
  lat?: number;
  lon?: number;
  vessel_name?: string;
  risk_score?: number;
}

interface UseWebSocketOptions {
  url: string;
  /** ms between reconnect attempts — doubles on each failure (capped at maxBackoff) */
  initialBackoff?: number;
  maxBackoff?: number;
  /** if true, only connect when window is visible */
  pauseWhenHidden?: boolean;
}

/**
 * Production-grade WebSocket hook with:
 * - Automatic reconnection + exponential backoff
 * - Heartbeat/pong response
 * - Pause when tab is hidden
 * - Clean teardown
 */
export function useBiqWebSocket({
  url,
  initialBackoff = 1000,
  maxBackoff = 30000,
  pauseWhenHidden = true,
}: UseWebSocketOptions) {
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const backoffRef = useRef(initialBackoff);
  const mountedRef = useRef(true);

  const { addLog, updateFusion, setProviders, incrementAlerts } =
    useIntelligenceStore.getState();

  const handleMessage = useCallback(
    (raw: string) => {
      let msg: WsMessage;
      try {
        msg = JSON.parse(raw);
      } catch {
        return;
      }

      // Handle Vessel Updates
      if (msg.mmsi) {
        // Handled by TacticalMap component's internal WS for 60fps rendering
        return;
      }

      // Standard Messages
      const priority = msg.priority || "low";
      const source = msg.source || "SYSTEM";
      const message = msg.message || "UPDATE";
      const location = msg.location || "GLOBAL";

      const severity =
        priority === "high"
          ? ("HIGH" as const)
          : priority === "medium"
            ? ("ELEVATED" as const)
            : ("INFO" as const);

      addLog({
        id: `ws-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
        timestamp: new Date(msg.timestamp).toLocaleTimeString("en-IN", {
          hour12: false,
        }),
        source: source.toUpperCase(),
        event: message.slice(0, 60).toUpperCase().replace(/\s+/g, "_"),
        location: location,
        severity,
      });

      if (severity === "HIGH" || severity === ("CRITICAL" as string)) {
        incrementAlerts();
      }
    },
    [addLog, incrementAlerts]
  );

  const connectRef = useRef<() => void>(() => {});

  const connect = useCallback(() => {
    if (!mountedRef.current) return;
    if (pauseWhenHidden && typeof document !== "undefined" && document.hidden)
      return;

    try {
      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onopen = () => {
        backoffRef.current = initialBackoff; // reset on success
      };

      ws.onmessage = (ev) => {
        if (ev.data === "biq-ping") {
          ws.send("pong");
          return;
        }
        handleMessage(ev.data as string);
      };

      ws.onclose = () => {
        if (!mountedRef.current) return;
        const delay = Math.min(backoffRef.current, maxBackoff);
        backoffRef.current = Math.min(backoffRef.current * 2, maxBackoff);
        reconnectTimer.current = setTimeout(() => connectRef.current(), delay);
      };

      ws.onerror = () => {
        ws.close();
      };
    } catch {
      const delay = Math.min(backoffRef.current, maxBackoff);
      backoffRef.current = Math.min(backoffRef.current * 2, maxBackoff);
      reconnectTimer.current = setTimeout(() => connectRef.current(), delay);
    }
  }, [url, initialBackoff, maxBackoff, pauseWhenHidden, handleMessage]);

  useEffect(() => {
    connectRef.current = connect;
  }, [connect]);

  useEffect(() => {
    mountedRef.current = true;
    connect();

    // Pause/resume on visibility change
    const handleVisibility = () => {
      if (!document.hidden && !wsRef.current) connect();
    };
    if (pauseWhenHidden) {
      document.addEventListener("visibilitychange", handleVisibility);
    }

    return () => {
      mountedRef.current = false;
      if (reconnectTimer.current) clearTimeout(reconnectTimer.current);
      if (wsRef.current) {
        wsRef.current.onclose = null; // prevent reconnect on teardown
        wsRef.current.close(1000, "component unmounted");
      }
      document.removeEventListener("visibilitychange", handleVisibility);
    };
  }, [connect, pauseWhenHidden]);
}
