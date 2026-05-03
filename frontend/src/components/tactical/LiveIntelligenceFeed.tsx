"use client";

import { motion, AnimatePresence } from "framer-motion";
import { Zap, ArrowRight, AlertTriangle, Ship, Satellite, Globe, Navigation } from "lucide-react";
import { useIntelligenceStore, FeedItem } from "@/lib/store/useIntelligenceStore";

const SOURCE_ICONS: Record<string, React.ElementType> = {
  aisstream: Ship,
  sentinel: Satellite,
  fusion: Zap,
  newsapi: Globe,
};

const TYPE_STYLES = {
  CRITICAL: {
    badge: "text-threat-red bg-threat-red/10 border-threat-red/30",
    dot: "bg-threat-red shadow-[0_0_6px_#FF3B5C]",
    border: "hover:border-threat-red/30",
  },
  HIGH: {
    badge: "text-threat-amber bg-threat-amber/10 border-threat-amber/30",
    dot: "bg-threat-amber shadow-[0_0_5px_#FFB020]",
    border: "hover:border-threat-amber/30",
  },
  ELEVATED: {
    badge: "text-intel-blue bg-intel-blue/10 border-intel-blue/30",
    dot: "bg-intel-blue shadow-[0_0_5px_#00D4FF]",
    border: "hover:border-intel-blue/30",
  },
  MONITORED: {
    badge: "text-muted-foreground bg-white/5 border-white/10",
    dot: "bg-muted-foreground",
    border: "hover:border-white/20",
  },
  INFO: {
    badge: "text-muted-foreground bg-white/5 border-white/10",
    dot: "bg-muted-foreground",
    border: "hover:border-white/20",
  },
};

// Static seed items shown before real data arrives
const SEED_ITEMS: FeedItem[] = [
  {
    id: "seed-1",
    type: "HIGH",
    title: "Border Monitoring Active",
    desc: "Continuous surveillance of Ladakh, Kargil and Siachen sectors",
    location: "34.15°N 77.58°E",
    time: "--:--:--",
    source: "fusion",
  },
  {
    id: "seed-2",
    type: "ELEVATED",
    title: "AIS Maritime Stream Active",
    desc: "Vessel tracking active for Indian Ocean and Arabian Sea",
    location: "Indian Ocean",
    time: "--:--:--",
    source: "aisstream",
  },
  {
    id: "seed-3",
    type: "MONITORED",
    title: "Satellite Surveillance Online",
    desc: "Sentinel-2 thermal & optical imagery processing",
    location: "North India",
    time: "--:--:--",
    source: "sentinel",
  },
];

export function LiveIntelligenceFeed() {
  const feedItems = useIntelligenceStore((s) => s.feedItems);
  const displayItems = feedItems.length > 0 ? feedItems : SEED_ITEMS;

  return (
    <div className="flex flex-col gap-2 h-full overflow-hidden bg-[#050816]/40 border border-white/5 rounded-sm p-3 relative">
      {/* HEADER */}
      <div className="flex items-center justify-between shrink-0 mb-2 border-b border-white/5 pb-2">
        <div className="flex items-center gap-2">
          <div className="h-2 w-2 bg-intel-blue rounded-full shadow-[0_0_8px_rgba(0,149,255,0.8)]" />
          <span className="text-[10px] text-white font-bold uppercase tracking-[0.2em]">
            Live Intelligence Feed
          </span>
        </div>
        <button className="text-[8px] text-muted-foreground uppercase hover:text-white transition-colors tracking-widest font-bold">
          View Ops
        </button>
      </div>

      <div className="space-y-2 overflow-y-auto pr-1 scrollbar-hide flex-1">
        <AnimatePresence initial={false}>
          {displayItems.map((item) => {
            const isCritical = item.type === 'CRITICAL' || item.type === 'HIGH';
            
            return (
              <motion.div
                key={item.id}
                initial={{ opacity: 0, x: -20 }}
                animate={{ opacity: 1, x: 0 }}
                className={`p-3 bg-white/[0.02] border-l-2 ${
                  item.type === 'CRITICAL' ? 'border-threat-red bg-threat-red/[0.03]' :
                  item.type === 'HIGH' ? 'border-threat-amber bg-threat-amber/[0.03]' :
                  'border-intel-blue bg-intel-blue/[0.03]'
                } flex gap-4 group cursor-pointer hover:bg-white/[0.05] transition-all relative overflow-hidden`}
              >
                {/* SCANLINE EFFECT ON CRITICAL */}
                {isCritical && (
                  <div className="absolute inset-0 pointer-events-none opacity-10 bg-[linear-gradient(rgba(18,16,16,0)_50%,rgba(0,0,0,0.25)_50%)] bg-[length:100%_2px]" />
                )}

                <div className="flex flex-col justify-start pt-1 shrink-0">
                   <span className="text-[8px] text-muted-foreground font-mono leading-none font-bold">{item.time}</span>
                   <div className="mt-2 h-1 w-full bg-white/5 rounded-full overflow-hidden">
                      <motion.div 
                        className={`h-full ${isCritical ? 'bg-threat-red' : 'bg-intel-blue'}`}
                        animate={{ width: ['0%', '100%'] }}
                        transition={{ duration: 0.5 }}
                      />
                   </div>
                </div>

                <div className="flex-1 min-w-0">
                  <div className="flex items-center justify-between mb-1">
                    <span className={`text-[8px] font-bold uppercase tracking-widest ${
                      item.type === 'CRITICAL' ? 'text-threat-red' :
                      item.type === 'HIGH' ? 'text-threat-amber' : 'text-intel-blue'
                    }`}>
                      {item.type} // {item.source.toUpperCase()}
                    </span>
                    <span className="text-[8px] text-white/40 font-bold uppercase tracking-tighter">
                      Conf: 92%
                    </span>
                  </div>
                  <h4 className="text-[11px] font-bold text-white truncate leading-tight tracking-tight uppercase group-hover:text-intel-blue transition-colors">
                    {item.title}
                  </h4>
                  <p className="text-[9px] text-muted-foreground/80 mt-1 leading-normal line-clamp-2 font-medium">
                    {item.desc}
                  </p>
                  <div className="flex items-center gap-2 mt-2 opacity-60">
                     <Navigation className="h-2 w-2 text-intel-blue" />
                     <span className="text-[8px] text-muted-foreground font-bold truncate uppercase tracking-tighter">
                       Sector: {item.location}
                     </span>
                  </div>
                </div>
              </motion.div>
            );
          })}
        </AnimatePresence>
      </div>
    </div>
  );
}
