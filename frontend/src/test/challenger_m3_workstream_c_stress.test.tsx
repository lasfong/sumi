import React from 'react';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import '@testing-library/jest-dom';

import { StrategyRuleBuilder } from '../components/strategy/StrategyRuleBuilder';
import { TechnicalFlowBBViewer } from '../components/chart/TechnicalFlowBBViewer';
import { ScannerPage } from '../pages/ScannerPage';
import { BacktestPage } from '../pages/BacktestPage';

import * as signalsApi from '../api/signalsApi';
import * as bbApi from '../api/bbApi';
import * as scannerApi from '../api/scannerApi';
import * as backtestApi from '../api/backtestApi';

// Mock dependencies
vi.mock('../api/signalsApi', () => ({
  getSignalRegistry: vi.fn(),
}));

vi.mock('../api/bbApi', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../api/bbApi')>();
  return {
    ...actual,
    getBBHorizons: vi.fn(),
    getSymbolBB: vi.fn(),
  };
});

vi.mock('../api/scannerApi', () => ({
  runScanner: vi.fn(),
  createReplaySessionFromSignal: vi.fn(),
  listScannerRuns: vi.fn(),
}));

vi.mock('../api/backtestApi', () => ({
  getAvailableStrategies: vi.fn(),
  runBacktest: vi.fn(),
}));

vi.mock('../components/analytics/EquityChart', () => ({
  EquityChart: () => <div data-testid="mock-equity-chart">Mock Equity Chart</div>,
}));

const renderWithProviders = (ui: React.ReactElement) => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>{ui}</MemoryRouter>
    </QueryClientProvider>
  );
};

// ===========================================================================
// SQA-010: StrategyRuleBuilder Adversarial Stress Tests
// ===========================================================================

