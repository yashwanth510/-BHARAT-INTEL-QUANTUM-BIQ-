"use client";

import { useState, useCallback, useRef, useEffect } from "react";
import { Search, Bell, User, ShieldAlert, Cpu, Wifi, WifiOff, Activity, Mail } from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
import { useIntelligenceStore } from "@/lib/store/useIntelligenceStore";
import { cn } from "@/lib/utils";
import { api } from "@/lib/api";

const SUGGESTIONS = [
  { label: "Ladakh border activity", category: "BORDER" },
  { label: "Dark vessel detection", category: "MARITIME" },
  { label: "Satellite anomaly scan", category: "SATELLITE" },
  { label: "Financial threat screening", category: "FINANCIAL" },
  { label: "Cross-border threat fusion", category: "FUSION" },
  { label: "Pakistan threat assessment", category: "BORDER" },
  { label: "China PLA activity", category: "BORDER" },
  { label: "Indian Ocean maritime status", category: "MARITIME" },
];

export function CommandHeader() {
  const { wsConnected, apiHealthy, threatLevel, fusionScore, activeAlerts, vessels, updateFusion, setLastRefresh, prependFeedItem } =
    useIntelligenceStore();

  const [query, setQuery] = useState("");
  const [isSearching, setIsSearching] = useState(false);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const searchRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const handleSearch = useCallback(async (searchQuery: string) => {
    if (!searchQuery.trim()) return;
    setIsSearching(true);
    setShowSuggestions(false);
    
    try {
      const result = await api.intelligence(searchQuery);
      if (result && result.fusion) {
        updateFusion(
          result.fusion.score,
          result.fusion.risk,
          result.metadata.confidence,
          result.fusion.recommendations
        );
        
        // Add a feed item for the search
        prependFeedItem({
          id: `search-${Date.now()}`,
          type: (result.fusion.risk as any) || "INFO",
          title: `INTELLIGENCE SYNTHESIS: ${searchQuery.toUpperCase()}`,
          desc: result.strategic_synthesis || "Multi-source intelligence fusion complete. Assessment generated.",
          location: result.location || "Global Sector",
          time: new Date().toLocaleTimeString(),
          source: "fusion"
        });

        setLastRefresh(new Date().toISOString());
      }
    } catch (error) {
      console.error("Intelligence search failed:", error);
    } finally {
      setIsSearching(false);
    }
  }, [updateFusion, setLastRefresh, prependFeedItem]);

  useEffect(() => {
    const handleFocusSearch = () => {
      inputRef.current?.focus();
      setShowSuggestions(true);
    };
    window.addEventListener("focus-ai-command", handleFocusSearch);
    
    const handleClickOutside = (event: MouseEvent) => {
      if (searchRef.current && !searchRef.current.contains(event.target as Node)) {
        setShowSuggestions(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => {
      window.removeEventListener("focus-ai-command", handleFocusSearch);
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, []);

  return (
    <header className="h-12 border-b border-white/5 bg-[#02040a] z-50 flex items-center justify-between px-4 shrink-0 overflow-hidden">
      {/* LOGO & TITLE */}
      <div className="flex items-center gap-3 shrink-0">
        <div className="h-7 w-7 bg-intel-blue/20 border border-intel-blue/40 rounded-sm flex items-center justify-center relative group shadow-[0_0_10px_rgba(0,149,255,0.2)]">
           <Cpu className="h-4 w-4 text-intel-blue relative z-10" />
        </div>
        <div className="flex flex-col">
          <span className="text-[11px] font-bold tracking-[0.2em] text-white uppercase leading-none">
            Bharat Intel Quantum
          </span>
          <span className="text-[7px] text-muted-foreground font-mono uppercase tracking-[0.3em] mt-1">
            Tactical Intelligence OS // V2.1.0
          </span>
        </div>
      </div>

      {/* AI COMMAND SEARCH */}
      <div className="flex-1 max-w-xl px-8 relative" ref={searchRef}>
        <div className="relative group">
          <div className={cn(
            "absolute inset-0 bg-intel-blue/5 rounded-sm transition-all duration-300",
            isSearching ? "opacity-100" : "opacity-0 group-hover:opacity-100"
          )} />
          <div className="relative flex items-center">
            <Search className={cn(
              "absolute left-3 h-3.5 w-3.5 transition-colors",
              isSearching ? "text-intel-blue animate-pulse" : "text-white/30 group-hover:text-intel-blue"
            )} />
            <input
              ref={inputRef}
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onFocus={() => setShowSuggestions(true)}
              onKeyDown={(e) => e.key === "Enter" && handleSearch(query)}
              placeholder="ENTER TACTICAL QUERY OR COMMAND..."
              className="w-full bg-white/[0.03] border border-white/10 rounded-sm py-1.5 pl-10 pr-4 text-[10px] text-white placeholder:text-white/20 focus:outline-none focus:border-intel-blue/40 focus:bg-white/[0.05] transition-all uppercase font-bold tracking-wider"
            />
            {isSearching && (
              <div className="absolute right-3 flex items-center gap-2">
                <div className="h-1 w-1 bg-intel-blue rounded-full animate-ping" />
                <span className="text-[7px] text-intel-blue font-bold animate-pulse">ANALYSING</span>
              </div>
            )}
          </div>
        </div>

        <AnimatePresence>
          {showSuggestions && (
            <motion.div
              initial={{ opacity: 0, y: -10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              className="absolute top-full left-8 right-8 mt-1 bg-[#0a0c14] border border-white/10 rounded-sm shadow-2xl z-[60] overflow-hidden"
            >
              <div className="p-2 border-b border-white/5 bg-white/[0.02]">
                <span className="text-[7px] text-white/40 uppercase font-bold tracking-widest">Suggested Tactical Queries</span>
              </div>
              <div className="max-h-48 overflow-y-auto scrollbar-hide">
                {SUGGESTIONS.map((s, i) => (
                  <button
                    key={i}
                    onClick={() => {
                      setQuery(s.label);
                      handleSearch(s.label);
                    }}
                    className="w-full flex items-center justify-between px-3 py-2 hover:bg-white/5 transition-colors group"
                  >
                    <div className="flex items-center gap-3">
                      <div className="h-1 w-1 bg-white/20 rounded-full group-hover:bg-intel-blue transition-colors" />
                      <span className="text-[9px] text-white/60 group-hover:text-white transition-colors uppercase font-bold tracking-tight">{s.label}</span>
                    </div>
                    <span className="text-[7px] text-white/20 group-hover:text-intel-blue font-mono">{s.category}</span>
                  </button>
                ))}
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      {/* METRICS ROW (Condensed) */}
      <div className="flex items-center gap-6 pr-4">
        {[
          { label: 'Fusion', value: fusionScore.toFixed(2), color: 'text-intel-blue' },
          { label: 'Conf', value: '87%', color: 'text-operational-green' },
          { label: 'Alerts', value: activeAlerts, color: 'text-threat-red' },
        ].map(m => (
          <div key={m.label} className="flex flex-col items-center">
             <span className="text-[6px] text-white/30 uppercase tracking-widest font-bold">{m.label}</span>
             <span className={cn("text-[10px] font-bold tracking-tighter", m.color)}>{m.value}</span>
          </div>
        ))}
      </div>

      {/* RIGHT ACTIONS */}
      <div className="flex items-center gap-4 shrink-0">
        <div className="flex items-center gap-2 pr-4 border-r border-white/5">
           <button className="p-1.5 hover:bg-white/5 rounded transition-colors relative">
              <Bell className="h-3.5 w-3.5 text-white/60" />
              <span className="absolute top-1 right-1 h-1.5 w-1.5 bg-threat-red rounded-full border border-[#02040a]" />
           </button>
           <button className="p-1.5 hover:bg-white/5 rounded transition-colors">
              <Mail className="h-3.5 w-3.5 text-white/60" />
           </button>
        </div>

        <div className="flex items-center gap-3 pl-2">
           <div className="flex flex-col items-end leading-none">
              <span className="text-[9px] font-bold text-white uppercase tracking-widest">Analyst</span>
              <span className="text-[7px] text-white/40 font-mono mt-0.5">ID: BIQ-7260</span>
           </div>
           <div className="h-7 w-7 rounded-sm bg-gradient-to-br from-intel-blue/20 to-purple-500/20 border border-white/10 flex items-center justify-center overflow-hidden">
              <User className="h-3.5 w-3.5 text-intel-blue" />
           </div>
        </div>
      </div>
    </header>
  );
}
