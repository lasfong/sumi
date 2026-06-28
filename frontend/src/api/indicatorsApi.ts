import { apiClient } from './client';

export interface IndicatorDataPoint {
  timestamp: string;
  [key: string]: string | number | null;
}

export const getIndicatorData = async (symbol: string, indicator: string, params: Record<string, string | number | boolean> = {}): Promise<IndicatorDataPoint[]> => {
  const response = await apiClient.get(`/indicators/${symbol}`, {
    params: {
      indicator,
      ...params
    }
  });
  return response.data.data;
};

export const getSessionIndicatorData = async (sessionId: number, indicator: string, params: Record<string, string | number | boolean> = {}): Promise<IndicatorDataPoint[]> => {
  const response = await apiClient.get(`/replay/sessions/${sessionId}/indicators`, {
    params: {
      indicator,
      ...params
    }
  });
  return response.data.data;
};
