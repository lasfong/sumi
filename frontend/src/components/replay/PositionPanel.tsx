import React from 'react';
import type { PracticePosition, PracticeWorkflowSnapshot } from '../../types';

interface PositionPanelProps {
  positions: PracticePosition[];
  snapshot: PracticeWorkflowSnapshot;
}

export const PositionPanel: React.FC<PositionPanelProps> = ({ positions, snapshot }) => {
  return (
    <div className="panel" style={{ flex: 1, overflowY: 'auto' }}>
      <h3 style={{ marginTop: 0, fontSize: '14px', textTransform: 'uppercase', letterSpacing: '0.5px', color: 'var(--text-muted)' }}>
        Position &amp; P/L
      </h3>
      <div data-testid="practice-cash" style={{ display: 'flex', justifyContent: 'space-between', fontSize: 12, marginBottom: 8 }}><span style={{ color: 'var(--text-muted)' }}>Projected cash</span><strong>{snapshot.current_cash.toLocaleString()}</strong></div>
      {positions.length === 0 && (
        <p style={{ color: 'var(--text-muted)', fontSize: '13px', fontStyle: 'italic' }}>No open positions.</p>
      )}
      {positions.map((p) => (
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
            <span style={{ color: 'var(--text-muted)' }}>Avg Price:</span>
            <span>{p.average_price.toLocaleString()}</span>
          </div>
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '13px' }}><span style={{ color: 'var(--text-muted)' }}>Current Price:</span><span>{p.current_price.toLocaleString()}</span></div>
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '13px' }}><span style={{ color: 'var(--text-muted)' }}>Realized PnL:</span><span>{p.realized_pnl.toLocaleString()}</span></div>
          <div data-testid="t2-available" style={{ display: 'flex', justifyContent: 'space-between', fontSize: '13px' }}><span style={{ color: 'var(--text-muted)' }}>T+2 Available:</span><strong>{p.available_quantity.toLocaleString()}</strong></div>
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '13px' }}>
            <span style={{ color: 'var(--text-muted)' }}>Unrealized PnL:</span>
            <span style={{ color: p.unrealized_pnl >= 0 ? 'var(--color-buy)' : 'var(--color-sell)', fontWeight: 600 }}>
              {p.unrealized_pnl.toLocaleString()}
            </span>
          </div>
        </div>
      ))}
    </div>
  );
};
