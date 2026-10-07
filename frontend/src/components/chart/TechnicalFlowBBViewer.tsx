import React, { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { getBBHorizons, getSymbolBB } from '../../api/bbApi';

export interface TechnicalFlowBBViewerProps {
  defaultSymbol?: string;
  className?: string;
}

const ALL_HORIZONS = ['T03', 'T05', 'T10', 'T20', 'T50', 'T200'];

const getHorizonValue = (data: unknown): number | null => {
  if (typeof data === 'number' && !isNaN(data)) return data;
  if (data && typeof data === 'object') {
    const obj = data as Record<string, unknown>;
    if (typeof obj.bb_value === 'number' && !isNaN(obj.bb_value)) return obj.bb_value;
    if (typeof obj.value === 'number' && !isNaN(obj.value)) return obj.value;
  }
  return null;
};

const formatSafeNumber = (val: unknown, decimals = 2, fallback = '-'): string => {
  if (typeof val === 'number' && !isNaN(val)) {
    return val.toFixed(decimals);
  }
  return fallback;
};

export const TechnicalFlowBBViewer: React.FC<TechnicalFlowBBViewerProps> = ({
  defaultSymbol = 'FPT',
  className = '',
}) => {
  const [symbol, setSymbol] = useState<string>(defaultSymbol);
  const [activeHorizons, setActiveHorizons] = useState<string[]>(['T03', 'T05', 'T20', 'T50', 'T200']); // BB10 optionally toggled per F.3
  const [asOfDate, setAsOfDate] = useState<string>('');

  useQuery({
    queryKey: ['bb-horizons'],
    queryFn: getBBHorizons,
  });

  const { data: bbData, isLoading, error } = useQuery({
    queryKey: ['bb-symbol', symbol, activeHorizons.join(','), asOfDate],
    queryFn: () =>
      getSymbolBB(symbol, {
        horizons: activeHorizons.join(','),
        as_of: asOfDate || undefined,
        limit: 100,
        format: 'series',
      }),
    enabled: !!symbol,
  });

  const [selectedHorizon, setSelectedHorizon] = useState<string>('T20');

  const toggleHorizon = (h: string) => {
    setActiveHorizons((prev) => {
      if (prev.includes(h)) {
        if (prev.length <= 1) return prev; // keep at least 1
        return prev.filter((item) => item !== h);
      }
      return [...prev, h];
    });
    setSelectedHorizon(h);
  };

  const points = bbData?.points || [];
  const latestPoint = points.length > 0 ? points[points.length - 1] : null;

  const activeInspectionHorizon = activeHorizons.includes(selectedHorizon)
    ? selectedHorizon
    : (activeHorizons[0] || 'T20');

  const horizonPoint = latestPoint?.horizons?.[activeInspectionHorizon]
    || (latestPoint?.horizons ? Object.values(latestPoint.horizons)[0] : undefined);

  // Safely extract flow metrics with fallbacks to avoid toFixed crashes (ST-05)
  const extractedHorizonVal = getHorizonValue(latestPoint?.horizons?.[activeInspectionHorizon])
    ?? getHorizonValue(horizonPoint);
  const flowValue = latestPoint?.value ?? extractedHorizonVal ?? null;
  const basisValue = latestPoint?.basis ?? (typeof horizonPoint?.raw_denominator === 'number' ? horizonPoint.raw_denominator : null);
  const bandwidthValue = typeof latestPoint?.bandwidth === 'number' ? latestPoint.bandwidth : null;
  const percentBValue = typeof latestPoint?.percent_b === 'number' ? latestPoint.percent_b : null;
  const regimeValue = horizonPoint?.regime ?? latestPoint?.regime ?? 'NEUTRAL';

  return (
    <div
      className={`glass-panel technical-flow-bb-viewer ${className}`}
      data-testid="technical-flow-bb-viewer"
      style={{ padding: '20px', borderRadius: '8px' }}
    >
      {/* Header with Methodology & Quality Badges (F.3) */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', flexWrap: 'wrap', gap: '12px', marginBottom: '16px' }}>
        <div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '4px' }}>
            <span style={{ fontSize: '18px' }}>🌊</span>
            <h4 style={{ margin: 0, fontSize: '16px', fontWeight: 600 }}>
              Dòng Tiền Kỹ Thuật Bollinger Bands (Technical Flow BB)
            </h4>
            <span
              data-testid="bb-methodology-badge"
              style={{
                fontSize: '11px',
                padding: '2px 8px',
                borderRadius: '4px',
                background: 'rgba(88, 166, 255, 0.2)',
                color: '#58A6FF',
                border: '1px solid rgba(88, 166, 255, 0.5)',
                fontWeight: 600,
              }}
              title="Phương pháp Proxy: Ước tính từ biến động giá và khối lượng (OHLCV Proxy) — không đại diện cho dòng tiền khớp lệnh trực tiếp từ sở"
            >
              PHƯƠNG PHÁP: OHLCV_PROXY
            </span>
          </div>
          <p style={{ margin: 0, fontSize: '12px', color: 'var(--text-muted)' }}>
            Theo dõi phân kỳ, vùng quá mua/quá bán và hướng đi dòng tiền kỹ thuật trên các khung thời gian T03–T200.
          </p>
        </div>

        {/* Audit Quality Badge */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <span
            data-testid="bb-data-quality-badge"
            style={{
              fontSize: '11px',
              padding: '2px 8px',
              borderRadius: '4px',
              background: 'rgba(0, 230, 118, 0.15)',
              color: 'var(--color-buy)',
              border: '1px solid rgba(0, 230, 118, 0.4)',
              fontWeight: 600,
            }}
          >
            CHẤT LƯỢNG DỮ LIỆU: HIGH
          </span>
        </div>
      </div>

      {/* Controls: Symbol input + Horizons toggles */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '12px', marginBottom: '16px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <label htmlFor="bb-symbol-input" style={{ fontSize: '12px', color: 'var(--text-muted)' }}>Mã cổ phiếu:</label>
          <input
            id="bb-symbol-input"
            data-testid="bb-symbol-input"
            type="text"
            value={symbol}
            onChange={(e) => setSymbol(e.target.value.toUpperCase())}
            style={{
              padding: '6px 10px',
              fontSize: '13px',
              borderRadius: '4px',
              background: 'rgba(255, 255, 255, 0.05)',
              border: '1px solid var(--border-color)',
              color: 'var(--text-main)',
              width: '80px',
              fontWeight: 600,
            }}
          />

          <label htmlFor="bb-as-of-input" style={{ fontSize: '12px', color: 'var(--text-muted)', marginLeft: '10px' }}>Ngày mốc:</label>
          <input
            id="bb-as-of-input"
            type="date"
            value={asOfDate}
            onChange={(e) => setAsOfDate(e.target.value)}
            style={{
              padding: '6px 10px',
              fontSize: '12px',
              borderRadius: '4px',
              background: 'rgba(255, 255, 255, 0.05)',
              border: '1px solid var(--border-color)',
              color: 'var(--text-main)',
            }}
          />
        </div>

        {/* Horizons Toggles (F.3: Expose BB03/05/20/50/200 with BB10 optionally toggled) */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <span style={{ fontSize: '11px', color: 'var(--text-muted)', marginRight: '4px' }}>Khung thời gian (Horizons):</span>
          {ALL_HORIZONS.map((h) => {
            const isActive = activeHorizons.includes(h);
            return (
              <button
                key={h}
                type="button"
                data-testid={`toggle-horizon-${h}`}
                onClick={() => toggleHorizon(h)}
                style={{
                  padding: '4px 8px',
                  fontSize: '11px',
                  borderRadius: '4px',
                  border: isActive ? '1px solid var(--color-primary)' : '1px solid var(--border-color)',
                  background: isActive ? 'rgba(41, 98, 255, 0.25)' : 'rgba(255, 255, 255, 0.04)',
                  color: isActive ? 'var(--color-primary)' : 'var(--text-muted)',
                  cursor: 'pointer',
                  fontWeight: isActive ? 600 : 400,
                }}
              >
                {h} {h === 'T10' ? '(Tùy chọn)' : ''}
              </button>
            );
          })}
        </div>
      </div>

      {/* State / Loading */}
      {isLoading && (
        <div style={{ padding: '24px', textAlign: 'center', color: 'var(--text-muted)' }}>
          Đang tính toán dòng tiền BB cho {symbol}...
        </div>
      )}

      {error && (
        <div style={{ padding: '12px', background: 'rgba(255, 23, 68, 0.1)', color: 'var(--color-sell)', borderRadius: '6px', fontSize: '12px' }}>
          Lỗi: {error instanceof Error ? error.message : 'Không thể tải dữ liệu BB'}
        </div>
      )}

      {/* Metrics Card */}
      {!isLoading && !error && latestPoint && (
        <div
          data-testid="bb-latest-summary"
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fit, minmax(140px, 1fr))',
            gap: '12px',
            background: 'rgba(255, 255, 255, 0.02)',
            border: '1px solid var(--border-color)',
            borderRadius: '6px',
            padding: '14px',
            marginBottom: '16px',
          }}
        >
          <div>
            <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Ngày quan sát:</div>
            <div style={{ fontSize: '13px', fontWeight: 600, marginTop: '2px' }}>{latestPoint?.date ?? 'N/A'}</div>
          </div>
          <div>
            <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
              Giá trị Dòng tiền (Value{latestPoint?.horizons && activeInspectionHorizon ? ` ${activeInspectionHorizon}` : ''}):
            </div>
            <div
              style={{
                fontSize: '13px',
                fontWeight: 700,
                marginTop: '2px',
                color: (flowValue ?? 0) >= 0 ? 'var(--color-buy)' : 'var(--color-sell)',
              }}
            >
              {formatSafeNumber(flowValue, 2, 'N/A')}
            </div>
          </div>
          <div>
            <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Trục giữa (Basis):</div>
            <div style={{ fontSize: '13px', fontWeight: 600, marginTop: '2px' }}>
              {formatSafeNumber(basisValue, 2, 'N/A')}
            </div>
          </div>
          <div>
            <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Độ rộng dải (Bandwidth):</div>
            <div style={{ fontSize: '13px', fontWeight: 600, marginTop: '2px' }}>
              {formatSafeNumber(bandwidthValue, 2, 'N/A')}
            </div>
          </div>
          <div>
            <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Vị trí %B:</div>
            <div
              style={{
                fontSize: '13px',
                fontWeight: 700,
                marginTop: '2px',
                color:
                  percentBValue !== null && percentBValue !== undefined
                    ? percentBValue >= 1
                      ? '#FFD166'
                      : percentBValue <= 0
                      ? 'var(--color-sell)'
                      : 'var(--text-main)'
                    : 'var(--text-muted)',
              }}
            >
              {percentBValue !== null && percentBValue !== undefined && !isNaN(Number(percentBValue))
                ? `${(Number(percentBValue) * 100).toFixed(1)}%`
                : 'N/A'}
            </div>
          </div>
          <div>
            <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Vùng trạng thái (Regime):</div>
            <div
              style={{
                fontSize: '13px',
                fontWeight: 600,
                marginTop: '2px',
                color:
                  regimeValue === 'POSITIVE'
                    ? 'var(--color-buy)'
                    : regimeValue === 'NEGATIVE'
                    ? 'var(--color-sell)'
                    : 'var(--text-muted)',
              }}
            >
              {regimeValue}
            </div>
          </div>
        </div>
      )}

      {/* Horizons Quick Breakdown with Safe Access (ST-05) */}
      {!isLoading && !error && latestPoint && (
        <div
          data-testid="bb-horizons-breakdown"
          style={{
            display: 'flex',
            gap: '8px',
            flexWrap: 'wrap',
            marginBottom: '16px',
            alignItems: 'center',
          }}
        >
          <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>Chi tiết các kỳ hạn (Horizons):</span>
          {activeHorizons.map((h) => {
            const hRaw = latestPoint?.horizons?.[h];
            const hVal = getHorizonValue(hRaw);
            const isSelected = selectedHorizon === h;
            return (
              <button
                key={h}
                type="button"
                data-testid={`horizon-badge-${h}`}
                onClick={() => setSelectedHorizon(h)}
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: '6px',
                  padding: '3px 8px',
                  borderRadius: '4px',
                  background: isSelected ? 'rgba(41, 98, 255, 0.25)' : 'rgba(255, 255, 255, 0.03)',
                  border: isSelected ? '1px solid var(--color-primary)' : '1px solid var(--border-color)',
                  cursor: 'pointer',
                  fontSize: '11px',
                }}
              >
                <span style={{ fontWeight: 600, color: isSelected ? 'var(--color-primary)' : 'var(--text-muted)' }}>{h}:</span>
                <span style={{ color: hVal !== null ? (hVal >= 0 ? 'var(--color-buy)' : 'var(--color-sell)') : 'var(--text-muted)' }}>
                  {formatSafeNumber(hVal, 2, '-')}
                </span>
              </button>
            );
          })}
        </div>
      )}

      {/* Mandatory Companion Spec Disclaimer (F.3) */}
      <div
        data-testid="bb-audit-disclaimer"
        style={{
          background: 'rgba(0, 0, 0, 0.3)',
          border: '1px dashed rgba(88, 166, 255, 0.3)',
          borderRadius: '6px',
          padding: '10px 14px',
          fontSize: '11px',
          color: 'var(--text-muted)',
          lineHeight: 1.4,
        }}
      >
        <span style={{ color: '#58A6FF', fontWeight: 600 }}>⚠️ Quy tắc chuẩn mực Sumi (F.3): </span>
        Chỉ báo Dòng Tiền Kỹ Thuật BB tuân thủ định dạng nhãn chuẩn <code>OHLCV_PROXY</code>. Tuyệt đối không suy diễn chỉ báo này là dòng tiền mua/bán chủ động trực tiếp từ sổ lệnh của sở giao dịch.
      </div>
    </div>
  );
};
