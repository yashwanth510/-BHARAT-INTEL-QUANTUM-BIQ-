import { useEffect, useRef, useState } from 'react';
import type { WsMessage } from './types';
import { BASE } from './api';

export function useWebSocket(onMessage: (msg: WsMessage) => void) {
  const handlerRef = useRef(onMessage);
  handlerRef.current = onMessage;
  const [connected, setConnected] = useState(false);
  useEffect(() => {
    let stopped = false;
    let socket: WebSocket | undefined;
    let retry: ReturnType<typeof setTimeout> | undefined;
    function connect() {
      if (stopped) return;
      const url = process.env.NEXT_PUBLIC_WS_URL || `${BASE.replace(/^http/, 'ws')}/ws/live`;
      socket = new WebSocket(url);
      socket.onopen = () => { if (!stopped) setConnected(true); };
      socket.onmessage = event => {
        try { handlerRef.current(JSON.parse(event.data)); } catch { /* Ignore malformed frames. */ }
      };
      socket.onclose = () => {
        if (stopped) return;
        setConnected(false);
        retry = setTimeout(connect, 3000);
      };
      socket.onerror = () => socket?.close();
    }
    connect();
    return () => { stopped = true; clearTimeout(retry); socket?.close(); };
  }, []);
  return connected;
}
