import type { IChartApi, IPaneApi, Time } from 'lightweight-charts';
import type { PaneId } from './workspaceTypes';

export class PaneManager {
  private readonly panes = new Map<PaneId, IPaneApi<Time>>();
  private readonly chart: IChartApi;

  constructor(chart: IChartApi) {
    this.chart = chart;
    this.panes.set('price', chart.panes()[0]);
  }

  get(id: PaneId): IPaneApi<Time> {
    const existing = this.panes.get(id);
    if (existing) return existing;
    const pane = this.chart.addPane(true);
    pane.setStretchFactor(id === 'volume' ? 0.22 : 0.28);
    this.panes.set(id, pane);
    this.normalizeSizes();
    return pane;
  }

  index(id: PaneId): number {
    return this.get(id).paneIndex();
  }

  removeIfEmpty(id: PaneId): void {
    if (id === 'price' || id === 'volume') return;
    const pane = this.panes.get(id);
    if (!pane || pane.getSeries().length > 0) return;
    this.chart.removePane(pane.paneIndex());
    this.panes.delete(id);
    this.normalizeSizes();
  }

  private normalizeSizes(): void {
    this.panes.get('price')?.setStretchFactor(1);
  }
}
