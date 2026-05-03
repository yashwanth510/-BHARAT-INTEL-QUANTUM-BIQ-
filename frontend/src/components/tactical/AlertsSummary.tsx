"use client";

import { useIntelligenceStore } from "@/lib/store/useIntelligenceStore";
import { AlertCircle, ChevronRight } from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
import { useMemo } from "react";

const SEVERITY_LEVELS = [
  { level: "Critical", severity: "CRITICAL", color: "bg-threat-red" },
  { level: "High", severity: "HIGH", color: "bg-threat-red/60" },
  { level: "Elevated", severity: "ELEVATED", color: "bg-threat-amber" },
  { level: "Monitored", severity: "MONITORED", color: "bg-intel-blue" },
  { level: "Info", severity: "INFO", color: "bg-muted-foreground" },
];

export function AlertsSummary() {
  const { activeAlerts, logs } = useIntelligenceStore();

  // Count alerts by severity from the live log stream
  const severityCounts = useMemo(() => {
    const counts: Record<string, number> = {
      CRITICAL: 0,
      HIGH: 0,
      ELEVATED: 0,
      MONITORED: 0,
      INFO: 0,
    };
    logs.slice(0, 100).forEach((l) => {
      if (l.severity in counts) counts[l.severity]++;
    });
    return counts;
  }, [logs]);

  const maxAlerts = Math.max(activeAlerts, 20);
  const dashOffset = 377 - 377 * Math.min(activeAlerts / maxAlerts, 1);

  const getAlertColor = () => {
    if (activeAlerts > 15) return "#FF3B5C";
    if (activeAlerts > 8) return "#FFB020";
    if (activeAlerts > 3) return "#00D4FF";
    return "#00FF85";
  };

  return (
    <div className="tactical-glass rounded-lg p-6 flex flex-col gap-6 h-full">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <AlertCircle className="h-4 w-4 text-threat-red" />
          <h3 className="text-sm font-bold text-white uppercase tracking-widest">
            Alerts Summary
          </h3>
        </div>
        <div className="h-1 w-1 bg-threat-red rounded-full animate-pulse" />
      </div>

      <div className="flex-1 flex flex-col items-center justify-center relative">
        {/* CIRCULAR GAUGE */}
        <div className="relative h-32 w-32 flex items-center justify-center">
          <svg className="absolute inset-0 h-full w-full -rotate-90">
            <circle
              cx="64"
              cy="64"
              r="58"
              fill="none"
              stroke="rgba(255,255,255,0.05)"
              strokeWidth="8"
            />
            <motion.circle
              cx="64"
              cy="64"
              r="58"
              fill="none"
              stroke={getAlertColor()}
              strokeWidth="8"
              strokeDasharray="377"
              initial={{ strokeDashoffset: 377 }}
              animate={{ strokeDashoffset: dashOffset }}
              transition={{ duration: 1.5, ease: "easeOut" }}
              strokeLinecap="round"
              style={{ filter: `drop-shadow(0 0 6px ${getAlertColor()})` }}
            />
          </svg>
          <div className="flex flex-col items-center z-10">
            <AnimatePresence mode="wait">
              <motion.span
                key={activeAlerts}
                initial={{ scale: 1.3, opacity: 0 }}
                animate={{ scale: 1, opacity: 1 }}
                exit={{ scale: 0.8, opacity: 0 }}
                className="text-3xl font-bold text-white"
              >
                {activeAlerts}
              </motion.span>
            </AnimatePresence>
            <span className="text-[10px] text-muted-foreground uppercase font-bold tracking-widest">
              Active
            </span>
            <span className="text-[10px] text-threat-red font-bold uppercase tracking-widest">
              Alerts
            </span>
          </div>
        </div>

        {/* SEVERITY BREAKDOWN */}
        <div className="w-full mt-6 space-y-2.5">
          {SEVERITY_LEVELS.map((a) => {
            const count = severityCounts[a.severity] ?? 0;
            return (
              <div key={a.level} className="flex items-center justify-between group">
                <div className="flex items-center gap-3">
                  <div className={`h-1.5 w-1.5 rounded-full ${a.color}`} />
                  <span className="text-xs text-white/80 group-hover:text-white transition-colors">
                    {a.level}
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  {/* mini bar */}
                  <div className="w-16 h-1 bg-white/5 rounded-full overflow-hidden">
                    <motion.div
                      className={`h-full rounded-full ${a.color}`}
                      initial={{ width: 0 }}
                      animate={{ width: `${Math.min((count / Math.max(logs.length / 10, 1)) * 100, 100)}%` }}
                      transition={{ duration: 0.8 }}
                    />
                  </div>
                  <span className="text-xs font-mono font-bold text-white min-w-[16px] text-right">
                    {count}
                  </span>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      <button className="w-full py-2 bg-white/5 border border-white/10 rounded text-[9px] text-white uppercase font-bold tracking-widest hover:bg-white/10 transition-all flex items-center justify-center gap-2">
        View All Alerts <ChevronRight className="h-3 w-3" />
      </button>
    </div>
  );
}
