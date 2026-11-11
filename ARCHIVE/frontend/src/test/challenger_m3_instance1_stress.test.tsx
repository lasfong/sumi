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
// SQA-010: StrategyRuleBuilder Boundary & Adversarial Inputs
// ===========================================================================
describe('Challenger M3 Instance 1 — SQA-010: StrategyRuleBuilder Boundary Inputs', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(signalsApi.getSignalRegistry).mockResolvedValue({
      signals: [
        {
          name: 'health.favorable',
          version: '1.0.0',
          category: 'health',
          label_vi: 'Sức Khỏe Kỹ Thuật Thuận Lợi',
          description: 'Health score >= +35.0',
          output_type: 'bool',
          parameters_schema: {},
          default_parameters: {},
          dependencies: [],
          warmup_bars: 50,
          causal_delay_bars: 0,
          status: 'ACTIVE',
          ast_alias: 'health__favorable',
        },
        {
          name: 'volume.rvol',
          version: '1.0.0',
          category: 'volume',
          label_vi: 'Khối Lượng RVOL',
          description: 'Relative volume',
          output_type: 'float',
          parameters_schema: {},
          default_parameters: {},
          dependencies: [],
          warmup_bars: 20,
          causal_delay_bars: 0,
          status: 'ACTIVE',
          ast_alias: 'volume__rvol',
        },
      ],
    });
  });

  it('rejects empty, whitespace, non-numeric strings, SQLi, and oversized numbers (1e309)', async () => {
    renderWithProviders(<StrategyRuleBuilder />);

    await waitFor(() => {
      expect(screen.getByTestId('strategy-rule-builder')).toBeInTheDocument();
    });

    // Add a numeric clause
    fireEvent.click(screen.getByTestId('add-clause-btn'));
    const sigSelect = screen.getByTestId('clause-signal-select-2');
    fireEvent.change(sigSelect, { target: { value: 'volume__rvol' } });

    const valInput = screen.getByTestId('clause-compare-value-2');
    const executeBtn = screen.getByTestId('execute-rule-btn');
    const copyYamlBtn = screen.getByTestId('copy-rule-yaml-btn');

    // 1. Empty string
    fireEvent.change(valInput, { target: { value: '' } });
    expect(screen.getByTestId('clause-error-2')).toHaveTextContent('Vui lòng nhập giá trị so sánh số');
    expect(executeBtn).toBeDisabled();
    expect(copyYamlBtn).toBeDisabled();

    // 2. Whitespace only
    fireEvent.change(valInput, { target: { value: '    ' } });
    expect(screen.getByTestId('clause-error-2')).toHaveTextContent('Vui lòng nhập giá trị so sánh số');
    expect(executeBtn).toBeDisabled();

    // 3. SQL injection string
    fireEvent.change(valInput, { target: { value: "'; DROP TABLE candles; --" } });
    expect(screen.getByTestId('clause-error-2')).toHaveTextContent('không phải là số hợp lệ');
    expect(executeBtn).toBeDisabled();

    // 4. HTML / Script injection
    fireEvent.change(valInput, { target: { value: '<script>alert(1)</script>' } });
    expect(screen.getByTestId('clause-error-2')).toHaveTextContent('không phải là số hợp lệ');
    expect(executeBtn).toBeDisabled();

    // 5. Oversized number (1e309 evaluates to Infinity in JS)
    fireEvent.change(valInput, { target: { value: '1e309' } });
    expect(screen.getByTestId('clause-error-2')).toHaveTextContent('không phải là số hợp lệ');
    expect(executeBtn).toBeDisabled();

    // 6. Valid scientific notation (1e-5)
    fireEvent.change(valInput, { target: { value: '1e-5' } });
    expect(screen.queryByTestId('clause-error-2')).not.toBeInTheDocument();
    expect(executeBtn).not.toBeDisabled();
    expect(copyYamlBtn).not.toBeDisabled();

    // 7. Valid negative number (-42.5)
    fireEvent.change(valInput, { target: { value: '-42.5' } });
    expect(screen.queryByTestId('clause-error-2')).not.toBeInTheDocument();
    expect(executeBtn).not.toBeDisabled();
  });
});

