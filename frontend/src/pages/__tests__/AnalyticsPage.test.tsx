import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { AnalyticsPage } from '../AnalyticsPage';
import * as analyticsApi from '../../api/analyticsApi';
import * as decisionApi from '../../api/decisionApi';
import '@testing-library/jest-dom';

vi.mock('../../api/analyticsApi', () => ({
  getSessionAnalytics: vi.fn(),
}));

vi.mock('../../api/decisionApi', () => ({
  getTrades: vi.fn(),
}));

const renderWithClient = (ui: React.ReactElement) => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      {ui}
    </QueryClientProvider>
  );
};

describe('AnalyticsPage', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(decisionApi.getTrades).mockResolvedValue([]);
    vi.mocked(analyticsApi.getSessionAnalytics).mockResolvedValue({
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
    expect(screen.getByText('VNINDEX')).toBeInTheDocument();
    expect(screen.getByText('FPT')).toBeInTheDocument();
    expect(screen.getByText('1.50')).toBeInTheDocument();
  });
});
