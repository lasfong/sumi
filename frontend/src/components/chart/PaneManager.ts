import type { IChartApi, IPaneApi, Time } from 'lightweight-charts';
import type { PaneId } from './workspaceTypes';

export class PaneManager {
  private readonly panes = new Map<PaneId, IPaneApi<Time>>();
  private readonly chart: IChartApi;
  private desiredOrder: PaneId[] = [];
  private availableHeight = 500;

  constructor(chart: IChartApi) {
    this.chart = chart;
    this.panes.set('price', chart.panes()[0]);
  }

  get(id: PaneId): IPaneApi<Time> {
    const existing = this.panes.get(id);
    if (existing) return existing;
    const pane = this.chart.addPane(true);
    this.panes.set(id, pane);
    return pane;
  }

  index(id: PaneId): number {
    return this.get(id).paneIndex();
  }

  removeIfEmpty(id: PaneId): void {
    if (id === 'price') return;
    const pane = this.panes.get(id);
    if (!pane || pane.getSeries().length > 0) return;
    this.chart.removePane(pane.paneIndex());
    this.panes.delete(id);
    this.reconcile();
  }

  layout(ordered: PaneId[], availableHeight: number): void {
    this.desiredOrder = ordered.filter((id, index) => id !== 'price' && ordered.indexOf(id) === index);
    this.availableHeight = availableHeight;
    // Materialize preserved empty panes in document order before asynchronous data
    // completes. Series completion can then only populate an existing stable pane and
    // cannot make native pane identity/order depend on network completion order.
    this.desiredOrder.forEach(id => {
      if (this.panes.has(id)) return;
      this.panes.set(id, this.chart.addPane(true));
    });
    [...this.panes.entries()]
      .filter(([id, pane]) => id !== 'price' && !this.desiredOrder.includes(id) && pane.getSeries().length === 0)
      .sort(([, a], [, b]) => b.paneIndex() - a.paneIndex())
      .forEach(([id, pane]) => {
        this.chart.removePane(pane.paneIndex());
        this.panes.delete(id);
      });
    this.reconcile();
  }

  resize(availableHeight: number): void {
    this.availableHeight = availableHeight;
    this.reconcile();
  }

  snapshot(): Array<{ id: PaneId; index: number; height: number; stretchFactor: number }> {
    return [...this.panes.entries()].map(([id, pane]) => ({
      id, index: pane.paneIndex(), height: pane.getHeight(), stretchFactor: pane.getStretchFactor(),
    })).sort((a, b) => a.index - b.index);
  }

  private reconcile(): void {
    void this.availableHeight; // The DOM boundary enforces minimum content height; native panes use relative factors.
    const price = this.panes.get('price');
    if (!price) return;
    const nativePanes = this.chart.panes();
    const visible = this.desiredOrder.filter(id => {
      const pane = this.panes.get(id);
      return !!pane && nativePanes.includes(pane);
    });
    price.setStretchFactor(4);
    visible.forEach(id => this.panes.get(id)?.setStretchFactor(1));
    // While authoritative requests complete independently, newly created panes stay at
    // their official append position. Moving a partial set makes subsequent native pane
    // indices transient in Lightweight Charts. Reorder once the full desired set exists.
    if (visible.length !== this.desiredOrder.length) return;
    if (price.paneIndex() !== 0) price.moveTo(0);
    // Resolve current indices on every step. moveTo mutates sibling indices immediately,
    // so this remains deterministic regardless of pane creation/completion order.
    visible.forEach((id, index) => {
      const pane = this.panes.get(id);
      if (pane && index + 1 < nativePanes.length && pane.paneIndex() !== index + 1) pane.moveTo(index + 1);
    });
  }
}
