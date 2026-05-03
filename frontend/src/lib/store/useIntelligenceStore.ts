import { create } from "zustand";

export interface LogEntry {
  id: string;
  timestamp: string;
  source: string;
  event: string;
  location: string;
  severity: "INFO" | "MONITORED" | "ELEVATED" | "HIGH" | "CRITICAL";
}

export interface ProviderStatus {
  provider: string;
  status: string;
  details?: string;
}

export interface MaritimeVessel {
  mmsi: string;
  vessel_name: string;
  lat: number;
  lon: number;
  status: string;
  dark: boolean;
  timestamp: string;
}

export interface FeedItem {
  id: string;
  type: "CRITICAL" | "HIGH" | "ELEVATED" | "MONITORED" | "INFO";
  title: string;
  desc: string;
  location: string;
  time: string;
  source: string;
}

interface IntelligenceState {
  // Fusion engine output
  fusionScore: number;
  threatLevel: string;
  confidence: number;
  recommendations: string[];

  // Live feed
  feedItems: FeedItem[];

  // Operations log
  logs: LogEntry[];

  // Provider health
  providers: ProviderStatus[];

  // Alerts
  activeAlerts: number;

  // Maritime
  vessels: MaritimeVessel[];

  // Connection state
  wsConnected: boolean;
  apiHealthy: boolean;

  // Metadata
  lastRefresh: string;
  mapMode: "2D" | "3D" | "Globe";

  // Selection State (Bidirectional Bridge)
  selectedEntityId: string | null;
  highlightedVesselIds: string[];
  graphFocusNodeId: string | null;

  // Actions
  updateFusion: (score: number, level: string, confidence: number, recs?: string[]) => void;
  addLog: (log: LogEntry) => void;
  prependFeedItem: (item: FeedItem) => void;
  updateProvider: (provider: string, status: string) => void;
  setProviders: (providers: ProviderStatus[]) => void;
  incrementAlerts: () => void;
  setAlerts: (count: number) => void;
  setVessels: (vessels: MaritimeVessel[]) => void;
  setWsConnected: (connected: boolean) => void;
  setApiHealthy: (healthy: boolean) => void;
  setLastRefresh: (ts: string) => void;
  setMapMode: (mode: "2D" | "3D" | "Globe") => void;
  
  // Selection Actions
  selectEntity: (id: string | null) => void;
  setHighlightedVessels: (ids: string[]) => void;
  focusGraphNode: (id: string | null) => void;
}

export const useIntelligenceStore = create<IntelligenceState>((set) => ({
  fusionScore: 0.0,
  threatLevel: "LOADING",
  confidence: 0,
  recommendations: [],
  feedItems: [],
  logs: [],
  providers: [],
  activeAlerts: 0,
  vessels: [],
  wsConnected: false,
  apiHealthy: false,
  lastRefresh: "",
  mapMode: "Globe",
  selectedEntityId: null,
  highlightedVesselIds: [],
  graphFocusNodeId: null,

  updateFusion: (score, level, confidence, recs = []) =>
    set({
      fusionScore: score,
      threatLevel: level,
      confidence,
      recommendations: recs,
    }),

  addLog: (log) =>
    set((state) => ({
      logs: [log, ...state.logs].slice(0, 200),
    })),

  prependFeedItem: (item) =>
    set((state) => ({
      feedItems: [item, ...state.feedItems].slice(0, 50),
    })),

  updateProvider: (name, status) =>
    set((state) => ({
      providers: state.providers.map((p) =>
        p.provider === name ? { ...p, status } : p
      ),
    })),

  setProviders: (providers) => set({ providers }),

  incrementAlerts: () =>
    set((state) => ({ activeAlerts: state.activeAlerts + 1 })),

  setAlerts: (count) => set({ activeAlerts: count }),

  setVessels: (vessels) => set({ vessels }),

  setWsConnected: (wsConnected) => set({ wsConnected }),

  setApiHealthy: (apiHealthy) => set({ apiHealthy }),

  setLastRefresh: (lastRefresh) => set({ lastRefresh }),

  setMapMode: (mapMode) => set({ mapMode }),

  selectEntity: (id) => set({ selectedEntityId: id }),
  setHighlightedVessels: (ids) => set({ highlightedVesselIds: ids }),
  focusGraphNode: (id) => set({ graphFocusNodeId: id }),
}));
