import React from 'react';
import type { DrawingType } from './CandleChart';

interface DrawingToolbarProps {
  activeTool: DrawingType;
  onSelectTool: (tool: DrawingType) => void;
  onClearAll: () => void;
}

export const DrawingToolbar: React.FC<DrawingToolbarProps> = ({ activeTool, onSelectTool, onClearAll }) => {
  const tools: { id: DrawingType; label: string; icon: React.ReactNode }[] = [
    {
      id: 'cursor',
      label: 'Cursor',
      icon: <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M3 3l7.07 16.97 2.51-7.39 7.39-2.51L3 3z"/></svg>
    },
    {
      id: 'trendline',
      label: 'Trendline',
      icon: <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M21 3L3 21M21 3v6M21 3h-6"/></svg>
    },
    {
      id: 'horizontal',
      label: 'Horizontal Line',
      icon: <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M3 12h18"/></svg>
    },
    {
      id: 'fibonacci',
      label: 'Fibonacci Retracement',
      icon: <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M4 4h16M4 12h16M4 20h16M4 8h8M4 16h8"/></svg>
    }
  ];

  return (
    <div style={{
      display: 'flex',
      flexDirection: 'column',
      gap: '8px',
      padding: '8px',
      background: 'var(--bg-panel)',
      borderRight: '1px solid var(--border-color)',
      width: '48px',
      alignItems: 'center'
    }}>
      {tools.map(t => (
        <button
          key={t.id}
          title={t.label}
          onClick={() => onSelectTool(t.id)}
          style={{
            background: activeTool === t.id ? 'rgba(41, 98, 255, 0.2)' : 'transparent',
            color: activeTool === t.id ? 'var(--color-primary)' : 'var(--text-muted)',
            border: `1px solid ${activeTool === t.id ? 'var(--color-primary)' : 'transparent'}`,
            borderRadius: '4px',
            padding: '8px',
            cursor: 'pointer',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            transition: 'all 0.2s'
          }}
          onMouseEnter={e => { if (activeTool !== t.id) e.currentTarget.style.color = 'white'; }}
          onMouseLeave={e => { if (activeTool !== t.id) e.currentTarget.style.color = 'var(--text-muted)'; }}
        >
          {t.icon}
        </button>
      ))}
      
      <div style={{ flex: 1 }} />
      
      <button
        title="Remove All Drawings"
        onClick={onClearAll}
        style={{
          background: 'transparent',
          color: 'var(--color-sell)',
          border: 'none',
          padding: '8px',
          cursor: 'pointer',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>
      </button>
    </div>
  );
};
