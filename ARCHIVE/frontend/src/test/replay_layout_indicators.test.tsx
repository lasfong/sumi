import { describe, expect, it, vi } from 'vitest';
import React from 'react';
import { render, screen, renderHook, act, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { Candle, PracticeWorkflowSnapshot } from '../types';
import { IchimokuCloudPlugin } from '../components/chart/plugins/IchimokuCloudPlugin';
import { ChartLegendOverlay } from '../components/chart/ChartLegendOverlay';
import { ReplayHeader } from '../components/replay/ReplayHeader';
import { ReplayWorkspace } from '../components/replay/ReplayWorkspace';
import { ReplayRightPanel } from '../components/replay/ReplayRightPanel';
import { useReplayWorkspaceController } from '../components/replay/ReplayWorkspaceController';
import { isGlobalShortcutEligible } from '../features/replay/globalShortcutPolicy';

vi.mock('../components/chart/CandleChart', () => ({
  CandleChart: () => <div data-testid="mock-candle-chart">CandleChart</div>,
}));

// Mock child components in ReplayRightPanel that make external queries
vi.mock('../components/signals/SignalInspector', () => ({
  SignalInspector: ({ currentTimestamp, currentIndex }: any) => (
    <div data-testid="mock-signal-inspector" data-timestamp={currentTimestamp} data-index={currentIndex}>
      SignalInspector Timestamp: {currentTimestamp}
    </div>
  ),
}));

vi.mock('../components/signals/SignalExplanationInspector', () => ({
  SignalExplanationInspector: ({ currentTimestamp, currentIndex }: any) => (
    <div data-testid="mock-signal-explanation-inspector" data-timestamp={currentTimestamp} data-index={currentIndex}>
      ExplanationInspector Timestamp: {currentTimestamp}
    </div>
  ),
}));

vi.mock('../api/replayApi', async () => {
  const actual = await vi.importActual<typeof import('../api/replayApi')>('../api/replayApi');
  return {
    ...actual,
    getReplaySession: vi.fn().mockResolvedValue({
      id: 42,
      symbol: 'FPT',
      timeframe: '1D',
      start_date: '2024-01-01',
      end_date: '2024-06-01',
      current_index: 15,
      status: 'active',
      mode: 'practice',
      has_candles: true,
      available_candles: 100,
    }),
  };
});

vi.mock('../api/indicatorsApi', () => ({
  getIndicatorRegistry: vi.fn().mockResolvedValue([]),
}));

vi.mock('../components/chart/DrawingInspector', () => ({
  DrawingInspector: () => <div>DrawingInspector</div>,
}));
vi.mock('../components/replay/DecisionJournal', () => ({
  DecisionJournal: () => <div>DecisionJournal</div>,
}));
vi.mock('../components/replay/PendingOrdersPanel', () => ({
  PendingOrdersPanel: () => <div>PendingOrdersPanel</div>,
}));
vi.mock('../components/replay/PositionPanel', () => ({
  PositionPanel: () => <div>PositionPanel</div>,
}));
vi.mock('../components/replay/PracticeJournal', () => ({
  PracticeJournal: () => <div>PracticeJournal</div>,
}));
vi.mock('../components/replay/PracticeRail', () => ({
  PracticeRail: ({ trade }: any) => <div data-testid="practice-rail">{trade}</div>,
}));
vi.mock('../components/replay/PracticeScoreboard', () => ({
  PracticeScoreboard: () => <div>PracticeScoreboard</div>,
}));
vi.mock('../components/replay/ScannerSourceContext', () => ({
  ScannerSourceContext: () => <div>ScannerSourceContext</div>,
}));
vi.mock('../components/replay/TradeControls', () => ({
  TradeControls: () => <div>TradeControls</div>,
}));

describe('Workstream B Remediation — Replay Chart & Indicators', () => {
  describe('SQA-012: Ichimoku Cloud Autoscale & Whitespace Isolation', () => {
    const plugin = new IchimokuCloudPlugin();

    it('identifies incomplete Span A or Span B warmup bars as whitespace', () => {
      // Both null
      expect(plugin.isWhitespace({ time: '2024-01-01', spanA: null, spanB: null })).toBe(true);
      // Span A present (after 26 bars), Span B null (needs 52 bars)
      expect(plugin.isWhitespace({ time: '2024-01-02', spanA: 95000, spanB: null })).toBe(true);
      // Span A null, Span B present
      expect(plugin.isWhitespace({ time: '2024-01-03', spanA: null, spanB: 92000 })).toBe(true);
      // Missing fields
      expect(plugin.isWhitespace({ time: '2024-01-04' })).toBe(true);
      // NaN values
      expect(plugin.isWhitespace({ time: '2024-01-05', spanA: NaN, spanB: 95000 })).toBe(true);
      expect(plugin.isWhitespace({ time: '2024-01-06', spanA: 95000, spanB: NaN })).toBe(true);
    });

    it('identifies complete numeric bars as non-whitespace data', () => {
      expect(plugin.isWhitespace({ time: '2024-01-07', spanA: 95000, spanB: 93000 })).toBe(false);
    });

    it('returns empty array or finite values only from priceValueBuilder (never injects 0)', () => {
      // Missing both bounds
      const empty = plugin.priceValueBuilder({ time: '2024-01-01', spanA: null, spanB: null });
      expect(empty).toEqual([]);
      expect(empty).not.toContain(0);

      // Single bound
      const single = plugin.priceValueBuilder({ time: '2024-01-02', spanA: 102000, spanB: null });
      expect(single).toEqual([102000, 102000, 102000]);
      expect(single).not.toContain(0);

      // Both bounds
      const full = plugin.priceValueBuilder({ time: '2024-01-03', spanA: 104000, spanB: 99000 });
      expect(full).toEqual([104000, 99000, 104000]);
      expect(full).not.toContain(0);
    });
  });

  describe('SQA-002: Unit Badge Correctness', () => {
    it('displays VND unit badge and does NOT display misleading x1,000 badge', () => {
      const candle: Candle = {
        id: 1,
        symbol: 'FPT',
        timeframe: '1D',
        source: 'doraemon',
        adjustment_type: 'unadjusted',
        timestamp: '2024-02-06T00:00:00',
        open: 99511.79,
        high: 101000.0,
        low: 99000.0,
        close: 100500.0,
        volume: 1250000,
      };

      render(
        <ChartLegendOverlay
          symbol="FPT"
          timeframe="1D"
          currentCandle={candle}
          indicators={[]}
          runtime={{}}
          onToggle={vi.fn()}
          onSettings={vi.fn()}
          onRemove={vi.fn()}
        />,
      );

      expect(screen.getByText('VND')).toBeInTheDocument();
      expect(screen.queryByText('x1,000 ₫')).toBeNull();
      expect(screen.getByText('FPT')).toBeInTheDocument();
      expect(screen.getByText('99.511,79')).toBeInTheDocument();
    });
  });

  describe('SQA-004: Supported Timeframe Restriction in ReplayHeader', () => {
    it('restricts timeframe options to only supported 1D and 1W', () => {
      const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });

      render(
        <QueryClientProvider client={queryClient}>
          <ReplayHeader
            sessionId={42}
            symbolName="FPT"
            sessionStatus="active"
            sessionData={{
              id: 42,
              symbol: 'FPT',
              timeframe: '1D',
              current_index: 10,
              mode: 'practice',
              adjustment_type: 'adjusted',
              status: 'active',
              initial_balance: 100000000,
              current_cash: 100000000,
              start_date: '2024-01-01',
              end_date: '2024-03-01',
              total_bars: 50,
              is_active: true,
              created_at: '2024-01-01T00:00:00',
            } as any}
            currentCandle={null}
            candleCount={10}
            onChangeTimeframe={vi.fn()}
            onResumeSession={vi.fn()}
            onOpenSymbolSwitcher={vi.fn()}
            onOpenDebrief={vi.fn()}
            onOpenShortcuts={vi.fn()}
            playSpeed={1000}
            onChangeSpeed={vi.fn()}
            isRightPanelOpen={true}
            onToggleRightPanel={vi.fn()}
            onClearSession={vi.fn()}
          />
        </QueryClientProvider>,
      );

      const options = screen.getAllByRole('option').map((o) => o.textContent);
      expect(options).toContain('1D');
      expect(options).toContain('1W');
      expect(options).not.toContain('5m');
      expect(options).not.toContain('15m');
      expect(options).not.toContain('1H');
      expect(options).not.toContain('1M');
    });
  });

  describe('SQA-001: ReplayHeader Button Ownership & Unclipped Layout', () => {
    it('renders primary action buttons in unclipped header group', () => {
      const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });

      render(
        <QueryClientProvider client={queryClient}>
          <ReplayHeader
            sessionId={42}
            symbolName="FPT"
            sessionStatus="active"
            currentCandle={null}
            candleCount={10}
            onChangeTimeframe={vi.fn()}
            onResumeSession={vi.fn()}
            onOpenSymbolSwitcher={vi.fn()}
            onOpenDebrief={vi.fn()}
            onOpenShortcuts={vi.fn()}
            playSpeed={1000}
            onChangeSpeed={vi.fn()}
            isRightPanelOpen={true}
            onToggleRightPanel={vi.fn()}
            onClearSession={vi.fn()}
          />
        </QueryClientProvider>,
      );

      expect(screen.getByTestId('open-add-indicator')).toBeInTheDocument();
      expect(screen.getByTestId('open-session-debrief')).toBeInTheDocument();
      expect(screen.getByTestId('open-keyboard-shortcuts')).toBeInTheDocument();
      expect(screen.getByTestId('symbol-switcher-button')).toBeInTheDocument();
    });
  });

  describe('SQA-009: Signal Inspector Weekly Timeframe Alignment', () => {
    it('passes practiceData.current_date to SignalInspector when chart is on 1W view', () => {
      // Suppose session cursor is on 1D date 2024-02-06, but user switched chart to 1W candle 2024-01-22
      const weeklyCandle: Candle = {
        id: 2,
        symbol: 'FPT',
        timeframe: '1W',
        source: 'doraemon',
        adjustment_type: 'unadjusted',
        timestamp: '2024-01-22T00:00:00',
        open: 95000,
        high: 98000,
        low: 94000,
        close: 97500,
        volume: 5000000,
      };

      const practiceData = {
        session_id: 1,
        mode: 'practice',
        initial_balance: 100000000,
        cash: 100000000,
        equity: 100000000,
        initial_cash: 100000000,
        current_cash: 100000000,
        current_price: 97500,
        current_volume: 5000000,
        current_index: 24,
        visible_bar: 25,
        total_bars: 100,
        current_date: '2024-02-06T00:00:00', // Authoritative session date
        symbol: 'FPT',
        timeframe: '1D',
        available_quantity: 0,
        blocked_quantity: 0,
        historical: false,
        can_trade: true,
        positions: [],
        pending_orders: [],
        orders: [],
        executions: [],
        trades: [],
        latest_activity_index: 24,
        settlement_label: 'T+1.5',
        decisions: [],
        metrics: {} as any,
      } as unknown as PracticeWorkflowSnapshot;

      render(
        <ReplayRightPanel
          sessionId={1}
          currentCandle={weeklyCandle}
          candleCount={100}
          drawingRevision={0}
          drawingPersistenceStatus="idle"
          onUpdateDrawing={vi.fn()}
          onDeleteDrawing={vi.fn()}
          showExtendedInspector={true}
          onToggleExtendedInspector={vi.fn()}
          selectedInspectorSignal="volume.spike"
          onOpenSignalCatalog={vi.fn()}
          practiceData={practiceData}
          practiceLoading={false}
          practiceError={false}
          journalLoading={false}
          journalError={false}
          onResetPractice={vi.fn()}
          onSubmitDecision={vi.fn()}
          onSaveJournal={vi.fn()}
        />,
      );

      const inspector = screen.getByTestId('mock-signal-inspector');
      expect(inspector).toHaveAttribute('data-timestamp', '2024-02-06T00:00:00');
      expect(inspector).toHaveAttribute('data-index', '24');

      const explanation = screen.getByTestId('mock-signal-explanation-inspector');
      expect(explanation).toHaveAttribute('data-timestamp', '2024-02-06T00:00:00');
      expect(explanation).toHaveAttribute('data-index', '24');
    });
  });

  describe('SQA-003: Keyboard Shortcuts Focus Guard & Policy', () => {
    it('rejects keyboard shortcuts when typing in input, textarea, or contenteditable', () => {
      const input = document.createElement('input');
      expect(isGlobalShortcutEligible({ target: input, defaultPrevented: false })).toBe(false);

      const textarea = document.createElement('textarea');
      expect(isGlobalShortcutEligible({ target: textarea, defaultPrevented: false })).toBe(false);

      const editable = document.createElement('div');
      editable.setAttribute('contenteditable', 'true');
      expect(isGlobalShortcutEligible({ target: editable, defaultPrevented: false })).toBe(false);

      const normalDiv = document.createElement('div');
      expect(isGlobalShortcutEligible({ target: normalDiv, defaultPrevented: false })).toBe(true);
    });

    it('toggles Auto-Play on Space, and does nothing when typing in an input', async () => {
      const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
      const wrapper = ({ children }: { children: React.ReactNode }) => (
        <QueryClientProvider client={queryClient}>
          <MemoryRouter initialEntries={['/replay?session=42']}>
            {children}
          </MemoryRouter>
        </QueryClientProvider>
      );

      const { result } = renderHook(() => useReplayWorkspaceController(), { wrapper });

      await waitFor(() => {
        expect(result.current.sessionId).toBe(42);
      });

      expect(result.current.isPlaying).toBe(false);

      // Press Space on document.body
      act(() => {
        window.dispatchEvent(new KeyboardEvent('keydown', { code: 'Space', bubbles: true }));
      });
      expect(result.current.isPlaying).toBe(true);

      // Press Space again on document.body
      act(() => {
        window.dispatchEvent(new KeyboardEvent('keydown', { code: 'Space', bubbles: true }));
      });
      expect(result.current.isPlaying).toBe(false);

      // Focus an input and press Space: should NOT toggle
      const input = document.createElement('input');
      document.body.appendChild(input);
      act(() => {
        input.dispatchEvent(new KeyboardEvent('keydown', { code: 'Space', bubbles: true }));
      });
      expect(result.current.isPlaying).toBe(false);
      document.body.removeChild(input);
    });
  });

  describe('SQA-001: ReplayWorkspace Layout Structure', () => {
    it('positions chart legend overlay container absolutely at top of chart and reserves dock clearance', async () => {
      const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
      render(
        <QueryClientProvider client={queryClient}>
          <MemoryRouter initialEntries={['/replay?session=42']}>
            <ReplayWorkspace />
          </MemoryRouter>
        </QueryClientProvider>,
      );

      await waitFor(() => {
        expect(screen.getByTestId('chart-legend-overlay-container')).toBeInTheDocument();
      });

      const overlayContainer = screen.getByTestId('chart-legend-overlay-container');
      expect(overlayContainer).toHaveStyle({
        position: 'absolute',
        top: '0px',
        left: '0px',
        right: '0px',
        display: 'flex',
        flexDirection: 'column',
      });

      const orderControls = screen.getByTestId('indicator-order-controls');
      expect(overlayContainer).toContainElement(orderControls);
      expect(orderControls).toHaveStyle({
        position: 'relative',
      });

      const scrollRegion = screen.getByTestId('replay-chart-scroll-region');
      expect(scrollRegion).toHaveStyle({
        boxSizing: 'border-box',
        padding: '0px 0px 56px 0px',
      });

      const dock = document.querySelector('.replay-control-dock');
      expect(dock).not.toBeNull();
      expect(dock).toHaveStyle({
        bottom: '-48px',
      });
    });
  });
});
