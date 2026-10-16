import { apiClient } from './client';
import type {
  SignalCalculationItemRequest,
  SignalCalculationRequest,
  SignalCalculationResponse,
  SignalRegistryResponse,
} from '../types/signals';

export const DEFAULT_SIGNAL_VERSION = '1.0.0';

export function validateAndNormalizeSignalRequest(
  request: SignalCalculationRequest
): SignalCalculationRequest {
  if (!request || typeof request !== 'object') {
    throw new Error('Signal calculation request payload is required');
  }

  const allowedTopKeys = new Set(['signals']);
  for (const key of Object.keys(request as unknown as Record<string, unknown>)) {
    if (!allowedTopKeys.has(key)) {
      throw new Error(`Forbidden extra field in SignalCalculationRequest: ${key}`);
    }
  }

  if (!Array.isArray(request.signals) || request.signals.length < 1 || request.signals.length > 2) {
    throw new Error('SignalCalculationRequest must specify between 1 and 2 signals');
  }

  const seenNames = new Set<string>();
  const normalizedSignals: SignalCalculationItemRequest[] = [];
  const allowedItemKeys = new Set(['name', 'version', 'params']);

  for (const item of request.signals) {
    if (!item || typeof item !== 'object') {
      throw new Error('Invalid signal calculation item');
    }
    for (const key of Object.keys(item as unknown as Record<string, unknown>)) {
      if (!allowedItemKeys.has(key)) {
        throw new Error(`Forbidden extra field in SignalCalculationItemRequest: ${key}`);
      }
    }
    const name = typeof item.name === 'string' ? item.name.trim() : '';
    if (!name) {
      throw new Error('Signal name cannot be empty');
    }
    if (seenNames.has(name)) {
      throw new Error(`Duplicate signal name in calculation request is prohibited: ${name}`);
    }
    seenNames.add(name);

    if (item.params === null || (item.params !== undefined && typeof item.params !== 'object')) {
      throw new Error(`Signal params cannot be null for ${name}`);
    }

    normalizedSignals.push({
      name,
      version: item.version ?? DEFAULT_SIGNAL_VERSION,
      params: item.params ?? {},
    });
  }

  return { signals: normalizedSignals };
}

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
  const normalizedRequest = validateAndNormalizeSignalRequest(request);
  const response = await apiClient.post(
    `/signals/replay/${sessionId}/calculate`,
    normalizedRequest
  );
  return response.data;
};
