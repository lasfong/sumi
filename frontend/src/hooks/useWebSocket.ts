import { useEffect, useRef, useState, useCallback } from 'react';

export interface WebSocketMessage {
  type: string;
  data: any;
}

export function useWebSocket(sessionId: number | null, onMessage: (msg: WebSocketMessage) => void) {
  const ws = useRef<WebSocket | null>(null);
  const [isConnected, setIsConnected] = useState(false);

  // Use useCallback so we don't recreate the reference if not needed
  const savedOnMessage = useRef(onMessage);
  useEffect(() => {
    savedOnMessage.current = onMessage;
  }, [onMessage]);

  useEffect(() => {
    if (!sessionId) return;

    // Determine WebSocket URL based on environment
    // Assuming backend is on port 8000 for local development
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.hostname;
    const wsUrl = `${protocol}//${host}:8000/api/ws/replay/${sessionId}`;

    ws.current = new WebSocket(wsUrl);

    ws.current.onopen = () => {
      console.log('WebSocket connected');
      setIsConnected(true);
    };

    ws.current.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data);
        savedOnMessage.current(msg);
      } catch (e) {
        console.error('Failed to parse WebSocket message', e);
      }
    };

    ws.current.onclose = () => {
      console.log('WebSocket disconnected');
      setIsConnected(false);
    };

    return () => {
      if (ws.current) {
        ws.current.close();
      }
    };
  }, [sessionId]);

  const sendCommand = useCallback((action: string, payload: any = {}) => {
    if (ws.current && ws.current.readyState === WebSocket.OPEN) {
      ws.current.send(JSON.stringify({ action, ...payload }));
    }
  }, []);

  return { isConnected, sendCommand };
}
