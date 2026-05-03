"use client";

import { useEffect, useRef, useState, useCallback } from "react";
import cytoscape from "cytoscape";
// @ts-ignore
import cola from 'cytoscape-cola';
import { Share2, Maximize2, RefreshCw, Loader } from "lucide-react";
import { api } from "@/lib/api";
import { useIntelligenceStore } from "@/lib/store/useIntelligenceStore";
import { useQuery } from "@tanstack/react-query";

cytoscape.use(cola);

interface GraphData {
  nodes: Array<{
    id: string;
    label: string;
    type: string;
    name: string;
    properties?: any;
  }>;
  edges: Array<{
    source: string;
    target: string;
    label: string;
  }>;
}

// ... existing NODE_COLORS ...
const NODE_COLORS: Record<string, string> = {
  event: "#FF3B5C",
  actor: "#FFB020",
  zone: "#00D4FF",
  provider: "#00FFC6",
  vessel: "#8B5CF6",
  financial: "#00FF85",
  synthesis: "#FF6B35",
  location: "#00D4FF",
};

export function KnowledgeGraph() {
  const containerRef = useRef<HTMLDivElement>(null);
  const cyRef = useRef<any>(null);
  const { selectedEntityId, selectEntity, setHighlightedVessels } = useIntelligenceStore();

  // 1. DYNAMIC NEO4J FETCHING (TANSTACK QUERY)
  const { data: graphData, isLoading, refetch } = useQuery({
    queryKey: ['graph', selectedEntityId],
    queryFn: async () => {
      // If we have a selected entity, fetch its neighborhood sub-graph
      // Otherwise fetch the global summary graph
      const response = selectedEntityId 
        ? await api.intelligence(selectedEntityId) // Use intelligence endpoint for context-aware graph
        : await api.graphData();
      
      // Transform response to Cytoscape elements
      return response;
    },
    enabled: true,
  });

  // ... (Transform logic)
  const buildElements = useCallback((data: any) => {
    if (!data) return FALLBACK_ELEMENTS;
    
    // Support both direct GraphData and UnifiedIntelligenceResponse
    const nodes = data.nodes || [];
    const edges = data.edges || [];

    return [
      ...nodes.map((n: any) => ({
        data: {
          id: n.id,
          label: n.label || n.type,
          type: n.type || "event",
          name: n.name || n.id,
        },
      })),
      ...edges.map((e: any) => ({
        data: { source: e.source, target: e.target, label: e.label },
      })),
    ];
  }, []);

  // 2. ADVANCED CYTOSCAPE LAYOUT & FOCUS
  const initGraph = useCallback((elements: any) => {
    if (!containerRef.current) return;
    if (cyRef.current) cyRef.current.destroy();

    const cy = cytoscape({
      container: containerRef.current,
      elements,
      style: [
        {
          selector: "node",
          style: {
            label: "data(name)",
            color: "#ffffff",
            "font-size": "8px",
            "text-valign": "bottom",
            "background-color": "#050816",
            "border-width": 2,
            "border-color": (ele: any) => NODE_COLORS[ele.data("type")] || "#00D4FF",
            width: 20,
            height: 20,
          },
        },
        {
          selector: "edge",
          style: {
            width: 1,
            "line-color": "rgba(255,255,255,0.1)",
            "target-arrow-shape": "triangle",
            "curve-style": "bezier",
          },
        },
        {
          selector: ".dimmed",
          style: {
            opacity: 0.1,
            "events": "no"
          }
        },
        {
          selector: ".highlighted",
          style: {
            "border-width": 4,
            "border-color": "#00D4FF",
            "width": 30,
            "height": 30,
            "z-index": 999
          }
        }
      ],
      layout: {
        name: "cola",
        animate: true,
        maxSimulationTime: 2000,
        nodeSpacing: 40,
        edgeLength: 100,
      } as any,
    });

    // 3. INTELLIGENT FOCUS & MAP SYNC
    cy.on('tap', 'node', (evt: any) => {
      const node = evt.target;
      const id = node.id();
      
      // Dim everything
      cy.elements().addClass('dimmed');
      
      // Highlight neighborhood
      node.removeClass('dimmed').addClass('highlighted');
      node.neighborhood().removeClass('dimmed');
      
      // Sync to Zustand
      selectEntity(id);

      // If it's an actor node, find associated vessels to highlight on map
      if (node.data('type') === 'actor') {
        const vesselIds = node.neighborhood('node[type="vessel"]').map((n: any) => n.id());
        setHighlightedVessels(vesselIds);
      }
    });

    cyRef.current = cy;
  }, [selectEntity, setHighlightedVessels]);

  useEffect(() => {
    if (graphData) {
      initGraph(buildElements(graphData));
    }
  }, [graphData, initGraph, buildElements]);

  // Handle Map -> Graph Sync
  useEffect(() => {
    if (!cyRef.current || !selectedEntityId) return;
    const node = cyRef.current.$id(selectedEntityId);
    if (node.length) {
      cyRef.current.elements().addClass('dimmed');
      node.removeClass('dimmed').addClass('highlighted');
      node.neighborhood().removeClass('dimmed');
      cyRef.current.animate({
        center: { eles: node },
        zoom: 2,
        duration: 1000
      });
    }
  }, [selectedEntityId]);

  return (
    <div className="bg-[#111827]/30 border border-white/5 rounded p-3 flex flex-col gap-1 h-full overflow-hidden group">
      <div className="flex items-center justify-between shrink-0 mb-1">
        <div className="flex items-center gap-2">
          <Share2 className="h-3 w-3 text-intel-blue" />
          <span className="text-[10px] font-bold text-white uppercase tracking-widest">
            Relational Intelligence Mesh
          </span>
        </div>
        {isLoading && <Loader className="h-3 w-3 text-intel-blue animate-spin" />}
      </div>

      <div className="flex-1 relative min-h-0">
        <div ref={containerRef} className="absolute inset-0" />
      </div>
    </div>
  );
}

const FALLBACK_ELEMENTS = [
  // ... existing fallback ...
  { data: { id: "e1", label: "ThreatEvent", type: "event", name: "LADAKH-2025" } },
  { data: { id: "a1", label: "Actor", type: "actor", name: "PLA Unit 702" } },
  { data: { id: "z1", label: "Zone", type: "zone", name: "Galwan Valley" } },
  { data: { id: "p1", label: "Provider", type: "provider", name: "Sentinel-2" } },
  { data: { id: "v1", label: "Vessel", type: "vessel", name: "Dark Cargo" } },
  { data: { id: "w1", label: "Financial", type: "financial", name: "OFAC Match" } },
  { data: { id: "s1", label: "Synthesis", type: "synthesis", name: "Correlation-001" } },
  { data: { source: "e1", target: "a1", label: "INVOLVES" } },
  { data: { source: "e1", target: "z1", label: "LOCATED_AT" } },
  { data: { source: "e1", target: "p1", label: "DETECTED_BY" } },
  { data: { source: "a1", target: "z1", label: "OPERATES_IN" } },
  { data: { source: "v1", target: "z1", label: "TRANSIT_NEAR" } },
  { data: { source: "w1", target: "e1", label: "FUNDING_LINK" } },
  { data: { source: "s1", target: "e1", label: "SYNTHESIZES" } },
  { data: { source: "s1", target: "a1", label: "IDENTIFIES" } },
];
