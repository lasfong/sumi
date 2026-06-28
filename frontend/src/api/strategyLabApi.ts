import { apiClient } from './client';
import type { BacktestResponse, StrategyConfig } from './backtestApi';

export interface SweepParameter {
  path: string;
  values: Array<string | number | boolean>;
}

export interface ParameterSweepRequest {
  symbol?: string;
  symbols?: string[];
  start_date: string;
  end_date: string;
  initial_cash?: number;
  benchmark_symbol?: string;
  strategy: StrategyConfig | Record<string, unknown>;
  sweep: SweepParameter[];
  max_variants?: number;
}

export interface SweepMetrics {
  status: string;
  total_trades: number;
  win_rate: number;
  net_pnl: number;
  profit_factor?: number | null;
  expectancy?: number | null;
}

export interface SweepVariant {
  label: string;
  parameters: Record<string, string | number | boolean>;
  response: BacktestResponse;
  metrics: SweepMetrics;
}

export interface ParameterSweepResponse {
  status: 'succeeded' | 'failed';
  total_variants?: number;
  truncated?: boolean;
  variants: SweepVariant[];
  error_code?: string;
  message?: string;
}

export async function runParameterSweep(config: ParameterSweepRequest): Promise<ParameterSweepResponse> {
  const response = await apiClient.post('/strategy-lab/sweep', config);
  return response.data;
}
