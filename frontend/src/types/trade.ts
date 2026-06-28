/** Position held in a replay session */
export interface Position {
  id: number;
  session_id: number;
  symbol: string;
  quantity: number;
  average_price: number;
  total_cost: number;
  realized_pnl: number;
  unrealized_pnl: number;
  status: 'open' | 'closed';
  opened_at: string;
  closed_at?: string;
}

/** Completed or open trade */
export interface Trade {
  id: number;
  session_id: number;
  symbol: string;
  entry_date: string;
  entry_price: number;
  exit_date?: string;
  exit_price?: number;
  quantity: number;
  gross_pnl?: number;
  net_pnl?: number;
  pnl_percent?: number;
  initial_stop_loss?: number;
  target_price?: number;
  initial_risk?: number;
  r_multiple?: number;
  holding_candles?: number;
  holding_days?: number;
  status?: string;
  result?: string;
  setup_type?: string;
  mistake_tag?: string;
}

export interface Order {
  id: number;
  session_id: number;
  decision_id: number;
  symbol: string;
  side: 'BUY' | 'SELL';
  order_type: 'MARKET_AT_CLOSE' | 'MARKET_NEXT_OPEN' | 'LIMIT' | 'CUSTOM_PRICE';
  requested_price?: number | null;
  quantity: number;
  capital_percent?: number | null;
  status: 'created' | 'pending' | 'executed' | 'cancelled' | 'rejected';
  created_at: string;
}
