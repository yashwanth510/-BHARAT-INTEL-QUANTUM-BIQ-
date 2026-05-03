"use client";

import { Terminal, Maximize2, Download, Search, Wifi, WifiOff } from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
import { useEffect, useRef, useState, useMemo } from "react";
import { useIntelligenceStore, LogEntry } from "@/lib/store/useIntelligenceStore";
import { cn } from "@/lib/utils";

const SEVERITY_STYLES = {
  CRITICAL: "bg-threat-red/20 text-threat-red",
  HIGH: "bg-threat-amber/20 text-threat-amber",
  ELEVATED: "bg-intel-blue/10 text-intel-blue",
  MONITORED: "bg-intel-blue/10 text-intel-blue",
  INFO: "bg-white/5 text-muted-foreground",
} as const;

const WS_URL =
  typeof window !== "undefined"
    ? `${window.location.origin.replace(/^http/, "ws")}/ws/stream/global`
    : `${process.env.NEXT_PUBLIC_WS_URL ?? "ws://localhost:8000"}/ws/stream/global`;

export function OperationsLog() {
  const logs = useIntelligenceStore((state) => state.logs);
  const wsConnected = useIntelligenceStore((s) => s.wsConnected);
  const setWsConnected = useIntelligenceStore((s) => s.setWsConnected);
  const addLog = useIntelligenceStore((s) => s.addLog);

  const scrollRef = useRef<HTMLDivElement>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const backoffRef = useRef(1000);
  const mountedRef = useRef(true);

  const [filter, setFilter] = useState("");
  const [autoScroll, setAutoScroll] = useState(true);

  // ── WebSocket direct connection for log stream ─────────────────────────────
  useEffect(() => {
    mountedRef.current = true;

    function connect() {
      if (!mountedRef.current) return;
      try {
        const ws = new WebSocket(WS_URL);
        wsRef.current = ws;

        ws.onopen = () => {
          setWsConnected(true);
          backoffRef.current = 1000;
        };

        ws.onmessage = (ev) => {
          if (ev.data === "biq-ping") {
            ws.send("pong");
            return;
          }
          try {
            const msg = JSON.parse(ev.data as string);
            const severity =
              msg.priority === "high"
                ? ("HIGH" as const)
                : msg.priority === "medium"
                  ? ("ELEVATED" as const)
                  : ("INFO" as const);

            addLog({
              id: `ws-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
              timestamp: new Date(msg.timestamp).toLocaleTimeString("en-IN", {
                hour12: false,
              }),
              source: (msg.source ?? "WS").toUpperCase(),
              event: (msg.message ?? "UPDATE")
                .slice(0, 60)
                .toUpperCase()
                .replace(/\s+/g, "_"),
              location: msg.location ?? "—",
              severity,
            });
          } catch {
            // Ignore malformed frames
          }
        };

        ws.onclose = () => {
          setWsConnected(false);
          if (!mountedRef.current) return;
          const delay = Math.min(backoffRef.current, 30_000);
          backoffRef.current = Math.min(backoffRef.current * 2, 30_000);
          reconnectRef.current = setTimeout(connect, delay);
        };

        ws.onerror = () => ws.close();
      } catch {
        const delay = Math.min(backoffRef.current, 30_000);
        backoffRef.current = Math.min(backoffRef.current * 2, 30_000);
        reconnectRef.current = setTimeout(connect, delay);
      }
    }

    connect();

    return () => {
      mountedRef.current = false;
      if (reconnectRef.current) clearTimeout(reconnectRef.current);
      if (wsRef.current) {
        wsRef.current.onclose = null;
        wsRef.current.close(1000, "unmounted");
      }
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // ── Auto-scroll ────────────────────────────────────────────────────────────
  useEffect(() => {
    if (autoScroll && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs, autoScroll]);

  // ── Filtered logs ──────────────────────────────────────────────────────────
  const filtered = useMemo(() => {
    if (!filter.trim()) return logs;
    const q = filter.toLowerCase();
    return logs.filter(
      (l) =>
        l.source.toLowerCase().includes(q) ||
        l.event.toLowerCase().includes(q) ||
        l.location.toLowerCase().includes(q)
    );
  }, [logs, filter]);

  const handleExport = () => {
    const csv = [
      "Timestamp,Source,Event,Location,Severity",
      ...logs.map(
        (l) =>
          `"${l.timestamp}","${l.source}","${l.event}","${l.location}","${l.severity}"`
      ),
    ].join("\n");
    const blob = new Blob([csv], { type: "text/csv" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `biq-ops-log-${Date.now()}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="h-64 border-t border-white/5 bg-black/40 flex flex-col font-mono shrink-0">
      {/* HEADER */}
      <div className="h-8 border-b border-white/5 bg-white/5 flex items-center justify-between px-4 shrink-0">
        <div className="flex items-center gap-2">
          <Terminal className="h-3 w-3 text-intel-blue" />
          <span className="text-[10px] text-white font-bold uppercase tracking-widest">
            Live Operations Log
          </span>
          <div className="flex items-center gap-1.5 ml-4">
            {wsConnected ? (
              <>
                <Wifi className="h-2.5 w-2.5 text-operational-green" />
                <span className="text-[9px] text-operational-green uppercase">
                  WS Live
                </span>
              </>
            ) : (
              <>
                <WifiOff className="h-2.5 w-2.5 text-threat-amber animate-pulse" />
                <span className="text-[9px] text-threat-amber uppercase">
                  Reconnecting…
                </span>
              </>
            )}
          </div>
          <span className="text-[9px] text-muted-foreground ml-2 font-mono">
            {filtered.length} entries
          </span>
        </div>

        <div className="flex items-center gap-4">
          <div className="flex items-center gap-1 border-x border-white/5 px-3">
            <Search className="h-3 w-3 text-muted-foreground" />
            <input
              type="text"
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              placeholder="Filter logs..."
              className="bg-transparent border-none text-[10px] text-white focus:outline-none w-32 placeholder:text-muted-foreground/30"
            />
          </div>
          <button
            onClick={handleExport}
            className="text-muted-foreground hover:text-white transition-colors"
            title="Export CSV"
          >
            <Download className="h-3 w-3" />
          </button>
          <button
            onClick={() => setAutoScroll((v) => !v)}
            className={`transition-colors ${autoScroll ? "text-intel-blue" : "text-muted-foreground hover:text-white"}`}
            title={autoScroll ? "Auto-scroll ON" : "Auto-scroll OFF"}
          >
            <Maximize2 className="h-3 w-3" />
          </button>
        </div>
      </div>

      {/* LOG STREAM */}
      <div
        ref={scrollRef}
        onScroll={(e) => {
          const el = e.currentTarget;
          const atBottom =
            el.scrollHeight - el.scrollTop - el.clientHeight < 20;
          setAutoScroll(atBottom);
        }}
        className="flex-1 overflow-y-auto p-3 space-y-0.5 scrollbar-thin scrollbar-track-transparent scrollbar-thumb-white/10"
      >
        <AnimatePresence initial={false}>
          {filtered.length === 0 && (
            <div className="flex items-center justify-center h-full">
              <span className="text-[10px] text-muted-foreground uppercase tracking-widest">
                Awaiting intelligence stream…
              </span>
            </div>
          )}
          {filtered.map((log: LogEntry) => (
            <motion.div
              key={log.id}
              initial={{ opacity: 0, x: -5 }}
              animate={{ opacity: 1, x: 0 }}
              className="flex items-start gap-4 text-[11px] group py-0.5 hover:bg-white/5 rounded px-2 transition-all"
            >
              <span className="text-muted-foreground whitespace-nowrap">
                [{log.timestamp}]
              </span>
              <span
                className={cn(
                  "font-bold px-1 rounded min-w-[80px] text-center",
                  SEVERITY_STYLES[log.severity] ?? SEVERITY_STYLES.INFO
                )}
              >
                [{log.source}]
              </span>
              <span
                className={cn(
                  "min-w-[160px]",
                  log.severity === "CRITICAL"
                    ? "text-threat-red"
                    : log.severity === "HIGH"
                      ? "text-threat-amber"
                      : "text-white/80"
                )}
              >
                [{log.event}]
              </span>
              <span className="text-muted-foreground flex-1 truncate">
                LOC: {log.location}
              </span>
            </motion.div>
          ))}
        </AnimatePresence>
      </div>

      {/* STATUS BAR */}
      <div className="h-6 border-t border-white/5 bg-black/40 px-4 flex items-center justify-between text-[9px] text-muted-foreground uppercase tracking-tighter shrink-0">
        <div className="flex gap-4">
          <span>WebSocket: {wsConnected ? "Connected" : "Reconnecting"}</span>
          <span>Stream Buffer: {logs.length}/200</span>
        </div>
        <div className="flex gap-2 items-center">
          <span className={wsConnected ? "text-operational-green" : "text-threat-amber"}>
            {wsConnected ? "Stream Intact" : "Degraded Mode"}
          </span>
          <div className="h-1 w-12 bg-white/5 rounded-full overflow-hidden">
            <div
              className={`h-full ${wsConnected ? "bg-operational-green" : "bg-threat-amber"} transition-all`}
              style={{ width: wsConnected ? "95%" : "30%" }}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
