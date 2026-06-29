/** Replay session from the backend */
export interface ReplaySession {
  id: number;
  symbol: string;
  timeframe: string;
  adjustment_type: string;
  start_date: string;
  end_date: string;
  current_index: number;
  initial_cash: number;
  current_cash: number;
  status: SessionStatus;
  mode: SessionMode;
  hide_symbol: number;
  hide_date: number;
  source_type?: string | null;
  source_payload?: string | null;
  created_at: string;
  updated_at: string;
}

/** Request payload for creating a new replay session */
export interface CreateSessionRequest {
  symbol: string;
  timeframe?: string;
  adjustment_type?: string;
  start_date: string;
  end_date: string;
  initial_cash?: number;
  mode?: SessionMode;
}

export type SessionStatus = 'created' | 'active' | 'paused' | 'completed' | 'archived';
export type SessionMode = 'normal' | 'random' | 'blind_symbol' | 'blind_date';
