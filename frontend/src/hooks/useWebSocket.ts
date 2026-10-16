import { useEffect, useRef, useState, useCallback } from 'react';

import type { ReplaySourceContext } from '../types';

export type WebSocketStatus =
  | 'DISCONNECTED'
  | 'CONNECTING'
  | 'OPEN'
  | 'RECONNECTING'
  | 'OFFLINE';

export const RECONNECT_BACKOFF_MS = [500, 1000, 2000, 4000, 8000] as const;

export interface WebSocketMessage {
  type: string;
  seq?: number;
  session_id?: number;
  current_index?: number;
  playback_state?: string;
  speed_ms?: number;
  data?: Record<string, unknown>;
  payload?: Record<string, unknown>;
  source_context?: ReplaySourceContext;
}

export interface UseWebSocketOptions {
  heartbeatIntervalMs?: number;
  autoReconnect?: boolean;
  onSendWhenClosed?: (action: string, payload: Record<string, unknown>) => void;
  onReconnected?: () => void;
}

const QUEUEABLE_ACTIONS = new Set(['start', 'play', 'pause', 'set_speed', 'sync', 'resume']);

export function useWebSocket(
  sessionId: number | null,
  onMessage: (msg: WebSocketMessage) => void,
  options?: UseWebSocketOptions,
) {
  const ws = useRef<WebSocket | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const [status, setStatus] = useState<WebSocketStatus>('DISCONNECTED');
  const [reconnectAttempt, setReconnectAttempt] = useState(0);

  const savedOnMessage = useRef(onMessage);
  const savedOptions = useRef(options);
  useEffect(() => {
    savedOnMessage.current = onMessage;
    savedOptions.current = options;
  }, [onMessage, options]);

  const activeSessionIdRef = useRef<number | null>(sessionId);
  const intentionalCloseRef = useRef(false);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const heartbeatTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const attemptRef = useRef(0);
  const lastSeqRef = useRef(0);
  const pendingCommandsRef = useRef<Array<{ action: string; payload: Record<string, unknown> }>>([]);
  const wasConnectedOnceRef = useRef(false);

  const clearTimers = useCallback(() => {
    if (reconnectTimerRef.current !== null) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }
    if (heartbeatTimerRef.current !== null) {
      clearInterval(heartbeatTimerRef.current);
      heartbeatTimerRef.current = null;
    }
  }, []);

  const closeSocket = useCallback(() => {
    intentionalCloseRef.current = true;
    clearTimers();
    pendingCommandsRef.current = [];
    attemptRef.current = 0;
    lastSeqRef.current = 0;
    setReconnectAttempt(0);
    if (ws.current) {
      const socket = ws.current;
      ws.current = null;
      socket.onopen = null;
      socket.onmessage = null;
      socket.onclose = null;
      socket.onerror = null;
      try {
        socket.close();
      } catch {
        // Ignore close errors on already-closed sockets
      }
    }
    setIsConnected(false);
    setStatus('DISCONNECTED');
  }, [clearTimers]);

  useEffect(() => {
    activeSessionIdRef.current = sessionId;
    if (!sessionId) {
      closeSocket();
      return;
    }

    intentionalCloseRef.current = false;
    attemptRef.current = 0;
    lastSeqRef.current = 0;
    wasConnectedOnceRef.current = false;
    setReconnectAttempt(0);

    const autoReconnect = savedOptions.current?.autoReconnect ?? true;
    const heartbeatMs = savedOptions.current?.heartbeatIntervalMs ?? 15_000;

    const connectSocket = (isReconnect: boolean) => {
      if (intentionalCloseRef.current || activeSessionIdRef.current !== sessionId) {
        return;
      }
      if (typeof navigator !== 'undefined' && navigator.onLine === false) {
        setIsConnected(false);
        setStatus('OFFLINE');
        return;
      }

      clearTimers();
      setStatus(isReconnect ? 'RECONNECTING' : 'CONNECTING');

      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const wsUrl = `${protocol}//${window.location.host}/api/ws/replay/${sessionId}`;
      const socket = new WebSocket(wsUrl);
      ws.current = socket;

      socket.onopen = () => {
        if (intentionalCloseRef.current || activeSessionIdRef.current !== sessionId || ws.current !== socket) {
          try {
            socket.close();
          } catch {
            // ignore
          }
          return;
        }
        const reconnected = wasConnectedOnceRef.current || isReconnect;
        wasConnectedOnceRef.current = true;
        attemptRef.current = 0;
        lastSeqRef.current = 0;
        setReconnectAttempt(0);
        setIsConnected(true);
        setStatus('OPEN');

        // Start heartbeat ping if enabled
        if (heartbeatMs > 0) {
          heartbeatTimerRef.current = setInterval(() => {
            if (ws.current === socket && socket.readyState === WebSocket.OPEN) {
              socket.send(JSON.stringify({ action: 'ping', ts: Date.now() }));
            }
          }, heartbeatMs);
        }

        // On reconnect, request authoritative state sync first (WS-01, WS-02)
        if (reconnected && socket.readyState === WebSocket.OPEN) {
          socket.send(JSON.stringify({ action: 'sync' }));
          savedOptions.current?.onReconnected?.();
        }

        // Flush any queued commands sent while socket was connecting/reconnecting (WS-08)
        if (pendingCommandsRef.current.length > 0 && socket.readyState === WebSocket.OPEN) {
          const queuedList = [...pendingCommandsRef.current];
          pendingCommandsRef.current = [];
          for (const queued of queuedList) {
            if (reconnected && queued.action === 'sync') {
              continue;
            }
            socket.send(JSON.stringify({ action: queued.action, ...queued.payload }));
          }
        }
      };

      socket.onmessage = (event) => {
        if (intentionalCloseRef.current || activeSessionIdRef.current !== sessionId || ws.current !== socket) {
          return;
        }
        try {
          const msg = JSON.parse(event.data) as WebSocketMessage;
          if (msg.session_id !== undefined && msg.session_id !== sessionId) {
            return;
          }
          if (typeof msg.seq === 'number') {
            if (msg.seq <= lastSeqRef.current) {
              return;
            }
            lastSeqRef.current = msg.seq;
          }
          if (msg.type === 'pong') {
            return;
          }
          savedOnMessage.current(msg);
        } catch (e) {
          console.error('Failed to parse WebSocket message', e);
        }
      };

      socket.onclose = () => {
        if (ws.current !== socket) return;
        clearTimers();
        setIsConnected(false);

        if (intentionalCloseRef.current || activeSessionIdRef.current !== sessionId) {
          setStatus('DISCONNECTED');
          return;
        }

        if (typeof navigator !== 'undefined' && navigator.onLine === false) {
          setStatus('OFFLINE');
          return;
        }

        if (!autoReconnect) {
          setStatus('DISCONNECTED');
          return;
        }

        const currentAttempt = attemptRef.current;
        const delay = RECONNECT_BACKOFF_MS[Math.min(currentAttempt, RECONNECT_BACKOFF_MS.length - 1)];
        attemptRef.current = currentAttempt + 1;
        setReconnectAttempt(attemptRef.current);
        setStatus('RECONNECTING');

        reconnectTimerRef.current = setTimeout(() => {
          connectSocket(true);
        }, delay);
      };
    };

    const handleOffline = () => {
      if (intentionalCloseRef.current || activeSessionIdRef.current !== sessionId) return;
      clearTimers();
      setIsConnected(false);
      setStatus('OFFLINE');
      if (ws.current) {
        const socket = ws.current;
        ws.current = null;
        socket.onclose = null;
        try {
          socket.close();
        } catch {
          // ignore
        }
      }
    };

    const handleOnline = () => {
      if (intentionalCloseRef.current || activeSessionIdRef.current !== sessionId) return;
      attemptRef.current = 0;
      setReconnectAttempt(0);
      connectSocket(true);
    };

    window.addEventListener('offline', handleOffline);
    window.addEventListener('online', handleOnline);

    connectSocket(false);

    return () => {
      window.removeEventListener('offline', handleOffline);
      window.removeEventListener('online', handleOnline);
      closeSocket();
    };
  }, [clearTimers, closeSocket, sessionId]);

  const sendCommand = useCallback((action: string, payload: Record<string, unknown> = {}): boolean => {
    if (ws.current && ws.current.readyState === WebSocket.OPEN) {
      ws.current.send(JSON.stringify({ action, ...payload }));
      return true;
    }
    if (QUEUEABLE_ACTIONS.has(action)) {
      const nextQueue = pendingCommandsRef.current.filter((item) => {
        if (action === 'set_speed') {
          return item.action !== 'set_speed';
        }
        if (action === 'start' || action === 'play' || action === 'pause') {
          return item.action !== 'start' && item.action !== 'play' && item.action !== 'pause';
        }
        return item.action !== action;
      });
      if (action === 'set_speed' && payload.speed !== undefined) {
        for (const item of nextQueue) {
          if (item.action === 'start' || item.action === 'play') {
            item.payload = { ...item.payload, speed: payload.speed };
          }
        }
      }
      nextQueue.push({ action, payload });
      pendingCommandsRef.current = nextQueue;
    }
    savedOptions.current?.onSendWhenClosed?.(action, payload);
    return false;
  }, []);

  return {
    isConnected,
    status,
    reconnectAttempt,
    sendCommand,
    closeSocket,
  };
}
