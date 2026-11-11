import { apiClient } from './client';

export interface BBHorizonInfo {
  id: string;
  lookback_bars: number;
}

export interface BBHorizonsResponse {
  horizons: BBHorizonInfo[];
  default_horizons: string[];
  methodology: string;
}

export interface BBHorizonPointData {
  horizon?: string;
  bb_value?: number | null;
  oib_raw?: number | null;
  raw_numerator?: number | null;
  raw_denominator?: number | null;
  is_warmup?: boolean;
  direction?: string;
  regime?: string;
  regime_run_length?: number;
  value_source?: string;
  quality?: string;
}

export interface BBPoint {
  bar_index?: number;
  date: string;
  close?: number;
  volume?: number;
  value?: number;
  lower?: number;
  upper?: number;
  basis?: number;
  bandwidth?: number;
  percent_b?: number;
  regime?: string;
  data_quality?: string;
  is_valid?: boolean;
  horizons?: Record<string, BBHorizonPointData>;
  daily_pressure?: number;
  daily_trading_value?: number;
  daily_value_source?: string;
  daily_quality?: string;
}

export interface BBChartPoint {
  time: string;
  pressure: number;
  trading_value: number;
  value_source: string;
  quality: string;
  [horizonKey: string]: string | number | null | undefined;
}

export interface BBSymbolSeriesResponse {
  symbol: string;
  flow_method?: string;
  methodology_version?: string;
  as_of?: string | null;
  requested_horizons?: string[];
  total_bars?: number;
  coverage_ratio?: number;
  points: BBPoint[];
  /** Legacy / chart-mode compatibility fields */
  timeframe?: string;
  methodology?: string;
  calculation_as_of?: string;
  summary?: Record<string, unknown>;
  label?: string;
  data?: BBChartPoint[];
  publication_status?: 'CANONICAL_PUBLISHED' | 'RESEARCH_RETROSPECTIVE' | 'UNAVAILABLE_DEGRADED';
  gate_reasons?: string[];
}

export interface MarketBBSeriesResponse {
  universe_id: string;
  universe_version: string;
  universe_mode: string;
  is_point_in_time: boolean;
  publication_status: 'CANONICAL_PUBLISHED' | 'RESEARCH_RETROSPECTIVE' | 'UNAVAILABLE_DEGRADED';
  flow_method: string;
  methodology_version: string;
  as_of?: string | null;
  survivor_bias_caveat?: string | null;
  gate_reasons: string[];
  total_sessions: number;
  points: Array<Record<string, unknown>>;
}

/**
 * Normalizes a raw BB response (either canonical series DTO or chart adapter DTO)
 * into a unified BBSymbolSeriesResponse with populated points[].
 */
export function normalizeBBSeriesEnvelope(raw: BBSymbolSeriesResponse): BBSymbolSeriesResponse {
  if (!raw || typeof raw !== 'object') {
    return {
      symbol: '',
      flow_method: 'OHLCV_PROXY',
      methodology_version: 'bb_v1_ohlcv_proxy',
      points: [],
    };
  }

  // If chart-mode response (`data` array instead of `points` array), adapt into `points`
  if ((!Array.isArray(raw.points) || raw.points.length === 0) && Array.isArray(raw.data) && raw.data.length > 0) {
    const convertedPoints: BBPoint[] = raw.data.map((rec) => {
      const horizons: Record<string, BBHorizonPointData> = {};
      for (const h of ['T03', 'T05', 'T10', 'T20', 'T50', 'T200']) {
        const key = h.toLowerCase();
        if (key in rec) {
          const val = typeof rec[key] === 'number' ? (rec[key] as number) : null;
          horizons[h] = {
            horizon: h,
            bb_value: val,
            is_warmup: val === null,
            direction: typeof rec[`${key}_dir`] === 'string' ? (rec[`${key}_dir`] as string) : 'UNKNOWN',
            regime: typeof rec[`${key}_regime`] === 'string' ? (rec[`${key}_regime`] as string) : 'UNKNOWN',
            regime_run_length: typeof rec[`${key}_run`] === 'number' ? (rec[`${key}_run`] as number) : 0,
            value_source: rec.value_source,
            quality: rec.quality,
          };
        }
      }
      return {
        date: rec.time,
        horizons,
        daily_pressure: rec.pressure,
        daily_trading_value: rec.trading_value,
        daily_value_source: rec.value_source,
        daily_quality: rec.quality,
      };
    });

    return {
      ...raw,
      flow_method: raw.flow_method || raw.methodology || 'OHLCV_PROXY',
      points: convertedPoints,
    };
  }

  return {
    ...raw,
    flow_method: raw.flow_method || raw.methodology || 'OHLCV_PROXY',
    points: Array.isArray(raw.points) ? raw.points : [],
  };
}

export async function getBBHorizons(): Promise<BBHorizonsResponse> {
  const response = await apiClient.get('/bb/horizons');
  return response.data;
}

export async function getSymbolBB(
  symbol: string,
  options?: {
    as_of?: string;
    horizons?: string;
    limit?: number;
    format?: 'series' | 'chart';
  }
): Promise<BBSymbolSeriesResponse> {
  const response = await apiClient.get(`/bb/symbol/${symbol}`, { params: options });
  return normalizeBBSeriesEnvelope(response.data);
}

export async function getMarketBB(
  universeId: string,
  options?: {
    as_of?: string;
    mode?: string;
    horizons?: string;
    buffer?: number;
    coverage_threshold?: number;
    limit?: number;
  }
): Promise<MarketBBSeriesResponse> {
  const response = await apiClient.get(`/bb/market/${universeId}`, { params: options });
  return response.data;
}
