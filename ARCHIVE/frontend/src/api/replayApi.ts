import { apiClient } from './client';
import type { ReplaySession, CreateSessionRequest, Candle } from '../types';

export interface DrawingStateResponse {
  id: number;
  session_id: number;
  symbol: string;
  state_data: string;
}

export const createReplaySession = async (data: CreateSessionRequest): Promise<ReplaySession> => {
  if ((data.mode as string | undefined) === 'backtest') {
    throw new Error('Cannot create manual replay session with mode="backtest"');
  }
  const response = await apiClient.post('/replay/sessions', data);
  return response.data;
};

export const getReplaySession = async (sessionId: number, signal?: AbortSignal): Promise<ReplaySession> => {
  const response = await apiClient.get(`/replay/sessions/${sessionId}`, signal ? { signal } : undefined);
  return response.data;
};

export const listReplaySessions = async (limit: number = 10): Promise<ReplaySession[]> => {
  const response = await apiClient.get('/replay/sessions', { params: { limit } });
  return response.data;
};

export const getSessionCandles = async (
  sessionId: number,
  targetTimeframe?: string,
  signal?: AbortSignal,
): Promise<Candle[]> => {
  const params = targetTimeframe ? { target_timeframe: targetTimeframe } : undefined;
  const config = signal ? { params, signal } : { params };
  const response = await apiClient.get(`/replay/sessions/${sessionId}/candles`, config);
  return response.data;
};

export const nextCandle = async (
  sessionId: number,
  steps: number = 1,
  signal?: AbortSignal,
): Promise<ReplaySession> => {
  const response = await apiClient.post(
    `/replay/sessions/${sessionId}/next?steps=${steps}`,
    undefined,
    signal ? { signal } : undefined,
  );
  return response.data;
};

export const previousCandle = async (
  sessionId: number,
  steps: number = 1,
  signal?: AbortSignal,
): Promise<ReplaySession> => {
  const response = await apiClient.post(
    `/replay/sessions/${sessionId}/previous`,
    null,
    signal ? { params: { steps }, signal } : { params: { steps } },
  );
  return response.data;
};

export const getDrawings = async (sessionId: number): Promise<DrawingStateResponse> => {
  const response = await apiClient.get(`/replay/sessions/${sessionId}/drawings`);
  if (response.headers['x-sumi-uat-reconciliation-unavailable'] === '1') throw new Error('Controlled drawing reconciliation GET unavailable after dispatch');
  return response.data;
};

export const updateDrawings = async (sessionId: number, stateData: string): Promise<DrawingStateResponse> => {
  const response = await apiClient.put(`/replay/sessions/${sessionId}/drawings`, { state_data: stateData });
  if (response.headers['x-sumi-uat-commit-then-error'] === '1') throw new Error('Controlled drawing PUT committed before client failure');
  return response.data;
};

export const resetPracticeSession = async (sessionId: number): Promise<{ status: string; message: string }> => {
  const response = await apiClient.post(`/replay/sessions/${sessionId}/reset-practice`);
  return response.data;
};
