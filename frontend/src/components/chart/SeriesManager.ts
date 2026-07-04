import {
  CandlestickSeries,
  HistogramSeries,
  LineSeries,
  createSeriesMarkers,
  type IChartApi,
  type ISeriesApi,
  type SeriesMarker,
  type Time,
} from 'lightweight-charts';
import { PaneManager } from './PaneManager';
import type { CandleData, IndicatorSeriesData, PaneId, VolumeData } from './workspaceTypes';

type ManagedSeries = ISeriesApi<'Line'> | ISeriesApi<'Histogram'>;

export class SeriesManager {
  readonly candles: ISeriesApi<'Candlestick'>;
  readonly volume: ISeriesApi<'Histogram'>;
  private readonly indicatorSeries = new Map<string, { paneId: PaneId; series: ManagedSeries[] }>();
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
    this.volume = chart.addSeries(HistogramSeries, {
      priceFormat: { type: 'volume' }, priceLineVisible: false, lastValueVisible: false,
    }, panes.index('volume'));
    this.markerPlugin = createSeriesMarkers(this.candles, []);
  }

  setCandles(data: CandleData[], markers: SeriesMarker<Time>[] = []): void {
    this.candles.setData(data);
    this.markerPlugin.setMarkers(markers);
  }

  setVolume(data: VolumeData[]): void { this.volume.setData(data); }
  updateCandle(candle: CandleData, volume?: VolumeData): void {
    this.candles.update(candle);
    if (volume) this.volume.update(volume);
  }

  setIndicator(key: string, paneId: PaneId, definitions: IndicatorSeriesData[]): void {
    this.removeIndicator(key);
    const paneIndex = this.panes.index(paneId);
    const series = definitions.map(definition => {
      const options = {
        color: definition.color ?? '#2962FF', lineWidth: 2 as const,
        crosshairMarkerVisible: false, priceLineVisible: false,
        lastValueVisible: true, title: definition.name,
      };
      const created: ManagedSeries = definition.type === 'histogram'
        ? this.chart.addSeries(HistogramSeries, options, paneIndex)
        : this.chart.addSeries(LineSeries, options, paneIndex);
      created.setData(definition.data);
      return created;
    });
    this.indicatorSeries.set(key, { paneId, series });
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
}
