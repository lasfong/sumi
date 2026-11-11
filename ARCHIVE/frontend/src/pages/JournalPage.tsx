import React, { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { getJournalEntries, exportJournal } from '../api/journalApi';
import { getPracticeState } from '../api/decisionApi';
import { getReplaySession } from '../api/replayApi';
import { useSessionSelection } from '../hooks/useSessionSelection';
import { SessionPicker } from '../components/common/SessionPicker';
import { formatVietnameseDate, formatVietnameseDateTime } from '../utils/formatters';
import type { JournalEntry } from '../types';
import { BookOpen, AlertCircle, FileText, ArrowRight, Download, BarChart2 } from 'lucide-react';
import { Link } from 'react-router-dom';

export const JournalPage: React.FC = () => {
  const { sessionId, selectSession, getSessionPath } = useSessionSelection();
  const [exporting, setExporting] = useState<boolean>(false);

  const { data: session } = useQuery({
    queryKey: ['replay-session', sessionId],
    queryFn: () => getReplaySession(sessionId!),
    enabled: !!sessionId,
  });

  const { data: practiceState } = useQuery({
    queryKey: ['practice-state', sessionId],
    queryFn: () => getPracticeState(sessionId!),
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

  const handleExport = async (format: 'json' | 'csv') => {
    if (!sessionId) return;
    setExporting(true);
    try {
      const data = await exportJournal(sessionId, format);
      const blob = format === 'csv'
        ? new Blob([data as string], { type: 'text/csv;charset=utf-8;' })
        : new Blob([JSON.stringify(data, null, 2)], { type: 'application/json;charset=utf-8;' });
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.setAttribute('download', `sumi_session_${sessionId}_journal.${format}`);
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      URL.revokeObjectURL(url);
    } catch (err) {
      console.error('Failed to export journal:', err);
    } finally {
      setExporting(false);
    }
  };

  const trades = practiceState?.trades ?? [];

  return (
    <div style={{ padding: '2rem', maxWidth: '1080px', margin: '0 auto' }}>
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
            Trade Journal &amp; Review
          </h2>
          <p style={{ margin: '4px 0 0 0', color: 'var(--text-muted)', fontSize: '13px' }}>
            Planned vs Executed variance, trade taxonomy, checklist audit, and local privacy export
          </p>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem', flexWrap: 'wrap' }}>
          <SessionPicker selectedSessionId={sessionId} onSelectSession={selectSession} />

          {sessionId && (
            <>
              <button
                type="button"
                data-testid="export-journal-json"
                onClick={() => handleExport('json')}
                disabled={exporting}
                style={{ display: 'inline-flex', alignItems: 'center', gap: '4px', padding: '6px 12px', fontSize: '12px' }}
              >
                <Download size={13} /> JSON
              </button>
              <button
                type="button"
                data-testid="export-journal-csv"
                onClick={() => handleExport('csv')}
                disabled={exporting}
                style={{ display: 'inline-flex', alignItems: 'center', gap: '4px', padding: '6px 12px', fontSize: '12px' }}
              >
                <Download size={13} /> CSV
              </button>
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
            </>
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
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
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

          {/* Section 1: Planned vs Executed Review Table */}
          <section className="glass-panel" style={{ padding: '16px', display: 'grid', gap: '12px' }}>
            <h3 style={{ margin: 0, fontSize: '15px', display: 'flex', alignItems: 'center', gap: '6px' }}>
              <BarChart2 size={16} style={{ color: 'var(--color-primary)' }} />
              Planned vs Executed Trade Review
            </h3>
            {trades.length === 0 ? (
              <p style={{ color: 'var(--text-muted)', fontSize: '13px', margin: 0 }}>No completed or open trades in this session yet.</p>
            ) : (
              <div style={{ overflowX: 'auto' }}>
                <table data-testid="planned-vs-executed-table" style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px' }}>
                  <thead>
                    <tr style={{ borderBottom: '1px solid var(--border-color)', color: 'var(--text-muted)', textAlign: 'left' }}>
                      <th style={{ padding: '6px 8px' }}>Trade</th>
                      <th style={{ padding: '6px 8px' }}>Entry (Plan / Exec)</th>
                      <th style={{ padding: '6px 8px' }}>Quantity (Plan / Exec)</th>
                      <th style={{ padding: '6px 8px' }}>R (Plan / Realized)</th>
                      <th style={{ padding: '6px 8px' }}>Net PnL</th>
                      <th style={{ padding: '6px 8px' }}>Taxonomy</th>
                      <th style={{ padding: '6px 8px' }}>Status</th>
                    </tr>
                  </thead>
                  <tbody>
                    {trades.map((t) => (
                      <tr key={t.id} style={{ borderBottom: '1px solid rgba(255,255,255,0.06)' }}>
                        <td style={{ padding: '8px' }}>
                          <strong>#{t.id} {t.symbol}</strong>
                          <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>{t.entry_date?.slice(0, 10)}</div>
                        </td>
                        <td style={{ padding: '8px' }}>
                          <div>{t.planned_entry_price ? `${t.planned_entry_price.toLocaleString()} / ` : ''}<strong>{t.entry_price.toLocaleString()}</strong></div>
                          {t.entry_drift !== undefined && t.entry_drift !== null && t.entry_drift !== 0 && (
                            <span style={{ fontSize: '10px', color: t.entry_drift > 0 ? '#FFD166' : '#00E676' }}>
                              drift: {t.entry_drift > 0 ? `+${t.entry_drift.toFixed(2)}` : t.entry_drift.toFixed(2)}
                            </span>
                          )}
                        </td>
                        <td style={{ padding: '8px' }}>
                          <div>{t.planned_quantity ? `${t.planned_quantity.toLocaleString()} / ` : ''}<strong>{t.quantity.toLocaleString()}</strong></div>
                          {t.size_variance !== undefined && t.size_variance !== null && t.size_variance !== 0 && (
                            <span style={{ fontSize: '10px', color: '#FFD166' }}>
                              var: {t.size_variance > 0 ? `+${t.size_variance.toLocaleString()}` : t.size_variance.toLocaleString()}
                            </span>
                          )}
                        </td>
                        <td style={{ padding: '8px' }}>
                          <div>
                            {t.planned_r ? `${t.planned_r.toFixed(2)}R / ` : ''}
                            <strong style={{ color: (t.r_multiple ?? 0) >= 0 ? 'var(--color-buy)' : 'var(--color-sell)' }}>
                              {t.r_multiple !== null && t.r_multiple !== undefined ? `${t.r_multiple.toFixed(2)}R` : '—'}
                            </strong>
                          </div>
                        </td>
                        <td style={{ padding: '8px' }}>
                          <span style={{ color: (t.net_pnl ?? 0) >= 0 ? 'var(--color-buy)' : 'var(--color-sell)', fontWeight: 600 }}>
                            {t.net_pnl !== null && t.net_pnl !== undefined ? `${t.net_pnl.toLocaleString()} (${t.pnl_percent?.toFixed(2)}%)` : 'Open'}
                          </span>
                        </td>
                        <td style={{ padding: '8px' }}>
                          <div style={{ display: 'flex', gap: '4px', flexWrap: 'wrap' }}>
                            {t.setup_type && <span style={{ background: 'rgba(56,152,255,0.15)', color: '#58A6FF', padding: '1px 6px', borderRadius: '3px', fontSize: '10px' }}>{t.setup_type}</span>}
                            {t.market_regime && <span style={{ background: 'rgba(255,255,255,0.08)', padding: '1px 6px', borderRadius: '3px', fontSize: '10px' }}>{t.market_regime}</span>}
                            {t.emotion && <span style={{ background: 'rgba(255,209,102,0.15)', color: '#FFD166', padding: '1px 6px', borderRadius: '3px', fontSize: '10px' }}>{t.emotion}</span>}
                            {t.mistake_tag && t.mistake_tag !== 'None' && <span style={{ background: 'rgba(255,100,100,0.15)', color: '#FF8A80', padding: '1px 6px', borderRadius: '3px', fontSize: '10px' }}>{t.mistake_tag}</span>}
                          </div>
                        </td>
                        <td style={{ padding: '8px' }}>
                          <span style={{
                            padding: '2px 6px',
                            borderRadius: '4px',
                            fontSize: '11px',
                            background: t.status === 'closed' ? (t.result === 'win' ? 'rgba(0,230,118,0.15)' : 'rgba(255,82,82,0.15)') : 'rgba(255,255,255,0.08)',
                            color: t.status === 'closed' ? (t.result === 'win' ? 'var(--color-buy)' : 'var(--color-sell)') : 'var(--text-muted)',
                          }}>
                            {t.status === 'closed' ? t.result.toUpperCase() : 'OPEN'}
                          </span>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </section>

          {/* Section 2: Journal & Checklist Entries */}
          <section style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
            <h3 style={{ margin: 0, fontSize: '15px' }}>Journal &amp; Checklist Records</h3>
            {(!entries || entries.length === 0) ? (
              <div className="glass-panel" style={{ padding: '2rem', textAlign: 'center' }}>
                <p style={{ color: 'var(--text-muted)', margin: '0 0 1rem 0' }}>No journal notes or checklist snapshots recorded yet.</p>
                <Link to={getSessionPath('/replay')} className="btn-primary" style={{ display: 'inline-flex', alignItems: 'center', gap: '6px', padding: '6px 14px', borderRadius: '6px', textDecoration: 'none' }}>
                  <span>Record in Replay Lab</span>
                  <ArrowRight size={14} />
                </Link>
              </div>
            ) : (
              entries.map((entry: JournalEntry) => (
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
                  {(entry.setup_type || entry.market_regime || entry.emotion || entry.mistake_tag || entry.tags) && (
                    <div style={{ display: 'flex', gap: '0.5rem', marginTop: '0.5rem', flexWrap: 'wrap' }}>
                      {entry.setup_type && <span style={{ background: 'rgba(56,152,255,0.15)', color: '#58A6FF', padding: '2px 8px', borderRadius: '4px', fontSize: '12px' }}>Setup: {entry.setup_type}</span>}
                      {entry.market_regime && <span style={{ background: 'rgba(255,255,255,0.08)', padding: '2px 8px', borderRadius: '4px', fontSize: '12px', color: 'var(--text-muted)' }}>Regime: {entry.market_regime}</span>}
                      {entry.emotion && <span style={{ background: 'rgba(255,209,102,0.15)', color: '#FFD166', padding: '2px 8px', borderRadius: '4px', fontSize: '12px' }}>{entry.emotion}</span>}
                      {entry.mistake_tag && entry.mistake_tag !== 'None' && <span style={{ background: 'rgba(255,100,100,0.15)', color: '#FF8A80', padding: '2px 8px', borderRadius: '4px', fontSize: '12px' }}>{entry.mistake_tag}</span>}
                      {entry.tags && entry.tags.split(',').map((tag: string) => (
                        <span key={tag.trim()} style={{ background: 'rgba(255,255,255,0.05)', padding: '2px 8px', borderRadius: '4px', fontSize: '12px', color: 'var(--text-muted)' }}>
                          #{tag.trim()}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              ))
            )}
          </section>
        </div>
      )}
    </div>
  );
};
