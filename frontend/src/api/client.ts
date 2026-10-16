import axios from 'axios';

export interface ApiFieldError {
  field: string;
  message: string;
  type?: string;
}

export interface ApiErrorEnvelope {
  code?: string;
  message?: string;
  field_errors?: ApiFieldError[];
  detail?: string;
  detail_data?: Record<string, unknown>;
}

export function formatValidationDetail(detail: unknown): {
  message: string;
  fieldErrors: ApiFieldError[];
  detailData?: Record<string, unknown>;
} {
  if (typeof detail === 'string' && detail.trim().length > 0) {
    return { message: detail, fieldErrors: [] };
  }

  if (Array.isArray(detail)) {
    const fieldErrors: ApiFieldError[] = [];
    const messages: string[] = [];
    for (const item of detail) {
      if (item && typeof item === 'object') {
        const rec = item as Record<string, unknown>;
        const loc = Array.isArray(rec.loc)
          ? rec.loc.filter((part) => part !== 'body').map(String).join('.')
          : typeof rec.field === 'string'
          ? rec.field
          : 'body';
        const msg =
          typeof rec.msg === 'string'
            ? rec.msg
            : typeof rec.message === 'string'
            ? rec.message
            : JSON.stringify(rec);
        const errType = typeof rec.type === 'string' ? rec.type : 'validation_error';
        fieldErrors.push({ field: loc || 'body', message: msg, type: errType });
        messages.push(loc && loc !== 'body' ? `${loc}: ${msg}` : msg);
      } else if (item !== null && item !== undefined) {
        messages.push(String(item));
      }
    }
    return {
      message: messages.join('; ') || 'Dữ liệu đầu vào không hợp lệ (422)',
      fieldErrors,
    };
  }

  if (detail && typeof detail === 'object') {
    const obj = detail as Record<string, unknown>;
    const msg =
      typeof obj.message === 'string' && obj.message.trim().length > 0
        ? obj.message
        : typeof obj.detail === 'string' && obj.detail.trim().length > 0
        ? obj.detail
        : typeof obj.error_code === 'string'
        ? obj.error_code
        : 'Yêu cầu không hợp lệ';
    const fieldErrors = Array.isArray(obj.field_errors) ? (obj.field_errors as ApiFieldError[]) : [];
    return { message: msg, fieldErrors, detailData: obj };
  }

  return { message: '', fieldErrors: [] };
}

export function normalizeApiErrorPayload(
  data: unknown,
  fallback = 'Đã xảy ra lỗi từ máy chủ'
): ApiErrorEnvelope {
  if (!data || typeof data !== 'object') {
    const text = typeof data === 'string' && data.trim().length > 0 ? data : fallback;
    return { code: 'API_ERROR', message: text, field_errors: [], detail: text };
  }

  const record = data as Record<string, unknown>;
  const { message: detailMessage, fieldErrors, detailData } = formatValidationDetail(record.detail);
  const topMessage = typeof record.message === 'string' && record.message.trim().length > 0 ? record.message : '';
  const resolvedMessage = topMessage || detailMessage || fallback;
  const resolvedFieldErrors = Array.isArray(record.field_errors)
    ? (record.field_errors as ApiFieldError[])
    : fieldErrors;
  const resolvedCode =
    typeof record.code === 'string'
      ? record.code
      : typeof detailData?.error_code === 'string'
      ? String(detailData.error_code)
      : 'API_ERROR';

  record.code = resolvedCode;
  record.message = resolvedMessage;
  record.field_errors = resolvedFieldErrors;
  if (detailData) {
    record.detail_data = detailData;
  }
  record.detail = resolvedMessage;

  return {
    code: resolvedCode,
    message: resolvedMessage,
    field_errors: resolvedFieldErrors,
    detail: resolvedMessage,
    detail_data: detailData,
  };
}

export function extractApiErrorMessage(error: unknown, fallback = 'Đã xảy ra lỗi không xác định'): string {
  if (!error) return fallback;
  if (typeof error === 'string') return error;
  if (typeof error === 'object') {
    const errObj = error as {
      response?: { data?: unknown };
      message?: unknown;
    };
    if (errObj.response?.data !== undefined) {
      const normalized = normalizeApiErrorPayload(errObj.response.data, '');
      if (normalized.message) {
        return normalized.message;
      }
    }
    if (
      typeof errObj.message === 'string' &&
      errObj.message.trim().length > 0 &&
      errObj.message !== '[object Object]'
    ) {
      return errObj.message;
    }
  }
  return fallback;
}

export const apiClient = axios.create({
  baseURL: '/api',
  headers: {
    'Content-Type': 'application/json',
  },
});

apiClient.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error?.response?.data !== undefined) {
      const normalized = normalizeApiErrorPayload(error.response.data, error.message || 'API request failed');
      error.message = normalized.message || error.message;
    }
    if (!axios.isCancel(error) && (!error.response || error.response.status >= 500)) {
      console.error('API Error:', error);
    }
    return Promise.reject(error);
  }
);
