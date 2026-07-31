import { useEffect, useRef, useState, useCallback, useMemo } from 'react';
import { useQuery, useMutation } from '@tanstack/react-query';
import type { CandleChartRef } from '../chart/CandleChart';
import type { SeriesMarker, Time, SeriesMarkerPosition, SeriesMarkerShape } from 'lightweight-charts';
import { createReplaySession, getReplaySession, getSessionCandles, nextCandle, previousCandle } from '../../api/replayApi';
import { getPracticeState, submitDecision } from '../../api/decisionApi';
import { createJournalEntry, getJournalEntries } from '../../api/journalApi';
import { getIndicatorRegistry, getSessionIndicatorData, type IndicatorDefinition } from '../../api/indicatorsApi';
import { useReplayStore } from '../../store/replayStore';
import toast from 'react-hot-toast';
import type {
  Candle, ChartCandle, ChartVolume, DecisionCreate, CreateSessionRequest, JournalEntryCreate,
  ReplaySession, ReplaySourceContext,
} from '../../types';
import { IndicatorRenderRegistry } from '../chart/IndicatorRenderRegistry';
import { useWebSocket } from '../../hooks/useWebSocket';
import type { WebSocketMessage } from '../../hooks/useWebSocket';
import { useQueryClient } from '@tanstack/react-query';
import { sortDateKeys, toDateKey, unixSecondsToDateKey } from '../../utils/date';
import { useDrawingWorkspaceController } from '../../features/drawings/useDrawingWorkspaceController';
import {
  addIndicator, approvedDefinitions, createIndicatorInstance, emptyIndicatorDocument, moveIndicator,
  removeIndicator, toggleIndicator, updateIndicator, validateIndicatorParams,
  type IndicatorDocumentV1, type IndicatorSeriesStyle,
} from '../../features/indicators/indicatorDomain';
import { IndicatorRepository } from '../../features/indicators/IndicatorRepository';
import { IndicatorRequestCoordinator } from '../../features/indicators/IndicatorRequestCoordinator';
import type { IndicatorDataPoint } from '../../api/indicatorsApi';
import type { IndicatorRuntimeState } from '../chart/IndicatorManager';
import type { DrawingTool } from '../../features/drawings/drawingDomain';
import { isGlobalShortcutEligible } from '../../features/replay/globalShortcutPolicy';

interface ApiError {
  response?: {
    data?: {
      detail?: string;
    };
  };
}

