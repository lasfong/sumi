import { apiClient } from './client';
import type { AnalyticsReport } from '../types/analytics';

export interface StrategyConfig {
  name: string;
  version?: string;
  description?: string;
  indicators: Record<string, unknown>[];
  entry_rules: Record<string, unknown>[];
  exit_rules: Record<string, unknown>[];
  position_sizing: Record<string, unknown>;
  risk_management?: Record<string, unknown> | null;
}

export interface BacktestRequest {
  symbol?: string;
  symbols?: string[];
  start_date: string;
  end_date: string;
  initial_cash?: number;
  strategy: StrategyConfig | Record<string, unknown>;
}

export interface BacktestRunSummary {
  total_symbols: number;
  succeeded_symbols: number;
  failed_symbols: number;
  total_candles: number;
  total_trades: number;
  win_rate: number;
  total_net_pnl: number;
  best_symbol?: { symbol: string; net_pnl: number } | null;
  worst_symbol?: { symbol: string; net_pnl: number } | null;
}

export interface BacktestResultSlice {
  group_type: string;
  key: string;
  trades: number;
  win_rate: number;
  net_pnl: number;
  average_pnl: number;
  best_trade?: number | null;
  worst_trade?: number | null;
}

export interface BacktestResponse {
  status?: 'succeeded' | 'failed' | 'partial';
  session_id?: number;
  strategy?: string;
  symbol?: string;
  symbols?: string[];
  total_candles?: number;
  analytics: AnalyticsReport | null;
  error?: string;
  error_code?: string;
  message?: string;
  runs?: BacktestResponse[];
  summary?: BacktestRunSummary;
  slices?: BacktestResultSlice[];
}

export interface AvailableStrategy {
  filename: string;
  name: string;
  description: string;
  config: StrategyConfig | Record<string, unknown>;
}

export async function getAvailableStrategies(): Promise<AvailableStrategy[]> {
  const response = await apiClient.get('/backtest/strategies');
  return response.data;
}

export async function runBacktest(config: BacktestRequest): Promise<BacktestResponse> {
  const response = await apiClient.post('/backtest/run', config);
  return response.data;
}
