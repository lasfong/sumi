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

export interface StrategyLabRunCreate {
  run_type: 'comparison' | 'sweep';
  label: string;
  request_config?: Record<string, unknown>;
  result_payload?: Record<string, unknown>;
  metrics?: Record<string, unknown>;
}

export interface StrategyLabRun {
  id: number;
  run_type: 'comparison' | 'sweep';
  label: string;
  request_config: Record<string, unknown>;
  result_payload: Record<string, unknown>;
  metrics: Record<string, unknown>;
  created_at: string;
}

export async function runParameterSweep(config: ParameterSweepRequest): Promise<ParameterSweepResponse> {
  const response = await apiClient.post('/strategy-lab/sweep', config);
  return response.data;
}

export async function saveStrategyLabRun(payload: StrategyLabRunCreate): Promise<StrategyLabRun> {
  const response = await apiClient.post('/strategy-lab/runs', payload);
  return response.data;
}

export async function listStrategyLabRuns(limit = 50): Promise<StrategyLabRun[]> {
  const response = await apiClient.get('/strategy-lab/runs', { params: { limit } });
  return response.data;
}

export async function deleteStrategyLabRun(runId: number): Promise<{ status: string; deleted: boolean }> {
  const response = await apiClient.delete(`/strategy-lab/runs/${runId}`);
  return response.data;
}
