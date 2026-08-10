import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useSessionSelection } from '../useSessionSelection';
import { useReplayStore } from '../../store/replayStore';
import { MemoryRouter } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';
import * as replayApi from '../../api/replayApi';
import type { ReplaySession } from '../../types';

vi.mock('../../api/replayApi', async () => {
  const actual = await vi.importActual<typeof import('../../api/replayApi')>('../../api/replayApi');
  return {
    ...actual,
    getReplaySession: vi.fn(),
  };
});

const mockSession42 = {
  id: 42,
  symbol: 'FPT',
  timeframe: '1D',
  start_date: '2024-01-01',
  end_date: '2024-06-01',
  current_index: 15,
  status: 'active',
  mode: 'normal',
} as ReplaySession;

const mockSession10 = {
  id: 10,
  symbol: 'VNINDEX',
  timeframe: '1D',
  start_date: '2024-01-01',
  end_date: '2024-06-01',
  current_index: 30,
  status: 'active',
  mode: 'normal',
} as ReplaySession;

const createWrapper = (initialEntry = '/journal') => {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={[initialEntry]}>
        {children}
      </MemoryRouter>
    </QueryClientProvider>
  );
};

describe('useSessionSelection controller (R02-01 / PRO-UX-02 / PRO-UX-03)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useReplayStore.getState().clearSession();
    vi.mocked(replayApi.getReplaySession).mockImplementation(async (id: number) => {
      if (id === 42) return mockSession42;
      if (id === 10) return mockSession10;
      throw new Error('Session not found');
    });
  });

  it('valid URL wins and synchronizes store', async () => {
    const wrapper = createWrapper('/journal?session=42');
    const { result } = renderHook(() => useSessionSelection(), { wrapper });

    await waitFor(() => {
      expect(result.current.sessionId).toBe(42);
    });
    expect(useReplayStore.getState().sessionId).toBe(42);
  });

  it('valid persisted fallback canonicalizes URL parameter', async () => {
    useReplayStore.getState().setSession(10);
    const wrapper = createWrapper('/journal');
    const { result } = renderHook(() => useSessionSelection(), { wrapper });

    await waitFor(() => {
      expect(result.current.sessionId).toBe(10);
    });
  });

  it('invalid syntax does not fall back silently to ID 1', async () => {
    const wrapper = createWrapper('/journal?session=abc');
    const { result } = renderHook(() => useSessionSelection(), { wrapper });

    expect(result.current.sessionId).toBeNull();
    expect(useReplayStore.getState().sessionId).toBeNull();
  });

  it('malformed URL syntax clears selection even when store is pre-populated with a valid session', async () => {
    useReplayStore.getState().setSession(10);
    const wrapper = createWrapper('/journal?session=abc');
    const { result } = renderHook(() => useSessionSelection(), { wrapper });

    await waitFor(() => {
      expect(result.current.sessionId).toBeNull();
      expect(useReplayStore.getState().sessionId).toBeNull();
    });
    expect(replayApi.getReplaySession).not.toHaveBeenCalled();
  });

  it('deleted or missing session ID is rejected and clears store', async () => {
    useReplayStore.getState().setSession(999);
    const wrapper = createWrapper('/journal?session=999');
    const { result } = renderHook(() => useSessionSelection(), { wrapper });

    await waitFor(() => {
      expect(result.current.sessionId).toBeNull();
      expect(useReplayStore.getState().sessionId).toBeNull();
    });
  });

  it('selectSession updates store and search parameters together', async () => {
    const wrapper = createWrapper('/journal');
    const { result } = renderHook(() => useSessionSelection(), { wrapper });

    act(() => {
      result.current.selectSession(42);
    });

    await waitFor(() => {
      expect(result.current.sessionId).toBe(42);
    });
    expect(useReplayStore.getState().sessionId).toBe(42);
  });

  it('clearSession resets selection cleanly', async () => {
    const wrapper = createWrapper('/journal?session=42');
    const { result } = renderHook(() => useSessionSelection(), { wrapper });

    await waitFor(() => {
      expect(result.current.sessionId).toBe(42);
    });

    act(() => {
      result.current.clearSession();
    });

    expect(useReplayStore.getState().sessionId).toBeNull();
  });

  it('generates session-preserving route paths', async () => {
    const wrapper = createWrapper('/journal?session=42');
    const { result } = renderHook(() => useSessionSelection(), { wrapper });

    await waitFor(() => {
      expect(result.current.sessionId).toBe(42);
    });

    expect(result.current.getSessionPath('/analytics')).toBe('/analytics?session=42');
    expect(result.current.getSessionPath('/analytics', 10)).toBe('/analytics?session=10');
  });
});
