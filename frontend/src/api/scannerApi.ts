import { apiClient } from './client';
import type { StrategyConfig } from './backtestApi';
import type { ReplaySession } from '../types';

export interface ScannerRequest {
  symbols: string[];
  start_date: string;
  end_date: string;
  strategy: StrategyConfig | Record<string, unknown>;
  benchmark_symbol?: string;
  max_results?: number;
}

export interface ScannerResult {
  symbol: string;
  timestamp: string;
  signal_type: string;
  strategy: string;
  price: number;
  regime: string;
}

export interface ScannerResponse {
  status: 'succeeded' | 'failed';
  run_id?: number;
  total_results?: number;
  truncated?: boolean;
  results: ScannerResult[];
  warnings: string[];
  error_code?: string;
  message?: string;
}

export interface ScannerReplaySessionRequest {
  symbol: string;
  signal_timestamp: string;
  signal_type?: string;
  strategy?: string;
  price?: number;
  regime?: string;
  timeframe?: string;
  adjustment_type?: string;
  lookback_days?: number;
  forward_days?: number;
  initial_cash?: number;
}

export interface ScannerReplaySessionResponse {
  session: ReplaySession;
  signal_timestamp: string;
  window_start: string;
  window_end: string;
}

export interface ScannerRun {
  id: number;
  label: string;
  status: string;
  total_results: number;
  request_config: ScannerRequest | Record<string, unknown>;
  result_payload: ScannerResponse;
  created_at: string;
}

export async function runScanner(config: ScannerRequest): Promise<ScannerResponse> {
  const response = await apiClient.post('/scanner/run', config);
  return response.data;
}

export async function createReplaySessionFromSignal(config: ScannerReplaySessionRequest): Promise<ScannerReplaySessionResponse> {
  const response = await apiClient.post('/scanner/replay-session', config);
  return response.data;
}

export async function listScannerRuns(limit: number = 20): Promise<ScannerRun[]> {
  const response = await apiClient.get('/scanner/runs', { params: { limit } });
  return response.data;
}
