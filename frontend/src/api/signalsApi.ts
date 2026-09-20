import { apiClient } from './client';
import type {
  SignalCalculationRequest,
  SignalCalculationResponse,
  SignalRegistryResponse,
} from '../types/signals';

/**
 * Fetch all registered signal definitions from the backend registry.
 */
export const getSignalRegistry = async (): Promise<SignalRegistryResponse> => {
  const response = await apiClient.get('/signals/registry');
  return response.data;
};

/**
 * Calculate signals for a replay session prefix up to its current_index.
 */
export const calculateReplaySignals = async (
  sessionId: number,
  request: SignalCalculationRequest
): Promise<SignalCalculationResponse> => {
  const response = await apiClient.post(
    `/signals/replay/${sessionId}/calculate`,
    request
  );
  return response.data;
};
