import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { SignalCatalog } from '../SignalCatalog';
import { SignalCatalogModal } from '../SignalCatalogModal';
import * as signalsApi from '../../../api/signalsApi';
import type { SignalDefinition } from '../../../types/signals';
import '@testing-library/jest-dom';

vi.mock('../../../api/signalsApi', () => ({
  getSignalRegistry: vi.fn(),
}));

const mockSignals: SignalDefinition[] = [
  {
    name: 'volume.spike',
    version: '1.0.0',
    category: 'volume',
    label_vi: 'Đột Biến Khối Lượng (Volume Spike)',
    description: 'Khối lượng giao dịch vượt ngưỡng trung bình.',
    output_type: 'bool',
    parameters_schema: {
      period: { type: 'int', default: 20, minimum: 5, maximum: 200, description: 'Chu kỳ nến' },
      multiplier: { type: 'float', default: 2.0, minimum: 1.0, maximum: 10.0, description: 'Hệ số đột biến' },
    },
    default_parameters: { period: 20, multiplier: 2.0 },
    dependencies: [],
    warmup_bars: 20,
    causal_delay_bars: 0,
    status: 'ACTIVE',
    ast_alias: 'volume__spike',
  },
  {
    name: 'health.score',
    version: '1.0.0',
    category: 'health',
    label_vi: 'Điểm Sức Khỏe Kỹ Thuật Tổng Hợp',
    description: 'Điểm tổng hợp 4 họ tín hiệu: Xu hướng, Động lượng, Chuyển dịch, Dòng tiền.',
    output_type: 'float',
    parameters_schema: {
      warmup: { type: 'int', default: 50, minimum: 20, maximum: 200, description: 'Số nến khởi động' },
    },
    default_parameters: { warmup: 50 },
    dependencies: [],
    warmup_bars: 50,
    causal_delay_bars: 0,
    status: 'ACTIVE',
    ast_alias: 'health__score',
  },
  {
    name: 'bb.turn_up',
    version: '1.0.0',
    category: 'flow',
    label_vi: 'Dòng Tiền Đảo Chiều Tăng (Turn Up)',
    description: 'Dòng tiền BB chạm cận dưới và bật tăng qua ngưỡng.',
    output_type: 'bool',
    parameters_schema: {
      threshold: { type: 'float', default: 30.0, minimum: 0.0, maximum: 100.0, description: 'Ngưỡng dưới' },
    },
    default_parameters: { threshold: 30.0 },
    dependencies: [],
    warmup_bars: 20,
    causal_delay_bars: 0,
    status: 'EXPERIMENTAL',
    ast_alias: 'bb__turn_up',
  },
  {
    name: 'pattern.bullish_engulfing',
    version: '1.0.0',
    category: 'pattern',
    label_vi: 'Nhấn Chìm Tăng (Bullish Engulfing)',
    description: 'Mẫu hình nến nhấn chìm đảo chiều tăng.',
    output_type: 'bool',
    parameters_schema: {},
    default_parameters: {},
    dependencies: [],
    warmup_bars: 2,
    causal_delay_bars: 0,
    status: 'ACTIVE',
    ast_alias: 'pattern__bullish_engulfing',
  },
];

const renderWithClient = (ui: React.ReactElement) => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

