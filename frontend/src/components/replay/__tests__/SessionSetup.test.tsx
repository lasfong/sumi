import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { SessionSetup } from '../SessionSetup';
import * as replayApi from '../../../api/replayApi';
import * as symbolsApi from '../../../api/symbolsApi';
import '@testing-library/jest-dom';

vi.mock('../../../api/replayApi', () => ({
  listReplaySessions: vi.fn(),
}));

vi.mock('../../../api/symbolsApi', () => ({
  getSymbols: vi.fn(),
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

describe('SessionSetup', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(symbolsApi.getSymbols).mockResolvedValue([]);
    vi.mocked(replayApi.listReplaySessions).mockResolvedValue([
      {
        id: 7,
        symbol: 'FPT',
        timeframe: '1D',
        adjustment_type: 'unadjusted',
        start_date: '2024-01-01',
        end_date: '2024-03-01',
        current_index: 3,
        initial_cash: 100000000,
        current_cash: 100000000,
        status: 'active',
        mode: 'normal',
        hide_symbol: 0,
        hide_date: 0,
        created_at: '2024-01-01T00:00:00',
        updated_at: '2024-01-02T00:00:00',
      },
    ]);
  });

  it('resumes a recent replay session', async () => {
    const onResumeSession = vi.fn();

    renderWithClient(
      <SessionSetup onCreateSession={vi.fn()} onResumeSession={onResumeSession} />
    );

    await waitFor(() => {
      expect(screen.getByText('FPT')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /Resume Session #7/i }));

    expect(onResumeSession).toHaveBeenCalledWith(7);
  });
});
