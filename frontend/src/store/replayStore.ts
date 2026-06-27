import { create } from 'zustand';
import type { Candle, Position, Trade } from '../types';

interface ReplayState {
  sessionId: number | null;
  candles: Candle[];
  currentCandleIndex: number;
  positions: Position[];
  trades: Trade[];
  setSession: (id: number) => void;
  clearSession: () => void;
  setCandles: (candles: Candle[]) => void;
  setPositions: (positions: Position[]) => void;
  setTrades: (trades: Trade[]) => void;
}

export const useReplayStore = create<ReplayState>((set) => ({
  sessionId: null,
  candles: [],
  currentCandleIndex: 0,
  positions: [],
  trades: [],
  setSession: (id) => set({ sessionId: id }),
  clearSession: () => set({ sessionId: null, candles: [], currentCandleIndex: 0, positions: [], trades: [] }),
  setCandles: (candles) => set({ candles }),
  setPositions: (positions) => set({ positions }),
  setTrades: (trades) => set({ trades }),
}));
