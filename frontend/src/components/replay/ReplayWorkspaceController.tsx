import { useEffect, useRef, useState, useCallback, useMemo } from 'react';
import { useQuery, useMutation } from '@tanstack/react-query';
import type { CandleChartRef } from '../chart/CandleChart';
import type { SeriesMarker, Time, SeriesMarkerPosition, SeriesMarkerShape } from 'lightweight-charts';
import { createReplaySession, getReplaySession, getSessionCandles, nextCandle, previousCandle, resetPracticeSession } from '../../api/replayApi';
import { extractApiErrorMessage } from '../../api/client';
import { cancelPendingOrder, getPracticeState, submitDecision } from '../../api/decisionApi';
import { createJournalEntry, getJournalEntries } from '../../api/journalApi';
import { getIndicatorRegistry, getSessionIndicatorResponse, type IndicatorDefinition, type ReplayIndicatorResponse } from '../../api/indicatorsApi';
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
import { useSessionSelection } from '../../hooks/useSessionSelection';
import type { IndicatorDataPoint } from '../../api/indicatorsApi';
import type { IndicatorRuntimeState } from '../chart/IndicatorManager';
import type { DrawingTool } from '../../features/drawings/drawingDomain';
import { isGlobalShortcutEligible } from '../../features/replay/globalShortcutPolicy';
import {
  cleanupSessionQueries,
  dedupeCandlesByTimestamp,
  normalizeAdjustmentType,
  normalizeReplayTimeframe,
  replayQueryKeys,
  ReplayNavigationCoordinator,
} from '../../features/replay/replayQueryKeys';

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

