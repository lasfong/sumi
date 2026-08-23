import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { DataSyncPanel } from '../../components/sync/DataSyncPanel';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import * as syncApi from '../../api/syncApi';

vi.mock('../../api/syncApi');

const mockProviders: syncApi.ProviderMetadata[] = [
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
  {
    provider_id: 'vnstock',
    display_name: 'vnstock (Community Fallback)',
    is_official: false,
    requires_auth: false,
    supported_timeframes: ['1D'],
    supported_adjustments: ['unadjusted', 'adjusted'],
    rate_limit_rps: 3,
    description: 'Open source community',
    auth_fields: [],
    is_configured: true,
  },
];

const mockPreviewResponse: syncApi.SyncPreviewResponse = {
  sync_id: 'sync-12345',
  provider_id: 'ssi',
  symbol: 'VNM',
  start_date: '2026-08-03',
  end_date: '2026-08-07',
  timeframe: '1D',
  adjustment_type: 'unadjusted',
  status: 'previewed',
  parsed_count: 5,
  rejected_count: 0,
  duplicate_count: 0,
  conflicting_count: 0,
  missing_count: 0,
  out_of_order_count: 0,
  can_accept: true,
  content_sha256: 'a1b2c3d4e5f67890a1b2c3d4e5f67890a1b2c3d4e5f67890a1b2c3d4e5f67890',
  items: [
    {
      row_index: 1,
      symbol: 'VNM',
      timeframe: '1D',
      timestamp: '2026-08-03',
      adjustment_type: 'unadjusted',
      open: 72000,
      high: 73000,
      low: 71500,
      close: 72500,
      volume: 500000,
      classification: 'parsed',
    },
  ],
};

const mockHistory: syncApi.SyncManifest[] = [
  {
    sync_id: 'sync-hist-1',
    created_at: '2026-08-16T12:00:00Z',
    provider_id: 'ssi',
    symbol: 'FPT',
    start_date: '2026-08-01',
    end_date: '2026-08-10',
    timeframe: '1D',
    adjustment_type: 'unadjusted',
    status: 'accepted',
    parsed_count: 6,
    duplicate_count: 0,
    conflicting_count: 0,
    accepted_count: 6,
    duration_ms: 12.5,
    accepted_at: '2026-08-16T12:00:01Z',
    manifest: {
      audit_version: 'SUMI_SYNC_MANIFEST_V1',
      symbol: 'FPT',
    },
  },
];

const renderComponent = () => {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={queryClient}>
      <DataSyncPanel />
    </QueryClientProvider>
  );
};

describe('DataSyncPanel (PRO-DATA-08, PRO-DATA-09, PRO-DATA-10)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(syncApi.getSyncProviders).mockResolvedValue(mockProviders);
    vi.mocked(syncApi.getSyncHistory).mockResolvedValue(mockHistory);
  });

  it('renders provider selection, symbol input, date picker, and history log', async () => {
    renderComponent();

    expect(screen.getByText(/Đồng bộ Dữ liệu Trực tuyến/i)).toBeInTheDocument();
    expect(await screen.findByRole('option', { name: /SSI FastConnect/i })).toBeInTheDocument();
    expect(screen.getByDisplayValue('VNM')).toBeInTheDocument();

    // Check history item
    expect(await screen.findByText('6 nến')).toBeInTheDocument();
    expect(screen.getAllByText('FPT').length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText(/Đã chấp nhận/i)).toBeInTheDocument();
  });

  it('tests provider connectivity with credential validation', async () => {
    vi.mocked(syncApi.testProviderConnection).mockResolvedValue({
      success: true,
      provider_id: 'ssi',
      message: 'Kết nối thành công đến SSI FastConnect (Độ trễ: 15.2ms)',
      latency_ms: 15.2,
    });

    renderComponent();

    const testBtn = await screen.findByRole('button', { name: /Kiểm tra kết nối/i });
    fireEvent.click(testBtn);

    await waitFor(() => {
      expect(syncApi.testProviderConnection).toHaveBeenCalledWith('ssi', undefined);
      expect(screen.getByText(/Kết nối thành công đến SSI FastConnect/i)).toBeInTheDocument();
    });
  });

  it('generates pre-commit dry-run preview and allows cancellation', async () => {
    vi.mocked(syncApi.generateSyncPreview).mockResolvedValue(mockPreviewResponse);

    renderComponent();

    const previewBtn = await screen.findByRole('button', { name: /Xem trước dữ liệu/i });
    fireEvent.click(previewBtn);

    await waitFor(() => {
      expect(syncApi.generateSyncPreview).toHaveBeenCalled();
      expect(screen.getByText(/Kết quả Phân loại & Kiểm duyệt/i)).toBeInTheDocument();
      expect(screen.getByText('Nến mới (Sẽ ghi)')).toBeInTheDocument();
    });

    // Cancel preview
    const cancelBtn = screen.getByRole('button', { name: /Hủy bỏ/i });
    fireEvent.click(cancelBtn);

    expect(screen.queryByText(/Kết quả Phân loại & Kiểm duyệt/i)).not.toBeInTheDocument();
  });

  it('executes atomic commit on confirmation and refreshes history', async () => {
    vi.mocked(syncApi.generateSyncPreview).mockResolvedValue(mockPreviewResponse);
    vi.mocked(syncApi.executeSync).mockResolvedValue({
      sync_id: 'sync-12345',
      status: 'accepted',
      accepted_count: 5,
      symbol: 'VNM',
      timeframe: '1D',
      adjustment_type: 'unadjusted',
      duration_ms: 25.0,
      message: 'Đã đồng bộ và lưu thành công 5 nến cho VNM',
      manifest: { audit_version: 'SUMI_SYNC_MANIFEST_V1' },
    });

    renderComponent();

    const previewBtn = await screen.findByRole('button', { name: /Xem trước dữ liệu/i });
    fireEvent.click(previewBtn);

    const confirmBtn = await screen.findByRole('button', { name: /Xác nhận & Đồng bộ/i });
    fireEvent.click(confirmBtn);

    await waitFor(() => {
      expect(syncApi.executeSync).toHaveBeenCalledWith(
        'sync-12345',
        'a1b2c3d4e5f67890a1b2c3d4e5f67890a1b2c3d4e5f67890a1b2c3d4e5f67890'
      );
    });
  });

  it('triggers rollback for an accepted sync run', async () => {
    vi.mocked(syncApi.rollbackSync).mockResolvedValue({
      sync_id: 'sync-hist-1',
      status: 'rolled_back',
      restored_mutations_count: 6,
      message: 'Hoàn tác thành công lượt đồng bộ FPT, khôi phục 6 điểm dữ liệu',
    });

    renderComponent();

    const rollbackBtn = await screen.findByRole('button', { name: /Hoàn tác/i });
    fireEvent.click(rollbackBtn);

    await waitFor(() => {
      expect(syncApi.rollbackSync).toHaveBeenCalledWith('sync-hist-1');
    });
  });
});
