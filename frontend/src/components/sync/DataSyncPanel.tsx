import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getSyncProviders,
  testProviderConnection,
  generateSyncPreview,
  executeSync,
  rollbackSync,
  getSyncHistory,
  type ProviderMetadata,
  type SyncPreviewResponse,
  type SyncManifest,
} from '../../api/syncApi';
import toast from 'react-hot-toast';

interface ApiError {
  response?: {
    data?: {
      detail?: string;
    };
  };
}

export const DataSyncPanel: React.FC = () => {
  const queryClient = useQueryClient();

  // State
  const [selectedProvider, setSelectedProvider] = useState<string>('ssi');
  const [consumerId, setConsumerId] = useState<string>('');
  const [consumerSecret, setConsumerSecret] = useState<string>('');
  const [symbol, setSymbol] = useState<string>('VNM');
  const [startDate, setStartDate] = useState<string>(() => {
    const d = new Date();
    d.setDate(d.getDate() - 30);
    return d.toISOString().split('T')[0];
  });
  const [endDate, setEndDate] = useState<string>(() => {
    return new Date().toISOString().split('T')[0];
  });
  const [adjustmentType, setAdjustmentType] = useState<string>('unadjusted');
  const [preview, setPreview] = useState<SyncPreviewResponse | null>(null);
  const [itemFilter, setItemFilter] = useState<string>('all');
  const [testResult, setTestResult] = useState<{ success: boolean; message: string; latency?: number | null } | null>(null);
  const [selectedManifest, setSelectedManifest] = useState<SyncManifest | null>(null);

  // Queries
  const providersQuery = useQuery<ProviderMetadata[]>({
    queryKey: ['syncProviders'],
    queryFn: getSyncProviders,
  });

  const historyQuery = useQuery<SyncManifest[]>({
    queryKey: ['syncHistory'],
    queryFn: () => getSyncHistory(50),
  });

  // Mutations
  const testConnMutation = useMutation({
    mutationFn: () => {
      const creds: Record<string, string> = {};
      if (consumerId) creds.consumer_id = consumerId;
      if (consumerSecret) creds.consumer_secret = consumerSecret;
      return testProviderConnection(selectedProvider, Object.keys(creds).length > 0 ? creds : undefined);
    },
    onSuccess: (data) => {
      setTestResult({
        success: data.success,
        message: data.message,
        latency: data.latency_ms,
      });
      if (data.success) {
        toast.success(data.message);
      } else {
        toast.error(data.message);
      }
    },
    onError: (err: ApiError) => {
      const msg = err?.response?.data?.detail || 'Lỗi khi kiểm tra kết nối.';
      setTestResult({ success: false, message: msg });
      toast.error(msg);
    },
  });

  const previewMutation = useMutation({
    mutationFn: () => {
      const creds: Record<string, string> = {};
      if (consumerId) creds.consumer_id = consumerId;
      if (consumerSecret) creds.consumer_secret = consumerSecret;
      return generateSyncPreview(
        symbol,
        startDate,
        endDate,
        selectedProvider,
        adjustmentType,
        Object.keys(creds).length > 0 ? creds : undefined
      );
    },
    onSuccess: (data) => {
      setPreview(data);
      if (data.can_accept) {
        toast.success(`Bản xem trước sẵn sàng: ${data.parsed_count} nến mới cần đồng bộ.`);
      } else {
        toast.error(data.block_reason || 'Bản xem trước bị từ chối.');
      }
    },
    onError: (err: ApiError) => {
      const msg = err?.response?.data?.detail || 'Không thể tạo bản xem trước.';
      toast.error(msg);
    },
  });

  const executeMutation = useMutation({
    mutationFn: () => {
      if (!preview) throw new Error('Không có bản xem trước.');
      return executeSync(preview.sync_id, preview.content_sha256);
    },
    onSuccess: (data) => {
      toast.success(data.message);
      setPreview(null);
      queryClient.invalidateQueries({ queryKey: ['syncHistory'] });
      queryClient.invalidateQueries({ queryKey: ['dataCatalog'] });
    },
    onError: (err: ApiError) => {
      const msg = err?.response?.data?.detail || 'Lỗi khi thực hiện đồng bộ.';
      toast.error(msg);
    },
  });

  const rollbackMutation = useMutation({
    mutationFn: (syncId: string) => rollbackSync(syncId),
    onSuccess: (data) => {
      toast.success(data.message);
      queryClient.invalidateQueries({ queryKey: ['syncHistory'] });
      queryClient.invalidateQueries({ queryKey: ['dataCatalog'] });
    },
    onError: (err: ApiError) => {
      const msg = err?.response?.data?.detail || 'Không thể hoàn tác lượt đồng bộ.';
      toast.error(msg);
    },
  });

  // Preset Date Range Handler
  const handlePresetDate = (days: number) => {
    const end = new Date();
    const start = new Date();
    start.setDate(end.getDate() - days);
    setStartDate(start.toISOString().split('T')[0]);
    setEndDate(end.toISOString().split('T')[0]);
  };

  const handleYtdDate = () => {
    const end = new Date();
    const start = new Date(end.getFullYear(), 0, 1);
    setStartDate(start.toISOString().split('T')[0]);
    setEndDate(end.toISOString().split('T')[0]);
  };

  const currentProviderMeta = (providersQuery.data || []).find(
    (p) => p.provider_id === selectedProvider
  );

  const filteredItems = (preview?.items || []).filter((item) => {
    if (itemFilter === 'all') return true;
    return item.classification === itemFilter;
  });

  return (
    <div className="data-sync-panel" style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
      {/* Header & Status Card */}
      <div
        className="glass-panel"
        style={{
          padding: '1.25rem',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          borderLeft: '4px solid var(--color-primary)',
        }}
      >
        <div>
          <h3 style={{ margin: 0, display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <span>🔄</span> Đồng bộ Dữ liệu Trực tuyến (One-Click Sync)
          </h3>
          <p style={{ margin: '0.25rem 0 0 0', color: 'var(--text-muted)', fontSize: '13px' }}>
            Kết nối trực tiếp qua Provider Boundary Adapter. Dữ liệu nến được kiểm tra xung đột, lưu trữ cục bộ và tự động tổng hợp nến tuần (1W).
          </p>
        </div>
        <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
          <span className="badge" style={{ background: 'rgba(38, 166, 154, 0.2)', color: '#26a69a', padding: '4px 8px', borderRadius: '4px', fontSize: '12px' }}>
            🔒 Local-First Privacy
          </span>
          <span className="badge" style={{ background: 'rgba(88, 166, 255, 0.2)', color: '#58a6ff', padding: '4px 8px', borderRadius: '4px', fontSize: '12px' }}>
            🛡️ Fail-Closed Rollback
          </span>
        </div>
      </div>

      {/* Sync Configuration Form */}
      <div className="glass-panel" style={{ padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
        <h4 style={{ margin: 0, fontSize: '15px', color: 'var(--text-main)' }}>1. Thiết lập Nguồn & Thông số Đồng bộ</h4>
        
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))', gap: '1rem' }}>
          {/* Provider Selection */}
          <div>
            <label style={{ display: 'block', fontSize: '12px', fontWeight: 600, color: 'var(--text-muted)', marginBottom: '0.25rem' }}>
              Nhà cung cấp dữ liệu (Provider)
            </label>
            <select
              id="sync-provider-select"
              value={selectedProvider}
              onChange={(e) => setSelectedProvider(e.target.value)}
              style={{
                width: '100%',
                padding: '8px 12px',
                background: 'var(--bg-input, #1c2128)',
                color: 'var(--text-main)',
                border: '1px solid var(--border-color)',
                borderRadius: '6px',
              }}
            >
              {(providersQuery.data || []).map((p) => (
                <option key={p.provider_id} value={p.provider_id}>
                  {p.display_name} {p.is_official ? '⭐ (Chính thức)' : '🌐 (Cộng đồng)'}
                </option>
              ))}
            </select>
          </div>

          {/* Symbol Input & Presets */}
          <div>
            <label style={{ display: 'block', fontSize: '12px', fontWeight: 600, color: 'var(--text-muted)', marginBottom: '0.25rem' }}>
              Mã chứng khoán / Chỉ số
            </label>
            <input
              id="sync-symbol-input"
              type="text"
              value={symbol}
              onChange={(e) => setSymbol(e.target.value.toUpperCase())}
              placeholder="VD: VNM, FPT, HPG, VNINDEX"
              style={{
                width: '100%',
                padding: '8px 12px',
                background: 'var(--bg-input, #1c2128)',
                color: 'var(--text-main)',
                border: '1px solid var(--border-color)',
                borderRadius: '6px',
                fontWeight: 700,
              }}
            />
            <div style={{ display: 'flex', gap: '0.25rem', marginTop: '0.35rem', flexWrap: 'wrap' }}>
              {['VNM', 'FPT', 'HPG', 'SSI', 'MWG', 'VNINDEX'].map((s) => (
                <button
                  key={s}
                  type="button"
                  onClick={() => setSymbol(s)}
                  style={{
                    padding: '2px 6px',
                    fontSize: '11px',
                    background: symbol === s ? 'var(--color-primary)' : 'rgba(255,255,255,0.06)',
                    color: symbol === s ? '#fff' : 'var(--text-muted)',
                    border: 'none',
                    borderRadius: '4px',
                    cursor: 'pointer',
                  }}
                >
                  {s}
                </button>
              ))}
            </div>
          </div>

          {/* Date Range Start & End */}
          <div>
            <label style={{ display: 'block', fontSize: '12px', fontWeight: 600, color: 'var(--text-muted)', marginBottom: '0.25rem' }}>
              Khoảng thời gian (Từ ngày - Đến ngày)
            </label>
            <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
              <input
                id="sync-start-date"
                type="date"
                value={startDate}
                onChange={(e) => setStartDate(e.target.value)}
                style={{
                  flex: 1,
                  padding: '7px 8px',
                  background: 'var(--bg-input, #1c2128)',
                  color: 'var(--text-main)',
                  border: '1px solid var(--border-color)',
                  borderRadius: '6px',
                  fontSize: '13px',
                }}
              />
              <span style={{ color: 'var(--text-muted)' }}>-</span>
              <input
                id="sync-end-date"
                type="date"
                value={endDate}
                onChange={(e) => setEndDate(e.target.value)}
                style={{
                  flex: 1,
                  padding: '7px 8px',
                  background: 'var(--bg-input, #1c2128)',
                  color: 'var(--text-main)',
                  border: '1px solid var(--border-color)',
                  borderRadius: '6px',
                  fontSize: '13px',
                }}
              />
            </div>
            <div style={{ display: 'flex', gap: '0.25rem', marginTop: '0.35rem', flexWrap: 'wrap' }}>
              <button
                type="button"
                onClick={() => handlePresetDate(30)}
                style={{ padding: '2px 6px', fontSize: '11px', background: 'rgba(255,255,255,0.06)', color: 'var(--text-muted)', border: 'none', borderRadius: '4px', cursor: 'pointer' }}
              >
                30 Ngày
              </button>
              <button
                type="button"
                onClick={() => handlePresetDate(90)}
                style={{ padding: '2px 6px', fontSize: '11px', background: 'rgba(255,255,255,0.06)', color: 'var(--text-muted)', border: 'none', borderRadius: '4px', cursor: 'pointer' }}
              >
                90 Ngày
              </button>
              <button
                type="button"
                onClick={handleYtdDate}
                style={{ padding: '2px 6px', fontSize: '11px', background: 'rgba(255,255,255,0.06)', color: 'var(--text-muted)', border: 'none', borderRadius: '4px', cursor: 'pointer' }}
              >
                YTD
              </button>
              <button
                type="button"
                onClick={() => handlePresetDate(365)}
                style={{ padding: '2px 6px', fontSize: '11px', background: 'rgba(255,255,255,0.06)', color: 'var(--text-muted)', border: 'none', borderRadius: '4px', cursor: 'pointer' }}
              >
                1 Năm
              </button>
            </div>
          </div>

          {/* Adjustment Type */}
          <div>
            <label style={{ display: 'block', fontSize: '12px', fontWeight: 600, color: 'var(--text-muted)', marginBottom: '0.25rem' }}>
              Loại điều chỉnh giá
            </label>
            <select
              id="sync-adjustment-select"
              value={adjustmentType}
              onChange={(e) => setAdjustmentType(e.target.value)}
              style={{
                width: '100%',
                padding: '8px 12px',
                background: 'var(--bg-input, #1c2128)',
                color: 'var(--text-main)',
                border: '1px solid var(--border-color)',
                borderRadius: '6px',
              }}
            >
              <option value="unadjusted">unadjusted (Giá gốc giao dịch thực tế)</option>
              <option value="adjusted">adjusted (Giá đã điều chỉnh cổ tức/chia tách)</option>
            </select>
          </div>
        </div>

        {/* Credentials / API Auth Inputs (if provider requires auth) */}
        {currentProviderMeta?.requires_auth && (
          <div
            style={{
              padding: '0.75rem',
              background: 'rgba(0,0,0,0.15)',
              borderRadius: '6px',
              border: '1px solid rgba(255,255,255,0.05)',
              display: 'flex',
              flexDirection: 'column',
              gap: '0.75rem',
            }}
          >
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <span style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-muted)' }}>
                🔑 Xác thực SSI FastConnect (Tùy chọn nếu đã cấu hình biến môi trường)
              </span>
              <button
                id="sync-test-connection-btn"
                type="button"
                onClick={() => testConnMutation.mutate()}
                disabled={testConnMutation.isPending}
                style={{
                  padding: '4px 10px',
                  fontSize: '12px',
                  background: 'rgba(88, 166, 255, 0.15)',
                  color: '#58a6ff',
                  border: '1px solid rgba(88, 166, 255, 0.3)',
                  borderRadius: '4px',
                  cursor: 'pointer',
                  fontWeight: 600,
                }}
              >
                {testConnMutation.isPending ? 'Đang kiểm tra...' : '⚡ Kiểm tra kết nối'}
              </button>
            </div>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '0.75rem' }}>
              <input
                id="sync-consumer-id"
                type="text"
                value={consumerId}
                onChange={(e) => setConsumerId(e.target.value)}
                placeholder="Consumer ID (VD: c7a89b...)"
                style={{
                  padding: '6px 10px',
                  background: 'var(--bg-input, #1c2128)',
                  color: 'var(--text-main)',
                  border: '1px solid var(--border-color)',
                  borderRadius: '4px',
                  fontSize: '12px',
                }}
              />
              <input
                id="sync-consumer-secret"
                type="password"
                value={consumerSecret}
                onChange={(e) => setConsumerSecret(e.target.value)}
                placeholder="Consumer Secret / Private Key"
                style={{
                  padding: '6px 10px',
                  background: 'var(--bg-input, #1c2128)',
                  color: 'var(--text-main)',
                  border: '1px solid var(--border-color)',
                  borderRadius: '4px',
                  fontSize: '12px',
                }}
              />
            </div>

            {testResult && (
              <div
                style={{
                  padding: '6px 10px',
                  borderRadius: '4px',
                  fontSize: '12px',
                  background: testResult.success ? 'rgba(38, 166, 154, 0.15)' : 'rgba(239, 83, 80, 0.15)',
                  color: testResult.success ? '#26a69a' : '#ef5350',
                  border: `1px solid ${testResult.success ? 'rgba(38, 166, 154, 0.3)' : 'rgba(239, 83, 80, 0.3)'}`,
                }}
              >
                {testResult.message}
              </div>
            )}
          </div>
        )}

        {/* Action Button: Generate Preview */}
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.75rem' }}>
          <button
            id="sync-preview-btn"
            type="button"
            onClick={() => previewMutation.mutate()}
            disabled={previewMutation.isPending || !symbol}
            style={{
              padding: '10px 20px',
              background: 'var(--color-primary, #238636)',
              color: '#fff',
              border: 'none',
              borderRadius: '6px',
              fontWeight: 700,
              cursor: previewMutation.isPending ? 'not-allowed' : 'pointer',
              display: 'flex',
              alignItems: 'center',
              gap: '0.5rem',
            }}
          >
            {previewMutation.isPending ? '⏳ Đang tải dữ liệu...' : '🔍 Xem trước dữ liệu (Dry-Run Preview)'}
          </button>
        </div>
      </div>

      {/* Pre-commit Preview & Diff Section */}
      {preview && (
        <div
          className="glass-panel"
          style={{
            padding: '1.5rem',
            display: 'flex',
            flexDirection: 'column',
            gap: '1rem',
            border: preview.can_accept ? '1px solid rgba(38, 166, 154, 0.4)' : '1px solid rgba(239, 83, 80, 0.4)',
          }}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <div>
              <h4 style={{ margin: 0, fontSize: '15px' }}>
                2. Kết quả Phân loại & Kiểm duyệt Trước khi Ghi ({preview.symbol} | {preview.start_date} → {preview.end_date})
              </h4>
              <p style={{ margin: '0.25rem 0 0 0', color: 'var(--text-muted)', fontSize: '12px' }}>
                Checksum: <code style={{ color: '#58a6ff' }}>{preview.content_sha256.substring(0, 16)}...</code> |
                Trạng thái: <strong>{preview.can_accept ? '✅ Hợp lệ để đồng bộ' : '❌ Bị chặn'}</strong>
              </p>
            </div>
            <div style={{ display: 'flex', gap: '0.5rem' }}>
              <button
                id="sync-cancel-preview-btn"
                type="button"
                onClick={() => setPreview(null)}
                style={{
                  padding: '6px 12px',
                  background: 'rgba(255,255,255,0.08)',
                  color: 'var(--text-muted)',
                  border: '1px solid var(--border-color)',
                  borderRadius: '4px',
                  cursor: 'pointer',
                  fontSize: '13px',
                }}
              >
                Hủy bỏ
              </button>
              <button
                id="sync-execute-confirm-btn"
                type="button"
                onClick={() => executeMutation.mutate()}
                disabled={!preview.can_accept || executeMutation.isPending}
                style={{
                  padding: '6px 16px',
                  background: preview.can_accept ? 'var(--color-primary, #238636)' : '#444',
                  color: '#fff',
                  border: 'none',
                  borderRadius: '4px',
                  cursor: preview.can_accept && !executeMutation.isPending ? 'pointer' : 'not-allowed',
                  fontWeight: 700,
                  fontSize: '13px',
                }}
              >
                {executeMutation.isPending ? '⏳ Đang lưu...' : '✅ Xác nhận & Đồng bộ vào CSDL'}
              </button>
            </div>
          </div>

          {/* Counts Summary Bar */}
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(5, 1fr)', gap: '0.75rem' }}>
            <div style={{ padding: '8px', background: 'rgba(255,255,255,0.03)', borderRadius: '4px', textAlign: 'center' }}>
              <div style={{ fontSize: '18px', fontWeight: 700, color: 'var(--text-main)' }}>{preview.items.length}</div>
              <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Tổng nến nhận</div>
            </div>
            <div style={{ padding: '8px', background: 'rgba(38, 166, 154, 0.1)', borderRadius: '4px', textAlign: 'center' }}>
              <div id="sync-count-parsed" style={{ fontSize: '18px', fontWeight: 700, color: '#26a69a' }}>{preview.parsed_count}</div>
              <div style={{ fontSize: '11px', color: '#26a69a' }}>Nến mới (Sẽ ghi)</div>
            </div>
            <div style={{ padding: '8px', background: 'rgba(88, 166, 255, 0.1)', borderRadius: '4px', textAlign: 'center' }}>
              <div id="sync-count-duplicate" style={{ fontSize: '18px', fontWeight: 700, color: '#58a6ff' }}>{preview.duplicate_count}</div>
              <div style={{ fontSize: '11px', color: '#58a6ff' }}>Trùng lặp (Bỏ qua)</div>
            </div>
            <div style={{ padding: '8px', background: 'rgba(255, 209, 102, 0.1)', borderRadius: '4px', textAlign: 'center' }}>
              <div id="sync-count-conflicting" style={{ fontSize: '18px', fontWeight: 700, color: '#ffd166' }}>{preview.conflicting_count}</div>
              <div style={{ fontSize: '11px', color: '#ffd166' }}>Xung đột giá</div>
            </div>
            <div style={{ padding: '8px', background: 'rgba(239, 83, 80, 0.1)', borderRadius: '4px', textAlign: 'center' }}>
              <div id="sync-count-rejected" style={{ fontSize: '18px', fontWeight: 700, color: '#ef5350' }}>{preview.rejected_count}</div>
              <div style={{ fontSize: '11px', color: '#ef5350' }}>Không hợp lệ</div>
            </div>
          </div>

          {preview.block_reason && (
            <div style={{ padding: '8px 12px', background: 'rgba(239, 83, 80, 0.15)', color: '#ef5350', borderRadius: '4px', fontSize: '13px' }}>
              ⚠️ Lý do chặn: {preview.block_reason}
            </div>
          )}

          {/* Filter Pills */}
          <div style={{ display: 'flex', gap: '0.5rem', borderBottom: '1px solid var(--border-color)', paddingBottom: '0.5rem' }}>
            {['all', 'parsed', 'duplicate', 'conflicting', 'rejected'].map((f) => (
              <button
                key={f}
                type="button"
                onClick={() => setItemFilter(f)}
                style={{
                  padding: '3px 8px',
                  fontSize: '11px',
                  background: itemFilter === f ? 'var(--color-primary)' : 'none',
                  color: itemFilter === f ? '#fff' : 'var(--text-muted)',
                  border: 'none',
                  borderRadius: '4px',
                  cursor: 'pointer',
                  fontWeight: itemFilter === f ? 700 : 500,
                }}
              >
                {f === 'all' && `Tất cả (${preview.items.length})`}
                {f === 'parsed' && `Nến mới (${preview.parsed_count})`}
                {f === 'duplicate' && `Trùng lặp (${preview.duplicate_count})`}
                {f === 'conflicting' && `Xung đột (${preview.conflicting_count})`}
                {f === 'rejected' && `Bị từ chối (${preview.rejected_count})`}
              </button>
            ))}
          </div>

          {/* Preview Items Table */}
          <div style={{ maxHeight: '240px', overflowY: 'auto', border: '1px solid var(--border-color)', borderRadius: '4px' }}>
            <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px', textAlign: 'left' }}>
              <thead>
                <tr style={{ background: 'rgba(255,255,255,0.04)', color: 'var(--text-muted)' }}>
                  <th style={{ padding: '6px 10px' }}>STT</th>
                  <th style={{ padding: '6px 10px' }}>Ngày</th>
                  <th style={{ padding: '6px 10px' }}>Mở</th>
                  <th style={{ padding: '6px 10px' }}>Cao</th>
                  <th style={{ padding: '6px 10px' }}>Thấp</th>
                  <th style={{ padding: '6px 10px' }}>Đóng</th>
                  <th style={{ padding: '6px 10px' }}>Khối lượng</th>
                  <th style={{ padding: '6px 10px' }}>Phân loại</th>
                </tr>
              </thead>
              <tbody>
                {filteredItems.map((it) => (
                  <tr key={it.row_index} style={{ borderBottom: '1px solid rgba(255,255,255,0.03)' }}>
                    <td style={{ padding: '6px 10px' }}>{it.row_index}</td>
                    <td style={{ padding: '6px 10px', fontWeight: 600 }}>{it.timestamp}</td>
                    <td style={{ padding: '6px 10px' }}>{it.open?.toLocaleString()}</td>
                    <td style={{ padding: '6px 10px' }}>{it.high?.toLocaleString()}</td>
                    <td style={{ padding: '6px 10px' }}>{it.low?.toLocaleString()}</td>
                    <td style={{ padding: '6px 10px', fontWeight: 700 }}>{it.close?.toLocaleString()}</td>
                    <td style={{ padding: '6px 10px' }}>{it.volume?.toLocaleString()}</td>
                    <td style={{ padding: '6px 10px' }}>
                      <span
                        style={{
                          padding: '2px 6px',
                          borderRadius: '3px',
                          fontSize: '11px',
                          fontWeight: 600,
                          background:
                            it.classification === 'parsed'
                              ? 'rgba(38, 166, 154, 0.2)'
                              : it.classification === 'duplicate'
                              ? 'rgba(88, 166, 255, 0.2)'
                              : it.classification === 'conflicting'
                              ? 'rgba(255, 209, 102, 0.2)'
                              : 'rgba(239, 83, 80, 0.2)',
                          color:
                            it.classification === 'parsed'
                              ? '#26a69a'
                              : it.classification === 'duplicate'
                              ? '#58a6ff'
                              : it.classification === 'conflicting'
                              ? '#ffd166'
                              : '#ef5350',
                        }}
                      >
                        {it.classification}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Sync History & Immutable Audit Trail */}
      <div className="glass-panel" style={{ padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '1rem' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <h4 style={{ margin: 0, fontSize: '15px' }}>
            3. Nhật ký Đồng bộ & Bản ghi Kiểm toán Bất biến (Immutable Sync Audit Log)
          </h4>
          <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>
            Tổng số lượt: {historyQuery.data?.length || 0}
          </span>
        </div>

        {historyQuery.isLoading ? (
          <div style={{ padding: '1rem', color: 'var(--text-muted)' }}>Đang tải lịch sử...</div>
        ) : (historyQuery.data || []).length === 0 ? (
          <div style={{ padding: '1.5rem', textAlign: 'center', color: 'var(--text-muted)', fontSize: '13px' }}>
            Chưa có lượt đồng bộ trực tuyến nào được thực hiện.
          </div>
        ) : (
          <div style={{ overflowX: 'auto' }}>
            <table id="sync-history-table" style={{ width: '100%', borderCollapse: 'collapse', fontSize: '13px', textAlign: 'left' }}>
              <thead>
                <tr style={{ background: 'rgba(255,255,255,0.04)', color: 'var(--text-muted)' }}>
                  <th style={{ padding: '8px 12px' }}>Thời gian</th>
                  <th style={{ padding: '8px 12px' }}>Nguồn</th>
                  <th style={{ padding: '8px 12px' }}>Mã</th>
                  <th style={{ padding: '8px 12px' }}>Khoảng ngày</th>
                  <th style={{ padding: '8px 12px' }}>Đã ghi</th>
                  <th style={{ padding: '8px 12px' }}>Thời gian xử lý</th>
                  <th style={{ padding: '8px 12px' }}>Trạng thái</th>
                  <th style={{ padding: '8px 12px', textAlign: 'right' }}>Thao tác</th>
                </tr>
              </thead>
              <tbody>
                {(historyQuery.data || []).map((run) => (
                  <tr key={run.sync_id} style={{ borderBottom: '1px solid rgba(255,255,255,0.03)' }}>
                    <td style={{ padding: '8px 12px', color: 'var(--text-muted)' }}>
                      {new Date(run.created_at).toLocaleString('vi-VN')}
                    </td>
                    <td style={{ padding: '8px 12px' }}>
                      <span className="badge" style={{ background: 'rgba(255,255,255,0.08)', padding: '2px 6px', borderRadius: '3px', fontSize: '11px' }}>
                        {run.provider_id.toUpperCase()}
                      </span>
                    </td>
                    <td style={{ padding: '8px 12px', fontWeight: 700, color: 'var(--text-main)' }}>{run.symbol}</td>
                    <td style={{ padding: '8px 12px', fontSize: '12px' }}>
                      {run.start_date} → {run.end_date}
                    </td>
                    <td style={{ padding: '8px 12px', fontWeight: 600, color: '#26a69a' }}>
                      {run.accepted_count} nến
                    </td>
                    <td style={{ padding: '8px 12px', color: 'var(--text-muted)', fontSize: '12px' }}>
                      {run.duration_ms?.toFixed(1) ?? '0.0'} ms
                    </td>
                    <td style={{ padding: '8px 12px' }}>
                      <span
                        className={`badge-status status-${run.status}`}
                        style={{
                          padding: '2px 8px',
                          borderRadius: '4px',
                          fontSize: '11px',
                          fontWeight: 600,
                          background:
                            run.status === 'accepted'
                              ? 'rgba(38, 166, 154, 0.2)'
                              : run.status === 'rolled_back'
                              ? 'rgba(88, 166, 255, 0.2)'
                              : 'rgba(239, 83, 80, 0.2)',
                          color:
                            run.status === 'accepted'
                              ? '#26a69a'
                              : run.status === 'rolled_back'
                              ? '#58a6ff'
                              : '#ef5350',
                        }}
                      >
                        {run.status === 'accepted' ? 'Đã chấp nhận' : run.status === 'rolled_back' ? 'Đã hoàn tác' : run.status}
                      </span>
                    </td>
                    <td style={{ padding: '8px 12px', textAlign: 'right' }}>
                      <div style={{ display: 'flex', gap: '0.5rem', justifyContent: 'flex-end' }}>
                        {run.manifest && (
                          <button
                            type="button"
                            onClick={() => setSelectedManifest(run)}
                            style={{
                              padding: '3px 8px',
                              fontSize: '11px',
                              background: 'rgba(88, 166, 255, 0.1)',
                              color: '#58a6ff',
                              border: '1px solid rgba(88, 166, 255, 0.2)',
                              borderRadius: '3px',
                              cursor: 'pointer',
                            }}
                          >
                            Manifest
                          </button>
                        )}
                        {run.status === 'accepted' && (
                          <button
                            id={`sync-rollback-btn-${run.sync_id}`}
                            type="button"
                            onClick={() => rollbackMutation.mutate(run.sync_id)}
                            disabled={rollbackMutation.isPending}
                            style={{
                              padding: '3px 8px',
                              fontSize: '11px',
                              background: 'rgba(239, 83, 80, 0.15)',
                              color: '#ef5350',
                              border: '1px solid rgba(239, 83, 80, 0.3)',
                              borderRadius: '3px',
                              cursor: 'pointer',
                              fontWeight: 600,
                            }}
                          >
                            ↩️ Hoàn tác
                          </button>
                        )}
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {/* Manifest Detail Modal */}
      {selectedManifest && (
        <div
          style={{
            position: 'fixed',
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: 'rgba(0,0,0,0.7)',
            display: 'flex',
            justifyContent: 'center',
            alignItems: 'center',
            zIndex: 1000,
          }}
        >
          <div
            className="glass-panel"
            style={{
              width: '90%',
              maxWidth: '650px',
              maxHeight: '85vh',
              overflowY: 'auto',
              padding: '1.5rem',
              display: 'flex',
              flexDirection: 'column',
              gap: '1rem',
            }}
          >
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <h4 style={{ margin: 0 }}>📜 Chi tiết Bản ghi Kiểm toán (Audit Manifest)</h4>
              <button
                type="button"
                onClick={() => setSelectedManifest(null)}
                style={{
                  background: 'none',
                  border: 'none',
                  color: 'var(--text-muted)',
                  fontSize: '18px',
                  cursor: 'pointer',
                }}
              >
                ✕
              </button>
            </div>
            <pre
              style={{
                background: '#0d1117',
                padding: '1rem',
                borderRadius: '6px',
                fontSize: '12px',
                color: '#58a6ff',
                overflowX: 'auto',
              }}
            >
              {JSON.stringify(selectedManifest.manifest, null, 2)}
            </pre>
            <button
              type="button"
              onClick={() => setSelectedManifest(null)}
              style={{
                alignSelf: 'flex-end',
                padding: '6px 16px',
                background: 'var(--color-primary)',
                color: '#fff',
                border: 'none',
                borderRadius: '4px',
                cursor: 'pointer',
              }}
            >
              Đóng
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
