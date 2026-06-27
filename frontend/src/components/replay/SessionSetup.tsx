import React, { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { getSymbols } from '../../api/symbolsApi';
import type { CreateSessionRequest, StockSymbol } from '../../types';

interface SessionSetupProps {
  onCreateSession: (data: CreateSessionRequest) => void;
  isLoading?: boolean;
}

export const SessionSetup: React.FC<SessionSetupProps> = ({ onCreateSession, isLoading }) => {
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

  return (
    <div className="panel" style={{ maxWidth: '500px', margin: '2rem auto', padding: '2rem' }}>
      <h2 style={{ marginTop: 0, marginBottom: '1.5rem', color: 'var(--color-primary)' }}>
        🎯 New Replay Session
      </h2>

      {/* Symbol search */}
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

      {/* Date range */}
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

      {/* Initial Cash */}
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
        {isLoading ? 'Creating...' : '▶ Start Replay'}
      </button>
    </div>
  );
};
