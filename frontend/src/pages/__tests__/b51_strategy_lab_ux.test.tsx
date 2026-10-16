import React from 'react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { StrategyLabPage } from '../StrategyLabPage';
import { Sidebar } from '../../components/layout/Sidebar';
import { DrawingToolbar } from '../../components/chart/DrawingToolbar';
import { IndicatorPaneChrome } from '../../components/chart/IndicatorPaneChrome';
import { isGlobalShortcutEligible } from '../../features/replay/globalShortcutPolicy';
import * as backtestApi from '../../api/backtestApi';
import * as signalsApi from '../../api/signalsApi';

vi.mock('../../api/backtestApi', () => ({
  getStrategies: vi.fn(),
  getAvailableStrategies: vi.fn(),
  runBacktest: vi.fn(),
  runParameterSweep: vi.fn(),
  cancelParameterSweep: vi.fn(),
  getStrategyLabRuns: vi.fn(),
  saveStrategyLabRun: vi.fn(),
  deleteStrategyLabRun: vi.fn(),
  getSavedRules: vi.fn(),
  createSavedRule: vi.fn(),
  deleteSavedRule: vi.fn(),
}));

vi.mock('../../api/signalsApi', () => ({
  getSignalRegistry: vi.fn(),
}));

vi.mock('../../api/strategyLabApi', () => ({
  getStrategyLabRuns: vi.fn().mockResolvedValue([]),
  saveStrategyLabRun: vi.fn().mockResolvedValue({}),
  deleteStrategyLabRun: vi.fn().mockResolvedValue({}),
  runParameterSweep: vi.fn(),
  cancelParameterSweep: vi.fn(),
}));

vi.mock('../../api/replayApi', () => ({
  getReplaySessions: vi.fn().mockResolvedValue([]),
}));

const renderWithProviders = (ui: React.ReactElement, initialEntries = ['/strategy-lab']) => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={initialEntries}>{ui}</MemoryRouter>
    </QueryClientProvider>,
  );
};