describe('SQA-010: StrategyRuleBuilder Adversarial UI Stress Tests', () => {
  const mockRegistry = {
    signals: [
      {
        name: 'volume.relative_volume',
        version: '1.0.0',
        category: 'volume',
        label_vi: 'Khối Lượng Tương Đối (RVOL)',
        description: 'Volume / 20-day SMA volume',
        output_type: 'float' as const,
        parameters_schema: {},
        default_parameters: {},
        dependencies: [],
        warmup_bars: 20,
        causal_delay_bars: 0,
        status: 'ACTIVE' as const,
        ast_alias: 'volume__relative_volume',
      },
      {
        name: 'health.favorable',
        version: '1.0.0',
        category: 'health',
        label_vi: 'Sức Khỏe Kỹ Thuật Thuận Lợi',
        description: 'Health score >= +35.0',
        output_type: 'bool' as const,
        parameters_schema: {},
        default_parameters: {},
        dependencies: [],
        warmup_bars: 50,
        causal_delay_bars: 0,
        status: 'ACTIVE' as const,
        ast_alias: 'health__favorable',
      },
    ],
  };

  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(signalsApi.getSignalRegistry).mockResolvedValue(mockRegistry);
  });

  it('rejects various adversarial non-numeric literals and keeps action buttons strictly disabled', async () => {
    const onExecuteRule = vi.fn();
    renderWithProviders(<StrategyRuleBuilder onExecuteRule={onExecuteRule} />);

    await waitFor(() => {
      expect(screen.getByTestId('add-clause-btn')).toBeInTheDocument();
    });

    // Add a numeric clause
    fireEvent.click(screen.getByTestId('add-clause-btn'));
    const signalSelect = screen.getByTestId('clause-signal-select-2');
    fireEvent.change(signalSelect, { target: { value: 'volume__relative_volume' } });

    const compareInput = screen.getByTestId('clause-compare-value-2');
    const executeBtn = screen.getByTestId('execute-rule-btn');
    const battleBtn = screen.getByTestId('switch-to-battle-btn');
    const singleBtn = screen.getByTestId('switch-to-single-btn');
    const copyBtn = screen.getByTestId('copy-rule-yaml-btn');
    const statusBanner = screen.getByTestId('ast-expression-status');

    const invalidInputs = [
      'abc_invalid',
      '   ',
      '',
      '--',
      '++1',
      '$500',
      '1.2.3.4',
      'Infinity',
      '-Infinity',
      'NaN',
      'null',
      'undefined',
      '{}',
      '[]',
      '10%',
    ];

    for (const hostileVal of invalidInputs) {
      fireEvent.change(compareInput, { target: { value: hostileVal } });

      // Inline error should appear
      const errorMsg = screen.getByTestId('clause-error-2');
      expect(errorMsg).toBeInTheDocument();

      // AST banner must signal invalid expression
      expect(statusBanner).toHaveTextContent(/Không Hợp Lệ/i);

      // All action buttons must be disabled
      expect(executeBtn).toBeDisabled();
      expect(battleBtn).toBeDisabled();
      expect(singleBtn).toBeDisabled();
      expect(copyBtn).toBeDisabled();

      // Clicking execute button must not trigger callback
      fireEvent.click(executeBtn);
      expect(onExecuteRule).not.toHaveBeenCalled();
    }

    // Now test valid numbers to confirm recovery
    const validInputs = ['0', '-0', '-12.5', '1000000', '1.5e3', '0.0005'];
    for (const validVal of validInputs) {
      fireEvent.change(compareInput, { target: { value: validVal } });

      expect(screen.queryByTestId('clause-error-2')).not.toBeInTheDocument();
      expect(statusBanner).toHaveTextContent(/Hợp Lệ/i);
      expect(executeBtn).not.toBeDisabled();
      expect(battleBtn).not.toBeDisabled();
      expect(singleBtn).not.toBeDisabled();
      expect(copyBtn).not.toBeDisabled();
    }
  });

  it('restores validity when an invalid clause is removed from multi-clause rule', async () => {
    renderWithProviders(<StrategyRuleBuilder />);

    await waitFor(() => {
      expect(screen.getByTestId('add-clause-btn')).toBeInTheDocument();
    });

    // Add clause 2
    fireEvent.click(screen.getByTestId('add-clause-btn'));
    const signalSelect = screen.getByTestId('clause-signal-select-2');
    fireEvent.change(signalSelect, { target: { value: 'volume__relative_volume' } });

    // Break clause 2 with invalid input
    const compareInput = screen.getByTestId('clause-compare-value-2');
    fireEvent.change(compareInput, { target: { value: 'corrupt_string' } });

    expect(screen.getByTestId('execute-rule-btn')).toBeDisabled();

    // Remove clause 2
    const removeButtons = screen.getAllByTitle('Xóa mệnh đề');
    fireEvent.click(removeButtons[2]);

    // Rule builder should return to 2 valid clauses and buttons re-enable
    expect(screen.queryByTestId('clause-row-2')).not.toBeInTheDocument();
    expect(screen.getByTestId('ast-expression-status')).toHaveTextContent(/Hợp Lệ/i);
    expect(screen.getByTestId('execute-rule-btn')).not.toBeDisabled();
  });
});

// ===========================================================================
// SQA-016: TechnicalFlowBBViewer Quality & Metrics Stress Tests
// ===========================================================================

