import React, { useCallback, useMemo, useState } from 'react';
import type { JournalEntry, PracticeWorkflowSnapshot } from '../../types';
import {
  CHECKLIST_FIELDS, createChecklistEntry, emptyChecklistChecks, parseChecklist,
  type ChecklistChecks,
} from '../../features/practice/practiceDomain';
import type { SubmissionResult } from './TradeControls';
import { useModalFocus } from '../../hooks/useModalFocus';

interface PracticeJournalProps {
  snapshot: PracticeWorkflowSnapshot;
  entries: JournalEntry[];
  loading: boolean;
  loadError: boolean;
  onSave: (entry: ReturnType<typeof createChecklistEntry>) => Promise<SubmissionResult>;
}

const LABELS: Record<keyof ChecklistChecks, string> = {
  trendIdentified: 'Trend identified', setupConfirmed: 'Setup confirmed',
  entryTriggerDefined: 'Entry trigger defined', riskDefined: 'Risk defined',
  exitPlanDefined: 'Exit plan defined', emotionChecked: 'Emotion checked',
};

export const PracticeJournal: React.FC<PracticeJournalProps> = ({ snapshot, entries, loading, loadError, onSave }) => {
  const [editing, setEditing] = useState(false);
  const [checks, setChecks] = useState<ChecklistChecks>(emptyChecklistChecks);
  const [observation, setObservation] = useState('');
  const [saving, setSaving] = useState(false);
  const [feedback, setFeedback] = useState<SubmissionResult | null>(null);
  const [dialogElement, setDialogElement] = useState<HTMLDivElement | null>(null);
  const parsed = useMemo(() => entries.map(entry => ({ entry, checklist: parseChecklist(entry) })), [entries]);
  const closeEditor = useCallback(() => { if (!saving) setEditing(false); }, [saving]);
  useModalFocus(editing, closeEditor, dialogElement);

  const save = async () => {
    if (saving) return;
    setSaving(true); setFeedback(null);
    const result = await onSave(createChecklistEntry(snapshot, checks, observation));
    setSaving(false); setFeedback(result);
    if (result.ok) { setEditing(false); setChecks(emptyChecklistChecks()); setObservation(''); }
  };

  return <section data-testid="practice-journal" style={{ display: 'grid', gap: 10 }}>
    <div className="panel" style={{ padding: 12 }}>
      <strong>Journal &amp; checklist · {snapshot.symbol}</strong>
      <div style={{ color: 'var(--text-muted)', fontSize: 11 }}>Bar {snapshot.visible_bar}/{snapshot.total_bars} · {snapshot.current_date.slice(0, 10)}</div>
      <p style={{ color: 'var(--text-muted)', fontSize: 12, margin: '8px 0' }}>Decision reason stays with the decision. Reflective notes and checklists are stored here.</p>
      <button className="btn-primary" onClick={() => { setEditing(true); setFeedback(null); }}>New observation</button>
      {feedback && <div role="status" style={{ marginTop: 8, color: feedback.ok ? 'var(--color-buy)' : 'var(--color-sell)' }}>{feedback.message}</div>}
    </div>
    {loading ? <div role="status">Loading journal…</div> : loadError ? <div role="alert" style={{ color: 'var(--color-sell)' }}>Journal could not be loaded. Retry by reopening this tab.</div> : parsed.length === 0 ? <div className="panel" data-testid="journal-empty" style={{ padding: 12, color: 'var(--text-muted)' }}>No journal entries at this replay bar.</div> : parsed.map(({ entry, checklist }) => <article className="panel" key={entry.id} style={{ padding: 12 }}>
      <strong>{checklist ? 'Practice checklist' : entry.note_type}</strong>
      {checklist ? <><div style={{ color: 'var(--text-muted)', fontSize: 11 }}>Bar {checklist.context.candleIndex + 1} · {checklist.context.date}</div><div style={{ fontSize: 12, marginTop: 6 }}>{CHECKLIST_FIELDS.filter(field => checklist.checks[field]).map(field => LABELS[field]).join(' · ') || 'No boxes checked'}</div><p style={{ whiteSpace: 'pre-wrap', marginBottom: 0 }}>{checklist.observation || 'No observation text.'}</p></> : <p style={{ whiteSpace: 'pre-wrap' }}>{entry.content}</p>}
    </article>)}

    {editing && <div ref={setDialogElement} role="dialog" aria-modal="true" aria-label="Practice checklist" data-testid="practice-checklist-dialog" style={{ position: 'fixed', inset: 0, zIndex: 1200, background: 'rgba(0,0,0,.72)', display: 'grid', placeItems: 'center', padding: 16 }}>
      <div className="glass-panel-solid" style={{ width: 'min(480px, 100%)', maxHeight: '90vh', overflowY: 'auto', padding: 20, display: 'grid', gap: 10 }}>
        <h3 style={{ margin: 0 }}>Checklist · {snapshot.symbol} · bar {snapshot.visible_bar}/{snapshot.total_bars}</h3>
        {CHECKLIST_FIELDS.map(field => <label key={field} style={{ display: 'flex', gap: 8 }}><input type="checkbox" checked={checks[field]} onChange={event => setChecks(previous => ({ ...previous, [field]: event.target.checked }))} />{LABELS[field]}</label>)}
        <label>Observation<textarea aria-label="Checklist observation" rows={5} value={observation} onChange={event => setObservation(event.target.value)} /></label>
        {feedback && !feedback.ok && <div role="alert" style={{ color: 'var(--color-sell)' }}>{feedback.message}</div>}
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8 }}><button disabled={saving} onClick={closeEditor}>Cancel</button><button className="btn-primary" disabled={saving} onClick={save}>{saving ? 'Saving…' : 'Save checklist'}</button></div>
      </div>
    </div>}
  </section>;
};
