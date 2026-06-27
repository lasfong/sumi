export interface SetupPerformance {
  setup_type: string;
  trades: number;
  win_rate: number;
  net_pnl: number;
}

/** Analytics report for a replay session */
export interface AnalyticsReport {
  total_trades: number;
  win_rate: number;
  total_net_pnl: number;
  average_win: number;
  average_loss: number;
  profit_factor: number;
  average_r?: number;
  expectancy?: number;
  largest_win?: number;
  largest_loss?: number;
  max_drawdown?: number;
  sharpe_ratio?: number;
  sortino_ratio?: number;
  sqn?: number;
  equity_curve?: { timestamp: string; equity: number; drawdown: number }[];
  setup_performance?: SetupPerformance[];
}
