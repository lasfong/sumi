import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useWebSocket, RECONNECT_BACKOFF_MS, type WebSocketMessage } from '../useWebSocket';

class MockWebSocket {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;
  static instances: MockWebSocket[] = [];

  url: string;
  readyState: number = MockWebSocket.CONNECTING;
  sent: string[] = [];
  onopen: (() => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }

  send(data: string) {
    this.sent.push(data);
  }

  close() {
    this.readyState = MockWebSocket.CLOSED;
    if (this.onclose) {
      this.onclose();
    }
  }

  simulateOpen() {
    this.readyState = MockWebSocket.OPEN;
    this.onopen?.();
  }

  simulateMessage(msg: unknown) {
    this.onmessage?.({ data: JSON.stringify(msg) });
  }

  simulateServerDrop() {
    this.readyState = MockWebSocket.CLOSED;
    this.onclose?.();
  }
}

describe('B31 — WebSocket Chaos Matrix (WS-01..WS-10)', () => {
  const OriginalWebSocket = globalThis.WebSocket;

  beforeEach(() => {
    vi.useFakeTimers();
    MockWebSocket.instances = [];
    Object.defineProperty(globalThis, 'WebSocket', {
      configurable: true,
      writable: true,
      value: MockWebSocket,
    });
    Object.defineProperty(navigator, 'onLine', {
      configurable: true,
      writable: true,
      value: true,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
    Object.defineProperty(globalThis, 'WebSocket', {
      configurable: true,
      writable: true,
      value: OriginalWebSocket,
    });
  });

  it('WS-01: backs off exponentially on server drop (no busy-loop) and sends sync upon reconnect', () => {
    const received: WebSocketMessage[] = [];
    const onReconnected = vi.fn();

    const { result } = renderHook(() =>
      useWebSocket(101, (msg) => received.push(msg), { onReconnected }),
    );

    expect(MockWebSocket.instances).toHaveLength(1);
    const ws1 = MockWebSocket.instances[0];

    act(() => {
      ws1.simulateOpen();
    });
    expect(result.current.status).toBe('OPEN');
    expect(result.current.isConnected).toBe(true);

    // Server drops connection mid-play
    act(() => {
      ws1.simulateServerDrop();
    });
    expect(result.current.status).toBe('RECONNECTING');
    expect(result.current.reconnectAttempt).toBe(1);
    // Must not busy-loop: still only 1 socket before 500ms elapses
    act(() => {
      vi.advanceTimersByTime(499);
    });
    expect(MockWebSocket.instances).toHaveLength(1);

    // At 500ms (RECONNECT_BACKOFF_MS[0]), attempt 1 fires
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(MockWebSocket.instances).toHaveLength(2);
    const ws2 = MockWebSocket.instances[1];

    // Simulate ws2 failing immediately -> schedules 1000ms backoff (RECONNECT_BACKOFF_MS[1])
    act(() => {
      ws2.simulateServerDrop();
    });
    expect(result.current.reconnectAttempt).toBe(2);

    act(() => {
      vi.advanceTimersByTime(999);
    });
    expect(MockWebSocket.instances).toHaveLength(2);

    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(MockWebSocket.instances).toHaveLength(3);
    const ws3 = MockWebSocket.instances[2];

    // ws3 succeeds -> automatically sends sync handshake!
    act(() => {
      ws3.simulateOpen();
    });
    expect(result.current.status).toBe('OPEN');
    expect(result.current.reconnectAttempt).toBe(0);
    expect(onReconnected).toHaveBeenCalledTimes(1);
    expect(ws3.sent.map((s) => JSON.parse(s))).toContainEqual({ action: 'sync' });
    expect(RECONNECT_BACKOFF_MS).toEqual([500, 1000, 2000, 4000, 8000]);
  });

  it('WS-02: transitions to OFFLINE on browser offline event and auto-reconnects + syncs on online', () => {
    const received: WebSocketMessage[] = [];
    const { result } = renderHook(() => useWebSocket(102, (msg) => received.push(msg)));

    const ws1 = MockWebSocket.instances[0];
    act(() => {
      ws1.simulateOpen();
    });
    expect(result.current.status).toBe('OPEN');

    // Browser goes offline
    act(() => {
      Object.defineProperty(navigator, 'onLine', { configurable: true, writable: true, value: false });
      window.dispatchEvent(new Event('offline'));
    });
    expect(result.current.status).toBe('OFFLINE');
    expect(result.current.isConnected).toBe(false);

    // Browser comes back online -> immediately reconnects and sends sync
    act(() => {
      Object.defineProperty(navigator, 'onLine', { configurable: true, writable: true, value: true });
      window.dispatchEvent(new Event('online'));
    });
    expect(MockWebSocket.instances).toHaveLength(2);
    const ws2 = MockWebSocket.instances[1];
    act(() => {
      ws2.simulateOpen();
    });
    expect(result.current.status).toBe('OPEN');
    expect(ws2.sent.map((s) => JSON.parse(s))).toContainEqual({ action: 'sync' });
  });

  it('WS-03, WS-04, WS-05, WS-09: enforces monotonic seq deduplication and delivers error, playback_state, and completed frames', () => {
    const received: WebSocketMessage[] = [];
    renderHook(() => useWebSocket(103, (msg) => received.push(msg)));

    const ws = MockWebSocket.instances[0];
    act(() => {
      ws.simulateOpen();
    });

    act(() => {
      ws.simulateMessage({
        type: 'error',
        seq: 1,
        session_id: 103,
        payload: { code: 'INVALID_JSON', message: 'Malformed JSON payload' },
      });
      // Duplicate seq=1 must be dropped!
      ws.simulateMessage({
        type: 'error',
        seq: 1,
        session_id: 103,
        payload: { code: 'DUPLICATE' },
      });
      // Clamped speed echo (WS-04)
      ws.simulateMessage({
        type: 'playback_state',
        seq: 3,
        session_id: 103,
        speed_ms: 100,
        payload: { speed_ms: 100, clamped: true },
      });
      // Out-of-order seq=2 arriving after seq=3 must be dropped!
      ws.simulateMessage({
        type: 'playback_state',
        seq: 2,
        session_id: 103,
        speed_ms: 4000,
      });
      // End-of-data completed frame (WS-09)
      ws.simulateMessage({
        type: 'completed',
        seq: 4,
        session_id: 103,
        current_index: 99,
        playback_state: 'COMPLETED',
        payload: { reason: 'END_OF_DATA', current_index: 99 },
      });
    });

    expect(received.map((m) => m.seq)).toEqual([1, 3, 4]);
    expect(received.map((m) => m.type)).toEqual(['error', 'playback_state', 'completed']);
  });

  it('WS-07: switching sessionId immediately closes old socket and ignores stale frames from previous session', () => {
    const received: WebSocketMessage[] = [];
    const { rerender } = renderHook(
      ({ sid }) => useWebSocket(sid, (msg) => received.push(msg)),
      { initialProps: { sid: 1 } },
    );

    const wsSession1 = MockWebSocket.instances[0];
    act(() => {
      wsSession1.simulateOpen();
    });

    // Switch to session 2
    rerender({ sid: 2 });
    expect(wsSession1.readyState).toBe(MockWebSocket.CLOSED);
    expect(MockWebSocket.instances).toHaveLength(2);
    const wsSession2 = MockWebSocket.instances[1];

    act(() => {
      wsSession2.simulateOpen();
      // Attempt to inject a frame with session_id=1 on session 2's socket
      wsSession2.simulateMessage({
        type: 'new_candle',
        seq: 1,
        session_id: 1,
        current_index: 50,
      });
      wsSession2.simulateMessage({
        type: 'new_candle',
        seq: 2,
        session_id: 2,
        current_index: 5,
      });
    });

    expect(received).toHaveLength(1);
    expect(received[0].session_id).toBe(2);
    expect(received[0].current_index).toBe(5);
  });

  it('WS-08: sendCommand while closed returns false, notifies callback, and queues control command for flush on open', () => {
    const onSendWhenClosed = vi.fn();
    const { result } = renderHook(() =>
      useWebSocket(108, () => {}, { onSendWhenClosed }),
    );

    const ws = MockWebSocket.instances[0];
    // Socket is still CONNECTING (not OPEN)
    let ok = true;
    act(() => {
      ok = result.current.sendCommand('set_speed', { speed: 250 });
    });
    expect(ok).toBe(false);
    expect(onSendWhenClosed).toHaveBeenCalledWith('set_speed', { speed: 250 });

    // Once socket opens, the queued command is automatically flushed
    act(() => {
      ws.simulateOpen();
    });
    expect(ws.sent.map((s) => JSON.parse(s))).toContainEqual({
      action: 'set_speed',
      speed: 250,
    });
  });

  it('WS-10: two independent hook instances (tabs) on the same session maintain independent seq tracking', () => {
    const tab1Messages: WebSocketMessage[] = [];
    const tab2Messages: WebSocketMessage[] = [];

    renderHook(() => useWebSocket(110, (m) => tab1Messages.push(m)));
    renderHook(() => useWebSocket(110, (m) => tab2Messages.push(m)));

    const wsTab1 = MockWebSocket.instances[0];
    const wsTab2 = MockWebSocket.instances[1];

    act(() => {
      wsTab1.simulateOpen();
      wsTab2.simulateOpen();
      wsTab1.simulateMessage({ type: 'playback_state', seq: 10, session_id: 110, current_index: 10 });
      wsTab2.simulateMessage({ type: 'playback_state', seq: 1, session_id: 110, current_index: 10 });
    });

    expect(tab1Messages).toHaveLength(1);
    expect(tab2Messages).toHaveLength(1);
    expect(tab1Messages[0].seq).toBe(10);
    expect(tab2Messages[0].seq).toBe(1);
  });

  it('WS-01 + WS-08: after termination during active playback (seq=50), new connection resets seq counter and flushes multiple queued commands without loss', () => {
    const received: WebSocketMessage[] = [];
    const { result } = renderHook(() => useWebSocket(111, (m) => received.push(m)));

    const ws1 = MockWebSocket.instances[0];
    act(() => {
      ws1.simulateOpen();
      ws1.simulateMessage({ type: 'playback_state', seq: 49, session_id: 111, current_index: 20, playback_state: 'PLAYING' });
      ws1.simulateMessage({ type: 'new_candle', seq: 50, session_id: 111, current_index: 21, playback_state: 'PLAYING' });
    });
    expect(received.map((m) => m.seq)).toEqual([49, 50]);

    // Terminate active connection ws1
    act(() => {
      ws1.simulateServerDrop();
    });
    expect(result.current.status).toBe('RECONNECTING');

    // Queue multiple valid commands while ws1 is closed
    act(() => {
      result.current.sendCommand('set_speed', { speed: 250 });
      result.current.sendCommand('start', { speed: 250 });
    });

    // Advance backoff timer to establish ws2
    act(() => {
      vi.advanceTimersByTime(500);
    });
    expect(MockWebSocket.instances).toHaveLength(2);
    const ws2 = MockWebSocket.instances[1];

    act(() => {
      ws2.simulateOpen();
      // New connection starts seq at 1; must NOT be discarded despite ws1 having reached seq=50!
      ws2.simulateMessage({ type: 'playback_state', seq: 1, session_id: 111, current_index: 22, playback_state: 'PAUSED' });
      ws2.simulateMessage({ type: 'playback_state', seq: 2, session_id: 111, current_index: 22, speed_ms: 250, playback_state: 'PAUSED', payload: { speed_ms: 250, clamped: false } });
      ws2.simulateMessage({ type: 'new_candle', seq: 3, session_id: 111, current_index: 23, speed_ms: 250, playback_state: 'PLAYING' });
    });

    expect(ws2.sent.map((s) => JSON.parse(s))).toEqual([
      { action: 'sync' },
      { action: 'set_speed', speed: 250 },
      { action: 'start', speed: 250 },
    ]);
    expect(received.map((m) => m.seq)).toEqual([49, 50, 1, 2, 3]);
    expect(received.at(-1)?.current_index).toBe(23);
  });
});
