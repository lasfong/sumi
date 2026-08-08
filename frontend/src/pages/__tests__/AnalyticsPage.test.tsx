import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { AnalyticsPage } from '../AnalyticsPage';
import * as analyticsApi from '../../api/analyticsApi';
import * as decisionApi from '../../api/decisionApi';
import * as replayApi from '../../api/replayApi';
import type { ReplaySession } from '../../types';
import '@testing-library/jest-dom';

vi.mock('../../api/analyticsApi', () => ({
  getSessionAnalytics: vi.fn(),
}));

vi.mock('../../api/decisionApi', () => ({
  getTrades: vi.fn(),
}));

vi.mock('../../api/replayApi', () => ({
  getReplaySession: vi.fn(),
  listReplaySessions: vi.fn().mockResolvedValue([]),
}));

import { MemoryRouter } from 'react-router-dom';

const renderWithClient = (ui: React.ReactElement) => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={['/analytics?session=1']}>
        {ui}
      </MemoryRouter>
    </QueryClientProvider>
  );
};

describe('AnalyticsPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(replayApi.getReplaySession).mockResolvedValue({
      id: 1,
      symbol: 'FPT',
      timeframe: '1D',
      start_date: '2024-01-01',
      end_date: '2024-06-01',
      current_index: 0,
      status: 'active',
      mode: 'normal',
    } as ReplaySession);
    vi.mocked(decisionApi.getTrades).mockResolvedValue([]);
    vi.mocked(analyticsApi.getSessionAnalytics).mockResolvedValue({
      benchmark_symbol: 'VNINDEX',
      total_trades: 1,
      win_rate: 1,
      total_net_pnl: 500,
      average_win: 500,
      average_loss: 0,
      profit_factor: null,
      max_drawdown: 100,
      max_drawdown_pct: 1,
      sharpe_ratio: 0,
      sortino_ratio: 0,
      sqn: 0,
      equity_curve: [],
      symbol_performance: [],
      mistake_performance: [],
      setup_performance: [],
      outlier_impact: {
        top_winners_pnl: 500,
        top_losers_pnl: 0,
        top_winners_share: 1,
        top_losers_share: 0,
        median_trade_pnl: 500,
        trimmed_expectancy: 500,
      },
      drawdown_periods: [
        { start: '2024-01-02', end: '2024-01-05', max_drawdown_pct: 1 },
      ],
      benchmark_curve: [
        { time: '2024-01-01', value: 100000000 },
        { time: '2024-01-05', value: 105000000 },
      ],
      metrics: {
        benchmark: { value: 0.05, status: 'valid', sample_size: 2, period: 'benchmark_candles' },
      },
      trade_distribution: [
        { trade_id: 1, symbol: 'FPT', net_pnl: 500, pnl_percent: 0.05, r_multiple: 1.5, result: 'win' },
      ],
    });
  });

  it('renders drawdown, benchmark, and trade distribution outputs', async () => {
    renderWithClient(<AnalyticsPage />);

    await waitFor(() => {
      expect(screen.getByText('Drawdown Periods')).toBeInTheDocument();
    });

    expect(screen.getByText('Trade Distribution')).toBeInTheDocument();
    expect(screen.getByText('VNINDEX benchmark')).toBeInTheDocument();
    expect(screen.getByTestId('analytics-benchmark')).toHaveAttribute('data-metric-status', 'valid');
    expect(screen.getAllByText('FPT').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('1,50')).toBeInTheDocument();
  });

  it('renders the configured alternate benchmark identity and authoritative metric', async () => {
    vi.mocked(analyticsApi.getSessionAnalytics).mockResolvedValueOnce({
      benchmark_symbol: 'HNXINDEX',
      total_trades: 0, win_rate: 0, total_net_pnl: 0, average_win: 0, average_loss: 0,
      profit_factor: null, equity_curve: [], benchmark_curve: [],
      metrics: {
        benchmark: { value: 0.2, status: 'valid', sample_size: 40, period: 'benchmark_candles' },
      },
    });

    renderWithClient(<AnalyticsPage />);

    expect(await screen.findByText('HNXINDEX benchmark')).toBeInTheDocument();
    expect(screen.getByTestId('analytics-benchmark')).toHaveTextContent('20,00%');
    expect(screen.queryByText('VNINDEX benchmark')).not.toBeInTheDocument();
  });

  it('renders a missing benchmark neutrally with the authoritative reason', async () => {
    vi.mocked(analyticsApi.getSessionAnalytics).mockResolvedValueOnce({
      benchmark_symbol: 'MISSING_BENCH',
      total_trades: 0, win_rate: 0, total_net_pnl: 0, average_win: 0, average_loss: 0,
      profit_factor: null, equity_curve: [], benchmark_curve: [],
      metrics: {
        benchmark: {
          value: null, status: 'not_applicable', sample_size: 0, period: 'benchmark_candles',
          reason: 'Benchmark MISSING_BENCH has insufficient coverage for this session.',
        },
      },
    });

    renderWithClient(<AnalyticsPage />);

    expect(await screen.findByText('MISSING_BENCH benchmark')).toBeInTheDocument();
    const benchmark = screen.getByTestId('analytics-benchmark');
    expect(benchmark).toHaveAttribute('data-metric-status', 'not_applicable');
    expect(benchmark).toHaveTextContent('Unavailable');
    expect(benchmark).toHaveTextContent('insufficient coverage');
    expect(benchmark.querySelector('div')).toHaveStyle({ color: 'var(--text-muted)' });
  });
});
