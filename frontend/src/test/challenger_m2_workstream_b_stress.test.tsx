import { describe, expect, it, vi } from 'vitest';
import React from 'react';
import { render, screen, renderHook, act, fireEvent, waitFor } from '@testing-library/react';
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
import type { IndicatorInstanceV1 } from '../features/indicators/indicatorDomain';

vi.mock('../components/chart/CandleChart', () => ({
  CandleChart: () => <div data-testid="mock-candle-chart">CandleChart</div>,
}));

vi.mock('../components/signals/SignalInspector', () => ({
  SignalInspector: ({ currentTimestamp, currentIndex, timeframe, sessionId }: any) => (
    <div
      data-testid="mock-signal-inspector"
      data-session-id={sessionId}
      data-timestamp={currentTimestamp}
      data-index={currentIndex}
      data-timeframe={timeframe}
    >
      Inspector:{currentTimestamp}@{currentIndex}
    </div>
  ),
}));

vi.mock('../components/signals/SignalExplanationInspector', () => ({
  SignalExplanationInspector: ({ currentTimestamp, currentIndex, timeframe, sessionId }: any) => (
    <div
      data-testid="mock-signal-explanation-inspector"
      data-session-id={sessionId}
      data-timestamp={currentTimestamp}
      data-index={currentIndex}
      data-timeframe={timeframe}
    >
      Explanation:{currentTimestamp}@{currentIndex}
    </div>
  ),
}));