// ===========================================================================
// SQA-016: TechnicalFlowBBViewer Quality Threshold Boundaries
// ===========================================================================
describe('Challenger M3 Instance 1 — SQA-016: TechnicalFlowBBViewer Threshold Boundaries', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(bbApi.getBBHorizons).mockResolvedValue({ horizons: ['T03', 'T05', 'T20', 'T50', 'T200'] } as any);
  });

  const basePoint = {
    date: '2024-03-01',
    value: 1250000000,
    basis: 1100000000,
    daily_pressure: 0.45,
    daily_trading_value: 15000000000,
    regime: 'EXPANSION_UP',
    horizons: {
      T20: {
        raw_numerator: 1250000000,
        raw_denominator: 1100000000,
        flow_value: 1250000000,
        quality: 'HIGH',
      },
    },
  };

  it('renders UNAVAILABLE when coverage ratio is 0.0 or points empty', async () => {
    vi.mocked(bbApi.getSymbolBB).mockResolvedValueOnce({
      symbol: 'FPT0',
      as_of: '2024-03-01',
      coverage_ratio: 0.0,
      points: [basePoint],
      gate_reasons: [],
    } as any);

    renderWithProviders(<TechnicalFlowBBViewer defaultSymbol="FPT0" />);
    await waitFor(() => {
      expect(screen.getByTestId('bb-data-quality-badge')).toHaveTextContent('CHẤT LƯỢNG DỮ LIỆU: UNAVAILABLE');
      expect(screen.getByTestId('bb-degraded-notice')).toBeInTheDocument();
    });
  });

  it('renders DEGRADED with low coverage warning when coverage is 10% (0.10)', async () => {
    vi.mocked(bbApi.getSymbolBB).mockResolvedValueOnce({
      symbol: 'FPT10',
      as_of: '2024-03-01',
      coverage_ratio: 0.10,
      points: [basePoint],
      gate_reasons: [],
    } as any);

    renderWithProviders(<TechnicalFlowBBViewer defaultSymbol="FPT10" />);
    await waitFor(() => {
      expect(screen.getByTestId('bb-data-quality-badge')).toHaveTextContent('CHẤT LƯỢNG DỮ LIỆU: DEGRADED');
      expect(screen.getByTestId('bb-degraded-notice')).toHaveTextContent('Độ phủ dữ liệu thấp (10.0% < 80%)');
    });
  });

  it('renders DEGRADED at boundary just below 80% (79% coverage)', async () => {
    vi.mocked(bbApi.getSymbolBB).mockResolvedValueOnce({
      symbol: 'FPT79',
      as_of: '2024-03-01',
      coverage_ratio: 0.79,
      points: [basePoint],
      gate_reasons: [],
    } as any);

    renderWithProviders(<TechnicalFlowBBViewer defaultSymbol="FPT79" />);
    await waitFor(() => {
      expect(screen.getByTestId('bb-data-quality-badge')).toHaveTextContent('CHẤT LƯỢNG DỮ LIỆU: DEGRADED');
      expect(screen.getByTestId('bb-degraded-notice')).toHaveTextContent('Độ phủ dữ liệu thấp (79.0% < 80%)');
    });
  });

  it('renders HIGH and NO degraded notice at exactly 80% boundary (0.80)', async () => {
    vi.mocked(bbApi.getSymbolBB).mockResolvedValueOnce({
      symbol: 'FPT80',
      as_of: '2024-03-01',
      coverage_ratio: 0.80,
      points: [basePoint],
      gate_reasons: [],
    } as any);

    renderWithProviders(<TechnicalFlowBBViewer defaultSymbol="FPT80" />);
    await waitFor(() => {
      expect(screen.getByTestId('bb-data-quality-badge')).toHaveTextContent('CHẤT LƯỢNG DỮ LIỆU: HIGH');
      expect(screen.queryByTestId('bb-degraded-notice')).not.toBeInTheDocument();
    });
  });

  it('renders HIGH and NO degraded notice at 100% coverage (1.0)', async () => {
    vi.mocked(bbApi.getSymbolBB).mockResolvedValueOnce({
      symbol: 'FPT100',
      as_of: '2024-03-01',
      coverage_ratio: 1.0,
      points: [basePoint],
      gate_reasons: [],
    } as any);

    renderWithProviders(<TechnicalFlowBBViewer defaultSymbol="FPT100" />);
    await waitFor(() => {
      expect(screen.getByTestId('bb-data-quality-badge')).toHaveTextContent('CHẤT LƯỢNG DỮ LIỆU: HIGH');
      expect(screen.queryByTestId('bb-degraded-notice')).not.toBeInTheDocument();
    });
  });
});

