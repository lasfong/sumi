import { apiClient } from './client';
import type { ReplaySession, CreateSessionRequest, Candle } from '../types';

export const createReplaySession = async (data: CreateSessionRequest): Promise<ReplaySession> => {
  const response = await apiClient.post('/replay/sessions', data);
  return response.data;
};

export const getSessionCandles = async (sessionId: number): Promise<Candle[]> => {
  const response = await apiClient.get(`/replay/sessions/${sessionId}/candles`);
  return response.data;
};

export const nextCandle = async (sessionId: number, steps: number = 1): Promise<ReplaySession> => {
  const response = await apiClient.post(`/replay/sessions/${sessionId}/next?steps=${steps}`);
  return response.data;
};

export const previousCandle = async (sessionId: number, steps: number = 1): Promise<ReplaySession> => {
  const response = await apiClient.post(`/replay/sessions/${sessionId}/previous`, null, { params: { steps } });
  return response.data;
};

export const getDrawings = async (sessionId: number): Promise<any> => {
  const response = await apiClient.get(`/replay/sessions/${sessionId}/drawings`);
  return response.data;
};

export const updateDrawings = async (sessionId: number, stateData: string): Promise<any> => {
  const response = await apiClient.put(`/replay/sessions/${sessionId}/drawings`, { state_data: stateData });
  return response.data;
};
