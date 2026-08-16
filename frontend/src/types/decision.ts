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

/** Trade planning input for position sizing calculator */
export interface TradePlanInput {
  risk_percent: number;
  entry_price: number;
  stop_loss: number;
  target_price: number;
  lot_size?: number;
  fee_rate?: number;
  tax_rate?: number;
}

/** Trade planning result from position sizing calculator */
export interface TradePlanResult {
  risk_per_share: number;
  reward_per_share: number;
  expected_r_multiple: number;
  max_risk_amount: number;
  raw_quantity: number;
  lot_rounded_quantity: number;
  max_cash_quantity: number;
  planned_quantity: number;
  planned_risk_amount: number;
  estimated_buy_gross: number;
  estimated_buy_fee: number;
  estimated_buy_net: number;
  estimated_sell_gross: number;
  estimated_sell_fee: number;
  estimated_sell_tax: number;
  estimated_sell_net: number;
  estimated_total_fees_and_taxes: number;
  estimated_net_profit: number;
  estimated_net_r: number;
  affordable: boolean;
  status_message: string;
}

/** Request payload for submitting a decision */
export interface DecisionCreate {
  action: DecisionAction;
  price?: number;
  quantity?: number;
  order_type?: string;
  stop_loss?: number;
  target_price?: number;
  planned_quantity?: number;
  planned_risk?: number;
  planned_r?: number;
  confidence_score?: number;
  setup_type?: string;
  market_context?: string;
  market_regime?: string;
  emotion?: string;
  reason?: string;
  note?: string;
  mistake_tag?: string;
  rule_violation?: string;
  checklist_snapshot?: string;
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
  quantity?: number;
  stop_loss?: number;
  target_price?: number;
  planned_quantity?: number;
  planned_risk?: number;
  planned_r?: number;
  confidence_score?: number;
  setup_type?: string;
  market_context?: string;
  market_regime?: string;
  emotion?: string;
  reason?: string;
  note?: string;
  mistake_tag?: string;
  rule_violation?: string;
  checklist_snapshot?: string;
  created_at: string;
}
