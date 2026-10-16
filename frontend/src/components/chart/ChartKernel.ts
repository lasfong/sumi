import {
  AreaSeries,
  HistogramSeries,
  createChart,
  type DeepPartial,
  type ChartOptions,
  type IChartApi,
  type ISeriesApi,
  type SeriesMarker,
  type Time,
} from 'lightweight-charts';
import { SumiPrimitiveDrawingProvider } from '../../features/drawings/SumiPrimitiveDrawingProvider';
import type { DrawingProvider, DrawingProviderEvent } from '../../features/drawings/DrawingProvider';
import type { SumiDrawingDocumentV1 } from '../../features/drawings/drawingDomain';
import { PaneManager } from './PaneManager';
import { SeriesManager } from './SeriesManager';
import type {
  CandleData,
  IndicatorChartSnapshot,
  IndicatorRenderInput,
  PaneId,
  VolumeData,
} from './workspaceTypes';

export interface ReplayKernelOptions {
  initialDrawingDocument?: SumiDrawingDocumentV1;
  getCurrentTime: () => string;
  onDrawingProviderEvent?: (event: DrawingProviderEvent) => void;
  chartOptions?: DeepPartial<ChartOptions>;
}

export interface EquitySeriesPoint {
  time: Time;
  value: number;
  color?: string;
}

export interface EquityKernelHandle {
  readonly chart: IChartApi;
  setData: (equityData: EquitySeriesPoint[], drawdownData: EquitySeriesPoint[]) => void;
  resize: (width: number, height: number) => void;
  destroy: () => void;
}

/**
 * ChartKernel — Thin Facade over Lightweight Charts v5 (ADR-CHART-001 / GAP-FE-05).
 * Owns all chart creation (`createChart`), series/pane lifecycle, primitive attachment,
 * resize coordination, and deterministic teardown.
 */
export class ChartKernel {
  readonly chart: IChartApi;
  readonly panes: PaneManager;
  readonly series: SeriesManager;
  readonly container: HTMLElement;
  private drawingProvider: DrawingProvider | null = null;
  private destroyed = false;

  private constructor(
    container: HTMLElement,
    chart: IChartApi,
    panes: PaneManager,
    series: SeriesManager,
  ) {
    this.container = container;
    this.chart = chart;
    this.panes = panes;
    this.series = series;
  }

  static createReplayKernel(
    container: HTMLElement,
    options: ReplayKernelOptions,
  ): ChartKernel {
    const chart = createChart(container, {
      autoSize: true,
      layout: {
        background: { color: '#0D1117' },
        textColor: '#F0F6FC',
        attributionLogo: true,
      },
      grid: {
        vertLines: { color: 'rgba(255,255,255,0.05)' },
        horzLines: { color: 'rgba(255,255,255,0.05)' },
      },
      crosshair: { mode: 1 },
      rightPriceScale: { borderColor: 'rgba(255,255,255,0.12)' },
      timeScale: {
        borderColor: 'rgba(255,255,255,0.12)',
        timeVisible: false,
        shiftVisibleRangeOnNewBar: true,
        rightOffset: 12,
      },
      height: container.clientHeight || 500,
      ...options.chartOptions,
    });

    const panes = new PaneManager(chart);
    const series = new SeriesManager(chart, panes);
    const kernel = new ChartKernel(container, chart, panes, series);

    if (options.initialDrawingDocument) {
      kernel.attachDrawingProvider(
        options.initialDrawingDocument,
        options.getCurrentTime,
        options.onDrawingProviderEvent,
      );
    }

    return kernel;
  }

