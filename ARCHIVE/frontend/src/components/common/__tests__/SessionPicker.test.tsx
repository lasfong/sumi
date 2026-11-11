import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { SessionPicker } from '../SessionPicker';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReplaySession } from '../../../types';
import * as replayApi from '../../../api/replayApi';

vi.mock('../../../api/replayApi', async () => {
  const actual = await vi.importActual<typeof import('../../../api/replayApi')>('../../../api/replayApi');
  return {
    ...actual,
    listReplaySessions: vi.fn(),
  };
});

const mockSessions = [
  {
    id: 1,
    symbol: 'FPT',
    timeframe: 'D',
    adjustment_type: 'split',
    start_date: '2024-01-01',
    end_date: '2024-06-01',
    current_index: 50,
    status: 'active',
    source_context: {
      schema_version: 1,
      source_type: 'scanner',
      replay_intent: 'blind_practice',
      reveal_at_index: 100,
      revealed: false,
      signal: null,
    },
  },
  {
    id: 2,
    symbol: 'VNM',
    timeframe: 'D',
    adjustment_type: 'split',
    start_date: '2024-02-01',
    end_date: '2024-07-01',
    current_index: 20,
    status: 'active',
    source_context: {
      schema_version: 1,
      source_type: 'scanner',
      replay_intent: 'signal_review',
      reveal_at_index: 20,
      revealed: true,
      signal: {
        timestamp: '2024-02-01',
        type: 'BUY',
        strategy: 'EMA Cross',
        price: 70.5,
        regime: 'Uptrend',
      },
    },
  },
];

const renderComponent = (selectedSessionId: number | null, onSelect: (id: number) => void) => {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={queryClient}>
      <SessionPicker selectedSessionId={selectedSessionId} onSelectSession={onSelect} />
    </QueryClientProvider>
  );
};

describe('SessionPicker (PRO-UX-02 / PRO-UX-08)', () => {
  it('renders trigger button and opens list on click', async () => {
    vi.mocked(replayApi.listReplaySessions).mockResolvedValue(mockSessions as unknown as ReplaySession[]);
    const onSelect = vi.fn();
    renderComponent(1, onSelect);

    const trigger = screen.getByRole('button', { name: /select session/i });
    expect(trigger).toBeInTheDocument();
    expect(await screen.findByText('#1')).toBeInTheDocument();
    expect(screen.getByText('FPT')).toBeInTheDocument();

    fireEvent.click(trigger);
    expect(await screen.findByPlaceholderText(/search session/i)).toBeInTheDocument();
    expect(screen.getByText('VNM')).toBeInTheDocument();
  });

  it('filters sessions by search query and calls onSelectSession', async () => {
    vi.mocked(replayApi.listReplaySessions).mockResolvedValue(mockSessions as unknown as ReplaySession[]);
    const onSelect = vi.fn();
    renderComponent(null, onSelect);

    const trigger = screen.getByRole('button', { name: /select session/i });
    fireEvent.click(trigger);

    const searchInput = await screen.findByPlaceholderText(/search session/i);
    fireEvent.change(searchInput, { target: { value: 'VNM' } });

    expect(screen.queryByText('FPT')).not.toBeInTheDocument();
    expect(screen.getByText('VNM')).toBeInTheDocument();

    fireEvent.click(screen.getByText('VNM'));
    expect(onSelect).toHaveBeenCalledWith(2);
  });

  it('isolates keydown events from parent shortcuts', async () => {
    vi.mocked(replayApi.listReplaySessions).mockResolvedValue(mockSessions as unknown as ReplaySession[]);
    const onSelect = vi.fn();
    renderComponent(1, onSelect);

    const trigger = screen.getByRole('button', { name: /select session/i });
    fireEvent.click(trigger);

    const searchInput = await screen.findByPlaceholderText(/search session/i);
    const event = new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true });
    const spy = vi.spyOn(event, 'stopPropagation');
    fireEvent(searchInput, event);

    expect(spy).toHaveBeenCalled();
  });
});
