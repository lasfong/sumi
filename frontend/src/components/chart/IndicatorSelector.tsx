import React, { useState } from 'react';

export interface IndicatorConfig {
  name: string;
  pane: 'main' | 'oscillator';
  params: Record<string, any>;
  color?: string;
}

interface IndicatorSelectorProps {
  onAddIndicator: (config: IndicatorConfig) => void;
  onClear: () => void;
}

export const IndicatorSelector: React.FC<IndicatorSelectorProps> = ({ onAddIndicator, onClear }) => {
  const [showMenu, setShowMenu] = useState(false);

  const categories = {
    'Trend': [
      { label: 'SMA (20)', config: { name: 'sma', pane: 'main', params: { length: 20 }, color: '#f1c40f' } },
      { label: 'EMA (20)', config: { name: 'ema', pane: 'main', params: { length: 20 }, color: '#f1c40f' } },
      { label: 'EMA (50)', config: { name: 'ema', pane: 'main', params: { length: 50 }, color: '#9b59b6' } },
      { label: 'MACD (12, 26, 9)', config: { name: 'macd', pane: 'oscillator', params: { fast: 12, slow: 26, signal: 9 } } },
      { label: 'Parabolic SAR', config: { name: 'psar', pane: 'main', params: { af0: 0.02, af: 0.02, max_af: 0.2 } } },
      { label: 'SuperTrend', config: { name: 'supertrend', pane: 'main', params: { length: 7, multiplier: 3.0 } } },
      { label: 'Ichimoku Cloud', config: { name: 'ichimoku', pane: 'main', params: {} } },
      { label: 'ADX (14)', config: { name: 'adx', pane: 'oscillator', params: { length: 14 } } },
    ],
    'Oscillators': [
      { label: 'RSI (14)', config: { name: 'rsi', pane: 'oscillator', params: { length: 14 }, color: '#e056fd' } },
      { label: 'Stochastic (14, 3, 3)', config: { name: 'stoch', pane: 'oscillator', params: { k: 14, d: 3, smooth_k: 3 } } },
      { label: 'CCI (20)', config: { name: 'cci', pane: 'oscillator', params: { length: 20 }, color: '#ffb8b8' } },
      { label: 'MFI (14)', config: { name: 'mfi', pane: 'oscillator', params: { length: 14 }, color: '#00E676' } },
    ],
    'Volatility': [
      { label: 'Bollinger Bands', config: { name: 'bbands', pane: 'main', params: { length: 20, std: 2.0 } } },
      { label: 'ATR (14)', config: { name: 'atr', pane: 'oscillator', params: { length: 14 }, color: '#7ed6df' } },
      { label: 'Keltner Channels', config: { name: 'kc', pane: 'main', params: { length: 20, scalar: 2.0 } } },
    ]
  };

  return (
    <div style={{ position: 'relative' }}>
      <button 
        onClick={() => setShowMenu(!showMenu)} 
        style={{ background: 'transparent', color: 'var(--text-main)', border: '1px solid var(--border-color)', fontSize: '12px', padding: '6px 12px', display: 'flex', alignItems: 'center', gap: '6px', borderRadius: '4px' }}
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M12 5v14M5 12h14"/></svg>
        Indicators
      </button>

      {showMenu && (
        <>
          <div style={{ position: 'fixed', inset: 0, zIndex: 99 }} onClick={() => setShowMenu(false)} />
          <div className="glass-panel-solid animate-fade-in" style={{ position: 'absolute', top: '100%', left: 0, marginTop: '8px', width: '220px', zIndex: 100, display: 'flex', flexDirection: 'column', padding: '8px' }}>
            
            {Object.entries(categories).map(([category, indicators], idx) => (
              <div key={category} style={{ marginBottom: idx < Object.keys(categories).length - 1 ? '12px' : 0 }}>
                <h4 style={{ margin: '0 0 4px 8px', fontSize: '11px', textTransform: 'uppercase', color: 'var(--text-muted)' }}>{category}</h4>
                {indicators.map((ind, i) => (
                  <button 
                    key={i}
                    style={{ display: 'block', width: '100%', background: 'transparent', color: 'white', border: 'none', textAlign: 'left', padding: '6px 8px', fontSize: '13px', borderRadius: '4px', cursor: 'pointer' }}
                    onMouseEnter={e => e.currentTarget.style.background = 'rgba(255,255,255,0.1)'}
                    onMouseLeave={e => e.currentTarget.style.background = 'transparent'}
                    onClick={() => {
                      onAddIndicator(ind.config as IndicatorConfig);
                      setShowMenu(false);
                    }}
                  >
                    {ind.label}
                  </button>
                ))}
              </div>
            ))}
            
            <div style={{ height: '1px', background: 'var(--border-color)', margin: '8px 0' }} />
            
            <button 
              style={{ background: 'transparent', color: 'var(--color-sell)', border: 'none', textAlign: 'left', padding: '8px', fontSize: '13px', borderRadius: '4px', cursor: 'pointer', display: 'flex', gap: '8px', alignItems: 'center' }}
              onMouseEnter={e => e.currentTarget.style.background = 'rgba(255,23,68,0.1)'}
              onMouseLeave={e => e.currentTarget.style.background = 'transparent'}
              onClick={() => {
                onClear();
                setShowMenu(false);
              }}
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>
              Remove All
            </button>
          </div>
        </>
      )}
    </div>
  );
};
