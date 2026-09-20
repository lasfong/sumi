import { apiClient } from './client';
import type { AnalyticsReport } from '../types/analytics';

export interface DataCoverage {
  requested_start: string;
  requested_end: string;
  actual_start?: string | null;
  actual_end?: string | null;
  symbols_requested: string[];
  symbols_covered: string[];
  candle_count: number;
  warmup_candles: number;
  gaps: string[];
  excluded_data: string[];
}

export interface ExecutionAssumptions {
  execution_timing: string;
  price_basis: string;
  fees: Record<string, number>;
  taxes: Record<string, number>;
  slippage: Record<string, unknown>;
  liquidity: string;
  position_sizing: Record<string, unknown>;
  settlement: string;
}

export interface RunManifest {
  strategy_name: string;
  strategy_version: string;
  strategy_parameters: Record<string, unknown>;
  data_identity: string;
  assumptions_identity: string;
  engine_version: string;
  run_timestamp: string;
  input_hash: string;
}

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
  benchmark_symbol?: string;
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
  data_coverage?: DataCoverage;
  execution_assumptions?: ExecutionAssumptions;
  run_manifest?: RunManifest;
}

export interface AvailableStrategy {
  filename: string;
  name: string;
  description: string;
  config: StrategyConfig | Record<string, unknown>;
}

export interface PhaseDefinitionRequest {
  name: string;
  start_date: string;
  end_date: string;
  description?: string;
}

export interface BatchTimingMetricsResponse {
  total_duration_ms: number;
  feature_compute_ms: number;
  simulation_ms: number;
  cache_hits: number;
  cache_misses: number;
}

export interface BatchBacktestRequest {
  symbols: string[];
  phases: PhaseDefinitionRequest[];
  strategy: StrategyConfig | Record<string, unknown>;
  initial_cash?: number;
  execution_profile?: string;
  exchange?: string;
  benchmark_symbol?: string;
  use_cache?: boolean;
}

export interface BenchmarkMetricRowResponse {
  ticker: string;
  phase_name?: string;
  start_date?: string;
  end_date?: string;
  initial_cash: number;
  final_cash: number;
  final_equity: number;
  net_profit: number;
  net_profit_pct: number;
  num_trades: int_or_number;
  avg_profit_loss_pct?: number | null;
  avg_bars_held?: number | null;
  win_rate_pct?: number | null;
  win_avg_profit_pct?: number | null;
  loss_avg_loss_pct?: number | null;
  num_winners: number;
  num_losers: number;
  num_breakeven: number;
  profit_factor?: number | null;
  max_drawdown: number;
  open_position_quantity: number;
  open_position_value: number;
}

type int_or_number = number;

export interface CrossPhaseDegradationResponse {
  ticker: string;
  base_phase_name: string;
  target_phase_name: string;
  net_profit_delta: number;
  return_delta_pct: number;
  win_rate_delta_pct?: number | null;
  profit_ratio?: number | null;
  degradation_pct?: number | null;
  is_degraded: boolean;
  drawdown_delta: number;
}

export interface PhaseMetricMatrixResponse {
  phase_names: string[];
  symbols: string[];
  rows_by_symbol: Record<string, Record<string, BenchmarkMetricRowResponse>>;
  portfolio_by_phase: Record<string, BenchmarkMetricRowResponse>;
  cross_phase_degradations: CrossPhaseDegradationResponse[];
  consistency_score: number;
}

export interface BatchPhaseResultResponse {
  symbol: string;
  phase_name: string;
  start_date: string;
  end_date: string;
  status: string;
  total_candles: number;
  initial_cash: number;
  final_cash: number;
  final_equity: number;
  net_pnl: number;
  net_return_pct: number;
  total_trades: number;
  open_position_quantity: number;
  open_position_value: number;
  warnings: string[];
  error_message?: string | null;
  benchmark_metrics?: BenchmarkMetricRowResponse | null;
}

export interface BatchBacktestResponse {
  status: string;
  total_symbols: number;
  total_phases: number;
  total_runs: number;
  feature_compute_count: number;
  simulation_run_count: number;
  phase_results: BatchPhaseResultResponse[];
  summary: Record<string, unknown>;
  metric_matrix?: PhaseMetricMatrixResponse | null;
  cross_phase_degradations?: CrossPhaseDegradationResponse[] | null;
  timing_metrics?: BatchTimingMetricsResponse | null;
  markdown_table?: string | null;
  csv_export?: string | null;
}

export async function getAvailableStrategies(): Promise<AvailableStrategy[]> {
  const response = await apiClient.get('/backtest/strategies');
  return response.data;
}

export async function runBacktest(config: BacktestRequest): Promise<BacktestResponse> {
  const response = await apiClient.post('/backtest/run', config);
  return response.data;
}

export async function runBatchBacktest(config: BatchBacktestRequest): Promise<BatchBacktestResponse> {
  const response = await apiClient.post('/backtest/batch/run', config);
  return response.data;
}

