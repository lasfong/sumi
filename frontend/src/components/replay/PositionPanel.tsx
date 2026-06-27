import React from 'react';
import type { Position } from '../../types';

interface PositionPanelProps {
  positions: Position[];
}

export const PositionPanel: React.FC<PositionPanelProps> = ({ positions }) => {
  return (
    <div className="panel" style={{ flex: 1, overflowY: 'auto' }}>
      <h3 style={{ marginTop: 0, fontSize: '14px', textTransform: 'uppercase', letterSpacing: '0.5px', color: 'var(--text-muted)' }}>
        Open Positions
      </h3>
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
