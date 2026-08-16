/** Journal entry creation request */
export interface JournalEntryCreate {
  note_type: string;
  content: string;
  tags?: string;
  decision_id?: number;
  trade_id?: number;
  setup_type?: string;
  market_regime?: string;
  confidence_score?: number;
  emotion?: string;
  mistake_tag?: string;
  rule_violation?: string;
}

/** Journal entry response from the backend */
export interface JournalEntry {
  id: number;
  session_id: number;
  decision_id?: number;
  trade_id?: number;
  note_type: string;
  content: string;
  tags?: string;
  setup_type?: string;
  market_regime?: string;
  confidence_score?: number;
  emotion?: string;
  mistake_tag?: string;
  rule_violation?: string;
  created_at: string;
  updated_at: string;
}
