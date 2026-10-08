import React, { useState, useMemo, useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';
import { getSignalRegistry, calculateReplaySignals } from '../../api/signalsApi';
import type {
  SignalCalculationResponse,
  SignalQuality,
} from '../../types/signals';

export interface SignalExplanationInspectorProps {
  sessionId?: number | null;
  currentIndex?: number;
  timeframe?: string;
  currentTimestamp?: string;
  onOpenCatalog?: () => void;
  className?: string;
  initialSignalName?: string;
}

export const SignalExplanationInspector: React.FC<SignalExplanationInspectorProps> = ({
  sessionId,
  currentIndex = 0,
  timeframe = '1D',
  currentTimestamp,
  onOpenCatalog,
  className = '',
  initialSignalName,
}) => {
  const [selectedSignalName, setSelectedSignalName] = useState<string>(initialSignalName || 'health.score');
  const [customParams, setCustomParams] = useState<Record<string, number>>({});

  useEffect(() => {
    if (initialSignalName) {
      setSelectedSignalName(initialSignalName);
      setCustomParams({});
    }
  }, [initialSignalName]);

  // Load registered signals
  const { data: registryData } = useQuery({
    queryKey: ['signal-registry'],
    queryFn: getSignalRegistry,
  });

  const allSignals = useMemo(() => registryData?.signals || [], [registryData]);
  const currentSignalDef = useMemo(() => {
    return allSignals.find((s) => s.name === selectedSignalName) || null;
  }, [allSignals, selectedSignalName]);

  const params = useMemo(() => {
    return { ...(currentSignalDef?.default_parameters || {}), ...customParams };
  }, [currentSignalDef, customParams]);

  // Query signal calculations reactively
  const {
    data: calcData,
    isLoading,
    error: queryError,
  } = useQuery<SignalCalculationResponse>({
    queryKey: ['signal-explanation', sessionId, currentIndex, selectedSignalName, params],
    queryFn: () => {
      if (!sessionId) {
        throw new Error('No session ID');
      }
      const signalsToRequest = [
        {
          name: selectedSignalName,
          params,
        },
        {
          name: 'health.score',
          params: {},
        },
        {
          name: 'bb.direction_rising',
          params: { horizon_bars: 20 },
        },
        {
          name: 'bb.regime_positive',
          params: { horizon_bars: 20 },
        },
      ];
      return calculateReplaySignals(sessionId, { signals: signalsToRequest });
    },
    enabled: Boolean(sessionId && sessionId > 0 && currentSignalDef),
  });

  const handleSignalChange = (name: string) => {
    setSelectedSignalName(name);
    setCustomParams({});
  };

  const handleParamChange = (pKey: string, rawVal: string) => {
    const num = Number(rawVal);
    if (!Number.isNaN(num)) {
      setCustomParams((prev) => ({ ...prev, [pKey]: num }));
    }
  };

  const apiError = queryError ? (queryError instanceof Error ? queryError.message : 'Calculation error') : null;

  const activeCalculation = useMemo(() => {
    if (!calcData || !sessionId || sessionId <= 0 || !currentSignalDef) return null;
    const series = calcData.results.find((r) => r.signal_name === selectedSignalName);
    return series && series.points.length > 0 ? series.points[series.points.length - 1] : null;
  }, [calcData, sessionId, currentSignalDef, selectedSignalName]);

  const activeHealth = useMemo(() => {
    if (!calcData || !sessionId || sessionId <= 0) return null;
    const series = calcData.results.find((r) => r.signal_name === 'health.score');
    return series && series.points.length > 0 ? series.points[series.points.length - 1] : null;
  }, [calcData, sessionId]);

  const activeFlowDirection = useMemo(() => {
    if (!calcData || !sessionId || sessionId <= 0) return null;
    const series = calcData.results.find((r) => r.signal_name === 'bb.direction_rising');
    return series && series.points.length > 0 ? series.points[series.points.length - 1] : null;
  }, [calcData, sessionId]);

  const activeFlowRegime = useMemo(() => {
    if (!calcData || !sessionId || sessionId <= 0) return null;
    const series = calcData.results.find((r) => r.signal_name === 'bb.regime_positive');
    return series && series.points.length > 0 ? series.points[series.points.length - 1] : null;
  }, [calcData, sessionId]);

  const getQualityBadgeStyle = (quality?: SignalQuality) => {
    switch (quality) {
      case 'VALID':
        return { background: 'rgba(0, 230, 118, 0.15)', color: 'var(--color-buy)', border: '1px solid rgba(0, 230, 118, 0.4)' };
      case 'INSUFFICIENT_HISTORY':
        return { background: 'rgba(255, 209, 102, 0.15)', color: '#FFD166', border: '1px solid rgba(255, 209, 102, 0.4)' };
      case 'INVALID_VOLUME':
        return { background: 'rgba(255, 23, 68, 0.15)', color: 'var(--color-sell)', border: '1px solid rgba(255, 23, 68, 0.4)' };
      case 'ZERO_BASELINE':
        return { background: 'rgba(255, 152, 0, 0.15)', color: '#FF9800', border: '1px solid rgba(255, 152, 0, 0.4)' };
      default:
        return { background: 'rgba(255, 255, 255, 0.05)', color: 'var(--text-muted)', border: '1px solid rgba(255, 255, 255, 0.1)' };
    }
  };

  return (
    <div
      className={`glass-panel signal-explanation-inspector ${className}`}
      data-testid="signal-explanation-inspector"
      style={{ padding: '16px', borderRadius: '8px' }}
    >
      {/* Header */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '12px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <span style={{ fontSize: '16px' }}>🔬</span>
          <h4 style={{ margin: 0, fontSize: '14px', fontWeight: 600 }}>
            Kiểm Tra & Giải Thích Tín Hiệu (Live Explanation)
          </h4>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          {onOpenCatalog && (
            <button
              type="button"
              data-testid="open-catalog-btn"
              onClick={onOpenCatalog}
              style={{
                padding: '4px 8px',
                fontSize: '11px',
                borderRadius: '4px',
                background: 'rgba(41, 98, 255, 0.2)',
                color: 'var(--color-primary)',
                border: '1px solid var(--color-primary)',
                cursor: 'pointer',
              }}
            >
              📚 Mở Danh Mục
            </button>
          )}
          <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
            Khung: {timeframe} • Nến: #{currentIndex} {currentTimestamp ? `(${currentTimestamp.slice(0, 10)})` : ''}
          </span>
        </div>
      </div>

      {/* Signal Selector */}
      <div style={{ marginBottom: '12px' }}>
        <label htmlFor="signal-picker-select" style={{ display: 'block', fontSize: '11px', color: 'var(--text-muted)', marginBottom: '4px' }}>
          Chọn tín hiệu cần kiểm tra:
        </label>
        <select
          id="signal-picker-select"
          data-testid="signal-picker-select"
          value={selectedSignalName}
          onChange={(e) => handleSignalChange(e.target.value)}
          style={{
            width: '100%',
            padding: '8px',
            fontSize: '12px',
            borderRadius: '4px',
            background: 'rgba(255, 255, 255, 0.05)',
            border: '1px solid var(--border-color)',
            color: 'var(--text-main)',
          }}
        >
          {allSignals.map((sig) => (
            <option key={sig.name} value={sig.name} style={{ background: '#1c1f26', color: '#fff' }}>
              [{sig.category.toUpperCase()}] {sig.label_vi} ({sig.name})
            </option>
          ))}
        </select>
      </div>

      {/* Parameter Controls if any */}
      {currentSignalDef && Object.keys(currentSignalDef.parameters_schema || {}).length > 0 && (
        <div
          style={{
            background: 'rgba(0, 0, 0, 0.2)',
            padding: '10px',
            borderRadius: '6px',
            marginBottom: '12px',
          }}
        >
          <div style={{ fontSize: '11px', fontWeight: 600, color: 'var(--text-muted)', marginBottom: '6px' }}>
            Hiệu chỉnh tham số:
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(130px, 1fr))', gap: '8px' }}>
            {Object.keys(currentSignalDef.parameters_schema).map((pKey) => {
              const schema = currentSignalDef.parameters_schema[pKey];
              return (
                <div key={pKey}>
                  <label htmlFor={`param-input-${pKey}`} style={{ display: 'block', fontSize: '10px', color: 'var(--text-muted)', marginBottom: '2px' }}>
                    {pKey} ({schema?.type}):
                  </label>
                  <input
                    id={`param-input-${pKey}`}
                    type="number"
                    value={params[pKey] ?? schema?.default ?? 0}
                    step={schema?.type === 'float' ? '0.1' : '1'}
                    min={schema?.minimum}
                    max={schema?.maximum}
                    onChange={(e) => handleParamChange(pKey, e.target.value)}
                    style={{
                      width: '100%',
                      padding: '4px 6px',
                      fontSize: '11px',
                      borderRadius: '4px',
                      background: 'rgba(255, 255, 255, 0.05)',
                      border: '1px solid var(--border-color)',
                      color: 'var(--text-main)',
                    }}
                  />
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Results Display Area (UI-EXP-001) */}
      <div
        data-testid="explanation-result-box"
        style={{
          background: 'rgba(255, 255, 255, 0.02)',
          border: '1px solid var(--border-color)',
          borderRadius: '6px',
          padding: '12px',
          marginBottom: '12px',
        }}
      >
        {isLoading && (
          <div style={{ textAlign: 'center', color: 'var(--text-muted)', fontSize: '12px', padding: '12px' }}>
            Đang tính toán nhân quả tại nến #{currentIndex}...
          </div>
        )}

        {apiError && (
          <div style={{ color: 'var(--color-sell)', fontSize: '12px' }}>
            Lỗi tính toán: {apiError}
          </div>
        )}

        {!isLoading && !apiError && !activeCalculation && (
          <div style={{ textAlign: 'center', color: 'var(--text-muted)', fontSize: '12px' }}>
            {sessionId ? 'Chưa có kết quả tại nến này.' : 'Vui lòng bắt đầu hoặc chọn một phiên Replay để kiểm tra.'}
          </div>
        )}

        {!isLoading && !apiError && activeCalculation && (
          <div>
            {/* Primary Signal Summary */}
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '10px' }}>
              <div>
                <span style={{ fontSize: '14px', fontWeight: 600 }}>
                  {currentSignalDef?.label_vi}
                </span>
                <div style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '2px' }}>
                  {currentSignalDef?.name} • AST: <code style={{ color: '#58A6FF' }}>{currentSignalDef?.ast_alias}</code>
                </div>
              </div>
              <div style={{ textAlign: 'right' }}>
                <span
                  data-testid="signal-explanation-quality-badge"
                  style={{
                    fontSize: '10px',
                    padding: '2px 6px',
                    borderRadius: '4px',
                    fontWeight: 600,
                    ...getQualityBadgeStyle(activeCalculation.quality),
                  }}
                >
                  {activeCalculation.quality}
                </span>
                <div style={{ fontSize: '16px', fontWeight: 700, marginTop: '4px' }}>
                  {typeof activeCalculation.value === 'boolean' ? (
                    activeCalculation.value ? (
                      <span style={{ color: 'var(--color-buy)' }}>KÍCH HOẠT (TRUE) ✓</span>
                    ) : (
                      <span style={{ color: 'var(--text-muted)' }}>KHÔNG KÍCH HOẠT (FALSE) ✗</span>
                    )
                  ) : activeCalculation.value !== null && typeof activeCalculation.value === 'number' ? (
                    <span style={{ color: activeCalculation.value >= 0 ? 'var(--color-buy)' : 'var(--color-sell)' }}>
                      {activeCalculation.value.toFixed(2)}
                    </span>
                  ) : (
                    <span style={{ color: 'var(--text-muted)' }}>N/A</span>
                  )}
                </div>
              </div>
            </div>

            {/* Authoritative Structured Reasons (UI-EXP-001) */}
            <div style={{ borderTop: '1px solid rgba(255, 255, 255, 0.08)', paddingTop: '8px' }}>
              <div style={{ fontSize: '11px', fontWeight: 600, color: 'var(--text-muted)', marginBottom: '4px' }}>
                Giải thích yếu tố kích hoạt (Reasons):
              </div>
              {activeCalculation.reasons && activeCalculation.reasons.length > 0 ? (
                <ul data-testid="signal-reasons-list" style={{ margin: 0, paddingLeft: '16px', fontSize: '12px', color: 'var(--text-main)' }}>
                  {activeCalculation.reasons.map((r, i) => (
                    <li key={i} style={{ marginBottom: '2px' }}>
                      {r.replace(/_/g, ' ')}
                    </li>
                  ))}
                </ul>
              ) : (
                <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Không có lý do cấu thành đặc biệt.</div>
              )}
            </div>

            {activeCalculation.quality === 'INSUFFICIENT_HISTORY' && (
              <div
                style={{
                  marginTop: '8px',
                  padding: '6px 10px',
                  borderRadius: '4px',
                  background: 'rgba(255, 209, 102, 0.1)',
                  border: '1px solid rgba(255, 209, 102, 0.3)',
                  fontSize: '11px',
                  color: '#FFD166',
                }}
              >
                ℹ️ Chưa đủ dữ liệu nến lịch sử để tính toán chỉ báo này. Hãy bấm phát (▶) hoặc bước tới vài phiên tiếp theo.
              </div>
            )}

            {/* Availability / Causal Verification Info */}
            <div
              style={{
                marginTop: '10px',
                paddingTop: '6px',
                borderTop: '1px dashed rgba(255, 255, 255, 0.06)',
                display: 'flex',
                justifyContent: 'space-between',
                fontSize: '10px',
                color: 'var(--text-muted)',
              }}
            >
              <span>Thời điểm khả dụng: {activeCalculation.availability_event || 'BAR_CLOSE'}</span>
              <span>Khả dụng tại nến: #{activeCalculation.available_at_index}</span>
            </div>
          </div>
        )}
      </div>

      {/* Technical Health Composite Section */}
      <div
        style={{
          background: 'rgba(41, 98, 255, 0.04)',
          border: '1px solid rgba(88, 166, 255, 0.2)',
          borderRadius: '6px',
          padding: '10px',
          marginBottom: '10px',
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '4px' }}>
          <span style={{ fontSize: '12px', fontWeight: 600, color: '#58A6FF' }}>
            🩺 Sức Khỏe Kỹ Thuật Tổng Hợp (Technical Health)
          </span>
          <span style={{ fontSize: '13px', fontWeight: 700 }}>
            {activeHealth?.value !== null && typeof activeHealth?.value === 'number'
              ? `${activeHealth.value > 0 ? '+' : ''}${activeHealth.value.toFixed(1)} / 100`
              : 'N/A'}
          </span>
        </div>
        <div style={{ display: 'flex', gap: '8px', fontSize: '11px', color: 'var(--text-muted)', flexWrap: 'wrap' }}>
          <span>4 Nhóm: Xu hướng (35%), Động lượng (25%), Chuyển dịch (20%), Dòng tiền (20%)</span>
          <span style={{ color: '#FFD166' }}>
            • Thiếu dữ liệu BB = Trung tính (không trừ điểm âm)
          </span>
        </div>
      </div>

      {/* Money Flow BB Guardrail Section (F.3) */}
      <div
        style={{
          background: 'rgba(0, 0, 0, 0.25)',
          border: '1px solid rgba(255, 255, 255, 0.06)',
          borderRadius: '6px',
          padding: '10px',
          fontSize: '11px',
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '4px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <span style={{ fontWeight: 600 }}>Dòng Tiền BB (Money Flow BB):</span>
            <span
              data-testid="badge-flow-method"
              style={{
                fontSize: '9px',
                padding: '1px 5px',
                borderRadius: '3px',
                background: 'rgba(88, 166, 255, 0.2)',
                color: '#58A6FF',
                border: '1px solid rgba(88, 166, 255, 0.4)',
                fontWeight: 600,
              }}
            >
              OHLCV_PROXY
            </span>
          </div>
          <span style={{ color: 'var(--text-muted)' }}>
            Chất lượng: <strong style={{ color: 'var(--color-buy)' }}>HIGH</strong>
          </span>
        </div>
        <div style={{ display: 'flex', gap: '12px', color: 'var(--text-muted)', marginTop: '4px' }}>
          <span>
            Xu hướng T20:{' '}
            <strong style={{ color: activeFlowDirection?.value ? 'var(--color-buy)' : 'var(--text-muted)' }}>
              {activeFlowDirection?.value ? 'TĂNG ▲' : 'GIẢM / TRUNG TÍNH'}
            </strong>
          </span>
          <span>
            Vùng T20:{' '}
            <strong style={{ color: activeFlowRegime?.value ? 'var(--color-buy)' : 'var(--text-muted)' }}>
              {activeFlowRegime?.value ? 'DƯƠNG (> 50)' : 'ÂM (≤ 50)'}
            </strong>
          </span>
        </div>
        <div style={{ fontSize: '10px', color: 'var(--text-muted)', marginTop: '4px', fontStyle: 'italic' }}>
          ⚠️ Phương pháp ước tính từ biến động giá & khối lượng — không thay thế dòng tiền lệnh khớp thực tế từ sở.
        </div>
      </div>
    </div>
  );
};
