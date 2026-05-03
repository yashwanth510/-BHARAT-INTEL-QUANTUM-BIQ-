"use client";

import { useEffect, useRef, useState, useMemo, useCallback } from "react";
import { motion } from "framer-motion";
import {
  Layers,
  Radio,
  Ship,
} from "lucide-react";
import { useIntelligenceStore } from "@/lib/store/useIntelligenceStore";
import { cn } from "@/lib/utils";
// @ts-ignore
import { MapboxOverlay } from '@deck.gl/mapbox';
// @ts-ignore
import { IconLayer, ScatterplotLayer } from '@deck.gl/layers';

// Dynamic import to avoid SSR issues with mapbox-gl
let mapboxgl: any = null;

const MAPBOX_TOKEN = process.env.NEXT_PUBLIC_MAPBOX_TOKEN ?? "";

// High-frequency buffer for vessel data (Non-reactive)
const vesselBuffer = new Map<string, any>();
let lastRedraw = 0;

// Threat zones for BIQ — real geographic coordinates
const THREAT_MARKERS = [
  {
    id: "galwan",
    lon: 79.9689,
    lat: 34.7337,
    type: "CRITICAL",
    label: "Galwan Valley",
    color: "#FF3B5C",
  },
  {
    id: "ladakh",
    lon: 77.577,
    lat: 34.1526,
    type: "HIGH",
    label: "Ladakh Sector",
    color: "#FFB020",
  },
  {
    id: "kargil",
    lon: 76.134,
    lat: 34.5553,
    type: "ELEVATED",
    label: "Kargil Sector",
    color: "#FFB020",
  },
  {
    id: "siachen",
    lon: 77.109,
    lat: 35.421,
    type: "HIGH",
    label: "Siachen Glacier",
    color: "#FF3B5C",
  },
  {
    id: "karachi",
    lon: 67.001,
    lat: 24.861,
    type: "HIGH",
    label: "Karachi Corridor",
    color: "#FFB020",
  },
  {
    id: "arabian",
    lon: 65.0,
    lat: 22.0,
    type: "ELEVATED",
    label: "Arabian Sea",
    color: "#00D4FF",
  },
];

const MAP_STYLES = {
  dark: "mapbox://styles/mapbox/dark-v11",
  satellite: "mapbox://styles/mapbox/satellite-streets-v12",
  navigation: "mapbox://styles/mapbox/navigation-night-v1",
};

type LayerKey = keyof typeof MAP_STYLES;

const TACTICAL_THEME = {
  background: "#050816",
  grid: "rgba(0, 212, 255, 0.05)",
  border: "rgba(255, 255, 255, 0.1)",
  label: "rgba(255, 255, 255, 0.6)",
  threats: {
    CRITICAL: "#FF3B5C",
    HIGH: "#FFB020",
    ELEVATED: "#FFB020",
    MONITORED: "#00D4FF",
    STABLE: "#00FF85"
  }
};

