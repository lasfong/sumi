import { describe, expect, it, vi } from 'vitest';
import { QueryClient } from '@tanstack/react-query';
import {
  replayQueryKeys,
  dedupeCandlesByTimestamp,
  cleanupSessionQueries,
  ReplayNavigationCoordinator,
} from '../replayQueryKeys';
import { useReplayStore } from '../../../store/replayStore';
import type { Candle } from '../../../types';

function makeCandle(timestamp: string, close: number): Candle {
  return {
    id: close,
    symbol: 'FPT',
    timeframe: '1D',
    timestamp,
    open: close - 1,
    high: close + 1,
    low: close - 2,
    close,
    volume: 100000,
    source: 'doraemon',
    adjustment_type: 'unadjusted',
  };
}

describe('B30 — Frontend Replay State, Canonical Query Keys & Navigation Race Protection', () => {
  it('constructs canonical, normalized query keys with timeframe and adjustment isolation', () => {
    expect(replayQueryKeys.session(42)).toEqual(['replay-session', 42]);
    expect(replayQueryKeys.candles(42, undefined, undefined, 'FPT')).toEqual([
      'candles',
      42,
      '1D',
      'unadjusted',
      'FPT',
    ]);
    expect(replayQueryKeys.candles(42, '1w', 'adjusted', 'vcb')).toEqual([
      'candles',
      42,
      '1W',
      'adjusted',
      'VCB',
    ]);
    expect(replayQueryKeys.practiceState(42)).toEqual([
      'practice-state',
      42,
    ]);
    expect(replayQueryKeys.journal(42)).toEqual(['journal', 42]);
    expect(
      replayQueryKeys.indicatorWorkKey({
        sessionId: 42,
        symbol: 'FPT',
        timeframe: '1D',
        adjustmentType: 'unadjusted',
        currentIndex: 15,
        definitionId: 'rsi',
        params: { b: 2, a: 1 },
      }),
    ).toBe('42:FPT:1D:unadjusted:15:rsi:[["a",1],["b",2]]');
  });

  it('isolates 1D and 1W candle caches so WebSocket/query updates on 1D never corrupt 1W', () => {
    const queryClient = new QueryClient();
    const key1D = replayQueryKeys.candles(7, '1D', 'RAW', 'FPT');
    const key1W = replayQueryKeys.candles(7, '1W', 'RAW', 'FPT');

    const candle1D = makeCandle('2024-01-02', 100);
    const candle1W = makeCandle('2024-01-05', 105);

    queryClient.setQueryData<Candle[]>(key1D, [candle1D]);
    queryClient.setQueryData<Candle[]>(key1W, [candle1W]);

    // Simulate WS tick arriving for 1D
    const newTick1D = makeCandle('2024-01-03', 102);
    queryClient.setQueryData<Candle[]>(key1D, (old) =>
      dedupeCandlesByTimestamp([...(old ?? []), newTick1D]),
    );

    expect(queryClient.getQueryData<Candle[]>(key1D)).toHaveLength(2);
    expect(queryClient.getQueryData<Candle[]>(key1W)).toEqual([candle1W]);
  });

  it('deduplicates candles deterministically by date key and sorts chronologically', () => {
    const c1 = makeCandle('2024-01-03T00:00:00', 103);
    const c2 = makeCandle('2024-01-01T00:00:00', 101);
    const c3Dup = makeCandle('2024-01-03', 104); // updated bar for 2024-01-03
    const c4 = makeCandle('2024-01-02', 102);

    const deduped = dedupeCandlesByTimestamp([c1, c2, c3Dup, c4]);
    expect(deduped.map((c) => c.close)).toEqual([101, 102, 104]);
  });

  it('enforces the 4-step session switch cleanup checklist (cancelQueries -> removeQueries -> closeWS -> single clearChart)', async () => {
    const queryClient = new QueryClient();
    const callOrder: string[] = [];

    const cancelSpy = vi.spyOn(queryClient, 'cancelQueries').mockImplementation(async () => {
      callOrder.push('cancelQueries');
    });
    const removeSpy = vi.spyOn(queryClient, 'removeQueries').mockImplementation(() => {
      callOrder.push('removeQueries');
    });
    const closeWs = vi.fn(() => {
      callOrder.push('closeWS');
    });
    const clearChart = vi.fn(() => {
      callOrder.push('clearChart');
    });

    await cleanupSessionQueries(queryClient, 99, { closeWs, clearChart });

    expect(cancelSpy).toHaveBeenCalledTimes(1);
    expect(removeSpy).toHaveBeenCalledTimes(1);
    expect(closeWs).toHaveBeenCalledTimes(1);
    expect(clearChart).toHaveBeenCalledTimes(1);
    expect(callOrder).toEqual(['cancelQueries', 'removeQueries', 'closeWS', 'clearChart']);
  });

  it('protects against rapid step-x20 out-of-order network race so only the latest step wins', async () => {
    const coordinator = new ReplayNavigationCoordinator();
    const queryClient = new QueryClient();
    const sessionId = 10;
    const candleKey = replayQueryKeys.candles(sessionId, '1D', 'RAW', 'FPT');

    queryClient.setQueryData<Candle[]>(candleKey, [makeCandle('2024-01-01', 100)]);

    let authoritativeCurrentIndex = 0;
    const appliedSteps: number[] = [];

    // Simulate 20 rapid steps where early steps resolve MUCH slower than later steps (out-of-order race)
    const tasks = Array.from({ length: 20 }, (_, i) => {
      const stepNumber = i + 1;
      const delayMs = 25 - i; // Step 1 takes 25ms, Step 20 takes 6ms -> completely inverted arrival order!

      return coordinator.run(async (signal) => {
        await new Promise((resolve) => setTimeout(resolve, delayMs));
        return {
          stepNumber,
          aborted: signal.aborted,
          candle: makeCandle(`2024-01-${String(stepNumber + 1).padStart(2, '0')}`, 100 + stepNumber),
        };
      }).then((outcome) => {
        if (!outcome.stale && outcome.data) {
          authoritativeCurrentIndex = outcome.data.stepNumber;
          appliedSteps.push(outcome.data.stepNumber);
          queryClient.setQueryData<Candle[]>(candleKey, (old) =>
            dedupeCandlesByTimestamp([...(old ?? []), outcome.data!.candle]),
          );
        }
        return outcome;
      });
    });

    const outcomes = await Promise.all(tasks);

    // Every step 1..19 must be marked stale; only step 20 is non-stale
    const nonStale = outcomes.filter((o) => !o.stale);
    expect(nonStale).toHaveLength(1);
    expect(nonStale[0].data?.stepNumber).toBe(20);
    expect(authoritativeCurrentIndex).toBe(20);
    expect(appliedSteps).toEqual([20]);

    const finalCandles = queryClient.getQueryData<Candle[]>(candleKey) ?? [];
    expect(finalCandles).toHaveLength(2);
    expect(finalCandles.at(-1)?.close).toBe(120);
  });

  it('reduces Zustand useReplayStore strictly to sessionId authority (no duplicate candles/positions/trades)', () => {
    const state = useReplayStore.getState() as unknown as Record<string, unknown>;
    expect(Object.keys(state).sort()).toEqual(['clearSession', 'sessionId', 'setSession']);
    expect('candles' in state).toBe(false);
    expect('currentCandleIndex' in state).toBe(false);
    expect('positions' in state).toBe(false);
    expect('trades' in state).toBe(false);

    useReplayStore.getState().setSession(55);
    expect(useReplayStore.getState().sessionId).toBe(55);
    useReplayStore.getState().clearSession();
    expect(useReplayStore.getState().sessionId).toBeNull();
  });
});
