import {
  CandlestickSeries,
  HistogramSeries,
  LineStyle,
  LineSeries,
  createSeriesMarkers,
  type IChartApi,
  type ISeriesApi,
  type SeriesMarker,
  type Time,
} from 'lightweight-charts';
import { PaneManager } from './PaneManager';
import type { CandleData, IndicatorChartSnapshot, IndicatorSeriesData, PaneId, VolumeData } from './workspaceTypes';

type ManagedSeries = ISeriesApi<'Line'> | ISeriesApi<'Histogram'>;

export class SeriesManager {
  readonly candles: ISeriesApi<'Candlestick'>;
  private candleData: CandleData[] = [];
  private volumeData: VolumeData[] = [];
  private readonly indicatorSeries = new Map<string, { paneId: PaneId; definitions: IndicatorSeriesData[]; series: ManagedSeries[] }>();
  private readonly markerPlugin;
  private readonly chart: IChartApi;
  private readonly panes: PaneManager;

  constructor(chart: IChartApi, panes: PaneManager) {
    this.chart = chart;
    this.panes = panes;
    this.candles = chart.addSeries(CandlestickSeries, {
      upColor: '#00E676', downColor: '#FF1744', borderVisible: false,
      wickUpColor: '#00E676', wickDownColor: '#FF1744',
    }, panes.index('price'));
    this.markerPlugin = createSeriesMarkers(this.candles, []);
  }

  setCandles(data: CandleData[], markers: SeriesMarker<Time>[] = []): void {
    this.candleData = data;
    this.candles.setData(data);
    this.markerPlugin.setMarkers(markers);
  }

  setVolume(data: VolumeData[]): void {
    this.volumeData = data;
    this.indicatorSeries.forEach(managed => {
      if (managed.paneId === 'volume') managed.series.forEach(series => series.setData(data));
    });
  }
  updateCandle(candle: CandleData, volume?: VolumeData): void {
    const index = this.candleData.findIndex(item => item.time === candle.time);
    this.candleData = index < 0 ? [...this.candleData, candle] : this.candleData.map((item, itemIndex) => itemIndex === index ? candle : item);
    this.candles.update(candle);
    if (volume) this.indicatorSeries.forEach(managed => {
      if (managed.paneId === 'volume') managed.series.forEach(series => series.update(volume));
    });
  }

  setIndicator(key: string, paneId: PaneId, definitions: IndicatorSeriesData[], paneOrder?: PaneId[], height = 500): void {
    const previous = this.indicatorSeries.get(key);
    const staged: ManagedSeries[] = [];
    let stage = 'resolve-pane';
    try {
      const paneIndex = this.panes.index(paneId);
      definitions.forEach(definition => {
        stage = `add-series:${definition.seriesKey}:pane-${paneIndex}`;
        const options = {
          color: definition.color ?? '#2962FF', lineWidth: 2 as const,
          crosshairMarkerVisible: false, priceLineVisible: false, lastValueVisible: paneId === 'price', title: definition.name,
          ...(definition.scale ? { autoscaleInfoProvider: () => ({ priceRange: { minValue: definition.scale!.minimum, maxValue: definition.scale!.maximum } }) } : {}),
        };
        const created: ManagedSeries = definition.type === 'histogram'
          ? this.chart.addSeries(HistogramSeries, options, paneIndex)
          : this.chart.addSeries(LineSeries, options, paneIndex);
        staged.push(created);
        stage = `set-data:${definition.seriesKey}`;
        created.setData((paneId === 'volume' ? this.volumeData : definition.data) as never);
        stage = `references:${definition.seriesKey}`;
        definition.references?.forEach(reference => created.createPriceLine({
          price: reference.value, title: reference.label, color: reference.color ?? 'rgba(255,255,255,.35)',
          lineWidth: 1, lineStyle: LineStyle.Dashed, axisLabelVisible: false,
        }));
      });
      stage = 'layout';
      if (paneOrder) this.panes.layout(paneOrder, height);
      stage = 'commit';
      this.indicatorSeries.set(key, { paneId, definitions, series: staged });
      previous?.series.forEach(series => this.chart.removeSeries(series));
      if (previous && previous.paneId !== paneId) this.panes.removeIfEmpty(previous.paneId);
    } catch (error) {
      staged.forEach(series => {
        try { this.chart.removeSeries(series); } catch { /* retain the originating error */ }
      });
      if (!previous || previous.paneId !== paneId) {
        try { this.panes.removeIfEmpty(paneId); } catch { /* retain the originating error */ }
      }
      const detail = error instanceof Error ? error.message : String(error);
      throw new Error(`${stage}: ${detail}`, { cause: error });
    }
  }

  removeIndicator(key: string): void {
    const managed = this.indicatorSeries.get(key);
    if (!managed) return;
    managed.series.forEach(series => this.chart.removeSeries(series));
    this.indicatorSeries.delete(key);
    this.panes.removeIfEmpty(managed.paneId);
  }

  clearIndicators(): void {
    [...this.indicatorSeries.keys()].forEach(key => this.removeIndicator(key));
  }

  layout(paneIds: PaneId[], height: number): void { this.panes.layout(paneIds, height); }
  resizeLayout(height: number): void { this.panes.resize(height); }

  snapshot(): IndicatorChartSnapshot {
    const seriesCounts = new Map<PaneId, number>([['price', 1]]);
    this.indicatorSeries.forEach(managed => seriesCounts.set(
      managed.paneId, (seriesCounts.get(managed.paneId) ?? 0) + managed.definitions.length,
    ));
    return {
      keys: [...this.indicatorSeries.keys()],
      candleInput: { count: this.candleData.length, maxDate: String(this.candleData.at(-1)?.time ?? '').slice(0, 10) || null },
      panes: this.panes.snapshot().map(pane => ({ ...pane, seriesCount: seriesCounts.get(pane.id) ?? 0 })),
      instances: [...this.indicatorSeries.entries()].map(([id, managed]) => ({
        id, paneId: managed.paneId, series: managed.definitions.map(item => item.seriesKey),
        seriesMaxDates: Object.fromEntries(managed.definitions.map(item => [item.seriesKey, String(item.data.at(-1)?.time ?? '').slice(0, 10) || null])),
        references: managed.definitions.flatMap(item => item.references?.map(reference => reference.label) ?? []),
      })),
    };
  }
}
