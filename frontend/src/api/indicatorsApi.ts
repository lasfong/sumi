import { apiClient } from './client';

export type MappingStatus = 'OK' | 'EMPTY_WARMING' | 'UNMAPPED_COLUMN' | 'INVALID_SPEC';

export interface IndicatorMeta {
  status: MappingStatus;
  warmup_bars: number;
  available_bars: number;
  is_projected_clamped: boolean;
}

export interface ReplayIndicatorResponse {
  session_id: number;
  timeframe: string;
  indicator: string;
  data: IndicatorDataPoint[];
  meta: IndicatorMeta;
}

export interface IndicatorDataPoint {
  timestamp: string;
  is_projected?: boolean;
  [key: string]: string | number | boolean | null | undefined;
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

export const getSessionIndicatorData = async (sessionId: number, indicator: string, params: Record<string, string | number | boolean> = {}, signal?: AbortSignal, timeframe?: string): Promise<IndicatorDataPoint[]> => {
  const queryParams: Record<string, string | number | boolean> = { indicator, ...params };
  if (timeframe) queryParams.timeframe = timeframe;
  const response = await apiClient.get<ReplayIndicatorResponse>(`/replay/sessions/${sessionId}/indicators`, {
    signal,
    params: queryParams
  });
  return response.data.data;
};

export const getSessionIndicatorResponse = async (sessionId: number, indicator: string, params: Record<string, string | number | boolean> = {}, signal?: AbortSignal, timeframe?: string): Promise<ReplayIndicatorResponse> => {
  const queryParams: Record<string, string | number | boolean> = { indicator, ...params };
  if (timeframe) queryParams.timeframe = timeframe;
  const response = await apiClient.get<ReplayIndicatorResponse>(`/replay/sessions/${sessionId}/indicators`, {
    signal,
    params: queryParams
  });
  return response.data;
};
