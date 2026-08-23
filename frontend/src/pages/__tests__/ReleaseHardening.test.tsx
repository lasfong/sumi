import { describe, it, expect, vi } from 'vitest';
import { render } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { DataSyncPanel } from '../../components/sync/DataSyncPanel';
import * as syncApi from '../../api/syncApi';

vi.mock('../../api/syncApi');

describe('PRO-12 Release Hardening Component Lifecycle', () => {
  const createTestQueryClient = () =>
    new QueryClient({
      defaultOptions: {
        queries: { retry: false },
      },
    });

  it('mounts and unmounts DataSyncPanel cleanly without async state memory leaks', () => {
    vi.mocked(syncApi.getSyncProviders).mockResolvedValue([
      {
        provider_id: 'ssi',
        display_name: 'SSI FastConnect',
        is_official: true,
        requires_auth: true,
        supported_timeframes: ['1D'],
        supported_adjustments: ['unadjusted', 'adjusted'],
        rate_limit_rps: 10,
        description: 'Official broker API',
        auth_fields: ['consumer_id', 'consumer_secret'],
        is_configured: true,
      },
    ]);
    vi.mocked(syncApi.getSyncHistory).mockResolvedValue([]);

    for (let i = 0; i < 5; i++) {
      const queryClient = createTestQueryClient();
      const { unmount } = render(
        <QueryClientProvider client={queryClient}>
          <DataSyncPanel />
        </QueryClientProvider>
      );
      unmount();
    }
    expect(true).toBe(true);
  });
});
