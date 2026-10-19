import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { TechnicalFlowBBViewer } from '../TechnicalFlowBBViewer';
import * as bbApi from '../../../api/bbApi';
import '@testing-library/jest-dom';

vi.mock('../../../api/bbApi', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../../api/bbApi')>();
  return {
    ...actual,
    getBBHorizons: vi.fn(),
    getSymbolBB: vi.fn(),
  };
});

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

  it('safely handles backend DTO with nested horizons without toFixed crash (ST-05)', async () => {
    // Mock backend schema response (BBSymbolPointResponse with nested horizons, without flat value/basis)
    const backendDTOResponse = {
      symbol: 'FPT',
      timeframe: '1D',
      methodology: 'OHLCV_PROXY',
      calculation_as_of: '2026-03-01',
      points: [
        {
          date: '2026-03-01',
          horizons: {
            T03: { horizon: 'T03', bb_value: 12.34, raw_denominator: 10.0, is_warmup: false, direction: 'UP', regime: 'POSITIVE', regime_run_length: 3, value_source: 'proxy', quality: 'HIGH' },
            T20: { horizon: 'T20', bb_value: 45.67, raw_denominator: 40.0, is_warmup: false, direction: 'UP', regime: 'POSITIVE', regime_run_length: 5, value_source: 'proxy', quality: 'HIGH' },
          },
          daily_pressure: 1.2,
          daily_trading_value: 50000000000,
          daily_value_source: 'proxy',
          daily_quality: 'HIGH',
        },
      ],
      summary: {},
    };

    vi.mocked(bbApi.getSymbolBB).mockResolvedValue(backendDTOResponse as unknown as bbApi.BBSymbolSeriesResponse);

    renderWithClient(<TechnicalFlowBBViewer defaultSymbol="FPT" />);

    await waitFor(() => {
      expect(screen.getByTestId('bb-latest-summary')).toBeInTheDocument();
      expect(screen.getByTestId('bb-horizons-breakdown')).toBeInTheDocument();
    });

    // Should display extracted values from T20 without throwing toFixed undefined TypeError
    const summary = screen.getByTestId('bb-latest-summary');
    expect(summary).toHaveTextContent('45.67');
    expect(summary).toHaveTextContent('40.00');

    // Horizon badge for T03 should show 12.34
    const badgeT03 = screen.getByTestId('horizon-badge-T03');
    expect(badgeT03).toHaveTextContent('12.34');

    // Horizon badge for T50 (missing in mock) should show '-' without crashing
    const badgeT50 = screen.getByTestId('horizon-badge-T50');
    expect(badgeT50).toHaveTextContent('-');
  });

  it('safely handles empty or missing horizons with fallback values (ST-05)', async () => {
    const emptyPointResponse = {
      symbol: 'FPT',
      timeframe: '1D',
      methodology: 'OHLCV_PROXY',
      calculation_as_of: '2026-03-01',
      points: [
        {
          date: '2026-03-01',
          // completely empty horizons object
          horizons: {},
        },
      ],
      summary: {},
    };

    vi.mocked(bbApi.getSymbolBB).mockResolvedValue(emptyPointResponse as unknown as bbApi.BBSymbolSeriesResponse);

    renderWithClient(<TechnicalFlowBBViewer defaultSymbol="FPT" />);

    await waitFor(() => {
      expect(screen.getByTestId('bb-latest-summary')).toBeInTheDocument();
    });

    const summary = screen.getByTestId('bb-latest-summary');
    expect(summary).toHaveTextContent('N/A');
  });

  it('unifies chart-format BB response (data[]) and surfaces warming/degraded notices (B41 / GAP-API-02)', async () => {
    const chartAndDegradedResponse = {
      symbol: 'FPT',
      flow_method: 'OHLCV_PROXY',
      methodology_version: 'bb_v1_ohlcv_proxy',
      total_bars: 5,
      coverage_ratio: 0.65,
      publication_status: 'UNAVAILABLE_DEGRADED',
      gate_reasons: ['CONSTITUENT_COVERAGE_BELOW_80_PCT'],
      points: [],
      data: [
        {
          time: '2026-03-02',
          pressure: 0.42,
          trading_value: 25000000000,
          value_source: 'CLOSE_X_VOLUME',
          quality: 'LOW',
          t03: 58.25,
          t03_dir: 'RISING',
          t03_regime: 'POSITIVE',
          t03_run: 2,
          t20: null,
          t20_dir: 'UNKNOWN',
          t20_regime: 'UNKNOWN',
          t20_run: 0,
        },
      ],
    };

    vi.mocked(bbApi.getSymbolBB).mockResolvedValue(
      chartAndDegradedResponse as unknown as bbApi.BBSymbolSeriesResponse
    );

    renderWithClient(<TechnicalFlowBBViewer defaultSymbol="FPT" />);

    await waitFor(() => {
      expect(screen.getByTestId('bb-warmup-notice')).toBeInTheDocument();
      expect(screen.getByTestId('bb-degraded-notice')).toBeInTheDocument();
    });

    expect(screen.getByTestId('bb-warmup-notice')).toHaveTextContent(/Đang tích lũy dữ liệu/i);
    expect(screen.getByTestId('bb-degraded-notice')).toHaveTextContent(/CONSTITUENT_COVERAGE_BELOW_80_PCT/i);
    expect(screen.getByTestId('horizon-badge-T03')).toHaveTextContent('58.25');
  });

  it('displays UNAVAILABLE quality badge on zero coverage or empty points, never HIGH (SQA-016)', async () => {
    const zeroCoverageResponse = {
      symbol: 'EMPTY_SYM',
      timeframe: '1D',
      methodology: 'OHLCV_PROXY',
      calculation_as_of: '2026-03-01',
      coverage_ratio: 0.0,
      publication_status: 'UNAVAILABLE_DEGRADED',
      points: [],
      data: [],
      summary: {},
    };

    vi.mocked(bbApi.getSymbolBB).mockResolvedValue(
      zeroCoverageResponse as unknown as bbApi.BBSymbolSeriesResponse
    );

    renderWithClient(<TechnicalFlowBBViewer defaultSymbol="EMPTY_SYM" />);

    await waitFor(() => {
      expect(screen.getByTestId('bb-data-quality-badge')).toBeInTheDocument();
    });

    const badge = screen.getByTestId('bb-data-quality-badge');
    expect(badge).toHaveTextContent(/CHẤT LƯỢNG DỮ LIỆU: UNAVAILABLE/i);
    expect(badge).not.toHaveTextContent('HIGH');
  });

  it('renders Daily Flow Pressure and Daily Trading Value metrics in summary (SQA-016)', async () => {
    const responseWithDailyMetrics = {
      symbol: 'FPT',
      timeframe: '1D',
      methodology: 'OHLCV_PROXY',
      calculation_as_of: '2026-03-01',
      coverage_ratio: 0.95,
      points: [
        {
          date: '2026-03-01',
          value: 50.0,
          basis: 45.0,
          daily_pressure: 1.85,
          daily_trading_value: 35000000000,
          daily_quality: 'HIGH',
          regime: 'POSITIVE',
          horizons: {
            T20: { horizon: 'T20', bb_value: 50.0, raw_denominator: 45.0, is_warmup: false, regime: 'POSITIVE' },
          },
        },
      ],
      summary: {},
    };

    vi.mocked(bbApi.getSymbolBB).mockResolvedValue(
      responseWithDailyMetrics as unknown as bbApi.BBSymbolSeriesResponse
    );

    renderWithClient(<TechnicalFlowBBViewer defaultSymbol="FPT" />);

    await waitFor(() => {
      expect(screen.getByTestId('bb-daily-pressure')).toBeInTheDocument();
      expect(screen.getByTestId('bb-daily-trading-value')).toBeInTheDocument();
    });

    expect(screen.getByTestId('bb-daily-pressure')).toHaveTextContent('1.85');
    expect(screen.getByTestId('bb-daily-trading-value')).toHaveTextContent('35.000.000.000');
  });
});
