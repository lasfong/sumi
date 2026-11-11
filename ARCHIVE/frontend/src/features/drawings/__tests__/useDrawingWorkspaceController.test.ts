import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import toast from 'react-hot-toast';
import { updateDrawings } from '../../../api/replayApi';
import type { SumiDrawing } from '../drawingDomain';
import { useDrawingWorkspaceController } from '../useDrawingWorkspaceController';

let backendRaw = '[]';
let failWrite = false;
let commitThenThrow = false;
let echoMismatch = false;
let thirdRaw: string | null = null;
let failReconciliationGet = false;
let writeDispatched = false;
let writeGate: Promise<void> | null = null;
vi.mock('react-hot-toast', () => ({ default: { error: vi.fn() } }));
vi.mock('../../../api/replayApi', () => ({
  getDrawings: vi.fn(async () => { if (failReconciliationGet && writeDispatched) throw new Error('get unavailable'); return { id: 1, session_id: 7, symbol: 'FPT', state_data: backendRaw }; }),
  updateDrawings: vi.fn(async (_sessionId: number, stateData: string) => {
    if (writeGate) await writeGate; writeDispatched = true;
    if (commitThenThrow) { backendRaw = stateData; throw new Error('connection lost after commit'); }
    if (failWrite) throw new Error('offline');
    if (echoMismatch) { backendRaw = thirdRaw ?? stateData; return { id: 1, session_id: 7, symbol: 'FPT', state_data: '{mismatched-echo}' }; }
    backendRaw = stateData; return { id: 1, session_id: 7, symbol: 'FPT', state_data: stateData };
  }),
}));

const drawing: SumiDrawing = {
  id: '123e4567-e89b-42d3-a456-426614174000', tool: 'horizontal', paneId: 'price', order: 0,
  visible: true, locked: false, anchors: [{ time: '2026-07-15', price: 100 }],
  style: { lineColor: '#E056FD', lineWidth: 2, lineStyle: 'solid' }, geometry: { kind: 'horizontal' },
};
const key = 'sumi:drawing-document:v1:7:FPT';
const forceRevision = (revision: number) => { const value = JSON.parse(window.localStorage.getItem(key) ?? '{}'); window.localStorage.setItem(key, JSON.stringify({ ...value, revision })); };
const readyHook = async () => {
  const hook = renderHook(() => useDrawingWorkspaceController(7, 'FPT'));
  await waitFor(() => expect(hook.result.current.persistenceStatus).toBe('ready')); return hook;
};
const create = async (result: { current: ReturnType<typeof useDrawingWorkspaceController> }) => {
  act(() => result.current.providerEvent({ type: 'created', drawing })); await waitFor(() => expect(result.current.document.revision).toBe(1));
};

