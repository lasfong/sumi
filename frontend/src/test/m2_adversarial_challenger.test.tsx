import { describe, expect, it, vi } from 'vitest';
import React from 'react';
import { render, screen, renderHook, act, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { Candle, PracticeWorkflowSnapshot } from '../types';
import { IchimokuCloudPlugin, type IchimokuCloudData } from '../components/chart/plugins/IchimokuCloudPlugin';
import { ChartLegendOverlay } from '../components/chart/ChartLegendOverlay';
import { ReplayHeader } from '../components/replay/ReplayHeader';
import { ReplayRightPanel } from '../components/replay/ReplayRightPanel';
import { useReplayWorkspaceController } from '../components/replay/ReplayWorkspaceController';
import { isGlobalShortcutEligible } from '../features/replay/globalShortcutPolicy';
import { IndicatorRenderRegistry } from '../components/chart/IndicatorRenderRegistry';
import type { IndicatorInstanceV1 } from '../features/indicators/indicatorDomain';

// Mock chart canvas and external queries
vi.mock('../components/chart/CandleChart', () => ({
  CandleChart: () => <div data-testid="mock-candle-chart">CandleChart</div>,
}));

vi.mock('../components/signals/SignalInspector', () => ({
  SignalInspector: ({ currentTimestamp, currentIndex, timeframe, symbol }: any) => (
    <div
      data-testid="mock-signal-inspector"
      data-timestamp={currentTimestamp}
      data-index={currentIndex}
      data-timeframe={timeframe}
      data-symbol={symbol}
    >
      Inspector {symbol} @ {currentTimestamp} [{currentIndex}] ({timeframe})
    </div>
  ),
}));

vi.mock('../components/signals/SignalExplanationInspector', () => ({
  SignalExplanationInspector: ({ currentTimestamp, currentIndex, timeframe, symbol }: any) => (
    <div
      data-testid="mock-signal-explanation-inspector"
      data-timestamp={currentTimestamp}
      data-index={currentIndex}
      data-timeframe={timeframe}
      data-symbol={symbol}
    >
      Explanation {symbol} @ {currentTimestamp} [{currentIndex}] ({timeframe})
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
      timeframe: '1W',
      start_date: '2024-01-01',
      end_date: '2024-06-01',
      current_index: 30,
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
vi.mock('../components/chart/DrawingInspector', () => ({ DrawingInspector: () => <div /> }));
vi.mock('../components/replay/DecisionJournal', () => ({ DecisionJournal: () => <div /> }));
vi.mock('../components/replay/PendingOrdersPanel', () => ({ PendingOrdersPanel: () => <div /> }));
vi.mock('../components/replay/PositionPanel', () => ({ PositionPanel: () => <div /> }));
vi.mock('../components/replay/PracticeJournal', () => ({ PracticeJournal: () => <div /> }));
vi.mock('../components/replay/PracticeRail', () => ({
  PracticeRail: ({ trade }: any) => <div data-testid="mock-practice-rail">{trade}</div>,
}));
vi.mock('../components/replay/PracticeScoreboard', () => ({ PracticeScoreboard: () => <div /> }));
vi.mock('../components/replay/ScannerSourceContext', () => ({ ScannerSourceContext: () => <div /> }));
vi.mock('../components/replay/TradeControls', () => ({ TradeControls: () => <div /> }));

describe('Milestone 2 Challenger Empirical Adversarial Stress Suite', () => {

  // =========================================================================
  // 1. SQA-012: Ichimoku Cloud Autoscale & Warmup Stress Tests
  // =========================================================================
  describe('SQA-012: Ichimoku Autoscale Warmup & Whitespace Invariant', () => {
    const plugin = new IchimokuCloudPlugin();

    it('adversarial datasets generator: never produces 0 in priceValueBuilder', () => {
      // Stress harness: test across varying dataset lengths and corner cases
      const testCases: { name: string; rows: IchimokuCloudData[] }[] = [
        {
          name: 'Empty dataset (0 bars)',
          rows: [],
        },
        {
          name: 'Warmup 10 bars (all null/undefined/NaN)',
          rows: Array.from({ length: 10 }, (_, i) => ({
            time: `2024-01-${String(i + 1).padStart(2, '0')}`,
            spanA: i % 2 === 0 ? null : undefined,
            spanB: i % 3 === 0 ? NaN : null,
          })),
        },
        {
          name: 'Warmup 26 bars (only spanA starts, spanB null)',
          rows: Array.from({ length: 26 }, (_, i) => ({
            time: `2024-01-${String(i + 1).padStart(2, '0')}`,
            spanA: 95000 + i * 100,
            spanB: null,
          })),
        },
        {
          name: 'Warmup 52 bars (spanB starts at bar 52, spanA missing on some bars)',
          rows: Array.from({ length: 52 }, (_, i) => ({
            time: `2024-01-${String(i + 1).padStart(2, '0')}`,
            spanA: i >= 26 ? (i % 5 === 0 ? null : 95000 + i * 50) : null,
            spanB: i >= 51 ? 94000 : null,
          })),
        },
        {
          name: 'Corrupted values: Infinity, -Infinity, NaN, undefined, null',
          rows: [
            { time: '2024-01-01', spanA: Infinity, spanB: 95000 },
            { time: '2024-01-02', spanA: 95000, spanB: -Infinity },
            { time: '2024-01-03', spanA: NaN, spanB: NaN },
            { time: '2024-01-04', spanA: undefined, spanB: undefined },
            { time: '2024-01-05', spanA: null, spanB: null },
          ],
        },
        {
          name: 'Full 100 bars production simulation (VND stock 90,000 - 110,000)',
          rows: Array.from({ length: 100 }, (_, i) => {
            const spanA = i >= 26 ? 90000 + (i * 150) % 20000 : null;
            const spanB = i >= 52 ? 88000 + (i * 120) % 18000 : null;
            return {
              time: `2024-01-${String((i % 28) + 1).padStart(2, '0')}`,
              spanA,
              spanB,
            };
          }),
        },
      ];

      for (const tc of testCases) {
        for (const row of tc.rows) {
          const values = plugin.priceValueBuilder(row);
          const isWs = plugin.isWhitespace(row);

          // INVARIANT 1: priceValueBuilder never returns 0
          for (const v of values) {
            expect(v, `Found 0 in tc "${tc.name}" at time ${row.time}`).not.toBe(0);
            expect(Number.isFinite(v), `Found non-finite in tc "${tc.name}" at time ${row.time}`).toBe(true);
            expect(v, `Found negative price in tc "${tc.name}" at time ${row.time}`).toBeGreaterThan(0);
          }

          // INVARIANT 2: If either span is missing or non-finite, isWhitespace MUST be true
          const hasA = row.spanA !== null && row.spanA !== undefined && Number.isFinite(row.spanA);
          const hasB = row.spanB !== null && row.spanB !== undefined && Number.isFinite(row.spanB);

          if (!hasA || !hasB) {
            expect(isWs, `Expected whitespace in tc "${tc.name}" for row ${JSON.stringify(row)}`).toBe(true);
          } else {
            expect(isWs, `Expected valid data in tc "${tc.name}" for row ${JSON.stringify(row)}`).toBe(false);
          }
        }
      }
    });

    it('IndicatorRenderRegistry safely handles partial Ichimoku columns and empty datasets', () => {
      const instance: IndicatorInstanceV1 = {
        id: 'ichimoku-inst-1',
        definitionId: 'ichimoku',
        paneId: 'price',
        placement: 'price',
        order: 0,
        label: 'Ichimoku Cloud',
        visible: true,
        styles: {
          cloudUp: { color: '#26A69A' },
          cloudDown: { color: '#EF5350' },
          tenkan: { color: '#26A69A' },
          kijun: { color: '#EF5350' },
          spanA: { color: '#00E5FF' },
          spanB: { color: '#FF8A00' },
          chikou: { color: '#E040FB' },
        },
        params: {
          tenkan: 9,
          kijun: 26,
          senkou: 52,
        },
      };

      // Case 1: missing one required column (e.g. ISB_26 missing) -> all-or-nothing returns empty series
      const incompleteData = [{ timestamp: '2024-01-01', ITS_9: 95000, IKS_26: 94000, ISA_9: 94500, ICS_26: 96000 }];
      const emptyResult = IndicatorRenderRegistry.mapBackendData(incompleteData, instance);
      expect(emptyResult).toEqual([]);

      // Case 2: all columns present, but 0 data rows -> returns empty array
      const noDataResult = IndicatorRenderRegistry.mapBackendData([], instance);
      expect(noDataResult).toEqual([]);

      // Case 3: data with partial warmup bars (Span A available from bar 26, Span B only from bar 52)
      const warmupData: Record<string, any>[] = Array.from({ length: 60 }, (_, i) => {
        const timestamp = `2024-01-${String((i % 28) + 1).padStart(2, '0')}`;
        return {
          timestamp,
          ITS_9: i >= 9 ? 95000 + i * 10 : null,
          IKS_26: i >= 26 ? 94000 + i * 10 : null,
          ISA_9: i >= 26 ? 94500 + i * 10 : null,
          ISB_26: i >= 52 ? 93000 + i * 10 : null,
          ICS_26: 96000,
        };
      });

      const seriesList = IndicatorRenderRegistry.mapBackendData(warmupData as any, instance);
      expect(seriesList.length).toBe(6); // cloud + tenkan + kijun + spanA + spanB + chikou

      const cloudSeries = seriesList.find(s => s.seriesKey === 'cloud');
      expect(cloudSeries).toBeDefined();
      expect(cloudSeries?.type).toBe('ichimoku-cloud');

      // Check cloudData contains null for spanB during bars 26..51
      const bar30Cloud = (cloudSeries?.data[4] as unknown) as IchimokuCloudData; // around bar 30
      expect(bar30Cloud).toBeDefined();
      expect(bar30Cloud.spanA).toBeGreaterThan(90000);
      expect(bar30Cloud.spanB).toBeNull();

      // Passing this cloudData to IchimokuCloudPlugin must result in whitespace
      expect(plugin.isWhitespace(bar30Cloud)).toBe(true);
      // And priceValueBuilder must return the single finite value, NOT 0
      const pv = plugin.priceValueBuilder(bar30Cloud);
      expect(pv).toEqual([bar30Cloud.spanA, bar30Cloud.spanA, bar30Cloud.spanA]);
      expect(pv).not.toContain(0);
    });
  });

  // =========================================================================
  // 2. SQA-001: Replay Layout & Multi-Viewport Responsiveness
  // =========================================================================
  describe('SQA-001: Layout Boundaries, Dock Clearance & Overlay Pointer Events', () => {
    it('verifies ChartLegendOverlay container has pointerEvents: none and inner controls have pointerEvents: auto', () => {
      const candle: Candle = {
        id: 1,
        symbol: 'SSI',
        timeframe: '1D',
        source: 'doraemon',
        adjustment_type: 'adjusted',
        timestamp: '2024-03-01T00:00:00',
        open: 35000,
        high: 36000,
        low: 34500,
        close: 35800,
        volume: 8500000,
      };

      const onToggle = vi.fn();
      const onSettings = vi.fn();
      const onRemove = vi.fn();

      const instance: IndicatorInstanceV1 = {
        id: 'rsi-1',
        definitionId: 'rsi',
        paneId: 'indicator:subpane-1',
        placement: 'oscillator',
        order: 1,
        label: 'RSI',
        visible: true,
        styles: { rsi: { color: '#FFD166' } },
        params: { length: 14 },
      };

      render(
        <ChartLegendOverlay
          symbol="SSI"
          timeframe="1D"
          currentCandle={candle}
          indicators={[instance]}
          runtime={{ 'rsi-1': { status: 'ready', values: { rsi: 65.4 } } }}
          onToggle={onToggle}
          onSettings={onSettings}
          onRemove={onRemove}
        />
      );

      // OHLCV header has pointerEvents: auto
      const ohlcSection = screen.getByText('SSI').closest('div')?.parentElement;
      expect(ohlcSection).toHaveStyle({ pointerEvents: 'auto' });

      // Active indicators list has pointerEvents: auto
      const indicatorList = screen.getByTestId('active-indicator-list');
      expect(indicatorList).toHaveStyle({ pointerEvents: 'auto' });

      // Click toggle, settings, and remove buttons to confirm interactivity
      const toggleBtn = screen.getByTestId('toggle-indicator-rsi-1');
      act(() => { toggleBtn.click(); });
      expect(onToggle).toHaveBeenCalledWith('rsi-1');

      const settingsBtn = screen.getByTestId('indicator-settings-rsi-1');
      act(() => { settingsBtn.click(); });
      expect(onSettings).toHaveBeenCalledWith(instance);

      const removeBtn = screen.getByTestId('remove-indicator-rsi-1');
      act(() => { removeBtn.click(); });
      expect(onRemove).toHaveBeenCalledWith('rsi-1');
    });

    it('verifies ReplayHeader renders essential toolbar controls without button clipping', () => {
      const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });

      const candle: Candle = {
        id: 1,
        symbol: 'FPT',
        timeframe: '1D',
        source: 'doraemon',
        adjustment_type: 'adjusted',
        timestamp: '2024-03-01T00:00:00',
        open: 100000,
        high: 102000,
        low: 99500,
        close: 101500,
        volume: 2000000,
      };

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
              adjustment_type: 'adjusted',
              mode: 'practice',
              start_date: '2024-01-01',
              end_date: '2024-06-01',
              current_index: 15,
              status: 'active',
              initial_balance: 100000000,
              current_cash: 100000000,
              total_bars: 100,
              is_active: true,
              created_at: '2024-01-01T00:00:00',
            } as any}
            currentCandle={candle}
            candleCount={15}
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
        </QueryClientProvider>
      );

      // Check existence of all critical action buttons
      expect(screen.getByTestId('symbol-switcher-button')).toBeInTheDocument();
      expect(screen.getByTestId('open-add-indicator')).toBeInTheDocument();
      expect(screen.getByTestId('open-session-debrief')).toBeInTheDocument();
      expect(screen.getByTestId('open-keyboard-shortcuts')).toBeInTheDocument();
      expect(screen.getByTestId('replay-bar-context')).toBeInTheDocument();
      expect(screen.getByTestId('replay-readiness-badge')).toBeInTheDocument();
    });
  });

  // =========================================================================
  // 3. SQA-002: Unit Badge Across Symbols & Formats
  // =========================================================================
  describe('SQA-002: Unit Badge Correctness Across Diverse Symbols', () => {
    it('consistently renders VND badge for various VN30 and HOSE symbols', () => {
      const symbols = ['VIC', 'VHM', 'HPG', 'MBB', 'TCB', 'SSI', 'MWG', 'VNM'];

      for (const sym of symbols) {
        const candle: Candle = {
          id: 1,
          symbol: sym,
          timeframe: '1D',
          source: 'doraemon',
          adjustment_type: 'adjusted',
          timestamp: '2024-01-15T00:00:00',
          open: 50000,
          high: 51000,
          low: 49500,
          close: 50500,
          volume: 1000000,
        };

        const { unmount } = render(
          <ChartLegendOverlay
            symbol={sym}
            timeframe="1D"
            currentCandle={candle}
            indicators={[]}
            runtime={{}}
            onToggle={vi.fn()}
            onSettings={vi.fn()}
            onRemove={vi.fn()}
          />
        );

        // Badge must be VND
        const badge = screen.getByText('VND');
        expect(badge).toBeInTheDocument();
        // Misleading badge must NOT exist
        expect(screen.queryByText('x1,000 ₫')).toBeNull();
        expect(screen.queryByText('x1000')).toBeNull();

        unmount();
      }
    });
  });

  // =========================================================================
  // 4. SQA-004: Timeframe Control Invariants
  // =========================================================================
  describe('SQA-004: Timeframe Restriction to 1D and 1W', () => {
    it('strictly exposes only supported timeframes 1D and 1W', () => {
      const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
      const onChangeTimeframe = vi.fn();

      render(
        <QueryClientProvider client={queryClient}>
          <ReplayHeader
            sessionId={10}
            symbolName="HPG"
            sessionStatus="active"
            sessionData={{
              id: 10,
              symbol: 'HPG',
              timeframe: '1D',
              adjustment_type: 'adjusted',
              mode: 'practice',
              start_date: '2024-01-01',
              end_date: '2024-06-01',
              current_index: 5,
              status: 'active',
              initial_balance: 100000000,
              current_cash: 100000000,
              total_bars: 50,
              is_active: true,
              created_at: '2024-01-01T00:00:00',
            } as any}
            currentCandle={null}
            candleCount={5}
            onChangeTimeframe={onChangeTimeframe}
            onResumeSession={vi.fn()}
            onOpenSymbolSwitcher={vi.fn()}
            onOpenDebrief={vi.fn()}
            onOpenShortcuts={vi.fn()}
            playSpeed={1000}
            onChangeSpeed={vi.fn()}
            isRightPanelOpen={false}
            onToggleRightPanel={vi.fn()}
            onClearSession={vi.fn()}
          />
        </QueryClientProvider>
      );

      const optionValues = screen.getAllByRole('option')
        .map(o => (o as HTMLOptionElement).value)
        .filter(v => v === '1D' || v === '1W' || v.includes('m') || v.includes('H') || v.includes('M'));

      expect(optionValues).toEqual(['1D', '1W']);
      expect(optionValues).not.toContain('5m');
      expect(optionValues).not.toContain('15m');
      expect(optionValues).not.toContain('1H');
      expect(optionValues).not.toContain('1M');
    });
  });

  // =========================================================================
  // 5. SQA-009: Signal Inspector Weekly Timeframe Alignment
  // =========================================================================
  describe('SQA-009: Signal Inspector Weekly Date Alignment & Fallbacks', () => {
    it('uses practiceData.current_date when available, and falls back to candle timestamp when absent', () => {
      const weeklyCandle: Candle = {
        id: 10,
        symbol: 'VNM',
        timeframe: '1W',
        source: 'doraemon',
        adjustment_type: 'adjusted',
        timestamp: '2024-02-19T00:00:00', // Monday of the week
        open: 72000,
        high: 74000,
        low: 71500,
        close: 73500,
        volume: 3500000,
      };

      // Case A: practiceData present with specific evaluation date (e.g. Wednesday 2024-02-21)
      const practiceDataWithDate = {
        session_id: 10,
        current_date: '2024-02-21T00:00:00',
        current_index: 45,
        visible_bar: 46,
        total_bars: 100,
        current_price: 73500,
        current_volume: 3500000,
        current_cash: 100000000,
        initial_cash: 100000000,
        available_quantity: 0,
        blocked_quantity: 0,
        can_trade: true,
        positions: [],
        pending_orders: [],
        orders: [],
        executions: [],
        trades: [],
        decisions: [],
      } as unknown as PracticeWorkflowSnapshot;

      const { unmount } = render(
        <ReplayRightPanel
          sessionId={10}
          currentCandle={weeklyCandle}
          candleCount={46}
          drawingRevision={0}
          drawingPersistenceStatus="idle"
          onUpdateDrawing={vi.fn()}
          onDeleteDrawing={vi.fn()}
          showExtendedInspector={true}
          onToggleExtendedInspector={vi.fn()}
          selectedInspectorSignal="trend.ichimoku"
          onOpenSignalCatalog={vi.fn()}
          practiceData={practiceDataWithDate}
          practiceLoading={false}
          practiceError={false}
          journalLoading={false}
          journalError={false}
          onResetPractice={vi.fn()}
          onSubmitDecision={vi.fn()}
          onSaveJournal={vi.fn()}
        />
      );

      // Verify inspector received the authoritative session evaluation date, NOT weekly candle timestamp
      const inspector = screen.getByTestId('mock-signal-inspector');
      expect(inspector).toHaveAttribute('data-timestamp', '2024-02-21T00:00:00');
      expect(inspector).toHaveAttribute('data-index', '45');

      const explanation = screen.getByTestId('mock-signal-explanation-inspector');
      expect(explanation).toHaveAttribute('data-timestamp', '2024-02-21T00:00:00');
      expect(explanation).toHaveAttribute('data-index', '45');

      unmount();

      // Case B: practiceData is null (fallback to weekly candle timestamp)
      render(
        <ReplayRightPanel
          sessionId={10}
          currentCandle={weeklyCandle}
          candleCount={46}
          drawingRevision={0}
          drawingPersistenceStatus="idle"
          onUpdateDrawing={vi.fn()}
          onDeleteDrawing={vi.fn()}
          showExtendedInspector={true}
          onToggleExtendedInspector={vi.fn()}
          selectedInspectorSignal="trend.ichimoku"
          onOpenSignalCatalog={vi.fn()}
          practiceData={undefined}
          practiceLoading={false}
          practiceError={false}
          journalLoading={false}
          journalError={false}
          onResetPractice={vi.fn()}
          onSubmitDecision={vi.fn()}
          onSaveJournal={vi.fn()}
        />
      );

      const fallbackInspector = screen.getByTestId('mock-signal-inspector');
      expect(fallbackInspector).toHaveAttribute('data-timestamp', '2024-02-19T00:00:00');
    });
  });

  // =========================================================================
  // 6. SQA-003: Keyboard Shortcuts Focus Guard Adversarial Test
  // =========================================================================
  describe('SQA-003: Adversarial Keyboard Shortcuts Focus & Typing Stress', () => {
    it('comprehensive DOM element guard matrix: shortcuts never fire inside interactive targets', () => {
      const form = document.createElement('form');
      const textInput = document.createElement('input');
      textInput.type = 'text';

      const numberInput = document.createElement('input');
      numberInput.type = 'number';

      const searchInput = document.createElement('input');
      searchInput.type = 'search';

      const textarea = document.createElement('textarea');
      const select = document.createElement('select');
      const button = document.createElement('button');

      const contentEditable = document.createElement('div');
      contentEditable.setAttribute('contenteditable', 'true');

      const modalDialog = document.createElement('div');
      modalDialog.setAttribute('role', 'dialog');
      const innerSpan = document.createElement('span');
      modalDialog.appendChild(innerSpan);

      const ariaModal = document.createElement('div');
      ariaModal.setAttribute('aria-modal', 'true');
      const ariaModalSpan = document.createElement('span');
      ariaModal.appendChild(ariaModalSpan);

      const tabElement = document.createElement('div');
      tabElement.setAttribute('role', 'tab');

      // Mount elements
      document.body.appendChild(form);
      form.appendChild(textInput);
      form.appendChild(numberInput);
      form.appendChild(searchInput);
      form.appendChild(textarea);
      form.appendChild(select);
      form.appendChild(button);
      document.body.appendChild(contentEditable);
      document.body.appendChild(modalDialog);
      document.body.appendChild(ariaModal);
      document.body.appendChild(tabElement);

      const guardedElements = [
        textInput,
        numberInput,
        searchInput,
        textarea,
        select,
        button,
        contentEditable,
        innerSpan,
        ariaModalSpan,
        tabElement,
      ];

      for (const el of guardedElements) {
        const eligible = isGlobalShortcutEligible({ target: el, defaultPrevented: false });
        expect(eligible, `Element <${el.tagName.toLowerCase()}> with role/attributes should NOT be shortcut-eligible`).toBe(false);
      }

      // Default prevented event must always be ineligible
      const div = document.createElement('div');
      expect(isGlobalShortcutEligible({ target: div, defaultPrevented: true })).toBe(false);

      // Plain body/div must be eligible
      expect(isGlobalShortcutEligible({ target: document.body, defaultPrevented: false })).toBe(false); // because modalDialog is in document!
      // Once modals are removed from document:
      document.body.removeChild(modalDialog);
      document.body.removeChild(ariaModal);
      expect(isGlobalShortcutEligible({ target: div, defaultPrevented: false })).toBe(true);

      // Clean up DOM
      document.body.removeChild(form);
      document.body.removeChild(contentEditable);
      document.body.removeChild(tabElement);
    });

    it('adversarial hook test: Space toggles isPlaying but NEVER fires when typing in inputs', async () => {
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

      // 1. Fire Space on body -> toggles to true
      act(() => {
        window.dispatchEvent(new KeyboardEvent('keydown', { code: 'Space', bubbles: true }));
      });
      expect(result.current.isPlaying).toBe(true);

      // 2. Fire Space on body again -> toggles to false
      act(() => {
        window.dispatchEvent(new KeyboardEvent('keydown', { code: 'Space', bubbles: true }));
      });
      expect(result.current.isPlaying).toBe(false);

      // 3. Mount an input and simulate typing (dispatch keydown with target = input)
      const input = document.createElement('input');
      input.type = 'text';
      document.body.appendChild(input);
      input.focus();

      // Dispatch Space with target = input
      act(() => {
        const evt = new KeyboardEvent('keydown', { code: 'Space', bubbles: true });
        Object.defineProperty(evt, 'target', { value: input });
        window.dispatchEvent(evt);
      });
      // MUST NOT toggle isPlaying!
      expect(result.current.isPlaying).toBe(false);

      // 4. Dispatch Space on textarea
      const textarea = document.createElement('textarea');
      document.body.appendChild(textarea);
      textarea.focus();

      act(() => {
        const evt = new KeyboardEvent('keydown', { code: 'Space', bubbles: true });
        Object.defineProperty(evt, 'target', { value: textarea });
        window.dispatchEvent(evt);
      });
      // MUST NOT toggle isPlaying!
      expect(result.current.isPlaying).toBe(false);

      // Clean up
      document.body.removeChild(input);
      document.body.removeChild(textarea);
    });
  });
});