describe('SQA-016: TechnicalFlowBBViewer Quality & Metrics Stress Tests', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(bbApi.getBBHorizons).mockResolvedValue({
      horizons: [{ id: 'T20', lookback_bars: 20 }],
      default_horizons: ['T20'],
      methodology: 'OHLCV_PROXY',
    });
  });

  it('displays UNAVAILABLE when points are empty or coverage ratio is 0.0', async () => {
    const emptyPointsResponse = {
      symbol: 'TEST_SYM',
      timeframe: '1D',
      methodology: 'OHLCV_PROXY',
      calculation_as_of: '2026-03-01',
      coverage_ratio: 0.0,
      publication_status: 'UNAVAILABLE_DEGRADED',
      points: [],
      summary: {},
    };
    vi.mocked(bbApi.getSymbolBB).mockResolvedValue(emptyPointsResponse as unknown as bbApi.BBSymbolSeriesResponse);

    renderWithProviders(<TechnicalFlowBBViewer defaultSymbol="TEST_SYM" />);

    await waitFor(() => {
      expect(screen.getByTestId('bb-data-quality-badge')).toBeInTheDocument();
    });

    const badge = screen.getByTestId('bb-data-quality-badge');
    expect(badge).toHaveTextContent(/CHẤT LƯỢNG DỮ LIỆU: UNAVAILABLE/i);
    expect(badge).not.toHaveTextContent('HIGH');
  });

  it('displays DEGRADED when coverage is between 0.0 and 0.8', async () => {
    const lowCoverageResponse = {
      symbol: 'TEST_SYM',
      timeframe: '1D',
      methodology: 'OHLCV_PROXY',
      calculation_as_of: '2026-03-01',
      coverage_ratio: 0.75, // < 0.8
      publication_status: 'NORMAL',
      points: [
        {
          date: '2026-03-01',
          value: 40.0,
          basis: 35.0,
          regime: 'NEUTRAL',
          horizons: { T20: { horizon: 'T20', bb_value: 40.0, raw_denominator: 35.0 } },
        },
      ],
      summary: {},
    };
    vi.mocked(bbApi.getSymbolBB).mockResolvedValue(lowCoverageResponse as unknown as bbApi.BBSymbolSeriesResponse);

    renderWithProviders(<TechnicalFlowBBViewer defaultSymbol="TEST_SYM" />);

    await waitFor(() => {
      expect(screen.getByTestId('bb-data-quality-badge')).toHaveTextContent(/CHẤT LƯỢNG DỮ LIỆU: DEGRADED/i);
    });
  });

  it('renders daily pressure with proper color polarity and daily trading value formatting', async () => {
    const responseWithMetrics = {
      symbol: 'TEST_SYM',
      timeframe: '1D',
      methodology: 'OHLCV_PROXY',
      calculation_as_of: '2026-03-01',
      coverage_ratio: 0.95,
      points: [
        {
          date: '2026-03-01',
          value: 60.0,
          basis: 50.0,
          daily_pressure: -2.45, // negative -> red
          daily_trading_value: 48000000000,
          daily_quality: 'HIGH',
          regime: 'POSITIVE',
          horizons: { T20: { horizon: 'T20', bb_value: 60.0, raw_denominator: 50.0 } },
        },
      ],
      summary: {},
    };
    vi.mocked(bbApi.getSymbolBB).mockResolvedValue(responseWithMetrics as unknown as bbApi.BBSymbolSeriesResponse);

    renderWithProviders(<TechnicalFlowBBViewer defaultSymbol="TEST_SYM" />);

    await waitFor(() => {
      expect(screen.getByTestId('bb-daily-pressure')).toBeInTheDocument();
      expect(screen.getByTestId('bb-daily-trading-value')).toBeInTheDocument();
    });

    const pressureEl = screen.getByTestId('bb-daily-pressure');
    expect(pressureEl).toHaveTextContent('-2.45');
    // Color should reflect negative value (var(--color-sell))
    expect(pressureEl).toHaveStyle({ color: 'var(--color-sell)' });

    const tradingValEl = screen.getByTestId('bb-daily-trading-value');
    expect(tradingValEl).toHaveTextContent('48.000.000.000 đ');
  });
});

// ===========================================================================
// SQA-015 & SQA-005: ScannerPage History Sync, Filtering & Numeric Sorting
// ===========================================================================

