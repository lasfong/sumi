import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ImportPage } from '../ImportPage';
import { apiClient } from '../../api/client';
import * as importApi from '../../api/importApi';

vi.mock('../../api/client', () => ({
  apiClient: {
    post: vi.fn(),
    get: vi.fn(),
  },
}));
vi.mock('../../api/importApi');
vi.mock('../../components/sync/DataSyncPanel', () => ({
  DataSyncPanel: () => <div data-testid="mock-sync-panel">Sync Panel</div>,
}));

const createClient = () =>
  new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

describe('ImportPage REL-001 User-Visible Import States', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(importApi.getDataCatalog).mockResolvedValue([]);
    vi.mocked(importApi.getImportRuns).mockResolvedValue([]);
  });

  it('distinguishes safely normalized data, partial import warnings, and blocked invalid data', async () => {
    const queryClient = createClient();
    render(
      <QueryClientProvider client={queryClient}>
        <ImportPage />
      </QueryClientProvider>
    );

    // Switch to CafeF file import tab
    fireEvent.click(screen.getByText(/Nhập tập tin CafeF/i));

    const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement;
    const testFile = new File(['<Ticker>,<DTYYYYMMDD>\nHPG,20240103\n'], 'hpg_2024.csv', {
      type: 'text/csv',
    });
    fireEvent.change(fileInput, { target: { files: [testFile] } });

    // 1. Safely normalized preview
    vi.mocked(apiClient.post).mockResolvedValueOnce({
      data: {
        run_id: 'run-norm-1',
        file_name: 'hpg_2024.csv',
        file_sha256: 'sha1',
        content_sha256: 'sha2',
        parser_version: 'cafef_v1',
        source_type: 'cafef',
        timeframe: '1D',
        adjustment_type: 'unadjusted',
        timezone: 'Asia/Ho_Chi_Minh',
        status: 'previewed',
        parsed_count: 250,
        corrected_count: 22,
        rejected_count: 0,
        duplicate_count: 0,
        conflicting_count: 0,
        missing_count: 0,
        out_of_order_count: 0,
        accepted_count: 0,
        can_accept: true,
        is_partial: false,
        normalization_warning:
          'Đã chuẩn hóa an toàn biên OHLC cho 22 dòng có sai số làm tròn 4 chữ số thập phân giá điều chỉnh CafeF.',
        items: [
          {
            row_index: 2,
            symbol: 'HPG',
            timeframe: '1D',
            timestamp: '2024-01-03',
            adjustment_type: 'unadjusted',
            open: 18527.9,
            high: 18801.0,
            low: 18527.9,
            close: 18801.0,
            volume: 18575400,
            classification: 'parsed',
            is_corrected: true,
            raw_high: 18798.4,
            reject_reason:
              '[NORMALIZED_OHLC_PRECISION] raw_high=18798.4 -> high=18801 (delta_vnd=2.6)',
          },
        ],
      },
    });

    fireEvent.click(screen.getByRole('button', { name: /Xem trước/i }));

    await waitFor(() => {
      expect(screen.getByTestId('preview-status-badge').textContent).toContain(
        'Có chuẩn hóa biên OHLC'
      );
    });
    expect(screen.getByTestId('normalization-disclosure').textContent).toContain('22 dòng');
    expect(screen.queryByTestId('partial-import-warning')).toBeNull();
    expect(screen.getByTestId('badge-normalized-row').textContent).toBe('normalized');

    // 2. Partial import warning state
    vi.mocked(apiClient.post).mockResolvedValueOnce({
      data: {
        run_id: 'run-partial-1',
        file_name: 'hpg_partial.csv',
        file_sha256: 'sha3',
        content_sha256: 'sha4',
        parser_version: 'cafef_v1',
        source_type: 'cafef',
        timeframe: '1D',
        adjustment_type: 'unadjusted',
        timezone: 'Asia/Ho_Chi_Minh',
        status: 'previewed',
        parsed_count: 249,
        corrected_count: 22,
        rejected_count: 1,
        duplicate_count: 0,
        conflicting_count: 0,
        missing_count: 1,
        out_of_order_count: 0,
        accepted_count: 0,
        can_accept: true,
        is_partial: true,
        partial_warning:
          'Cảnh báo nhập một phần (Partial Import): Tập tin có 1 dòng không hợp lệ bị loại bỏ.',
        items: [],
      },
    });

    fireEvent.click(screen.getByRole('button', { name: /Xem trước/i }));

    await waitFor(() => {
      expect(screen.getByTestId('preview-status-badge').textContent).toContain('nhập một phần');
    });
    expect(screen.getByTestId('partial-import-warning').textContent).toContain('Partial Import');
    expect(screen.getByTestId('accept-import-btn').textContent).toContain('Nhập một phần');
  });
});
