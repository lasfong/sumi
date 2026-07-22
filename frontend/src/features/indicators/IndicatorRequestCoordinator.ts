export type IndicatorFetcher<T> = (signal: AbortSignal) => Promise<T>;

interface SharedRequest<T> { controller: AbortController; promise: Promise<T>; consumers: Set<string> }

export class IndicatorRequestCoordinator<T> {
  private readonly inFlight = new Map<string, SharedRequest<T>>();
  private readonly generations = new Map<string, number>();

  async request(instanceId: string, workKey: string, fetcher: IndicatorFetcher<T>): Promise<{ stale: boolean; data?: T }> {
    for (const [activeKey, active] of this.inFlight) {
      if (activeKey === workKey || !active.consumers.delete(instanceId)) continue;
      if (!active.consumers.size) active.controller.abort();
    }
    const generation = (this.generations.get(instanceId) ?? 0) + 1; this.generations.set(instanceId, generation);
    let shared = this.inFlight.get(workKey);
    if (!shared) {
      const controller = new AbortController();
      const request = fetcher(controller.signal);
      const owner: SharedRequest<T> = { controller, promise: request, consumers: new Set() };
      owner.promise = request.finally(() => {
        if (this.inFlight.get(workKey) === owner) this.inFlight.delete(workKey);
      });
      shared = owner; this.inFlight.set(workKey, shared);
    }
    shared.consumers.add(instanceId);
    try {
      const data = await shared.promise;
      return this.generations.get(instanceId) === generation ? { stale: false, data } : { stale: true };
    } finally { shared.consumers.delete(instanceId); }
  }

  invalidate(instanceId: string): void {
    this.generations.set(instanceId, (this.generations.get(instanceId) ?? 0) + 1);
    for (const request of this.inFlight.values()) {
      request.consumers.delete(instanceId);
      if (!request.consumers.size) request.controller.abort();
    }
  }

  cancelAll(): void {
    this.generations.forEach((generation, id) => this.generations.set(id, generation + 1));
    this.inFlight.forEach(request => request.controller.abort()); this.inFlight.clear();
  }

  get pendingCount(): number { return this.inFlight.size; }
}
