import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export interface ReplayState {
  sessionId: number | null;
  setSession: (id: number) => void;
  clearSession: () => void;
}

export const useReplayStore = create<ReplayState>()(
  persist(
    (set) => ({
      sessionId: null,
      setSession: (id) => set({ sessionId: id }),
      clearSession: () => set({ sessionId: null }),
    }),
    {
      name: 'sumi-replay-store',
      partialize: (state) => ({ sessionId: state.sessionId }),
    }
  )
);
