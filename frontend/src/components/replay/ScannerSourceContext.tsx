import React from 'react';
import type { ReplaySourceContext } from '../../types';
import { toDateKey } from '../../utils/date';

interface ScannerSourceContextProps {
  context: ReplaySourceContext;
  display?: 'status' | 'details';
}

export const ScannerSourceContext: React.FC<ScannerSourceContextProps> = ({ context, display = 'status' }) => {
  if (context.source_type !== 'scanner_signal' || !context.replay_intent) return null;

  const isReview = context.replay_intent === 'signal_review';
  const signal = context.revealed ? context.signal : null;
  const modeLabel = isReview ? 'Signal review' : 'Blind practice';

  if (display === 'status') {
    return (
      <>
      <span
        data-testid="scanner-replay-intent"
        style={{
          padding: '4px 8px',
          background: isReview ? 'rgba(255, 209, 102, 0.12)' : 'rgba(41, 98, 255, 0.12)',
          color: isReview ? '#FFD166' : '#8fb3ff',
          borderRadius: '4px',
          fontWeight: 600,
          fontSize: '12px',
          border: `1px solid ${isReview ? 'rgba(255, 209, 102, 0.35)' : 'rgba(41, 98, 255, 0.35)'}`,
        }}
      >
        {modeLabel}
      </span>
      {!signal && (
        <span data-testid="scanner-signal-hidden" style={{ color: 'var(--text-muted)', fontSize: '12px' }}>
          Signal context hidden until its candle
        </span>
      )}
      </>
    );
  }

  if (!signal) return null;
  return (
    <section
          data-testid="scanner-signal-context"
          aria-label={`${modeLabel} revealed signal context`}
          className="panel"
          style={{ padding: '12px', borderColor: 'rgba(255, 209, 102, 0.35)' }}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', gap: '8px', alignItems: 'center', marginBottom: '8px' }}>
            <h3 style={{ margin: 0, fontSize: '14px', color: '#FFD166' }}>Scanner Signal</h3>
            <span style={{ color: 'var(--text-muted)', fontSize: '12px' }}>{toDateKey(signal.timestamp)}</span>
          </div>
          <div style={{ display: 'grid', gap: '6px', fontSize: '12px' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', gap: '8px' }}>
              <span style={{ color: 'var(--text-muted)' }}>Strategy</span>
              <span style={{ textAlign: 'right' }}>{signal.strategy}</span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', gap: '8px' }}>
              <span style={{ color: 'var(--text-muted)' }}>Signal</span>
              <span style={{ textTransform: 'uppercase' }}>{signal.type}</span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', gap: '8px' }}>
              <span style={{ color: 'var(--text-muted)' }}>Regime</span>
              <span>{signal.regime || 'n/a'}</span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', gap: '8px' }}>
              <span style={{ color: 'var(--text-muted)' }}>Signal Price</span>
              <span style={{ fontFamily: 'monospace' }}>{signal.price.toLocaleString()}</span>
            </div>
          </div>
    </section>
  );
};