describe('B51 Strategy Lab and Application UX (GAP-U-03..13)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
    const mockStrats = [
      {
        filename: 'ema_crossover.yaml',
        name: 'EMA Trend Following',
        description: 'EMA crossover trend strategy',
        config: {
          name: 'EMA Trend Following',
          version: '1.0',
          indicators: [{ name: 'ema_20', type: 'ema', params: { period: 20 } }],
          entry: { logic: 'AND', conditions: [] },
          exit: { logic: 'OR', conditions: [] },
        },
      },
    ];
    vi.mocked(backtestApi.getAvailableStrategies).mockResolvedValue(mockStrats);
    vi.mocked(signalsApi.getSignalRegistry).mockResolvedValue({
      signals: [
        {
          name: 'momentum.rsi',
          ast_alias: 'rsi_14',
          category: 'momentum',
          version: '1.0.0',
          label_vi: 'RSI (14)',
          output_type: 'float',
          warmup_bars: 14,
          causal_delay_bars: 0,
          status: 'ACTIVE',
          parameters_schema: {},
          default_parameters: {},
          dependencies: [],
          description: 'RSI (14)',
        },
      ],
    } as unknown as Awaited<ReturnType<typeof signalsApi.getSignalRegistry>>);
    vi.mocked(backtestApi.runBacktest).mockResolvedValue({
      status: 'succeeded',
      summary: {
        initial_cash: 100_000_000,
        final_equity: 112_500_000,
        total_return_pct: 12.5,
        total_trades: 6,
        win_rate: 0.6667,
        total_net_pnl: 12_500_000,
      },
      analytics: {
        total_trades: 6,
        win_rate: 0.6667,
        total_net_pnl: 12_500_000,
        profit_factor: 1.85,
        max_drawdown_pct: 4.2,
        expectancy: 2_083_333,
        equity_curve: [],
        metrics: {
          total_net_pnl: { value: 12_500_000, status: 'valid', code: 'VALID', message: 'OK' },
          win_rate: { value: 0.6667, status: 'valid', code: 'VALID', message: 'OK' },
          profit_factor: { value: 1.85, status: 'valid', code: 'VALID', message: 'OK' },
        },
      },
      trades: [],
      diagnostics: [],
    } as unknown as backtestApi.BacktestResponse);
  });

  it('completes StrategyRuleBuilder -> StrategyLab Battle handoff and executes custom AST rule (GAP-U-03)', async () => {
    renderWithProviders(<StrategyLabPage />, ['/strategy-lab?tab=builder']);

    const runBtn = await screen.findByTestId('execute-rule-btn');
    fireEvent.click(runBtn);

    // Should switch to battle tab and display the prefill banner
    expect(await screen.findByTestId('pending-rule-prefill-banner')).toBeInTheDocument();
    expect(screen.getByTestId('single-vs-battle-banner')).toBeInTheDocument();
    expect(screen.getByTestId('market-assumptions-banner')).toHaveTextContent(/T\+1\.5/);
    expect(screen.getByTestId('market-assumptions-banner')).toHaveTextContent(/Next-Open/);

    // Execute comparison and verify custom AST strategy config is sent to runBacktest
    const compareBtn = screen.getByRole('button', { name: /Compare Strategies/i });
    fireEvent.click(compareBtn);

    await waitFor(() => {
      expect(backtestApi.runBacktest).toHaveBeenCalled();
    });
    const calls = vi.mocked(backtestApi.runBacktest).mock.calls;
    const customCall = calls.find(
      ([req]) => (req.strategy as Record<string, unknown>)?.entry_rules !== undefined,
    );
    expect(customCall).toBeDefined();
    expect(customCall?.[0].strategy).toMatchObject({
      indicators: expect.arrayContaining([expect.objectContaining({ type: 'signal' })]),
      entry_rules: expect.arrayContaining([expect.objectContaining({ condition: expect.any(String) })]),
    });
  });

  it('renders Single Backtest tab inside StrategyLabPage and shares pending rule handoff (GAP-U-05)', async () => {
    renderWithProviders(<StrategyLabPage />, ['/strategy-lab?tab=builder']);

    const singleHandoffBtn = await screen.findByTestId('switch-to-single-btn');
    fireEvent.click(singleHandoffBtn);

    expect(await screen.findByTestId('pending-rule-prefill-banner')).toBeInTheDocument();
    expect(screen.getByTestId('single-vs-battle-banner')).toBeInTheDocument();
    expect(screen.getByTestId('market-assumptions-banner')).toHaveTextContent(/0\.15%/);
  });

  it('highlights the correct Sidebar navigation item across Strategy Lab tabs (GAP-U-06)', () => {
    const { unmount } = render(
      <MemoryRouter initialEntries={['/strategy-lab?tab=single']}>
        <Sidebar isCollapsed={false} onToggleCollapse={() => {}} />
      </MemoryRouter>,
    );
    const labLink = screen.getByTestId('nav-item-strategy-lab');
    const builderLink = screen.getByTestId('nav-item-rule-builder');
    expect(labLink.className).toContain('active');
    expect(builderLink.className).not.toContain('active');
    unmount();

    render(
      <MemoryRouter initialEntries={['/strategy-lab?tab=builder']}>
        <Sidebar isCollapsed={false} onToggleCollapse={() => {}} />
      </MemoryRouter>,
    );
    expect(screen.getByTestId('nav-item-rule-builder').className).toContain('active');
    expect(screen.getByTestId('nav-item-strategy-lab').className).not.toContain('active');
  });

  it('renders Undo/Redo buttons with shortcut tooltips and high-contrast enabled state (GAP-U-04)', () => {
    const onUndo = vi.fn();
    const onRedo = vi.fn();
    render(
      <DrawingToolbar
        activeTool="select"
        onSelectTool={() => {}}
        onClearAll={() => {}}
        onUndo={onUndo}
        onRedo={onRedo}
        canUndo={true}
        canRedo={false}
        pendingText={false}
        onCommitText={() => true}
        onCancelText={() => {}}
        magnetMode="off"
        onMagnetMode={() => {}}
        persistenceStatus="ready"
      />,
    );

    const undoBtn = screen.getByTestId('undo-drawing');
    const redoBtn = screen.getByTestId('redo-drawing');
    expect(undoBtn).toHaveAttribute('title', expect.stringContaining('Ctrl+Z'));
    expect(redoBtn).toHaveAttribute('title', expect.stringContaining('Ctrl+Y'));
    expect(undoBtn).not.toBeDisabled();
    expect(redoBtn).toBeDisabled();

    fireEvent.click(undoBtn);
    expect(onUndo).toHaveBeenCalledTimes(1);
  });

  it('renders explanatory Vietnamese tooltip on warming indicator badge while preserving warming (X/Y) text (GAP-U-07)', () => {
    render(
      <IndicatorPaneChrome
        document={{
          schemaVersion: 1,
          sessionId: 1,
          instances: [
            {
              id: 'rsi-1',
              definitionId: 'rsi',
              label: 'RSI',
              paneId: 'indicator:rsi-1',
              placement: 'oscillator',
              visible: true,
              order: 0,
              params: { length: 14 },
              styles: {},
            },
          ],
        }}
        runtime={{
          'rsi-1': {
            status: 'warming',
            availableBars: 8,
            warmupBars: 14,
            values: {},
          },
        }}
        onSettings={() => {}}
        onToggle={() => {}}
        onRemove={() => {}}
      />,
    );

    const badge = screen.getByTestId('indicator-warming-badge-rsi-1');
    expect(badge).toHaveTextContent('warming (8/14)');
    expect(badge).toHaveAttribute('title', 'Đang làm ấm chỉ báo: đã có 8/14 nến cần thiết');
  });

  it('blocks global shortcuts when a button is focused or a modal dialog is open (GAP-U-13)', () => {
    const btn = document.createElement('button');
    document.body.appendChild(btn);
    btn.focus();
    expect(isGlobalShortcutEligible({ target: document.body, defaultPrevented: false })).toBe(false);
    document.body.removeChild(btn);

    const dialog = document.createElement('div');
    dialog.setAttribute('role', 'dialog');
    document.body.appendChild(dialog);
    expect(isGlobalShortcutEligible({ target: document.body, defaultPrevented: false })).toBe(false);
    document.body.removeChild(dialog);
  });
});
