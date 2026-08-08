import React from 'react';
import { useQuery } from '@tanstack/react-query';
import { getJournalEntries } from '../api/journalApi';
import { getReplaySession } from '../api/replayApi';
import { useSessionSelection } from '../hooks/useSessionSelection';
import { SessionPicker } from '../components/common/SessionPicker';
import { formatVietnameseDate, formatVietnameseDateTime } from '../utils/formatters';
import type { JournalEntry } from '../types';
import { BookOpen, AlertCircle, FileText, ArrowRight } from 'lucide-react';
import { Link } from 'react-router-dom';

export const JournalPage: React.FC = () => {
  const { sessionId, selectSession, getSessionPath } = useSessionSelection();

  const { data: session } = useQuery({
    queryKey: ['replay-session', sessionId],
    queryFn: () => getReplaySession(sessionId!),
    enabled: !!sessionId,
  });

  const {
    data: entries,
    isLoading,
    isError,
  } = useQuery({
    queryKey: ['journal', sessionId],
    queryFn: () => getJournalEntries(sessionId!),
    enabled: !!sessionId,
  });

  return (
    <div style={{ padding: '2rem', maxWidth: '960px', margin: '0 auto' }}>
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: '2rem',
          flexWrap: 'wrap',
          gap: '1rem',
        }}
      >
        <div>
          <h2 style={{ margin: 0, display: 'flex', alignItems: 'center', gap: '8px' }}>
            <BookOpen size={24} style={{ color: 'var(--color-primary)' }} />
            Decision &amp; Checklist Journal
          </h2>
          <p style={{ margin: '4px 0 0 0', color: 'var(--text-muted)', fontSize: '13px' }}>
            Structured decision notes and checklist entries for active replay practice
          </p>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
          <SessionPicker selectedSessionId={sessionId} onSelectSession={selectSession} />

          {sessionId && (
            <Link
              to={getSessionPath('/replay')}
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: '4px',
                padding: '8px 12px',
                background: 'rgba(56, 152, 255, 0.1)',
                border: '1px solid rgba(56, 152, 255, 0.3)',
                color: 'var(--color-primary)',
                borderRadius: '6px',
                textDecoration: 'none',
                fontSize: '13px',
                fontWeight: 600,
              }}
            >
              <span>Back to Replay</span>
              <ArrowRight size={14} />
            </Link>
          )}
        </div>
      </div>

      {!sessionId ? (
        <div className="glass-panel" style={{ padding: '3rem', textAlign: 'center' }}>
          <FileText size={40} style={{ color: 'var(--text-muted)', marginBottom: '1rem' }} />
          <h3 style={{ margin: '0 0 8px 0' }}>No Replay Session Selected</h3>
          <p style={{ color: 'var(--text-muted)', margin: '0 0 1.5rem 0', fontSize: '14px' }}>
            Choose an existing replay session using the session picker above to view its journal records.
          </p>
        </div>
      ) : isLoading ? (
        <div className="glass-panel" style={{ padding: '2rem', textAlign: 'center' }}>
          <p style={{ color: 'var(--text-muted)', margin: 0 }}>Loading journal records...</p>
        </div>
      ) : isError ? (
        <div className="glass-panel" style={{ borderColor: 'var(--color-sell)', padding: '1.5rem' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px', color: 'var(--color-sell)' }}>
            <AlertCircle size={20} />
            <strong style={{ fontSize: '14px' }}>Failed to load journal entries for Session #{sessionId}.</strong>
          </div>
          <p style={{ color: 'var(--text-muted)', margin: '8px 0 0 0', fontSize: '13px' }}>
            Ensure the session exists and local backend server is running.
          </p>
        </div>
      ) : !entries || entries.length === 0 ? (
        <div className="glass-panel" style={{ padding: '2.5rem', textAlign: 'center' }}>
          <p style={{ color: 'var(--text-muted)', margin: '0 0 1rem 0' }}>
            No journal entries recorded for Session #{sessionId} ({session?.symbol || 'Symbol'}).
          </p>
          <Link
            to={getSessionPath('/replay')}
            className="btn-primary"
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '6px',
              padding: '8px 16px',
              borderRadius: '6px',
              textDecoration: 'none',
            }}
          >
            <span>Record Notes in Replay Lab</span>
            <ArrowRight size={14} />
          </Link>
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
          {session && (
            <div
              className="glass-panel"
              style={{
                padding: '12px 16px',
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                fontSize: '13px',
              }}
            >
              <div>
                <strong>Session #{session.id}</strong> — <span style={{ fontWeight: 600 }}>{session.symbol}</span> (
                {session.timeframe || 'D'})
              </div>
              <div style={{ color: 'var(--text-muted)' }}>
                Index: Bar #{session.current_index} | Start:{' '}
                {session.start_date ? formatVietnameseDate(session.start_date) : '—'}
              </div>
            </div>
          )}

          {entries.map((entry: JournalEntry) => (
            <div key={entry.id} className="glass-panel" style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem', padding: '16px' }}>
              <div
                style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
                  borderBottom: '1px solid var(--border-color)',
                  paddingBottom: '0.5rem',
                }}
              >
                <span style={{ fontWeight: 'bold', color: 'var(--color-primary)' }}>{entry.note_type}</span>
                <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>
                  {formatVietnameseDateTime(entry.created_at)}
                </span>
              </div>
              <p style={{ margin: 0, whiteSpace: 'pre-wrap', lineHeight: '1.5', fontSize: '14px' }}>{entry.content}</p>
              {entry.tags && entry.tags.length > 0 && (
                <div style={{ display: 'flex', gap: '0.5rem', marginTop: '0.5rem', flexWrap: 'wrap' }}>
                  {entry.tags.split(',').map((tag: string) => (
                    <span
                      key={tag.trim()}
                      style={{
                        background: 'rgba(255,255,255,0.08)',
                        padding: '2px 8px',
                        borderRadius: '4px',
                        fontSize: '12px',
                        color: 'var(--text-muted)',
                      }}
                    >
                      #{tag.trim()}
                    </span>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
