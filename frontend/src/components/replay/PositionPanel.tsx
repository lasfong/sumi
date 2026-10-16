import React from 'react';
import type { PracticePosition, PracticeWorkflowSnapshot } from '../../types';

interface PositionPanelProps {
  positions: PracticePosition[];
  snapshot: PracticeWorkflowSnapshot;
}

export const PositionPanel: React.FC<PositionPanelProps> = ({ positions, snapshot }) => {
  const openTrade = (snapshot.trades || []).find(t => t.status === 'open');
  const settlementLabel = snapshot.settlement_label || 'T+1 · về sau 1 phiên';

  return (
    <div className="panel" data-testid="position-panel" style={{ flex: 1, overflowY: 'auto' }}>
      <h3 style={{ marginTop: 0, fontSize: '14px', textTransform: 'uppercase', letterSpacing: '0.5px', color: 'var(--text-muted)' }}>
        Vị Thế &amp; Setup Hiện Tại
      </h3>
      {positions.length === 0 && (
        <p style={{ color: 'var(--text-muted)', fontSize: '13px', fontStyle: 'italic' }}>Chưa có vị thế đang mở.</p>
      )}
      {positions.map((p) => {
        const isShort = p.quantity < 0;
        const absQty = Math.abs(p.quantity);
        const availableQty = Math.max(0, p.available_quantity ?? snapshot.available_quantity ?? 0);
        const unsettledQty = Math.max(
          0,
          p.blocked_quantity !== undefined ? p.blocked_quantity : absQty - availableQty,
        );
        const releaseDate = (p.earliest_release_date ?? snapshot.earliest_release_date)?.slice(0, 10) || null;

        const priceDelta = isShort ? (p.average_price - p.current_price) : (p.current_price - p.average_price);
        const pnlPct = p.average_price > 0 ? (priceDelta / p.average_price) * 100 : 0;

        let riskPerShare: number | null = null;
        if (openTrade?.initial_stop_loss != null) {
          if (!isShort && p.average_price > openTrade.initial_stop_loss) {
            riskPerShare = p.average_price - openTrade.initial_stop_loss;
          } else if (isShort && openTrade.initial_stop_loss > p.average_price) {
            riskPerShare = openTrade.initial_stop_loss - p.average_price;
          }
        }
        const currentR = riskPerShare && riskPerShare > 0 ? priceDelta / riskPerShare : null;

        const slPct =
          openTrade?.initial_stop_loss != null && p.average_price > 0
            ? isShort
              ? -(((openTrade.initial_stop_loss - p.average_price) / p.average_price) * 100)
              : ((openTrade.initial_stop_loss - p.average_price) / p.average_price) * 100
            : null;

        const tpPct =
          openTrade?.target_price != null && p.average_price > 0
            ? isShort
              ? ((p.average_price - openTrade.target_price) / p.average_price) * 100
              : ((openTrade.target_price - p.average_price) / p.average_price) * 100
            : null;

        return (
          <div
            key={p.id}
            style={{
              padding: '0.75rem',
              border: '1px solid var(--border-color)',
              borderRadius: '6px',
              marginBottom: '0.5rem',
              background: 'var(--bg-dark)',
            }}
          >
            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '4px' }}>
              <span style={{ fontWeight: 'bold' }}>{p.symbol}</span>
              <span
                style={{
                  color: !isShort ? 'var(--color-buy)' : 'var(--color-sell)',
                  fontSize: '13px',
                  fontWeight: 600,
                }}
              >
                {!isShort ? 'LONG' : 'SHORT'} {absQty.toLocaleString()}
              </span>
            </div>

            <div
              data-testid="position-settlement-summary"
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                fontSize: '12px',
                padding: '4px 6px',
                marginBottom: '6px',
                borderRadius: '4px',
                background: unsettledQty > 0 ? 'rgba(255, 209, 102, 0.1)' : 'rgba(0, 230, 118, 0.08)',
                border: `1px solid ${unsettledQty > 0 ? 'rgba(255, 209, 102, 0.3)' : 'rgba(0, 230, 118, 0.25)'}`,
              }}
            >
              <span>
                Khả dụng: <strong data-testid="position-available-qty">{availableQty.toLocaleString()}</strong>
              </span>
              <span>
                Chờ về ({settlementLabel}): <strong data-testid="position-unsettled-qty">{unsettledQty.toLocaleString()}</strong>
                {releaseDate ? ` · ${releaseDate}` : ''}
              </span>
            </div>

            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '13px' }}>
              <span style={{ color: 'var(--text-muted)' }}>Giá vào (Entry):</span>
              <span>{p.average_price.toLocaleString()}</span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '13px' }}>
              <span style={{ color: 'var(--text-muted)' }}>Giá hiện tại:</span>
              <span>{p.current_price.toLocaleString()}</span>
            </div>

            {openTrade?.initial_stop_loss != null && (
              <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '13px' }}>
                <span style={{ color: 'var(--text-muted)' }}>Cắt lỗ (SL):</span>
                <span style={{ color: 'var(--color-sell)' }}>
                  {openTrade.initial_stop_loss.toLocaleString()} ({slPct !== null ? `${slPct >= 0 ? '+' : ''}${slPct.toFixed(1)}%` : ''})
                </span>
              </div>
            )}

            {openTrade?.target_price != null && (
              <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '13px' }}>
                <span style={{ color: 'var(--text-muted)' }}>Chốt lời (TP):</span>
                <span style={{ color: 'var(--color-buy)' }}>
                  {openTrade.target_price.toLocaleString()} ({tpPct !== null ? `${tpPct >= 0 ? '+' : ''}${tpPct.toFixed(1)}%` : ''})
                </span>
              </div>
            )}

            {openTrade?.planned_r != null && (
              <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '13px' }}>
                <span style={{ color: 'var(--text-muted)' }}>Tỷ lệ R:R mục tiêu:</span>
                <strong style={{ color: '#58A6FF' }}>1 : {openTrade.planned_r.toFixed(2)}</strong>
              </div>
            )}

            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '13px', borderTop: '1px solid rgba(255,255,255,0.06)', paddingTop: '4px', marginTop: '4px' }}>
              <span style={{ color: 'var(--text-muted)' }}>Lãi/Lỗ tạm tính:</span>
              <span data-testid="position-unrealized-pnl" style={{ color: pnlPct >= 0 ? 'var(--color-buy)' : 'var(--color-sell)', fontWeight: 600 }}>
                {pnlPct >= 0 ? '+' : ''}{pnlPct.toFixed(2)}% {currentR !== null ? `(${currentR >= 0 ? '+' : ''}${currentR.toFixed(2)}R)` : ''}
              </span>
            </div>
          </div>
        );
      })}
    </div>
  );
};
