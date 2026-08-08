import React, { useState, useMemo, useRef } from 'react';
import { useQuery } from '@tanstack/react-query';
import { listReplaySessions } from '../../api/replayApi';
import type { ReplaySession } from '../../types';
import { formatVietnameseDate } from '../../utils/formatters';
import { Search, ChevronDown, Check, FolderOpen, AlertCircle, RefreshCw } from 'lucide-react';
import './SessionPicker.css';

export interface SessionPickerProps {
  selectedSessionId: number | null;
  onSelectSession: (sessionId: number) => void;
  className?: string;
  placeholder?: string;
  compact?: boolean;
}

export const SessionPicker: React.FC<SessionPickerProps> = ({
  selectedSessionId,
  onSelectSession,
  className = '',
  placeholder = 'Choose practice session...',
  compact = false,
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const searchInputRef = useRef<HTMLInputElement>(null);

  const {
    data: sessions = [],
    isLoading,
    isError,
    refetch,
  } = useQuery({
    queryKey: ['replay-sessions-list'],
    queryFn: () => listReplaySessions(50),
    staleTime: 10000,
  });

  const selectedSession = useMemo(() => {
    if (!selectedSessionId) return null;
    return sessions.find((s) => s.id === selectedSessionId) || null;
  }, [sessions, selectedSessionId]);

  const filteredSessions = useMemo(() => {
    if (!searchQuery.trim()) return sessions;
    const query = searchQuery.toLowerCase().trim();
    return sessions.filter((s) => {
      const matchId = String(s.id).includes(query);
      const matchSymbol = s.symbol.toLowerCase().includes(query);
      const matchTimeframe = (s.timeframe || '').toLowerCase().includes(query);
      const matchMode = (s.source_context?.replay_intent || '').toLowerCase().includes(query);
      const matchStatus = (s.status || '').toLowerCase().includes(query);
      return matchId || matchSymbol || matchTimeframe || matchMode || matchStatus;
    });
  }, [sessions, searchQuery]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    // STOP PROPAGATION to isolate form keyboard focus from Replay/chart shortcuts (PRO-UX-08)
    e.stopPropagation();

    if (e.key === 'Escape') {
      setIsOpen(false);
      return;
    }

    if (!isOpen) {
      if (e.key === 'ArrowDown' || e.key === 'Enter') {
        setIsOpen(true);
      }
      return;
    }

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((prev) => (prev < filteredSessions.length - 1 ? prev + 1 : 0));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((prev) => (prev > 0 ? prev - 1 : Math.max(0, filteredSessions.length - 1)));
    } else if (e.key === 'Enter' && filteredSessions.length > 0) {
      e.preventDefault();
      const target = filteredSessions[selectedIndex] || filteredSessions[0];
      if (target) {
        onSelectSession(target.id);
        setIsOpen(false);
      }
    }
  };

  const formatSessionMode = (session: ReplaySession) => {
    const intent = session.source_context?.replay_intent;
    if (intent === 'signal_review') return 'Signal Review';
    if (intent === 'blind_practice') return 'Blind Practice';
    return 'Replay Practice';
  };

  return (
    <div className={`session-picker-container ${compact ? 'compact' : ''} ${className}`}>
      <button
        type="button"
        className={`session-picker-trigger ${isOpen ? 'active' : ''}`}
        onClick={() => {
          setIsOpen(!isOpen);
          if (!isOpen) setTimeout(() => searchInputRef.current?.focus(), 50);
        }}
        onKeyDown={handleKeyDown}
        aria-label="Select session"
        aria-expanded={isOpen}
        aria-haspopup="listbox"
      >
        <FolderOpen size={16} className="trigger-icon" />
        <span className="trigger-label">
          {selectedSession ? (
            <span className="selected-session-info">
              <strong className="session-id">#{selectedSession.id}</strong>
              <span className="session-symbol">{selectedSession.symbol}</span>
              <span className="session-tf">({selectedSession.timeframe || 'D'})</span>
              <span className="session-mode-tag">
                {selectedSession.source_context?.replay_intent === 'signal_review'
                  ? 'Review'
                  : 'Blind'}
              </span>
            </span>
          ) : (
            <span className="placeholder">{placeholder}</span>
          )}
        </span>
        <ChevronDown size={16} className={`dropdown-arrow ${isOpen ? 'open' : ''}`} />
      </button>

      {isOpen && (
        <div className="session-picker-dropdown glass-panel">
          <div className="picker-search-bar">
            <Search size={14} className="search-icon" />
            <input
              ref={searchInputRef}
              type="text"
              className="picker-search-input"
              placeholder="Search session by symbol, mode, or ID..."
              value={searchQuery}
              onChange={(e) => {
                setSearchQuery(e.target.value);
                setSelectedIndex(0);
              }}
              onKeyDown={handleKeyDown}
              aria-label="Search replay sessions"
            />
          </div>

          <div className="picker-session-list" role="listbox">
            {isLoading ? (
              <div className="picker-state-msg">
                <RefreshCw size={16} className="animate-spin" />
                <span>Loading available sessions...</span>
              </div>
            ) : isError ? (
              <div className="picker-state-msg error">
                <AlertCircle size={16} />
                <span>Failed to load sessions.</span>
                <button type="button" className="btn-retry" onClick={() => refetch()}>
                  Retry
                </button>
              </div>
            ) : filteredSessions.length === 0 ? (
              <div className="picker-state-msg empty">
                <span>No matching replay sessions found.</span>
              </div>
            ) : (
              filteredSessions.map((session, index) => {
                const isSelected = session.id === selectedSessionId;
                const isFocused = index === selectedIndex;
                const modeLabel = formatSessionMode(session);
                const modeClass = session.source_context?.replay_intent || 'blind_practice';

                return (
                  <div
                    key={session.id}
                    className={`picker-session-item ${isSelected ? 'selected' : ''} ${
                      isFocused ? 'focused' : ''
                    }`}
                    role="option"
                    aria-selected={isSelected}
                    onClick={() => {
                      onSelectSession(session.id);
                      setIsOpen(false);
                    }}
                  >
                    <div className="item-header">
                      <span className="item-title">
                        <strong className="item-id">#{session.id}</strong> {session.symbol}
                      </span>
                      <span className={`item-badge ${modeClass}`}>{modeLabel}</span>
                    </div>

                    <div className="item-details">
                      <span className="detail-tag">TF: {session.timeframe || 'D'}</span>
                      <span className="detail-tag">Index: {session.current_index}</span>
                      <span className="detail-date">
                        {session.start_date ? formatVietnameseDate(session.start_date) : ''}
                      </span>
                    </div>

                    {isSelected && <Check size={14} className="check-icon" />}
                  </div>
                );
              })
            )}
          </div>
        </div>
      )}
    </div>
  );
};
