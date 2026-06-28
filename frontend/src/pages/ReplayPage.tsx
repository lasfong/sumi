import React, { useEffect, useRef, useState, useCallback } from 'react';
import { useQuery, useMutation } from '@tanstack/react-query';
import { CandleChart } from '../components/chart/CandleChart';
import type { CandleChartRef, DrawingLine, DrawingType } from '../components/chart/CandleChart';
import type { SeriesMarker, Time, SeriesMarkerPosition, SeriesMarkerShape } from 'lightweight-charts';
import { TradeControls } from '../components/replay/TradeControls';
import { PositionPanel } from '../components/replay/PositionPanel';
import { DecisionJournal } from '../components/replay/DecisionJournal';
import { PendingOrdersPanel } from '../components/replay/PendingOrdersPanel';
import { SessionSetup } from '../components/replay/SessionSetup';
import { DrawingToolbar } from '../components/chart/DrawingToolbar';
import { createReplaySession, getSessionCandles, nextCandle, previousCandle, getDrawings, updateDrawings } from '../api/replayApi';
import { submitDecision, getPositions, getDecisions, getOrders } from '../api/decisionApi';
import { getSessionIndicatorData } from '../api/indicatorsApi';
import { MultiChartLayout } from '../components/layout/MultiChartLayout';
import { useReplayStore } from '../store/replayStore';
import toast from 'react-hot-toast';
import type { Candle, ChartCandle, ChartVolume, DecisionCreate, CreateSessionRequest, Decision } from '../types';
import { IndicatorSelector } from '../components/chart/IndicatorSelector';
import type { IndicatorConfig } from '../components/chart/IndicatorSelector';
import type { IndicatorSeriesData } from '../components/chart/CandleChart';
import { useWebSocket } from '../hooks/useWebSocket';
import type { WebSocketMessage } from '../hooks/useWebSocket';
import { useQueryClient } from '@tanstack/react-query';
import type { IndicatorDataPoint } from '../api/indicatorsApi';

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

const parseDrawings = (stateData?: string): DrawingLine[] => {
  if (!stateData) return [];
  try {
    const parsed = JSON.parse(stateData) as unknown;
    return Array.isArray(parsed) ? parsed as DrawingLine[] : [];
  } catch {
    return [];
  }
};

const isWebSocketCandle = (data: unknown): data is WebSocketCandle => {
  if (!data || typeof data !== 'object') return false;
  const candidate = data as Record<string, unknown>;
  return ['time', 'open', 'high', 'low', 'close', 'volume'].every(key => typeof candidate[key] === 'number');
};

