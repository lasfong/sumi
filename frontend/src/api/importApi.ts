import { apiClient } from './client';

export interface ImportWarning {
  row_index: number;
  message: string;
}

export interface ImportResult {
  imported_rows: number;
  symbols_count: number;
  skipped_rows: number;
  start_date?: string | null;
  end_date?: string | null;
  warnings?: ImportWarning[];
}

export interface ImportRunItem {
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

export interface ImportPreviewResponse {
  run_id: string;
  file_name: string;
  file_sha256: string;
  content_sha256: string;
  parser_version: string;
  source_type: string;
  timeframe: string;
  adjustment_type: string;
  timezone: string;
  status: string;
  parsed_count: number;
  rejected_count: number;
  duplicate_count: number;
  conflicting_count: number;
  missing_count: number;
  out_of_order_count: number;
  accepted_count: number;
  can_accept: boolean;
  block_reason?: string | null;
  items: ImportRunItem[];
}

export interface ImportAcceptResponse {
  run_id: string;
  status: string;
  accepted_count: number;
  message: string;
}

export interface ImportRollbackResponse {
  run_id: string;
  status: string;
  restored_mutations_count: number;
  message: string;
}

export interface CatalogItem {
  symbol: string;
  exchange?: string | null;
  timeframe: string;
  adjustment_type: string;
  start_date?: string | null;
  end_date?: string | null;
  row_count: number;
  last_accepted_at?: string | null;
  provenance_state: string;
}

export interface ImportRunSummary {
  run_id: string;
  file_name: string;
  created_at?: string | null;
  source_type: string;
  timeframe: string;
  adjustment_type: string;
  status: string;
  parsed_count: number;
  rejected_count: number;
  duplicate_count: number;
  conflicting_count: number;
  accepted_count: number;
  can_accept: boolean;
  accepted_at?: string | null;
  rolled_back_at?: string | null;
}

export const previewImportData = async (
  file: File,
  adjustmentType: string = 'unadjusted',
  sourceType: string = 'cafef',
  timeframe: string = '1D',
  timezone: string = 'Asia/Ho_Chi_Minh'
): Promise<ImportPreviewResponse> => {
  const formData = new FormData();
  formData.append('file', file);
  formData.append('adjustment_type', adjustmentType);
  formData.append('source_type', sourceType);
  formData.append('timeframe', timeframe);
  formData.append('timezone', timezone);

  const response = await apiClient.post('/import/preview', formData, {
    headers: {
      'Content-Type': 'multipart/form-data',
    },
  });
  return response.data;
};

export const acceptImportRun = async (runId: string, contentSha256: string): Promise<ImportAcceptResponse> => {
  const response = await apiClient.post(`/import/runs/${runId}/accept`, {
    content_sha256: contentSha256,
  });
  return response.data;
};

export const rollbackImportRun = async (runId: string): Promise<ImportRollbackResponse> => {
  const response = await apiClient.post(`/import/runs/${runId}/rollback`);
  return response.data;
};

export const getDataCatalog = async (): Promise<CatalogItem[]> => {
  const response = await apiClient.get('/data/catalog');
  return response.data;
};

export const getImportRuns = async (): Promise<ImportRunSummary[]> => {
  const response = await apiClient.get('/import/runs');
  return response.data;
};

export const importCafefData = async (file: File, adjustmentType: string = 'unadjusted'): Promise<ImportResult> => {
  const formData = new FormData();
  formData.append('file', file);
  formData.append('adjustment_type', adjustmentType);
  
  const response = await apiClient.post('/import/cafef', formData, {
    headers: {
      'Content-Type': 'multipart/form-data',
    },
  });
  return response.data;
};
