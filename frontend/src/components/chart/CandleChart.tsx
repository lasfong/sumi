import { forwardRef, useEffect, useImperativeHandle, useRef } from 'react';
import { createChart, type IChartApi, type ISeriesApi, type Time } from 'lightweight-charts';
import { DrawingToolRegistry } from './DrawingToolRegistry';
import { IndicatorRenderRegistry } from './IndicatorRenderRegistry';
import { PaneManager } from './PaneManager';
import { SeriesManager } from './SeriesManager';
import type {
  ChartWorkspaceProps,
  ChartWorkspaceRef,
  DrawingLine,
  DrawingPoint,
} from './workspaceTypes';

type ChartMouseParam = Parameters<Parameters<IChartApi['subscribeClick']>[0]>[0];
type OhlcPoint = { open?: number; high?: number; low?: number; close?: number };
type PendingDrawing = { points: DrawingPoint[]; preview: ISeriesApi<'Line'> };

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
  drawings = [],
  activeTool = 'cursor',
  onDrawingComplete,
}, ref) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const panesRef = useRef<PaneManager | null>(null);
  const seriesRef = useRef<SeriesManager | null>(null);
  const drawingRegistryRef = useRef<DrawingToolRegistry | null>(null);
  const pendingRef = useRef<PendingDrawing | null>(null);

  useImperativeHandle(ref, () => ({
    addIndicator: input => {
      const paneId = IndicatorRenderRegistry.paneFor(input.id, input.pane);
      seriesRef.current?.setIndicator(input.key, paneId, input.series);
    },
    removeIndicator: key => seriesRef.current?.removeIndicator(key),
    clearIndicators: () => seriesRef.current?.clearIndicators(),
    updateCandle: (candle, volume) => seriesRef.current?.updateCandle(candle, volume),
  }), []);

  useEffect(() => {
    if (!containerRef.current) return;
    const chart = createChart(containerRef.current, {
      autoSize: true,
      layout: { background: { color: '#0D1117' }, textColor: '#F0F6FC', attributionLogo: true },
      grid: {
        vertLines: { color: 'rgba(255,255,255,0.05)' },
        horzLines: { color: 'rgba(255,255,255,0.05)' },
      },
      crosshair: { mode: 1 },
      rightPriceScale: { borderColor: 'rgba(255,255,255,0.12)' },
      timeScale: { borderColor: 'rgba(255,255,255,0.12)', timeVisible: false, shiftVisibleRangeOnNewBar: true },
      height: containerRef.current.clientHeight || 500,
    });
    const panes = new PaneManager(chart);
    const series = new SeriesManager(chart, panes);
    const drawingRegistry = new DrawingToolRegistry(chart, series.candles, panes.index('price'));
    chartRef.current = chart;
    panesRef.current = panes;
    seriesRef.current = series;
    drawingRegistryRef.current = drawingRegistry;

    return () => {
      pendingRef.current = null;
      drawingRegistry.clear();
      chart.remove();
      chartRef.current = null;
      panesRef.current = null;
      seriesRef.current = null;
      drawingRegistryRef.current = null;
    };
  }, []);

  useEffect(() => { seriesRef.current?.setCandles(data, markers); }, [data, markers]);
  useEffect(() => { seriesRef.current?.setVolume(volumeData); }, [volumeData]);
  useEffect(() => { drawingRegistryRef.current?.render(drawings); }, [drawings]);

  useEffect(() => {
    const chart = chartRef.current;
    const series = seriesRef.current?.candles;
    const registry = drawingRegistryRef.current;
    if (!chart || !series || !registry) return;

    const snappedPrice = (param: ChartMouseParam, fallback: number): number => {
      if (!param.point || !param.seriesData) return fallback;
      const candle = param.seriesData.get(series) as OhlcPoint | undefined;
      if (!candle) return fallback;
      return [candle.open, candle.high, candle.low, candle.close]
        .filter((price): price is number => typeof price === 'number')
        .reduce((best, price) => {
          const coordinate = series.priceToCoordinate(price);
          return coordinate !== null && Math.abs(coordinate - param.point!.y) <= 10
            && Math.abs(coordinate - param.point!.y) < Math.abs((series.priceToCoordinate(best) ?? Infinity) - param.point!.y)
            ? price : best;
        }, fallback);
    };

    const click = (param: ChartMouseParam) => {
      if (!param.point || !param.time || !activeTool || activeTool === 'cursor') return;
      const rawPrice = series.coordinateToPrice(param.point.y);
      if (rawPrice === null) return;
      const point = { time: param.time, price: snappedPrice(param, rawPrice) };
      if (activeTool === 'horizontal') {
        onDrawingComplete?.(newDrawing('horizontal', [point]));
        return;
      }
      if (!pendingRef.current) {
        pendingRef.current = { points: [point], preview: registry.createPreview() };
        return;
      }
      const pending = pendingRef.current;
      registry.removePreview(pending.preview);
      pendingRef.current = null;
      onDrawingComplete?.(newDrawing(activeTool, [pending.points[0], point]));
    };

    const move = (param: ChartMouseParam) => {
      if (!param.point || !param.time || !pendingRef.current) return;
      const rawPrice = series.coordinateToPrice(param.point.y);
      if (rawPrice === null) return;
      const start = pendingRef.current.points[0];
      const end = { time: param.time, value: snappedPrice(param, rawPrice) };
      const points = [{ time: start.time, value: start.price }, end]
        .sort((a, b) => timeValue(a.time) - timeValue(b.time));
      pendingRef.current.preview.setData(points);
    };

    chart.subscribeClick(click);
    chart.subscribeCrosshairMove(move);
    return () => {
      chart.unsubscribeClick(click);
      chart.unsubscribeCrosshairMove(move);
      if (pendingRef.current) {
        registry.removePreview(pendingRef.current.preview);
        pendingRef.current = null;
      }
    };
  }, [activeTool, onDrawingComplete]);

  return <div ref={containerRef} data-testid="chart-workspace" style={{ width: '100%', height: '100%', minHeight: 360 }} />;
});

ChartWorkspace.displayName = 'ChartWorkspace';
export const CandleChart = ChartWorkspace;

const newDrawing = (type: DrawingLine['type'], points: DrawingPoint[]): DrawingLine => ({
  id: globalThis.crypto?.randomUUID?.() ?? Math.random().toString(36).slice(2),
  type,
  points,
  color: '#E056FD',
});

const timeValue = (time: Time): number => typeof time === 'string' ? new Date(time).getTime() : Number(time);
