import { StrictMode, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import {
  CandlestickSeries,
  ColorType,
  createChart,
  type CandlestickData,
  type IChartApi,
  type ISeriesApi,
  type Time,
} from 'lightweight-charts';
import {
  DrawingManager,
  getToolRegistry,
  type Anchor,
  type IDrawing,
  type SerializedDrawing,
} from 'lightweight-charts-drawing';
import './styles.css';

type Tool = 'trend-line' | 'horizontal-line' | 'ray' | 'rectangle' | 'fib-retracement' | 'text-annotation';
type Snapshot = SerializedDrawing[];

const tools: Array<{ id: Tool; label: string }> = [
  { id: 'horizontal-line', label: 'Horizontal' },
  { id: 'trend-line', label: 'Trendline' },
  { id: 'ray', label: 'Ray' },
  { id: 'rectangle', label: 'Rectangle' },
  { id: 'fib-retracement', label: 'Fibonacci Retracement' },
  { id: 'text-annotation', label: 'Text/Note' },
];

function candles(count = 100): CandlestickData<Time>[] {
  const rows: CandlestickData<Time>[] = [];
  let close = 100;
  for (let i = 0; i < count; i += 1) {
    const date = new Date(Date.UTC(2025, 0, 1 + i));
    const open = close;
    close = open + Math.sin(i / 5) * 1.1 + 0.18;
    rows.push({
      time: date.toISOString().slice(0, 10) as Time,
      open,
      high: Math.max(open, close) + 1.6,
      low: Math.min(open, close) - 1.4,
      close,
    });
  }
  return rows;
}

function createFromSnapshot(item: SerializedDrawing): IDrawing | null {
  const drawing = getToolRegistry().createDrawing(item.type, item.id, item.anchors, item.style, item.options);
  drawing?.fromJSON(item);
  return drawing;
}

function ChartSurface({
  activeTool,
  epoch,
  compact,
  onReady,
  onState,
  onCancel,
}: {
  activeTool: Tool | null;
  epoch: number;
  compact: boolean;
  onReady: (api: { manager: DrawingManager; chart: IChartApi; series: ISeriesApi<'Candlestick'> }) => void;
  onState: (state: Snapshot, event: string) => void;
  onCancel: () => void;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const toolRef = useRef(activeTool);
  const pendingRef = useRef<Anchor[]>([]);
  const counterRef = useRef(0);
  const onStateRef = useRef(onState);
  toolRef.current = activeTool;
  onStateRef.current = onState;

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const chart = createChart(container, {
      width: container.clientWidth,
      height: compact ? 430 : 650,
      layout: { background: { type: ColorType.Solid, color: '#111827' }, textColor: '#d1d5db' },
      grid: { vertLines: { color: '#1f2937' }, horzLines: { color: '#1f2937' } },
      timeScale: { timeVisible: true },
    });
    const series = chart.addSeries(CandlestickSeries, {
      upColor: '#22c55e', downColor: '#ef4444', borderVisible: false,
      wickUpColor: '#22c55e', wickDownColor: '#ef4444',
    });
    series.setData(candles());
    chart.timeScale().fitContent();
    const manager = new DrawingManager();
    manager.attach(chart, series, container);
    const emit = (event: string) => onStateRef.current(manager.exportDrawings(), event);
    const unsubscribers = [
      manager.on('drawing:added', () => emit('drawing:added')),
      manager.on('drawing:selected', () => emit('drawing:selected')),
      manager.on('drawing:updated', () => emit('drawing:updated')),
      manager.on('drawing:removed', () => emit('drawing:removed')),
      manager.on('drawing:cleared', () => emit('drawing:cleared')),
    ];
    const saved = localStorage.getItem('sumi-deepentropy-spike-v1');
    if (saved) {
      const parsed = JSON.parse(saved) as Snapshot;
      manager.importDrawings(parsed, (_type, item) => createFromSnapshot(item));
    }
    const click = (event: MouseEvent) => {
      const tool = toolRef.current;
      if (!tool) return;
      const rect = container.getBoundingClientRect();
      const time = chart.timeScale().coordinateToTime(event.clientX - rect.left);
      const price = series.coordinateToPrice(event.clientY - rect.top);
      if (time === null || price === null) return;
      pendingRef.current.push({ time, price });
      const required = getToolRegistry().get(tool)?.requiredAnchors ?? 2;
      if (pendingRef.current.length < required) return;
      const drawing = getToolRegistry().createDrawing(
        tool,
        `spike-${++counterRef.current}`,
        pendingRef.current.slice(),
        { lineColor: '#38bdf8', lineWidth: 2, fillColor: 'rgba(56,189,248,.18)', showLabels: true },
        tool === 'text-annotation' ? ({ text: 'Sumi note' } as never) : undefined,
      );
      pendingRef.current = [];
      if (drawing) {
        manager.addDrawing(drawing);
        manager.selectDrawing(drawing.id);
      }
    };
    const keydown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        pendingRef.current = [];
        onCancel();
        emit('ui:cancel');
      } else if ((event.key === 'Delete' || event.key === 'Backspace') && manager.getSelectedDrawing()) {
        manager.removeDrawing(manager.getSelectedDrawing()!.id);
      }
    };
    container.addEventListener('click', click);
    document.addEventListener('keydown', keydown);
    manager.setActiveTool(activeTool);
    onReady({ manager, chart, series });
    emit('mounted');
    return () => {
      container.removeEventListener('click', click);
      document.removeEventListener('keydown', keydown);
      unsubscribers.forEach(unsubscribe => unsubscribe());
      manager.detach();
      chart.remove();
      onStateRef.current([], 'unmounted');
    };
  }, [epoch, onCancel, onReady]);

  useEffect(() => {
    pendingRef.current = [];
  }, [activeTool]);

  return <div data-testid="chart" className="chart" ref={containerRef} />;
}

