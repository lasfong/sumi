import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { SignalExplanationInspector } from '../SignalExplanationInspector';
import * as signalsApi from '../../../api/signalsApi';
import type { SignalCalculationResponse, SignalRegistryResponse } from '../../../types/signals';
import '@testing-library/jest-dom';

vi.mock('../../../api/signalsApi', () => ({
  getSignalRegistry: vi.fn(),
  calculateReplaySignals: vi.fn(),
}));

const mockRegistry: SignalRegistryResponse = {
  signals: [
    {
      name: 'health.score',
      version: '1.0.0',
      category: 'health',
      label_vi: 'Điểm Sức Khỏe Kỹ Thuật Tổng Hợp',
      description: 'Điểm tổng hợp 4 họ tín hiệu',
      output_type: 'float' as const,
      parameters_schema: {
        warmup: { type: 'int' as const, default: 50, minimum: 20, maximum: 200, description: 'Warmup' },
      },
      default_parameters: { warmup: 50 },
      dependencies: [],
      warmup_bars: 50,
      causal_delay_bars: 0,
      status: 'ACTIVE' as const,
      ast_alias: 'health__score',
    },
    {
      name: 'vsa.strong_demand',
      version: '1.0.0',
      category: 'vsa',
      label_vi: 'Cầu Mạnh (Strong Demand)',
      description: 'Lực cầu lớn đẩy giá đóng cửa sát mức cao nhất phiên kèm khối lượng cao.',
      output_type: 'bool' as const,
      parameters_schema: {},
      default_parameters: {},
      dependencies: [],
      warmup_bars: 20,
      causal_delay_bars: 0,
      status: 'ACTIVE' as const,
      ast_alias: 'vsa__strong_demand',
    },
  ],
};

const renderWithClient = (ui: React.ReactElement) => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