describe('SignalCatalog Component (UI-SIG-001, UI-SIG-002, UI-SIG-003)', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(signalsApi.getSignalRegistry).mockResolvedValue({
      signals: mockSignals,
    });
  });

  it('renders signal catalog and browses signals by category (UI-SIG-001)', async () => {
    renderWithClient(<SignalCatalog />);

    await waitFor(() => {
      expect(screen.getByText('Đột Biến Khối Lượng (Volume Spike)')).toBeInTheDocument();
    });

    // All 4 mock signals should initially appear
    expect(screen.getByText('Đột Biến Khối Lượng (Volume Spike)')).toBeInTheDocument();
    expect(screen.getByText('Điểm Sức Khỏe Kỹ Thuật Tổng Hợp')).toBeInTheDocument();
    expect(screen.getByText('Dòng Tiền Đảo Chiều Tăng (Turn Up)')).toBeInTheDocument();
    expect(screen.getByText('Nhấn Chìm Tăng (Bullish Engulfing)')).toBeInTheDocument();

    // Click Category Tab: Health
    const healthTab = screen.getByTestId('category-tab-health');
    fireEvent.click(healthTab);

    // Only Health signal should remain
    expect(screen.getByText('Điểm Sức Khỏe Kỹ Thuật Tổng Hợp')).toBeInTheDocument();
    expect(screen.queryByText('Đột Biến Khối Lượng (Volume Spike)')).not.toBeInTheDocument();
    expect(screen.queryByText('Dòng Tiền Đảo Chiều Tăng (Turn Up)')).not.toBeInTheDocument();

    // Click Category Tab: Flow
    const flowTab = screen.getByTestId('category-tab-flow');
    fireEvent.click(flowTab);

    expect(screen.getByText('Dòng Tiền Đảo Chiều Tăng (Turn Up)')).toBeInTheDocument();
    expect(screen.queryByText('Điểm Sức Khỏe Kỹ Thuật Tổng Hợp')).not.toBeInTheDocument();
  });

  it('displays Vietnamese label, parameters, version, status, and experimental badge (UI-SIG-002)', async () => {
    renderWithClient(<SignalCatalog />);

    await waitFor(() => {
      expect(screen.getByText('Dòng Tiền Đảo Chiều Tăng (Turn Up)')).toBeInTheDocument();
    });

    const flowCard = screen.getByTestId('signal-card-bb.turn_up');
    expect(flowCard).toHaveTextContent('THỬ NGHIỆM'); // EXPERIMENTAL badge
    expect(flowCard).toHaveTextContent('v1.0.0');
    expect(flowCard).toHaveTextContent('BOOL');
    expect(flowCard).toHaveTextContent('AST: bb__turn_up');
    expect(flowCard).toHaveTextContent('threshold:');
    expect(flowCard).toHaveTextContent('30');
  });

  it('hides advanced parameter boundaries by default and reveals on toggle (UI-SIG-003)', async () => {
    renderWithClient(<SignalCatalog />);

    await waitFor(() => {
      expect(screen.getByText('Đột Biến Khối Lượng (Volume Spike)')).toBeInTheDocument();
    });

    // Default view: advanced parameters are hidden
    expect(screen.queryByTestId('advanced-params-details')).not.toBeInTheDocument();

    // Click toggle button
    const toggleBtn = screen.getByTestId('toggle-advanced-params-btn');
    expect(toggleBtn).toHaveTextContent('⚙️ Xem tham số nâng cao');
    fireEvent.click(toggleBtn);

    // Advanced parameters should now be displayed
    expect(toggleBtn).toHaveTextContent('👁️ Ẩn tham số nâng cao');
    expect(screen.getAllByTestId('advanced-params-details').length).toBeGreaterThan(0);
    expect(screen.getByText(/Phạm vi: \[5, 200\]/i)).toBeInTheDocument();

    // Toggle back to hide
    fireEvent.click(toggleBtn);
    expect(screen.queryByTestId('advanced-params-details')).not.toBeInTheDocument();
  });

  it('filters signals by search query input', async () => {
    renderWithClient(<SignalCatalog />);

    await waitFor(() => {
      expect(screen.getByText('Đột Biến Khối Lượng (Volume Spike)')).toBeInTheDocument();
    });

    const searchInput = screen.getByTestId('signal-catalog-search-input');
    fireEvent.change(searchInput, { target: { value: 'engulfing' } });

    expect(screen.getByText('Nhấn Chìm Tăng (Bullish Engulfing)')).toBeInTheDocument();
    expect(screen.queryByText('Đột Biến Khối Lượng (Volume Spike)')).not.toBeInTheDocument();
    expect(screen.queryByText('Điểm Sức Khỏe Kỹ Thuật Tổng Hợp')).not.toBeInTheDocument();
  });

  it('triggers onSelectSignal callback when card button is clicked', async () => {
    const handleSelect = vi.fn();
    renderWithClient(<SignalCatalog onSelectSignal={handleSelect} />);

    await waitFor(() => {
      expect(screen.getByText('Điểm Sức Khỏe Kỹ Thuật Tổng Hợp')).toBeInTheDocument();
    });

    const healthCard = screen.getByTestId('signal-card-health.score');
    fireEvent.click(healthCard);

    expect(handleSelect).toHaveBeenCalledWith(
      expect.objectContaining({ name: 'health.score', ast_alias: 'health__score' })
    );
  });

  it('renders SignalCatalogModal and handles close on button click', async () => {
    const handleClose = vi.fn();
    renderWithClient(<SignalCatalogModal isOpen={true} onClose={handleClose} />);

    await waitFor(() => {
      expect(screen.getByTestId('signal-catalog-modal')).toBeInTheDocument();
    });

    const closeBtn = screen.getByTestId('close-signal-catalog-btn');
    fireEvent.click(closeBtn);

    expect(handleClose).toHaveBeenCalledTimes(1);
  });
});
