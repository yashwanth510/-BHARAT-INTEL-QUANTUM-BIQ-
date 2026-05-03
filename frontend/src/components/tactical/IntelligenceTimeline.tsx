"use client";

import { motion } from "framer-motion";
import { Clock } from "lucide-react";

const timelineEvents = [
  { id: 'timeline-1', time: '12:31', type: 'CRITICAL', label: 'Dark vessel detected near Karachi corridor', status: 'active' },
  { id: 'timeline-2', time: '12:30', type: 'HIGH', label: 'Construction activity detected in Siachen', status: 'pending' },
  { id: 'timeline-3', time: '12:28', type: 'ELEVATED', label: 'Cross-border movement reported', status: 'processed' },
  { id: 'timeline-4', time: '12:26', type: 'MONITORED', label: 'OFAC-linked transaction detected', status: 'processed' },
];

export function IntelligenceTimeline() {
  return (
    <div className="tactical-glass rounded-lg p-6 flex flex-col gap-4 h-full min-h-[300px]">
      <div className="flex items-center justify-between shrink-0">
        <div className="flex items-center gap-2">
          <Clock className="h-4 w-4 text-intel-blue" />
          <h3 className="text-sm font-bold text-white uppercase tracking-widest">Intelligence Timeline</h3>
        </div>
        <div className="flex items-center gap-1.5 px-2 py-0.5 bg-white/5 rounded border border-white/10">
           <span className="text-[10px] text-muted-foreground uppercase font-mono">Window: 24H</span>
        </div>
      </div>

      <div className="relative flex-1 pt-4 overflow-hidden">
        {/* TIMELINE AXIS */}
        <div className="absolute top-8 left-0 w-full h-[1px] bg-white/10" />
        
        <div className="flex justify-between relative px-2">
          {[12, 16, 20, 0, 4, 8, 12].map((hour, i) => (
            <div key={`hour-${i}`} className="flex flex-col items-center gap-2">
               <div className="h-2 w-[1px] bg-white/20" />
               <span className="text-[9px] text-muted-foreground font-mono">{hour.toString().padStart(2, '0')}:00</span>
            </div>
          ))}
          
          {/* INTERACTIVE MARKERS */}
          <motion.div 
            style={{ left: '85%' }}
            className="absolute top-7 h-3 w-3 bg-threat-red rounded-full shadow-[0_0_10px_#FF3B5C] cursor-pointer z-10"
            whileHover={{ scale: 1.5 }}
          />
          <motion.div 
            style={{ left: '40%' }}
            className="absolute top-7 h-2 w-2 bg-operational-green rounded-full shadow-[0_0_10px_#00FF85] cursor-pointer z-10"
            whileHover={{ scale: 1.5 }}
          />
        </div>

        <div className="mt-12 space-y-4 overflow-y-auto pr-2 h-[calc(100%-4rem)] scrollbar-hide">
           <span className="text-[10px] text-muted-foreground uppercase tracking-widest font-bold sticky top-0 bg-transparent backdrop-blur-sm py-1">Latest Events</span>
           <div className="space-y-3">
              {timelineEvents.map((event, i) => (
                <motion.div 
                  key={event.id}
                  initial={{ opacity: 0, x: -10 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ delay: i * 0.1 }}
                  className="flex items-center gap-4 group"
                >
                  <span className="text-[10px] text-muted-foreground font-mono w-10 shrink-0">{event.time}</span>
                  <div className="flex-1 flex items-center justify-between p-2 bg-white/5 rounded border border-white/5 group-hover:border-white/10 transition-all min-w-0">
                    <span className="text-[11px] text-white/80 truncate pr-2">{event.label}</span>
                    <span className={`text-[9px] font-bold px-1.5 py-0.5 rounded shrink-0 ${
                      event.type === 'CRITICAL' ? 'bg-threat-red/20 text-threat-red' : 
                      event.type === 'HIGH' ? 'bg-threat-amber/20 text-threat-amber' : 
                      'bg-intel-blue/20 text-intel-blue'
                    }`}>
                      {event.type}
                    </span>
                  </div>
                </motion.div>
              ))}
           </div>
        </div>
      </div>
    </div>
  );
}
