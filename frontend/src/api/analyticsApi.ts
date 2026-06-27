import { apiClient } from './client';
import type { AnalyticsReport } from '../types';

export const getSessionAnalytics = async (sessionId: number): Promise<AnalyticsReport> => {
  const response = await apiClient.get(`/replay/sessions/${sessionId}/analytics`);
  return response.data;
};
