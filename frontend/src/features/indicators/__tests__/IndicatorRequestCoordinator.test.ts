import { describe, expect, it, vi } from 'vitest';
import { IndicatorRequestCoordinator } from '../IndicatorRequestCoordinator';

const deferred = <T>() => {
  let resolve!: (value: T) => void; let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((ok, fail) => { resolve = ok; reject = fail; });
  return { promise, resolve, reject };
};

describe('IndicatorRequestCoordinator', () => {
  it('deduplicates equivalent backend work across independent instances', async () => {
    const coordinator = new IndicatorRequestCoordinator<number[]>(); const work = deferred<number[]>();
    const fetcher = vi.fn(() => work.promise);
    const first = coordinator.request('ema-a', 'session:ema:20', fetcher);
    const second = coordinator.request('ema-b', 'session:ema:20', fetcher);
    expect(fetcher).toHaveBeenCalledTimes(1); expect(coordinator.pendingCount).toBe(1);
    work.resolve([1, 2]);
    await expect(Promise.all([first, second])).resolves.toEqual([{ stale: false, data: [1, 2] }, { stale: false, data: [1, 2] }]);
  });

  it('marks an older out-of-order result stale for the same instance', async () => {
    const coordinator = new IndicatorRequestCoordinator<number>(); const old = deferred<number>(); const fresh = deferred<number>();
    const first = coordinator.request('rsi-a', 'old', () => old.promise);
    const second = coordinator.request('rsi-a', 'fresh', () => fresh.promise);
    fresh.resolve(2); old.resolve(1);
    await expect(second).resolves.toEqual({ stale: false, data: 2 });
    await expect(first).resolves.toEqual({ stale: true });
  });

  it('aborts orphaned backend work when an instance changes work key', async () => {
    const coordinator = new IndicatorRequestCoordinator<number>(); const fresh = deferred<number>();
    let oldSignal: AbortSignal | undefined;
    const first = coordinator.request('rsi-a', 'old', signal => {
      oldSignal = signal;
      return new Promise((_, reject) => signal.addEventListener('abort', () => reject(new DOMException('cancelled', 'AbortError'))));
    });
    const second = coordinator.request('rsi-a', 'fresh', () => fresh.promise);

    expect(oldSignal?.aborted).toBe(true);
    await expect(first).rejects.toMatchObject({ name: 'AbortError' });
    fresh.resolve(2);
    await expect(second).resolves.toEqual({ stale: false, data: 2 });
  });

  it('aborts orphaned work on invalidation and all work on unmount cleanup', () => {
    const coordinator = new IndicatorRequestCoordinator<number>(); const signals: AbortSignal[] = [];
    void coordinator.request('ema-a', 'a', signal => { signals.push(signal); return new Promise(() => undefined); });
    coordinator.invalidate('ema-a'); expect(signals[0].aborted).toBe(true);
    void coordinator.request('cci-a', 'b', signal => { signals.push(signal); return new Promise(() => undefined); });
    coordinator.cancelAll(); expect(signals[1].aborted).toBe(true); expect(coordinator.pendingCount).toBe(0);
  });

  it('isolates one request failure from unrelated work', async () => {
    const coordinator = new IndicatorRequestCoordinator<number>(); const healthy = deferred<number>();
    const failed = coordinator.request('ema-a', 'bad', () => Promise.reject(new Error('network')));
    const success = coordinator.request('rsi-a', 'good', () => healthy.promise);
    healthy.resolve(42);
    await expect(failed).rejects.toThrow('network');
    await expect(success).resolves.toEqual({ stale: false, data: 42 });
  });

  it('does not let a cancelled old promise delete newer work with the same key', async () => {
    const coordinator = new IndicatorRequestCoordinator<number>();
    const old = deferred<number>(); const fresh = deferred<number>();
    void coordinator.request('old-instance', 'same-key', () => old.promise).catch(() => undefined);
    coordinator.cancelAll();
    const current = coordinator.request('new-instance', 'same-key', () => fresh.promise);
    old.reject(new DOMException('cancelled', 'AbortError'));
    await Promise.resolve(); await Promise.resolve();
    expect(coordinator.pendingCount).toBe(1);
    fresh.resolve(9);
    await expect(current).resolves.toEqual({ stale: false, data: 9 });
  });
});
