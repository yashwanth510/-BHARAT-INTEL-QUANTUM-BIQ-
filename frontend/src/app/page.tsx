import { CommandHeader } from "@/components/tactical/CommandHeader";
import { MainGrid } from "@/components/tactical/MainGrid";
import { OpsSidebar } from "@/components/tactical/OpsSidebar";

export default function Home() {
  return (
    <main className="flex flex-col h-screen overflow-hidden selection:bg-intel-blue/30 bg-[#02040a] text-white font-sans antialiased">
      {/* GLOBAL COMMAND HEADER */}
      <CommandHeader />

      <div className="flex flex-1 overflow-hidden">
        {/* LEFT OPS SIDEBAR */}
        <OpsSidebar />

        {/* MAIN OPERATIONAL AREA */}
        <div className="flex-1 flex flex-col overflow-hidden relative">
          {/* SCANLINE / ATMOSPHERIC GLOBAL OVERLAY */}
          <div className="absolute inset-0 pointer-events-none z-50 opacity-[0.015] bg-[linear-gradient(rgba(18,16,16,0)_50%,rgba(0,0,0,0.25)_50%),linear-gradient(90deg,rgba(255,0,0,0.06),rgba(0,255,0,0.02),rgba(0,0,255,0.06))] bg-[length:100%_2px,3px_100%]" />
          
          {/* MAIN INTELLIGENCE GRID */}
          <MainGrid />
        </div>
      </div>

      {/* FOOTER / CLASSIFICATION BAR */}
      <footer className="h-5 bg-black border-t border-white/5 flex items-center justify-between px-4 z-50 shrink-0">
        <div className="flex items-center gap-4">
           <div className="flex items-center gap-2">
              <div className="h-1 w-1 bg-operational-green rounded-full shadow-[0_0_5px_#00FF85]" />
              <span className="text-[7px] font-bold text-white/40 uppercase tracking-widest">Network: Connected</span>
           </div>
           <span className="text-[7px] font-bold text-white/20 uppercase tracking-widest">Node: DEL-PRIMARY-01</span>
        </div>
        <div className="flex items-center gap-6">
           <span className="text-[7px] font-bold text-threat-amber uppercase tracking-[0.4em] animate-pulse">Classification: Secret // Releasable to BIQ Only</span>
        </div>
        <div className="flex items-center gap-4 text-white/20">
           <span className="text-[7px] font-bold font-mono">v2.1.0-R900</span>
           <span className="text-[7px] font-bold font-mono">UTC: {new Date().toISOString().slice(11, 19)}</span>
        </div>
      </footer>
    </main>
  );
}
