/** Journal entry creation request */
export interface JournalEntryCreate {
  note_type: string;
  content: string;
  tags?: string;
  decision_id?: number;
  trade_id?: number;
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
  created_at: string;
  updated_at: string;
}
