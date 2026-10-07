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

export interface BBSymbolSeriesResponse {
  symbol: string;
  timeframe: string;
  methodology: string;
  calculation_as_of: string;
  points: BBPoint[];
  summary: Record<string, unknown>;
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
  return response.data;
}
