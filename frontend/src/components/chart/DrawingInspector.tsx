import React, { useMemo, useState } from 'react';
import {
  TEXT_MAX_LENGTH, validateDrawingDocument, type SumiDrawing, type SumiDrawingDocumentV1,
} from '../../features/drawings/drawingDomain';

interface Props { selected?: SumiDrawing; persistenceStatus: string; onApply: (drawing: SumiDrawing) => boolean; onDelete: () => void }

const validDraft = (drawing: SumiDrawing): boolean => validateDrawingDocument({
  schemaVersion: 1, revision: 0, sessionId: 1, symbol: 'DRAFT', drawings: [{ ...drawing, order: 0 }],
} satisfies SumiDrawingDocumentV1);

export const DrawingInspector: React.FC<Props> = ({ selected, persistenceStatus, onApply, onDelete }) => {
  const [draft, setDraft] = useState<SumiDrawing | null>(() => selected ? structuredClone(selected) : null);
  const [message, setMessage] = useState('');
  const valid = useMemo(() => draft ? validDraft(draft) : false, [draft]);
  if (!selected || !draft) return null;
  const controlStyle: React.CSSProperties = { width: '100%', minWidth: 0, boxSizing: 'border-box' };
  const updateAnchor = (index: number, field: 'time' | 'price', value: string | number) => setDraft(previous => previous ? ({
    ...previous, anchors: previous.anchors.map((anchor, anchorIndex) => anchorIndex === index ? { ...anchor, [field]: value } : anchor),
  } as SumiDrawing) : previous);
  const updateStyle = (patch: Partial<SumiDrawing['style']>) => setDraft(previous => previous ? ({ ...previous, style: { ...previous.style, ...patch } } as SumiDrawing) : previous);
  const apply = () => {
    if (!valid) { setMessage('Check dates, positive prices, required text, style ranges, and Ray direction.'); return; }
    if (!onApply(structuredClone(draft))) { setMessage('The change was not accepted. Review the highlighted values.'); return; }
    setMessage('Saving one drawing command…');
  };
  const cancel = () => { setDraft(structuredClone(selected)); setMessage('Draft discarded.'); };
  const stopChartKeys = (event: React.KeyboardEvent) => {
    if (['Escape', 'Delete', 'Backspace', 'ArrowLeft', 'ArrowRight', 'Space'].includes(event.code)) event.stopPropagation();
    if (event.code === 'Escape') { event.preventDefault(); cancel(); }
  };
  return <section data-testid="drawing-selection-toolbar" aria-label={`Selected ${selected.tool} ${selected.id}`} onKeyDownCapture={stopChartKeys}
    style={{ display: 'grid', gap: 8, padding: 10, border: '1px solid var(--border-color)', borderRadius: 7, background: 'var(--bg-panel)', fontSize: 12, width: '100%', maxWidth: '100%', minWidth: 0, boxSizing: 'border-box', overflowX: 'hidden' }}>
    <header style={{ display: 'grid', gap: 3 }}>
      <strong data-testid="selected-drawing-type" style={{ fontSize: 14 }}>{selected.tool}</strong>
      <span>Drawing ID</span>
      <code data-testid="selected-drawing-id" title={selected.id} aria-label={`Full drawing ID ${selected.id}`} style={{ overflowWrap: 'anywhere' }}>{selected.id.slice(0, 8)}…</code>
    </header>
    <fieldset style={{ display: 'grid', gap: 8, minWidth: 0 }}><legend>Anchors</legend>
      {draft.anchors.map((anchor, index) => {
        const anchorLabel = draft.tool === 'risk-reward'
          ? index === 0 ? 'Entry' : index === 1 ? 'Stop Loss' : 'Target Price'
          : `Anchor ${index + 1}`;
        return <div key={index} style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr)', gap: 6 }}>
          <label style={{ display: 'grid', gap: 3 }}>{anchorLabel} date
            <input aria-label={`${anchorLabel} time`} data-testid={`drawing-anchor-${index}-time`} type="date" value={anchor.time} onChange={event => updateAnchor(index, 'time', event.target.value)} style={controlStyle} />
          </label>
          <label style={{ display: 'grid', gap: 3 }}>{anchorLabel} price
            <input aria-label={index === 0 ? 'Selected line price' : `${anchorLabel} price`} data-testid={index === 0 ? 'drawing-price-input' : `drawing-anchor-${index}-price`}
              type="number" min="0.01" step="0.01" value={anchor.price} onChange={event => updateAnchor(index, 'price', Number(event.target.value))} style={controlStyle} />
          </label>
        </div>;
      })}
    </fieldset>
    <fieldset style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)', gap: 8, minWidth: 0 }}><legend>Line</legend>
      <label style={{ display: 'grid', gap: 3, minWidth: 0 }}>Color<input data-testid="drawing-line-color" aria-label="Line color" value={draft.style.lineColor} onChange={event => updateStyle({ lineColor: event.target.value })} style={controlStyle} /></label>
      <label style={{ display: 'grid', gap: 3, minWidth: 0 }}>Width<input data-testid="drawing-line-width" aria-label="Line width" type="number" min="1" max="8" value={draft.style.lineWidth} onChange={event => updateStyle({ lineWidth: Number(event.target.value) })} style={controlStyle} /></label>
      <label style={{ display: 'grid', gap: 3, gridColumn: '1 / -1', minWidth: 0 }}>Style<select data-testid="drawing-line-style" aria-label="Line style" value={draft.style.lineStyle} onChange={event => updateStyle({ lineStyle: event.target.value as SumiDrawing['style']['lineStyle'] })} style={controlStyle}><option value="solid">Solid</option><option value="dashed">Dashed</option><option value="dotted">Dotted</option></select></label>
    </fieldset>
    {(draft.tool === 'rectangle' || draft.tool === 'fibonacci-retracement' || draft.tool === 'risk-reward') && <fieldset style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 1fr)', gap: 8, minWidth: 0 }}><legend>Fill</legend>
      <label style={{ display: 'grid', gap: 3, minWidth: 0 }}>Color<input data-testid="drawing-fill-color" aria-label="Fill color" value={draft.style.fillColor ?? draft.style.lineColor} onChange={event => updateStyle({ fillColor: event.target.value })} style={controlStyle} /></label>
      <label style={{ display: 'grid', gap: 3, minWidth: 0 }}>Opacity<input data-testid="drawing-fill-opacity" aria-label="Fill opacity" type="number" min="0" max="1" step="0.05" value={draft.style.fillOpacity ?? 0} onChange={event => updateStyle({ fillOpacity: Number(event.target.value) })} style={controlStyle} /></label>
    </fieldset>}
    {draft.tool === 'risk-reward' && <div style={{ display: 'grid', gap: 4, padding: 6, background: 'rgba(41,98,255,.1)', borderRadius: 4 }}>
      <div>Direction: <strong style={{ color: '#58A6FF' }}>{draft.geometry.direction.toUpperCase()}</strong></div>
      <div>Planned R:R: <strong style={{ color: '#00E676' }}>{draft.geometry.riskRewardRatio.toFixed(2)}</strong></div>
    </div>}
    {draft.tool === 'fibonacci-retracement' && <label style={{ display: 'grid', gap: 3 }}>Fibonacci direction
      <select data-testid="drawing-fibonacci-direction" value={draft.geometry.direction} onChange={event => setDraft(previous => previous?.tool === 'fibonacci-retracement' ? { ...previous, geometry: { ...previous.geometry, direction: event.target.value as 'start-to-end' | 'end-to-start' } } : previous)} style={controlStyle}><option value="start-to-end">Start to end</option><option value="end-to-start">End to start</option></select>
      <button type="button" data-testid="reverse-fibonacci" onClick={() => setDraft(previous => previous?.tool === 'fibonacci-retracement' ? { ...previous, geometry: { ...previous.geometry, direction: previous.geometry.direction === 'start-to-end' ? 'end-to-start' : 'start-to-end' } } : previous)}>Reverse direction</button>
    </label>}
    {draft.tool === 'text' && <fieldset style={{ display: 'grid', gap: 8 }}><legend>Text</legend>
      <label style={{ display: 'grid', gap: 3 }}>Content<textarea aria-label="Drawing text" data-testid="drawing-text-input" value={draft.geometry.text} maxLength={TEXT_MAX_LENGTH} onChange={event => setDraft(previous => previous?.tool === 'text' ? { ...previous, geometry: { ...previous.geometry, text: event.target.value } } : previous)} style={{ ...controlStyle, minHeight: 88, resize: 'vertical' }} /></label>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
        <label style={{ display: 'grid', gap: 3, minWidth: 0 }}>Text color<input data-testid="drawing-text-color" aria-label="Text color" value={draft.style.textColor ?? draft.style.lineColor} onChange={event => updateStyle({ textColor: event.target.value })} style={controlStyle} /></label>
        <label style={{ display: 'grid', gap: 3, minWidth: 0 }}>Font size<input data-testid="drawing-font-size" aria-label="Font size" type="number" min="8" max="72" value={draft.style.fontSize ?? 14} onChange={event => updateStyle({ fontSize: Number(event.target.value) })} style={controlStyle} /></label>
      </div>
      <small>{draft.geometry.text.length}/{TEXT_MAX_LENGTH}</small>
    </fieldset>}
    <div style={{ display: 'flex', gap: 12 }}>
      <label><input data-testid="drawing-visible" type="checkbox" checked={draft.visible} onChange={event => setDraft(previous => previous ? { ...previous, visible: event.target.checked } : previous)} /> Visible</label>
      <label><input data-testid="drawing-locked" type="checkbox" checked={draft.locked} onChange={event => setDraft(previous => previous ? { ...previous, locked: event.target.checked } : previous)} /> Locked</label>
    </div>
    {!valid && <div role="alert" data-testid="drawing-inspector-validation" style={{ color: '#ff8a80' }}>Invalid values — changes are not persisted.</div>}
    {message && <small aria-live="polite">{message}</small>}
    <div style={{ display: 'flex', flexWrap: 'wrap', gap: 7 }}>
      <button type="button" data-testid="apply-drawing-settings" disabled={!valid || persistenceStatus !== 'ready'} onClick={apply}>Apply</button>
      <button type="button" data-testid="cancel-drawing-settings" onClick={cancel}>Cancel</button>
      <button type="button" title="Delete selected drawing" aria-label="Delete selected drawing" data-testid="delete-selected-drawing" onClick={onDelete} style={{ marginLeft: 'auto' }}>Delete</button>
    </div>
  </section>;
};
