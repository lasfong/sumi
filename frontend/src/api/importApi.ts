import { apiClient } from './client';

export const importCafefData = async (file: File, adjustmentType: string = 'unadjusted') => {
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
