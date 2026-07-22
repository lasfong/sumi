import React, { useCallback, useState } from 'react';
import type { MagnetMode } from '../../features/drawings/drawingMagnet';
import { TEXT_MAX_LENGTH, type DrawingTool } from '../../features/drawings/drawingDomain';
import { useModalFocus } from '../../hooks/useModalFocus';

interface Props {
  activeTool: DrawingTool; onSelectTool: (tool: DrawingTool) => void;
  onClearAll: () => void; onUndo: () => void; onRedo: () => void; canUndo: boolean; canRedo: boolean;
  pendingText: boolean; onCommitText: (text: string) => boolean; onCancelText: () => void;
  magnetMode: MagnetMode; onMagnetMode: (mode: MagnetMode) => void; persistenceStatus: string;
}

const TOOLS: Array<{ id: DrawingTool; label: string; icon: string; hint: string }> = [
  { id: 'select', label: 'Cursor / Select', icon: '↖', hint: 'Select topmost drawing; empty click deselects' },
  { id: 'horizontal', label: 'Horizontal Line', icon: '—', hint: 'Click once to place' },
  { id: 'trendline', label: 'Trendline', icon: '╱', hint: 'Click two endpoints' },
  { id: 'ray', label: 'Ray', icon: '↗', hint: 'Click origin and direction' },
  { id: 'rectangle', label: 'Rectangle', icon: '□', hint: 'Click opposite corners' },
  { id: 'fibonacci-retracement', label: 'Fibonacci Retracement', icon: '≋', hint: 'Click two directional anchors' },
  { id: 'text', label: 'Text / Note', icon: 'T', hint: 'Click anchor, then enter text' },
];

export const DrawingToolbar: React.FC<Props> = props => {
  const [confirmClear, setConfirmClear] = useState(false); const [textDraft, setTextDraft] = useState('');
  const [dialogElement, setDialogElement] = useState<HTMLDivElement | null>(null);
  const { onCancelText } = props;
  const cancelText = useCallback(() => { setTextDraft(''); onCancelText(); }, [onCancelText]);
  useModalFocus(props.pendingText, cancelText, dialogElement);
  const commitText = () => { if (props.onCommitText(textDraft)) setTextDraft(''); };
  return <>
    <div className="drawing-toolbar" data-testid="drawing-toolbar" style={{ display: 'flex', flexDirection: 'column', gap: 2, padding: 4, background: 'var(--bg-panel)', borderRight: '1px solid var(--border-color)', width: 72, alignItems: 'center', overflowY: 'auto' }}>
      {TOOLS.map(tool => <button key={tool.id} type="button" title={tool.id === 'text' ? 'Text' : tool.label} aria-label={tool.label} aria-description={tool.hint} data-testid={`drawing-tool-${tool.id}`} aria-pressed={props.activeTool === tool.id} onClick={() => props.onSelectTool(tool.id)} style={{ width: 40, height: 28, minHeight: 28, padding: 0, fontSize: 16, background: props.activeTool === tool.id ? 'rgba(41,98,255,.25)' : 'transparent', color: props.activeTool === tool.id ? '#75a7ff' : 'var(--text-muted)', border: `1px solid ${props.activeTool === tool.id ? '#2962ff' : 'transparent'}`, borderRadius: 5 }}>{tool.icon}</button>)}
      <label title="Drawing magnet mode" style={{ display: 'grid', gap: 2, fontSize: 9, color: 'var(--text-muted)', textAlign: 'center' }}>Magnet
        <select aria-label="Drawing magnet mode" data-testid="drawing-magnet-mode" value={props.magnetMode} onChange={event => props.onMagnetMode(event.target.value as MagnetMode)} style={{ width: 60, height: 24, padding: 0, fontSize: 10 }}><option value="off">Off</option><option value="ohlc">OHLC</option></select>
      </label>
      <button type="button" title="Undo drawing" aria-label="Undo drawing" data-testid="undo-drawing" disabled={!props.canUndo} onClick={props.onUndo} style={{ width: 40, height: 26, padding: 0 }}>↶</button>
      <button type="button" title="Redo drawing" aria-label="Redo drawing" data-testid="redo-drawing" disabled={!props.canRedo} onClick={props.onRedo} style={{ width: 40, height: 26, padding: 0 }}>↷</button>
      <div style={{ flex: 1 }} />
      <span data-testid="drawing-persistence-status" title={`Drawing persistence: ${props.persistenceStatus}`} style={{ fontSize: 8, color: props.persistenceStatus === 'ready' ? '#00E676' : '#FFD166' }}>{props.persistenceStatus}</span>
      {!confirmClear ? <button type="button" title="Clear All Drawings" aria-label="Clear All Drawings" data-testid="clear-all-drawings" onClick={() => setConfirmClear(true)} style={{ width: 40, height: 28, padding: 0 }}>🗑</button>
        : <button type="button" data-testid="confirm-clear-drawings" onClick={() => { props.onClearAll(); setConfirmClear(false); }} style={{ height: 28, padding: 0 }}>Confirm</button>}
    </div>
    {props.pendingText && <div ref={setDialogElement} role="dialog" aria-modal="true" aria-label="Create Text / Note" data-testid="drawing-text-dialog" style={{ position: 'fixed', inset: 0, zIndex: 1000, background: 'rgba(0,0,0,.55)', display: 'grid', placeItems: 'center' }}>
      <div className="panel" style={{ width: 360, padding: 16, display: 'grid', gap: 10 }}><strong>Text / Note</strong>
        <textarea autoFocus aria-label="Text note content" data-testid="new-drawing-text" value={textDraft} maxLength={TEXT_MAX_LENGTH} onChange={event => setTextDraft(event.target.value)} onKeyDown={event => { if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') commitText(); }} />
        <small>{textDraft.length}/{TEXT_MAX_LENGTH}</small><div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8 }}><button type="button" onClick={cancelText}>Cancel</button><button type="button" data-testid="commit-drawing-text" disabled={!textDraft.trim()} onClick={commitText}>Add note</button></div>
      </div>
    </div>}
  </>;
};
