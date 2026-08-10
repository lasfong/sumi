import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  previewImportData,
  acceptImportRun,
  rollbackImportRun,
  getDataCatalog,
  getImportRuns,
  type ImportPreviewResponse,
  type CatalogItem,
  type ImportRunSummary
} from '../api/importApi';
import toast from 'react-hot-toast';

interface ApiError {
  response?: {
    data?: {
      detail?: string;
    };
  };
}

export const ImportPage: React.FC = () => {
  const queryClient = useQueryClient();
  const [activeTab, setActiveTab] = useState<'import' | 'catalog' | 'history'>('import');
  const [file, setFile] = useState<File | null>(null);
  const [adjustmentType, setAdjustmentType] = useState<string>('unadjusted');
  const [preview, setPreview] = useState<ImportPreviewResponse | null>(null);
  const [itemFilter, setItemFilter] = useState<string>('all');
  const [error, setError] = useState<string | null>(null);

  // Data Catalog query
  const catalogQuery = useQuery<CatalogItem[]>({
    queryKey: ['dataCatalog'],
    queryFn: getDataCatalog,
  });

  // Import Runs History query
  const historyQuery = useQuery<ImportRunSummary[]>({
    queryKey: ['importRuns'],
    queryFn: getImportRuns,
  });

  // Preview mutation
  const previewMutation = useMutation({
    mutationFn: (fileObj: File) => previewImportData(fileObj, adjustmentType),
    onSuccess: (data) => {
      setPreview(data);
      setError(null);
      if (data.can_accept) {
        toast.success('Bản xem trước đã sẵn sàng! Vui lòng kiểm tra và xác nhận.');
      } else {
        toast.error(data.block_reason || 'Tập tin bị từ chối nhập.');
      }
    },
    onError: (err: ApiError) => {
      const errMsg = err?.response?.data?.detail || 'Không thể xem trước dữ liệu.';
      setError(errMsg);
      setPreview(null);
      toast.error(errMsg);
    },
  });

  // Accept mutation
  const acceptMutation = useMutation({
    mutationFn: ({ runId, checksum }: { runId: string; checksum: string }) => acceptImportRun(runId, checksum),
    onSuccess: (data) => {
      toast.success(data.message);
      setPreview(null);
      setFile(null);
      queryClient.invalidateQueries({ queryKey: ['dataCatalog'] });
      queryClient.invalidateQueries({ queryKey: ['importRuns'] });
    },
    onError: (err: ApiError) => {
      const errMsg = err?.response?.data?.detail || 'Không thể chấp nhận nhập dữ liệu.';
      toast.error(errMsg);
    },
  });

  // Rollback mutation
  const rollbackMutation = useMutation({
    mutationFn: (runId: string) => rollbackImportRun(runId),
    onSuccess: (data) => {
      toast.success(data.message);
      queryClient.invalidateQueries({ queryKey: ['dataCatalog'] });
      queryClient.invalidateQueries({ queryKey: ['importRuns'] });
    },
    onError: (err: ApiError) => {
      const errMsg = err?.response?.data?.detail || 'Không thể hoàn tác lượt nhập.';
      toast.error(errMsg);
    },
  });

  const handlePreview = () => {
    if (file) {
      previewMutation.mutate(file);
    }
  };

  const handleAccept = () => {
    if (preview && preview.can_accept) {
      acceptMutation.mutate({ runId: preview.run_id, checksum: preview.content_sha256 });
    }
  };

  const filteredItems = (preview?.items || []).filter((item) => {
    if (itemFilter === 'all') return true;
    return item.classification === itemFilter;
  });

  return (
    <div style={{ padding: '2rem', maxWidth: '1100px', margin: '0 auto' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem' }}>
        <div>
          <h2 style={{ margin: 0 }}>📊 Quản lý Danh mục & Nhập dữ liệu Dữ liệu Thị trường</h2>
          <p style={{ color: 'var(--text-muted)', margin: '0.25rem 0 0 0', fontSize: '14px' }}>
            Xem trước, kiểm duyệt và hoàn tác nhập dữ liệu lịch sử chứng khoán Việt Nam (Daily/Weekly).
          </p>
        </div>
      </div>

      {/* Tabs */}
      <div style={{ display: 'flex', gap: '0.5rem', borderBottom: '1px solid var(--border-color)', marginBottom: '1.5rem' }}>
        <button
          onClick={() => setActiveTab('import')}
          style={{
            padding: '8px 16px', background: 'none', border: 'none',
            borderBottom: activeTab === 'import' ? '2px solid var(--color-primary)' : '2px solid transparent',
            color: activeTab === 'import' ? 'var(--color-primary)' : 'var(--text-muted)',
            fontWeight: 700, cursor: 'pointer',
          }}
        >
          📥 Nhập dữ liệu mới
        </button>
        <button
          onClick={() => setActiveTab('catalog')}
          style={{
            padding: '8px 16px', background: 'none', border: 'none',
            borderBottom: activeTab === 'catalog' ? '2px solid var(--color-primary)' : '2px solid transparent',
            color: activeTab === 'catalog' ? 'var(--color-primary)' : 'var(--text-muted)',
            fontWeight: 700, cursor: 'pointer',
          }}
        >
          📚 Danh mục dữ liệu ({catalogQuery.data?.length || 0})
        </button>
        <button
          onClick={() => setActiveTab('history')}
          style={{
            padding: '8px 16px', background: 'none', border: 'none',
            borderBottom: activeTab === 'history' ? '2px solid var(--color-primary)' : '2px solid transparent',
            color: activeTab === 'history' ? 'var(--color-primary)' : 'var(--text-muted)',
            fontWeight: 700, cursor: 'pointer',
          }}
        >
          📜 Lịch sử nhập & Hoàn tác ({historyQuery.data?.length || 0})
        </button>
      </div>

      {/* TAB 1: IMPORT WORKFLOW */}
      {activeTab === 'import' && (
        <div>
          <div className="panel" style={{ marginBottom: '1.5rem' }}>
            <h3 style={{ marginTop: 0, fontSize: '16px' }}>1. Chọn tập tin & Cấu hình loại dữ liệu</h3>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 200px 150px', gap: '1rem', alignItems: 'end' }}>
              <div>
                <label style={{ display: 'block', fontSize: '12px', color: 'var(--text-muted)', marginBottom: '4px' }}>
                  Tập tin dữ liệu CafeF (.csv, .txt, .zip)
                </label>
                <label
                  style={{
                    display: 'block', padding: '0.75rem 1rem', border: '2px dashed var(--border-color)',
                    borderRadius: '6px', textAlign: 'center', cursor: 'pointer',
                    color: file ? 'var(--color-buy)' : 'var(--text-muted)', fontSize: '14px',
                  }}
                >
                  <input
                    type="file"
                    accept=".csv,.txt,.zip"
                    onChange={(e) => setFile(e.target.files?.[0] || null)}
                    style={{ display: 'none' }}
                  />
                  {file ? `📄 ${file.name} (${(file.size / 1024).toFixed(1)} KB)` : '📎 Nhấp để chọn tập tin'}
                </label>
              </div>

              <div>
                <label style={{ display: 'block', fontSize: '12px', color: 'var(--text-muted)', marginBottom: '4px' }}>
                  Loại dữ liệu điều chỉnh
                </label>
                <select
                  value={adjustmentType}
                  onChange={(e) => setAdjustmentType(e.target.value)}
                  style={{ width: '100%', padding: '8px 12px', background: 'var(--bg-card)', color: 'var(--text-main)', border: '1px solid var(--border-color)', borderRadius: '6px' }}
                >
                  <option value="unadjusted">Chưa điều chỉnh (unadjusted)</option>
                  <option value="adjusted">Đã điều chỉnh (adjusted)</option>
                </select>
              </div>

              <button
                onClick={handlePreview}
                disabled={!file || previewMutation.isPending}
                style={{
                  background: 'var(--color-primary)', color: 'white',
                  padding: '10px 16px', fontSize: '14px', fontWeight: 700,
                  height: '40px', borderRadius: '6px',
                }}
              >
                {previewMutation.isPending ? '⏳ Đang kiểm tra...' : '🔍 Xem trước'}
              </button>
            </div>
          </div>

          {error && (
            <div className="panel" style={{ borderColor: 'var(--color-sell)', marginBottom: '1.5rem' }}>
              <p style={{ color: 'var(--color-sell)', margin: 0 }}>❌ {error}</p>
            </div>
          )}

          {/* PREVIEW RESULTS */}
          {preview && (
            <div className="panel">
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
                <h3 style={{ margin: 0 }}>2. Kết quả kiểm duyệt bản xem trước</h3>
                <span
                  style={{
                    padding: '4px 12px', borderRadius: '4px', fontSize: '12px', fontWeight: 700,
                    background: preview.can_accept ? 'rgba(38, 166, 154, 0.15)' : 'rgba(239, 83, 80, 0.15)',
                    color: preview.can_accept ? 'var(--color-buy)' : 'var(--color-sell)',
                  }}
                >
                  {preview.can_accept ? '✅ Đủ điều kiện nhập' : '❌ Bị chặn (Chứa lỗi/Xung đột)'}
                </span>
              </div>

              {/* Counts metrics */}
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(6, 1fr)', gap: '0.5rem', marginBottom: '1.5rem' }}>
                <div className="card" style={{ padding: '10px', textAlign: 'center' }}>
                  <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Hợp lệ</div>
                  <div style={{ fontSize: '18px', fontWeight: 700, color: 'var(--color-buy)' }}>{preview.parsed_count}</div>
                </div>
                <div className="card" style={{ padding: '10px', textAlign: 'center' }}>
                  <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Sai định dạng</div>
                  <div style={{ fontSize: '18px', fontWeight: 700, color: preview.rejected_count > 0 ? 'var(--color-sell)' : 'var(--text-main)' }}>{preview.rejected_count}</div>
                </div>
                <div className="card" style={{ padding: '10px', textAlign: 'center' }}>
                  <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Trùng lặp</div>
                  <div style={{ fontSize: '18px', fontWeight: 700, color: 'var(--text-muted)' }}>{preview.duplicate_count}</div>
                </div>
                <div className="card" style={{ padding: '10px', textAlign: 'center' }}>
                  <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Xung đột</div>
                  <div style={{ fontSize: '18px', fontWeight: 700, color: preview.conflicting_count > 0 ? 'var(--color-sell)' : 'var(--text-main)' }}>{preview.conflicting_count}</div>
                </div>
                <div className="card" style={{ padding: '10px', textAlign: 'center' }}>
                  <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Sai thứ tự</div>
                  <div style={{ fontSize: '18px', fontWeight: 700, color: preview.out_of_order_count > 0 ? 'var(--color-sell)' : 'var(--text-main)' }}>{preview.out_of_order_count}</div>
                </div>
                <div className="card" style={{ padding: '10px', textAlign: 'center' }}>
                  <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Thiếu ngày (Gaps)</div>
                  <div style={{ fontSize: '18px', fontWeight: 700, color: 'var(--color-close)' }}>{preview.missing_count}</div>
                </div>
              </div>

              {preview.block_reason && (
                <div style={{ padding: '10px', background: 'rgba(239, 83, 80, 0.1)', border: '1px solid var(--color-sell)', borderRadius: '6px', marginBottom: '1rem', fontSize: '13px', color: 'var(--color-sell)' }}>
                  ⚠️ <strong>Lý do từ chối:</strong> {preview.block_reason}
                </div>
              )}

              {/* Item classification filter & table */}
              <div style={{ marginBottom: '1rem' }}>
                <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '0.5rem' }}>
                  {['all', 'parsed', 'rejected', 'conflicting', 'duplicate', 'missing', 'out_of_order'].map((cat) => (
                    <button
                      key={cat}
                      onClick={() => setItemFilter(cat)}
                      style={{
                        padding: '4px 8px', fontSize: '12px', borderRadius: '4px',
                        background: itemFilter === cat ? 'var(--color-primary)' : 'var(--bg-card)',
                        color: itemFilter === cat ? 'white' : 'var(--text-muted)',
                        border: '1px solid var(--border-color)', cursor: 'pointer'
                      }}
                    >
                      {cat.toUpperCase()}
                    </button>
                  ))}
                </div>

                <div style={{ maxHeight: '250px', overflowY: 'auto', border: '1px solid var(--border-color)', borderRadius: '6px' }}>
                  <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px' }}>
                    <thead>
                      <tr style={{ background: 'var(--bg-card)', borderBottom: '1px solid var(--border-color)', textAlign: 'left' }}>
                        <th style={{ padding: '6px 10px' }}>Dòng</th>
                        <th style={{ padding: '6px 10px' }}>Mã</th>
                        <th style={{ padding: '6px 10px' }}>Ngày</th>
                        <th style={{ padding: '6px 10px' }}>Giá Open/High/Low/Close</th>
                        <th style={{ padding: '6px 10px' }}>KL</th>
                        <th style={{ padding: '6px 10px' }}>Phân loại</th>
                        <th style={{ padding: '6px 10px' }}>Ghi chú</th>
                      </tr>
                    </thead>
                    <tbody>
                      {filteredItems.slice(0, 50).map((it, idx) => (
                        <tr key={idx} style={{ borderBottom: '1px solid var(--border-color)' }}>
                          <td style={{ padding: '6px 10px', color: 'var(--text-muted)' }}>{it.row_index || '—'}</td>
                          <td style={{ padding: '6px 10px', fontWeight: 700 }}>{it.symbol}</td>
                          <td style={{ padding: '6px 10px' }}>{it.timestamp}</td>
                          <td style={{ padding: '6px 10px' }}>
                            {it.open != null ? `${it.open} / ${it.high} / ${it.low} / ${it.close}` : '—'}
                          </td>
                          <td style={{ padding: '6px 10px' }}>{it.volume != null ? it.volume.toLocaleString() : '—'}</td>
                          <td style={{ padding: '6px 10px' }}>
                            <span
                              style={{
                                padding: '2px 6px', borderRadius: '4px', fontSize: '10px', fontWeight: 700,
                                background: it.classification === 'parsed' ? 'rgba(38, 166, 154, 0.15)' :
                                            it.classification === 'rejected' ? 'rgba(239, 83, 80, 0.15)' : 'rgba(255, 255, 255, 0.1)',
                                color: it.classification === 'parsed' ? 'var(--color-buy)' :
                                       it.classification === 'rejected' ? 'var(--color-sell)' : 'var(--text-muted)'
                              }}
                            >
                              {it.classification}
                            </span>
                          </td>
                          <td style={{ padding: '6px 10px', color: 'var(--text-muted)' }}>{it.reject_reason || '—'}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                  {filteredItems.length > 50 && (
                    <div style={{ padding: '6px', textAlign: 'center', fontSize: '11px', color: 'var(--text-muted)' }}>
                      Hiển thị 50 / {filteredItems.length} dòng dữ liệu...
                    </div>
                  )}
                </div>
              </div>

              {/* Action */}
              <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '1rem', marginTop: '1.5rem' }}>
                <button
                  onClick={handleAccept}
                  disabled={!preview.can_accept || acceptMutation.isPending}
                  style={{
                    background: preview.can_accept ? 'var(--color-buy)' : 'var(--border-color)',
                    color: 'white', padding: '12px 24px', fontSize: '14px', fontWeight: 700,
                    borderRadius: '6px', cursor: preview.can_accept ? 'pointer' : 'not-allowed'
                  }}
                >
                  {acceptMutation.isPending ? '⏳ Đang lưu...' : '✅ Chấp nhận & Lưu dữ liệu vào hệ thống'}
                </button>
              </div>
            </div>
          )}
        </div>
      )}

      {/* TAB 2: DATA CATALOG */}
      {activeTab === 'catalog' && (
        <div className="panel">
          <h3 style={{ marginTop: 0 }}>📚 Danh mục Dữ liệu Chứng khoán Hiện có</h3>
          {catalogQuery.isLoading ? (
            <p>⏳ Đang tải danh mục...</p>
          ) : (
            <div style={{ overflowX: 'auto' }}>
              <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '13px' }}>
                <thead>
                  <tr style={{ background: 'var(--bg-card)', borderBottom: '1px solid var(--border-color)', textAlign: 'left' }}>
                    <th style={{ padding: '8px 12px' }}>Mã CK</th>
                    <th style={{ padding: '8px 12px' }}>Sàn</th>
                    <th style={{ padding: '8px 12px' }}>Khung thời gian</th>
                    <th style={{ padding: '8px 12px' }}>Điều chỉnh</th>
                    <th style={{ padding: '8px 12px' }}>Từ ngày</th>
                    <th style={{ padding: '8px 12px' }}>Đến ngày</th>
                    <th style={{ padding: '8px 12px' }}>Số nến</th>
                    <th style={{ padding: '8px 12px' }}>Cập nhật lần cuối</th>
                    <th style={{ padding: '8px 12px' }}>Nguồn gốc</th>
                  </tr>
                </thead>
                <tbody>
                  {(catalogQuery.data || []).map((item, idx) => (
                    <tr key={idx} style={{ borderBottom: '1px solid var(--border-color)' }}>
                      <td style={{ padding: '8px 12px', fontWeight: 700, color: 'var(--color-primary)' }}>{item.symbol}</td>
                      <td style={{ padding: '8px 12px' }}>{item.exchange || '—'}</td>
                      <td style={{ padding: '8px 12px' }}>
                        <span style={{ padding: '2px 6px', borderRadius: '4px', background: 'var(--bg-card)', fontSize: '11px' }}>
                          {item.timeframe}
                        </span>
                      </td>
                      <td style={{ padding: '8px 12px' }}>{item.adjustment_type}</td>
                      <td style={{ padding: '8px 12px' }}>{item.start_date || '—'}</td>
                      <td style={{ padding: '8px 12px' }}>{item.end_date || '—'}</td>
                      <td style={{ padding: '8px 12px', fontWeight: 700 }}>{item.row_count.toLocaleString()}</td>
                      <td style={{ padding: '8px 12px', color: 'var(--text-muted)' }}>
                        {item.last_accepted_at ? new Date(item.last_accepted_at).toLocaleString('vi-VN') : '—'}
                      </td>
                      <td style={{ padding: '8px 12px' }}>
                        <span
                          style={{
                            padding: '2px 6px', borderRadius: '4px', fontSize: '11px',
                            background: item.provenance_state === 'import_run' ? 'rgba(38, 166, 154, 0.15)' : 'rgba(255, 255, 255, 0.08)',
                            color: item.provenance_state === 'import_run' ? 'var(--color-buy)' : 'var(--text-muted)'
                          }}
                        >
                          {item.provenance_state === 'import_run' ? 'Lượt nhập (Audited)' : 'Mộc local'}
                        </span>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}

      {/* TAB 3: RUN HISTORY & ROLLBACK */}
      {activeTab === 'history' && (
        <div className="panel">
          <h3 style={{ marginTop: 0 }}>📜 Nhật ký Nhập dữ liệu & Quyền Hoàn tác (Rollback)</h3>
          {historyQuery.isLoading ? (
            <p>⏳ Đang tải lịch sử...</p>
          ) : (
            <div style={{ overflowX: 'auto' }}>
              <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '13px' }}>
                <thead>
                  <tr style={{ background: 'var(--bg-card)', borderBottom: '1px solid var(--border-color)', textAlign: 'left' }}>
                    <th style={{ padding: '8px 12px' }}>Thời gian</th>
                    <th style={{ padding: '8px 12px' }}>Tên tập tin</th>
                    <th style={{ padding: '8px 12px' }}>Khung / Điều chỉnh</th>
                    <th style={{ padding: '8px 12px' }}>Trạng thái</th>
                    <th style={{ padding: '8px 12px' }}>Số dòng chấp nhận</th>
                    <th style={{ padding: '8px 12px' }}>Hành động</th>
                  </tr>
                </thead>
                <tbody>
                  {(historyQuery.data || []).map((run) => (
                    <tr key={run.run_id} style={{ borderBottom: '1px solid var(--border-color)' }}>
                      <td style={{ padding: '8px 12px', color: 'var(--text-muted)' }}>
                        {run.created_at ? new Date(run.created_at).toLocaleString('vi-VN') : '—'}
                      </td>
                      <td style={{ padding: '8px 12px', fontWeight: 700 }}>{run.file_name}</td>
                      <td style={{ padding: '8px 12px' }}>{run.timeframe} / {run.adjustment_type}</td>
                      <td style={{ padding: '8px 12px' }}>
                        <span
                          style={{
                            padding: '2px 8px', borderRadius: '4px', fontSize: '11px', fontWeight: 700,
                            background: run.status === 'accepted' ? 'rgba(38, 166, 154, 0.15)' :
                                        run.status === 'rolled_back' ? 'rgba(239, 83, 80, 0.15)' : 'rgba(255, 255, 255, 0.1)',
                            color: run.status === 'accepted' ? 'var(--color-buy)' :
                                   run.status === 'rolled_back' ? 'var(--color-sell)' : 'var(--text-muted)'
                          }}
                        >
                          {run.status.toUpperCase()}
                        </span>
                      </td>
                      <td style={{ padding: '8px 12px', fontWeight: 700 }}>{run.accepted_count}</td>
                      <td style={{ padding: '8px 12px' }}>
                        {run.status === 'accepted' && (
                          <button
                            onClick={() => {
                              if (window.confirm(`Xác nhận hoàn tác lượt nhập "${run.file_name}"?`)) {
                                rollbackMutation.mutate(run.run_id);
                              }
                            }}
                            disabled={rollbackMutation.isPending}
                            style={{
                              background: 'var(--color-sell)', color: 'white',
                              border: 'none', padding: '4px 10px', fontSize: '12px',
                              borderRadius: '4px', cursor: 'pointer'
                            }}
                          >
                            ↩️ Hoàn tác (Rollback)
                          </button>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}
    </div>
  );
};
