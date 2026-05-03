"use client";

import { useIntelligenceStore } from "@/lib/store/useIntelligenceStore";
import { Activity, Globe, Cloud, Ship, Satellite, Shield, Share2, Wifi } from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";

const PROVIDER_ICONS: Record<string, React.ElementType> = {
  newsapi: Globe,
  openweather: Cloud,
  aisstream: Ship,
  sentinel: Satellite,
  ofac: Shield,
  tavily: Activity,
  neo4j: Share2,
  redis: Wifi,
};

const PROVIDER_LABELS: Record<string, string> = {
  newsapi: "NewsAPI",
  openweather: "OpenWeather",
  aisstream: "AISStream",
  sentinel: "Sentinel Hub",
  ofac: "OFAC Screening",
  tavily: "Tavily OSINT",
  neo4j: "Neo4j Graph",
  redis: "Redis Cache",
};

function latencyBadge(status: string): string {
  if (status === "reachable" || status === "operational") return "operational";
  if (status === "degraded" || status === "limited") return "degraded";
  return "offline";
}

export function ProviderHealth() {
  const providers = useIntelligenceStore((state) => state.providers);

  // Seed providers while real data loads
  const displayProviders =
    providers.length > 0
      ? providers
      : [
          { provider: "tavily", status: "checking" },
          { provider: "newsapi", status: "checking" },
          { provider: "openweather", status: "checking" },
          { provider: "aisstream", status: "checking" },
          { provider: "sentinel", status: "checking" },
          { provider: "ofac", status: "checking" },
          { provider: "neo4j", status: "checking" },
          { provider: "redis", status: "checking" },
        ];

  const operationalCount = displayProviders.filter((p) =>
    ["reachable", "operational"].includes(p.status.toLowerCase())
  ).length;

  return (
    <div className="tactical-glass rounded-lg p-5 space-y-4 h-full overflow-hidden">
      <div className="flex items-center justify-between">
        <span className="text-[10px] text-muted-foreground uppercase tracking-widest font-bold">
          Live Provider Matrix
        </span>
        <div className="flex items-center gap-1.5">
          <div className="h-1.5 w-1.5 bg-operational-green rounded-full animate-pulse" />
          <span className="text-[9px] text-operational-green uppercase font-mono">
            {operationalCount}/{displayProviders.length} Online
          </span>
        </div>
      </div>

      <div className="grid grid-cols-1 gap-2.5 overflow-y-auto scrollbar-hide">
        <AnimatePresence>
          {displayProviders.map((p, i) => {
            const Icon = PROVIDER_ICONS[p.provider.toLowerCase()] ?? Activity;
            const state = latencyBadge(p.status.toLowerCase());
            const isOk = state === "operational";
            const isDegraded = state === "degraded";
            const isChecking = p.status === "checking";

            return (
              <motion.div
                key={p.provider}
                initial={{ opacity: 0, x: 10 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: i * 0.04 }}
                className="flex items-center justify-between group py-0.5"
              >
                <div className="flex items-center gap-3">
                  <div className="p-1.5 rounded bg-white/5 group-hover:bg-white/10 transition-colors">
                    <Icon
                      className={`h-3 w-3 ${
                        isOk
                          ? "text-intel-blue"
                          : isDegraded
                            ? "text-threat-amber"
                            : isChecking
                              ? "text-muted-foreground animate-pulse"
                              : "text-threat-red"
                      }`}
                    />
                  </div>
                  <span className="text-xs text-white/80 group-hover:text-white transition-colors">
                    {PROVIDER_LABELS[p.provider.toLowerCase()] ?? p.provider}
                  </span>
                </div>

                <div className="flex items-center gap-3">
                  <span
                    className={`text-[9px] font-mono font-bold uppercase tracking-tighter ${
                      isOk
                        ? "text-operational-green"
                        : isDegraded
                          ? "text-threat-amber"
                          : isChecking
                            ? "text-muted-foreground"
                            : "text-threat-red"
                    }`}
                  >
                    {isChecking ? "—" : p.status}
                  </span>
                  <div
                    className={`h-1.5 w-1.5 rounded-full ${
                      isOk
                        ? "bg-operational-green shadow-[0_0_5px_rgba(0,255,133,0.5)]"
                        : isDegraded
                          ? "bg-threat-amber shadow-[0_0_5px_rgba(255,176,32,0.3)]"
                          : isChecking
                            ? "bg-white/20 animate-pulse"
                            : "bg-threat-red"
                    }`}
                  />
                </div>
              </motion.div>
            );
          })}
        </AnimatePresence>
      </div>
    </div>
  );
}
