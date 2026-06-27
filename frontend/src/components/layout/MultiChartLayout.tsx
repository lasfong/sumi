import React from 'react';

interface MultiChartLayoutProps {
  children: React.ReactNode;
  layoutType: '1x1' | '1x2' | '2x2';
}

export const MultiChartLayout: React.FC<MultiChartLayoutProps> = ({ children, layoutType }) => {
  const childArray = React.Children.toArray(children);

  if (layoutType === '1x1' || childArray.length === 1) {
    return <div style={{ flex: 1, height: '100%' }}>{childArray[0]}</div>;
  }

  if (layoutType === '1x2') {
    return (
      <div style={{ display: 'flex', flex: 1, height: '100%', gap: '4px' }}>
        <div style={{ flex: 1, borderRight: '1px solid var(--border-color)' }}>
          {childArray[0]}
        </div>
        <div style={{ flex: 1 }}>
          {childArray[1] || <div style={{display:'flex', alignItems:'center', justifyContent:'center', height:'100%', color:'var(--text-muted)'}}>No Chart</div>}
        </div>
      </div>
    );
  }

  if (layoutType === '2x2') {
    return (
      <div style={{ display: 'flex', flexDirection: 'column', flex: 1, height: '100%', gap: '4px' }}>
        <div style={{ display: 'flex', flex: 1, gap: '4px' }}>
          <div style={{ flex: 1, borderRight: '1px solid var(--border-color)' }}>{childArray[0]}</div>
          <div style={{ flex: 1 }}>{childArray[1]}</div>
        </div>
        <div style={{ display: 'flex', flex: 1, gap: '4px', borderTop: '1px solid var(--border-color)' }}>
          <div style={{ flex: 1, borderRight: '1px solid var(--border-color)' }}>{childArray[2]}</div>
          <div style={{ flex: 1 }}>{childArray[3]}</div>
        </div>
      </div>
    );
  }

  return <>{childArray[0]}</>;
};