interface WebSocketCandle {
  time: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

const isWebSocketCandle = (data: unknown): data is WebSocketCandle => {
  if (!data || typeof data !== 'object') return false;
  const candidate = data as Record<string, unknown>;
  return ['time', 'open', 'high', 'low', 'close', 'volume'].every(key => typeof candidate[key] === 'number');
};

export const buildScannerSignalMarker = (
  sourceContext: ReplaySourceContext | undefined,
  currentDate: string | null,
): SeriesMarker<Time>[] => {
  const signalDate = toDateKey(sourceContext?.revealed ? sourceContext.signal?.timestamp : null);
  if (!signalDate || !currentDate || signalDate > currentDate) return [];
  return [{
    time: signalDate as Time,
    position: 'aboveBar',
    color: '#FFD166',
    shape: 'circle',
    text: sourceContext?.signal?.type ? `Signal: ${sourceContext.signal.type}` : 'Signal',
  }];
};

/** Application controller boundary for replay queries, commands and workspace state. */
export const useReplayWorkspaceController = () => {
  const store = useReplayStore();
  const chartRef = useRef<CandleChartRef>(null);
  const queryClient = useQueryClient();

  const [isPlaying, setIsPlaying] = useState(false);
  const [playSpeed, setPlaySpeed] = useState(500); // ms per candle

  const indicatorRepository = useMemo(() => new IndicatorRepository(window.localStorage), []);
  const indicatorRequests = useMemo(() => new IndicatorRequestCoordinator<IndicatorDataPoint[]>(), []);
  const indicatorHydratedSessionRef = useRef<number | null>(null);
  const { data: registry = [] } = useQuery({ queryKey: ['indicator-registry'], queryFn: getIndicatorRegistry, staleTime: Infinity });
  const indicatorDefinitions = useMemo(() => approvedDefinitions(registry), [registry]);
  const [indicatorDocument, setIndicatorDocument] = useState<IndicatorDocumentV1>(() => emptyIndicatorDocument(store.sessionId ?? 1));
  const [indicatorRuntime, setIndicatorRuntime] = useState<Record<string, IndicatorRuntimeState>>({});

  useEffect(() => {
    // Volume is locally defined, so approvedDefinitions is non-empty before the backend
    // registry arrives. Hydrating against that partial registry would reject valid EMA/RSI state.
    if (!store.sessionId || !registry.length || indicatorHydratedSessionRef.current === store.sessionId) return;
    indicatorHydratedSessionRef.current = store.sessionId;
    setIndicatorDocument(indicatorRepository.load(store.sessionId, indicatorDefinitions));
    setIndicatorRuntime({}); chartRef.current?.clearIndicators();
  }, [indicatorDefinitions, indicatorRepository, registry.length, store.sessionId]);

  const createMutation = useMutation({
    mutationFn: createReplaySession,
    onSuccess: (data) => {
      indicatorHydratedSessionRef.current = data.id;
      const empty = emptyIndicatorDocument(data.id); setIndicatorDocument(empty); indicatorRepository.save(empty); setIndicatorRuntime({});
      store.setSession(data.id);
      toast.success(`Session #${data.id} created!`);
    },
    onError: (err: ApiError) => {
      toast.error(err?.response?.data?.detail || 'Failed to create session');
    }
  });

  // ... (keeping query blocks identical)
  const { data: candlesData, refetch: refetchCandles } = useQuery({
    queryKey: ['candles', store.sessionId],
    queryFn: () => getSessionCandles(store.sessionId!),
    enabled: !!store.sessionId,
  });

  const { data: sessionData, isError: isSessionError, refetch: refetchSession } = useQuery({
    queryKey: ['replay-session', store.sessionId],
    queryFn: () => getReplaySession(store.sessionId!),
    enabled: !!store.sessionId,
  });

  const { data: practiceData, isLoading: practiceLoading, isError: practiceError, refetch: refetchPractice } = useQuery({
    queryKey: ['practice-state', store.sessionId],
    queryFn: () => getPracticeState(store.sessionId!),
    enabled: !!store.sessionId,
  });

  const { data: journalData, isLoading: journalLoading, isError: journalError, refetch: refetchJournal } = useQuery({
    queryKey: ['journal', store.sessionId],
    queryFn: () => getJournalEntries(store.sessionId!),
    enabled: !!store.sessionId,
  });

  const synchronizeNavigation = useCallback(async () => {
    await Promise.all([refetchCandles(), refetchSession(), refetchPractice(), refetchJournal()]);
  }, [refetchCandles, refetchJournal, refetchPractice, refetchSession]);

  const nextMutation = useMutation({
    mutationFn: (steps: number) => nextCandle(store.sessionId!, steps),
    onSuccess: synchronizeNavigation,
    onError: () => {
      setIsPlaying(false);
      toast.error('End of data or error');
    },
  });

  const prevMutation = useMutation({
    mutationFn: (steps: number) => previousCandle(store.sessionId!, steps),
    onSuccess: synchronizeNavigation,
    onError: () => toast.error('Start of data or error'),
  });

  const handleCreateSession = useCallback((data: CreateSessionRequest) => {
    createMutation.mutate(data);
  }, [createMutation]);

  const handleResumeSession = useCallback((sessionId: number) => {
    store.setSession(sessionId);
    toast.success(`Session #${sessionId} resumed`);
  }, [store]);

  const handleClearSession = useCallback(() => {
    setIsPlaying(false);
    setIndicatorDocument(emptyIndicatorDocument(1)); setIndicatorRuntime({}); indicatorRequests.cancelAll(); chartRef.current?.clearIndicators();
    store.clearSession();
  }, [indicatorRequests, store]);

  const handleSubmitDecision = useCallback(async (decision: DecisionCreate) => {
    if (!store.sessionId) return { ok: false, message: 'No replay session is active.' };
    try {
      await submitDecision(store.sessionId, decision);
      await Promise.all([refetchPractice(), refetchSession()]);

      const message = decision.action === 'HOLD' || decision.action === 'SKIP'
        ? `${decision.action} decision recorded without an order.`
        : decision.order_type === 'LIMIT' ? `${decision.action} LIMIT order recorded.` : `${decision.action} executed at the visible close.`;
      toast.success(message);
      return { ok: true, message };
    } catch (err: unknown) {
      const apiError = err as ApiError;
      const message = apiError?.response?.data?.detail || 'Transport error: decision was not confirmed. Retry when the local API is available.';
      toast.error(message);
      await refetchPractice();
      return { ok: false, message };
    }
  }, [store.sessionId, refetchPractice, refetchSession]);

  const handleSaveJournal = useCallback(async (entry: JournalEntryCreate) => {
    if (!store.sessionId) return { ok: false, message: 'No replay session is active.' };
    try {
      await createJournalEntry(store.sessionId, entry);
      await refetchJournal();
      return { ok: true, message: 'Checklist saved for the current replay context.' };
    } catch (err: unknown) {
      const apiError = err as ApiError;
      return { ok: false, message: apiError?.response?.data?.detail || 'Transport error: journal draft was not saved.' };
    }
  }, [refetchJournal, store.sessionId]);


  const symbolName = candlesData?.[0]?.symbol || '—';
  const currentCandle = candlesData?.length ? candlesData[candlesData.length - 1] : null;
  const sourceContext = sessionData?.source_context;
  const currentDate = toDateKey(currentCandle?.timestamp);
  const drawing = useDrawingWorkspaceController(store.sessionId ?? 1, symbolName);
  const selectedDrawing = drawing.document.drawings.find(item => drawing.selection.includes(item.id));
  const handleDrawingTool = useCallback((tool: DrawingTool) => {
    if (tool === 'select') chartRef.current?.cancelDrawing();
    drawing.setTool(tool);
  }, [drawing]);

  const handleWebSocketMessage = useCallback((msg: WebSocketMessage) => {
    if (msg.type === 'new_candle') {
      const newCandle = msg.data;
      if (!isWebSocketCandle(newCandle)) return;
      const wsDateKey = unixSecondsToDateKey(newCandle.time);

      // Update chart directly for smoothness
      if (chartRef.current) {
        const chartCandle = {
          time: wsDateKey as Time,
          open: newCandle.open,
          high: newCandle.high,
          low: newCandle.low,
          close: newCandle.close,
        };
        chartRef.current.updateCandle(chartCandle, {
          time: newCandle.time as Time,
          value: newCandle.volume,
          color: newCandle.close >= newCandle.open ? 'rgba(0, 230, 118, 0.5)' : 'rgba(255, 23, 68, 0.5)'
        });
      }

      // Update react-query cache to sync rest of UI
      queryClient.setQueryData(['candles', store.sessionId], (old: Candle[] | undefined) => {
        if (!old) return old;
        const dbCandle = {
          id: 0,
          session_id: store.sessionId!,
          symbol: symbolName,
          timeframe: 'D',
          adjustment_type: 'split',
          timestamp: wsDateKey,
          open: newCandle.open,
          high: newCandle.high,
          low: newCandle.low,
          close: newCandle.close,
          volume: newCandle.volume
        };
        return [...old, dbCandle];
      });

      // Sync positions silently
      queryClient.invalidateQueries({ queryKey: ['practice-state', store.sessionId] });
      if (msg.source_context) {
        queryClient.setQueryData(
          ['replay-session', store.sessionId],
          (old: ReplaySession | undefined) => old ? { ...old, source_context: msg.source_context! } : old,
        );
      } else {
        queryClient.invalidateQueries({ queryKey: ['replay-session', store.sessionId] });
      }
      queryClient.invalidateQueries({ queryKey: ['journal', store.sessionId] });
    }
  }, [store.sessionId, queryClient, symbolName]);

  const { isConnected, sendCommand } = useWebSocket(store.sessionId, handleWebSocketMessage);
  const handleNext = useCallback((steps: number = 1) => {
    if (store.sessionId) {
      nextMutation.mutate(steps);
    }
  }, [store.sessionId, nextMutation]);

  const handlePrev = useCallback((steps: number = 1) => {
    if (store.sessionId) {
      prevMutation.mutate(steps);
    }
  }, [store.sessionId, prevMutation]);

  useEffect(() => {
    if (isSessionError) {
      toast.error('Saved replay session was not found');
      store.clearSession();
    }
  }, [isSessionError, store]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!isGlobalShortcutEligible(e)) return;
      if (!store.sessionId) return;

      switch (e.code) {
        case 'Escape':
          chartRef.current?.cancelDrawing();
          drawing.setTool('select');
          break;
        case 'Delete':
        case 'Backspace':
          if (drawing.selection.length) { e.preventDefault(); drawing.remove(); }
          break;
        case 'Space':
        case 'ArrowRight':
          e.preventDefault();
          handleNext(e.shiftKey ? 5 : 1);
          break;
        case 'ArrowLeft':
          e.preventDefault();
          handlePrev(e.shiftKey ? 5 : 1);
          break;
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [drawing, handleNext, handlePrev, store.sessionId]);

  const chartCandleRows = useMemo(() => Array.from(
    new Map((candlesData || []).map((c: Candle) => [toDateKey(c.timestamp) || c.timestamp, c] as const)).entries()
  ).sort(([a], [b]) => sortDateKeys(a, b)), [candlesData]);

  const formattedCandles: ChartCandle[] = useMemo(() => chartCandleRows.map(([time, c]) => ({
    time,
    open: c.open,
    high: c.high,
    low: c.low,
    close: c.close,
  })), [chartCandleRows]);

  const volumeData: ChartVolume[] = useMemo(() => chartCandleRows.map(([time, c]) => ({
    time,
    value: c.volume,
    color: c.close >= c.open ? 'rgba(0, 230, 118, 0.5)' : 'rgba(255, 23, 68, 0.5)',
  })), [chartCandleRows]);

  const decisionMarkers: SeriesMarker<Time>[] = (practiceData?.executions || [])
    .map((execution) => {
      const isBuy = execution.side === 'BUY';
      const dateStr = toDateKey(execution.execution_date) || execution.execution_date;
      return {
        time: dateStr as Time,
        position: (isBuy ? 'belowBar' : 'aboveBar') as SeriesMarkerPosition,
        color: isBuy ? '#00E676' : '#FF1744',
        shape: (isBuy ? 'arrowUp' : 'arrowDown') as SeriesMarkerShape,
        text: `${execution.side} ${execution.quantity}`,
      } as SeriesMarker<Time>;
    })
    .sort((a, b) => (a.time as string).localeCompare(b.time as string));

  const signalMarker = buildScannerSignalMarker(sourceContext, currentDate);

  const markers: SeriesMarker<Time>[] = [...decisionMarkers, ...signalMarker]
    .sort((a, b) => (a.time as string).localeCompare(b.time as string));

  // Auto-play effect via WebSocket
  useEffect(() => {
    if (isPlaying && isConnected) {
      sendCommand('start', { speed: playSpeed });
    } else if (!isPlaying && isConnected) {
      sendCommand('pause');
    }
  }, [isPlaying, playSpeed, isConnected, sendCommand]);

  const commitIndicatorDocument = useCallback((next: IndicatorDocumentV1) => {
    indicatorRepository.save(next); setIndicatorDocument(next);
  }, [indicatorRepository]);

  const addIndicatorInstance = useCallback((definition: IndicatorDefinition, params: Record<string, unknown>) => {
    if (!store.sessionId) return;
    try {
      const instance = createIndicatorInstance(definition, params, indicatorDocument.instances.length);
      commitIndicatorDocument(addIndicator(indicatorDocument, instance));
    } catch (error) { toast.error(error instanceof Error ? error.message : 'Invalid indicator settings'); }
  }, [commitIndicatorDocument, indicatorDocument, store.sessionId]);

  const updateIndicatorInstance = useCallback((id: string, params: Record<string, unknown>, styles: Record<string, IndicatorSeriesStyle>) => {
    const instance = indicatorDocument.instances.find(item => item.id === id);
    const definition = indicatorDefinitions.find(item => item.id === instance?.definitionId); if (!instance || !definition) return;
    const validated = validateIndicatorParams(definition, params);
    if (Object.keys(validated.errors).length) { toast.error(Object.values(validated.errors)[0]); return; }
    indicatorRequests.invalidate(id);
    commitIndicatorDocument(updateIndicator(indicatorDocument, id, { params: validated.params, styles }));
  }, [commitIndicatorDocument, indicatorDefinitions, indicatorDocument, indicatorRequests]);

  const removeIndicatorInstance = useCallback((id: string) => {
    indicatorRequests.invalidate(id); chartRef.current?.removeIndicator(id);
    setIndicatorRuntime(previous => { const next = { ...previous }; delete next[id]; return next; });
    commitIndicatorDocument(removeIndicator(indicatorDocument, id));
  }, [commitIndicatorDocument, indicatorDocument, indicatorRequests]);

  const toggleIndicatorInstance = useCallback((id: string) => {
    const instance = indicatorDocument.instances.find(item => item.id === id); if (!instance) return;
    if (instance.visible) { indicatorRequests.invalidate(id); chartRef.current?.removeIndicator(id); }
    commitIndicatorDocument(toggleIndicator(indicatorDocument, id));
  }, [commitIndicatorDocument, indicatorDocument, indicatorRequests]);

  const moveIndicatorInstance = useCallback((id: string, direction: -1 | 1) => {
    commitIndicatorDocument(moveIndicator(indicatorDocument, id, direction));
  }, [commitIndicatorDocument, indicatorDocument]);

  useEffect(() => {
    if (!currentCandle || !store.sessionId) return;
    let effectActive = true;
    const visible = indicatorDocument.instances.filter(instance => instance.visible);
    indicatorDocument.instances.filter(instance => !instance.visible).forEach(instance => chartRef.current?.removeIndicator(instance.id));
    const paneOrder = visible.filter(instance => instance.placement !== 'price').map(instance => instance.paneId);
    const publishFailure = (instanceId: string, errorKind: 'transport' | 'mapping' | 'chart', message: string) => {
      if (!effectActive) return;
      setIndicatorRuntime(previous => ({ ...previous, [instanceId]: {
        status: 'error', errorKind, error: message, values: previous[instanceId]?.values ?? {},
      } }));
      window.dispatchEvent(new CustomEvent('sumi:indicator-runtime-error', { detail: { instanceId, errorKind, message } }));
    };
    const technicalDetail = (error: unknown) => error instanceof Error && error.message ? ` (${error.message})` : '';
    try {
      chartRef.current?.setIndicatorOrder(paneOrder);
    } catch {
      visible.filter(instance => instance.placement !== 'price').forEach(instance => publishFailure(
        instance.id, 'chart', 'Chart layout failed — the previous valid display was retained.',
      ));
      return () => { effectActive = false; indicatorRequests.cancelAll(); };
    }
    for (const instance of visible) {
      if (instance.definitionId === 'volume') {
        const series = IndicatorRenderRegistry.mapVolume(volumeData, instance);
        try {
          if (!chartRef.current) throw new Error('Chart workspace is not mounted');
          chartRef.current.addIndicator({ instanceId: instance.id, paneId: instance.paneId, series, paneOrder });
          queueMicrotask(() => effectActive && setIndicatorRuntime(previous => ({ ...previous, [instance.id]: {
            status: series[0].data.length ? 'ready' : 'warming', values: IndicatorRenderRegistry.currentValues(series),
            inputMaxDate: currentDate, responseMaxDate: String(series[0].data.at(-1)?.time ?? '').slice(0, 10) || null,
            responseCount: series[0].data.length,
          } })));
        } catch (error) {
          publishFailure(instance.id, 'chart', `Chart rendering failed — the previous valid display was retained.${technicalDetail(error)}`);
        }
        continue;
      }
      queueMicrotask(() => effectActive && setIndicatorRuntime(previous => ({
        ...previous, [instance.id]: { ...previous[instance.id], status: 'loading', values: previous[instance.id]?.values ?? {} },
      })));
      const paramsKey = JSON.stringify(Object.entries(instance.params).sort(([a], [b]) => a.localeCompare(b)));
      const workKey = `${store.sessionId}:${candlesData?.length ?? 0}:${instance.definitionId}:${paramsKey}`;
      void (async () => {
        let result: { stale: boolean; data?: IndicatorDataPoint[] };
        try {
          result = await indicatorRequests.request(instance.id, workKey, signal => getSessionIndicatorData(store.sessionId!, instance.definitionId, instance.params, signal));
        } catch (error) {
          if (!effectActive || (error as { code?: string }).code === 'ERR_CANCELED' || (error as { name?: string }).name === 'AbortError') return;
          publishFailure(instance.id, 'transport', 'Data request failed — retry by showing or editing this indicator.');
          return;
        }
        if (!effectActive || result.stale || !result.data) return;
        const responseData = result.data;

        let series;
        try {
          series = IndicatorRenderRegistry.mapBackendData(responseData, instance);
        } catch (error) {
          publishFailure(instance.id, 'mapping', `Indicator data could not be interpreted — the previous valid display was retained.${technicalDetail(error)}`);
          return;
        }
        if (!series.length) {
          chartRef.current?.removeIndicator(instance.id);
          setIndicatorRuntime(previous => ({ ...previous, [instance.id]: {
            status: 'warming', values: {}, inputMaxDate: currentDate,
            responseMaxDate: toDateKey(responseData.at(-1)?.timestamp) || null, responseCount: responseData.length,
          } }));
          return;
        }
        try {
          if (!chartRef.current) throw new Error('Chart workspace is not mounted');
          chartRef.current.addIndicator({ instanceId: instance.id, paneId: instance.paneId, series, paneOrder });
          chartRef.current.setIndicatorOrder(paneOrder);
        } catch (error) {
          publishFailure(instance.id, 'chart', `Chart rendering failed — the previous valid display was retained.${technicalDetail(error)}`);
          return;
        }
        if (effectActive) setIndicatorRuntime(previous => ({ ...previous, [instance.id]: {
          status: 'ready', values: IndicatorRenderRegistry.currentValues(series),
          inputMaxDate: currentDate, responseMaxDate: toDateKey(responseData.at(-1)?.timestamp) || null, responseCount: responseData.length,
        } }));
      })();
    }
    return () => { effectActive = false; indicatorRequests.cancelAll(); };
  }, [candlesData?.length, currentCandle, currentDate, indicatorDocument, indicatorRequests, store.sessionId, volumeData]);

  useEffect(() => () => indicatorRequests.cancelAll(), [indicatorRequests]);
  return {
    sessionId: store.sessionId, chartRef, symbolName, sessionStatus: sessionData?.status, sourceContext, currentDate, currentCandle, candleCount: candlesData?.length ?? 0,
    handleCreateSession, handleResumeSession, isCreatingSession: createMutation.isPending, handleClearSession,
    indicatorDefinitions, indicatorDocument, indicatorRuntime, addIndicatorInstance, updateIndicatorInstance,
    removeIndicatorInstance, toggleIndicatorInstance, moveIndicatorInstance,
    playSpeed, setPlaySpeed, isPlaying, setIsPlaying,
    handlePrev, handleNext, navigationPending: prevMutation.isPending || nextMutation.isPending,
    drawing, selectedDrawing, handleDrawingTool, formattedCandles, volumeData, markers,
    practiceData, practiceLoading, practiceError,
    journalData, journalLoading, journalError, handleSaveJournal, handleSubmitDecision,
  };
};
