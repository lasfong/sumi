import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { TechnicalFlowBBViewer } from '../TechnicalFlowBBViewer';
import * as bbApi from '../../../api/bbApi';
import '@testing-library/jest-dom';

vi.mock('../../../api/bbApi', () => ({
  getBBHorizons: vi.fn(),
  getSymbolBB: vi.fn(),
}));

const mockBBResponse = {
  symbol: 'FPT',
  timeframe: '1D',
  methodology: 'OHLCV_PROXY',
  calculation_as_of: '2026-03-01',
  points: [
    {
      bar_index: 100,
      date: '2026-03-01',
      close: 120.5,
      volume: 1500000,
      value: 65.4,
      lower: 35.0,
      upper: 85.0,
      basis: 50.0,
      bandwidth: 0.35,
      percent_b: 0.608,
      regime: 'POSITIVE',
      data_quality: 'HIGH',
      is_valid: true,
    },
  ],
  summary: { current_value: 65.4 },
};

const renderWithClient = (ui: React.ReactElement) => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

describe('TechnicalFlowBBViewer Component (Master Appendix F.3)', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(bbApi.getBBHorizons).mockResolvedValue({
      horizons: [
        { id: 'T03', lookback_bars: 3 },
        { id: 'T05', lookback_bars: 5 },
        { id: 'T10', lookback_bars: 10 },
        { id: 'T20', lookback_bars: 20 },
        { id: 'T50', lookback_bars: 50 },
        { id: 'T200', lookback_bars: 200 },
      ],
      default_horizons: ['T03', 'T05', 'T20', 'T50', 'T200'],
      methodology: 'OHLCV_PROXY',
    });
    vi.mocked(bbApi.getSymbolBB).mockResolvedValue(mockBBResponse);
  });

  it('renders BB viewer with OHLCV_PROXY methodology badge and HIGH data quality badge', async () => {
    renderWithClient(<TechnicalFlowBBViewer defaultSymbol="FPT" />);

    await waitFor(() => {
      expect(screen.getByTestId('bb-latest-summary')).toBeInTheDocument();
      expect(screen.getByTestId('bb-methodology-badge')).toHaveTextContent('PHƯƠNG PHÁP: OHLCV_PROXY');
      expect(screen.getByTestId('bb-data-quality-badge')).toHaveTextContent('CHẤT LƯỢNG DỮ LIỆU: HIGH');
    });

    // Summary metrics
    const summary = screen.getByTestId('bb-latest-summary');
    expect(summary).toHaveTextContent('65.40'); // Value
    expect(summary).toHaveTextContent('50.00'); // Basis
    expect(summary).toHaveTextContent('POSITIVE'); // Regime

    // Mandatory disclaimer
    const disclaimer = screen.getByTestId('bb-audit-disclaimer');
    expect(disclaimer).toHaveTextContent(/không suy diễn chỉ báo này là dòng tiền mua\/bán chủ động trực tiếp từ sổ lệnh/i);
  });

  it('allows toggling BB horizons including optional T10 (F.3)', async () => {
    renderWithClient(<TechnicalFlowBBViewer defaultSymbol="FPT" />);

    await waitFor(() => {
      expect(screen.getByTestId('toggle-horizon-T10')).toBeInTheDocument();
    });

    // Toggle T10 on
    const t10Btn = screen.getByTestId('toggle-horizon-T10');
    fireEvent.click(t10Btn);

    expect(bbApi.getSymbolBB).toHaveBeenCalled();
  });
});
