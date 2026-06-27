/** Decision actions available to the user */
export type DecisionAction =
  | 'BUY'
  | 'SELL'
  | 'HOLD'
  | 'SKIP'
  | 'ADD'
  | 'REDUCE'
  | 'CLOSE'
  | 'CUT_LOSS'
  | 'TAKE_PROFIT';

/** Request payload for submitting a decision */
export interface DecisionCreate {
  action: DecisionAction;
  price?: number;
  quantity?: number;
  order_type?: string;
  stop_loss?: number;
  target_price?: number;
  confidence_score?: number;
  setup_type?: string;
  market_context?: string;
  reason?: string;
  note?: string;
  mistake_tag?: string;
}

/** Decision response from the backend */
export interface Decision {
  id: number;
  session_id: number;
  symbol: string;
  decision_date: string;
  candle_index: number;
  action: DecisionAction;
  price?: number;
  confidence_score?: number;
  setup_type?: string;
  market_context?: string;
  reason?: string;
  note?: string;
  mistake_tag?: string;
  created_at: string;
}
