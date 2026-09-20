import React from 'react';
import type { PracticePosition, PracticeWorkflowSnapshot } from '../../types';

interface PositionPanelProps {
  positions: PracticePosition[];
  snapshot: PracticeWorkflowSnapshot;
}

export const PositionPanel: React.FC<PositionPanelProps> = ({ positions, snapshot }) => {
  const openTrade = (snapshot.trades || []).find(t => t.status === 'open');

  return (
    <div className="panel" style={{ flex: 1, overflowY: 'auto' }}>
      <h3 style={{ marginTop: 0, fontSize: '14px', textTransform: 'uppercase', letterSpacing: '0.5px', color: 'var(--text-muted)' }}>
        Vị Thế &amp; Setup Hiện Tại
      </h3>
      {positions.length === 0 && (
        <p style={{ color: 'var(--text-muted)', fontSize: '13px', fontStyle: 'italic' }}>Chưa có vị thế đang mở.</p>
      )}
      {positions.map((p) => {
        const pnlPct = p.average_price > 0 ? ((p.current_price - p.average_price) / p.average_price) * 100 : 0;
        const riskPerShare = (openTrade?.initial_stop_loss && p.average_price > openTrade.initial_stop_loss) ? (p.average_price - openTrade.initial_stop_loss) : null;
        const currentR = riskPerShare ? ((p.current_price - p.average_price) / riskPerShare) : null;

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
                  color: p.quantity > 0 ? 'var(--color-buy)' : 'var(--color-sell)',
                  fontSize: '13px',
                  fontWeight: 600,
                }}
              >
                {p.quantity > 0 ? 'LONG' : 'SHORT'} {Math.abs(p.quantity)}
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
                  {openTrade.initial_stop_loss.toLocaleString()} ({(((openTrade.initial_stop_loss - p.average_price) / p.average_price) * 100).toFixed(1)}%)
                </span>
              </div>
            )}

            {openTrade?.target_price != null && (
              <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '13px' }}>
                <span style={{ color: 'var(--text-muted)' }}>Chốt lời (TP):</span>
                <span style={{ color: 'var(--color-buy)' }}>
                  {openTrade.target_price.toLocaleString()} (+{(((openTrade.target_price - p.average_price) / p.average_price) * 100).toFixed(1)}%)
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
              <span style={{ color: pnlPct >= 0 ? 'var(--color-buy)' : 'var(--color-sell)', fontWeight: 600 }}>
                {pnlPct >= 0 ? '+' : ''}{pnlPct.toFixed(2)}% {currentR !== null ? `(${currentR >= 0 ? '+' : ''}${currentR.toFixed(2)}R)` : ''}
              </span>
            </div>
          </div>
        );
      })}
    </div>
  );
};