describe('SQA-015 & SQA-005: Scanner History Sync, Filtering & Numeric Sorting', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(backtestApi.getAvailableStrategies).mockResolvedValue([
      { filename: 'strat1.yaml', name: 'Strategy 1', description: 'Test', config: { name: 'Strategy 1' } },
    ]);
  });

  it('switches between multiple historical runs without cross-run contamination (SQA-015)', async () => {
    const runA: scannerApi.ScannerRun = {
      id: 101,
      label: 'Run 101 (FPT, MWG)',
      status: 'succeeded',
      total_results: 2,
      created_at: '2024-01-01T10:00:00Z',
      request_config: { symbols: ['FPT', 'MWG'] },
      result_payload: {
        status: 'succeeded',
        total_results: 2,
        truncated: false,
        warnings: [],
        results: [
          { symbol: 'FPT', timestamp: '2024-01-01T00:00:00', signal_type: 'entry', strategy: 'S1', price: 95.0, regime: 'bullish' },
          { symbol: 'MWG', timestamp: '2024-01-01T00:00:00', signal_type: 'exit', strategy: 'S1', price: 45.0, regime: 'neutral' },
        ],
      },
    };

    const runB: scannerApi.ScannerRun = {
      id: 102,
      label: 'Run 102 (VIC)',
      status: 'succeeded',
      total_results: 1,
      created_at: '2024-01-02T10:00:00Z',
      request_config: { symbols: ['VIC'] },
      result_payload: {
        status: 'succeeded',
        total_results: 1,
        truncated: false,
        warnings: [],
        results: [
          { symbol: 'VIC', timestamp: '2024-01-02T00:00:00', signal_type: 'entry', strategy: 'S1', price: 42.0, regime: 'bearish' },
        ],
      },
    };

    vi.mocked(scannerApi.listScannerRuns).mockResolvedValue([runA, runB]);

    renderWithProviders(<ScannerPage />);
    await screen.findByText(/Strategy 1/i);

    // Click Run 101
    fireEvent.click(await screen.findByTestId('history-run-101'));
    await screen.findByText('FPT');
    expect(screen.getByText('MWG')).toBeInTheDocument();
    expect(screen.queryByText('VIC')).not.toBeInTheDocument();

    // Click Run 102
    fireEvent.click(await screen.findByTestId('history-run-102'));
    await screen.findByText('VIC');
    expect(screen.queryByText('FPT')).not.toBeInTheDocument();
    expect(screen.queryByText('MWG')).not.toBeInTheDocument();

    // Click Run 101 again to verify flawless back-and-forth switching
    fireEvent.click(await screen.findByTestId('history-run-101'));
    await screen.findByText('FPT');
    expect(screen.getByText('MWG')).toBeInTheDocument();
    expect(screen.queryByText('VIC')).not.toBeInTheDocument();
  });

  it('performs strictly numeric price sorting and multi-field combined filtering (SQA-005)', async () => {
    vi.mocked(scannerApi.listScannerRuns).mockResolvedValue([]);
    vi.mocked(scannerApi.runScanner).mockResolvedValue({
      status: 'succeeded',
      total_results: 4,
      truncated: false,
      warnings: [],
      results: [
        { symbol: 'AAA', timestamp: '2024-01-10T00:00:00', signal_type: 'entry', strategy: 'S1', price: 10.0, regime: 'bullish' },
        { symbol: 'BBB', timestamp: '2024-01-12T00:00:00', signal_type: 'exit', strategy: 'S1', price: 100.0, regime: 'bearish' },
        { symbol: 'CCC', timestamp: '2024-01-15T00:00:00', signal_type: 'entry', strategy: 'S1', price: 50.0, regime: 'bullish' },
        { symbol: 'DDD', timestamp: '2024-01-20T00:00:00', signal_type: 'exit', strategy: 'S1', price: 5.0, regime: 'bearish' },
      ],
    });

    renderWithProviders(<ScannerPage />);
    await screen.findByText(/Strategy 1/i);
    fireEvent.click(screen.getByRole('button', { name: /Run Scanner/i }));

    await screen.findByText('AAA');
    expect(screen.getByText('BBB')).toBeInTheDocument();
    expect(screen.getByText('CCC')).toBeInTheDocument();
    expect(screen.getByText('DDD')).toBeInTheDocument();

    // --- NUMERIC SORT TEST ---
    // If sorted lexicographically as strings, "100.0" comes before "5.0".
    // Under numeric sort:
    // Ascending: 5.0 (DDD), 10.0 (AAA), 50.0 (CCC), 100.0 (BBB)
    const priceTh = screen.getByTestId('th-sort-price');
    fireEvent.click(priceTh); // Click 1: Ascending

    const rowsAsc = screen.getAllByRole('row');
    // Row 1 should be DDD (price 5.0)
    expect(rowsAsc[1]).toHaveTextContent('DDD');
    // Row 2 should be AAA (price 10.0)
    expect(rowsAsc[2]).toHaveTextContent('AAA');
    // Row 3 should be CCC (price 50.0)
    expect(rowsAsc[3]).toHaveTextContent('CCC');
    // Row 4 should be BBB (price 100.0)
    expect(rowsAsc[4]).toHaveTextContent('BBB');

    // Click 2: Descending
    fireEvent.click(priceTh);
    const rowsDesc = screen.getAllByRole('row');
    expect(rowsDesc[1]).toHaveTextContent('BBB'); // 100.0
    expect(rowsDesc[4]).toHaveTextContent('DDD'); // 5.0

    // --- COMBINED FILTER TEST ---
    // Filter by signal: entry (should leave AAA and CCC)
    const signalSelect = screen.getByTestId('scanner-signal-filter');
    fireEvent.change(signalSelect, { target: { value: 'entry' } });

    expect(screen.getByText('AAA')).toBeInTheDocument();
    expect(screen.getByText('CCC')).toBeInTheDocument();
    expect(screen.queryByText('BBB')).not.toBeInTheDocument();
    expect(screen.queryByText('DDD')).not.toBeInTheDocument();

    // Filter by search: "CC" (should leave only CCC)
    const filterInput = screen.getByTestId('scanner-filter-input');
    fireEvent.change(filterInput, { target: { value: 'CC' } });

    expect(screen.getByText('CCC')).toBeInTheDocument();
    expect(screen.queryByText('AAA')).not.toBeInTheDocument();
    expect(screen.getByTestId('scanner-results-count')).toHaveTextContent('Hiển thị 1 / 4 tín hiệu');

    // Non-matching filter: "XYZ" -> zero results message
    fireEvent.change(filterInput, { target: { value: 'XYZ' } });
    expect(screen.getByTestId('no-filtered-results')).toHaveTextContent(/Không tìm thấy tín hiệu nào khớp với bộ lọc/i);
    expect(screen.getByTestId('scanner-results-count')).toHaveTextContent('Hiển thị 0 / 4 tín hiệu');
  });
});

