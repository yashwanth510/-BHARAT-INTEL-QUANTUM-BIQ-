"use client";

import { motion, AnimatePresence } from "framer-motion";
import { useIntelligenceStore } from "@/lib/store/useIntelligenceStore";
import { Zap, Shield, TrendingUp, Info, RefreshCw } from "lucide-react";
import { useCallback, useState } from "react";
import { api } from "@/lib/api";
import { cn } from "@/lib/utils";

const CONTRIBUTOR_WEIGHTS = [
  { name: "Mistral AI Assessment", weight: 0.35 },
  { name: "News & Media Signals", weight: 0.20 },
  { name: "Maritime Anomalies", weight: 0.15 },
  { name: "Satellite Intelligence", weight: 0.10 },
  { name: "Financial Intel", weight: 0.10 },
  { name: "Terrain & Environment", weight: 0.10 },
];

export function IntelligenceFusionPanel() {
  const {
    fusionScore,
    threatLevel,
    confidence,
    recommendations,
    lastRefresh,
  } = useIntelligenceStore();

  const [refreshing, setRefreshing] = useState(false);

  const handleRefresh = useCallback(async () => {
    setRefreshing(true);
    try {
      await api.intelligence();
    } catch {
      // Non-fatal
    } finally {
      setRefreshing(false);
    }
  }, []);

  const getThreatColor = (level: string) => {
    switch (level) {
      case "CRITICAL":
      case "HIGH":
        return "text-threat-red";
      case "ELEVATED":
        return "text-threat-amber";
      case "MONITORED":
        return "text-intel-blue";
      case "LOADING":
        return "text-muted-foreground";
      default:
        return "text-operational-green";
    }
  };

  const getGaugeColor = (score: number) => {
    if (score > 0.8) return "#FF3B5C";
    if (score > 0.6) return "#FFB020";
    if (score > 0.3) return "#00D4FF";
    return "#00FF85";
  };

  const gaugeColor = getGaugeColor(fusionScore);
  const gaugeRotation = -90 + fusionScore * 180;

  const refreshTime = lastRefresh
    ? new Date(lastRefresh).toLocaleTimeString("en-IN", {
        hour12: false,
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
      })
    : "—";

  return (
    <div className="bg-[#050816]/40 border border-white/5 rounded-sm p-4 flex flex-col gap-4 relative overflow-hidden h-full group">
      {/* ATMOSPHERIC GLOW */}
      <div 
        className="absolute top-0 left-0 w-full h-1 opacity-50"
        style={{ background: `linear-gradient(90deg, transparent, ${gaugeColor}, transparent)` }}
      />

      {/* HEADER */}
      <div className="flex items-center justify-between shrink-0">
        <div className="flex items-center gap-2">
          <div className={`h-1.5 w-1.5 rounded-full animate-pulse`} style={{ backgroundColor: gaugeColor }} />
          <span className="text-[10px] font-bold text-white uppercase tracking-[0.2em]">
            Strategic Fusion Engine
          </span>
        </div>
        <div className="flex items-center gap-2">
          <button 
            onClick={handleRefresh}
            disabled={refreshing}
            className={cn(
              "p-1 hover:bg-white/5 rounded-full transition-all",
              refreshing && "animate-spin"
            )}
          >
            <RefreshCw className="h-3 w-3 text-muted-foreground" />
          </button>
          <span className="text-[8px] text-muted-foreground font-mono font-bold uppercase">
            T-ID: {refreshTime}
          </span>
        </div>
      </div>

      {/* SEMICIRCLE GAUGE ENGINE */}
      <div className="flex flex-col items-center justify-center relative shrink-0 py-2">
        <div className="relative w-48 h-24 overflow-hidden">
          {/* Background tracks */}
          <div className="absolute top-0 left-0 w-48 h-48 border-[12px] border-white/5 rounded-full" />
          <div className="absolute top-0 left-0 w-48 h-48 border-[1px] border-white/10 rounded-full" />

          {/* Active Gauge Arc */}
          <motion.div
            animate={{ rotate: gaugeRotation }}
            transition={{ duration: 2, ease: "circOut" }}
            className="absolute top-0 left-0 w-48 h-48 border-[12px] border-transparent rounded-full origin-center"
            style={{
              borderTopColor: gaugeColor,
              filter: `drop-shadow(0 0 10px ${gaugeColor}40)`,
            }}
          />

          {/* Numerical Readout */}
          <div className="absolute bottom-0 left-1/2 -translate-x-1/2 flex flex-col items-center">
            <motion.span
              key={fusionScore}
              initial={{ opacity: 0, scale: 0.8 }}
              animate={{ opacity: 1, scale: 1 }}
              className="text-4xl font-bold tracking-tighter text-white leading-none"
            >
              {(fusionScore * 100).toFixed(0)}
            </motion.span>
            <span
              className={`text-[9px] font-bold uppercase tracking-[0.3em] mt-2 ${getThreatColor(
                threatLevel
              )}`}
            >
              {threatLevel}
            </span>
          </div>
        </div>

        {/* METRICS SUB-PANEL */}
        <div className="flex justify-between w-full px-6 mt-6">
          <div className="flex flex-col items-start gap-1">
            <span className="text-[7px] text-muted-foreground uppercase font-bold tracking-widest">Confidence</span>
            <div className="flex items-center gap-2">
              <Shield className="h-3 w-3 text-operational-green" />
              <span className="text-xs font-bold text-white">{confidence}%</span>
            </div>
          </div>
          <div className="flex flex-col items-end gap-1">
            <span className="text-[7px] text-muted-foreground uppercase font-bold tracking-widest">Risk Trend</span>
            <div className="flex items-center gap-2">
              <TrendingUp className="h-3 w-3 text-threat-red" />
              <span className="text-xs font-bold text-threat-red">+14.2%</span>
            </div>
          </div>
        </div>
      </div>

      {/* CONTRIBUTOR BREAKDOWN */}
      <div className="flex-1 overflow-hidden flex flex-col border-t border-white/5 pt-4">
        <div className="flex items-center justify-between text-[7px] text-muted-foreground uppercase tracking-widest font-bold mb-3 px-1">
          <span className="w-1/2">Intelligence Source</span>
          <span className="w-1/6 text-center">Score</span>
          <span className="w-1/6 text-right">Status</span>
        </div>
        <div className="space-y-1.5 overflow-y-auto scrollbar-hide px-1">
          {CONTRIBUTOR_WEIGHTS.map((c, i) => {
            const score = Math.max(0, Math.min(1, fusionScore + (i * 0.05 - 0.1)));
            const status = score > 0.7 ? "high" : score > 0.4 ? "med" : "low";
            return (
              <div key={c.name} className="flex items-center justify-between p-1.5 bg-white/[0.02] border border-white/[0.03] rounded-sm group hover:bg-white/[0.05] transition-all">
                <span className="text-[9px] text-white/70 w-1/2 group-hover:text-white transition-colors font-bold uppercase truncate tracking-tight">
                  {c.name}
                </span>
                <div className="w-1/6 flex justify-center">
                   <div className="h-1 w-12 bg-white/5 rounded-full overflow-hidden">
                      <motion.div 
                        className={`h-full ${status === 'high' ? 'bg-threat-red' : 'bg-intel-blue'}`}
                        initial={{ width: 0 }}
                        animate={{ width: `${score * 100}%` }}
                      />
                   </div>
                </div>
                <div className="w-1/6 flex justify-end">
                  <div
                    className={`h-1.5 w-1.5 rounded-full ${
                      status === "high" ? "bg-threat-red shadow-[0_0_5px_#FF3B5C]" : 
                      status === "med" ? "bg-threat-amber" : "bg-operational-green"
                    }`}
                  />
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* AI RECOMMENDATION ENGINE */}
      <div className="mt-2 p-2 bg-intel-blue/[0.03] border border-intel-blue/10 rounded-sm shrink-0">
        <div className="text-[7px] text-intel-blue uppercase tracking-widest mb-1.5 font-bold">Strategic Directives</div>
        <div className="space-y-1">
           {(recommendations.length > 0 ? recommendations : ["Deploy SIGINT to Ladakh Sector", "Initiate Maritime ISR Sweep"]).slice(0, 2).map((rec, i) => (
             <div key={i} className="flex items-center gap-2">
                <div className="h-1 w-1 bg-intel-blue rounded-full" />
                <span className="text-[8px] text-white/80 font-medium uppercase tracking-tight">{rec}</span>
             </div>
           ))}
        </div>
      </div>
    </div>
  );
}