// ===========================================================================
// SQA-015 & SQA-005: Scanner Rapid History Switching & Regex Search
// ===========================================================================
describe('Challenger M3 Instance 1 — SQA-015 & SQA-005: Scanner History & Regex Chars', () => {
  const run1 = {
    id: 1,
    label: 'Scan 1 - Multi Results',
    status: 'succeeded',
    total_results: 3,
    created_at: '2024-03-01T10:00:00Z',
    result_payload: {
      status: 'succeeded',
      total_results: 3,
      results: [
        { symbol: 'FPT', timestamp: '2024-03-01T00:00:00Z', signal_type: 'entry', price: 105000, regime: 'bullish' },
        { symbol: 'VNM', timestamp: '2024-03-01T00:00:00Z', signal_type: 'exit', price: 68000, regime: 'bearish' },
        { symbol: 'HPG', timestamp: '2024-03-01T00:00:00Z', signal_type: 'entry', price: 29500, regime: 'bullish' },
      ],
    },
  };

  const run2 = {
    id: 2,
    label: 'Scan 2 - Empty Results',
    status: 'succeeded',
    total_results: 0,
    created_at: '2024-03-01T11:00:00Z',
    result_payload: {
      status: 'succeeded',
      total_results: 0,
      results: [],
    },
  };

  const run3 = {
    id: 3,
    label: 'Scan 3 - Single Result',
    status: 'succeeded',
    total_results: 1,
    created_at: '2024-03-01T12:00:00Z',
    result_payload: {
      status: 'succeeded',
      total_results: 1,
      results: [
        { symbol: 'MWG', timestamp: '2024-03-01T00:00:00Z', signal_type: 'entry', price: 52000, regime: 'neutral' },
      ],
    },
  };

  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(scannerApi.listScannerRuns).mockResolvedValue([run1, run2, run3] as any);
    vi.mocked(backtestApi.getAvailableStrategies).mockResolvedValue([
      { filename: 'ema_cross.yaml', name: 'EMA Cross', description: 'Test', config: {} as any },
    ]);
  });

  it('rapidly toggles between history runs with zero cross-contamination', async () => {
    renderWithProviders(<ScannerPage />);

    await waitFor(() => {
      expect(screen.getByTestId('history-run-1')).toBeInTheDocument();
      expect(screen.getByTestId('history-run-2')).toBeInTheDocument();
      expect(screen.getByTestId('history-run-3')).toBeInTheDocument();
    });

    // 1. Click Run 1 -> displays 3 items
    fireEvent.click(screen.getByTestId('history-run-1'));
    expect(screen.getByText('FPT')).toBeInTheDocument();
    expect(screen.getByText('VNM')).toBeInTheDocument();
    expect(screen.getByText('HPG')).toBeInTheDocument();
    expect(screen.getByTestId('scanner-results-count')).toHaveTextContent('3 / 3 tín hiệu');

    // 2. Click Run 2 -> displays empty state
    fireEvent.click(screen.getByTestId('history-run-2'));
    expect(screen.queryByText('FPT')).not.toBeInTheDocument();
    expect(screen.getByText('No signals found.')).toBeInTheDocument();

    // 3. Click Run 3 -> displays MWG
    fireEvent.click(screen.getByTestId('history-run-3'));
    expect(screen.getByText('MWG')).toBeInTheDocument();
    expect(screen.queryByText('FPT')).not.toBeInTheDocument();

    // 4. Click Run 1 again -> restores FPT, VNM, HPG
    fireEvent.click(screen.getByTestId('history-run-1'));
    expect(screen.getByText('FPT')).toBeInTheDocument();
    expect(screen.getByText('VNM')).toBeInTheDocument();
    expect(screen.getByText('HPG')).toBeInTheDocument();
  });

  it('handles regex special characters in search box without throwing SyntaxError', async () => {
    renderWithProviders(<ScannerPage />);

    await waitFor(() => {
      expect(screen.getByTestId('history-run-1')).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId('history-run-1'));

    const searchInput = screen.getByTestId('scanner-filter-input');

    // Type hostile regex characters that break `new RegExp()`
    const hostileInputs = ['.*', '(', '[A-Z]', '+', '?', '\\', '{2,}', '^$'];
    for (const char of hostileInputs) {
      fireEvent.change(searchInput, { target: { value: char } });
      // Should cleanly show no results or matching symbols without crashing!
      expect(screen.getByTestId('no-filtered-results')).toHaveTextContent('Không tìm thấy tín hiệu nào khớp với bộ lọc.');
    }

    // Type matching substring
    fireEvent.change(searchInput, { target: { value: 'FP' } });
    expect(screen.getByText('FPT')).toBeInTheDocument();
    expect(screen.queryByText('VNM')).not.toBeInTheDocument();
  });
});

