import React, { useState, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { getSignalRegistry } from '../../api/signalsApi';
import type { SignalDefinition, SignalParameterSchema } from '../../types/signals';

export interface SignalCatalogProps {
  onSelectSignal?: (signal: SignalDefinition) => void;
  selectedSignalName?: string;
  className?: string;
}

const CATEGORY_LABELS: Record<string, { vi: string; icon: string }> = {
  all: { vi: 'Tất cả', icon: '🌐' },
  health: { vi: 'Sức khỏe kỹ thuật', icon: '🩺' },
  flow: { vi: 'Dòng tiền BB (Proxy)', icon: '🌊' },
  divergence: { vi: 'Phân kỳ 4 chiều', icon: '⚡' },
  ichimoku: { vi: 'Mây Ichimoku', icon: '☁️' },
  vsa: { vi: 'VSA (Volume Spread)', icon: '📊' },
  volume: { vi: 'Khối lượng & RVOL', icon: '📶' },
  technical: { vi: 'Chỉ báo & Giao cắt', icon: '📈' },
  regime: { vi: 'Trạng thái xu hướng', icon: '🧭' },
  pattern: { vi: 'Mẫu hình nến', icon: '🕯️' },
  sr: { vi: 'Hỗ trợ & Kháng cự', icon: '🧱' },
  structure: { vi: 'Cấu trúc nến', icon: '🏛️' },
};

export const SignalCatalog: React.FC<SignalCatalogProps> = ({
  onSelectSignal,
  selectedSignalName,
  className = '',
}) => {
  const [selectedCategory, setSelectedCategory] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [showAdvancedParams, setShowAdvancedParams] = useState<boolean>(false); // UI-SIG-003: Default hide advanced params

  const { data, isLoading, error } = useQuery({
    queryKey: ['signal-registry'],
    queryFn: getSignalRegistry,
  });

  const allSignals = useMemo(() => data?.signals || [], [data]);

  // Group counts per category
  const categoryCounts = useMemo(() => {
    const counts: Record<string, number> = { all: allSignals.length };
    allSignals.forEach((sig) => {
      const cat = sig.category.toLowerCase();
      counts[cat] = (counts[cat] || 0) + 1;
    });
    return counts;
  }, [allSignals]);

  // Filtered signals
  const filteredSignals = useMemo(() => {
    return allSignals.filter((sig) => {
      // Category filter
      if (selectedCategory !== 'all' && sig.category.toLowerCase() !== selectedCategory.toLowerCase()) {
        return false;
      }
      // Search filter
      if (searchQuery.trim()) {
        const query = searchQuery.trim().toLowerCase();
        const matchesName = sig.name.toLowerCase().includes(query);
        const matchesLabel = sig.label_vi.toLowerCase().includes(query);
        const matchesDesc = sig.description.toLowerCase().includes(query);
        const matchesAlias = sig.ast_alias.toLowerCase().includes(query);
        return matchesName || matchesLabel || matchesDesc || matchesAlias;
      }
      return true;
    });
  }, [allSignals, selectedCategory, searchQuery]);

  return (
    <div className={`signal-catalog-container ${className}`} data-testid="signal-catalog">
      {/* Header & Controls */}
      <div style={{ marginBottom: '16px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '12px', marginBottom: '12px' }}>
          <div>
            <h3 style={{ margin: '0 0 4px 0', fontSize: '18px', fontWeight: 600 }}>
              📚 Danh Mục Tín Hiệu Định Lượng ({allSignals.length} Tín Hiệu)
            </h3>
            <p style={{ margin: 0, fontSize: '13px', color: 'var(--text-muted)' }}>
              Cơ sở dữ liệu tín hiệu kỹ thuật chuẩn hóa, kiểm định nhân quả (zero lookahead) và tích hợp AST.
            </p>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <button
              type="button"
              data-testid="toggle-advanced-params-btn"
              onClick={() => setShowAdvancedParams((prev) => !prev)}
              style={{
                padding: '6px 12px',
                fontSize: '12px',
                borderRadius: '4px',
                background: showAdvancedParams ? 'rgba(41, 98, 255, 0.2)' : 'rgba(255, 255, 255, 0.06)',
                border: `1px solid ${showAdvancedParams ? 'var(--color-primary)' : 'var(--border-color)'}`,
                color: showAdvancedParams ? 'var(--color-primary)' : 'var(--text-muted)',
                cursor: 'pointer',
              }}
            >
              {showAdvancedParams ? '👁️ Ẩn tham số nâng cao' : '⚙️ Xem tham số nâng cao'}
            </button>
          </div>
        </div>

        {/* Search Bar */}
        <div style={{ marginBottom: '12px' }}>
          <input
            type="text"
            data-testid="signal-catalog-search-input"
            placeholder="🔍 Tìm theo tên tín hiệu, mô tả tiếng Việt, mã AST (ví dụ: health, rsi, engulfing)..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            style={{
              width: '100%',
              padding: '10px 14px',
              fontSize: '13px',
              borderRadius: '6px',
              background: 'rgba(255, 255, 255, 0.04)',
              border: '1px solid var(--border-color)',
              color: 'var(--text-main)',
            }}
          />
        </div>

        {/* Category Tabs (UI-SIG-001) */}
        <div
          data-testid="signal-category-tabs"
          style={{
            display: 'flex',
            flexWrap: 'wrap',
            gap: '6px',
            borderBottom: '1px solid var(--border-color)',
            paddingBottom: '10px',
          }}
        >
          {Object.keys(CATEGORY_LABELS).map((catKey) => {
            const info = CATEGORY_LABELS[catKey];
            const count = categoryCounts[catKey] ?? 0;
            const isSelected = selectedCategory === catKey;
            return (
              <button
                key={catKey}
                type="button"
                data-testid={`category-tab-${catKey}`}
                onClick={() => setSelectedCategory(catKey)}
                style={{
                  padding: '5px 10px',
                  fontSize: '12px',
                  borderRadius: '4px',
                  border: isSelected ? '1px solid var(--color-primary)' : '1px solid transparent',
                  background: isSelected ? 'rgba(41, 98, 255, 0.25)' : 'rgba(255, 255, 255, 0.04)',
                  color: isSelected ? 'var(--color-primary)' : 'var(--text-muted)',
                  cursor: 'pointer',
                  fontWeight: isSelected ? 600 : 400,
                  display: 'flex',
                  alignItems: 'center',
                  gap: '5px',
                }}
              >
                <span>{info.icon}</span>
                <span>{info.vi}</span>
                <span
                  style={{
                    fontSize: '10px',
                    padding: '0 5px',
                    borderRadius: '10px',
                    background: isSelected ? 'rgba(41, 98, 255, 0.5)' : 'rgba(255, 255, 255, 0.1)',
                    color: '#fff',
                  }}
                >
                  {count}
                </span>
              </button>
            );
          })}
        </div>
      </div>

      {/* Content Area */}
      {isLoading && (
        <div style={{ padding: '32px', textAlign: 'center', color: 'var(--text-muted)' }}>
          Đang tải danh mục tín hiệu từ máy chủ...
        </div>
      )}

      {error && (
        <div style={{ padding: '16px', background: 'rgba(255, 23, 68, 0.1)', color: 'var(--color-sell)', borderRadius: '6px' }}>
          Lỗi khi tải danh mục tín hiệu: {error instanceof Error ? error.message : 'Unknown error'}
        </div>
      )}

      {!isLoading && !error && filteredSignals.length === 0 && (
        <div style={{ padding: '32px', textAlign: 'center', color: 'var(--text-muted)' }}>
          Không tìm thấy tín hiệu nào phù hợp với điều kiện tìm kiếm.
        </div>
      )}

      {/* Signals Grid (UI-SIG-002) */}
      <div
        data-testid="signal-cards-grid"
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fill, minmax(320px, 1fr))',
          gap: '12px',
          maxHeight: '620px',
          overflowY: 'auto',
          paddingRight: '4px',
        }}
      >
        {filteredSignals.map((sig) => {
          const isSelected = selectedSignalName === sig.name;
          const isExperimental = sig.status === 'EXPERIMENTAL';
          const paramKeys = Object.keys(sig.parameters_schema || {});

          return (
            <div
              key={sig.name}
              data-testid={`signal-card-${sig.name}`}
              onClick={() => onSelectSignal?.(sig)}
              style={{
                padding: '14px',
                borderRadius: '8px',
                background: isSelected ? 'rgba(41, 98, 255, 0.12)' : 'rgba(255, 255, 255, 0.03)',
                border: isSelected ? '1px solid var(--color-primary)' : '1px solid rgba(255, 255, 255, 0.08)',
                cursor: onSelectSignal ? 'pointer' : 'default',
                display: 'flex',
                flexDirection: 'column',
                justifyContent: 'space-between',
                transition: 'all 0.15s ease',
              }}
            >
              <div>
                {/* Header row: Vietnamese label + Badges */}
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', gap: '8px', marginBottom: '6px' }}>
                  <h4 style={{ margin: 0, fontSize: '14px', fontWeight: 600, color: 'var(--text-main)' }}>
                    {sig.label_vi}
                  </h4>
                  <div style={{ display: 'flex', gap: '4px', flexWrap: 'wrap', justifyContent: 'flex-end' }}>
                    {isExperimental && (
                      <span
                        data-testid="badge-experimental"
                        style={{
                          fontSize: '10px',
                          padding: '1px 5px',
                          borderRadius: '3px',
                          background: 'rgba(255, 209, 102, 0.2)',
                          color: '#FFD166',
                          border: '1px solid rgba(255, 209, 102, 0.4)',
                          fontWeight: 600,
                        }}
                      >
                        THỬ NGHIỆM
                      </span>
                    )}
                    <span
                      style={{
                        fontSize: '10px',
                        padding: '1px 5px',
                        borderRadius: '3px',
                        background: 'rgba(255, 255, 255, 0.06)',
                        color: 'var(--text-muted)',
                      }}
                    >
                      v{sig.version}
                    </span>
                    <span
                      style={{
                        fontSize: '10px',
                        padding: '1px 5px',
                        borderRadius: '3px',
                        background: sig.output_type === 'bool' ? 'rgba(0, 230, 118, 0.15)' : 'rgba(88, 166, 255, 0.15)',
                        color: sig.output_type === 'bool' ? 'var(--color-buy)' : 'var(--color-primary)',
                        fontWeight: 600,
                      }}
                    >
                      {sig.output_type.toUpperCase()}
                    </span>
                  </div>
                </div>

                {/* System identifier & AST alias */}
                <div style={{ display: 'flex', gap: '8px', fontSize: '11px', color: 'var(--text-muted)', marginBottom: '8px' }}>
                  <span>{sig.name}</span>
                  <span>•</span>
                  <span style={{ fontFamily: 'monospace', color: '#58A6FF' }}>AST: {sig.ast_alias}</span>
                </div>

                {/* Short meaning / description */}
                <p style={{ fontSize: '12px', color: 'var(--text-muted)', margin: '0 0 10px 0', lineHeight: 1.4 }}>
                  {sig.description}
                </p>

                {/* Warmup & Causal Delay */}
                <div style={{ display: 'flex', gap: '12px', fontSize: '11px', color: 'var(--text-muted)', marginBottom: '8px' }}>
                  <span>Warmup: <strong>{sig.warmup_bars} bars</strong></span>
                  {sig.causal_delay_bars > 0 && (
                    <span style={{ color: '#FFD166' }}>
                      Độ trễ xác nhận: <strong>+{sig.causal_delay_bars} bars</strong>
                    </span>
                  )}
                </div>

                {/* Parameters & Defaults */}
                {paramKeys.length > 0 && (
                  <div
                    style={{
                      background: 'rgba(0, 0, 0, 0.2)',
                      padding: '8px',
                      borderRadius: '4px',
                      fontSize: '11px',
                      marginBottom: '8px',
                    }}
                  >
                    <div style={{ fontWeight: 600, marginBottom: '4px', color: 'var(--text-main)' }}>
                      Tham số ({paramKeys.length}):
                    </div>
                    {paramKeys.map((pKey) => {
                      const schema: SignalParameterSchema = sig.parameters_schema[pKey];
                      const defVal = sig.default_parameters[pKey];
                      return (
                        <div key={pKey} style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '2px' }}>
                          <span style={{ fontFamily: 'monospace' }}>{pKey}:</span>
                          <span style={{ color: 'var(--text-main)', fontWeight: 500 }}>
                            {String(defVal)} ({schema?.type || 'num'})
                          </span>
                        </div>
                      );
                    })}
                  </div>
                )}

                {/* Advanced parameters view (UI-SIG-003) */}
                {showAdvancedParams && paramKeys.length > 0 && (
                  <div
                    data-testid="advanced-params-details"
                    style={{
                      background: 'rgba(41, 98, 255, 0.05)',
                      border: '1px dashed rgba(88, 166, 255, 0.3)',
                      padding: '8px',
                      borderRadius: '4px',
                      fontSize: '11px',
                      marginBottom: '8px',
                    }}
                  >
                    <div style={{ fontWeight: 600, color: '#58A6FF', marginBottom: '4px' }}>Chi tiết biên tham số:</div>
                    {paramKeys.map((pKey) => {
                      const schema = sig.parameters_schema[pKey];
                      return (
                        <div key={pKey} style={{ marginBottom: '4px' }}>
                          <div><strong>{pKey}</strong>: {schema?.description || 'Tham số cấu hình'}</div>
                          <div style={{ color: 'var(--text-muted)' }}>
                            Phạm vi: [{schema?.minimum ?? '-∞'}, {schema?.maximum ?? '+∞'}]
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>

              {/* Action Button */}
              {onSelectSignal && (
                <div style={{ marginTop: '10px', paddingTop: '8px', borderTop: '1px solid rgba(255, 255, 255, 0.06)' }}>
                  <button
                    type="button"
                    style={{
                      width: '100%',
                      padding: '6px',
                      fontSize: '12px',
                      background: isSelected ? 'var(--color-primary)' : 'rgba(255, 255, 255, 0.08)',
                      color: isSelected ? '#fff' : 'var(--text-main)',
                      border: 'none',
                      borderRadius: '4px',
                      cursor: 'pointer',
                      fontWeight: 500,
                    }}
                  >
                    {isSelected ? '✓ Đang chọn tín hiệu này' : '🔍 Chọn để kiểm tra / cấu hình'}
                  </button>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};
