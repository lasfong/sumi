import { forwardRef, useEffect, useImperativeHandle, useRef } from 'react';
import { createChart, type IChartApi } from 'lightweight-charts';
import { SumiPrimitiveDrawingProvider } from '../../features/drawings/SumiPrimitiveDrawingProvider';
import type { DrawingProvider } from '../../features/drawings/DrawingProvider';
import { PaneManager } from './PaneManager';
import { SeriesManager } from './SeriesManager';
import type {
  ChartWorkspaceProps,
  ChartWorkspaceRef,
} from './workspaceTypes';

export type {
  ChartWorkspaceRef as CandleChartRef,
  DrawingLine,
  DrawingType,
  IndicatorSeriesData,
} from './workspaceTypes';

export const ChartWorkspace = forwardRef<ChartWorkspaceRef, ChartWorkspaceProps>(({
  data,
  volumeData = [],
  markers = [],
  drawingDocument,
  drawingTool = 'select',
  drawingSelection = [],
  currentDrawingTime = '1970-01-01',
  onDrawingProviderEvent,
  drawingMagnetMode = 'off',
  minimumHeight = 360,
}, ref) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const panesRef = useRef<PaneManager | null>(null);
  const seriesRef = useRef<SeriesManager | null>(null);
  const providerRef = useRef<DrawingProvider | null>(null);
  const providerListenerRef = useRef(onDrawingProviderEvent);
  const currentTimeRef = useRef(currentDrawingTime);
  const initialDrawingDocumentRef = useRef(drawingDocument);
  providerListenerRef.current = onDrawingProviderEvent;
  currentTimeRef.current = currentDrawingTime;

  useImperativeHandle(ref, () => ({
    addIndicator: input => {
      if (!seriesRef.current || !containerRef.current) throw new Error('Chart workspace is not mounted');
      seriesRef.current.setIndicator(input.instanceId, input.paneId, input.series, input.paneOrder, containerRef.current.clientHeight || 500);
      containerRef.current.dataset.indicatorChartState = JSON.stringify(seriesRef.current.snapshot());
    },
    removeIndicator: key => { seriesRef.current?.removeIndicator(key); if (containerRef.current && seriesRef.current) containerRef.current.dataset.indicatorChartState = JSON.stringify(seriesRef.current.snapshot()); },
    clearIndicators: () => { seriesRef.current?.clearIndicators(); if (containerRef.current && seriesRef.current) containerRef.current.dataset.indicatorChartState = JSON.stringify(seriesRef.current.snapshot()); },
    setIndicatorOrder: paneIds => { seriesRef.current?.layout(paneIds, containerRef.current?.clientHeight || 500); if (containerRef.current && seriesRef.current) containerRef.current.dataset.indicatorChartState = JSON.stringify(seriesRef.current.snapshot()); },
    getIndicatorState: () => seriesRef.current?.snapshot() ?? null,
    updateCandle: (candle, volume) => seriesRef.current?.updateCandle(candle, volume),
    cancelDrawing: () => providerRef.current?.cancel(),
    getDrawingInteractionState: () => providerRef.current?.snapshotInteraction() ?? null,
  }), []);

  useEffect(() => {
    if (!containerRef.current) return;
    const container = containerRef.current;
    const chart = createChart(container, {
      autoSize: true,
      layout: { background: { color: '#0D1117' }, textColor: '#F0F6FC', attributionLogo: true },
      grid: {
        vertLines: { color: 'rgba(255,255,255,0.05)' },
        horzLines: { color: 'rgba(255,255,255,0.05)' },
      },
      crosshair: { mode: 1 },
      rightPriceScale: { borderColor: 'rgba(255,255,255,0.12)' },
      timeScale: { borderColor: 'rgba(255,255,255,0.12)', timeVisible: false, shiftVisibleRangeOnNewBar: true },
      height: container.clientHeight || 500,
    });
    const panes = new PaneManager(chart);
    const series = new SeriesManager(chart, panes);
    chartRef.current = chart;
    panesRef.current = panes;
    seriesRef.current = series;
    if (initialDrawingDocumentRef.current) {
      const provider = new SumiPrimitiveDrawingProvider(
        chart, series.candles, container, initialDrawingDocumentRef.current,
        () => currentTimeRef.current, () => panes.get('price').getHTMLElement(),
      );
      provider.subscribe(event => providerListenerRef.current?.(event));
      providerRef.current = provider;
    }
    const publishIndicatorSnapshot = () => {
      container.dataset.indicatorChartState = JSON.stringify(series.snapshot());
    };
    container.dataset.indicatorChartState = JSON.stringify(series.snapshot());
    container.addEventListener('sumi:indicator-snapshot-request', publishIndicatorSnapshot);
    const resizeObserver = new ResizeObserver(entries => {
      const height = Math.round(entries[0]?.contentRect.height ?? container.clientHeight);
      series.resizeLayout(height);
      publishIndicatorSnapshot();
    });
    resizeObserver.observe(container);

    return () => {
      resizeObserver.disconnect();
      container.removeEventListener('sumi:indicator-snapshot-request', publishIndicatorSnapshot);
      providerRef.current?.destroy();
      providerRef.current = null;
      chart.remove();
      chartRef.current = null;
      panesRef.current = null;
      seriesRef.current = null;
    };
  }, []);

  useEffect(() => {
    seriesRef.current?.setCandles(data, markers);
    if (containerRef.current && seriesRef.current) containerRef.current.dataset.indicatorChartState = JSON.stringify(seriesRef.current.snapshot());
  }, [data, markers]);
  useEffect(() => {
    seriesRef.current?.setVolume(volumeData);
    if (containerRef.current && seriesRef.current) containerRef.current.dataset.indicatorChartState = JSON.stringify(seriesRef.current.snapshot());
  }, [volumeData]);
  useEffect(() => { if (drawingDocument) providerRef.current?.replaceDocument(drawingDocument); }, [drawingDocument]);
  useEffect(() => { providerRef.current?.setTool(drawingTool); }, [drawingTool]);
  useEffect(() => { providerRef.current?.select(drawingSelection); }, [drawingSelection]);
  useEffect(() => { providerRef.current?.setMagnet(drawingMagnetMode, data.map(candle => ({ ...candle, time: String(candle.time) }))); }, [data, drawingMagnetMode]);

  return <div ref={containerRef} data-testid="chart-workspace" data-minimum-height={minimumHeight} style={{ width: '100%', height: '100%', minHeight: minimumHeight }} />;
});

ChartWorkspace.displayName = 'ChartWorkspace';
export const CandleChart = ChartWorkspace;
