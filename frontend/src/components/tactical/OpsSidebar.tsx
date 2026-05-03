"use client";

import { 
  LayoutDashboard, Map, Shield, Anchor, Plane, 
  Globe, Wallet, Users, Share2, AlertTriangle, 
  FileText, Settings, Database, Activity,
  Search, Crosshair, Radar, FileBarChart, Bot
} from "lucide-react";
import { cn } from "@/lib/utils";
import { motion } from "framer-motion";

const menuItems = [
  { id: 'dashboard', icon: LayoutDashboard, label: 'Command Center', badge: null },
  { id: 'map', icon: Map, label: 'Global Map', badge: 'LIVE' },
  { id: 'threat', icon: Shield, label: 'Threat Dashboard', badge: null },
  { id: 'intel', icon: FileBarChart, label: 'Intelligence Feed', badge: null },
  { id: 'maritime', icon: Anchor, label: 'Maritime Domain', badge: '12' },
  { id: 'air', icon: Plane, label: 'Air & Space', badge: null },
  { id: 'cyber', icon: Globe, label: 'Cyber Intel', badge: null },
  { id: 'financial', icon: Wallet, label: 'Financial Intel', badge: null },
  { id: 'graph', icon: Share2, label: 'Knowledge Graph', badge: null },
  { id: 'predictive', icon: Radar, label: 'Predictive Analytics', badge: null },
  { id: 'ai-command', icon: Bot, label: 'AI Command', badge: 'NEW' },
  { id: 'ops-log', icon: FileText, label: 'Operations Log', badge: null },
  { id: 'status', icon: Activity, label: 'System Status', badge: null },
];

export function OpsSidebar() {
  return (
    <aside className="w-56 border-r border-white/5 bg-[#02040a] flex flex-col h-full shrink-0">
      <div className="flex-1 overflow-y-auto py-2 scrollbar-hide">
        <nav className="space-y-0.5 px-2">
          {menuItems.map((item) => (
            <button
              key={item.id}
              onClick={() => {
                if (item.id === 'ai-command') {
                  window.dispatchEvent(new CustomEvent('focus-ai-command'));
                }
              }}
              className={cn(
                "w-full flex items-center justify-between px-2.5 py-1.5 rounded-sm text-[10px] uppercase tracking-wider group transition-all duration-200 relative",
                item.id === 'map' 
                  ? "bg-intel-blue/10 text-intel-blue border border-intel-blue/20" 
                  : "text-white/50 hover:bg-white/[0.03] hover:text-white"
              )}
            >
              <div className="flex items-center gap-2.5">
                <item.icon className={cn(
                  "h-3.5 w-3.5",
                  item.id === 'map' ? "text-intel-blue" : "group-hover:text-intel-blue transition-colors"
                )} />
                <span className="font-bold">{item.label}</span>
              </div>
              
              {item.badge && (
                <span className={cn(
                  "text-[8px] px-1 py-0.5 rounded-full font-mono font-bold leading-none",
                  item.badge === 'LIVE' 
                    ? "bg-operational-green/20 text-operational-green animate-pulse" 
                    : item.badge === 'NEW'
                      ? "bg-intel-blue/20 text-intel-blue"
                      : "bg-threat-red/20 text-threat-red"
                )}>
                  {item.badge}
                </span>
              )}
              
              {item.id === 'map' && (
                <motion.div 
                  layoutId="active-indicator"
                  className="absolute left-0 w-0.5 h-4 bg-intel-blue rounded-r-full"
                />
              )}
            </button>
          ))}
        </nav>
      </div>

      <div className="p-3 border-t border-white/5 bg-black/40 m-2 rounded-sm">
        <div className="flex flex-col gap-2">
          <div className="text-[8px] text-white/40 uppercase tracking-[0.2em] font-bold">Quick Actions</div>
          <div className="space-y-1 mt-1">
             {[
               { icon: Search, label: 'New Query', action: 'focus-ai-command' },
               { icon: Crosshair, label: 'Threat Search' },
               { icon: Radar, label: 'Entity Track' },
               { icon: FileText, label: 'Generate Report' },
               { icon: Bot, label: 'AI Briefing' },
             ].map(action => (
               <button 
                key={action.label} 
                onClick={() => {
                  if (action.action) {
                    window.dispatchEvent(new CustomEvent(action.action));
                  }
                }}
                className="w-full flex items-center gap-2 px-2 py-1.5 rounded-sm hover:bg-white/5 transition-colors group"
               >
                  <action.icon className="h-3 w-3 text-white/30 group-hover:text-intel-blue" />
                  <span className="text-[9px] text-white/60 font-bold uppercase tracking-tighter">{action.label}</span>
               </button>
             ))}
          </div>
        </div>
        
        <div className="mt-4 pt-3 border-t border-white/5">
           <div className="flex items-center gap-2 px-2 py-2 bg-intel-blue/5 border border-intel-blue/10 rounded-sm">
              <div className="h-1.5 w-1.5 bg-intel-blue rounded-full shadow-[0_0_8px_#00D4FF]" />
              <span className="text-[7px] text-white/40 font-bold uppercase tracking-widest leading-tight">Data Encrypted<br/>Quantum Secure</span>
           </div>
        </div>
      </div>
    </aside>
  );
}