function TacticalMapFallback() {
  const { vessels, mapMode, setMapMode } = useIntelligenceStore();

  return (
    <div className="absolute inset-0 bg-[#02040a] overflow-hidden">
      {/* MAP MODE TOGGLE IN FALLBACK */}
      <div className="absolute top-4 left-4 z-50 flex items-center gap-2 px-2 py-1 bg-black/60 border border-white/10 rounded-sm">
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

      {/* PERSPECTIVE WRAPPER */}
      <div 
        className={cn(
          "absolute inset-0 transition-all duration-1000 ease-in-out flex items-center justify-center",
          mapMode === '3D' ? "perspective-[1000px] rotate-x-[45deg] scale-110" : 
          mapMode === 'Globe' ? "scale-100" : "scale-100"
        )}
      >
        {/* GLOBE SPHERE (Only visible in Globe mode) */}
        {mapMode === 'Globe' && (
          <motion.div 
            initial={{ scale: 0, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            className="absolute w-[500px] h-[500px] rounded-full border border-intel-blue/20 bg-[radial-gradient(circle_at_30%_30%,#0a1930_0%,#02040a_70%)] shadow-[0_0_100px_rgba(0,212,255,0.1)] overflow-hidden"
          >
            {/* Spinning inner detail */}
            <motion.div 
              className="absolute inset-0 opacity-20"
              animate={{ rotate: 360 }}
              transition={{ duration: 60, repeat: Infinity, ease: "linear" }}
              style={{
                backgroundImage: 'url("https://www.transparenttextures.com/patterns/carbon-fibre.png")',
              }}
            />
          </motion.div>
        )}

        {/* ATMOSPHERIC FOG & GRID */}
        <div className={cn(
          "absolute inset-0 pointer-events-none z-10",
          mapMode === 'Globe' ? "w-[500px] h-[500px] rounded-full overflow-hidden" : "w-full h-full"
        )}>
          <div className="absolute inset-0 bg-[radial-gradient(circle_at_center,transparent_0%,#02040a_90%)]" />
          <div 
            className="absolute inset-0 opacity-[0.03] pointer-events-none z-10"
            style={{
              backgroundImage: `linear-gradient(${TACTICAL_THEME.grid} 1px, transparent 1px), linear-gradient(90deg, ${TACTICAL_THEME.grid} 1px, transparent 1px)`,
              backgroundSize: mapMode === 'Globe' ? "30px 30px" : "60px 60px"
            }}
          />
        </div>

        {/* SATELLITE BASE LAYER */}
        <div 
          className={cn(
            "absolute inset-0 opacity-30 grayscale-[0.5] contrast-[1.2] transition-all duration-1000",
            mapMode === 'Globe' ? "w-[500px] h-[500px] rounded-full" : "w-full h-full"
          )}
          style={{ 
            backgroundImage: `url('/assets/satellite.png')`,
            backgroundSize: 'cover',
            backgroundPosition: 'center',
            filter: 'brightness(0.4) sepia(0.2) hue-rotate(180deg)'
          }}
        />

        {/* VECTOR OVERLAY */}
        <svg 
          className={cn(
            "absolute pointer-events-none transition-all duration-1000",
            mapMode === 'Globe' ? "w-[500px] h-[500px] z-20" : "w-full h-full z-0 opacity-20"
          )} 
          viewBox="0 0 1000 500"
        >
           <path d="M 100 100 Q 200 80 300 120 T 450 100 T 600 150 T 800 120" fill="none" stroke="rgba(255,255,255,0.2)" strokeWidth="0.5" />
           <path d="M 620 180 L 780 150 L 850 320 L 650 380 Z" fill="rgba(255, 176, 32, 0.05)" stroke="rgba(255, 176, 32, 0.2)" strokeWidth="1" strokeDasharray="2 2" />
           <path d="M 450 140 L 580 120 L 620 250 L 480 280 Z" fill="rgba(255, 59, 92, 0.05)" stroke="rgba(255, 59, 92, 0.2)" strokeWidth="1" strokeDasharray="2 2" />
        </svg>

        {/* TACTICAL NODES */}
        <svg 
          className={cn(
            "absolute pointer-events-none z-20 transition-all duration-1000",
            mapMode === 'Globe' ? "w-[500px] h-[500px]" : "w-full h-full"
          )} 
          viewBox="0 0 1000 500"
        >
           {/* Flight Arcs */}
           <motion.path 
             d={mapMode === 'Globe' ? "M 300 250 Q 500 100 700 250" : "M 280 180 Q 450 80 620 180"}
             fill="none" 
             stroke="rgba(0, 212, 255, 0.4)" 
             strokeWidth={mapMode === '3D' ? "1" : "0.5"}
             strokeDasharray="4 4"
             initial={{ pathLength: 0 }}
             animate={{ pathLength: 1 }}
             transition={{ duration: 5, repeat: Infinity }}
           />
           
           {/* Vessels */}
           {vessels.slice(0, 15).map((v, i) => {
             const x = 40 + i * 4;
             const y = 30 + (i % 5) * 8;
             return (
               <g key={v.mmsi}>
                  <circle cx={`${x}%`} cy={`${y}%`} r={mapMode === '3D' ? "2" : "1.5"} fill={v.dark ? TACTICAL_THEME.threats.CRITICAL : TACTICAL_THEME.threats.MONITORED} className="opacity-80" />
               </g>
             );
           })}
        </svg>

        {/* RADAR SWEEP (Only 2D/3D) */}
        {mapMode !== 'Globe' && (
          <motion.div
            className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[800px] h-[800px] pointer-events-none z-10"
            style={{
              background: "conic-gradient(from 0deg, rgba(0, 212, 255, 0.08), transparent 60deg)",
              borderRadius: "50%",
            }}
            animate={{ rotate: 360 }}
            transition={{ duration: 15, repeat: Infinity, ease: "linear" }}
          />
        )}
      </div>

      {/* SCANLINE / ATMOSPHERIC OVERLAY */}
      <div className="absolute inset-0 pointer-events-none z-40 opacity-[0.02] bg-[linear-gradient(rgba(18,16,16,0)_50%,rgba(0,0,0,0.25)_50%),linear-gradient(90deg,rgba(255,0,0,0.06),rgba(0,255,0,0.02),rgba(0,0,255,0.06))] bg-[length:100%_2px,3px_100%]" />
    </div>
  );
}

export function TacticalMap() {
  const containerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<any>(null);
  const deckRef = useRef<any>(null);
  const [style, setStyle] = useState<LayerKey>("dark");
  const [loaded, setLoaded] = useState(false);
  const { mapMode, selectedEntityId, selectEntity, highlightedVesselIds, setMapMode } = useIntelligenceStore();

  // 2. DECK.GL LAYER RENDERING
  const renderDeckLayers = useCallback(() => {
    const data = Array.from(vesselBuffer.values());
    
    return [
      new IconLayer({
        id: 'vessels-layer',
        data,
        getPosition: (d: any) => d.coordinates,
        getIcon: (d: any) => ({
          url: '/assets/icons/vessel-tactical.svg',
          width: 128,
          height: 128,
          anchorY: 64
        }),
        getSize: (d: any) => 32,
        getColor: (d: any) => {
          if (d.id === selectedEntityId) return [0, 212, 255, 255]; // Selected cyan
          if (highlightedVesselIds.includes(d.id)) return [255, 59, 92, 255]; // Highlighted red
          return [255, 255, 255, 200];
        },
        pickable: true,
        onClick: (info: any) => {
          if (info.object) selectEntity(info.object.id);
        },
        updateTriggers: {
          getColor: [selectedEntityId, highlightedVesselIds]
        }
      }),
      new ScatterplotLayer({
        id: 'vessel-halos',
        data: data.filter(d => highlightedVesselIds.includes(d.id)),
        getPosition: (d: any) => d.coordinates,
        getRadius: 1000,
        getFillColor: [255, 59, 92, 100],
        stroked: true,
        getLineColor: [255, 59, 92, 255],
        getLineWidth: 2
      })
    ];
  }, [selectedEntityId, highlightedVesselIds, selectEntity]);

  // 1. HIGH-FREQUENCY WEBSOCKET HANDLING (NON-REACTIVE)
  useEffect(() => {
    const ws = new WebSocket(process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:8000/ws/stream/global');
    
    ws.onmessage = (event) => {
      try {
        if (event.data === 'biq-ping') {
          ws.send('pong');
          return;
        }
        const data = JSON.parse(event.data);
        if (data.type === 'vessel_update' || data.source === 'maritime' || data.mmsi) {
          if (data.mmsi && data.lat && data.lon) {
            vesselBuffer.set(data.mmsi, {
              id: data.mmsi,
              coordinates: [data.lon, data.lat],
              timestamp: Date.now(),
              ...data
            });
          }

          // Throttle redraws to 60fps
          const now = Date.now();
          if (now - lastRedraw > 16 && deckRef.current) {
            deckRef.current.setProps({
              layers: renderDeckLayers()
            });
            lastRedraw = now;
          }
        }
      } catch (e) {
        // console.error("WS Parse Error", e);
      }
    };

    return () => ws.close();
  }, [loaded, renderDeckLayers]);

  // Initialize Map
  useEffect(() => {
    if (!containerRef.current || mapRef.current || !MAPBOX_TOKEN) return;

    import("mapbox-gl").then((mb) => {
      const mbgl = mb.default || mb;
      if (!mbgl) return;

      mbgl.accessToken = MAPBOX_TOKEN;
      const map = new mbgl.Map({
        container: containerRef.current!,
        style: MAP_STYLES[style],
        center: [77.577, 34.1526], // Ladakh
        zoom: mapMode === 'Globe' ? 1.5 : 3.5,
        pitch: mapMode === '2D' ? 0 : 60,
        projection: mapMode === 'Globe' ? 'globe' : 'mercator',
        antialias: true,
      });

      // Initialize deck.gl overlay
      const deckOverlay = new MapboxOverlay({
        interleaved: true,
        layers: []
      });
      map.addControl(deckOverlay);
      deckRef.current = deckOverlay;

      map.on("load", () => {
        setLoaded(true);
        // ... (terrain, sky, fog setup)
      });

      mapRef.current = map;
    });

    return () => {
      if (mapRef.current) {
        mapRef.current.remove();
        mapRef.current = null;
      }
    };
  }, []);

  // Handle Graph -> Map Sync (Focus/FlyTo)
  useEffect(() => {
    if (!mapRef.current || !selectedEntityId || !loaded) return;
    
    const vessel = vesselBuffer.get(selectedEntityId);
    if (vessel) {
      mapRef.current.flyTo({
        center: vessel.coordinates,
        zoom: 12,
        duration: 3000,
        essential: true
      });
    }
  }, [selectedEntityId, loaded]);

  // ... (Update Map Mode & Style effects)
  // Update Map Mode without re-initializing
  useEffect(() => {
    const map = mapRef.current;
    if (!map || !loaded) return;

    if (mapMode === 'Globe') {
      map.setProjection('globe');
      map.easeTo({ zoom: 1.5, pitch: 0, duration: 2000 });
    } else if (mapMode === '3D') {
      map.setProjection('mercator');
      map.easeTo({ zoom: 3.5, pitch: 60, duration: 2000 });
    } else {
      map.setProjection('mercator');
      map.easeTo({ zoom: 3.5, pitch: 0, duration: 2000 });
    }
  }, [mapMode, loaded]);

  // Update Style
  useEffect(() => {
    const map = mapRef.current;
    if (!map || !loaded) return;
    map.setStyle(MAP_STYLES[style]);
  }, [style, loaded]);

  if (!MAPBOX_TOKEN) {
    return <TacticalMapFallback />;
  }

  return (
    <div className="absolute inset-0 bg-[#050816]">
      <div ref={containerRef} className="w-full h-full" />
      <div className="absolute top-4 right-4 flex flex-col gap-2 z-10">
        <button 
          onClick={() => setStyle(style === 'dark' ? 'satellite' : 'dark')}
          className="p-2 bg-black/60 backdrop-blur-md border border-white/10 rounded-sm hover:bg-black/80 transition-colors"
        >
          <Layers className="h-4 w-4 text-white/70" />
        </button>
      </div>

      {!loaded && (
        <div className="absolute inset-0 flex items-center justify-center bg-[#050816] z-20">
          <div className="flex flex-col items-center gap-3">
            <Radio className="h-6 w-6 text-intel-blue animate-pulse" />
            <span className="text-[10px] text-muted-foreground font-mono uppercase tracking-widest">
              Initializing tactical map...
            </span>
          </div>
        </div>
      )}
    </div>
  );
}