export const ReplayPage: React.FC = () => {
  const store = useReplayStore();
  const chartRef = useRef<CandleChartRef>(null);
  
  const [isPlaying, setIsPlaying] = useState(false);
  const [playSpeed, setPlaySpeed] = useState(500); // ms per candle
  
  const [activeIndicators, setActiveIndicators] = useState<IndicatorConfig[]>([]);

  const [activeTool, setActiveTool] = useState<DrawingType>('cursor');
  const [localDrawings, setLocalDrawings] = useState<DrawingLine[] | null>(null);

  const createMutation = useMutation({
    mutationFn: createReplaySession,
    onSuccess: (data) => {
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

  const { data: positionData, refetch: refetchPosition } = useQuery({
    queryKey: ['position', store.sessionId],
    queryFn: () => getPositions(store.sessionId!),
    enabled: !!store.sessionId,
  });

  const { data: decisionsData, refetch: refetchDecisions } = useQuery({
    queryKey: ['decisions', store.sessionId],
    queryFn: () => getDecisions(store.sessionId!),
    enabled: !!store.sessionId,
  });

  const { data: ordersData, refetch: refetchOrders } = useQuery({
    queryKey: ['orders', store.sessionId],
    queryFn: () => getOrders(store.sessionId!),
    enabled: !!store.sessionId,
  });

  const { data: drawingsData } = useQuery({
    queryKey: ['drawings', store.sessionId],
    queryFn: () => getDrawings(store.sessionId!),
    enabled: !!store.sessionId,
  });

  const saveDrawingsMutation = useMutation({
    mutationFn: (newDrawings: DrawingLine[]) => updateDrawings(store.sessionId!, JSON.stringify(newDrawings)),
  });

  const persistedDrawings = parseDrawings(drawingsData?.state_data);
  const drawings = localDrawings ?? persistedDrawings;

  const handleDrawingComplete = (d: DrawingLine) => {
    setLocalDrawings(prev => {
      const base = prev ?? persistedDrawings;
      const next = [...base, d];
      saveDrawingsMutation.mutate(next);
      return next;
    });
  };

  const handleClearDrawings = () => {
    setLocalDrawings([]);
    saveDrawingsMutation.mutate([]);
  };

  const nextMutation = useMutation({
    mutationFn: (steps: number) => nextCandle(store.sessionId!, steps),
    onSuccess: () => {
      refetchCandles();
      refetchPosition();
      refetchOrders();
    },
    onError: () => {
      setIsPlaying(false);
      toast.error('End of data or error');
    },
  });

  const prevMutation = useMutation({
    mutationFn: (steps: number) => previousCandle(store.sessionId!, steps),
    onSuccess: () => {
      refetchCandles();
      refetchPosition();
      refetchOrders();
    },
    onError: () => toast.error('Start of data or error'),
  });

  const handleCreateSession = useCallback((data: CreateSessionRequest) => {
    createMutation.mutate(data);
  }, [createMutation]);

  const handleSubmitDecision = useCallback(async (decision: DecisionCreate) => {
    if (!store.sessionId) return;
    try {
      await submitDecision(store.sessionId, decision);
      refetchPosition();
      refetchDecisions();
      refetchOrders();
      
      if (decision.action === 'BUY' || decision.action === 'ADD') toast.success(`Bought ${decision.quantity || 0} shares`);
      else if (decision.action === 'SELL' || decision.action === 'REDUCE') toast.success(`Sold ${decision.quantity || 0} shares`);
      else if (decision.action === 'CLOSE') toast.success(`Position closed`);
      else if (decision.action === 'CUT_LOSS') toast.error(`Cut loss executed`);
      else if (decision.action === 'TAKE_PROFIT') toast.success(`Take profit executed`);
      else toast.success(`Decision logged: ${decision.action}`);
    } catch (err: unknown) {
      const apiError = err as ApiError;
      toast.error(apiError?.response?.data?.detail || 'Failed to submit decision');
    }
  }, [store.sessionId, refetchPosition, refetchDecisions, refetchOrders]);


  const symbolName = candlesData?.[0]?.symbol || '—';
  const currentCandle = candlesData?.length ? candlesData[candlesData.length - 1] : null;

  const queryClient = useQueryClient();

  const handleWebSocketMessage = useCallback((msg: WebSocketMessage) => {
    if (msg.type === 'new_candle') {
      const newCandle = msg.data;
      if (!isWebSocketCandle(newCandle)) return;
      
      // Update chart directly for smoothness
      if (chartRef.current) {
        const chartCandle = {
          time: newCandle.time as Time,
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
          timestamp: new Date(newCandle.time * 1000).toISOString(),
          open: newCandle.open,
          high: newCandle.high,
          low: newCandle.low,
          close: newCandle.close,
          volume: newCandle.volume
        };
        return [...old, dbCandle];
      });
      
      // Sync positions silently
      queryClient.invalidateQueries({ queryKey: ['position', store.sessionId] });
      queryClient.invalidateQueries({ queryKey: ['orders', store.sessionId] });
    }
  }, [store.sessionId, queryClient, symbolName]);

  const { isConnected, sendCommand } = useWebSocket(store.sessionId, handleWebSocketMessage);
  const handleNext = useCallback((steps: number = 1) => {
    if (store.sessionId) {
      if (isConnected && steps === 1) {
        sendCommand('next');
      } else {
        nextMutation.mutate(steps);
      }
    }
  }, [store.sessionId, nextMutation, isConnected, sendCommand]);

  const handlePrev = useCallback((steps: number = 1) => {
    if (store.sessionId) {
      prevMutation.mutate(steps);
    }
  }, [store.sessionId, prevMutation]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (document.activeElement?.tagName === 'INPUT' || document.activeElement?.tagName === 'TEXTAREA' || document.activeElement?.tagName === 'SELECT') return;
      if (!store.sessionId) return;

      switch (e.code) {
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
  }, [handleNext, handlePrev, store.sessionId]);

  const formattedCandles: ChartCandle[] = (candlesData || []).map((c: Candle) => ({
    time: c.timestamp,
    open: c.open,
    high: c.high,
    low: c.low,
    close: c.close,
  }));

  const volumeData: ChartVolume[] = (candlesData || []).map((c: Candle) => ({
    time: c.timestamp,
    value: c.volume,
    color: c.close >= c.open ? 'rgba(0, 230, 118, 0.5)' : 'rgba(255, 23, 68, 0.5)',
  }));

  const markers: SeriesMarker<Time>[] = (decisionsData || [])
    .filter((d: Decision) => ['BUY', 'ADD', 'SELL', 'REDUCE', 'CLOSE', 'CUT_LOSS', 'TAKE_PROFIT'].includes(d.action))
    .map((d: Decision) => {
      const isBuy = d.action === 'BUY' || d.action === 'ADD';
      const dateStr = typeof d.decision_date === 'string' ? d.decision_date.split('T')[0] : String(d.decision_date);
      return {
        time: dateStr as Time,
        position: (isBuy ? 'belowBar' : 'aboveBar') as SeriesMarkerPosition,
        color: isBuy ? '#00E676' : '#FF1744',
        shape: (isBuy ? 'arrowUp' : 'arrowDown') as SeriesMarkerShape,
        text: d.action,
      };
    })
    .sort((a, b) => (a.time as string).localeCompare(b.time as string));

  const [isLoadingIndicator, setIsLoadingIndicator] = useState(false);


  // Auto-play effect via WebSocket
  useEffect(() => {
    if (isPlaying && isConnected) {
      sendCommand('start', { speed: playSpeed });
    } else if (!isPlaying && isConnected) {
      sendCommand('pause');
    }
  }, [isPlaying, playSpeed, isConnected, sendCommand]);

  const loadIndicator = async (config: IndicatorConfig) => {
    if (!store.sessionId) return;
    setIsLoadingIndicator(true);
    try {
      if (!activeIndicators.some(i => `${i.name}_${JSON.stringify(i.params)}` === `${config.name}_${JSON.stringify(config.params)}`)) {
        setActiveIndicators(prev => [...prev, config]);
      }
    } catch (error) {
      console.error(error);
      toast.error(`Failed to load ${config.name}`);
    } finally {
      setIsLoadingIndicator(false);
    }
  };

  const handleClearIndicators = () => {
    setActiveIndicators([]);
    chartRef.current?.clearIndicators();
  };

  // Sync indicators with current candle data
  useEffect(() => {
    if (!currentCandle || !store.sessionId || activeIndicators.length === 0) return;

    const fetchActiveIndicators = async () => {
      for (const config of activeIndicators) {
        try {
          // Fetch session-scoped indicator data (no future leak)
        const data = await getSessionIndicatorData(store.sessionId!, config.name, config.params);
          if (data.length === 0) continue;

          const seriesList: IndicatorSeriesData[] = [];
          const keys = Object.keys(data[0]).filter(k => k !== 'timestamp');

      keys.forEach(k => {
        const lineData = data.map((d: IndicatorDataPoint) => ({
          time: d.timestamp.split('T')[0] as Time,
          value: Number(d[k]) || 0
        })).filter((d) => !isNaN(d.value));

        let color = config.color || '#2962FF';
        let type: 'line' | 'histogram' = 'line';

        const lk = k.toLowerCase();
        
        // MACD
        if (lk.startsWith('macd') && !lk.includes('h') && !lk.includes('s')) color = '#2962FF';
        if (lk.startsWith('macds')) color = '#FF6D00';
        if (lk.startsWith('macdh')) { color = 'rgba(0, 230, 118, 0.5)'; type = 'histogram'; }
        
        // BBANDS
        if (lk.startsWith('bbl') || lk.startsWith('bbu')) color = 'rgba(41, 98, 255, 0.5)';
        if (lk.startsWith('bbm')) color = '#FF6D00';

        // ICHIMOKU
        if (lk.startsWith('isa')) color = 'rgba(0, 230, 118, 0.5)';
        if (lk.startsWith('isb')) color = 'rgba(255, 23, 68, 0.5)';
        if (lk.startsWith('its')) color = '#2962FF';
        if (lk.startsWith('iks')) color = '#FF1744';
        if (lk.startsWith('ics')) color = '#00E676';
        
        // ADX
        if (lk.startsWith('adx')) color = '#F0F6FC';
        if (lk.startsWith('dmp')) color = '#00E676';
        if (lk.startsWith('dmn')) color = '#FF1744';
        
        // SUPERTREND
        if (lk.startsWith('supert_')) color = '#00E676'; // actually supertrend changes color, but simple line is ok
        if (lk.startsWith('supertd') || lk.startsWith('supertl') || lk.startsWith('superts')) return; // skip these

        // STOCH
        if (lk.startsWith('stochk')) color = '#2962FF';
        if (lk.startsWith('stochd')) color = '#FF6D00';

        seriesList.push({ name: k, data: lineData, color, type });
      });

      const key = `${config.name}_${JSON.stringify(config.params)}`;
      chartRef.current?.addIndicator(key, seriesList, config.pane);
    } catch (error) {
      console.error("Failed to fetch indicator:", error);
    }
  }
};

fetchActiveIndicators();
}, [activeIndicators, candlesData, currentCandle, store.sessionId]);

  if (!store.sessionId) {
    return <SessionSetup onCreateSession={handleCreateSession} isLoading={createMutation.isPending} />;
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', background: 'var(--bg-dark)' }}>
      <header style={{
        padding: '12px 20px', background: 'var(--bg-header)', backdropFilter: 'var(--backdrop-blur)',
        display: 'flex', justifyContent: 'space-between', alignItems: 'center',
        borderBottom: '1px solid var(--border-color)',
        boxShadow: '0 4px 20px rgba(0,0,0,0.5)', zIndex: 10
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
          <h2 style={{ margin: 0, fontSize: '18px', fontWeight: 600 }}>Sumi Replay</h2>
          <span style={{ padding: '4px 8px', background: 'rgba(41, 98, 255, 0.1)', color: 'var(--color-primary)', borderRadius: '4px', fontWeight: 600, fontSize: '14px', border: '1px solid rgba(41, 98, 255, 0.3)' }}>
            {symbolName}
          </span>
          <span style={{ color: 'var(--text-muted)', fontSize: '13px' }}>Session #{store.sessionId}</span>
          
          {currentCandle && (
            <div style={{ display: 'flex', gap: '12px', marginLeft: '16px', fontSize: '13px', fontFamily: 'monospace' }}>
              <span style={{ color: 'var(--text-muted)' }}>Bar: <span style={{ color: 'white' }}>#{candlesData?.length || 0}</span></span>
              <span style={{ color: 'var(--text-muted)' }}>O: <span style={{ color: 'white' }}>{currentCandle.open.toLocaleString()}</span></span>
              <span style={{ color: 'var(--text-muted)' }}>H: <span style={{ color: 'white' }}>{currentCandle.high.toLocaleString()}</span></span>
              <span style={{ color: 'var(--text-muted)' }}>L: <span style={{ color: 'white' }}>{currentCandle.low.toLocaleString()}</span></span>
              <span style={{ color: 'var(--text-muted)' }}>C: <span style={{ color: currentCandle.close >= currentCandle.open ? 'var(--color-buy)' : 'var(--color-sell)', fontWeight: 600 }}>{currentCandle.close.toLocaleString()}</span></span>
            </div>
          )}
        </div>

        <div style={{ display: 'flex', gap: '16px', alignItems: 'center' }}>
          <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
            <IndicatorSelector onAddIndicator={loadIndicator} onClear={handleClearIndicators} />
            {isLoadingIndicator && <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>Loading...</span>}
          </div>

          <div style={{ width: '1px', height: '24px', background: 'var(--border-color)' }}></div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <select 
              value={playSpeed} 
              onChange={(e) => setPlaySpeed(Number(e.target.value))}
              style={{ background: 'var(--bg-panel)', color: 'white', border: '1px solid var(--border-color)', borderRadius: '4px', padding: '4px', fontSize: '12px' }}
            >
              <option value={1000}>1x</option>
              <option value={500}>2x</option>
              <option value={200}>5x</option>
              <option value={100}>10x</option>
            </select>

            <button 
              onClick={() => setIsPlaying(!isPlaying)}
              style={{ 
                background: isPlaying ? 'rgba(255, 23, 68, 0.2)' : 'rgba(0, 230, 118, 0.2)', 
                color: isPlaying ? '#FF1744' : '#00E676', 
                border: `1px solid ${isPlaying ? '#FF1744' : '#00E676'}`, 
                fontSize: '12px', padding: '6px 12px', display: 'flex', alignItems: 'center', gap: '4px' 
              }}
            >
              {isPlaying ? (
                <><svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><path d="M6 4h4v16H6zm8 0h4v16h-4z"/></svg> Pause</>
              ) : (
                <><svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg> Auto-Play</>
              )}
            </button>
          </div>

          <div style={{ width: '1px', height: '24px', background: 'var(--border-color)' }}></div>

          <div style={{ display: 'flex', border: '1px solid var(--border-color)', borderRadius: '6px', overflow: 'hidden' }}>
            <button onClick={() => handlePrev(5)} disabled={prevMutation.isPending || isPlaying} style={{ background: 'var(--bg-panel)', color: 'white', fontSize: '13px', padding: '6px 12px', border: 'none', borderRight: '1px solid var(--border-color)', borderRadius: 0 }}>-5</button>
            <button onClick={() => handlePrev(1)} disabled={prevMutation.isPending || isPlaying} style={{ background: 'var(--bg-panel)', color: 'white', fontSize: '13px', padding: '6px 12px', border: 'none', borderRadius: 0 }}>← Prev</button>
          </div>
          <div style={{ display: 'flex', border: '1px solid var(--color-primary)', borderRadius: '6px', overflow: 'hidden', boxShadow: '0 0 10px rgba(41,98,255,0.2)' }}>
            <button onClick={() => handleNext(1)} disabled={nextMutation.isPending || isPlaying} style={{ background: 'var(--color-primary)', color: 'white', fontSize: '13px', padding: '6px 12px', border: 'none', borderRight: '1px solid rgba(255,255,255,0.2)', borderRadius: 0 }}>Next →</button>
            <button onClick={() => handleNext(5)} disabled={nextMutation.isPending || isPlaying} style={{ background: 'var(--color-primary)', color: 'white', fontSize: '13px', padding: '6px 12px', border: 'none', borderRadius: 0 }}>+5</button>
          </div>
        </div>
      </header>

      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        <DrawingToolbar 
          activeTool={activeTool} 
          onSelectTool={setActiveTool} 
          onClearAll={handleClearDrawings} 
        />
        
        <main style={{ flex: 3, padding: '0.5rem', display: 'flex', flexDirection: 'column' }}>
          <div className="panel" style={{ flex: 1, padding: 0, overflow: 'hidden' }}>
            <MultiChartLayout layoutType="1x1">
              <CandleChart 
                ref={chartRef} 
                data={formattedCandles} 
                volumeData={volumeData} 
                markers={markers} 
                activeTool={activeTool}
                drawings={drawings}
                onDrawingComplete={handleDrawingComplete}
              />
            </MultiChartLayout>
          </div>
        </main>

        <aside style={{ flex: 1, padding: '0.5rem', display: 'flex', flexDirection: 'column', gap: '0.5rem', borderLeft: '1px solid var(--border-color)', minWidth: '280px', maxWidth: '360px' }}>
          <TradeControls
            sessionId={store.sessionId}
            onSubmitDecision={handleSubmitDecision}
          />
          <PendingOrdersPanel orders={ordersData || []} />
          <PositionPanel positions={positionData || []} />
          <DecisionJournal decisions={decisionsData || []} />
        </aside>
      </div>
    </div>
  );
};
