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
  type WhitespaceData,
} from 'lightweight-charts';
import { PaneManager } from './PaneManager';
import { PositionLineManager } from './PositionLineManager';
import { IchimokuCloudPlugin } from './plugins/IchimokuCloudPlugin';
import type { CandleData, IndicatorChartSnapshot, IndicatorSeriesData, PaneId, VolumeData } from './workspaceTypes';
import { addTradingDays } from '../../features/drawings/drawingDomain';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type ManagedSeries = ISeriesApi<'Line'> | ISeriesApi<'Histogram'> | ISeriesApi<'Custom'> | any;

export class SeriesManager {
  readonly candles: ISeriesApi<'Candlestick'>;
  private readonly whitespaceSeries: ISeriesApi<'Line'>;
  private candleData: CandleData[] = [];
  private volumeData: VolumeData[] = [];
  private readonly indicatorSeries = new Map<string, { paneId: PaneId; definitions: IndicatorSeriesData[]; series: ManagedSeries[] }>();
  private readonly positionLines: PositionLineManager;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  private markerPlugin: any;
  private readonly chart: IChartApi;
  private readonly panes: PaneManager;

  constructor(chart: IChartApi, panes: PaneManager) {
    this.chart = chart;
    this.panes = panes;
    this.candles = chart.addSeries(CandlestickSeries, {
      upColor: '#00E676', downColor: '#FF1744', borderVisible: false,
      wickUpColor: '#00E676', wickDownColor: '#FF1744',
    }, panes.index('price'));
    this.whitespaceSeries = chart.addSeries(LineSeries, { visible: false, crosshairMarkerVisible: false, priceLineVisible: false, lastValueVisible: false });
    this.positionLines = new PositionLineManager(this.candles);
    this.markerPlugin = createSeriesMarkers(this.candles, []);
  }

  setCandles(data: CandleData[], markers: SeriesMarker<Time>[] = []): void {
    this.candleData = data;
    this.candles.setData(data.map(d => ({ ...d })));
    if (data.length > 0) {
      const lastCandleTime = data[data.length - 1].time as string;
      const wsData: WhitespaceData[] = [];
      for (let i = 1; i <= 100; i++) {
        wsData.push({ time: addTradingDays(lastCandleTime, i) as Time });
      }
      this.whitespaceSeries.setData(wsData);
    } else {
      this.whitespaceSeries.setData([]);
    }
    this.markerPlugin.setMarkers(markers);
  }

  setVolume(data: VolumeData[]): void {
    this.volumeData = data;
    this.indicatorSeries.forEach(managed => {
      if (managed.paneId === 'volume') {
        managed.series.forEach((series, index) => {
          if (managed.definitions[index]?.seriesKey === 'raw-volume' || managed.definitions[index]?.type === 'histogram') {
            series.setData(data.map(d => ({ ...d })));
          }
        });
      }
    });
  }
  updateCandle(candle: CandleData, volume?: VolumeData): void {
    const index = this.candleData.findIndex(item => item.time === candle.time);
    this.candleData = index < 0 ? [...this.candleData, candle] : this.candleData.map((item, itemIndex) => itemIndex === index ? candle : item);

    if (volume) {
      const volIndex = this.volumeData.findIndex(item => item.time === volume.time);
      this.volumeData = volIndex < 0 ? [...this.volumeData, volume] : this.volumeData.map((item, i) => i === volIndex ? volume : item);
    }

    // Push the whitespace series forward
    const lastCandleTime = candle.time as string;
    const wsData: WhitespaceData[] = [];
    for (let i = 1; i <= 100; i++) {
      wsData.push({ time: addTradingDays(lastCandleTime, i) as Time });
    }
    this.whitespaceSeries.setData(wsData);

    this.candles.update({ ...candle });

    if (volume) this.indicatorSeries.forEach(managed => {
      if (managed.paneId === 'volume') {
        managed.series.forEach((series, index) => {
          if (managed.definitions[index]?.seriesKey === 'raw-volume' || managed.definitions[index]?.type === 'histogram') {
            series.update({ ...volume });
          }
        });
      }
    });
  }

  setPositionLines(position: { average_price: number }, trade: { initial_stop_loss?: number | null; target_price?: number | null } | null): void {
    this.positionLines.update(position, trade);
  }

  clearPositionLines(): void {
    this.positionLines.clear();
  }

  updateIndicatorData(key: string, paneId: PaneId, definitions: IndicatorSeriesData[], paneOrder?: PaneId[], height = 500): void {
    const existing = this.indicatorSeries.get(key);
    // Can reuse if: same pane, same number of definitions, same series types
    if (existing && existing.paneId === paneId && existing.definitions.length === definitions.length &&
        existing.definitions.every((def, i) => def.type === definitions[i].type && def.seriesKey === definitions[i].seriesKey)) {
      // Incremental path: just update data on existing series handles
      existing.series.forEach((series, index) => {
        const seriesData = (paneId === 'volume' && (definitions[index].seriesKey === 'raw-volume' || definitions[index].type === 'histogram')) 
          ? this.volumeData : definitions[index].data;
        const previousData = existing.definitions[index].data;
        if (previousData && previousData.length > 0) {
            if (seriesData.length === previousData.length + 1) {
                const prevInNew = seriesData[seriesData.length - 2];
                if (previousData[previousData.length - 1].time === prevInNew.time) {
                    series.update({ ...seriesData[seriesData.length - 1] } as never);
                    return;
                }
            } else if (seriesData.length === previousData.length) {
                const lastInNew = seriesData[seriesData.length - 1];
                if (previousData[previousData.length - 1].time === lastInNew.time) {
                    series.update({ ...lastInNew } as never);
                    return;
                }
            }
        }
        series.setData(seriesData.map(d => ({ ...d })) as never);
      });
      // Update stored definitions (new data references)
      existing.definitions = definitions;
      return;
    }
    // Structural change: fall through to full recreation
    this.setIndicator(key, paneId, definitions, paneOrder, height);
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
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        let created: any;
        if (definition.type === 'ichimoku-cloud') {
          // Ichimoku Cloud Custom Series
          created = this.chart.addCustomSeries(new IchimokuCloudPlugin(), options, paneIndex);
        } else if (definition.type === 'histogram') {
          created = this.chart.addSeries(HistogramSeries, options, paneIndex);
        } else {
          created = this.chart.addSeries(LineSeries, options, paneIndex);
        }
        staged.push(created);
        stage = `set-data:${definition.seriesKey}`;
        const seriesData = (paneId === 'volume' && (definition.seriesKey === 'raw-volume' || definition.type === 'histogram')) ? this.volumeData : definition.data;
        created.setData(seriesData.map(d => ({ ...d })) as never);
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
