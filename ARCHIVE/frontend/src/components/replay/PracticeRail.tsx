import React, { useEffect, useMemo, useRef, useState } from 'react';

type RailTab = 'trade' | 'journal' | 'decisions' | 'drawing';

interface PracticeRailProps {
  trade: React.ReactNode;
  decisions: React.ReactNode;
  journal?: React.ReactNode;
  drawing?: React.ReactNode;
  selectedDrawingId?: string;
}

export const PracticeRail: React.FC<PracticeRailProps> = ({
  trade,
  decisions,
  journal,
  drawing,
  selectedDrawingId,
}) => {
  const [prevSelectedDrawingId, setPrevSelectedDrawingId] = useState(selectedDrawingId);
  const [tab, setTab] = useState<RailTab>(selectedDrawingId ? 'drawing' : 'trade');

  if (selectedDrawingId !== prevSelectedDrawingId) {
    setPrevSelectedDrawingId(selectedDrawingId);
    if (selectedDrawingId) {
      setTab('drawing');
    }
  }

  const tabs: Array<[RailTab, string]> = useMemo(() => {
    const list: Array<[RailTab, string]> = [['trade', 'Trade']];
    if (journal) list.push(['journal', 'Journal']);
    list.push(['decisions', 'Decisions']);
    if (drawing) list.push(['drawing', 'Drawing']);
    return list;
  }, [journal, drawing]);

  const panels: Record<RailTab, React.ReactNode> = {
    trade,
    journal: journal ?? null,
    decisions,
    drawing: drawing ?? null,
  };
  const tabRefs = useRef<Partial<Record<RailTab, HTMLButtonElement>>>({});
  const activateTab = (next: RailTab) => {
    setTab(next);
    tabRefs.current[next]?.focus();
  };

  useEffect(() => {
    const handleSwitchTab = (e: Event) => {
      const targetTab = (e as CustomEvent<RailTab>).detail;
      if (targetTab && ['trade', 'journal', 'decisions', 'drawing'].includes(targetTab)) {
        activateTab(targetTab);
      }
    };
    window.addEventListener('sumi:switch-tab', handleSwitchTab);
    return () => window.removeEventListener('sumi:switch-tab', handleSwitchTab);
  }, []);

  const handleTabKey = (event: React.KeyboardEvent<HTMLButtonElement>, current: RailTab) => {
    const index = tabs.findIndex(([id]) => id === current);
    let next: RailTab | undefined;
    if (event.key === 'ArrowRight' || event.key === 'ArrowDown') next = tabs[(index + 1) % tabs.length][0];
    if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') next = tabs[(index - 1 + tabs.length) % tabs.length][0];
    if (event.key === 'Home') next = tabs[0][0];
    if (event.key === 'End') next = tabs[tabs.length - 1][0];
    if (!next) return;
    event.preventDefault();
    event.stopPropagation();
    activateTab(next);
  };

  return (
    <aside
      className="replay-details-region practice-rail"
      data-testid="practice-rail"
      style={{
        flex: '0 0 clamp(250px, 25%, 320px)',
        borderLeft: '1px solid var(--border-color)',
        minWidth: 0,
        display: 'flex',
        flexDirection: 'column',
        overflow: 'hidden',
      }}
    >
      <div
        role="tablist"
        aria-label="Practice workspace"
        style={{
          display: 'grid',
          gridTemplateColumns: `repeat(${tabs.length}, 1fr)`,
          gap: 2,
          padding: 6,
          borderBottom: '1px solid var(--border-color)',
        }}
      >
        {tabs.map(([id, label]) => (
          <button
            key={id}
            ref={(element) => {
              if (element) tabRefs.current[id] = element;
            }}
            id={`practice-tab-${id}`}
            role="tab"
            aria-selected={tab === id}
            aria-controls={`practice-panel-${id}`}
            tabIndex={tab === id ? 0 : -1}
            onKeyDown={(event) => handleTabKey(event, id)}
            onClick={() => setTab(id)}
            style={{
              padding: '7px 2px',
              fontSize: 11,
              fontWeight: tab === id ? 600 : 400,
              color: tab === id ? 'white' : 'var(--text-muted)',
              borderBottom: tab === id ? '2px solid var(--color-primary)' : '2px solid transparent',
              background: 'transparent',
              borderRadius: 0,
              cursor: 'pointer',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {label}
          </button>
        ))}
      </div>
      {tabs.map(([id]) => (
        <div
          key={id}
          id={`practice-panel-${id}`}
          role="tabpanel"
          aria-labelledby={`practice-tab-${id}`}
          data-testid={`practice-tab-${id}`}
          hidden={tab !== id}
          style={{ padding: 8, overflowY: 'auto', minHeight: 0 }}
        >
          {tab === id ? panels[id] : null}
        </div>
      ))}
    </aside>
  );
};