const isAbortError = (error: unknown): boolean => {
  if (!error || typeof error !== 'object') return false;
  const candidate = error as { name?: string; code?: string };
  return candidate.name === 'AbortError' || candidate.name === 'CanceledError' || candidate.code === 'ERR_CANCELED';
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
  const { sessionId, selectSession, clearSession, isValidating, invalidatedSession } = useSessionSelection();
  const chartRef = useRef<CandleChartRef>(null);
  const queryClient = useQueryClient();

  const [isPlaying, setIsPlaying] = useState(false);
  const [playSpeed, setPlaySpeed] = useState(1000); // Canonical default 1000ms (1x) per ADR-WS-001
  const [navigationPending, setNavigationPending] = useState(false);

  const navCoordinator = useMemo(() => new ReplayNavigationCoordinator(), []);
  const indicatorRepository = useMemo(() => new IndicatorRepository(window.localStorage), []);
  const indicatorRequests = useMemo(() => new IndicatorRequestCoordinator<ReplayIndicatorResponse | IndicatorDataPoint[]>(), []);
  const indicatorHydratedSessionRef = useRef<number | null>(null);
  const { data: registry = [] } = useQuery({ queryKey: ['indicator-registry'], queryFn: getIndicatorRegistry, staleTime: Infinity });
  const indicatorDefinitions = useMemo(() => approvedDefinitions(registry), [registry]);
  const [indicatorDocument, setIndicatorDocument] = useState<IndicatorDocumentV1>(() => emptyIndicatorDocument(sessionId ?? 1));
  const [indicatorRuntime, setIndicatorRuntime] = useState<Record<string, IndicatorRuntimeState>>({});

  useEffect(() => {
    // Volume is locally defined, so approvedDefinitions is non-empty before the backend
    // registry arrives. Hydrating against that partial registry would reject valid EMA/RSI state.
    if (!sessionId || !registry.length || indicatorHydratedSessionRef.current === sessionId) return;
    indicatorHydratedSessionRef.current = sessionId;
    setIndicatorDocument(indicatorRepository.load(sessionId, indicatorDefinitions));
    setIndicatorRuntime({}); chartRef.current?.clearIndicators();
  }, [indicatorDefinitions, indicatorRepository, registry.length, sessionId]);

  const [targetTimeframe, setTargetTimeframeState] = useState<string | undefined>(undefined);
  const prevSessionIdRef = useRef<number | null | undefined>(sessionId);

  useEffect(() => {
    const prevSession = prevSessionIdRef.current;
    if (prevSession !== sessionId) {
      prevSessionIdRef.current = sessionId;
      setIsPlaying(false);
      setNavigationPending(false);
      setTargetTimeframeState(undefined);
      if (prevSession !== null && prevSession !== undefined) {
        cleanupSessionQueries(queryClient, prevSession, {
          abortNavigation: () => { navCoordinator.cancelAll(sessionId); },
          cancelIndicators: () => indicatorRequests.cancelAll(),
          clearChart: () => {
            chartRef.current?.clearIndicators();
            chartRef.current?.clearPositionLines();
          },
        });
      } else {
        navCoordinator.cancelAll(sessionId);
      }
    }
  }, [indicatorRequests, navCoordinator, queryClient, sessionId]);

  const setTargetTimeframe = useCallback((nextTimeframe: string | undefined) => {
    navCoordinator.cancelAll(sessionId);
    indicatorRequests.cancelAll();
    chartRef.current?.clearIndicators();
    setNavigationPending(false);
    setTargetTimeframeState(nextTimeframe);
  }, [indicatorRequests, navCoordinator, sessionId]);

  const { data: sessionData, isError: isSessionError, refetch: refetchSession } = useQuery({
    queryKey: replayQueryKeys.session(sessionId),
    queryFn: () => getReplaySession(sessionId!),
    enabled: !!sessionId,
  });

  const sessionHasCandles =
    !sessionData ||
    (sessionData.has_candles !== false &&
      (sessionData.available_candles === undefined || sessionData.available_candles > 0));

  const effectiveTimeframe = normalizeReplayTimeframe(targetTimeframe ?? sessionData?.timeframe ?? '1D');
  const effectiveAdjustment = normalizeAdjustmentType(sessionData?.adjustment_type ?? 'unadjusted');
  const effectiveSymbol = sessionData?.symbol ?? undefined;
  const candlesQueryKey = useMemo(
    () => replayQueryKeys.candles(sessionId, effectiveTimeframe, effectiveAdjustment, effectiveSymbol),
    [effectiveAdjustment, effectiveSymbol, effectiveTimeframe, sessionId],
  );

  const { data: candlesData } = useQuery({
    queryKey: candlesQueryKey,
    queryFn: () => getSessionCandles(sessionId!, targetTimeframe),
    enabled: !!sessionId && sessionHasCandles,
  });

  const { data: practiceData, isLoading: practiceLoading, isError: practiceError, refetch: refetchPractice } = useQuery({
    queryKey: replayQueryKeys.practiceState(sessionId),
    queryFn: () => getPracticeState(sessionId!),
    enabled: !!sessionId && sessionHasCandles,
  });

  const { data: journalData, isLoading: journalLoading, isError: journalError, refetch: refetchJournal } = useQuery({
    queryKey: replayQueryKeys.journal(sessionId),
    queryFn: () => getJournalEntries(sessionId!),
    enabled: !!sessionId && sessionHasCandles,
  });

  const createMutation = useMutation({
    mutationFn: createReplaySession,
    onSuccess: (data) => {
      if (sessionId !== null) {
        cleanupSessionQueries(queryClient, sessionId, {
          abortNavigation: () => { navCoordinator.cancelAll(data.id); },
          cancelIndicators: () => indicatorRequests.cancelAll(),
          clearChart: () => {
            chartRef.current?.clearIndicators();
            chartRef.current?.clearPositionLines();
          },
        });
      }
      indicatorHydratedSessionRef.current = data.id;
      const empty = emptyIndicatorDocument(data.id); setIndicatorDocument(empty); indicatorRepository.save(empty); setIndicatorRuntime({});
      selectSession(data.id);
      toast.success(`Session #${data.id} created!`);
    },
    onError: (err: ApiError) => {
      toast.error(extractApiErrorMessage(err, 'Failed to create session'));
    }
  });

  const executeStepNavigation = useCallback(
    async (direction: 'next' | 'prev', steps: number) => {
      if (!sessionId) return;
      const ticket = navCoordinator.begin(sessionId);
      setNavigationPending(true);
      try {
        const stepFn = direction === 'next' ? nextCandle : previousCandle;
        const updatedSession = await stepFn(sessionId, steps, ticket.signal);
        if (!navCoordinator.isCurrent(ticket.token, sessionId)) {
          return;
        }
        if (updatedSession && typeof updatedSession.current_index === 'number') {
          queryClient.setQueryData(replayQueryKeys.session(sessionId), updatedSession);
          if (updatedSession.status?.toUpperCase() === 'COMPLETED') {
            setIsPlaying(false);
          }
        }
        const [freshCandles, freshPractice, freshJournal, freshSession] = await Promise.all([
          getSessionCandles(sessionId, targetTimeframe, ticket.signal),
          getPracticeState(sessionId, ticket.signal),
          getJournalEntries(sessionId, ticket.signal),
          updatedSession ? Promise.resolve(updatedSession) : getReplaySession(sessionId, ticket.signal),
        ]);
        if (!navCoordinator.isCurrent(ticket.token, sessionId)) {
          return;
        }
        const resolvedSession = freshSession ?? updatedSession ?? sessionData;
        const tfKey = normalizeReplayTimeframe(targetTimeframe ?? resolvedSession?.timeframe ?? '1D');
        const adjKey = normalizeAdjustmentType(resolvedSession?.adjustment_type ?? effectiveAdjustment);
        const symKey = resolvedSession?.symbol ?? effectiveSymbol;
        if (resolvedSession) {
          queryClient.setQueryData(replayQueryKeys.session(sessionId), resolvedSession);
        }
        queryClient.setQueryData(
          replayQueryKeys.candles(sessionId, tfKey, adjKey, symKey),
          dedupeCandlesByTimestamp(freshCandles ?? []),
        );
        if (freshPractice) {
          queryClient.setQueryData(replayQueryKeys.practiceState(sessionId), freshPractice);
        }
        if (freshJournal) {
          queryClient.setQueryData(replayQueryKeys.journal(sessionId), freshJournal);
        }
      } catch (err) {
        if (isAbortError(err) || !navCoordinator.isCurrent(ticket.token, sessionId)) {
          return;
        }
        if (direction === 'next') {
          setIsPlaying(false);
          toast.error('End of data or error');
        } else {
          toast.error('Start of data or error');
        }
      } finally {
        if (navCoordinator.token === ticket.token) {
          setNavigationPending(false);
        }
      }
    },
    [effectiveAdjustment, effectiveSymbol, navCoordinator, queryClient, sessionData, sessionId, targetTimeframe],
  );

  const handleCreateSession = useCallback((data: CreateSessionRequest) => {
    createMutation.mutate(data);
  }, [createMutation]);

  const handleResumeSession = useCallback((resumedId: number) => {
    if (sessionId !== null && sessionId !== resumedId) {
      setIsPlaying(false);
      cleanupSessionQueries(queryClient, sessionId, {
        abortNavigation: () => { navCoordinator.cancelAll(resumedId); },
        cancelIndicators: () => indicatorRequests.cancelAll(),
        clearChart: () => {
          chartRef.current?.clearIndicators();
          chartRef.current?.clearPositionLines();
        },
      });
    }
    selectSession(resumedId);
    toast.success(`Session #${resumedId} resumed`);
  }, [indicatorRequests, navCoordinator, queryClient, selectSession, sessionId]);

  const handleClearSession = useCallback(() => {
    setIsPlaying(false);
    setNavigationPending(false);
    setIndicatorDocument(emptyIndicatorDocument(1));
    setIndicatorRuntime({});
    cleanupSessionQueries(queryClient, sessionId, {
      abortNavigation: () => { navCoordinator.cancelAll(null); },
      cancelIndicators: () => indicatorRequests.cancelAll(),
      clearChart: () => {
        chartRef.current?.clearIndicators();
        chartRef.current?.clearPositionLines();
      },
    });
    clearSession();
  }, [clearSession, indicatorRequests, navCoordinator, queryClient, sessionId]);

  const handleSubmitDecision = useCallback(async (decision: DecisionCreate) => {
    if (!sessionId) return { ok: false, message: 'No replay session is active.' };
    try {
      await submitDecision(sessionId, decision);
      await Promise.all([refetchPractice(), refetchSession()]);

      const message = decision.action === 'HOLD' || decision.action === 'SKIP'
        ? `${decision.action} decision recorded without an order.`
        : decision.order_type === 'LIMIT'
          ? `${decision.action} LIMIT order recorded.`
          : decision.order_type === 'MARKET_NEXT_OPEN'
            ? `${decision.action} order queued for execution at next candle open.`
            : `${decision.action} executed at the visible close.`;
      toast.success(message);
      return { ok: true, message };
    } catch (err: unknown) {
      const message = extractApiErrorMessage(err, 'Transport error: decision was not confirmed. Retry when the local API is available.');
      toast.error(message);
      await refetchPractice();
      return { ok: false, message };
    }
  }, [sessionId, refetchPractice, refetchSession]);

  const handleSaveJournal = useCallback(async (entry: JournalEntryCreate) => {
    if (!sessionId) return { ok: false, message: 'No replay session is active.' };
    try {
      await createJournalEntry(sessionId, entry);
      await refetchJournal();
      return { ok: true, message: 'Checklist saved for the current replay context.' };
    } catch (err: unknown) {
      return { ok: false, message: extractApiErrorMessage(err, 'Transport error: journal draft was not saved.') };
    }
  }, [refetchJournal, sessionId]);

  const handleResetPractice = useCallback(async () => {
    if (!sessionId) return;
    try {
      await resetPracticeSession(sessionId);
      await Promise.all([refetchPractice(), refetchSession(), refetchJournal()]);
      chartRef.current?.clearPositionLines();
      toast.success('Đã reset bảng điểm và lệnh luyện tập');
    } catch {
      toast.error('Không thể reset trạng thái luyện tập');
    }
  }, [sessionId, refetchPractice, refetchSession, refetchJournal]);

  const handleCancelOrder = useCallback(async (orderId: number) => {
    if (!sessionId) return;
    try {
      await cancelPendingOrder(sessionId, orderId);
      await refetchPractice();
      toast.success(`Đã hủy lệnh chờ #${orderId}`);
    } catch (err: unknown) {
      toast.error(extractApiErrorMessage(err, 'Không thể hủy lệnh chờ'));
    }
  }, [sessionId, refetchPractice]);

  const symbolName = sessionData?.symbol || candlesData?.[0]?.symbol || '—';
  const currentCandle = candlesData?.length ? candlesData[candlesData.length - 1] : null;
  const sourceContext = sessionData?.source_context;
  const currentDate = toDateKey(currentCandle?.timestamp);
  const drawing = useDrawingWorkspaceController(sessionId ?? 1, symbolName);
  const selectedDrawing = drawing.document.drawings.find(item => drawing.selection.includes(item.id));
  const handleDrawingTool = useCallback((tool: DrawingTool) => {
    if (tool === 'select') chartRef.current?.cancelDrawing();
    drawing.setTool(tool);
  }, [drawing]);

  const resyncSessionCandles = useCallback(async () => {
    if (!sessionId) return;
    try {
      const [freshCandles, freshSession, freshPractice] = await Promise.all([
        getSessionCandles(sessionId, targetTimeframe),
        getReplaySession(sessionId),
        getPracticeState(sessionId),
      ]);
      if (prevSessionIdRef.current !== sessionId) return;
      const tfKey = normalizeReplayTimeframe(targetTimeframe ?? freshSession?.timeframe ?? '1D');
      const adjKey = normalizeAdjustmentType(freshSession?.adjustment_type ?? effectiveAdjustment);
      const symKey = freshSession?.symbol ?? effectiveSymbol;
      if (freshSession) {
        queryClient.setQueryData(
          replayQueryKeys.session(sessionId),
          (old: ReplaySession | undefined) => {
            if (!old) return freshSession;
            if (typeof old.current_index === 'number' && old.current_index > freshSession.current_index) {
              return old;
            }
            return freshSession;
          },
        );
      }
      if (freshCandles) {
        queryClient.setQueryData(
          replayQueryKeys.candles(sessionId, tfKey, adjKey, symKey),
          (old: Candle[] | undefined) => dedupeCandlesByTimestamp([...(old ?? []), ...freshCandles]),
        );
      }
      if (freshPractice) {
        queryClient.setQueryData(replayQueryKeys.practiceState(sessionId), freshPractice);
      }
    } catch {
      // Ignore transient fetch errors during reconnect resync
    }
  }, [effectiveAdjustment, effectiveSymbol, queryClient, sessionId, targetTimeframe]);

  const handleWebSocketMessage = useCallback((msg: WebSocketMessage) => {
    if (msg.session_id !== undefined && msg.session_id !== sessionId) {
      return;
    }

    if (msg.type === 'error') {
      const errMessage = String(msg.payload?.message ?? msg.data?.message ?? 'WebSocket playback error');
      toast.error(errMessage);
      return;
    }

    if (msg.type === 'completed') {
      setIsPlaying(false);
      const serverIndex =
        typeof msg.current_index === 'number'
          ? msg.current_index
          : typeof msg.payload?.current_index === 'number'
            ? (msg.payload.current_index as number)
            : undefined;
      queryClient.setQueryData(
        replayQueryKeys.session(sessionId),
        (old: ReplaySession | undefined) => {
          if (!old) return old;
          return {
            ...old,
            status: 'completed',
            current_index: serverIndex !== undefined ? serverIndex : old.current_index,
          };
        },
      );
      toast('Đã đến cuối dữ liệu phiên Replay');
      return;
    }

    if (msg.type === 'playback_state' || msg.type === 'state') {
      const echoedSpeed =
        typeof msg.speed_ms === 'number'
          ? msg.speed_ms
          : typeof msg.payload?.speed_ms === 'number'
            ? (msg.payload.speed_ms as number)
            : undefined;
      const hasExplicitSpeedEcho =
        msg.payload !== undefined && 'clamped' in msg.payload;
      if (echoedSpeed !== undefined && hasExplicitSpeedEcho) {
        playSpeedRef.current = echoedSpeed;
        setPlaySpeed(echoedSpeed);
      }
      if (msg.playback_state === 'COMPLETED') {
        setIsPlaying(false);
      }
      const serverIndex =
        typeof msg.current_index === 'number'
          ? msg.current_index
          : typeof msg.payload?.current_index === 'number'
            ? (msg.payload.current_index as number)
            : undefined;
      const serverContext = (msg.payload?.source_context as ReplaySourceContext | undefined) ?? msg.source_context;
      if (serverIndex !== undefined) {
        let indexChanged = false;
        queryClient.setQueryData(
          replayQueryKeys.session(sessionId),
          (old: ReplaySession | undefined) => {
            if (!old) return old;
            indexChanged = old.current_index !== serverIndex;
            return {
              ...old,
              ...(msg.playback_state === 'COMPLETED' ? { status: 'completed' } : {}),
              current_index: serverIndex,
              ...(serverContext ? { source_context: serverContext } : {}),
            };
          },
        );
        if (indexChanged) {
          void resyncSessionCandles();
        }
      }
      return;
    }

    if (msg.type === 'new_candle' || msg.type === 'tick') {
      const candlePayload = (msg.payload?.candle ?? msg.data) as unknown;
      if (!isWebSocketCandle(candlePayload)) return;
      const wsDateKey = unixSecondsToDateKey(candlePayload.time);
      const baseTimeframe = normalizeReplayTimeframe(sessionData?.timeframe ?? '1D');
      const baseAdjustment = normalizeAdjustmentType(sessionData?.adjustment_type ?? 'unadjusted');
      const baseSymbol = sessionData?.symbol ?? symbolName;
      const isViewingBaseTimeframe = effectiveTimeframe === baseTimeframe;

      // Update chart directly only when viewing the session's base timeframe
      if (chartRef.current && isViewingBaseTimeframe) {
        const chartCandle = {
          time: wsDateKey as Time,
          open: candlePayload.open,
          high: candlePayload.high,
          low: candlePayload.low,
          close: candlePayload.close,
        };
        chartRef.current.updateCandle(chartCandle, {
          time: wsDateKey as Time,
          value: candlePayload.volume,
          color: candlePayload.close >= candlePayload.open ? 'rgba(0, 230, 118, 0.5)' : 'rgba(255, 23, 68, 0.5)'
        });
      }

      // Sync replay session current_index from server authority (never blindly +1 when server echoes current_index)
      const serverIndex =
        typeof msg.current_index === 'number'
          ? msg.current_index
          : typeof msg.payload?.current_index === 'number'
            ? (msg.payload.current_index as number)
            : undefined;
      const serverContext = (msg.payload?.source_context as ReplaySourceContext | undefined) ?? msg.source_context;

      // Update react-query cache using the exact canonical key (never prefix ['candles', sessionId])
      const baseCandleQueryKey = replayQueryKeys.candles(sessionId, baseTimeframe, baseAdjustment, baseSymbol);
      let detectedCandleGap = false;
      queryClient.setQueryData(
        baseCandleQueryKey,
        (old: Candle[] | undefined) => {
          if (!old) return old;
          const dbCandle: Candle = {
            id: old.length ? Math.max(...old.map(c => c.id)) + 1 : 0,
            symbol: baseSymbol,
            timeframe: baseTimeframe,
            adjustment_type: baseAdjustment,
            timestamp: wsDateKey,
            open: candlePayload.open,
            high: candlePayload.high,
            low: candlePayload.low,
            close: candlePayload.close,
            volume: candlePayload.volume,
            source: 'websocket',
          };
          const merged = dedupeCandlesByTimestamp([...old, dbCandle]);
          if (serverIndex !== undefined && merged.length < serverIndex + 1) {
            detectedCandleGap = true;
          }
          return merged;
        }
      );

      if (!isViewingBaseTimeframe || detectedCandleGap) {
        void resyncSessionCandles();
      }

      if (msg.playback_state === 'COMPLETED') {
        setIsPlaying(false);
      }

      let detectedIndexGap = false;
      queryClient.setQueryData(
        replayQueryKeys.session(sessionId),
        (old: ReplaySession | undefined) => {
          if (!old) return old;
          if (serverIndex !== undefined && typeof old.current_index === 'number' && serverIndex > old.current_index + 1) {
            detectedIndexGap = true;
          }
          return {
            ...old,
            ...(msg.playback_state === 'COMPLETED' ? { status: 'completed' } : {}),
            current_index: serverIndex !== undefined ? serverIndex : old.current_index,
            ...(serverContext ? { source_context: serverContext } : {}),
          };
        }
      );

      if (detectedIndexGap && !detectedCandleGap) {
        void resyncSessionCandles();
      }

      // Sync positions and journal silently
      queryClient.invalidateQueries({ queryKey: replayQueryKeys.practiceState(sessionId) });
      queryClient.invalidateQueries({ queryKey: replayQueryKeys.journal(sessionId) });
    }
  }, [effectiveTimeframe, queryClient, resyncSessionCandles, sessionData?.adjustment_type, sessionData?.symbol, sessionData?.timeframe, sessionId, symbolName]);

  const { isConnected, status: wsStatus, reconnectAttempt: wsReconnectAttempt, sendCommand, closeSocket } = useWebSocket(
    sessionId,
    handleWebSocketMessage,
    { onReconnected: () => { void resyncSessionCandles(); } },
  );
  const handleNext = useCallback((steps: number = 1) => {
    if (sessionId) {
      void executeStepNavigation('next', steps);
    }
  }, [sessionId, executeStepNavigation]);

  const handlePrev = useCallback((steps: number = 1) => {
    if (sessionId) {
      void executeStepNavigation('prev', steps);
    }
  }, [sessionId, executeStepNavigation]);

  useEffect(() => {
    if (isSessionError) {
      toast.error('Saved replay session was not found');
      clearSession();
    }
  }, [isSessionError, clearSession]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!isGlobalShortcutEligible(e)) return;
      if (!sessionId) return;

      // Ctrl/Cmd-key combinations for drawing Undo / Redo (GAP-U-04)
      if ((e.ctrlKey || e.metaKey) && !e.altKey) {
        if (e.code === 'KeyZ' && !e.shiftKey) {
          if (drawing.canUndo) {
            e.preventDefault();
            drawing.undo();
            toast('Đã hoàn tác hình vẽ (Undo)', { duration: 1500 });
          }
          return;
        }
        if ((e.code === 'KeyZ' && e.shiftKey) || (e.code === 'KeyY' && !e.shiftKey)) {
          if (drawing.canRedo) {
            e.preventDefault();
            drawing.redo();
            toast('Đã làm lại hình vẽ (Redo)', { duration: 1500 });
          }
          return;
        }
      }

      // Alt-key combinations for drawing tools
      if (e.altKey && !e.ctrlKey && !e.metaKey) {
        if (e.code === 'KeyT') {
          e.preventDefault();
          handleDrawingTool('trendline');
          return;
        }
        if (e.code === 'KeyH') {
          e.preventDefault();
          handleDrawingTool('horizontal');
          return;
        }
        if (e.code === 'KeyR') {
          e.preventDefault();
          handleDrawingTool('risk-reward');
          return;
        }
      }

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
        case 'KeyB':
          e.preventDefault();
          window.dispatchEvent(new CustomEvent('sumi:switch-tab', { detail: 'trade' }));
          window.dispatchEvent(new CustomEvent('sumi:trade-action', { detail: 'BUY' }));
          break;
        case 'KeyS':
          e.preventDefault();
          window.dispatchEvent(new CustomEvent('sumi:switch-tab', { detail: 'trade' }));
          window.dispatchEvent(new CustomEvent('sumi:trade-action', { detail: 'SELL' }));
          break;
        case 'KeyH':
          e.preventDefault();
          window.dispatchEvent(new CustomEvent('sumi:switch-tab', { detail: 'trade' }));
          window.dispatchEvent(new CustomEvent('sumi:trade-action', { detail: 'HOLD' }));
          break;
        case 'KeyC':
          e.preventDefault();
          window.dispatchEvent(new CustomEvent('sumi:switch-tab', { detail: 'journal' }));
          break;
        case 'KeyD':
          e.preventDefault();
          window.dispatchEvent(new CustomEvent('sumi:open-debrief'));
          break;
        case 'Slash':
          if (e.shiftKey) {
            e.preventDefault();
            window.dispatchEvent(new CustomEvent('sumi:open-shortcuts'));
          } else {
            e.preventDefault();
            window.dispatchEvent(new CustomEvent('sumi:open-symbol-switcher'));
          }
          break;
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [drawing, handleDrawingTool, handleNext, handlePrev, sessionId]);

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

  const playSpeedRef = useRef(playSpeed);
  useEffect(() => {
    playSpeedRef.current = playSpeed;
  }, [playSpeed]);

  const handleSetPlaySpeed = useCallback((nextSpeed: number) => {
    playSpeedRef.current = nextSpeed;
    setPlaySpeed(nextSpeed);
    if (sessionId) {
      sendCommand('set_speed', { speed: nextSpeed });
    }
  }, [sendCommand, sessionId]);

  // Auto-play start/pause effect via WebSocket
  useEffect(() => {
    if (isPlaying && isConnected) {
      sendCommand('start', { speed: playSpeedRef.current });
    } else if (!isPlaying && isConnected) {
      sendCommand('pause');
    }
  }, [isPlaying, isConnected, sendCommand]);

  const commitIndicatorDocument = useCallback((next: IndicatorDocumentV1) => {
    indicatorRepository.save(next); setIndicatorDocument(next);
  }, [indicatorRepository]);

  const addIndicatorInstance = useCallback((definition: IndicatorDefinition, params: Record<string, unknown>) => {
    if (!sessionId) return;
    try {
      const instance = createIndicatorInstance(definition, params, indicatorDocument.instances.length);
      commitIndicatorDocument(addIndicator(indicatorDocument, instance));
    } catch (error) { toast.error(error instanceof Error ? error.message : 'Invalid indicator settings'); }
  }, [commitIndicatorDocument, indicatorDocument, sessionId]);

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
    if (!sessionId || indicatorDocument.sessionId !== sessionId) return;
    if (!currentCandle) {
      chartRef.current?.clearIndicators();
      return;
    }
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
          chartRef.current.updateIndicatorData({ instanceId: instance.id, paneId: instance.paneId, series, paneOrder });
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
      const baseIndex = sessionData?.current_index ?? candlesData?.length ?? 0;
      const workKey = replayQueryKeys.indicatorWorkKey({
        sessionId,
        symbol: effectiveSymbol,
        timeframe: effectiveTimeframe,
        adjustmentType: effectiveAdjustment,
        currentIndex: baseIndex,
        definitionId: instance.definitionId,
        params: instance.params,
      });
      void (async () => {
        let result: { stale: boolean; data?: ReplayIndicatorResponse | IndicatorDataPoint[] };
        try {
          result = await indicatorRequests.request(instance.id, workKey, signal => getSessionIndicatorResponse(sessionId!, instance.definitionId, instance.params, signal, targetTimeframe));
        } catch (error) {
          if (!effectActive || (error as { code?: string }).code === 'ERR_CANCELED' || (error as { name?: string }).name === 'AbortError') return;
          publishFailure(instance.id, 'transport', 'Data request failed — retry by showing or editing this indicator.');
          return;
        }
        if (!effectActive || result.stale || !result.data) return;
        const responsePayload = result.data;
        const responseData = Array.isArray(responsePayload) ? responsePayload : (responsePayload.data ?? []);
        const responseMeta = Array.isArray(responsePayload) ? undefined : responsePayload.meta;

        let mapped;
        try {
          mapped = IndicatorRenderRegistry.mapBackendDataWithResult(responseData, instance, responseMeta);
        } catch (error) {
          publishFailure(instance.id, 'mapping', `Indicator data could not be interpreted — the previous valid display was retained.${technicalDetail(error)}`);
          return;
        }
        if (mapped.status === 'UNMAPPED_COLUMN' || mapped.status === 'INVALID_SPEC') {
          publishFailure(
            instance.id,
            'mapping',
            mapped.message || 'Indicator data could not be interpreted — the previous valid display was retained.',
          );
          if (effectActive) {
            setIndicatorRuntime(previous => ({
              ...previous,
              [instance.id]: {
                ...previous[instance.id],
                status: 'error',
                errorKind: 'mapping',
                mappingStatus: mapped.status,
                error: mapped.message || 'Indicator column mapping missing.',
                values: previous[instance.id]?.values ?? {},
              },
            }));
          }
          return;
        }
        const series = mapped.series;
        if (!series.length || mapped.status === 'EMPTY_WARMING') {
          chartRef.current?.removeIndicator(instance.id);
          setIndicatorRuntime(previous => ({ ...previous, [instance.id]: {
            status: 'warming',
            mappingStatus: 'EMPTY_WARMING',
            warmupBars: mapped.warmup_bars,
            availableBars: mapped.available_bars,
            isProjectedClamped: mapped.is_projected_clamped,
            values: {},
            inputMaxDate: currentDate,
            responseMaxDate: toDateKey(responseData.at(-1)?.timestamp) || null,
            responseCount: responseData.length,
          } }));
          return;
        }
        try {
          if (!chartRef.current) throw new Error('Chart workspace is not mounted');
          chartRef.current.updateIndicatorData({ instanceId: instance.id, paneId: instance.paneId, series, paneOrder });
          chartRef.current.setIndicatorOrder(paneOrder);
        } catch (error) {
          publishFailure(instance.id, 'chart', `Chart rendering failed — the previous valid display was retained.${technicalDetail(error)}`);
          return;
        }
        if (effectActive) setIndicatorRuntime(previous => ({ ...previous, [instance.id]: {
          status: 'ready',
          mappingStatus: 'OK',
          warmupBars: mapped.warmup_bars,
          availableBars: mapped.available_bars,
          isProjectedClamped: mapped.is_projected_clamped,
          values: IndicatorRenderRegistry.currentValues(series),
          inputMaxDate: currentDate,
          responseMaxDate: toDateKey(responseData.at(-1)?.timestamp) || null,
          responseCount: responseData.length,
        } }));
      })();
    }
    return () => { effectActive = false; indicatorRequests.cancelAll(); };
  }, [candlesData?.length, currentCandle, currentDate, effectiveAdjustment, effectiveSymbol, effectiveTimeframe, indicatorDocument, indicatorRequests, sessionId, sessionData?.current_index, volumeData, targetTimeframe]);

  useEffect(() => {
    if (!practiceData) return;
    const openPosition = practiceData.positions.find(p => p.quantity > 0);
    if (openPosition) {
      const activeTrade = practiceData.trades.find(t => t.status === 'open') ?? null;
      chartRef.current?.setPositionLines(
        { average_price: openPosition.average_price },
        activeTrade ? { initial_stop_loss: activeTrade.initial_stop_loss, target_price: activeTrade.target_price } : null
      );
    } else {
      chartRef.current?.clearPositionLines();
    }
  }, [practiceData]);

  const handleQuickSwitchSymbol = useCallback((newSymbol: string) => {
    if (!newSymbol) return;
    createMutation.mutate({
      symbol: newSymbol.toUpperCase(),
      start_date: sessionData?.start_date ? String(sessionData.start_date).slice(0, 10) : '2023-01-01',
      end_date: sessionData?.end_date ? String(sessionData.end_date).slice(0, 10) : '2024-01-01',
      initial_cash: sessionData?.initial_cash || 100000000,
    });
  }, [createMutation, sessionData]);

  useEffect(() => () => indicatorRequests.cancelAll(), [indicatorRequests]);
  return {
    sessionId, chartRef, symbolName, sessionStatus: sessionData?.status, sessionData, sourceContext, currentDate, currentCandle, candleCount: candlesData?.length ?? 0,
    targetTimeframe, setTargetTimeframe,
    handleCreateSession, handleResumeSession, isCreatingSession: createMutation.isPending, handleClearSession, isValidating, invalidatedSession,
    onSwitchSymbol: handleQuickSwitchSymbol,
    indicatorDefinitions, indicatorDocument, indicatorRuntime, addIndicatorInstance, updateIndicatorInstance,
    removeIndicatorInstance, toggleIndicatorInstance, moveIndicatorInstance,
    playSpeed, setPlaySpeed: handleSetPlaySpeed, isPlaying, setIsPlaying,
    wsStatus, wsReconnectAttempt, isConnected, closeSocket,
    handlePrev, handleNext, navigationPending,
    drawing, selectedDrawing, handleDrawingTool, formattedCandles, volumeData, markers,
    practiceData, practiceLoading, practiceError, handleResetPractice, handleCancelOrder,
    journalData, journalLoading, journalError, handleSaveJournal, handleSubmitDecision,
  };
};
