export interface SetupPerformance {
  setup_type: string;
  trades: number;
  win_rate: number;
  net_pnl: number;
}

export interface EquityPoint {
  timestamp: string;
  equity: number;
  cash: number;
  holdings_value: number;
  drawdown: number;
  drawdown_pct: number;
}

export interface GroupPerformance {
  key: string;
  trades: number;
  win_rate: number;
  net_pnl: number;
  average_pnl: number;
  best_trade?: number;
  worst_trade?: number;
}

export interface OutlierImpact {
  top_winners_pnl: number;
  top_losers_pnl: number;
  top_winners_share?: number;
  top_losers_share?: number;
  median_trade_pnl?: number;
  trimmed_expectancy?: number;
}

export interface DrawdownPeriod {
  start: string;
  end: string;
  max_drawdown_pct: number;
}

export interface BenchmarkPoint {
  time: string;
  value: number;
}

export interface TradeDistribution {
  trade_id: number;
  symbol: string;
  net_pnl: number;
  pnl_percent: number;
  r_multiple?: number | null;
  result: string;
}

export type MetricStatus = 'valid' | 'insufficient_data' | 'not_applicable';

export interface MetricResult {
  value: number | null;
  status: MetricStatus;
  sample_size: number;
  period?: string | null;
  reason?: string | null;
}

/** Analytics report for a replay session */
export interface AnalyticsReport {
  benchmark_symbol?: string;
  total_trades: number;
  win_rate: number;
  total_net_pnl: number;
  average_win: number;
  average_loss: number;
  profit_factor: number | null;
  profit_factor_display?: string | null;
  low_sample_warning?: boolean | null;
  low_sample_message?: string | null;
  average_r?: number | null;
  expectancy?: number | null;
  largest_win?: number | null;
  largest_loss?: number | null;
  max_drawdown?: number | null;
  sharpe_ratio?: number | null;
  sortino_ratio?: number | null;
  sqn?: number | null;
  max_drawdown_pct?: number | null;
  equity_curve?: EquityPoint[] | null;
  setup_performance?: SetupPerformance[] | null;
  symbol_performance?: GroupPerformance[] | null;
  mistake_performance?: GroupPerformance[] | null;
  drawdown_periods?: DrawdownPeriod[] | null;
  benchmark_curve?: BenchmarkPoint[] | null;
  trade_distribution?: TradeDistribution[] | null;
  outlier_impact?: OutlierImpact | null;
  metrics?: Record<string, MetricResult>;
}
