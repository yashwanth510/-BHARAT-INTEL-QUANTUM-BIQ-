"use client";

import {
  Activity,
  Shield,
  AlertCircle,
  Ship,
  Database,
  Zap,
  TrendingUp,
  Globe,
  Map as MapIcon,
  Plus,
  Minus,
  Maximize2,
  Target,
  Settings,
} from "lucide-react";
import { motion } from "framer-motion";
import { useIntelligenceStore } from "@/lib/store/useIntelligenceStore";
import { IntelligenceFusionPanel } from "./IntelligenceFusionPanel";
import { LiveIntelligenceFeed } from "./LiveIntelligenceFeed";
import { TacticalMap } from "./TacticalMap";
import { KnowledgeGraph } from "./KnowledgeGraph";
import { cn } from "@/lib/utils";

export function MainGrid() {
  const {
    fusionScore,
    threatLevel,
    confidence,
    activeAlerts,
    vessels,
    providers,
    apiHealthy,
    mapMode,
    setMapMode,
  } = useIntelligenceStore();

  const darkVessels = vessels.filter((v) => v.dark).length;
  const operationalProviders = providers.filter((p) =>
    ["reachable", "operational"].includes(p.status.toLowerCase())
  ).length;

 const stats = [
    {
      label: "Fusion Score",
      value: fusionScore.toFixed(2),
      sub: threatLevel || "LOADING",
      icon: Zap,
      color:
        fusionScore > 0.8
          ? "text-threat-red"
          : fusionScore > 0.5
            ? "text-threat-amber"
            : "text-intel-blue",
      spark: true,
    },
    {
      label: "Confidence",
      value: `${confidence}%`,
      sub: confidence > 80 ? "High" : confidence > 50 ? "Medium" : "Low",
      icon: Shield,
      color: "text-operational-green",
      spark: false,
    },
    {
      label: "Active Alerts",
      value: activeAlerts.toString(),
      sub: "3 (34b)",
      icon: AlertCircle,
      color: "text-threat-red",
      spark: false,
    },
    {
      label: "Monitored Regions",
      value: "128",
      sub: "8 (81h)",
      icon: MapIcon,
      color: "text-intel-blue",
      spark: false,
    },
    {
      label: "Vessels Tracked",
      value: "1,247",
      sub: "56 (550)",
      icon: Ship,
      color: "text-tactical-cyan",
      spark: false,
    },
    {
      label: "Data Sources",
      value:
        providers.length > 0
          ? `${operationalProviders} / ${providers.length}`
          : "12 / 15",
      sub: "OPERATIONAL",
      icon: Database,
      color: "text-operational-green",
      spark: false,
    },
    {
      label: "System Health",
      value: "98.7%",
      sub: "OPTIMAL",
      icon: Activity,
      color: "text-operational-green",
      spark: false,
    },
  ];

  return (
    <div className="flex-1 flex flex-col p-1.5 gap-1.5 overflow-hidden bg-[#02040a]">
      {/* TOP METRICS BAR */}
      <div className="flex items-center gap-1.5 h-12 shrink-0 px-2">
         <div className="flex items-center gap-6">
            <div className="flex items-center gap-2 px-2 py-1 bg-white/5 rounded-sm">
               {(['2D', '3D', 'Globe'] as const).map(mode => (
                 <button 
                   key={mode}
                   onClick={() => setMapMode(mode)}
                   className={cn(
                     "text-[7px] uppercase font-bold px-1 transition-all",
                     mapMode === mode ? "text-intel-blue border-b border-intel-blue" : "text-white/40 hover:text-white/60"
                   )}
                 >
                   {mode}
                 </button>
               ))}
            </div>

            <div className="flex items-center gap-6 border-l border-white/10 pl-6">
               {[
                 { label: 'Fusion Score', value: (fusionScore * 100).toFixed(0), sub: threatLevel, color: 'text-intel-blue' },
                 { label: 'Confidence', value: `${confidence}%`, sub: confidence > 80 ? 'HIGH' : 'MED', color: 'text-operational-green' },
                 { label: 'Active Alerts', value: activeAlerts, sub: 'NEW', color: 'text-threat-red' },
                 { label: 'Maritime', value: vessels.length, sub: 'ASSETS', color: 'text-white' },
                 { label: 'Providers', value: `${operationalProviders}/${providers.length}`, color: 'text-white' },
                 { label: 'Status', value: apiHealthy ? 'STABLE' : 'ERROR', color: apiHealthy ? 'text-operational-green' : 'text-threat-red' },
               ].map(m => (
                 <div key={m.label} className="flex flex-col">
                    <span className="text-[5px] text-white/30 uppercase tracking-widest font-bold">{m.label}</span>
                    <div className="flex items-baseline gap-1.5">
                       <span className={cn("text-[11px] font-bold tracking-tighter", m.color)}>{m.value}</span>
                       <span className={cn("text-[6px] font-bold uppercase", m.color)}>{m.sub}</span>
                    </div>
                 </div>
               ))}
            </div>
         </div>
         
         <div className="flex-1 flex justify-end gap-2">
            <div className="h-6 w-24 bg-white/5 rounded-sm relative overflow-hidden">
               <svg className="absolute inset-0 h-full w-full" viewBox="0 0 100 24">
                  <path d="M0 20 L20 15 L40 18 L60 10 L80 12 L100 8" fill="none" stroke="#00D4FF" strokeWidth="1" opacity="0.3" />
               </svg>
            </div>
         </div>
      </div>

      {/* MAIN SECTION */}
      <div className="flex-1 grid grid-cols-12 gap-1.5 overflow-hidden">
        {/* LEFT: MAP */}
        <div className="col-span-9 relative bg-[#02040a] border border-white/5 rounded-sm overflow-hidden shadow-2xl">
          <TacticalMap />
          
          {/* MAP OVERLAYS */}
          <div className="absolute top-4 left-4 z-30 pointer-events-none flex flex-col gap-2">
             <div className="bg-black/60 backdrop-blur-md px-3 py-2 border border-white/10 rounded-sm w-32">
                <div className="text-[6px] text-white/40 uppercase font-bold mb-2">Layers</div>
                <div className="space-y-1.5">
                   {['Terrain', 'Borders', 'Satellite'].map(l => (
                     <div key={l} className="flex items-center gap-2">
                        <div className="h-2 w-2 border border-white/20 rounded-[1px]" />
                        <span className="text-[6px] text-white/60 font-bold uppercase">{l}</span>
                     </div>
                   ))}
                   {[
                     { l: 'Threat Zones', c: 'bg-threat-red' },
                     { l: 'Vessels', c: 'bg-intel-blue' },
                     { l: 'Air Corridors', c: 'bg-tactical-cyan' },
                     { l: 'Satellites', c: 'bg-purple-500' },
                     { l: 'Weather', c: 'bg-operational-green' },
                     { l: 'Radar Sweep', c: 'bg-white/40' },
                   ].map(l => (
                     <div key={l.l} className="flex items-center gap-2">
                        <div className={cn("h-2 w-2 rounded-[1px]", l.c)} />
                        <span className="text-[6px] text-white font-bold uppercase">{l.l}</span>
                     </div>
                   ))}
                </div>
             </div>
          </div>

          <div className="absolute top-4 right-4 z-30 pointer-events-none flex flex-col items-end gap-1">
             <span className="text-[8px] font-mono text-white/60 font-bold">Lat: 28.6139° N</span>
             <span className="text-[8px] font-mono text-white/60 font-bold">Lng: 77.2090° E</span>
             <span className="text-[8px] font-mono text-white/60 font-bold">Alt: 3420 km</span>
          </div>

          <div className="absolute bottom-4 right-4 z-30 flex flex-col gap-1">
             {[Plus, Minus, Maximize2, Target, Settings].map((Icon, i) => (
               <button key={i} className="p-1.5 bg-black/60 border border-white/10 rounded-sm hover:bg-white/10 transition-colors">
                  <Icon className="h-3 w-3 text-white/60" />
               </button>
             ))}
          </div>
        </div>

        {/* RIGHT: ANALYTICS COLUMN */}
        <div className="col-span-3 flex flex-col gap-1.5 overflow-hidden">
          <div className="flex-[1.2] overflow-hidden">
            <LiveIntelligenceFeed />
          </div>
          <div className="flex-1 overflow-hidden">
            <IntelligenceFusionPanel />
          </div>
        </div>
      </div>

      {/* BOTTOM PANELS */}
      <div className="h-44 grid grid-cols-12 gap-1.5 shrink-0">
        {/* MARITIME DOMAIN */}
        <div className="col-span-2 bg-[#050816]/40 border border-white/5 rounded-sm p-2.5 flex flex-col relative overflow-hidden group">
           <div className="absolute top-0 left-0 w-full h-[1px] bg-gradient-to-r from-transparent via-intel-blue/40 to-transparent" />
           <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-1.5">
                 <Ship className="h-3 w-3 text-intel-blue" />
                 <span className="text-[8px] font-bold uppercase tracking-widest text-intel-blue">Maritime</span>
              </div>
              <div className="h-1 w-1 bg-operational-green rounded-full animate-pulse" />
           </div>
           <div className="flex-1 flex flex-col justify-between">
              <div>
                 <div className="text-[18px] font-bold tracking-tighter text-white">1,247</div>
                 <div className="text-[7px] text-white/40 uppercase font-bold tracking-tighter">Total Assets Tracked</div>
              </div>
              <div className="space-y-1.5 mt-2">
                 <div className="flex justify-between items-center bg-white/[0.02] p-1 rounded-sm border border-white/5">
                    <span className="text-[7px] text-white/40 font-bold uppercase">Dark Vessels</span>
                    <span className="text-[8px] text-threat-red font-bold">56</span>
                 </div>
                 <div className="flex justify-between items-center bg-white/[0.02] p-1 rounded-sm border border-white/5">
                    <span className="text-[7px] text-white/40 font-bold uppercase">Anomalies</span>
                    <span className="text-[8px] text-threat-amber font-bold">12</span>
                 </div>
              </div>
           </div>
        </div>

        {/* AIR & SPACE */}
        <div className="col-span-2 bg-[#050816]/40 border border-white/5 rounded-sm p-2.5 flex flex-col relative overflow-hidden group">
           <div className="absolute top-0 left-0 w-full h-[1px] bg-gradient-to-r from-transparent via-purple-500/40 to-transparent" />
           <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-1.5">
                 <Activity className="h-3 w-3 text-purple-400" />
                 <span className="text-[8px] font-bold uppercase tracking-widest text-purple-400">Air & Space</span>
              </div>
           </div>
           <div className="flex-1 flex flex-col justify-between">
              <div>
                 <div className="text-[18px] font-bold tracking-tighter text-white">347</div>
                 <div className="text-[7px] text-white/40 uppercase font-bold tracking-tighter">Active Air Assets</div>
              </div>
              <div className="space-y-1.5 mt-2">
                 <div className="flex justify-between items-center bg-white/[0.02] p-1 rounded-sm border border-white/5">
                    <span className="text-[7px] text-white/40 font-bold uppercase">Satellites</span>
                    <span className="text-[8px] text-purple-400 font-bold">23</span>
                 </div>
                 <div className="flex justify-between items-center bg-white/[0.02] p-1 rounded-sm border border-white/5">
                    <span className="text-[7px] text-white/40 font-bold uppercase">ISR Flights</span>
                    <span className="text-[8px] text-intel-blue font-bold">08</span>
                 </div>
              </div>
           </div>
        </div>

        {/* CYBER INTEL */}
        <div className="col-span-2 bg-[#050816]/40 border border-white/5 rounded-sm p-2.5 flex flex-col relative overflow-hidden group">
           <div className="absolute top-0 left-0 w-full h-[1px] bg-gradient-to-r from-transparent via-operational-green/40 to-transparent" />
           <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-1.5">
                 <Shield className="h-3 w-3 text-operational-green" />
                 <span className="text-[8px] font-bold uppercase tracking-widest text-operational-green">Cyber</span>
              </div>
           </div>
           <div className="flex-1 flex flex-col justify-between">
              <div>
                 <div className="text-[18px] font-bold tracking-tighter text-white">98.2</div>
                 <div className="text-[7px] text-white/40 uppercase font-bold tracking-tighter">Integrity Score</div>
              </div>
              <div className="space-y-1.5 mt-2">
                 <div className="flex justify-between items-center bg-white/[0.02] p-1 rounded-sm border border-white/5">
                    <span className="text-[7px] text-white/40 font-bold uppercase">Threats</span>
                    <span className="text-[8px] text-threat-amber font-bold">04</span>
                 </div>
                 <div className="flex justify-between items-center bg-white/[0.02] p-1 rounded-sm border border-white/5">
                    <span className="text-[7px] text-white/40 font-bold uppercase">Nodes</span>
                    <span className="text-[8px] text-white font-bold">12,4k</span>
                 </div>
              </div>
           </div>
        </div>

        {/* ENVIRONMENT */}
        <div className="col-span-2 bg-[#050816]/40 border border-white/5 rounded-sm p-2.5 flex flex-col relative overflow-hidden group">
           <div className="absolute top-0 left-0 w-full h-[1px] bg-gradient-to-r from-transparent via-threat-amber/40 to-transparent" />
           <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-1.5">
                 <Zap className="h-3 w-3 text-threat-amber" />
                 <span className="text-[8px] font-bold uppercase tracking-widest text-threat-amber">Weather</span>
              </div>
           </div>
           <div className="flex-1 flex flex-col justify-between">
              <div className="flex items-center gap-3">
                 <div className="text-[22px] font-bold tracking-tighter text-white">14°C</div>
                 <div className="h-8 w-[1px] bg-white/5" />
                 <div className="flex flex-col">
                    <span className="text-[7px] text-white/40 font-bold uppercase">Ladakh Sector</span>
                    <span className="text-[9px] text-white font-bold uppercase tracking-tighter">Partly Cloudy</span>
                 </div>
              </div>
              <div className="grid grid-cols-3 gap-1 mt-2">
                 {['WIN', 'VIS', 'HUM'].map(m => (
                   <div key={m} className="bg-white/[0.02] p-1 rounded-sm border border-white/5 flex flex-col items-center">
                      <span className="text-[6px] text-white/30 font-bold uppercase">{m}</span>
                      <span className="text-[8px] text-white font-bold">12.4</span>
                   </div>
                 ))}
              </div>
           </div>
        </div>

        {/* PROVIDER STATUS */}
        <div className="col-span-2 bg-[#050816]/40 border border-white/5 rounded-sm p-2.5 flex flex-col relative overflow-hidden group">
           <div className="absolute top-0 left-0 w-full h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent" />
           <div className="flex items-center justify-between mb-2">
              <span className="text-[8px] font-bold uppercase tracking-widest text-white/60">Providers</span>
              <span className="text-[7px] font-mono text-white/20">8/12</span>
           </div>
           <div className="flex-1 overflow-y-auto scrollbar-hide space-y-1">
              {providers.length > 0 ? (
                providers.slice(0, 5).map(p => (
                  <div key={p.provider} className="flex items-center justify-between bg-white/[0.02] px-1.5 py-1 rounded-sm border border-white/[0.03]">
                    <div className="flex items-center gap-1.5">
                        <div className={cn(
                          "h-1 w-1 rounded-full",
                          ["reachable", "operational"].includes(p.status.toLowerCase()) ? "bg-operational-green" : "bg-threat-red"
                        )} />
                        <span className="text-[7px] text-white/60 font-bold uppercase">{p.provider}</span>
                    </div>
                    <span className="text-[6px] text-white/20 font-mono uppercase">{p.status}</span>
                  </div>
                ))
              ) : (
                <div className="text-[7px] text-white/20 uppercase text-center py-4">No Providers Linked</div>
              )}
           </div>
        </div>

        {/* KNOWLEDGE GRAPH */}
        <div className="col-span-2 bg-[#050816]/40 border border-white/5 rounded-sm overflow-hidden relative">
           <KnowledgeGraph />
        </div>
      </div>
    </div>
  );
}
