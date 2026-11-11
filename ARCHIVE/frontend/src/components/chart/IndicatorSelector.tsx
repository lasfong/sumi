import React, { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { getIndicatorRegistry } from '../../api/indicatorsApi';
import type { IndicatorDefinition } from '../../api/indicatorsApi';

export interface IndicatorConfig {
  name: string;
  pane: 'main' | 'oscillator';
  params: Record<string, string | number | boolean>;
  color?: string;
}

interface IndicatorSelectorProps {
  onAddIndicator: (config: IndicatorConfig) => void;
  onClear: () => void;
}

export const IndicatorSelector: React.FC<IndicatorSelectorProps> = ({ onAddIndicator, onClear }) => {
  const [showMenu, setShowMenu] = useState(false);
  const { data: definitions = [], isLoading } = useQuery({
    queryKey: ['indicator-registry'],
    queryFn: getIndicatorRegistry,
    staleTime: Infinity,
  });
  const categories = definitions.reduce<Record<string, IndicatorDefinition[]>>((groups, definition) => {
    (groups[definition.category] ??= []).push(definition);
    return groups;
  }, {});

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
            
            {isLoading && <span style={{ padding: '8px', color: 'var(--text-muted)', fontSize: '12px' }}>Loading indicators...</span>}
            {Object.entries(categories).map(([category, indicators], idx) => (
              <div key={category} style={{ marginBottom: idx < Object.keys(categories).length - 1 ? '12px' : 0 }}>
                <h4 style={{ margin: '0 0 4px 8px', fontSize: '11px', textTransform: 'uppercase', color: 'var(--text-muted)' }}>{category}</h4>
                {(indicators ?? []).map(ind => {
                  const params = ind.params.reduce<Record<string, string | number | boolean>>((values, param) => {
                    if (param.default !== null) values[param.name] = param.default;
                    return values;
                  }, {});
                  const config: IndicatorConfig = { name: ind.id, pane: ind.pane, params };
                  return (
                  <button 
                    key={ind.id}
                    style={{ display: 'block', width: '100%', background: 'transparent', color: 'white', border: 'none', textAlign: 'left', padding: '6px 8px', fontSize: '13px', borderRadius: '4px', cursor: 'pointer' }}
                    onMouseEnter={e => e.currentTarget.style.background = 'rgba(255,255,255,0.1)'}
                    onMouseLeave={e => e.currentTarget.style.background = 'transparent'}
                    onClick={() => {
                      onAddIndicator(config);
                      setShowMenu(false);
                    }}
                  >
                    {ind.label}
                  </button>
                )})}
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