  static createEquityKernel(container: HTMLElement): EquityKernelHandle {
    const chart = createChart(container, {
      layout: {
        background: { color: 'transparent' },
        textColor: '#F0F6FC',
      },
      grid: {
        vertLines: { color: 'rgba(255,255,255,0.05)' },
        horzLines: { color: 'rgba(255,255,255,0.05)' },
      },
      rightPriceScale: {
        borderColor: 'rgba(255,255,255,0.1)',
      },
      timeScale: {
        borderColor: 'rgba(255,255,255,0.1)',
      },
      crosshair: {
        mode: 1,
      },
    });

    const equitySeries: ISeriesApi<'Area'> = chart.addSeries(AreaSeries, {
      lineColor: '#2962FF',
      topColor: 'rgba(41, 98, 255, 0.4)',
      bottomColor: 'rgba(41, 98, 255, 0.0)',
      lineWidth: 2,
    });

    const drawdownSeries: ISeriesApi<'Histogram'> = chart.addSeries(HistogramSeries, {
      color: 'rgba(255, 23, 68, 0.5)',
      priceFormat: {
        type: 'volume',
      },
      priceScaleId: '',
    });
    drawdownSeries.priceScale().applyOptions({
      scaleMargins: {
        top: 0.8,
        bottom: 0,
      },
    });

    let isDestroyed = false;

    return {
      chart,
      setData: (equityData, drawdownData) => {
        if (isDestroyed || equityData.length === 0) return;
        equitySeries.setData(equityData);
        drawdownSeries.setData(drawdownData);
        chart.timeScale().fitContent();
      },
      resize: (width, height) => {
        if (isDestroyed) return;
        chart.applyOptions({ width, height });
      },
      destroy: () => {
        if (isDestroyed) return;
        isDestroyed = true;
        chart.remove();
      },
    };
  }

  attachDrawingProvider(
    initialDocument: SumiDrawingDocumentV1,
    getCurrentTime: () => string,
    onEvent?: (event: DrawingProviderEvent) => void,
  ): DrawingProvider {
    this.drawingProvider?.destroy();
    const provider = new SumiPrimitiveDrawingProvider(
      this.chart,
      this.series.candles,
      this.container,
      initialDocument,
      getCurrentTime,
      () => this.panes.get('price').getHTMLElement(),
    );
    if (onEvent) {
      provider.subscribe((event) => onEvent(event));
    }
    this.drawingProvider = provider;
    return provider;
  }

  get provider(): DrawingProvider | null {
    return this.drawingProvider;
  }

  setCandles(data: CandleData[], markers: SeriesMarker<Time>[] = []): void {
    if (this.destroyed) return;
    this.series.setCandles(data, markers);
  }

  setVolume(volumeData: VolumeData[]): void {
    if (this.destroyed) return;
    this.series.setVolume(volumeData);
  }

  updateCandle(candle: CandleData, volume?: VolumeData): void {
    if (this.destroyed) return;
    this.series.updateCandle(candle, volume);
  }

  refreshCandles(): void {
    if (this.destroyed) return;
    this.series.refreshCandles();
  }

  addIndicator(input: IndicatorRenderInput, containerHeight?: number): void {
    if (this.destroyed) return;
    const height = containerHeight ?? (this.container.clientHeight || 500);
    this.series.setIndicator(input.instanceId, input.paneId, input.series, input.paneOrder, height);
  }

  updateIndicatorData(input: IndicatorRenderInput, containerHeight?: number): void {
    if (this.destroyed) return;
    const height = containerHeight ?? (this.container.clientHeight || 500);
    this.series.updateIndicatorData(input.instanceId, input.paneId, input.series, input.paneOrder, height);
    this.series.refreshCandles();
  }

  removeIndicator(key: string): void {
    if (this.destroyed) return;
    this.series.removeIndicator(key);
    this.series.refreshCandles();
  }

  clearIndicators(): void {
    if (this.destroyed) return;
    this.series.clearIndicators();
    this.series.refreshCandles();
  }

  setIndicatorOrder(paneIds: PaneId[], containerHeight?: number): void {
    if (this.destroyed) return;
    const height = containerHeight ?? (this.container.clientHeight || 500);
    this.series.layout(paneIds, height);
    this.series.refreshCandles();
  }

  resizeLayout(height: number): void {
    if (this.destroyed) return;
    this.series.resizeLayout(height);
  }

  setPositionLines(
    position: { average_price: number },
    trade: { initial_stop_loss?: number | null; target_price?: number | null } | null,
  ): void {
    if (this.destroyed) return;
    this.series.setPositionLines(position, trade);
  }

  clearPositionLines(): void {
    if (this.destroyed) return;
    this.series.clearPositionLines();
  }

  snapshot(): IndicatorChartSnapshot {
    return this.series.snapshot();
  }

  destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;
    this.drawingProvider?.destroy();
    this.drawingProvider = null;
    this.chart.remove();
  }
}
