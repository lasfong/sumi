import { apiClient } from './client';
import type { Decision, DecisionCreate, Order, Position, PracticeWorkflowSnapshot, Trade } from '../types';

export const submitDecision = async (sessionId: number, data: DecisionCreate): Promise<Decision> => {
  const response = await apiClient.post(`/replay/sessions/${sessionId}/decisions`, data);
  return response.data;
};

export const getDecisions = async (sessionId: number): Promise<Decision[]> => {
  const response = await apiClient.get(`/replay/sessions/${sessionId}/decisions`);
  return response.data;
};

export const getPositions = async (sessionId: number): Promise<Position[]> => {
  const response = await apiClient.get(`/replay/sessions/${sessionId}/position`);
  return response.data;
};

export const getOrders = async (sessionId: number): Promise<Order[]> => {
  const response = await apiClient.get(`/replay/sessions/${sessionId}/orders`);
  return response.data;
};

export const getTrades = async (sessionId: number): Promise<Trade[]> => {
  const response = await apiClient.get(`/replay/sessions/${sessionId}/trades`);
  return response.data;
};

export const getPracticeState = async (sessionId: number): Promise<PracticeWorkflowSnapshot> => {
  const response = await apiClient.get(`/replay/sessions/${sessionId}/practice-state`);
  return response.data;
};

export const planTradeSizing = async (
  sessionId: number,
  plan: import('../types/decision').TradePlanInput
): Promise<import('../types/decision').TradePlanResult> => {
  const response = await apiClient.post(`/replay/sessions/${sessionId}/plan-sizing`, plan);
  return response.data;
};
