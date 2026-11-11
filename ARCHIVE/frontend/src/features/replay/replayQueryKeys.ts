import type { QueryClient } from '@tanstack/react-query';
import type { Candle } from '../../types';
import { sortDateKeys, toDateKey } from '../../utils/date';

const SESSION_SCOPED_PREFIXES = new Set([
  'candles',
  'replay-session',
  'practice-state',
  'journal',
  'replay-session-validation',
  'replay-indicator',
]);

export const normalizeReplayTimeframe = (timeframe?: string | null): string => {
  if (!timeframe) return '1D';
  const trimmed = timeframe.trim();
  const upper = trimmed.toUpperCase();
  if (upper === 'D' || upper === '1D' || upper === 'DAILY') return '1D';
  if (upper === 'W' || upper === '1W' || upper === 'WEEKLY') return '1W';
  if (upper === 'M' || upper === '1M' || upper === 'MONTHLY') return '1M';
  if (upper === '1H' || upper === 'H') return '1H';
  if (trimmed === '5m' || trimmed === '15m') return trimmed;
  return trimmed;
};

export const normalizeAdjustmentType = (adjustmentType?: string | null): string => {
  if (!adjustmentType) return 'unadjusted';
  const lower = adjustmentType.trim().toLowerCase();
  if (lower === 'raw' || lower === 'unadjusted' || lower === 'none') return 'unadjusted';
  if (lower === 'split' || lower === 'all' || lower === 'adjusted') return 'adjusted';
  return lower;
};

export const normalizeSymbolKey = (symbol?: string | null): string => {
  if (!symbol || symbol === '—') return 'default';
  return symbol.trim().toUpperCase() || 'default';
};

export const replayQueryKeys = {
  validation: (sessionId: number | null) => ['replay-session-validation', sessionId] as const,
  session: (sessionId: number | null) => ['replay-session', sessionId] as const,
  candles: (
    sessionId: number | null,
    timeframe?: string | null,
    adjustmentType?: string | null,
    symbol?: string | null,
  ) =>
    [
      'candles',
      sessionId,
      normalizeReplayTimeframe(timeframe),
      normalizeAdjustmentType(adjustmentType),
      normalizeSymbolKey(symbol),
    ] as const,
  practiceState: (sessionId: number | null) => ['practice-state', sessionId] as const,
  journal: (sessionId: number | null) => ['journal', sessionId] as const,
  indicatorWorkKey: (params: {
    sessionId: number;
    symbol?: string | null;
    timeframe?: string | null;
    adjustmentType?: string | null;
    currentIndex: number;
    definitionId: string;
    params: Record<string, unknown>;
  }): string => {
    const sortedParams = JSON.stringify(
      Object.entries(params.params).sort(([a], [b]) => a.localeCompare(b)),
    );
    return [
      params.sessionId,
      normalizeSymbolKey(params.symbol),
      normalizeReplayTimeframe(params.timeframe),
      normalizeAdjustmentType(params.adjustmentType),
      params.currentIndex,
      params.definitionId,
      sortedParams,
    ].join(':');
  },
};

export const dedupeCandlesByTimestamp = (candles: Candle[]): Candle[] => {
  if (!candles.length) return [];
  const byDate = new Map<string, Candle>();
  for (const candle of candles) {
    const key = toDateKey(candle.timestamp) || candle.timestamp;
    byDate.set(key, candle);
  }
  return Array.from(byDate.entries())
    .sort(([a], [b]) => sortDateKeys(a, b))
    .map(([, candle]) => candle);
};

export const isSessionScopedQueryKey = (
  queryKey: readonly unknown[],
  targetSessionId?: number | null,
): boolean => {
  if (!Array.isArray(queryKey) || queryKey.length === 0) return false;
  const prefix = String(queryKey[0]);
  if (!SESSION_SCOPED_PREFIXES.has(prefix)) return false;
  if (targetSessionId === undefined || targetSessionId === null) return true;
  return queryKey[1] === targetSessionId;
};

export interface SessionSwitchCleanupHooks {
  abortNavigation?: () => void;
  cancelIndicators?: () => void;
  closeWebSocket?: () => void;
  closeWs?: () => void;
  clearChart?: () => void;
}

/**
 * Executes the 4-step session-switch cleanup checklist per ADR-FE-001 / GAP-FE-02:
 * 1. Cancel all in-flight queries/requests for the outgoing session.
 * 2. Remove cached queries for the outgoing session so stale data never flashes.
 * 3. Close the outgoing WebSocket connection.
 * 4. Perform a single chart clear (no duplicate store/chart clears).
 */
export const cleanupSessionQueries = (
  queryClient: QueryClient,
  outgoingSessionId?: number | null,
  hooks?: SessionSwitchCleanupHooks,
): void => {
  // 1. Cancel in-flight navigation, indicator requests, and TanStack queries
  hooks?.abortNavigation?.();
  hooks?.cancelIndicators?.();
  void queryClient.cancelQueries({
    predicate: (query) => isSessionScopedQueryKey(query.queryKey, outgoingSessionId),
  });

  // 2. Remove outgoing session queries from cache
  queryClient.removeQueries({
    predicate: (query) => isSessionScopedQueryKey(query.queryKey, outgoingSessionId),
  });

  // 3. Close outgoing WebSocket
  hooks?.closeWebSocket?.();
  hooks?.closeWs?.();

  // 4. Single chart clear
  hooks?.clearChart?.();
};

export interface NavigationTicket {
  token: number;
  sessionId: number;
  signal: AbortSignal;
}

/**
 * Monotonic navigation token and AbortController coordinator (ADR-FE-001 / GAP-FE-01).
 * Ensures rapid stepping (e.g., 20 steps), timeframe switches, or session switches
 * abort superseded in-flight requests and ignore out-of-order responses.
 */
export class ReplayNavigationCoordinator {
  private currentToken = 0;
  private activeSessionId: number | null = null;
  private activeController: AbortController | null = null;

  begin(sessionId = 1): NavigationTicket {
    if (this.activeController) {
      this.activeController.abort();
    }
    this.currentToken += 1;
    this.activeSessionId = sessionId;
    this.activeController = new AbortController();
    return {
      token: this.currentToken,
      sessionId,
      signal: this.activeController.signal,
    };
  }

  async run<T>(
    task: (signal: AbortSignal) => Promise<T>,
    sessionId = 1,
  ): Promise<{ stale: boolean; token: number; data?: T }> {
    const ticket = this.begin(sessionId);
    try {
      const data = await task(ticket.signal);
      if (!this.isCurrent(ticket.token, ticket.sessionId)) {
        return { stale: true, token: ticket.token };
      }
      return { stale: false, token: ticket.token, data };
    } catch (error) {
      if (!this.isCurrent(ticket.token, ticket.sessionId)) {
        return { stale: true, token: ticket.token };
      }
      throw error;
    }
  }

  cancelAll(nextSessionId: number | null = null): number {
    if (this.activeController) {
      this.activeController.abort();
      this.activeController = null;
    }
    this.currentToken += 1;
    this.activeSessionId = nextSessionId;
    return this.currentToken;
  }

  isCurrent(token: number, sessionId = 1): boolean {
    if (this.activeController?.signal.aborted) return false;
    return this.currentToken === token && this.activeSessionId === sessionId;
  }

  get token(): number {
    return this.currentToken;
  }
}
