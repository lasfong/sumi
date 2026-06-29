import React, { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { getSymbols } from '../../api/symbolsApi';
import { listReplaySessions } from '../../api/replayApi';
import type { CreateSessionRequest, ReplaySession, StockSymbol } from '../../types';

interface SessionSetupProps {
  onCreateSession: (data: CreateSessionRequest) => void;
  onResumeSession?: (sessionId: number) => void;
  isLoading?: boolean;
}

export const SessionSetup: React.FC<SessionSetupProps> = ({ onCreateSession, onResumeSession, isLoading }) => {
  const [symbol, setSymbol] = useState('');
  const [search, setSearch] = useState('');
  const [startDate, setStartDate] = useState('2023-01-01');
  const [endDate, setEndDate] = useState('2024-01-01');
  const [initialCash, setInitialCash] = useState('100000000');
  const [showDropdown, setShowDropdown] = useState(false);

  const { data: symbols } = useQuery({
    queryKey: ['symbols', search],
    queryFn: () => getSymbols({ search: search || undefined }),
    enabled: search.length > 0,
    staleTime: 30000,
  });

  const { data: recentSessions } = useQuery({
    queryKey: ['replay-sessions', 8],
    queryFn: () => listReplaySessions(8),
    staleTime: 30000,
  });

  const handleSubmit = () => {
    if (!symbol) return;
    onCreateSession({
      symbol,
      start_date: startDate,
      end_date: endDate,
      initial_cash: parseFloat(initialCash) || 100000000,
    });
  };

  const selectSymbol = (sym: string) => {
    setSymbol(sym);
    setSearch(sym);
    setShowDropdown(false);
  };

  const formatSessionDate = (value: string) => new Date(value).toLocaleDateString();

  return (
    <div style={{ maxWidth: '980px', margin: '2rem auto', display: 'grid', gridTemplateColumns: 'minmax(320px, 500px) minmax(280px, 1fr)', gap: '1rem', alignItems: 'start' }}>
      <div className="panel" style={{ padding: '2rem' }}>
        <h2 style={{ marginTop: 0, marginBottom: '1.5rem', color: 'var(--color-primary)' }}>
          New Replay Session
        </h2>

        <div style={{ marginBottom: '1rem', position: 'relative' }}>
          <label style={{ display: 'block', marginBottom: '0.5rem', color: 'var(--text-muted)', fontSize: '13px' }}>Symbol</label>
          <input
            value={search}
            onChange={(e) => {
              setSearch(e.target.value);
              setSymbol(e.target.value.toUpperCase());
              setShowDropdown(true);
            }}
            onFocus={() => search.length > 0 && setShowDropdown(true)}
            onBlur={() => setTimeout(() => setShowDropdown(false), 200)}
            placeholder="Search symbol (e.g. VNINDEX, VN30, FPT...)"
            style={{ width: '100%' }}
          />
          {showDropdown && symbols && symbols.length > 0 && (
            <div style={{
              position: 'absolute', top: '100%', left: 0, right: 0,
              background: 'var(--bg-panel)', border: '1px solid var(--border-color)',
              borderRadius: '0 0 6px 6px', maxHeight: '200px', overflowY: 'auto', zIndex: 10,
            }}>
              {symbols.map((s: StockSymbol) => (
                <div
                  key={s.symbol}
                  onClick={() => selectSymbol(s.symbol)}
                  style={{
                    padding: '0.5rem 1rem', cursor: 'pointer',
                    borderBottom: '1px solid var(--border-color)',
                  }}
                  onMouseEnter={(e) => (e.currentTarget.style.background = 'var(--bg-dark)')}
                  onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                >
                  <span style={{ fontWeight: 'bold' }}>{s.symbol}</span>
                  {s.company_name && <span style={{ color: 'var(--text-muted)', marginLeft: '0.5rem', fontSize: '12px' }}>{s.company_name}</span>}
                </div>
              ))}
            </div>
          )}
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem', marginBottom: '1rem' }}>
          <div>
            <label style={{ display: 'block', marginBottom: '0.5rem', color: 'var(--text-muted)', fontSize: '13px' }}>Start Date</label>
            <input type="date" value={startDate} onChange={(e) => setStartDate(e.target.value)} style={{ width: '100%' }} />
          </div>
          <div>
            <label style={{ display: 'block', marginBottom: '0.5rem', color: 'var(--text-muted)', fontSize: '13px' }}>End Date</label>
            <input type="date" value={endDate} onChange={(e) => setEndDate(e.target.value)} style={{ width: '100%' }} />
          </div>
        </div>

        <div style={{ marginBottom: '1.5rem' }}>
          <label style={{ display: 'block', marginBottom: '0.5rem', color: 'var(--text-muted)', fontSize: '13px' }}>Initial Cash (VND)</label>
          <input
            type="number"
            value={initialCash}
            onChange={(e) => setInitialCash(e.target.value)}
            style={{ width: '100%' }}
          />
        </div>

        <button
          onClick={handleSubmit}
          disabled={!symbol || isLoading}
          style={{
            width: '100%',
            background: 'var(--color-primary)',
            color: 'white',
            padding: '12px',
            fontSize: '16px',
            fontWeight: 700,
          }}
        >
          {isLoading ? 'Creating...' : 'Start Replay'}
        </button>
      </div>

      <div className="panel" style={{ padding: '1.25rem' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', gap: '12px', alignItems: 'center', marginBottom: '1rem' }}>
          <h3 style={{ margin: 0, fontSize: '16px' }}>Recent Sessions</h3>
          <span style={{ color: 'var(--text-muted)', fontSize: '12px' }}>{recentSessions?.length || 0} saved</span>
        </div>

        {!recentSessions?.length && (
          <p style={{ color: 'var(--text-muted)', fontSize: '14px', margin: 0 }}>No replay sessions yet.</p>
        )}

        <div style={{ display: 'grid', gap: '8px' }}>
          {(recentSessions || []).map((session: ReplaySession) => (
            <div key={session.id} style={{ border: '1px solid var(--border-color)', borderRadius: '6px', padding: '10px', background: 'rgba(255,255,255,0.02)' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', gap: '8px', marginBottom: '6px' }}>
                <strong>{session.symbol}</strong>
                <span style={{ color: session.status === 'completed' ? 'var(--color-buy)' : 'var(--color-primary)', fontSize: '12px', textTransform: 'uppercase' }}>
                  {session.status}
                </span>
              </div>
              <div style={{ display: 'grid', gap: '4px', color: 'var(--text-muted)', fontSize: '12px', marginBottom: '8px' }}>
                <span>{session.start_date} to {session.end_date}</span>
                <span>Updated {formatSessionDate(session.updated_at)}</span>
              </div>
              <button
                type="button"
                onClick={() => onResumeSession?.(session.id)}
                style={{ width: '100%', padding: '8px', fontSize: '13px' }}
              >
                Resume Session #{session.id}
              </button>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