vi.mock('../api/replayApi', async () => {
  const actual = await vi.importActual<typeof import('../api/replayApi')>('../api/replayApi');
  return {
    ...actual,
    getReplaySession: vi.fn().mockResolvedValue({
      id: 99,
      symbol: 'VIC',
      timeframe: '1D',
      start_date: '2024-01-01',
      end_date: '2024-06-01',
      current_index: 20,
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
vi.mock('../components/chart/DrawingInspector', () => ({ DrawingInspector: () => <div>DrawingInspector</div> }));
vi.mock('../components/replay/DecisionJournal', () => ({ DecisionJournal: () => <div>DecisionJournal</div> }));
vi.mock('../components/replay/PendingOrdersPanel', () => ({ PendingOrdersPanel: () => <div>PendingOrdersPanel</div> }));
vi.mock('../components/replay/PositionPanel', () => ({ PositionPanel: () => <div>PositionPanel</div> }));
vi.mock('../components/replay/PracticeJournal', () => ({ PracticeJournal: () => <div>PracticeJournal</div> }));
vi.mock('../components/replay/PracticeRail', () => ({ PracticeRail: ({ trade }: any) => <div data-testid="practice-rail">{trade}</div> }));
vi.mock('../components/replay/PracticeScoreboard', () => ({ PracticeScoreboard: () => <div>PracticeScoreboard</div> }));
vi.mock('../components/replay/ScannerSourceContext', () => ({ ScannerSourceContext: () => <div>ScannerSourceContext</div> }));
vi.mock('../components/replay/TradeControls', () => ({ TradeControls: () => <div>TradeControls</div> }));

describe('CHALLENGER EMPIRICAL STRESS TEST SUITE — Milestone 2 Workstream B', () => {

  // =========================================================================
  // 1. SQA-012: Ichimoku Cloud Autoscale & Whitespace Isolation
  // =========================================================================
  describe('SQA-012: Ichimoku Cloud Autoscale & Whitespace Edge Cases', () => {
    const plugin = new IchimokuCloudPlugin();

    it('rigorously classifies any non-finite or missing span bar as whitespace', () => {
      const invalidCombinations: Array<{ label: string; spanA?: any; spanB?: any }> = [
        { label: 'both null', spanA: null, spanB: null },
        { label: 'both undefined', spanA: undefined, spanB: undefined },
        { label: 'empty object' },
        { label: 'spanA null, spanB finite', spanA: null, spanB: 85000 },
        { label: 'spanA finite, spanB null', spanA: 92000, spanB: null },
        { label: 'spanA undefined, spanB finite', spanA: undefined, spanB: 85000 },
        { label: 'spanA finite, spanB undefined', spanA: 92000, spanB: undefined },
        { label: 'spanA NaN, spanB finite', spanA: NaN, spanB: 85000 },
        { label: 'spanA finite, spanB NaN', spanA: 92000, spanB: NaN },
        { label: 'both NaN', spanA: NaN, spanB: NaN },
        { label: 'spanA Infinity, spanB finite', spanA: Infinity, spanB: 85000 },
        { label: 'spanA finite, spanB Infinity', spanA: 92000, spanB: Infinity },
        { label: 'both Infinity', spanA: Infinity, spanB: Infinity },
        { label: 'both -Infinity', spanA: -Infinity, spanB: -Infinity },
      ];

      for (const { label, spanA, spanB } of invalidCombinations) {
        const isWs = plugin.isWhitespace({ time: '2024-01-01', spanA, spanB });
        expect(isWs, `Failed whitespace detection for case: ${label}`).toBe(true);
      }
    });

    it('classifies completely finite numeric spans (even zero or negative) as non-whitespace', () => {
      expect(plugin.isWhitespace({ time: '2024-01-01', spanA: 95000, spanB: 92000 })).toBe(false);
      expect(plugin.isWhitespace({ time: '2024-01-02', spanA: 0, spanB: 0 })).toBe(false);
      expect(plugin.isWhitespace({ time: '2024-01-03', spanA: -500, spanB: -600 })).toBe(false);
      expect(plugin.isWhitespace({ time: '2024-01-04', spanA: 1e-7, spanB: 1e-7 })).toBe(false);
    });

    it('guarantees priceValueBuilder NEVER returns 0 when values are missing or non-finite', () => {
      // Both missing/non-finite -> must return []
      expect(plugin.priceValueBuilder({ time: '2024-01-01', spanA: null, spanB: null })).toEqual([]);
      expect(plugin.priceValueBuilder({ time: '2024-01-01', spanA: undefined, spanB: undefined })).toEqual([]);
      expect(plugin.priceValueBuilder({ time: '2024-01-01', spanA: NaN, spanB: NaN })).toEqual([]);
      expect(plugin.priceValueBuilder({ time: '2024-01-01', spanA: Infinity, spanB: -Infinity })).toEqual([]);

      // One missing/non-finite -> must return triple of the valid finite span ONLY
      expect(plugin.priceValueBuilder({ time: '2024-01-01', spanA: 98000, spanB: null })).toEqual([98000, 98000, 98000]);
      expect(plugin.priceValueBuilder({ time: '2024-01-01', spanA: null, spanB: 94000 })).toEqual([94000, 94000, 94000]);
      expect(plugin.priceValueBuilder({ time: '2024-01-01', spanA: 98000, spanB: NaN })).toEqual([98000, 98000, 98000]);
      expect(plugin.priceValueBuilder({ time: '2024-01-01', spanA: Infinity, spanB: 94000 })).toEqual([94000, 94000, 94000]);

      // Both valid finite -> returns [spanA, spanB, spanA]
      expect(plugin.priceValueBuilder({ time: '2024-01-01', spanA: 105000, spanB: 101000 })).toEqual([105000, 101000, 105000]);
    });

    it('renders polygon segments without throwing on incomplete series and correctly selects bullish/bearish color', () => {
      const renderer = plugin.renderer();
      const mockCtx = {
        beginPath: vi.fn(),
        moveTo: vi.fn(),
        lineTo: vi.fn(),
        closePath: vi.fn(),
        fill: vi.fn(),
        fillStyle: '',
      };
      const mockTarget = {
        useMediaCoordinateSpace: (cb: (scope: { context: any }) => void) => {
          cb({ context: mockCtx });
        },
      } as any;
      const priceConverter: any = (price: number) => 1000 - price * 0.01;

      // Case A: No bars -> no draw
      plugin.update({ bars: [] } as any, { upColor: 'green', downColor: 'red' } as any);
      renderer.draw(mockTarget, priceConverter, false);
      expect(mockCtx.beginPath).not.toHaveBeenCalled();

      // Case B: Single bar -> no polygon possible
      plugin.update({
        bars: [{ x: 10, originalData: { time: '2024-01-01', spanA: 95000, spanB: 90000 } }],
      } as any, { upColor: 'green', downColor: 'red' } as any);
      renderer.draw(mockTarget, priceConverter, false);
      expect(mockCtx.beginPath).not.toHaveBeenCalled();

      // Case C: Consecutive bars with missing values -> skips segments cleanly without throwing
      plugin.update({
        bars: [
          { x: 10, originalData: { time: '2024-01-01', spanA: null, spanB: null } },
          { x: 20, originalData: { time: '2024-01-02', spanA: 95000, spanB: null } },
          { x: 30, originalData: { time: '2024-01-03', spanA: 96000, spanB: 92000 } }, // Bullish (currA > currB)
          { x: 40, originalData: { time: '2024-01-04', spanA: 91000, spanB: 93000 } }, // Bearish (currA < currB)
        ],
      } as any, { upColor: 'rgba(0,255,0,0.2)', downColor: 'rgba(255,0,0,0.2)' } as any);

      renderer.draw(mockTarget, priceConverter, false);
      // Segment 1->2 skipped (nulls)
      // Segment 2->3 skipped (bar 2 spanB is null)
      // Segment 3->4 drawn!
      expect(mockCtx.beginPath).toHaveBeenCalledTimes(1);
      expect(mockCtx.moveTo).toHaveBeenCalledWith(30, priceConverter(96000));
      expect(mockCtx.lineTo).toHaveBeenCalledWith(40, priceConverter(91000));
      expect(mockCtx.lineTo).toHaveBeenCalledWith(40, priceConverter(93000));
      expect(mockCtx.lineTo).toHaveBeenCalledWith(30, priceConverter(92000));
      expect(mockCtx.closePath).toHaveBeenCalledTimes(1);
      expect(mockCtx.fill).toHaveBeenCalledTimes(1);
      expect(mockCtx.fillStyle).toBe('rgba(255,0,0,0.2)'); // 91000 < 93000 -> downColor
    });
  });

  // =========================================================================
  // 2. SQA-002: Unit Badge Correctness & Price Scaling
  // =========================================================================
  describe('SQA-002: Unit Badge Correctness & Interaction', () => {
    it('displays authoritative "VND" unit badge and eliminates "x1,000 ₫" completely', () => {
      const candle: Candle = {
        id: 1,
        symbol: 'TCB',
        timeframe: '1D',
        source: 'doraemon',
        adjustment_type: 'unadjusted',
        timestamp: '2024-03-01T00:00:00',
        open: 45000.0,
        high: 46200.0,
        low: 44800.0,
        close: 45800.0,
        volume: 8500000,
      };

      const mockToggle = vi.fn();
      const mockSettings = vi.fn();
      const mockRemove = vi.fn();

      const indicators: IndicatorInstanceV1[] = [
        {
          id: 'sma-20-inst',
          definitionId: 'sma',
          label: 'SMA (20)',
          placement: 'price',
          paneId: 'price',
          order: 0,
          visible: true,
          params: { period: 20 },
          styles: { plot: { color: '#FF9800' } },
        },
      ];

      render(
        <ChartLegendOverlay
          symbol="TCB"
          timeframe="1D"
          currentCandle={candle}
          indicators={indicators}
          runtime={{ 'sma-20-inst': { status: 'ready', values: { plot: 45100.5 } } }}
          onToggle={mockToggle}
          onSettings={mockSettings}
          onRemove={mockRemove}
        />,
      );

      // Verify VND badge exists
      expect(screen.getByText('VND')).toBeInTheDocument();
      // Verify misleading x1,000 ₫ does NOT exist
      expect(screen.queryByText('x1,000 ₫')).toBeNull();
      expect(screen.queryByText(/x1,000/)).toBeNull();

      // Verify full VND numeric formatting (Vietnamese locale format: 45.000, 45.800)
      expect(screen.getByText('45.000,00')).toBeInTheDocument();
      expect(screen.getByText('45.800,00')).toBeInTheDocument();

      // Test interaction buttons on indicator chips via data-testid:
      const toggleBtn = screen.getByTestId('toggle-indicator-sma-20-inst');
      fireEvent.click(toggleBtn);
      expect(mockToggle).toHaveBeenCalledWith('sma-20-inst');

      const settingsBtn = screen.getByTestId('indicator-settings-sma-20-inst');
      fireEvent.click(settingsBtn);
      expect(mockSettings).toHaveBeenCalledWith(indicators[0]);

      const removeBtn = screen.getByTestId('remove-indicator-sma-20-inst');
      fireEvent.click(removeBtn);
      expect(mockRemove).toHaveBeenCalledWith('sma-20-inst');
    });

    it('handles zero, extreme prices, and null currentCandle gracefully', () => {
      const { rerender } = render(
        <ChartLegendOverlay
          symbol="VNM"
          timeframe="1D"
          currentCandle={null}
          indicators={[]}
          runtime={{}}
          onToggle={vi.fn()}
          onSettings={vi.fn()}
          onRemove={vi.fn()}
        />,
      );

      // Null candle: Should not crash, displays empty header
      expect(screen.queryByText('VND')).toBeNull();

      // Extreme price candle (1,000,000,000 VND)
      const billionCandle: Candle = {
        id: 2,
        symbol: 'VNM',
        timeframe: '1D',
        source: 'doraemon',
        adjustment_type: 'unadjusted',
        timestamp: '2024-03-01T00:00:00',
        open: 1000000000,
        high: 1000000000,
        low: 1000000000,
        close: 1000000000,
        volume: 1,
      };

      rerender(
        <ChartLegendOverlay
          symbol="VNM"
          timeframe="1D"
          currentCandle={billionCandle}
          indicators={[]}
          runtime={{}}
          onToggle={vi.fn()}
          onSettings={vi.fn()}
          onRemove={vi.fn()}
        />,
      );

      expect(screen.getByText('VND')).toBeInTheDocument();
      const matchElements = screen.getAllByText('1.000.000.000,00');
      expect(matchElements.length).toBeGreaterThanOrEqual(4); // O, H, L, C
    });
  });

  // =========================================================================
  // 3. SQA-004: Supported Timeframe Restriction in ReplayHeader
  // =========================================================================
  describe('SQA-004: Supported Timeframe Options in ReplayHeader', () => {
    it('only presents genuinely supported 1D and 1W options without unsupported timeframes', () => {
      const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
      const handleChangeTimeframe = vi.fn();

      render(
        <QueryClientProvider client={queryClient}>
          <ReplayHeader
            sessionId={101}
            symbolName="HPG"
            sessionStatus="active"
            sessionData={{
              id: 101,
              symbol: 'HPG',
              timeframe: '1D',
              current_index: 5,
              mode: 'practice',
              adjustment_type: 'adjusted',
              status: 'active',
              initial_balance: 100000000,
              current_cash: 100000000,
              start_date: '2024-01-01',
              end_date: '2024-03-01',
              total_bars: 40,
              is_active: true,
              created_at: '2024-01-01T00:00:00',
            } as any}
            currentCandle={null}
            candleCount={5}
            onChangeTimeframe={handleChangeTimeframe}
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

      // Select timeframe dropdown (first combobox in ReplayHeader)
      const selects = screen.getAllByRole('combobox');
      const tfSelect = selects[0];
      const options = Array.from(tfSelect.querySelectorAll('option')).map((o) => o.value);

      expect(options).toEqual(['1D', '1W']);
      expect(options).not.toContain('5m');
      expect(options).not.toContain('15m');
      expect(options).not.toContain('1H');
      expect(options).not.toContain('1M');

      // Change to 1W: fires onChangeTimeframe with '1W'
      fireEvent.change(tfSelect, { target: { value: '1W' } });
      expect(handleChangeTimeframe).toHaveBeenCalledWith('1W');

      // Change back to session timeframe '1D': fires onChangeTimeframe with undefined (reverts to session default)
      fireEvent.change(tfSelect, { target: { value: '1D' } });
      expect(handleChangeTimeframe).toHaveBeenCalledWith(undefined);
    });
  });

  // =========================================================================
  // 4. SQA-009: Weekly Timeframe Signal Inspector Alignment
  // =========================================================================
  describe('SQA-009: Weekly Timeframe Signal Inspector Alignment', () => {
    it('authoritatively passes session current_date and current_index from practiceData when viewing 1W candles', () => {
      const weeklyCandle: Candle = {
        id: 50,
        symbol: 'FPT',
        timeframe: '1W',
        source: 'doraemon',
        adjustment_type: 'unadjusted',
        timestamp: '2024-01-15T00:00:00', // Weekly bar timestamp
        open: 92000,
        high: 96000,
        low: 91000,
        close: 95500,
        volume: 6200000,
      };

      const practiceData = {
        session_id: 101,
        mode: 'practice',
        initial_balance: 100000000,
        cash: 100000000,
        equity: 100000000,
        initial_cash: 100000000,
        current_cash: 100000000,
        current_price: 95500,
        current_volume: 6200000,
        current_index: 38,
        visible_bar: 39,
        total_bars: 100,
        current_date: '2024-01-24T00:00:00', // Actual daily session date!
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
        latest_activity_index: 38,
        settlement_label: 'T+1.5',
        decisions: [],
        metrics: {} as any,
      } as unknown as PracticeWorkflowSnapshot;

      render(
        <ReplayRightPanel
          sessionId={101}
          currentCandle={weeklyCandle}
          candleCount={100}
          drawingRevision={0}
          drawingPersistenceStatus="idle"
          onUpdateDrawing={vi.fn()}
          onDeleteDrawing={vi.fn()}
          showExtendedInspector={true}
          onToggleExtendedInspector={vi.fn()}
          selectedInspectorSignal="trend.ma_alignment"
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
      // Must use practiceData.current_date (2024-01-24), NOT weeklyCandle.timestamp (2024-01-15)
      expect(inspector).toHaveAttribute('data-timestamp', '2024-01-24T00:00:00');
      // Must use practiceData.current_index (38)
      expect(inspector).toHaveAttribute('data-index', '38');

      const explanation = screen.getByTestId('mock-signal-explanation-inspector');
      expect(explanation).toHaveAttribute('data-timestamp', '2024-01-24T00:00:00');
      expect(explanation).toHaveAttribute('data-index', '38');
    });

    it('falls back safely to currentCandle and candleCount when practiceData is null', () => {
      const dailyCandle: Candle = {
        id: 10,
        symbol: 'FPT',
        timeframe: '1D',
        source: 'doraemon',
        adjustment_type: 'unadjusted',
        timestamp: '2024-02-15T00:00:00',
        open: 102000,
        high: 104000,
        low: 101000,
        close: 103500,
        volume: 3200000,
      };

      render(
        <ReplayRightPanel
          sessionId={101}
          currentCandle={dailyCandle}
          candleCount={15}
          drawingRevision={0}
          drawingPersistenceStatus="idle"
          onUpdateDrawing={vi.fn()}
          onDeleteDrawing={vi.fn()}
          showExtendedInspector={true}
          onToggleExtendedInspector={vi.fn()}
          selectedInspectorSignal="volume.spike"
          onOpenSignalCatalog={vi.fn()}
          practiceData={null as any}
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
      expect(inspector).toHaveAttribute('data-timestamp', '2024-02-15T00:00:00');
      // 15 candles -> index 14
      expect(inspector).toHaveAttribute('data-index', '14');
    });
  });

  // =========================================================================
  // 5. SQA-003: DOM Event Simulation for Keyboard Shortcuts & Typing Guard
  // =========================================================================
  describe('SQA-003: Keyboard Shortcuts & Focus Guard Policy', () => {
    it('comprehensive isGlobalShortcutEligible policy rejects interactive and modal contexts', () => {
      // Elements that are always interactive
      const formElements = ['input', 'textarea', 'select', 'button'];
      for (const tag of formElements) {
        const el = document.createElement(tag);
        expect(isGlobalShortcutEligible({ target: el, defaultPrevented: false }), `Allowed shortcut in <${tag}>`).toBe(false);
      }

      // Link with href
      const link = document.createElement('a');
      link.href = 'https://example.com';
      expect(isGlobalShortcutEligible({ target: link, defaultPrevented: false })).toBe(false);

      // Contenteditable
      const editable = document.createElement('div');
      editable.setAttribute('contenteditable', 'true');
      expect(isGlobalShortcutEligible({ target: editable, defaultPrevented: false })).toBe(false);

      // Event already default-prevented
      const normalDiv = document.createElement('div');
      expect(isGlobalShortcutEligible({ target: normalDiv, defaultPrevented: true })).toBe(false);

      // Non-interactive body/div
      expect(isGlobalShortcutEligible({ target: normalDiv, defaultPrevented: false })).toBe(true);
    });

    it('Space toggles Auto-Play and prevents page scroll, while inputs remain fully isolated', async () => {
      const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
      const wrapper = ({ children }: { children: React.ReactNode }) => (
        <QueryClientProvider client={queryClient}>
          <MemoryRouter initialEntries={['/replay?session=99']}>
            {children}
          </MemoryRouter>
        </QueryClientProvider>
      );

      const { result } = renderHook(() => useReplayWorkspaceController(), { wrapper });

      await waitFor(() => {
        expect(result.current.sessionId).toBe(99);
      });

      expect(result.current.isPlaying).toBe(false);

      // 1. Dispatch Space on document body -> toggles isPlaying to true
      const spaceEvent1 = new KeyboardEvent('keydown', { code: 'Space', bubbles: true, cancelable: true });
      act(() => {
        window.dispatchEvent(spaceEvent1);
      });
      expect(result.current.isPlaying).toBe(true);
      expect(spaceEvent1.defaultPrevented).toBe(true); // Space default scroll prevented!

      // 2. Dispatch Space again -> toggles isPlaying back to false
      const spaceEvent2 = new KeyboardEvent('keydown', { code: 'Space', bubbles: true, cancelable: true });
      act(() => {
        window.dispatchEvent(spaceEvent2);
      });
      expect(result.current.isPlaying).toBe(false);
      expect(spaceEvent2.defaultPrevented).toBe(true);

      // 3. Typing in input: Space should NOT toggle isPlaying
      const input = document.createElement('input');
      document.body.appendChild(input);
      const inputSpaceEvent = new KeyboardEvent('keydown', { code: 'Space', bubbles: true, cancelable: true });
      act(() => {
        input.dispatchEvent(inputSpaceEvent);
      });
      expect(result.current.isPlaying).toBe(false);
      expect(inputSpaceEvent.defaultPrevented).toBe(false); // Typing space is NOT eaten by replay controller

      // 4. Typing in textarea: Space should NOT toggle isPlaying
      const textarea = document.createElement('textarea');
      document.body.appendChild(textarea);
      const textareaSpaceEvent = new KeyboardEvent('keydown', { code: 'Space', bubbles: true, cancelable: true });
      act(() => {
        textarea.dispatchEvent(textareaSpaceEvent);
      });
      expect(result.current.isPlaying).toBe(false);
      expect(textareaSpaceEvent.defaultPrevented).toBe(false);

      document.body.removeChild(input);
      document.body.removeChild(textarea);
    });
  });

  // =========================================================================
  // 6. SQA-001: Replay Layout Ownership & Responsive Geometry Structure
  // =========================================================================
  describe('SQA-001: Replay Layout Ownership & Responsive Geometry', () => {
    it('verifies ReplayWorkspace positions chart legend overlay absolutely and reserves dock clearance', async () => {
      const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
      render(
        <QueryClientProvider client={queryClient}>
          <MemoryRouter initialEntries={['/replay?session=99']}>
            <ReplayWorkspace />
          </MemoryRouter>
        </QueryClientProvider>,
      );

      await waitFor(() => {
        expect(screen.getByTestId('chart-legend-overlay-container')).toBeInTheDocument();
      });

      // ChartLegendOverlay must be absolutely positioned at the top of the chart
      const overlayContainer = screen.getByTestId('chart-legend-overlay-container');
      expect(overlayContainer).toHaveStyle({
        position: 'absolute',
        top: '0px',
        left: '0px',
        right: '0px',
        zIndex: '20',
      });

      // Scroll region must have bottom padding to prevent dock collision
      const scrollRegion = screen.getByTestId('replay-chart-scroll-region');
      expect(scrollRegion).toHaveStyle({
        padding: '0px 0px 56px 0px',
        boxSizing: 'border-box',
      });
    });

    it('verifies ReplayHeader responsive breakdown classes and unclipped action buttons', () => {
      const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
      const onOpenDebrief = vi.fn();
      const onOpenShortcuts = vi.fn();
      const onOpenSymbolSwitcher = vi.fn();

      const { container } = render(
        <QueryClientProvider client={queryClient}>
          <ReplayHeader
            sessionId={99}
            symbolName="VIC"
            sessionStatus="active"
            currentCandle={{
              id: 1,
              symbol: 'VIC',
              timeframe: '1D',
              source: 'doraemon',
              adjustment_type: 'unadjusted',
              timestamp: '2024-03-01T00:00:00',
              open: 45000,
              high: 46000,
              low: 44500,
              close: 45500,
              volume: 1000000,
            }}
            candleCount={20}
            onChangeTimeframe={vi.fn()}
            onResumeSession={vi.fn()}
            onOpenSymbolSwitcher={onOpenSymbolSwitcher}
            onOpenDebrief={onOpenDebrief}
            onOpenShortcuts={onOpenShortcuts}
            playSpeed={1000}
            onChangeSpeed={vi.fn()}
            isRightPanelOpen={true}
            onToggleRightPanel={vi.fn()}
            onClearSession={vi.fn()}
          />
        </QueryClientProvider>,
      );

      // Verify action buttons are present and clickable
      const debriefBtn = screen.getByTestId('open-session-debrief');
      fireEvent.click(debriefBtn);
      expect(onOpenDebrief).toHaveBeenCalledTimes(1);

      const shortcutsBtn = screen.getByTestId('open-keyboard-shortcuts');
      fireEvent.click(shortcutsBtn);
      expect(onOpenShortcuts).toHaveBeenCalledTimes(1);

      const symbolBtn = screen.getByTestId('symbol-switcher-button');
      fireEvent.click(symbolBtn);
      expect(onOpenSymbolSwitcher).toHaveBeenCalledTimes(1);

      // Verify the responsive style sheet exists
      const styleEl = container.querySelector('style');
      expect(styleEl).not.toBeNull();
      const css = styleEl?.textContent || '';
      expect(css).toContain('@media (max-width: 1439px)');
      expect(css).toContain('.replay-header-ohlcv-numbers { display: none !important; }');
      expect(css).toContain('@media (max-width: 1279px)');
      expect(css).toContain('.replay-header-ohlcv-summary { display: none !important; }');

      // Verify container elements have the corresponding CSS classes
      expect(container.querySelector('.replay-header-ohlcv-numbers')).not.toBeNull();
      expect(container.querySelector('.replay-header-ohlcv-summary')).not.toBeNull();
    });
  });
});
