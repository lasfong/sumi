import { describe, it, expect } from 'vitest';
import {
  extractApiErrorMessage,
  formatValidationDetail,
  normalizeApiErrorPayload,
} from '../client';
import {
  DEFAULT_SIGNAL_VERSION,
  validateAndNormalizeSignalRequest,
} from '../signalsApi';
import { normalizeBBSeriesEnvelope } from '../bbApi';

describe('B40 & B41 Frontend API Contract Alignment', () => {
  it('normalizes FastAPI 422 validation detail arrays into readable string without [object Object] (GAP-API-09)', () => {
    const rawFastApi422 = {
      detail: [
        {
          loc: ['body', 'signals'],
          msg: 'List should have at most 2 items after validation, not 4',
          type: 'too_long',
        },
        {
          loc: ['body', 'symbol'],
          msg: 'Field required',
          type: 'missing',
        },
      ],
    };

    const formatted = formatValidationDetail(rawFastApi422.detail);
    expect(formatted.message).toContain('signals: List should have at most 2 items');
    expect(formatted.message).toContain('symbol: Field required');
    expect(formatted.message).not.toContain('[object Object]');
    expect(formatted.fieldErrors).toHaveLength(2);

    const mockAxiosError = {
      message: 'Request failed with status code 422',
      response: {
        status: 422,
        data: rawFastApi422,
      },
    };
    const extracted = extractApiErrorMessage(mockAxiosError);
    expect(extracted).toContain('signals: List should have at most 2 items');
    expect(extracted).not.toContain('[object Object]');
    expect(typeof (rawFastApi422 as Record<string, unknown>).detail).toBe('string');
  });

  it('normalizes structured object detail (e.g. SettlementLockDetail) without losing detail_data (GAP-API-09)', () => {
    const rawSettlementError = {
      detail: {
        error_code: 'LOT_SETTLEMENT_LOCKED',
        status: 'REJECTED_SETTLEMENT_LOCK',
        message: 'Cannot sell: T+2 constraint. Shares locked under T+1 · về sau 1 phiên.',
        available_qty: 0,
        blocked_qty: 200,
      },
    };

    const envelope = normalizeApiErrorPayload(rawSettlementError);
    expect(envelope.code).toBe('LOT_SETTLEMENT_LOCKED');
    expect(envelope.message).toContain('Cannot sell: T+2 constraint');
    expect(envelope.detail).toContain('Cannot sell: T+2 constraint');
    expect(envelope.detail_data?.blocked_qty).toBe(200);
  });

  it('enforces client-side signal calculation contract invariants (GAP-API-07, GAP-FE-08)', () => {
    // Valid 2-signal request pins default version and empty params
    const valid = validateAndNormalizeSignalRequest({
      signals: [{ name: 'volume.spike', params: { period: 20 } }, { name: 'health.score' }],
    });
    expect(valid.signals).toHaveLength(2);
    expect(valid.signals[0].version).toBe(DEFAULT_SIGNAL_VERSION);
    expect(valid.signals[1].version).toBe(DEFAULT_SIGNAL_VERSION);
    expect(valid.signals[1].params).toEqual({});

    // Rejects > 2 signals
    expect(() =>
      validateAndNormalizeSignalRequest({
        signals: [
          { name: 'volume.spike' },
          { name: 'health.score' },
          { name: 'bb.direction_rising' },
        ],
      })
    ).toThrow(/between 1 and 2 signals/i);

    // Rejects duplicate signal names
    expect(() =>
      validateAndNormalizeSignalRequest({
        signals: [{ name: 'health.score' }, { name: 'health.score' }],
      })
    ).toThrow(/Duplicate signal name/i);

    // Rejects null params
    expect(() =>
      validateAndNormalizeSignalRequest({
        signals: [{ name: 'volume.spike', params: null as unknown as Record<string, number> }],
      })
    ).toThrow(/params cannot be null/i);
  });

  it('unifies BB chart-mode (data[]) and series-mode (points[]) envelopes (GAP-API-02)', () => {
    const chartEnvelope = normalizeBBSeriesEnvelope({
      symbol: 'FPT',
      flow_method: 'OHLCV_PROXY',
      points: [],
      data: [
        {
          time: '2026-03-01',
          pressure: 0.5,
          trading_value: 10000000,
          value_source: 'CLOSE_X_VOLUME',
          quality: 'HIGH',
          t20: 61.2,
          t20_dir: 'RISING',
          t20_regime: 'POSITIVE',
          t20_run: 4,
        },
      ],
    });

    expect(chartEnvelope.points).toHaveLength(1);
    expect(chartEnvelope.points[0].date).toBe('2026-03-01');
    expect(chartEnvelope.points[0].horizons?.T20?.bb_value).toBe(61.2);
    expect(chartEnvelope.points[0].horizons?.T20?.regime).toBe('POSITIVE');
  });
});