describe('SignalExplanationInspector Component (UI-EXP-001)', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(signalsApi.getSignalRegistry).mockResolvedValue(mockRegistry);
  });

  it('renders active signal explanation, structured reasons and health breakdown (UI-EXP-001)', async () => {
    const mockCalcResponse: SignalCalculationResponse = {
      session_id: 1,
      observed_current_index: 25,
      timeframe: '1D',
      results: [
        {
          signal_name: 'health.score',
          signal_version: '1.0.0',
          resolved_params: { warmup: 50 },
          params_hash: 'hash123',
          points: [
            {
              bar_index: 25,
              timestamp: '2026-03-01',
              output_type: 'float',
              value: 42.5,
              quality: 'VALID',
              reasons: [
                'trend_ema20_above_ema50_slope_positive',
                'momentum_rsi_above_center_slope_positive',
                'transition_macd_hist_expanding',
                'bb_absent_neutral',
              ],
              baseline: null,
              current_volume: 1200000,
              relative_volume: 1.45,
              threshold: 35.0,
              availability_event: 'BAR_CLOSE',
              available_at_index: 25,
              available_at_timestamp: '2026-03-01',
            },
          ],
        },
        {
          signal_name: 'bb.direction_rising',
          signal_version: '1.0.0',
          resolved_params: { horizon_bars: 20 },
          params_hash: 'hashbbdir',
          points: [
            {
              bar_index: 25,
              timestamp: '2026-03-01',
              output_type: 'bool',
              value: true,
              quality: 'VALID',
              reasons: ['bb_proxy_slope_positive'],
              baseline: null,
              current_volume: null,
              relative_volume: null,
              threshold: null,
              availability_event: 'BAR_CLOSE',
              available_at_index: 25,
              available_at_timestamp: '2026-03-01',
            },
          ],
        },
        {
          signal_name: 'bb.regime_positive',
          signal_version: '1.0.0',
          resolved_params: { horizon_bars: 20 },
          params_hash: 'hashbbreg',
          points: [
            {
              bar_index: 25,
              timestamp: '2026-03-01',
              output_type: 'bool',
              value: true,
              quality: 'VALID',
              reasons: ['bb_proxy_value_above_50'],
              baseline: null,
              current_volume: null,
              relative_volume: null,
              threshold: null,
              availability_event: 'BAR_CLOSE',
              available_at_index: 25,
              available_at_timestamp: '2026-03-01',
            },
          ],
        },
      ],
    };

    vi.mocked(signalsApi.calculateReplaySignals).mockResolvedValue(mockCalcResponse);

    renderWithClient(
      <SignalExplanationInspector
        sessionId={1}
        currentIndex={25}
        timeframe="1D"
        currentTimestamp="2026-03-01"
      />
    );

    await waitFor(() => {
      expect(screen.getByTestId('signal-explanation-inspector')).toBeInTheDocument();
      expect(screen.getByTestId('signal-explanation-quality-badge')).toHaveTextContent('VALID');
    });

    // Verify Score & Reasons
    expect(screen.getByText('42.50')).toBeInTheDocument();
    const reasonsList = screen.getByTestId('signal-reasons-list');
    expect(reasonsList).toHaveTextContent('trend ema20 above ema50 slope positive');
    expect(reasonsList).toHaveTextContent('bb absent neutral');

    // Verify Money Flow BB Section and OHLCV_PROXY badge
    expect(screen.getByTestId('badge-flow-method')).toHaveTextContent('OHLCV_PROXY');
    expect(screen.getByText(/Xu hướng T20:/i)).toBeInTheDocument();
    expect(screen.getByText('TĂNG ▲')).toBeInTheDocument();
    expect(screen.getByText(/Vùng T20:/i)).toBeInTheDocument();
    expect(screen.getByText('DƯƠNG (> 50)')).toBeInTheDocument();

    // Verify Causal availability note
    expect(screen.getByText(/Thời điểm khả dụng: BAR_CLOSE/i)).toBeInTheDocument();
  });

  it('allows switching selected signal from dropdown to inspect other signals', async () => {
    vi.mocked(signalsApi.calculateReplaySignals).mockResolvedValue({
      session_id: 1,
      observed_current_index: 25,
      timeframe: '1D',
      results: [
        {
          signal_name: 'health.score',
          signal_version: '1.0.0',
          resolved_params: { warmup: 50 },
          params_hash: 'hash123',
          points: [
            {
              bar_index: 25,
              timestamp: '2026-03-01',
              output_type: 'float',
              value: 42.5,
              quality: 'VALID',
              reasons: ['trend_positive'],
              baseline: null,
              current_volume: 1200000,
              relative_volume: 1.45,
              threshold: 35.0,
              availability_event: 'BAR_CLOSE',
              available_at_index: 25,
              available_at_timestamp: '2026-03-01',
            },
          ],
        },
        {
          signal_name: 'vsa.strong_demand',
          signal_version: '1.0.0',
          resolved_params: {},
          params_hash: 'hashvsa',
          points: [
            {
              bar_index: 25,
              timestamp: '2026-03-01',
              output_type: 'bool',
              value: true,
              quality: 'VALID',
              reasons: ['relative_volume_1.82_ge_1.50', 'close_location_0.84_ge_0.70'],
              baseline: null,
              current_volume: 1800000,
              relative_volume: 1.82,
              threshold: 1.5,
              availability_event: 'BAR_CLOSE',
              available_at_index: 25,
              available_at_timestamp: '2026-03-01',
            },
          ],
        },
      ],
    });

    renderWithClient(
      <SignalExplanationInspector
        sessionId={1}
        currentIndex={25}
        timeframe="1D"
        currentTimestamp="2026-03-01"
      />
    );

    await waitFor(() => {
      expect(screen.getByText(/Cầu Mạnh/i)).toBeInTheDocument();
    });

    const select = screen.getByTestId('signal-picker-select');
    fireEvent.change(select, { target: { value: 'vsa.strong_demand' } });

    await waitFor(() => {
      expect(screen.getByText('Cầu Mạnh (Strong Demand)')).toBeInTheDocument();
      expect(screen.getByText('KÍCH HOẠT (TRUE) ✓')).toBeInTheDocument();
      expect(screen.getByText(/relative volume 1.82 ge 1.50/i)).toBeInTheDocument();
    });
  });
});