describe('useDrawingWorkspaceController transactions', () => {
  beforeEach(() => {
    backendRaw = '[]'; failWrite = false; commitThenThrow = false; echoMismatch = false; thirdRaw = null;
    failReconciliationGet = false; writeDispatched = false; writeGate = null; window.localStorage.clear();
    vi.mocked(toast.error).mockClear(); vi.mocked(updateDrawings).mockClear();
  });
  it('keeps migration loading until verified echo and only then publishes canonical local state', async () => {
    backendRaw = JSON.stringify([{ id: drawing.id, type: 'horizontal', color: '#E056FD', points: drawing.anchors }]);
    let release = () => {}; writeGate = new Promise<void>(resolve => { release = resolve; });
    const { result } = renderHook(() => useDrawingWorkspaceController(7, 'FPT'));
    await waitFor(() => expect(updateDrawings).toHaveBeenCalledOnce()); expect(result.current.persistenceStatus).toBe('loading'); expect(window.localStorage.getItem(key)).toBeNull();
    release(); await waitFor(() => expect(result.current.persistenceStatus).toBe('ready'));
    expect(JSON.parse(window.localStorage.getItem(key) ?? 'null')).toEqual(result.current.document); expect(backendRaw).toBe(window.localStorage.getItem(key));
  });
  it('retains backup and pauses commands when migration echo fails', async () => {
    backendRaw = JSON.stringify([{ id: drawing.id, type: 'horizontal', color: '#E056FD', points: drawing.anchors }]); const original = backendRaw; failWrite = true;
    const { result } = renderHook(() => useDrawingWorkspaceController(7, 'FPT')); await waitFor(() => expect(result.current.persistenceStatus).toBe('error'));
    expect(window.localStorage.getItem('sumi:drawing-legacy-backup:v1:7:FPT')).toBe(original); expect(window.localStorage.getItem(key)).toBeNull();
    act(() => result.current.providerEvent({ type: 'created', drawing: { ...drawing, id: '323e4567-e89b-42d3-a456-426614174000' } }));
    expect(result.current.document.revision).toBe(0); expect(backendRaw).toBe(original);
  });
  it.each(['commit-then-throw', 'mismatched-echo'] as const)('accepts migrated intended remote exactly once after %s reconciliation', async mode => {
    backendRaw = JSON.stringify([{ id: drawing.id, type: 'horizontal', color: '#E056FD', points: drawing.anchors }]);
    if (mode === 'commit-then-throw') commitThenThrow = true; else echoMismatch = true;
    const { result } = renderHook(() => useDrawingWorkspaceController(7, 'FPT'));
    await waitFor(() => expect(result.current.persistenceStatus).toBe('ready'));
    expect(JSON.parse(backendRaw)).toEqual(result.current.document); expect(JSON.parse(window.localStorage.getItem(key) ?? 'null')).toEqual(result.current.document);
    expect(result.current.document).toMatchObject({ revision: 0, drawings: [{ id: drawing.id }] }); expect(result.current.canUndo).toBe(false);
  });
  it('blocks migration with prior/intended evidence when reconciliation sees a third remote', async () => {
    const prior = JSON.stringify([{ id: drawing.id, type: 'horizontal', color: '#E056FD', points: drawing.anchors }]); backendRaw = prior;
    echoMismatch = true; thirdRaw = JSON.stringify({ schemaVersion: 1, revision: 9, sessionId: 7, symbol: 'FPT', drawings: [] });
    const { result } = renderHook(() => useDrawingWorkspaceController(7, 'FPT')); await waitFor(() => expect(result.current.persistenceStatus).toBe('indeterminate'));
    expect(result.current.document.drawings).toEqual([]); expect(window.localStorage.getItem(key)).toBeNull(); expect(backendRaw).toBe(thirdRaw);
    expect(JSON.parse(window.localStorage.getItem('sumi:drawing-indeterminate:v1:7:FPT') ?? 'null')).toMatchObject({ priorRaw: prior, observedRaw: thirdRaw });
  });
  it('blocks migration honestly when its reconciliation GET is unavailable', async () => {
    const prior = JSON.stringify([{ id: drawing.id, type: 'horizontal', color: '#E056FD', points: drawing.anchors }]); backendRaw = prior;
    commitThenThrow = true; failReconciliationGet = true;
    const { result } = renderHook(() => useDrawingWorkspaceController(7, 'FPT')); await waitFor(() => expect(result.current.persistenceStatus).toBe('indeterminate'));
    expect(result.current.document.drawings).toEqual([]); expect(window.localStorage.getItem(key)).toBeNull();
    expect(JSON.parse(window.localStorage.getItem('sumi:drawing-indeterminate:v1:7:FPT') ?? 'null')).toMatchObject({ priorRaw: prior, observedRaw: null });
  });
  it('visibly blocks malformed remote state after quarantine and never overwrites it with a later edit', async () => {
    backendRaw = '{malformed'; const original = backendRaw; const { result } = renderHook(() => useDrawingWorkspaceController(7, 'FPT'));
    await waitFor(() => expect(result.current.persistenceStatus).toBe('conflict'));
    expect(window.localStorage.getItem('sumi:drawing-quarantine:v1:7:FPT')).toContain('Invalid JSON');
    act(() => result.current.providerEvent({ type: 'created', drawing })); expect(backendRaw).toBe(original); expect(result.current.document.drawings).toEqual([]); expect(updateDrawings).not.toHaveBeenCalled();
  });
  it('serializes two immediate valid edits from the latest committed revision without lost state', async () => {
    const { result } = await readyHook(); const second = { ...drawing, id: '223e4567-e89b-42d3-a456-426614174000', order: 1 } as SumiDrawing;
    act(() => { result.current.providerEvent({ type: 'created', drawing }); result.current.providerEvent({ type: 'created', drawing: second }); });
    await waitFor(() => expect(result.current.document.revision).toBe(2));
    expect(result.current.document.drawings.map(item => item.id)).toEqual([drawing.id, second.id]); expect(JSON.parse(backendRaw)).toEqual(result.current.document);
    expect(JSON.parse(window.localStorage.getItem(key) ?? 'null')).toEqual(result.current.document); expect(result.current.canUndo).toBe(true);
  });
  it.each(['commit-then-throw', 'mismatched-echo'] as const)('accepts an ordinary intended write exactly once after %s reconciliation', async mode => {
    const { result } = await readyHook(); if (mode === 'commit-then-throw') commitThenThrow = true; else echoMismatch = true;
    act(() => result.current.providerEvent({ type: 'created', drawing })); await waitFor(() => expect(result.current.document.revision).toBe(1));
    expect(result.current.persistenceStatus).toBe('ready'); expect(result.current.canUndo).toBe(true);
    expect(JSON.parse(backendRaw)).toEqual(result.current.document); expect(JSON.parse(window.localStorage.getItem(key) ?? 'null')).toEqual(result.current.document);
  });
  it('restores the exact prior ordinary state when reconciliation confirms prior remote', async () => {
    const { result } = await readyHook(); failWrite = true; act(() => result.current.providerEvent({ type: 'created', drawing }));
    await waitFor(() => expect(result.current.persistenceStatus).toBe('error'));
    expect(result.current.document).toMatchObject({ revision: 0, drawings: [] }); expect(result.current.canUndo).toBe(false);
    expect(window.localStorage.getItem(key)).toBeNull(); expect(backendRaw).toBe('[]');
  });
  it('blocks an ordinary divergent remote and preserves both recovery copies', async () => {
    const { result } = await readyHook(); echoMismatch = true; thirdRaw = JSON.stringify({ schemaVersion: 1, revision: 9, sessionId: 7, symbol: 'FPT', drawings: [] });
    act(() => result.current.providerEvent({ type: 'created', drawing })); await waitFor(() => expect(result.current.persistenceStatus).toBe('indeterminate'));
    expect(result.current.document).toMatchObject({ revision: 0, drawings: [] }); expect(result.current.canUndo).toBe(false); expect(window.localStorage.getItem(key)).toBeNull();
    const evidence = JSON.parse(window.localStorage.getItem('sumi:drawing-indeterminate:v1:7:FPT') ?? 'null');
    expect(evidence).toMatchObject({ priorRaw: '[]', observedRaw: thirdRaw }); expect(JSON.parse(evidence.intendedRaw)).toMatchObject({ revision: 1, drawings: [{ id: drawing.id }] });
  });
  it('blocks an ordinary commit-then-error when reconciliation GET is unavailable', async () => {
    const { result } = await readyHook(); commitThenThrow = true; failReconciliationGet = true;
    act(() => result.current.providerEvent({ type: 'created', drawing })); await waitFor(() => expect(result.current.persistenceStatus).toBe('indeterminate'));
    expect(result.current.document).toMatchObject({ revision: 0, drawings: [] }); expect(result.current.canUndo).toBe(false); expect(window.localStorage.getItem(key)).toBeNull();
    expect(JSON.parse(window.localStorage.getItem('sumi:drawing-indeterminate:v1:7:FPT') ?? 'null')).toMatchObject({ priorRaw: '[]', observedRaw: null });
  });
  it('accepts reconciled intended undo and redo once without corrupting history', async () => {
    const { result } = await readyHook(); await create(result);
    commitThenThrow = true; act(() => result.current.undo()); await waitFor(() => expect(result.current.document.revision).toBe(2));
    expect(result.current.document.drawings).toEqual([]); expect(result.current.canUndo).toBe(false); expect(result.current.canRedo).toBe(true);
    commitThenThrow = false; echoMismatch = true; act(() => result.current.redo()); await waitFor(() => expect(result.current.document.revision).toBe(3));
    expect(result.current.document.drawings).toHaveLength(1); expect(result.current.canUndo).toBe(true); expect(result.current.canRedo).toBe(false);
    expect(JSON.parse(backendRaw)).toEqual(result.current.document);
  });
  it('keeps document and history unchanged when an ordinary commit conflicts', async () => {
    const { result } = await readyHook(); await create(result); const before = structuredClone(result.current.document); forceRevision(9);
    act(() => result.current.providerEvent({ type: 'created', drawing: { ...drawing, id: '223e4567-e89b-42d3-a456-426614174000', order: 1 } }));
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('Drawing changed in another workspace. Reload before editing.'));
    expect(result.current.document).toEqual(before); expect(result.current.canUndo).toBe(true); expect(result.current.canRedo).toBe(false);
  });
  it('does not mutate history when undo persistence conflicts', async () => {
    const { result } = await readyHook(); await create(result); const before = structuredClone(result.current.document); forceRevision(9); act(() => result.current.undo());
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('Drawing changed in another workspace. Reload before editing.'));
    expect(result.current.document).toEqual(before); expect(result.current.canUndo).toBe(true); expect(result.current.canRedo).toBe(false);
  });
  it('does not mutate history when redo persistence conflicts', async () => {
    const { result } = await readyHook(); await create(result); act(() => result.current.undo()); await waitFor(() => expect(result.current.document.revision).toBe(2));
    const before = structuredClone(result.current.document); forceRevision(9); act(() => result.current.redo());
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('Drawing changed in another workspace. Reload before editing.'));
    expect(result.current.document).toEqual(before); expect(result.current.canUndo).toBe(false); expect(result.current.canRedo).toBe(true);
  });
  it('rolls local state back and does not advance history when backend persistence fails', async () => {
    const { result } = await readyHook(); failWrite = true; act(() => result.current.providerEvent({ type: 'created', drawing }));
    await waitFor(() => expect(result.current.persistenceStatus).toBe('error'));
    expect(result.current.document).toMatchObject({ revision: 0, drawings: [] }); expect(result.current.canUndo).toBe(false);
    expect(window.localStorage.getItem(key)).toBeNull(); expect(result.current.persistenceStatus).toBe('error');
  });
});
