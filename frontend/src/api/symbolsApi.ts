import { apiClient } from './client';
import type { StockSymbol } from '../types';

export interface DataReadiness {
  status: 'ready' | 'empty' | 'partial';
  symbols_count: number;
  symbols_with_candles: string[];
  timeframes: string[];
  total_candles: number;
  earliest_timestamp?: string | null;
  latest_timestamp?: string | null;
}

/** Fetch all available symbols, optionally filtered */
export const getSymbols = async (params?: {
  asset_type?: string;
  exchange?: string;
  search?: string;
}): Promise<StockSymbol[]> => {
  const response = await apiClient.get('/symbols', { params });
  return response.data;
};

/** Fetch honest local candle readiness metrics */
export const getDataReadiness = async (): Promise<DataReadiness> => {
  const response = await apiClient.get('/symbols/readiness');
  return response.data;
};
