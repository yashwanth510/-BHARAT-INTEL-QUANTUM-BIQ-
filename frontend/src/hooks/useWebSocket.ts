"use client";

import { useEffect, useRef, useState, useCallback } from 'react';
import { useIntelligenceStore } from '@/lib/store/useIntelligenceStore';

export function useWebSocket(url: string) {
  const [isConnected, setIsConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const socketRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const reconnectAttemptsRef = useRef(0);
  
  const addLog = useIntelligenceStore((state) => state.addLog);
  const updateFusion = useIntelligenceStore((state) => state.updateFusion);

  const connectRef = useRef<() => void>(() => {});

  const connect = useCallback(() => {
    if (socketRef.current?.readyState === WebSocket.OPEN) return;

    try {
      const ws = new WebSocket(url);
      socketRef.current = ws;

      ws.onopen = () => {
        console.log(`[WS] Connected to ${url}`);
        setIsConnected(true);
        setError(null);
        reconnectAttemptsRef.current = 0;
      };

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          
          if (data.type === 'update') {
            addLog({
              id: Math.random().toString(36).substr(2, 9),
              timestamp: new Date().toLocaleTimeString('en-GB'),
              source: data.source.toUpperCase(),
              event: data.message.toUpperCase().replace(/\s/g, '_'),
              location: data.location || 'Global',
              severity: data.priority.toUpperCase() as any,
            });
            
            if (data.source === 'fusion') {
              // Update fusion store if needed
            }
          }
        } catch (e) {
          console.error('[WS] Parse Error:', e);
        }
      };

      ws.onclose = () => {
        setIsConnected(false);
        const backoff = Math.min(1000 * Math.pow(2, reconnectAttemptsRef.current), 30000);
        console.log(`[WS] Disconnected. Reconnecting in ${backoff}ms...`);
        
        reconnectTimeoutRef.current = setTimeout(() => {
          reconnectAttemptsRef.current += 1;
          connectRef.current();
        }, backoff);
      };

      ws.onerror = (err) => {
        console.error('[WS] Error:', err);
        setError('WebSocket Connection Failed');
      };

    } catch (e) {
      setError('Initialization Error');
    }
  }, [url, addLog, updateFusion]);

  useEffect(() => {
    connectRef.current = connect;
  }, [connect]);

  useEffect(() => {
    connect();
    return () => {
      if (reconnectTimeoutRef.current) clearTimeout(reconnectTimeoutRef.current);
      socketRef.current?.close();
    };
  }, [connect]);

  const sendMessage = useCallback((msg: any) => {
    if (socketRef.current?.readyState === WebSocket.OPEN) {
      socketRef.current.send(JSON.stringify(msg));
    }
  }, []);

  return { isConnected, error, sendMessage };
}
