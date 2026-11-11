import { apiClient } from './client';
import type { BacktestResponse, StrategyConfig } from './backtestApi';
import type { MetricResult } from '../types/analytics';

export interface TypedParameterDefinition {
  target_type: 'indicator' | 'position_sizing' | 'risk_management';
  target_name: string;
  parameter: string;
  label: string;
  type: 'int' | 'float' | 'str';
  current_value?: string | number | null;
  path: string;
}

export interface SweepParameter {
  path?: string;
  target_type?: string;
  target_name?: string;
  parameter?: string;
  values: Array<string | number | boolean>;
}

export interface ParameterSweepRequest {
  sweep_id?: string;
  symbol?: string;
  symbols?: string[];
  start_date: string;
  end_date: string;
  oos_start_date?: string;
  oos_end_date?: string;
  initial_cash?: number;
  benchmark_symbol?: string;
  strategy: StrategyConfig | Record<string, unknown>;
  sweep: SweepParameter[];
  max_variants?: number;
}

export interface RobustnessMetrics {
  badge: 'Robust' | 'Overfitted' | 'Low Sample' | 'Unvalidated' | 'Unprofitable' | 'Degraded';
  score: number;
  stability_ratio?: number | null;
  profit_factor_degradation?: number | null;
  sample_size_is: number;
  sample_size_oos?: number | null;
}

export interface PeriodMetrics {
  status: string;
  total_trades: number;
  win_rate: number;
  net_pnl: number;
  profit_factor?: number | null;
  expectancy?: number | null;
  metric_results?: Record<string, MetricResult>;
}

export interface SweepMetrics extends PeriodMetrics {
  ranking_eligible?: boolean;
  ranking_reason?: string | null;
  out_of_sample?: PeriodMetrics | null;
  robustness?: RobustnessMetrics | null;
  robustness_score?: number;
  robustness_badge?: string;
}

export interface SweepVariant {
  label: string;
  parameters: Record<string, string | number | boolean>;
  response: BacktestResponse;
  oos_response?: BacktestResponse | null;
  metrics: SweepMetrics;
}

export interface DatePeriod {
  start_date: string;
  end_date: string;
}

export interface ParameterSweepResponse {
  status: 'succeeded' | 'failed' | 'cancelled';
  sweep_id?: string;
  cancelled?: boolean;
  total_variants?: number;
  truncated?: boolean;
  ranking_metric?: string;
  in_sample_period?: DatePeriod;
  out_of_sample_period?: DatePeriod | null;
  execution_assumptions?: Record<string, unknown>;
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

export interface StrategyValidationResponse {
  valid: boolean;
  strategy_name?: string;
  version?: string;
  indicators?: Array<Record<string, unknown>>;
  errors: string[];
}

export interface StrategyParametersResponse {
  status: 'succeeded' | 'failed';
  parameters: TypedParameterDefinition[];
}

export async function runParameterSweep(config: ParameterSweepRequest): Promise<ParameterSweepResponse> {
  const response = await apiClient.post('/strategy-lab/sweep', config);
  return response.data;
}

export async function cancelParameterSweep(sweepId: string): Promise<{ status: string; message: string; sweep_id: string }> {
  const response = await apiClient.post('/strategy-lab/sweep/cancel', { sweep_id: sweepId });
  return response.data;
}

export async function validateStrategy(strategy: Record<string, unknown>): Promise<StrategyValidationResponse> {
  const response = await apiClient.post('/strategy-lab/validate', { strategy });
  return response.data;
}

export async function getStrategyParameters(strategy: Record<string, unknown>): Promise<StrategyParametersResponse> {
  const response = await apiClient.post('/strategy-lab/parameters', { strategy });
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
