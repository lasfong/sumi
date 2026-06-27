import { apiClient } from './client';
import type { StockSymbol } from '../types';

/** Fetch all available symbols, optionally filtered */
export const getSymbols = async (params?: {
  asset_type?: string;
  exchange?: string;
  search?: string;
}): Promise<StockSymbol[]> => {
  const response = await apiClient.get('/symbols', { params });
  return response.data;
};
