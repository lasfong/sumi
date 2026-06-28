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
