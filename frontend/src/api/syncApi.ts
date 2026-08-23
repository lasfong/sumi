import { apiClient } from './client';

export interface ProviderMetadata {
  provider_id: string;
  display_name: string;
  is_official: boolean;
  requires_auth: boolean;
  supported_timeframes: string[];
  supported_adjustments: string[];
  rate_limit_rps: number;
  description: string;
  auth_fields: string[];
  is_configured: boolean;
}

export interface TestConnectionResponse {
  success: boolean;
  provider_id: string;
  message: string;
  latency_ms?: number | null;
}

export interface SyncRunItem {
  row_index: number;
  symbol: string;
  timeframe: string;
  timestamp: string;
  adjustment_type: string;
  open?: number | null;
  high?: number | null;
  low?: number | null;
  close?: number | null;
  volume?: number | null;
  classification: string;
  reject_reason?: string | null;
}

export interface SyncPreviewResponse {
  sync_id: string;
  provider_id: string;
  symbol: string;
  start_date: string;
  end_date: string;
  timeframe: string;
  adjustment_type: string;
  status: string;
  parsed_count: number;
  rejected_count: number;
  duplicate_count: number;
  conflicting_count: number;
  missing_count: number;
  out_of_order_count: number;
  can_accept: boolean;
  block_reason?: string | null;
  content_sha256: string;
  items: SyncRunItem[];
}

export interface SyncExecuteResponse {
  sync_id: string;
  status: string;
  accepted_count: number;
  symbol: string;
  timeframe: string;
  adjustment_type: string;
  duration_ms: number;
  message: string;
  manifest?: Record<string, unknown> | null;
}

export interface SyncRollbackResponse {
  sync_id: string;
  status: string;
  restored_mutations_count: number;
  message: string;
}

export interface SyncManifest {
  sync_id: string;
  created_at: string;
  provider_id: string;
  symbol: string;
  start_date: string;
  end_date: string;
  timeframe: string;
  adjustment_type: string;
  status: string;
  parsed_count: number;
  duplicate_count: number;
  conflicting_count: number;
  accepted_count: number;
  duration_ms: number;
  accepted_at?: string | null;
  rolled_back_at?: string | null;
  manifest?: Record<string, unknown> | null;
}

export const getSyncProviders = async (): Promise<ProviderMetadata[]> => {
  const response = await apiClient.get('/sync/providers');
  return response.data;
};

export const testProviderConnection = async (
  providerId: string,
  credentials?: Record<string, unknown>
): Promise<TestConnectionResponse> => {
  const response = await apiClient.post('/sync/test-connection', {
    provider_id: providerId,
    credentials: credentials || undefined,
  });
  return response.data;
};

export const generateSyncPreview = async (
  symbol: string,
  startDate: string,
  endDate: string,
  providerId: string = 'ssi',
  adjustmentType: string = 'unadjusted',
  credentials?: Record<string, unknown>
): Promise<SyncPreviewResponse> => {
  const response = await apiClient.post('/sync/preview', {
    symbol,
    start_date: startDate,
    end_date: endDate,
    provider_id: providerId,
    adjustment_type: adjustmentType,
    credentials: credentials || undefined,
  });
  return response.data;
};

export const executeSync = async (
  syncId: string,
  contentSha256: string
): Promise<SyncExecuteResponse> => {
  const response = await apiClient.post('/sync/execute', {
    sync_id: syncId,
    content_sha256: contentSha256,
  });
  return response.data;
};

export const rollbackSync = async (syncId: string): Promise<SyncRollbackResponse> => {
  const response = await apiClient.post('/sync/rollback', {
    sync_id: syncId,
  });
  return response.data;
};

export const getSyncHistory = async (limit: number = 50): Promise<SyncManifest[]> => {
  const response = await apiClient.get('/sync/history', {
    params: { limit },
  });
  return response.data;
};

export const getSyncManifest = async (syncId: string): Promise<SyncManifest> => {
  const response = await apiClient.get(`/sync/${syncId}/manifest`);
  return response.data;
};