function App() {
  const [activeTool, setActiveTool] = useState<Tool | null>(null);
  const [mounted, setMounted] = useState(true);
  const [compact, setCompact] = useState(false);
  const [epoch, setEpoch] = useState(0);
  const [state, setState] = useState<Snapshot>([]);
  const [selected, setSelected] = useState('none');
  const [lastEvent, setLastEvent] = useState('boot');
  const apiRef = useRef<{ manager: DrawingManager; chart: IChartApi; series: ISeriesApi<'Candlestick'> } | null>(null);
  const historyRef = useRef<Snapshot[]>([]);
  const redoRef = useRef<Snapshot[]>([]);
  const restoringRef = useRef(false);

  const restore = useCallback((snapshot: Snapshot) => {
    const manager = apiRef.current?.manager;
    if (!manager) return;
    restoringRef.current = true;
    manager.clearAll();
    manager.importDrawings(snapshot, (_type, item) => createFromSnapshot(item));
    restoringRef.current = false;
    setState(manager.exportDrawings());
  }, []);

  const onState = useCallback((next: Snapshot, event: string) => {
    setState(next);
    setLastEvent(event);
    const manager = apiRef.current?.manager;
    setSelected(manager?.getSelectedDrawing()?.id ?? 'none');
    if (!restoringRef.current && ['drawing:added', 'drawing:updated', 'drawing:removed'].includes(event)) {
      historyRef.current.push(next);
      redoRef.current = [];
    }
  }, []);

  const onReady = useCallback((api: { manager: DrawingManager; chart: IChartApi; series: ISeriesApi<'Candlestick'> }) => {
    apiRef.current = api;
  }, []);
  const onCancel = useCallback(() => setActiveTool(null), []);

  useEffect(() => {
    apiRef.current?.manager.setActiveTool(activeTool);
  }, [activeTool]);

  const selectedDrawing = apiRef.current?.manager.getSelectedDrawing();
  const invoke = (action: () => void, event: string) => {
    action();
    const manager = apiRef.current?.manager;
    if (manager) onState(manager.exportDrawings(), event);
  };
  const save = () => localStorage.setItem('sumi-deepentropy-spike-v1', JSON.stringify(apiRef.current?.manager.exportDrawings() ?? []));
  const undo = () => {
    if (historyRef.current.length < 2) return;
    const current = historyRef.current.pop();
    if (current) redoRef.current.push(current);
    restore(historyRef.current.at(-1) ?? []);
  };
  const redo = () => {
    const next = redoRef.current.pop();
    if (!next) return;
    historyRef.current.push(next);
    restore(next);
  };

  const serialized = useMemo(() => JSON.stringify(state, null, 2), [state]);
  return (
    <main>
      <header><h1>Sumi / deepentropy isolated spike</h1><span>revision 5f2afc335028</span></header>
      <section className="toolbar" aria-label="Drawing tools">
        <button data-testid="tool-cursor" className={activeTool === null ? 'active' : ''} onClick={() => setActiveTool(null)}>Cursor/Select</button>
        {tools.map(tool => <button key={tool.id} data-testid={`tool-${tool.id}`} className={activeTool === tool.id ? 'active' : ''} onClick={() => setActiveTool(tool.id)}>{tool.label}</button>)}
      </section>
      <section className="actions">
        <button data-testid="delete-selected" disabled={!selectedDrawing} onClick={() => invoke(() => selectedDrawing && apiRef.current?.manager.removeDrawing(selectedDrawing.id), 'ui:delete')}>Delete selected</button>
        <button data-testid="clear-all" onClick={() => invoke(() => apiRef.current?.manager.clearAll(), 'ui:clear')}>Clear all</button>
        <button data-testid="undo" onClick={undo}>Undo</button><button data-testid="redo" onClick={redo}>Redo</button>
        <button data-testid="save" onClick={save}>Persist</button><button data-testid="reload" onClick={() => window.location.reload()}>Reload</button>
        <button data-testid="pan" onClick={() => apiRef.current?.chart.timeScale().scrollToPosition(8, false)}>Pan</button>
        <button data-testid="zoom" onClick={() => apiRef.current?.chart.timeScale().applyOptions({ barSpacing: 12 })}>Zoom</button>
        <button data-testid="advance" onClick={() => apiRef.current?.series.update(candles(101).at(-1)!)}>Replay advance</button>
        <button data-testid="resize" onClick={() => setCompact(value => { const next = !value; apiRef.current?.chart.applyOptions({ height: next ? 430 : 650 }); return next; })}>Resize</button>
        <button data-testid="mount-toggle" onClick={() => { setMounted(value => !value); setEpoch(value => value + 1); }}>Mount/unmount</button>
      </section>
      <div className="status" data-testid="status">tool={activeTool ?? 'cursor'} count={state.length} selected={selected} event={lastEvent}</div>
      {mounted ? <ChartSurface activeTool={activeTool} compact={compact} epoch={epoch} onReady={onReady} onState={onState} onCancel={onCancel} /> : <div data-testid="unmounted" className="unmounted">Chart unmounted</div>}
      <pre data-testid="serialized">{serialized}</pre>
    </main>
  );
}

createRoot(document.getElementById('root')!).render(<StrictMode><App /></StrictMode>);
