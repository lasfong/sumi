import React from 'react';
import type { Decision } from '../../types';

interface DecisionJournalProps {
  decisions: Decision[];
}

const actionColor = (action: string): string => {
  switch (action) {
    case 'BUY':
    case 'ADD':
      return 'var(--color-buy)';
    case 'SELL':
    case 'REDUCE':
    case 'CUT_LOSS':
      return 'var(--color-sell)';
    case 'CLOSE':
    case 'TAKE_PROFIT':
      return 'var(--color-close)';
    default:
      return 'var(--text-muted)';
  }
};

export const DecisionJournal: React.FC<DecisionJournalProps> = ({ decisions }) => {
  const sorted = [...decisions].reverse();

  return (
    <div className="panel" style={{ flex: 1, overflowY: 'auto' }}>
      <h3 style={{ marginTop: 0, fontSize: '14px', textTransform: 'uppercase', letterSpacing: '0.5px', color: 'var(--text-muted)' }}>
        Decision Log
      </h3>
      {sorted.length === 0 && (
        <p style={{ color: 'var(--text-muted)', fontSize: '13px', fontStyle: 'italic' }}>No decisions yet.</p>
      )}
      {sorted.map((j) => (
        <div
          key={j.id}
          style={{
            padding: '0.75rem',
            border: '1px solid var(--border-color)',
            borderRadius: '6px',
            marginBottom: '0.5rem',
            background: 'var(--bg-dark)',
          }}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '4px' }}>
            <span style={{ fontWeight: 'bold', color: actionColor(j.action) }}>
              {j.action}
            </span>
            <span style={{ color: 'var(--text-muted)', fontSize: '12px' }}>
              {new Date(j.decision_date).toLocaleDateString()}
            </span>
          </div>
          {j.setup_type && (
            <div style={{ fontSize: '13px', color: 'var(--color-primary)' }}>
              Setup: {j.setup_type} {j.confidence_score ? `(${j.confidence_score}/5)` : ''}
            </div>
          )}
          {j.market_context && (
            <div style={{ fontSize: '12px', marginTop: '4px', color: 'var(--text-muted)' }}>
              Context: {j.market_context}
            </div>
          )}
          {j.mistake_tag && (
            <div style={{ fontSize: '12px', marginTop: '4px', color: 'var(--color-sell)' }}>
              Mistake: {j.mistake_tag}
            </div>
          )}
          {j.reason && (
            <div style={{ fontSize: '13px', marginTop: '4px', color: 'var(--text-muted)' }}>
              {j.reason}
            </div>
          )}
          {j.note && (
            <div style={{ fontSize: '12px', marginTop: '4px', color: 'var(--text-muted)', fontStyle: 'italic' }}>
              {j.note}
            </div>
          )}
        </div>
      ))}
    </div>
  );
};
