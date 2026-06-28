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
  max_drawdown_pct?: number;
  equity_curve?: EquityPoint[];
  setup_performance?: SetupPerformance[];
  symbol_performance?: GroupPerformance[];
  mistake_performance?: GroupPerformance[];
  outlier_impact?: OutlierImpact;
}
