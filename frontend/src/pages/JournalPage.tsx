import React, { useState, useEffect } from 'react';
import { useQuery } from '@tanstack/react-query';
import { getJournalEntries } from '../api/journalApi';
import { useReplayStore } from '../store/replayStore';
import type { JournalEntry } from '../types';

export const JournalPage: React.FC = () => {
  const store = useReplayStore();
  const [inputVal, setInputVal] = useState<string>(store.sessionId?.toString() || '1');
  const [sessionId, setSessionId] = useState<number>(store.sessionId || 1);

  // Sync when store.sessionId changes
  useEffect(() => {
    if (store.sessionId) {
      setSessionId(store.sessionId);
      setInputVal(store.sessionId.toString());
    }
  }, [store.sessionId]);

  const { data: entries, isLoading, isError } = useQuery({
    queryKey: ['journal', sessionId],
    queryFn: () => getJournalEntries(sessionId),
    enabled: !!sessionId,
  });

  const handleApply = () => {
    const val = parseInt(inputVal, 10);
    if (!isNaN(val) && val > 0) setSessionId(val);
  };

  return (
    <div style={{ padding: '2rem', maxWidth: '800px', margin: '0 auto' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '2rem' }}>
        <h2 style={{ margin: 0 }}>📓 Session Journal</h2>
        <div style={{ display: 'flex', gap: '0.5rem' }}>
          <input
            type="number"
            min="1"
            value={inputVal}
            onChange={(e) => setInputVal(e.target.value)}
            style={{ width: '80px', padding: '0.5rem', background: 'var(--bg-dark)', color: 'white', border: '1px solid var(--border-color)', borderRadius: '4px' }}
          />
          <button
            onClick={handleApply}
            style={{ background: 'var(--color-primary)', color: 'white', padding: '0.5rem 1rem', border: 'none', borderRadius: '4px', cursor: 'pointer' }}
          >
            Load
          </button>
        </div>
      </div>

      {isLoading ? (
        <p>Loading journal...</p>
      ) : isError ? (
        <div className="panel" style={{ borderColor: 'var(--color-sell)' }}>
          <p style={{ color: 'var(--color-sell)', margin: 0 }}>Failed to load journal. Is the Session ID correct?</p>
        </div>
      ) : !entries || entries.length === 0 ? (
        <div className="panel">
          <p style={{ color: 'var(--text-muted)', margin: 0, textAlign: 'center' }}>No journal entries found for Session #{sessionId}.</p>
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
          {entries.map((entry: JournalEntry) => (
            <div key={entry.id} className="panel" style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', borderBottom: '1px solid var(--border-color)', paddingBottom: '0.5rem' }}>
                <span style={{ fontWeight: 'bold', color: 'var(--color-primary)' }}>{entry.note_type}</span>
                <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>
                  {new Date(entry.created_at).toLocaleString()}
                </span>
              </div>
              <p style={{ margin: 0, whiteSpace: 'pre-wrap', lineHeight: '1.5' }}>{entry.content}</p>
              {entry.tags && entry.tags.length > 0 && (
                <div style={{ display: 'flex', gap: '0.5rem', marginTop: '0.5rem', flexWrap: 'wrap' }}>
                  {entry.tags.split(',').map((tag: string) => (
                    <span key={tag.trim()} style={{ background: 'rgba(255,255,255,0.1)', padding: '2px 8px', borderRadius: '4px', fontSize: '12px', color: 'var(--text-muted)' }}>
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
