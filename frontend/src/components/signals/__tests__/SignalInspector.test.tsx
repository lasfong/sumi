import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import '@testing-library/jest-dom';
import { SignalInspector } from '../SignalInspector';
import * as signalsApi from '../../../api/signalsApi';
import type { CalculateSignalsResponse } from '../../../types/signals';

vi.mock('../../../api/signalsApi', () => ({
  calculateReplaySignals: vi.fn(),
}));

describe('SignalInspector', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    vi.mocked(signalsApi.calculateReplaySignals).mockResolvedValue({
      session_id: 1,
      observed_current_index: 0,
      timeframe: '1D',
      results: [],
    });
  });

  it('renders warmup state (INSUFFICIENT_HISTORY) correctly', async () => {
    vi.mocked(signalsApi.calculateReplaySignals).mockResolvedValue({
      session_id: 1,
      observed_current_index: 2,
      timeframe: '1D',
      results: [
        {
          signal_name: 'volume.spike',
          signal_version: '1.0.0',
          resolved_params: { period: 20, multiplier: 2.0 },
          params_hash: 'hash1234567890abcdef',
          points: [
            {
              bar_index: 2,
              timestamp: '2026-01-03',
              output_type: 'bool',
              value: null,
              quality: 'INSUFFICIENT_HISTORY',
              reasons: ['INSUFFICIENT_HISTORY'],
              baseline: null,
              current_volume: 500,
              relative_volume: null,
              threshold: 2.0,
              availability_event: 'BAR_CLOSE',
              available_at_index: 2,
              available_at_timestamp: '2026-01-03',
            },
          ],
        },
      ],
    });

    render(<SignalInspector sessionId={1} currentIndex={2} timeframe="1D" />);

    await waitFor(() => {
      expect(screen.getByTestId('signal-quality-badge')).toHaveTextContent('INSUFFICIENT_HISTORY');
      expect(screen.getByTestId('signal-spike-status')).toHaveTextContent('CHƯA ĐỦ DỮ LIỆU');
      expect(screen.getByTestId('signal-rvol-value')).toHaveTextContent('N/A');
      expect(screen.getByTestId('signal-reason')).toHaveTextContent('INSUFFICIENT_HISTORY');
    });
  });

  it('renders active spike true state correctly', async () => {
    vi.mocked(signalsApi.calculateReplaySignals).mockResolvedValue({
      session_id: 1,
      observed_current_index: 20,
      timeframe: '1D',
      results: [
        {
          signal_name: 'volume.spike',
          signal_version: '1.0.0',
          resolved_params: { period: 20, multiplier: 2.0 },
          params_hash: 'hash1234567890abcdef',
          points: [
            {
              bar_index: 20,
              timestamp: '2026-01-21',
              output_type: 'bool',
              value: true,
              quality: 'VALID',
              reasons: ['VOLUME_SPIKE'],
              baseline: 1000,
              current_volume: 2500,
              relative_volume: 2.5,
              threshold: 2.0,
              availability_event: 'BAR_CLOSE',
              available_at_index: 20,
              available_at_timestamp: '2026-01-21',
            },
          ],
        },
      ],
    });

    render(<SignalInspector sessionId={1} currentIndex={20} timeframe="1D" />);

    await waitFor(() => {
      expect(screen.getByTestId('signal-quality-badge')).toHaveTextContent('VALID');
      expect(screen.getByTestId('signal-spike-status')).toHaveTextContent('ĐỘT BIẾN');
      expect(screen.getByTestId('signal-rvol-value')).toHaveTextContent('2.50');
      expect(screen.getByTestId('signal-reason')).toHaveTextContent('VOLUME_SPIKE');
      expect(screen.getByTestId('signal-availability')).toHaveTextContent('Khớp: BAR_CLOSE');
    });
  });

  it('renders active spike false state correctly', async () => {
    vi.mocked(signalsApi.calculateReplaySignals).mockResolvedValue({
      session_id: 1,
      observed_current_index: 20,
      timeframe: '1D',
      results: [
        {
          signal_name: 'volume.spike',
          signal_version: '1.0.0',
          resolved_params: { period: 20, multiplier: 2.0 },
          params_hash: 'hash1234567890abcdef',
          points: [
            {
              bar_index: 20,
              timestamp: '2026-01-21',
              output_type: 'bool',
              value: false,
              quality: 'VALID',
              reasons: ['NORMAL_VOLUME'],
              baseline: 1000,
              current_volume: 1200,
              relative_volume: 1.2,
              threshold: 2.0,
              availability_event: 'BAR_CLOSE',
              available_at_index: 20,
              available_at_timestamp: '2026-01-21',
            },
          ],
        },
      ],
    });

    render(<SignalInspector sessionId={1} currentIndex={20} timeframe="1D" />);

    await waitFor(() => {
      expect(screen.getByTestId('signal-quality-badge')).toHaveTextContent('VALID');
      expect(screen.getByTestId('signal-spike-status')).toHaveTextContent('BÌNH THƯỜNG');
      expect(screen.getByTestId('signal-rvol-value')).toHaveTextContent('1.20');
      expect(screen.getByTestId('signal-reason')).toHaveTextContent('NORMAL_VOLUME');
    });
  });

  it('displays validation error when period or multiplier is out of bounds', async () => {
    render(<SignalInspector sessionId={1} currentIndex={5} timeframe="1D" />);
    vi.mocked(signalsApi.calculateReplaySignals).mockClear();

    const periodInput = screen.getByTestId('signal-period-input');
    fireEvent.change(periodInput, { target: { value: '0' } });

    await waitFor(() => {
      expect(screen.getByTestId('signal-validation-error')).toHaveTextContent(
        'Tham số chu kỳ (period) phải là số nguyên từ 1 đến 252.'
      );
    });
    expect(signalsApi.calculateReplaySignals).not.toHaveBeenCalled();
  });

  it('displays warning when timeframe is not 1D', async () => {
    render(<SignalInspector sessionId={1} currentIndex={5} timeframe="1H" />);

    expect(screen.getByTestId('signal-unsupported-timeframe')).toHaveTextContent(
      'Khung thời gian 1H chưa được hỗ trợ. Vui lòng chọn 1D.'
    );
    expect(signalsApi.calculateReplaySignals).not.toHaveBeenCalled();
  });

  it('displays API error when calculation endpoint fails', async () => {
    vi.mocked(signalsApi.calculateReplaySignals).mockRejectedValue({
      response: { data: { detail: 'Session expired or not found' } },
    });

    render(<SignalInspector sessionId={1} currentIndex={5} timeframe="1D" />);

    await waitFor(() => {
      expect(screen.getByTestId('signal-error')).toHaveTextContent('Lỗi: Session expired or not found');
    });
  });

  it('P1-OR-10: discards delayed stale response when index or config changes', async () => {
    let resolveFirstRequest: (value: CalculateSignalsResponse) => void = () => {};
    const firstPromise = new Promise<CalculateSignalsResponse>((resolve) => {
      resolveFirstRequest = resolve;
    });

    vi.mocked(signalsApi.calculateReplaySignals).mockImplementationOnce(() => firstPromise);

    const { rerender } = render(<SignalInspector sessionId={1} currentIndex={3} timeframe="1D" />);

    // First request is in flight
    expect(screen.getByTestId('signal-loading')).toBeInTheDocument();

    // Now user steps to index 4 before first request finishes
    vi.mocked(signalsApi.calculateReplaySignals).mockResolvedValueOnce({
      session_id: 1,
      observed_current_index: 4,
      timeframe: '1D',
      results: [
        {
          signal_name: 'volume.spike',
          signal_version: '1.0.0',
          resolved_params: { period: 20, multiplier: 2.0 },
          params_hash: 'hash_index_4',
          points: [
            {
              bar_index: 4,
              timestamp: '2026-01-05',
              output_type: 'bool',
              value: true,
              quality: 'VALID',
              reasons: ['VOLUME_SPIKE_BAR_4'],
              baseline: 1000,
              current_volume: 3000,
              relative_volume: 3.0,
              threshold: 2.0,
              availability_event: 'BAR_CLOSE',
              available_at_index: 4,
              available_at_timestamp: '2026-01-05',
            },
          ],
        },
      ],
    });

    rerender(<SignalInspector sessionId={1} currentIndex={4} timeframe="1D" />);

    // Index 4 resolves first
    await waitFor(() => {
      expect(screen.getByTestId('signal-reason')).toHaveTextContent('VOLUME_SPIKE_BAR_4');
    });

    // Now old request for index 3 finishes late
    resolveFirstRequest({
      session_id: 1,
      observed_current_index: 3,
      timeframe: '1D',
      results: [
        {
          signal_name: 'volume.spike',
          signal_version: '1.0.0',
          resolved_params: { period: 20, multiplier: 2.0 },
          params_hash: 'hash_stale_3',
          points: [
            {
              bar_index: 3,
              timestamp: '2026-01-04',
              output_type: 'bool',
              value: false,
              quality: 'VALID',
              reasons: ['OLD_STALE_BAR_3'],
              baseline: 1000,
              current_volume: 1000,
              relative_volume: 1.0,
              threshold: 2.0,
              availability_event: 'BAR_CLOSE',
              available_at_index: 3,
              available_at_timestamp: '2026-01-04',
            },
          ],
        },
      ],
    });

    // Verify index 4 is STILL displayed and the stale index 3 response was discarded!
    await waitFor(() => {
      expect(screen.getByTestId('signal-reason')).toHaveTextContent('VOLUME_SPIKE_BAR_4');
      expect(screen.queryByText('OLD_STALE_BAR_3')).not.toBeInTheDocument();
    });
  });

  it('ABA: old index-3 response arrives during the new index-3 request; it neither renders nor ends loading. Only the new response renders', async () => {
    let resolveOld3!: (value: CalculateSignalsResponse) => void;
    let resolveNew3!: (value: CalculateSignalsResponse) => void;
    const old3 = new Promise<CalculateSignalsResponse>((resolve) => {
      resolveOld3 = resolve;
    });
    const new3 = new Promise<CalculateSignalsResponse>((resolve) => {
      resolveNew3 = resolve;
    });

    vi.mocked(signalsApi.calculateReplaySignals)
      .mockImplementationOnce(() => old3)
      .mockResolvedValueOnce({
        session_id: 1,
        observed_current_index: 4,
        timeframe: '1D',
        results: [
          {
            signal_name: 'volume.spike',
            signal_version: '1.0.0',
            resolved_params: { period: 20, multiplier: 2.0 },
            params_hash: 'hash-4',
            points: [
              {
                bar_index: 4,
                timestamp: '2026-01-05',
                output_type: 'bool',
                value: true,
                quality: 'VALID',
                reasons: ['CURRENT_4'],
                baseline: 100,
                current_volume: 300,
                relative_volume: 3,
                threshold: 2.0,
                availability_event: 'BAR_CLOSE',
                available_at_index: 4,
                available_at_timestamp: '2026-01-05',
              },
            ],
          },
        ],
      })
      .mockImplementationOnce(() => new3);

    const { rerender } = render(<SignalInspector sessionId={1} currentIndex={3} timeframe="1D" />);
    rerender(<SignalInspector sessionId={1} currentIndex={4} timeframe="1D" />);
    await screen.findByText('CURRENT_4');

    rerender(<SignalInspector sessionId={1} currentIndex={3} timeframe="1D" />);
    // Verify loading is shown immediately upon returning to index 3
    expect(screen.getByTestId('signal-loading')).toBeInTheDocument();
    expect(screen.queryByText('CURRENT_4')).not.toBeInTheDocument();

    // Resolve old index 3 request
    resolveOld3({
      session_id: 1,
      observed_current_index: 3,
      timeframe: '1D',
      results: [
        {
          signal_name: 'volume.spike',
          signal_version: '1.0.0',
          resolved_params: { period: 20, multiplier: 2.0 },
          params_hash: 'hash-stale-3',
          points: [
            {
              bar_index: 3,
              timestamp: '2026-01-04',
              output_type: 'bool',
              value: true,
              quality: 'VALID',
              reasons: ['STALE_OLD_3'],
              baseline: 100,
              current_volume: 300,
              relative_volume: 3,
              threshold: 2.0,
              availability_event: 'BAR_CLOSE',
              available_at_index: 3,
              available_at_timestamp: '2026-01-04',
            },
          ],
        },
      ],
    });

    // Verify old response neither renders nor ends loading
    await waitFor(() => {
      expect(screen.queryByText('STALE_OLD_3')).not.toBeInTheDocument();
      expect(screen.getByTestId('signal-loading')).toBeInTheDocument();
    });

    // Now resolve new index 3 request
    resolveNew3({
      session_id: 1,
      observed_current_index: 3,
      timeframe: '1D',
      results: [
        {
          signal_name: 'volume.spike',
          signal_version: '1.0.0',
          resolved_params: { period: 20, multiplier: 2.0 },
          params_hash: 'hash-new-3',
          points: [
            {
              bar_index: 3,
              timestamp: '2026-01-04',
              output_type: 'bool',
              value: true,
              quality: 'VALID',
              reasons: ['CURRENT_NEW_3'],
              baseline: 100,
              current_volume: 300,
              relative_volume: 3,
              threshold: 2.0,
              availability_event: 'BAR_CLOSE',
              available_at_index: 3,
              available_at_timestamp: '2026-01-04',
            },
          ],
        },
      ],
    });

    // Only the new response renders
    await screen.findByText('CURRENT_NEW_3');
    expect(screen.queryByTestId('signal-loading')).not.toBeInTheDocument();
  });

  it('a late rejected promise cannot replace a newer success or show an error', async () => {
    let rejectOld3!: (err: unknown) => void;
    const old3 = new Promise<CalculateSignalsResponse>((_, reject) => {
      rejectOld3 = reject;
    });

    vi.mocked(signalsApi.calculateReplaySignals)
      .mockImplementationOnce(() => old3)
      .mockResolvedValueOnce({
        session_id: 1,
        observed_current_index: 4,
        timeframe: '1D',
        results: [
          {
            signal_name: 'volume.spike',
            signal_version: '1.0.0',
            resolved_params: { period: 20, multiplier: 2.0 },
            params_hash: 'hash-4',
            points: [
              {
                bar_index: 4,
                timestamp: '2026-01-05',
                output_type: 'bool',
                value: true,
                quality: 'VALID',
                reasons: ['SUCCESS_4'],
                baseline: 100,
                current_volume: 300,
                relative_volume: 3,
                threshold: 2.0,
                availability_event: 'BAR_CLOSE',
                available_at_index: 4,
                available_at_timestamp: '2026-01-05',
              },
            ],
          },
        ],
      });

    const { rerender } = render(<SignalInspector sessionId={1} currentIndex={3} timeframe="1D" />);
    rerender(<SignalInspector sessionId={1} currentIndex={4} timeframe="1D" />);
    await screen.findByText('SUCCESS_4');

    // Reject old 3 late
    rejectOld3({ response: { data: { detail: 'LATE_ERROR_OLD_3' } } });

    // Success 4 remains rendered, no error displayed
    await waitFor(() => {
      expect(screen.getByText('SUCCESS_4')).toBeInTheDocument();
      expect(screen.queryByTestId('signal-error')).not.toBeInTheDocument();
      expect(screen.queryByText(/LATE_ERROR_OLD_3/)).not.toBeInTheDocument();
    });
  });

  it('completed A -> pending B -> pending new A: completed A is hidden and loading remains until new A resolves', async () => {
    let resolveB!: (value: CalculateSignalsResponse) => void;
    let resolveNewA!: (value: CalculateSignalsResponse) => void;

    const promiseB = new Promise<CalculateSignalsResponse>((resolve) => {
      resolveB = resolve;
    });
    const promiseNewA = new Promise<CalculateSignalsResponse>((resolve) => {
      resolveNewA = resolve;
    });

    const makeResponse = (idx: number, reason: string): CalculateSignalsResponse => ({
      session_id: 1,
      observed_current_index: idx,
      timeframe: '1D',
      results: [
        {
          signal_name: 'volume.spike',
          signal_version: '1.0.0',
          resolved_params: { period: 20, multiplier: 2.0 },
          params_hash: `hash_${idx}`,
          points: [
            {
              bar_index: idx,
              timestamp: `2026-01-0${idx + 1}`,
              output_type: 'bool',
              value: true,
              quality: 'VALID',
              reasons: [reason],
              baseline: 100,
              current_volume: 300,
              relative_volume: 3.0,
              threshold: 2.0,
              availability_event: 'BAR_CLOSE',
              available_at_index: idx,
              available_at_timestamp: `2026-01-0${idx + 1}`,
            },
          ],
        },
      ],
    });

    // 1. Initial request for A (index 3) resolves
    vi.mocked(signalsApi.calculateReplaySignals).mockResolvedValueOnce(
      makeResponse(3, 'COMPLETED_OLD_A')
    );

    const { rerender } = render(<SignalInspector sessionId={1} currentIndex={3} timeframe="1D" />);
    await screen.findByText('COMPLETED_OLD_A');

    // 2. Navigate to B (index 4) which is pending
    vi.mocked(signalsApi.calculateReplaySignals).mockImplementationOnce(() => promiseB);
    rerender(<SignalInspector sessionId={1} currentIndex={4} timeframe="1D" />);

    // Completed A is immediately hidden, loading is shown
    expect(screen.queryByText('COMPLETED_OLD_A')).not.toBeInTheDocument();
    expect(screen.getByTestId('signal-loading')).toBeInTheDocument();

    // 3. Navigate back to new A (index 3) which is also pending
    vi.mocked(signalsApi.calculateReplaySignals).mockImplementationOnce(() => promiseNewA);
    rerender(<SignalInspector sessionId={1} currentIndex={3} timeframe="1D" />);

    // Completed A is STILL hidden, and loading remains!
    expect(screen.queryByText('COMPLETED_OLD_A')).not.toBeInTheDocument();
    expect(screen.getByTestId('signal-loading')).toBeInTheDocument();

    // 4. Resolve new A
    resolveNewA(makeResponse(3, 'COMPLETED_NEW_A'));

    // New A resolves and renders
    await screen.findByText('COMPLETED_NEW_A');
    expect(screen.queryByTestId('signal-loading')).not.toBeInTheDocument();

    // Resolve B late just to clean up promise
    resolveB(makeResponse(4, 'CLEANUP_B'));
  });

  it('multiplier 2 -> 2.0000005: old result is hidden; a response resolved as 2 is rejected; exact 2.0000005 is accepted', async () => {
    const makeResponse = (
      multiplier: number,
      reason: string,
      period = 20,
      index = 5
    ): CalculateSignalsResponse => ({
      session_id: 1,
      observed_current_index: index,
      timeframe: '1D',
      results: [
        {
          signal_name: 'volume.spike',
          signal_version: '1.0.0',
          resolved_params: { period, multiplier },
          params_hash: `hash_${multiplier}`,
          points: [
            {
              bar_index: index,
              timestamp: '2026-01-06',
              output_type: 'bool',
              value: true,
              quality: 'VALID',
              reasons: [reason],
              baseline: 100,
              current_volume: 300,
              relative_volume: 3.0,
              threshold: multiplier,
              availability_event: 'BAR_CLOSE',
              available_at_index: index,
              available_at_timestamp: '2026-01-06',
            },
          ],
        },
      ],
    });

    // 1. Initial multiplier 2 resolves
    vi.mocked(signalsApi.calculateReplaySignals).mockResolvedValueOnce(
      makeResponse(2, 'MULTIPLIER_2_RESULT')
    );

    render(<SignalInspector sessionId={1} currentIndex={5} timeframe="1D" />);
    await screen.findByText('MULTIPLIER_2_RESULT');

    // 2. Change multiplier to 2.0000005, server will return response resolved as 2 (which must be rejected)
    let resolveResponseAs2!: (value: CalculateSignalsResponse) => void;
    const pendingPromise = new Promise<CalculateSignalsResponse>((resolve) => {
      resolveResponseAs2 = resolve;
    });
    vi.mocked(signalsApi.calculateReplaySignals).mockImplementationOnce(() => pendingPromise);

    const multiplierInput = screen.getByTestId('signal-multiplier-input');
    fireEvent.change(multiplierInput, { target: { value: '2.0000005' } });

    // Old result is immediately hidden; loading starts
    expect(screen.queryByText('MULTIPLIER_2_RESULT')).not.toBeInTheDocument();
    expect(screen.getByTestId('signal-loading')).toBeInTheDocument();

    // Server returns response resolved as 2
    resolveResponseAs2(makeResponse(2, 'INCORRECT_RESOLVED_2'));

    // Response resolved as 2 is rejected with controlled error; no result rendered
    await screen.findByTestId('signal-error');
    expect(screen.queryByText('INCORRECT_RESOLVED_2')).not.toBeInTheDocument();
    expect(screen.queryByTestId('signal-results')).not.toBeInTheDocument();

    // 3. Now server returns exact 2.0000005
    vi.mocked(signalsApi.calculateReplaySignals).mockResolvedValueOnce(
      makeResponse(2.0000005, 'EXACT_2_0000005_ACCEPTED', 21)
    );

    // Trigger new request with multiplier 2.0000005 by changing period to 21
    const periodInput = screen.getByTestId('signal-period-input');
    fireEvent.change(periodInput, { target: { value: '21' } });

    // Exact response is accepted and rendered
    await screen.findByText('EXACT_2_0000005_ACCEPTED');
    expect(screen.queryByTestId('signal-error')).not.toBeInTheDocument();
    expect(screen.getByTestId('signal-threshold-value')).toHaveTextContent('2.0000005x');
  });

  it('prior A error -> B -> new A: prior error is hidden and new A shows loading', async () => {
    let resolveNewA!: (value: CalculateSignalsResponse) => void;
    const promiseNewA = new Promise<CalculateSignalsResponse>((resolve) => {
      resolveNewA = resolve;
    });

    // 1. Initial render A (index 3) fails with error
    vi.mocked(signalsApi.calculateReplaySignals).mockRejectedValueOnce({
      response: { data: { detail: 'ERROR_AT_A' } },
    });

    const { rerender } = render(<SignalInspector sessionId={1} currentIndex={3} timeframe="1D" />);
    await screen.findByText(/ERROR_AT_A/);

    // 2. Move to B (index 4) - pending
    let resolveB!: (value: CalculateSignalsResponse) => void;
    const promiseB = new Promise<CalculateSignalsResponse>((resolve) => {
      resolveB = resolve;
    });
    vi.mocked(signalsApi.calculateReplaySignals).mockImplementationOnce(() => promiseB);

    rerender(<SignalInspector sessionId={1} currentIndex={4} timeframe="1D" />);

    // Prior error is hidden, B shows loading
    expect(screen.queryByText(/ERROR_AT_A/)).not.toBeInTheDocument();
    expect(screen.getByTestId('signal-loading')).toBeInTheDocument();

    // 3. Move to new A (index 3) - pending
    vi.mocked(signalsApi.calculateReplaySignals).mockImplementationOnce(() => promiseNewA);
    rerender(<SignalInspector sessionId={1} currentIndex={3} timeframe="1D" />);

    // Prior error is STILL hidden, new A shows loading!
    expect(screen.queryByText(/ERROR_AT_A/)).not.toBeInTheDocument();
    expect(screen.getByTestId('signal-loading')).toBeInTheDocument();

    // Clean up promises
    resolveB({
      session_id: 1,
      observed_current_index: 4,
      timeframe: '1D',
      results: [],
    });
    resolveNewA({
      session_id: 1,
      observed_current_index: 3,
      timeframe: '1D',
      results: [],
    });
  });

  it('mismatched session/index/timeframe/version/resolved params, missing or non-unique exact point, future point, bad availability, empty timestamp, and mismatched timestamp are rejected with controlled error and no result', async () => {
    const validPoint = (index: number, timestamp = `2026-01-0${index + 1}`) => ({
      bar_index: index,
      timestamp,
      output_type: 'bool' as const,
      value: true,
      quality: 'VALID' as const,
      reasons: [`POINT_${index}`],
      baseline: 100,
      current_volume: 300,
      relative_volume: 3.0,
      threshold: 2.0,
      availability_event: 'BAR_CLOSE',
      available_at_index: index,
      available_at_timestamp: timestamp,
    });

    // Helper to render and test that an invalid response payload produces a controlled error and no results
    const assertRejectedResponse = async (
      makeInvalidResponse: (index: number) => CalculateSignalsResponse,
      props?: { currentTimestamp?: string }
    ) => {
      vi.mocked(signalsApi.calculateReplaySignals).mockResolvedValueOnce(makeInvalidResponse(5));
      const { unmount } = render(
        <SignalInspector sessionId={1} currentIndex={5} timeframe="1D" {...props} />
      );
      // Every mismatched-response case must await the controlled error before asserting no result;
      // no test may pass from the initial empty render.
      await screen.findByTestId('signal-error');
      expect(screen.queryByTestId('signal-results')).not.toBeInTheDocument();
      expect(screen.queryByTestId('signal-loading')).not.toBeInTheDocument();
      unmount();
    };

    // 1. Mismatched session ID
    await assertRejectedResponse((index) => ({
      session_id: 999, // mismatch (requested 1)
      observed_current_index: index,
      timeframe: '1D',
      results: [
        {
          signal_name: 'volume.spike',
          signal_version: '1.0.0',
          resolved_params: { period: 20, multiplier: 2.0 },
          params_hash: 'hash-test',
          points: [validPoint(index)],
        },
      ],
    }));

    // 2. Mismatched observed current index
    await assertRejectedResponse(() => ({
      session_id: 1,
      observed_current_index: 4, // mismatch (requested 5)
      timeframe: '1D',
      results: [
        {
          signal_name: 'volume.spike',
          signal_version: '1.0.0',
          resolved_params: { period: 20, multiplier: 2.0 },
          params_hash: 'hash-test',
          points: [validPoint(5)],
        },
      ],
    }));

    // 3. Mismatched timeframe
    await assertRejectedResponse((index) => ({
      session_id: 1,
      observed_current_index: index,
      timeframe: '1H', // mismatch (requested 1D)
      results: [
        {
          signal_name: 'volume.spike',
          signal_version: '1.0.0',
          resolved_params: { period: 20, multiplier: 2.0 },
          params_hash: 'hash-test',
          points: [validPoint(index)],
        },
      ],
    }));

    // 4. Mismatched signal version
    await assertRejectedResponse((index) => ({
      session_id: 1,
      observed_current_index: index,
      timeframe: '1D',
      results: [
        {
          signal_name: 'volume.spike',
          signal_version: '2.0.0', // mismatch (expected 1.0.0)
          resolved_params: { period: 20, multiplier: 2.0 },
          params_hash: 'hash-test',
          points: [validPoint(index)],
        },
      ],
    }));

    // 5. Mismatched resolved params (period mismatch)
    await assertRejectedResponse((index) => ({
      session_id: 1,
      observed_current_index: index,
      timeframe: '1D',
      results: [
        {
          signal_name: 'volume.spike',
          signal_version: '1.0.0',
          resolved_params: { period: 30, multiplier: 2.0 }, // mismatch (expected 20)
          params_hash: 'hash-test',
          points: [validPoint(index)],
        },
      ],
    }));

    // 6. Mismatched resolved params (multiplier mismatch)
    await assertRejectedResponse((index) => ({
      session_id: 1,
      observed_current_index: index,
      timeframe: '1D',
      results: [
        {
          signal_name: 'volume.spike',
          signal_version: '1.0.0',
          resolved_params: { period: 20, multiplier: 3.5 }, // mismatch (expected 2.0)
          params_hash: 'hash-test',
          points: [validPoint(index)],
        },
      ],
    }));

    // 7. Missing exact point for requested index
    await assertRejectedResponse((index) => ({
      session_id: 1,
      observed_current_index: index,
      timeframe: '1D',
      results: [
        {
          signal_name: 'volume.spike',
          signal_version: '1.0.0',
          resolved_params: { period: 20, multiplier: 2.0 },
          params_hash: 'hash-test',
          points: [validPoint(index - 1)], // index 4 instead of 5
        },
      ],
    }));

    // 8. Non-unique exact point (duplicate points for index 5)
    await assertRejectedResponse((index) => ({
      session_id: 1,
      observed_current_index: index,
      timeframe: '1D',
      results: [
        {
          signal_name: 'volume.spike',
          signal_version: '1.0.0',
          resolved_params: { period: 20, multiplier: 2.0 },
          params_hash: 'hash-test',
          points: [validPoint(index), validPoint(index)], // duplicate!
        },
      ],
    }));

    // 9. Future points present (points contains index 6 when requested 5)
    await assertRejectedResponse((index) => ({
      session_id: 1,
      observed_current_index: index,
      timeframe: '1D',
      results: [
        {
          signal_name: 'volume.spike',
          signal_version: '1.0.0',
          resolved_params: { period: 20, multiplier: 2.0 },
          params_hash: 'hash-test',
          points: [validPoint(index), validPoint(index + 1)], // future point index 6!
        },
      ],
    }));

    // 10. Bad availability event (not BAR_CLOSE)
    await assertRejectedResponse((index) => ({
      session_id: 1,
      observed_current_index: index,
      timeframe: '1D',
      results: [
        {
          signal_name: 'volume.spike',
          signal_version: '1.0.0',
          resolved_params: { period: 20, multiplier: 2.0 },
          params_hash: 'hash-test',
          points: [
            {
              ...validPoint(index),
              availability_event: 'TICK', // bad event!
            },
          ],
        },
      ],
    }));

    // 11. Bad availability index
    await assertRejectedResponse((index) => ({
      session_id: 1,
      observed_current_index: index,
      timeframe: '1D',
      results: [
        {
          signal_name: 'volume.spike',
          signal_version: '1.0.0',
          resolved_params: { period: 20, multiplier: 2.0 },
          params_hash: 'hash-test',
          points: [
            {
              ...validPoint(index),
              available_at_index: index - 1, // bad availability index!
            },
          ],
        },
      ],
    }));

    // 12. Empty point timestamp when currentTimestamp is supplied
    await assertRejectedResponse(
      (index) => ({
        session_id: 1,
        observed_current_index: index,
        timeframe: '1D',
        results: [
          {
            signal_name: 'volume.spike',
            signal_version: '1.0.0',
            resolved_params: { period: 20, multiplier: 2.0 },
            params_hash: 'hash-test',
            points: [validPoint(index, '')], // empty point timestamp!
          },
        ],
      }),
      { currentTimestamp: '2026-01-06' }
    );

    // 13. Mismatched point timestamp when currentTimestamp is supplied
    await assertRejectedResponse(
      (index) => ({
        session_id: 1,
        observed_current_index: index,
        timeframe: '1D',
        results: [
          {
            signal_name: 'volume.spike',
            signal_version: '1.0.0',
            resolved_params: { period: 20, multiplier: 2.0 },
            params_hash: 'hash-test',
            points: [validPoint(index, '2026-01-06')], // point timestamp is 06
          },
        ],
      }),
      { currentTimestamp: '2026-01-07' } // requested is 07!
    );

    // 14. Empty available_at_timestamp when currentTimestamp is supplied
    await assertRejectedResponse(
      (index) => ({
        session_id: 1,
        observed_current_index: index,
        timeframe: '1D',
        results: [
          {
            signal_name: 'volume.spike',
            signal_version: '1.0.0',
            resolved_params: { period: 20, multiplier: 2.0 },
            params_hash: 'hash-test',
            points: [
              {
                ...validPoint(index, '2026-01-06'),
                available_at_timestamp: '', // empty available_at_timestamp!
              },
            ],
          },
        ],
      }),
      { currentTimestamp: '2026-01-06' }
    );

    // 15. Mismatched available_at_timestamp when currentTimestamp is supplied
    await assertRejectedResponse(
      (index) => ({
        session_id: 1,
        observed_current_index: index,
        timeframe: '1D',
        results: [
          {
            signal_name: 'volume.spike',
            signal_version: '1.0.0',
            resolved_params: { period: 20, multiplier: 2.0 },
            params_hash: 'hash-test',
            points: [
              {
                ...validPoint(index, '2026-01-06'),
                available_at_timestamp: '2026-01-07', // mismatch!
              },
            ],
          },
        ],
      }),
      { currentTimestamp: '2026-01-06' }
    );

    // 16. Reject point.timestamp="2026-01-06Tnot-a-time" even though its date prefix matches
    await assertRejectedResponse(
      (index) => ({
        session_id: 1,
        observed_current_index: index,
        timeframe: '1D',
        results: [
          {
            signal_name: 'volume.spike',
            signal_version: '1.0.0',
            resolved_params: { period: 20, multiplier: 2.0 },
            params_hash: 'hash-test',
            points: [validPoint(index, '2026-01-06Tnot-a-time')],
          },
        ],
      }),
      { currentTimestamp: '2026-01-06' }
    );

    // 17. Reject malformed available_at_timestamp="2026-01-06Tnot-a-time"
    await assertRejectedResponse(
      (index) => ({
        session_id: 1,
        observed_current_index: index,
        timeframe: '1D',
        results: [
          {
            signal_name: 'volume.spike',
            signal_version: '1.0.0',
            resolved_params: { period: 20, multiplier: 2.0 },
            params_hash: 'hash-test',
            points: [
              {
                ...validPoint(index, '2026-01-06'),
                available_at_timestamp: '2026-01-06Tnot-a-time',
              },
            ],
          },
        ],
      }),
      { currentTimestamp: '2026-01-06' }
    );

    // 18. Reject impossible calendar date 2026-02-30 in point.timestamp
    await assertRejectedResponse(
      (index) => ({
        session_id: 1,
        observed_current_index: index,
        timeframe: '1D',
        results: [
          {
            signal_name: 'volume.spike',
            signal_version: '1.0.0',
            resolved_params: { period: 20, multiplier: 2.0 },
            params_hash: 'hash-test',
            points: [validPoint(index, '2026-02-30')],
          },
        ],
      }),
      { currentTimestamp: '2026-01-06' }
    );

    // 19. Reject impossible calendar date 2026-02-30 in available_at_timestamp
    await assertRejectedResponse(
      (index) => ({
        session_id: 1,
        observed_current_index: index,
        timeframe: '1D',
        results: [
          {
            signal_name: 'volume.spike',
            signal_version: '1.0.0',
            resolved_params: { period: 20, multiplier: 2.0 },
            params_hash: 'hash-test',
            points: [
              {
                ...validPoint(index, '2026-01-06'),
                available_at_timestamp: '2026-02-30',
              },
            ],
          },
        ],
      }),
      { currentTimestamp: '2026-01-06' }
    );
  });

  it('returned period/threshold/hash are rendered from the accepted server response; no last-point fallback', async () => {
    vi.mocked(signalsApi.calculateReplaySignals).mockResolvedValueOnce({
      session_id: 1,
      observed_current_index: 10,
      timeframe: '1D',
      results: [
        {
          signal_name: 'volume.spike',
          signal_version: '1.0.0',
          resolved_params: { period: 20, multiplier: 2.0 },
          params_hash: 'canonical_hash_evidence_xyz123',
          points: [
            {
              bar_index: 10,
              timestamp: '2026-01-11',
              output_type: 'bool',
              value: true,
              quality: 'VALID',
              reasons: ['SPIKE_EXACT'],
              baseline: 400,
              current_volume: 1200,
              relative_volume: 3.0,
              threshold: 2.0,
              availability_event: 'BAR_CLOSE',
              available_at_index: 10,
              available_at_timestamp: '2026-01-11',
            },
          ],
        },
      ],
    });

    render(<SignalInspector sessionId={1} currentIndex={10} timeframe="1D" />);

    await waitFor(() => {
      // Threshold rendered from accepted point.threshold
      expect(screen.getByTestId('signal-threshold-value')).toHaveTextContent('2x');
      // Resolved period rendered from accepted resolved_params.period
      expect(screen.getByText(/20 phiên/)).toBeInTheDocument();
      // Hash rendered from accepted params_hash
      expect(screen.getByTestId('signal-params-hash')).toHaveTextContent(
        'canonical_hash_evidence_xyz123'
      );
    });
  });

  it('period 1.5 and multiplier 0 do not call the API; multiplier 0.001 does', async () => {
    render(<SignalInspector sessionId={1} currentIndex={5} timeframe="1D" />);
    vi.mocked(signalsApi.calculateReplaySignals).mockClear();

    const periodInput = screen.getByTestId('signal-period-input');
    const multiplierInput = screen.getByTestId('signal-multiplier-input');

    // Period 1.5 is not an integer -> must not call API
    fireEvent.change(periodInput, { target: { value: '1.5' } });
    await waitFor(() => {
      expect(screen.getByTestId('signal-validation-error')).toHaveTextContent(
        'Tham số chu kỳ (period) phải là số nguyên từ 1 đến 252.'
      );
    });
    expect(signalsApi.calculateReplaySignals).not.toHaveBeenCalled();

    // Restore period to valid integer
    fireEvent.change(periodInput, { target: { value: '20' } });
    vi.mocked(signalsApi.calculateReplaySignals).mockClear();

    // Multiplier 0 is not > 0 -> must not call API
    fireEvent.change(multiplierInput, { target: { value: '0' } });
    await waitFor(() => {
      expect(screen.getByTestId('signal-validation-error')).toHaveTextContent(
        'Hệ số nhân (multiplier) phải lớn hơn 0 và nhỏ hơn hoặc bằng 100.'
      );
    });
    expect(signalsApi.calculateReplaySignals).not.toHaveBeenCalled();

    // Multiplier 0.001 is a valid positive float -> MUST call API
    fireEvent.change(multiplierInput, { target: { value: '0.001' } });
    await waitFor(() => {
      expect(screen.queryByTestId('signal-validation-error')).not.toBeInTheDocument();
    });
    expect(signalsApi.calculateReplaySignals).toHaveBeenCalledWith(1, {
      signals: [
        {
          name: 'volume.spike',
          version: '1.0.0',
          params: { period: 20, multiplier: 0.001 },
        },
      ],
    });
  });

  it('defined whitespace/malformed currentTimestamp: controlled validation error, zero API calls, no loading/result', async () => {
    vi.mocked(signalsApi.calculateReplaySignals).mockClear();

    // 1. Whitespace-only currentTimestamp
    const { rerender } = render(
      <SignalInspector sessionId={1} currentIndex={5} timeframe="1D" currentTimestamp="   " />
    );

    expect(screen.getByTestId('signal-validation-error')).toHaveTextContent(
      'Dấu thời gian nến yêu cầu không hợp lệ hoặc không đúng định dạng.'
    );
    expect(screen.queryByTestId('signal-loading')).not.toBeInTheDocument();
    expect(screen.queryByTestId('signal-results')).not.toBeInTheDocument();
    expect(signalsApi.calculateReplaySignals).not.toHaveBeenCalled();

    // 2. Empty string currentTimestamp
    rerender(<SignalInspector sessionId={1} currentIndex={5} timeframe="1D" currentTimestamp="" />);
    expect(screen.getByTestId('signal-validation-error')).toBeInTheDocument();
    expect(screen.queryByTestId('signal-loading')).not.toBeInTheDocument();
    expect(screen.queryByTestId('signal-results')).not.toBeInTheDocument();
    expect(signalsApi.calculateReplaySignals).not.toHaveBeenCalled();

    // 3. Malformed time suffix in currentTimestamp
    rerender(
      <SignalInspector
        sessionId={1}
        currentIndex={5}
        timeframe="1D"
        currentTimestamp="2026-01-06Tnot-a-time"
      />
    );
    expect(screen.getByTestId('signal-validation-error')).toBeInTheDocument();
    expect(screen.queryByTestId('signal-loading')).not.toBeInTheDocument();
    expect(screen.queryByTestId('signal-results')).not.toBeInTheDocument();
    expect(signalsApi.calculateReplaySignals).not.toHaveBeenCalled();

    // 4. Impossible calendar date in currentTimestamp
    rerender(
      <SignalInspector
        sessionId={1}
        currentIndex={5}
        timeframe="1D"
        currentTimestamp="2026-02-30"
      />
    );
    expect(screen.getByTestId('signal-validation-error')).toBeInTheDocument();
    expect(screen.queryByTestId('signal-loading')).not.toBeInTheDocument();
    expect(screen.queryByTestId('signal-results')).not.toBeInTheDocument();
    expect(signalsApi.calculateReplaySignals).not.toHaveBeenCalled();
  });

  it('accept and match valid 2026-01-06, 2026-01-06T15:30:00Z, valid offset ISO timestamp, and 2026-01-06 15:30:00', async () => {
    const validTimestamps = [
      '2026-01-06',
      '2026-01-06T15:30:00Z',
      '2026-01-06T15:30:00+07:00',
      '2026-01-06 15:30:00',
    ];

    for (const ts of validTimestamps) {
      vi.mocked(signalsApi.calculateReplaySignals).mockReset();
      vi.mocked(signalsApi.calculateReplaySignals).mockResolvedValueOnce({
        session_id: 1,
        observed_current_index: 5,
        timeframe: '1D',
        results: [
          {
            signal_name: 'volume.spike',
            signal_version: '1.0.0',
            resolved_params: { period: 20, multiplier: 2.0 },
            params_hash: `hash_${ts}`,
            points: [
              {
                bar_index: 5,
                timestamp: ts,
                output_type: 'bool',
                value: true,
                quality: 'VALID',
                reasons: ['VALID_TIMESTAMP_MATCH'],
                baseline: 100,
                current_volume: 300,
                relative_volume: 3.0,
                threshold: 2.0,
                availability_event: 'BAR_CLOSE',
                available_at_index: 5,
                available_at_timestamp: ts,
              },
            ],
          },
        ],
      });

      const { unmount } = render(
        <SignalInspector
          sessionId={1}
          currentIndex={5}
          timeframe="1D"
          currentTimestamp="2026-01-06"
        />
      );

      await screen.findByText('VALID_TIMESTAMP_MATCH');
      expect(screen.queryByTestId('signal-error')).not.toBeInTheDocument();
      expect(screen.queryByTestId('signal-validation-error')).not.toBeInTheDocument();
      unmount();
    }
  });
});