// ===========================================================================
// SQA-011: BacktestPage Executed Trades Rendering Stress
// ===========================================================================
describe('Challenger M3 Instance 1 — SQA-011: Executed Trades Rendering', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(backtestApi.getAvailableStrategies).mockResolvedValue([
      { filename: 'test_strat.yaml', name: 'Test Strat', description: '', config: {} as any },
    ]);
  });

  it('handles 0 trades, open positions with null exit dates/prices, and 100+ trades cleanly', async () => {
    // Generate 120 simulated trades
    const largeTradesList = Array.from({ length: 120 }, (_, i) => ({
      id: i + 1,
      symbol: i % 2 === 0 ? 'FPT' : 'VNM',
      entry_date: '2023-01-05T09:15:00',
      entry_price: 50000 + i * 10,
      exit_date: i === 0 ? null : '2023-01-20T14:30:00', // Trade 1 is OPEN!
      exit_price: i === 0 ? null : 52000 + i * 10,
      quantity: 1000,
      gross_pnl: i === 0 ? 0 : 2000000,
      net_pnl: i === 0 ? 0 : 1980000,
      pnl_percent: i === 0 ? 0 : 0.04,
      holding_candles: 11,
      status: i === 0 ? 'open' : 'closed',
      result: i === 0 ? undefined : 'win',
    }));

    vi.mocked(backtestApi.runBacktest).mockResolvedValueOnce({
      status: 'succeeded',
      session_id: 101,
      strategy: 'Test Strat',
      symbol: 'FPT',
      total_candles: 500,
      trades: largeTradesList,
      analytics: {
        total_net_pnl: 200000000,
        win_rate: 0.99,
        sharpe_ratio: 1.8,
        max_drawdown: 0.05,
        total_trades: 120,
        average_win: 2000000,
        average_loss: 500000,
        profit_factor: 4.0,
        sqn: 3.2,
        metrics: {},
        equity_curve: [],
      } as any,
    });

    renderWithProviders(<BacktestPage />);

    // Wait for strategy options to load and combobox to have the strategy selected
    await waitFor(() => {
      expect(screen.getByRole('combobox')).toHaveValue('test_strat.yaml');
    });

    // Click submit
    const submitBtn = screen.getByRole('button', { name: /Run Backtest/i });
    fireEvent.click(submitBtn);

    await waitFor(() => {
      expect(screen.getByTestId('backtest-executed-trades')).toBeInTheDocument();
    });

    // Check header counter
    expect(screen.getByText(/Danh sách Lệnh Đã Thực Hiện \(120 lệnh\)/)).toBeInTheDocument();

    // Check open trade (row 1)
    const row1 = screen.getByTestId('trade-row-1');
    expect(within(row1).getByText('Đang mở')).toBeInTheDocument();
    expect(within(row1).getByText('—')).toBeInTheDocument();

    // Check closed trade (row 120)
    const row120 = screen.getByTestId('trade-row-120');
    expect(within(row120).getByText('WIN')).toBeInTheDocument();
  });

  it('renders clean fallback empty state when trades list is empty', async () => {
    vi.mocked(backtestApi.runBacktest).mockResolvedValueOnce({
      status: 'succeeded',
      session_id: 102,
      strategy: 'Test Strat',
      symbol: 'FPT',
      total_candles: 100,
      trades: [],
      analytics: {
        total_net_pnl: 0,
        win_rate: 0.0,
        sharpe_ratio: 0.0,
        max_drawdown: 0.0,
        total_trades: 0,
        average_win: 0,
        average_loss: 0,
        profit_factor: 0,
        sqn: 0,
        metrics: {},
        equity_curve: [],
      } as any,
    });

    renderWithProviders(<BacktestPage />);
    await waitFor(() => {
      expect(screen.getByRole('combobox')).toHaveValue('test_strat.yaml');
    });

    const submitBtn = screen.getByRole('button', { name: /Run Backtest/i });
    fireEvent.click(submitBtn);

    await waitFor(() => {
      expect(screen.getByTestId('no-executed-trades')).toHaveTextContent(
        'Không có giao dịch nào được thực hiện trong kỳ kiểm thử này.'
      );
    });
  });
});
