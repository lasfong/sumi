export interface PracticeDecision {
  id: number;
  action: string;
  candle_index: number;
  decision_date: string;
  price?: number | null;
  quantity?: number | null;
  stop_loss?: number | null;
  target_price?: number | null;
  planned_quantity?: number | null;
  planned_risk?: number | null;
  planned_r?: number | null;
  setup_type?: string | null;
  confidence_score?: number | null;
  market_context?: string | null;
  market_regime?: string | null;
  emotion?: string | null;
  reason?: string | null;
  note?: string | null;
  mistake_tag?: string | null;
  rule_violation?: string | null;
  checklist_snapshot?: string | null;
}

export interface PracticeOrder {
  id: number;
  decision_id: number;
  side: 'BUY' | 'SELL';
  order_type: string;
  requested_price?: number | null;
  quantity: number;
  status: 'created' | 'pending' | 'executed' | 'cancelled' | 'rejected';
  decision_index: number;
  explanation: string;
}

export interface PracticeExecution {
  id: number;
  order_id: number;
  decision_id: number;
  side: 'BUY' | 'SELL';
  execution_index: number;
  execution_date: string;
  execution_price: number;
  quantity: number;
  net_amount: number;
}

export interface PracticePosition {
  id: number;
  symbol: string;
  quantity: number;
  average_price: number;
  total_cost: number;
  current_price: number;
  realized_pnl: number;
  unrealized_pnl: number;
  available_quantity: number;
  blocked_quantity?: number;
  earliest_release_date?: string | null;
  opened_at: string;
}

export interface PracticeTrade {
  id: number;
  symbol: string;
  entry_date: string;
  entry_price: number;
  quantity: number;
  exit_date?: string | null;
  exit_price?: number | null;
  net_pnl?: number | null;
  pnl_percent?: number | null;
  initial_stop_loss?: number | null;
  target_price?: number | null;
  initial_risk?: number | null;
  r_multiple?: number | null;
  planned_entry_price?: number | null;
  planned_quantity?: number | null;
  planned_r?: number | null;
  setup_type?: string | null;
  market_regime?: string | null;
  emotion?: string | null;
  mistake_tag?: string | null;
  rule_violation?: string | null;
  entry_drift?: number | null;
  size_variance?: number | null;
  r_variance?: number | null;
  notes?: string | null;
  status: string;
  result: string;
}

export interface PracticeWorkflowSnapshot {
  session_id: number;
  symbol: string;
  current_index: number;
  visible_bar: number;
  total_bars: number;
  current_date: string;
  current_price: number;
  current_volume: number;
  initial_cash: number;
  current_cash: number;
  available_quantity: number;
  blocked_quantity?: number;
  earliest_release_date?: string | null;
  latest_activity_index: number;
  historical: boolean;
  can_trade: boolean;
  trade_block_reason?: string | null;
  decisions: PracticeDecision[];
  orders: PracticeOrder[];
  executions: PracticeExecution[];
  positions: PracticePosition[];
  trades: PracticeTrade[];
}
