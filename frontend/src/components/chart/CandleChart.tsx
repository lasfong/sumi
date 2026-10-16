import { forwardRef, useEffect, useImperativeHandle, useRef } from 'react';
import { ChartKernel } from './ChartKernel';
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

const IS_DEV_OR_TEST = Boolean(import.meta.env.DEV || import.meta.env.MODE === 'test');

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
  const kernelRef = useRef<ChartKernel | null>(null);
  const providerListenerRef = useRef(onDrawingProviderEvent);
  const currentTimeRef = useRef(currentDrawingTime);
  const initialDrawingDocumentRef = useRef(drawingDocument);
  providerListenerRef.current = onDrawingProviderEvent;
  currentTimeRef.current = currentDrawingTime;

  const syncDebugSnapshot = () => {
    if (!IS_DEV_OR_TEST || !containerRef.current || !kernelRef.current) return;
    containerRef.current.dataset.indicatorChartState = JSON.stringify(kernelRef.current.snapshot());
  };

  useImperativeHandle(ref, () => ({
    addIndicator: input => {
      if (!kernelRef.current || !containerRef.current) throw new Error('Chart workspace is not mounted');
      kernelRef.current.addIndicator(input, containerRef.current.clientHeight || 500);
      syncDebugSnapshot();
    },
    removeIndicator: key => {
      kernelRef.current?.removeIndicator(key);
      syncDebugSnapshot();
    },
    clearIndicators: () => {
      kernelRef.current?.clearIndicators();
      syncDebugSnapshot();
    },
    setIndicatorOrder: paneIds => {
      kernelRef.current?.setIndicatorOrder(paneIds, containerRef.current?.clientHeight || 500);
      syncDebugSnapshot();
    },
    getIndicatorState: () => kernelRef.current?.snapshot() ?? null,
    updateCandle: (candle, volume) => kernelRef.current?.updateCandle(candle, volume),
    refreshCandles: () => kernelRef.current?.refreshCandles(),
    cancelDrawing: () => kernelRef.current?.provider?.cancel(),
    getDrawingInteractionState: () => kernelRef.current?.provider?.snapshotInteraction() ?? null,
    updateIndicatorData: input => {
      if (!kernelRef.current || !containerRef.current) throw new Error('Chart workspace is not mounted');
      kernelRef.current.updateIndicatorData(input, containerRef.current.clientHeight || 500);
      syncDebugSnapshot();
    },
    setPositionLines: (position, trade) => kernelRef.current?.setPositionLines(position, trade),
    clearPositionLines: () => kernelRef.current?.clearPositionLines(),
  }), []);

  useEffect(() => {
    if (!containerRef.current) return;
    const container = containerRef.current;
    const kernel = ChartKernel.createReplayKernel(container, {
      initialDrawingDocument: initialDrawingDocumentRef.current,
      getCurrentTime: () => currentTimeRef.current,
      onDrawingProviderEvent: event => providerListenerRef.current?.(event),
    });
    kernelRef.current = kernel;

    const publishIndicatorSnapshot = () => {
      container.dataset.indicatorChartState = JSON.stringify(kernel.snapshot());
    };
    if (IS_DEV_OR_TEST) {
      publishIndicatorSnapshot();
    }
    container.addEventListener('sumi:indicator-snapshot-request', publishIndicatorSnapshot);

    const resizeObserver = new ResizeObserver(entries => {
      const height = Math.round(entries[0]?.contentRect.height ?? container.clientHeight);
      kernel.resizeLayout(height);
      if (IS_DEV_OR_TEST) {
        publishIndicatorSnapshot();
      }
    });
    resizeObserver.observe(container);

    return () => {
      resizeObserver.disconnect();
      container.removeEventListener('sumi:indicator-snapshot-request', publishIndicatorSnapshot);
      kernel.destroy();
      kernelRef.current = null;
    };
  }, []);

  useEffect(() => {
    kernelRef.current?.setCandles(data, markers);
    syncDebugSnapshot();
  }, [data, markers]);
  useEffect(() => {
    kernelRef.current?.setVolume(volumeData);
    syncDebugSnapshot();
  }, [volumeData]);
  useEffect(() => { if (drawingDocument) kernelRef.current?.provider?.replaceDocument(drawingDocument); }, [drawingDocument]);
  useEffect(() => { kernelRef.current?.provider?.setTool(drawingTool); }, [drawingTool]);
  useEffect(() => { kernelRef.current?.provider?.select(drawingSelection); }, [drawingSelection]);
  useEffect(() => { kernelRef.current?.provider?.setMagnet(drawingMagnetMode, data.map(candle => ({ ...candle, time: String(candle.time) }))); }, [data, drawingMagnetMode]);

  return <div ref={containerRef} data-testid="chart-workspace" data-minimum-height={minimumHeight} style={{ width: '100%', height: '100%', minHeight: minimumHeight }} />;
});

ChartWorkspace.displayName = 'ChartWorkspace';
export const CandleChart = ChartWorkspace;