// ===========================================================================
// SQA-011: BacktestPage Executed Trades Table Rendering Fidelity
// ===========================================================================

describe('SQA-011: BacktestPage Executed Trades Table Rendering Fidelity', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(backtestApi.getAvailableStrategies).mockResolvedValue([
      { filename: 'strategy.yaml', name: 'Test Strat', description: 'Test', config: {} },
    ]);
  });

  it('renders executed trades table with closed win, closed loss, and open trade states', async () => {
    vi.mocked(backtestApi.runBacktest).mockResolvedValue({
      session_id: 999,
      strategy: 'Test Strat',
      symbol: 'VND',
      total_candles: 120,
      trades: [
        {
          id: 1,
          symbol: 'VND',
          entry_date: '2024-01-05T09:00:00Z',
          entry_price: 20000,
          exit_date: '2024-01-15T15:00:00Z',
          exit_price: 23000,
          quantity: 2000,
          gross_pnl: 6000000,
          net_pnl: 5700000,
          pnl_percent: 14.25,
          holding_candles: 7,
          status: 'closed',
          result: 'win',
        },
        {
          id: 2,
          symbol: 'VND',
          entry_date: '2024-02-01T09:00:00Z',
          entry_price: 24000,
          exit_date: '2024-02-08T15:00:00Z',
          exit_price: 22000,
          quantity: 1500,
          gross_pnl: -3000000,
          net_pnl: -3200000,
          pnl_percent: -8.89,
          holding_candles: 5,
          status: 'closed',
          result: 'loss',
        },
        {
          id: 3,
          symbol: 'VND',
          entry_date: '2024-03-01T09:00:00Z',
          entry_price: 21500,
          exit_date: null,
          exit_price: null,
          quantity: 1000,
          gross_pnl: 0,
          net_pnl: 0,
          pnl_percent: 0.0,
          holding_candles: 3,
          status: 'open',
          result: 'open',
        },
      ],
      analytics: {
        total_net_pnl: 2500000,
        win_rate: 0.5,
        sharpe_ratio: 1.2,
        max_drawdown: -3200000,
        total_trades: 2,
        average_win: 5700000,
        average_loss: 3200000,
        profit_factor: 1.78,
        sqn: 0.9,
        metrics: {},
        equity_curve: [],
      },
    });

    renderWithProviders(<BacktestPage />);
    await waitFor(() => expect(screen.getByRole('combobox')).toHaveValue('strategy.yaml'));
    fireEvent.click(screen.getByRole('button', { name: /Run Backtest/i }));

    await waitFor(() => {
      expect(backtestApi.runBacktest).toHaveBeenCalled();
    });

    await screen.findByTestId('backtest-executed-trades');
    expect(screen.getByText(/Danh sách Lệnh Đã Thực Hiện \(3 lệnh\)/i)).toBeInTheDocument();

    // Verify Trade 1 (Win)
    const row1 = screen.getByTestId('trade-row-1');
    expect(within(row1).getByText('20,000.00')).toBeInTheDocument();
    expect(within(row1).getByText('23,000.00')).toBeInTheDocument();
    expect(within(row1).getByText('5,700,000.00')).toBeInTheDocument();
    expect(within(row1).getByText('14.25%')).toBeInTheDocument();
    expect(within(row1).getByText('WIN')).toBeInTheDocument();
    expect(within(row1).getByText('7 nến')).toBeInTheDocument();

    // Verify Trade 2 (Loss)
    const row2 = screen.getByTestId('trade-row-2');
    expect(within(row2).getByText('24,000.00')).toBeInTheDocument();
    expect(within(row2).getByText('22,000.00')).toBeInTheDocument();
    expect(within(row2).getByText('-3,200,000.00')).toBeInTheDocument();
    expect(within(row2).getByText('-8.89%')).toBeInTheDocument();
    expect(within(row2).getByText('LOSS')).toBeInTheDocument();
    expect(within(row2).getByText('5 nến')).toBeInTheDocument();

    // Verify Trade 3 (Open)
    const row3 = screen.getByTestId('trade-row-3');
    expect(within(row3).getByText('21,500.00')).toBeInTheDocument();
    expect(within(row3).getByText('Đang mở')).toBeInTheDocument();
    expect(within(row3).getByText('OPEN')).toBeInTheDocument();
  });

  it('aggregates and renders trades across all symbols in multi-symbol backtest run', async () => {
    vi.mocked(backtestApi.runBacktest).mockResolvedValue({
      session_id: 1000,
      strategy: 'Test Strat',
      symbol: 'PORTFOLIO',
      total_candles: 240,
      analytics: null,
      runs: [
        {
          symbol: 'FPT',
          status: 'succeeded',
          total_candles: 120,
          analytics: null,
          trades: [
            {
              id: 1,
              symbol: 'FPT',
              entry_date: '2024-01-10T00:00:00Z',
              entry_price: 90000,
              exit_date: '2024-01-20T00:00:00Z',
              exit_price: 99000,
              quantity: 500,
              net_pnl: 4500000,
              pnl_percent: 10.0,
              holding_candles: 8,
              status: 'closed',
              result: 'win',
            },
          ],
        },
        {
          symbol: 'MWG',
          status: 'succeeded',
          total_candles: 120,
          analytics: null,
          trades: [
            {
              id: 2,
              symbol: 'MWG',
              entry_date: '2024-01-15T00:00:00Z',
              entry_price: 45000,
              exit_date: '2024-01-25T00:00:00Z',
              exit_price: 42000,
              quantity: 1000,
              net_pnl: -3000000,
              pnl_percent: -6.67,
              holding_candles: 7,
              status: 'closed',
              result: 'loss',
            },
          ],
        },
      ],
      summary: {
        total_symbols: 2,
        succeeded_symbols: 2,
        failed_symbols: 0,
        total_candles: 240,
        total_trades: 2,
        win_rate: 0.5,
        total_net_pnl: 1500000,
      },
    });

    renderWithProviders(<BacktestPage />);
    await waitFor(() => expect(screen.getByRole('combobox')).toHaveValue('strategy.yaml'));
    fireEvent.change(screen.getByLabelText(/Symbol\(s\)/i), { target: { value: 'FPT, MWG' } });
    fireEvent.click(screen.getByRole('button', { name: /Run Backtest/i }));

    await screen.findByTestId('backtest-executed-trades');
    expect(screen.getByText(/Danh sách Lệnh Đã Thực Hiện \(2 lệnh\)/i)).toBeInTheDocument();

    const rowFPT = screen.getByTestId('trade-row-1');
    expect(within(rowFPT).getByText('FPT')).toBeInTheDocument();
    expect(within(rowFPT).getByText('4,500,000.00')).toBeInTheDocument();

    const rowMWG = screen.getByTestId('trade-row-2');
    expect(within(rowMWG).getByText('MWG')).toBeInTheDocument();
    expect(within(rowMWG).getByText('-3,000,000.00')).toBeInTheDocument();
  });
});
