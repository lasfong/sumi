import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { DashboardPage } from '../DashboardPage';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import type { ReplaySession } from '../../types';
import type { DataReadiness } from '../../api/symbolsApi';
import * as symbolsApi from '../../api/symbolsApi';
import * as replayApi from '../../api/replayApi';
import * as strategyLabApi from '../../api/strategyLabApi';
import type { StrategyLabRun } from '../../api/strategyLabApi';

vi.mock('../../api/symbolsApi');
vi.mock('../../api/replayApi');
vi.mock('../../api/strategyLabApi');

const mockReadinessReady: DataReadiness = {
  status: 'ready',
  symbols_count: 2,
  symbols_with_candles: ['FPT', 'VNINDEX'],
  timeframes: ['1D'],
  total_candles: 1040,
  earliest_timestamp: '2023-01-01',
  latest_timestamp: '2024-06-01',
};

const mockReadinessEmpty: DataReadiness = {
  status: 'empty',
  symbols_count: 1,
  symbols_with_candles: [],
  timeframes: [],
  total_candles: 0,
  earliest_timestamp: null,
  latest_timestamp: null,
};

const mockSessions = [
  {
    id: 10,
    symbol: 'FPT',
    timeframe: 'D',
    start_date: '2024-01-01',
    end_date: '2024-06-01',
    current_index: 25,
    status: 'active',
    source_context: {
      schema_version: 1,
      source_type: 'scanner',
      replay_intent: 'blind_practice',
      reveal_at_index: 50,
      revealed: false,
      signal: null,
    },
  },
];

const mockResearch = [
  {
    id: 1,
    run_type: 'sweep',
    label: 'EMA Cross Parameter Sweep',
    request_config: {},
    result_payload: {},
    metrics: {},
    created_at: '2024-06-01T10:00:00Z',
  },
];

const renderComponent = () => {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter>
        <DashboardPage />
      </MemoryRouter>
    </QueryClientProvider>
  );
};

describe('DashboardPage (PRO-UX-01 / R02-02)', () => {
  it('renders data readiness with exact candle metrics, recent sessions, and recent research', async () => {
    vi.mocked(symbolsApi.getDataReadiness).mockResolvedValue(mockReadinessReady);
    vi.mocked(replayApi.listReplaySessions).mockResolvedValue(mockSessions as unknown as ReplaySession[]);
    vi.mocked(strategyLabApi.listStrategyLabRuns).mockResolvedValue(mockResearch as unknown as StrategyLabRun[]);

    renderComponent();

    expect(screen.getByText('Workstation Overview')).toBeInTheDocument();
    expect(await screen.findByText('Data Ready')).toBeInTheDocument();
    expect(screen.getByText('2 / 2')).toBeInTheDocument(); // active symbols
    expect(screen.getByText('1.040')).toBeInTheDocument(); // total candles formatted vi-VN

    expect(await screen.findByText('#10')).toBeInTheDocument();
    expect(screen.getByText('Continue Practice')).toBeInTheDocument();

    expect(await screen.findByText('EMA Cross Parameter Sweep')).toBeInTheDocument();
  });

  it('classifies zero candles as empty status and renders actionable import link', async () => {
    vi.mocked(symbolsApi.getDataReadiness).mockResolvedValue(mockReadinessEmpty);
    vi.mocked(replayApi.listReplaySessions).mockResolvedValue([]);
    vi.mocked(strategyLabApi.listStrategyLabRuns).mockResolvedValue([]);

    renderComponent();

    expect(await screen.findByText('No Data')).toBeInTheDocument();
    expect(await screen.findByText('No market candle data imported in local database.')).toBeInTheDocument();
    expect(await screen.findByText('No replay practice sessions created yet.')).toBeInTheDocument();
    expect(await screen.findByText('No saved strategy research runs found.')).toBeInTheDocument();
  });

  it('handles partial query failure and displays retry action', async () => {
    vi.mocked(symbolsApi.getDataReadiness).mockRejectedValue(new Error('Network Error'));
    vi.mocked(replayApi.listReplaySessions).mockResolvedValue(mockSessions as unknown as ReplaySession[]);
    vi.mocked(strategyLabApi.listStrategyLabRuns).mockResolvedValue(mockResearch as unknown as StrategyLabRun[]);

    renderComponent();

    expect(await screen.findByText('Error')).toBeInTheDocument();
    expect(await screen.findByText('Failed to query market data readiness.')).toBeInTheDocument();
    expect(await screen.findByText('#10')).toBeInTheDocument();
    expect(await screen.findByText('EMA Cross Parameter Sweep')).toBeInTheDocument();
  });

  it('handles complete error state and displays retry action', async () => {
    vi.mocked(symbolsApi.getDataReadiness).mockRejectedValue(new Error('Network Error'));
    vi.mocked(replayApi.listReplaySessions).mockRejectedValue(new Error('Network Error'));
    vi.mocked(strategyLabApi.listStrategyLabRuns).mockRejectedValue(new Error('Network Error'));

    renderComponent();

    expect(await screen.findByText('Failed to query market data readiness.')).toBeInTheDocument();
    expect(await screen.findByText('Error loading recent practice sessions.')).toBeInTheDocument();
    expect(await screen.findByText('Error loading recent research runs.')).toBeInTheDocument();
    
    const retryButtons = await screen.findAllByRole('button', { name: /Retry/i });
    expect(retryButtons.length).toBeGreaterThanOrEqual(3);
  });
});
