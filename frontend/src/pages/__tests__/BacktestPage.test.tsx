import React from 'react';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { BacktestPage } from '../BacktestPage';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import * as backtestApi from '../../api/backtestApi';
import '@testing-library/jest-dom';

// Mock the API calls
vi.mock('../../api/backtestApi', () => ({
  getAvailableStrategies: vi.fn(),
  runBacktest: vi.fn()
}));

// Mock EquityChart because lightweight-charts relies on Canvas/DOM elements not fully supported in jsdom
vi.mock('../../components/analytics/EquityChart', () => ({
  EquityChart: () => <div data-testid="mock-equity-chart">Equity Chart</div>
}));

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { retry: false }
  }
});

const renderWithClient = (ui: React.ReactElement) => {
  return render(
    <QueryClientProvider client={queryClient}>
      {ui}
    </QueryClientProvider>
  );
};

describe('BacktestPage UI Test', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    
    // Setup mock data for available strategies
    vi.mocked(backtestApi.getAvailableStrategies).mockResolvedValue([
      { filename: 'strategy1.yaml', name: 'Strategy 1', description: 'Test Strategy', config: {} }
    ]);
  });

  it('renders the form correctly', async () => {
    renderWithClient(<BacktestPage />);
    
    // Wait for strategies to load and populate the select
    await waitFor(() => {
      expect(screen.getByRole('combobox')).toHaveValue('strategy1.yaml');
    });

    expect(screen.getByLabelText(/Symbol/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Start Date/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/End Date/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Initial Cash/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Run Backtest/i })).toBeInTheDocument();
  });

  it('calls runBacktest on submit and displays analytics', async () => {
    // Setup mock response for runBacktest
    vi.mocked(backtestApi.runBacktest).mockResolvedValue({
      session_id: 123,
      strategy: 'Strategy 1',
      symbol: 'FPT',
      total_candles: 100,
      analytics: {
        total_net_pnl: 50000,
        win_rate: 0.75,
        sharpe_ratio: 1.5,
        max_drawdown: -2000,
        total_trades: 4,
        average_win: 15000,
        average_loss: 5000,
        profit_factor: 3,
        sqn: 2.1,
        metrics: {
          win_rate: { value: 0.75, status: 'valid', sample_size: 4, period: 'closed_trades' },
          profit_factor: { value: 3, status: 'valid', sample_size: 4, period: 'closed_trades' },
          sharpe_ratio: { value: 1.5, status: 'valid', sample_size: 30, period: 'daily_equity_returns' },
          sortino_ratio: { value: 1.2, status: 'valid', sample_size: 30, period: 'daily_equity_returns' },
          sqn: { value: 2.1, status: 'valid', sample_size: 30, period: 'closed_trades' },
        },
        equity_curve: []
      }
    });

    renderWithClient(<BacktestPage />);
    
    // Wait for options to load
    await waitFor(() => expect(screen.getByRole('combobox')).toBeInTheDocument());

    // Select the strategy
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'strategy1.yaml' } });
    
    // Click run
    fireEvent.click(screen.getByRole('button', { name: /Run Backtest/i }));

    // Wait for the analytics to render
    await waitFor(() => {
      expect(screen.getByText(/Results \(Session #123\)/i)).toBeInTheDocument();
    });

    // Check if key metrics are displayed
    expect(screen.getByText('50,000.00')).toBeInTheDocument(); // Net PnL
    expect(screen.getByText('75.00%')).toBeInTheDocument(); // Win rate
    expect(screen.getByText('1.50')).toBeInTheDocument(); // Sharpe
  });

  it('renders executed trades table with percentage-point pnl_percent formatting (SQA-011)', async () => {
    vi.mocked(backtestApi.runBacktest).mockResolvedValue({
      session_id: 124,
      strategy: 'Strategy 1',
      symbol: 'FPT',
      total_candles: 100,
      trades: [
        {
          id: 1,
          symbol: 'FPT',
          entry_date: '2024-01-10T00:00:00+00:00',
          entry_price: 25000,
          exit_date: '2024-01-20T00:00:00+00:00',
          exit_price: 28000,
          quantity: 1000,
          gross_pnl: 3000000,
          net_pnl: 2850000,
          pnl_percent: 8.421735,
          holding_candles: 10,
          status: 'closed',
          result: 'win',
        },
        {
          id: 2,
          symbol: 'FPT',
          entry_date: '2024-02-01T00:00:00+00:00',
          entry_price: 29000,
          exit_date: '2024-02-05T00:00:00+00:00',
          exit_price: 27500,
          quantity: 1000,
          gross_pnl: -1500000,
          net_pnl: -1650000,
          pnl_percent: -4.514517,
          holding_candles: 4,
          status: 'closed',
          result: 'loss',
        },
        {
          id: 3,
          symbol: 'FPT',
          entry_date: '2024-03-01T00:00:00+00:00',
          entry_price: 28000,
          exit_date: '2024-03-05T00:00:00+00:00',
          exit_price: 28000,
          quantity: 1000,
          gross_pnl: 0,
          net_pnl: 0,
          pnl_percent: 0,
          holding_candles: 3,
          status: 'closed',
          result: 'breakeven',
        },
      ],
      analytics: {
        total_net_pnl: 1200000,
        win_rate: 0.5,
        sharpe_ratio: 1.1,
        max_drawdown: -1650000,
        total_trades: 3,
        average_win: 2850000,
        average_loss: 1650000,
        profit_factor: 1.73,
        sqn: 1.0,
        metrics: {},
        equity_curve: [],
      },
    });

    renderWithClient(<BacktestPage />);
    await waitFor(() => expect(screen.getByRole('combobox')).toBeInTheDocument());

    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'strategy1.yaml' } });
    fireEvent.click(screen.getByRole('button', { name: /Run Backtest/i }));

    await waitFor(() => {
      expect(screen.getByTestId('backtest-executed-trades')).toBeInTheDocument();
    });

    expect(screen.getByText(/Danh sách Lệnh Đã Thực Hiện \(3 lệnh\)/i)).toBeInTheDocument();
    const trade1 = screen.getByTestId('trade-row-1');
    const trade2 = screen.getByTestId('trade-row-2');
    const trade3 = screen.getByTestId('trade-row-3');

    expect(within(trade1).getByText('25,000.00')).toBeInTheDocument();
    expect(within(trade1).getByText('28,000.00')).toBeInTheDocument();
    expect(within(trade1).getByText('2,850,000.00')).toBeInTheDocument();
    expect(within(trade1).getByText('8.42%')).toBeInTheDocument();
    expect(within(trade1).queryByText('842.17%')).not.toBeInTheDocument();
    expect(within(trade1).getByText('WIN')).toBeInTheDocument();

    expect(within(trade2).getByText('29,000.00')).toBeInTheDocument();
    expect(within(trade2).getByText('27,500.00')).toBeInTheDocument();
    expect(within(trade2).getByText('-1,650,000.00')).toBeInTheDocument();
    expect(within(trade2).getByText('-4.51%')).toBeInTheDocument();
    expect(within(trade2).queryByText('-451.45%')).not.toBeInTheDocument();
    expect(within(trade2).getByText('LOSS')).toBeInTheDocument();

    expect(within(trade3).getByText('0.00%')).toBeInTheDocument();
    expect(within(trade3).getByText('BREAKEVEN')).toBeInTheDocument();
  });

  it('supports switching between backtest runs and multi-symbol runs with accurate % PnL (SQA-011)', async () => {
    vi.mocked(backtestApi.runBacktest)
      .mockResolvedValueOnce({
        session_id: 201,
        strategy: 'Strategy 1',
        symbol: 'PORTFOLIO',
        total_candles: 200,
        analytics: null,
        runs: [
          {
            symbol: 'FPT',
            status: 'succeeded',
            total_candles: 100,
            analytics: null,
            trades: [
              {
                id: 10,
                symbol: 'FPT',
                entry_date: '2024-01-10T00:00:00Z',
                entry_price: 90000,
                exit_date: '2024-01-20T00:00:00Z',
                exit_price: 97580,
                quantity: 1000,
                net_pnl: 7580000,
                pnl_percent: 8.421735,
                holding_candles: 8,
                status: 'closed',
                result: 'win',
              },
            ],
          },
          {
            symbol: 'MWG',
            status: 'succeeded',
            total_candles: 100,
            analytics: null,
            trades: [
              {
                id: 11,
                symbol: 'MWG',
                entry_date: '2024-02-10T00:00:00Z',
                entry_price: 45000,
                exit_date: '2024-02-20T00:00:00Z',
                exit_price: 42968,
                quantity: 1000,
                net_pnl: -2031532,
                pnl_percent: -4.514517,
                holding_candles: 6,
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
          total_candles: 200,
          total_trades: 2,
          win_rate: 0.5,
          total_net_pnl: 5548468,
        },
      })
      .mockResolvedValueOnce({
        session_id: 202,
        strategy: 'Strategy 1',
        symbol: 'VNM',
        total_candles: 100,
        trades: [
          {
            id: 20,
            symbol: 'VNM',
            entry_date: '2024-04-01T00:00:00Z',
            entry_price: 65000,
            exit_date: '2024-04-15T00:00:00Z',
            exit_price: 68250,
            quantity: 500,
            net_pnl: 1625000,
            pnl_percent: 5.0,
            holding_candles: 10,
            status: 'closed',
            result: 'win',
          },
        ],
        analytics: {
          total_net_pnl: 1625000,
          win_rate: 1.0,
          sharpe_ratio: 1.4,
          max_drawdown: 0,
          total_trades: 1,
          average_win: 1625000,
          average_loss: 0,
          profit_factor: 99,
          sqn: 2.0,
          metrics: {},
          equity_curve: [],
        },
      });

    renderWithClient(<BacktestPage />);
    await waitFor(() => expect(screen.getByRole('combobox')).toBeInTheDocument());

    fireEvent.change(screen.getByLabelText(/Symbol/i), { target: { value: 'FPT, MWG' } });
    fireEvent.click(screen.getByRole('button', { name: /Run Backtest/i }));

    await waitFor(() => {
      expect(screen.getByTestId('trade-row-10')).toBeInTheDocument();
    });
    expect(within(screen.getByTestId('trade-row-10')).getByText('8.42%')).toBeInTheDocument();
    expect(within(screen.getByTestId('trade-row-11')).getByText('-4.51%')).toBeInTheDocument();

    // Switch symbol and run second backtest
    fireEvent.change(screen.getByLabelText(/Symbol/i), { target: { value: 'VNM' } });
    fireEvent.click(screen.getByRole('button', { name: /Run Backtest/i }));

    await waitFor(() => {
      expect(screen.getByTestId('trade-row-20')).toBeInTheDocument();
    });
    expect(screen.queryByTestId('trade-row-10')).not.toBeInTheDocument();
    expect(within(screen.getByTestId('trade-row-20')).getByText('5.00%')).toBeInTheDocument();
  });

  it('renders empty message when no trades were executed (SQA-011)', async () => {
    vi.mocked(backtestApi.runBacktest).mockResolvedValue({
      session_id: 125,
      strategy: 'Strategy 1',
      symbol: 'FPT',
      total_candles: 100,
      trades: [],
      analytics: {
        total_net_pnl: 0,
        win_rate: 0,
        sharpe_ratio: 0,
        max_drawdown: 0,
        total_trades: 0,
        average_win: 0,
        average_loss: 0,
        profit_factor: 0,
        sqn: 0,
        metrics: {},
        equity_curve: [],
      },
    });

    renderWithClient(<BacktestPage />);
    await waitFor(() => expect(screen.getByRole('combobox')).toBeInTheDocument());

    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'strategy1.yaml' } });
    fireEvent.click(screen.getByRole('button', { name: /Run Backtest/i }));

    await waitFor(() => {
      expect(screen.getByTestId('no-executed-trades')).toBeInTheDocument();
    });

    expect(screen.getByText(/Không có giao dịch nào được thực hiện/i)).toBeInTheDocument();
  });
});
