import { apiClient } from './client';

export interface IndicatorDataPoint {
  timestamp: string;
  [key: string]: string | number | null;
}

export interface IndicatorParamDefinition {
  name: string;
  type: 'int' | 'float' | 'string' | 'str' | 'bool';
  default: string | number | boolean | null;
  minimum: number | null;
  maximum: number | null;
}

export interface IndicatorDefinition {
  id: string;
  label: string;
  category: string;
  pane: 'main' | 'oscillator';
  params: IndicatorParamDefinition[];
  description: string;
}

export const getIndicatorRegistry = async (): Promise<IndicatorDefinition[]> => {
  const response = await apiClient.get('/indicators/registry');
  return response.data.indicators;
};

export const getIndicatorData = async (symbol: string, indicator: string, params: Record<string, string | number | boolean> = {}): Promise<IndicatorDataPoint[]> => {
  const response = await apiClient.get(`/indicators/${symbol}`, {
    params: {
      indicator,
      ...params
    }
  });
  return response.data.data;
};

export const getSessionIndicatorData = async (sessionId: number, indicator: string, params: Record<string, string | number | boolean> = {}, signal?: AbortSignal): Promise<IndicatorDataPoint[]> => {
  const response = await apiClient.get(`/replay/sessions/${sessionId}/indicators`, {
    signal,
    params: {
      indicator,
      ...params
    }
  });
  return response.data.data;
};
