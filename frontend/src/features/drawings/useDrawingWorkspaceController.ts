import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import toast from 'react-hot-toast';
import { getDrawings, updateDrawings } from '../../api/replayApi';
import { DrawingCommandHistory, type DrawingCommand } from './DrawingCommandHistory';
import { DrawingPersistenceConflict, DrawingRepository, DrawingRevisionConflict } from './DrawingRepository';
import type { DrawingProviderEvent } from './DrawingProvider';
import type { MagnetMode } from './drawingMagnet';
import {
  createDrawing, emptyDrawingDocument, normalizeDrawingOrder, TEXT_MAX_LENGTH, validateDrawingDocument,
  type DrawingTool, type SumiDrawing, type SumiDrawingAnchor, type SumiDrawingDocumentV1,
} from './drawingDomain';

type CommandKind = DrawingCommand['kind'];
type PersistenceStatus = 'loading' | 'ready' | 'saving' | 'error' | 'conflict' | 'indeterminate';
type Mutation = (current: SumiDrawingDocumentV1) => SumiDrawingDocumentV1 | null;
type ReconciliationOutcome = { state: 'intended' | 'prior' | 'indeterminate'; observedRaw: string | null };

export const useDrawingWorkspaceController = (sessionId: number, symbol: string) => {
  const repository = useMemo(() => new DrawingRepository(window.localStorage), []);
  const history = useMemo(() => new DrawingCommandHistory(), []);
  const initial = repository.load(sessionId, symbol) ?? emptyDrawingDocument(sessionId, symbol);
  const [document, setDocument] = useState<SumiDrawingDocumentV1>(initial);
  const committedRef = useRef<SumiDrawingDocumentV1>(initial);
  const [tool, setToolState] = useState<DrawingTool>('select');
  const [selection, setSelectionState] = useState<string[]>([]); const selectionRef = useRef<string[]>([]);
  const [pendingTextAnchor, setPendingTextAnchor] = useState<SumiDrawingAnchor | null>(null);
  const [persistenceStatus, setPersistenceStatusState] = useState<PersistenceStatus>('loading');
  const statusRef = useRef<PersistenceStatus>('loading'); const remoteRaw = useRef<string>('[]');
  const queue = useRef(Promise.resolve()); const generation = useRef(0);
  const magnetKey = `sumi:drawing-magnet:v1:${sessionId}:${encodeURIComponent(symbol)}`;
  const [magnetMode, setMagnetState] = useState<MagnetMode>(() => window.localStorage.getItem(magnetKey) === 'ohlc' ? 'ohlc' : 'off');

  const setStatus = useCallback((status: PersistenceStatus) => { statusRef.current = status; setPersistenceStatusState(status); }, []);
  const setSelection = useCallback((ids: string[]) => { selectionRef.current = ids; setSelectionState(ids); }, []);
  const publishCommitted = useCallback((next: SumiDrawingDocumentV1) => {
    committedRef.current = structuredClone(next); setDocument(structuredClone(next));
  }, []);
  const runSerialized = useCallback(<T,>(work: () => Promise<T>): Promise<T> => {
    const next = queue.current.then(work, work); queue.current = next.then(() => undefined, () => undefined); return next;
  }, []);
  const reconcileWrite = useCallback(async (priorRaw: string, intendedRaw: string, reason: string): Promise<ReconciliationOutcome> => {
    let observedRaw: string | null = null;
    try { observedRaw = (await getDrawings(sessionId)).state_data; } catch { /* unreadable remote is explicitly indeterminate */ }
    if (observedRaw === intendedRaw) return { state: 'intended', observedRaw };
    if (observedRaw === priorRaw) return { state: 'prior', observedRaw };
    repository.preserveIndeterminate(sessionId, symbol, { priorRaw, intendedRaw, observedRaw, reason });
    return { state: 'indeterminate', observedRaw };
  }, [repository, sessionId, symbol]);

  useEffect(() => {
    if (symbol === '—') return;
    const requestGeneration = ++generation.current;
    // Repository identity and interaction state change together on replay resume.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setStatus('loading'); setSelection([]); setToolState('select'); setPendingTextAnchor(null); history.clear();
    const local = repository.load(sessionId, symbol) ?? emptyDrawingDocument(sessionId, symbol);
    publishCommitted(local); remoteRaw.current = '[]';
    void runSerialized(async () => {
      try {
        const localRawBeforeHydration = repository.raw(sessionId, symbol);
        const response = await getDrawings(sessionId); if (generation.current !== requestGeneration) return;
        const hydrated = repository.hydrate(sessionId, symbol, response.state_data); remoteRaw.current = response.state_data;
        publishCommitted(hydrated.document);
        if (hydrated.conflict) { setStatus('conflict'); toast.error(hydrated.conflict); return; }
        if (hydrated.migrated) {
          const serialized = JSON.stringify(hydrated.document); let outcome: ReconciliationOutcome | null = null;
          try {
            const echoed = await updateDrawings(sessionId, serialized); if (generation.current !== requestGeneration) return;
            outcome = echoed.state_data === serialized ? { state: 'intended', observedRaw: echoed.state_data }
              : await reconcileWrite(response.state_data, serialized, 'migration PUT returned a mismatched echo');
          } catch { outcome = await reconcileWrite(response.state_data, serialized, 'migration PUT failed after dispatch'); }
          if (generation.current !== requestGeneration) return;
          if (outcome.state === 'intended') {
            repository.put(hydrated.document); remoteRaw.current = serialized; publishCommitted(hydrated.document); setStatus('ready');
          } else if (outcome.state === 'prior') {
            repository.restore(sessionId, symbol, localRawBeforeHydration); publishCommitted(local); remoteRaw.current = response.state_data;
            setStatus('error'); toast.error('Legacy drawing migration was not committed; the prior state was restored and editing is paused.');
          } else {
            repository.restore(sessionId, symbol, localRawBeforeHydration); publishCommitted(local); setStatus('indeterminate');
            toast.error('Drawing migration outcome is indeterminate. Prior and intended recovery copies were preserved; reload or reconcile before editing.');
          }
          return;
        }
        setStatus('ready');
      } catch {
        if (generation.current !== requestGeneration) return;
        publishCommitted(repository.load(sessionId, symbol) ?? emptyDrawingDocument(sessionId, symbol));
        setStatus('error'); toast.error('Backend drawings are unavailable; committed edits are paused.');
      }
    });
    return () => { if (generation.current === requestGeneration) generation.current += 1; };
  }, [history, publishCommitted, reconcileWrite, repository, runSerialized, sessionId, setSelection, setStatus, symbol]);

  useEffect(() => {
    const value = window.localStorage.getItem(magnetKey) === 'ohlc' ? 'ohlc' : 'off';
    // Magnet preference follows the active session/symbol identity.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setMagnetState(value);
  }, [magnetKey]);
  const setMagnetMode = useCallback((mode: MagnetMode) => { window.localStorage.setItem(magnetKey, mode); setMagnetState(mode); }, [magnetKey]);

  const persistMutation = useCallback((mutation: Mutation, kind: CommandKind, recordHistory = true): Promise<SumiDrawingDocumentV1 | null> => {
    if (statusRef.current !== 'ready' && statusRef.current !== 'saving') { toast.error('Drawing persistence is not ready. Reload or reconcile before editing.'); return Promise.resolve(null); }
    return runSerialized(async () => {
      if (statusRef.current !== 'ready') { toast.error('Drawing persistence is not ready. Reload or reconcile before editing.'); return null; }
      const before = structuredClone(committedRef.current); const next = mutation(structuredClone(before));
      if (!next || JSON.stringify(next) === JSON.stringify(before)) { setDocument(structuredClone(before)); return null; }
      if (!validateDrawingDocument(next)) { setDocument(structuredClone(before)); toast.error('Drawing change is invalid and was not committed.'); return null; }
      const localBefore = repository.raw(before.sessionId, before.symbol); setStatus('saving');
      try {
        const remote = await getDrawings(before.sessionId);
        if (remote.state_data !== remoteRaw.current) throw new DrawingPersistenceConflict('Backend drawings changed since the last synchronized revision.');
        const saved = repository.save(next, before.revision); const serialized = JSON.stringify(saved);
        let outcome: ReconciliationOutcome;
        try {
          const echoed = await updateDrawings(before.sessionId, serialized);
          outcome = echoed.state_data === serialized ? { state: 'intended', observedRaw: echoed.state_data }
            : await reconcileWrite(remote.state_data, serialized, 'drawing PUT returned a mismatched echo');
        } catch { outcome = await reconcileWrite(remote.state_data, serialized, 'drawing PUT failed after dispatch'); }
        if (outcome.state === 'prior') {
          repository.restore(before.sessionId, before.symbol, localBefore); publishCommitted(before); remoteRaw.current = remote.state_data;
          setStatus('error'); toast.error('Drawing was not committed; reconciliation confirmed the prior backend state.'); return null;
        }
        if (outcome.state === 'indeterminate') {
          repository.restore(before.sessionId, before.symbol, localBefore); publishCommitted(before); setStatus('indeterminate');
          toast.error('Drawing write outcome is indeterminate. Prior and intended recovery copies were preserved; reload or reconcile before editing.'); return null;
        }
        remoteRaw.current = serialized;
        if (recordHistory) history.commit({ before, after: saved, kind }); publishCommitted(saved); setStatus('ready'); return saved;
      } catch (error) {
        repository.restore(before.sessionId, before.symbol, localBefore); publishCommitted(before);
        if (error instanceof DrawingRevisionConflict || error instanceof DrawingPersistenceConflict) {
          setStatus('conflict'); toast.error('Drawing changed in another workspace. Reload before editing.');
        } else { setStatus('error'); toast.error('Drawing was not committed because backend persistence failed.'); }
        return null;
      }
    });
  }, [history, publishCommitted, reconcileWrite, repository, runSerialized, setStatus]);

  const providerEvent = useCallback((event: DrawingProviderEvent) => {
    if (event.type === 'selection-changed') { setSelection(event.drawingIds); return; }
    if (event.type === 'cancelled') { setToolState('select'); setPendingTextAnchor(null); return; }
    if (event.type === 'text-placement-requested') { setToolState('select'); setPendingTextAnchor(event.anchor); return; }
    if (event.type === 'change-preview') {
      const committed = committedRef.current;
      setDocument({ ...structuredClone(committed), drawings: committed.drawings.map(d => event.drawings.find(item => item.id === d.id) ?? d) });
      return;
    }
    if (event.type === 'created') {
      void persistMutation(before => ({ ...before, drawings: normalizeDrawingOrder([...before.drawings, { ...event.drawing, order: before.drawings.length }]) }), 'create')
        .then(saved => { setToolState('select'); setSelection(saved ? [event.drawing.id] : []); });
      return;
    }
    if (event.type === 'change-committed') {
      const afterMap = new Map(event.after.map(d => [d.id, d]));
      void persistMutation(before => ({ ...before, drawings: before.drawings.map(d => afterMap.get(d.id) ?? d) }), 'change');
    }
  }, [persistMutation, setSelection]);

  const setTool = useCallback((next: DrawingTool) => { setPendingTextAnchor(null); setToolState(next); if (next !== 'select') setSelection([]); }, [setSelection]);
  const commitText = useCallback((text: string) => {
    const value = text.trim(); if (!pendingTextAnchor || !value || value.length > TEXT_MAX_LENGTH) return false;
    const anchor = structuredClone(pendingTextAnchor); setPendingTextAnchor(null);
    void persistMutation(before => ({ ...before, drawings: normalizeDrawingOrder([...before.drawings, createDrawing('text', [anchor], before.drawings.length, value)]) }), 'create')
      .then(saved => { if (saved) setSelection([saved.drawings.at(-1)!.id]); });
    return true;
  }, [pendingTextAnchor, persistMutation, setSelection]);
  const cancelText = useCallback(() => setPendingTextAnchor(null), []);
  const remove = useCallback((ids?: string[]) => {
    const targets = ids ?? selectionRef.current; if (!targets.length) return;
    void persistMutation(before => ({ ...before, drawings: normalizeDrawingOrder(before.drawings.filter(d => !targets.includes(d.id))) }), 'delete')
      .then(saved => { if (saved) setSelection([]); });
  }, [persistMutation, setSelection]);
  const updateDrawing = useCallback((drawing: SumiDrawing) => {
    if (!selectionRef.current.includes(drawing.id)) return false;
    const candidate = { ...structuredClone(committedRef.current), drawings: committedRef.current.drawings.map(item => item.id === drawing.id ? structuredClone(drawing) : item) };
    if (!validateDrawingDocument(candidate)) { toast.error('Drawing settings are invalid. Check anchors and values.'); return false; }
    void persistMutation(before => ({ ...before, drawings: before.drawings.map(item => item.id === drawing.id ? structuredClone(drawing) : item) }), 'change'); return true;
  }, [persistMutation]);
  const updateSelected = useCallback((update: (drawing: SumiDrawing) => SumiDrawing) => {
    const id = selectionRef.current.length === 1 ? selectionRef.current[0] : null; if (!id) return;
    void persistMutation(before => ({ ...before, drawings: before.drawings.map(d => d.id === id ? update(structuredClone(d)) : d) }), 'change');
  }, [persistMutation]);
  const editAnchor = useCallback((index: number, patch: Partial<SumiDrawingAnchor>) => updateSelected(drawing => ({ ...drawing, anchors: drawing.anchors.map((anchor, anchorIndex) => anchorIndex === index ? { ...anchor, ...patch } : anchor) } as SumiDrawing)), [updateSelected]);
  const editPrice = useCallback((price: number) => { if (price > 0) editAnchor(0, { price }); }, [editAnchor]);
  const editText = useCallback((text: string) => { const value = text.trim(); if (!value || value.length > TEXT_MAX_LENGTH) return; updateSelected(drawing => drawing.tool === 'text' ? { ...drawing, geometry: { ...drawing.geometry, text: value } } : drawing); }, [updateSelected]);
  const reverseFibonacci = useCallback(() => updateSelected(drawing => drawing.tool === 'fibonacci-retracement' ? { ...drawing, geometry: { ...drawing.geometry, direction: drawing.geometry.direction === 'start-to-end' ? 'end-to-start' : 'start-to-end' } } : drawing), [updateSelected]);
  const clearAll = useCallback(() => { void persistMutation(before => before.drawings.length ? { ...before, drawings: [] } : null, 'clear').then(saved => { if (saved) setSelection([]); }); }, [persistMutation, setSelection]);
  const applyHistory = useCallback((direction: 'undo' | 'redo') => {
    if (statusRef.current !== 'ready') return;
    let command: DrawingCommand | null = null;
    void persistMutation(before => {
      command = direction === 'undo' ? history.peekUndo() : history.peekRedo(); if (!command) return null;
      const semantic = direction === 'undo' ? command.before : command.after;
      return { ...structuredClone(semantic), revision: before.revision };
    }, 'change', false).then(saved => {
      if (!saved || !command) return; if (direction === 'undo') history.acceptUndo(); else history.acceptRedo(); setSelection([]);
    });
  }, [history, persistMutation, setSelection]);
  const undo = useCallback(() => applyHistory('undo'), [applyHistory]); const redo = useCallback(() => applyHistory('redo'), [applyHistory]);

  return { document, tool, selection, setTool, providerEvent, remove, editPrice, editAnchor, editText, reverseFibonacci, updateDrawing,
    clearAll, undo, redo, canUndo: history.canUndo, canRedo: history.canRedo, pendingTextAnchor, commitText, cancelText,
    magnetMode, setMagnetMode, persistenceStatus };
};
