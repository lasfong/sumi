import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { IndicatorDefinition } from '../../api/indicatorsApi';
import {
  defaultsFor, formatIndicatorParams, validateIndicatorParams,
  type IndicatorDocumentV1, type IndicatorInstanceV1, type IndicatorSeriesStyle,
} from '../../features/indicators/indicatorDomain';
import { useModalFocus } from '../../hooks/useModalFocus';

export interface IndicatorRuntimeState {
  status: 'idle' | 'loading' | 'ready' | 'warming' | 'error';
  values: Record<string, number | null>;
  error?: string;
  errorKind?: 'transport' | 'mapping' | 'chart';
  inputMaxDate?: string | null;
  responseMaxDate?: string | null;
  responseCount?: number;
}

interface Props {
  definitions: IndicatorDefinition[];
  document: IndicatorDocumentV1;
  runtime: Record<string, IndicatorRuntimeState>;
  onAdd: (definition: IndicatorDefinition, params: Record<string, unknown>) => void;
  onUpdate: (id: string, params: Record<string, unknown>, styles: Record<string, IndicatorSeriesStyle>) => void;
  onRemove: (id: string) => void;
  onToggle: (id: string) => void;
  onMove: (id: string, direction: -1 | 1) => void;
}

const valueText = (value: number | null | undefined) => value === null || value === undefined ? 'warming up' : value.toLocaleString(undefined, { maximumFractionDigits: 2 });

export const IndicatorManager: React.FC<Props> = ({ definitions, document, runtime, onAdd, onUpdate, onRemove, onToggle, onMove }) => {
  const [mode, setMode] = useState<'add' | 'settings' | null>(null);
  const [search, setSearch] = useState('');
  const [selectedDefinition, setSelectedDefinition] = useState<IndicatorDefinition | null>(null);
  const [selectedInstance, setSelectedInstance] = useState<IndicatorInstanceV1 | null>(null);
  const [draft, setDraft] = useState<Record<string, unknown>>({});
  const [styleDraft, setStyleDraft] = useState<Record<string, IndicatorSeriesStyle>>({});
  const [dialogElement, setDialogElement] = useState<HTMLDivElement | null>(null);
  const close = useCallback(() => { setMode(null); setSelectedDefinition(null); setSelectedInstance(null); setDraft({}); setStyleDraft({}); setSearch(''); }, []);
  useModalFocus(mode !== null, close, dialogElement);
  const firstInputRef = useRef<HTMLInputElement>(null);
  useEffect(() => { if (mode) firstInputRef.current?.focus(); }, [mode, selectedDefinition]);

  const filtered = useMemo(() => definitions.filter(definition =>
    `${definition.label} ${definition.id} ${definition.category}`.toLowerCase().includes(search.toLowerCase())), [definitions, search]);
  const activeDefinition = mode === 'settings' && selectedInstance
    ? definitions.find(definition => definition.id === selectedInstance.definitionId) ?? null : selectedDefinition;
  const validation = activeDefinition ? validateIndicatorParams(activeDefinition, draft) : { params: {}, errors: {} };
  const canSubmit = !!activeDefinition && Object.keys(validation.errors).length === 0;

  const openAdd = () => { close(); setMode('add'); };
  const chooseDefinition = (definition: IndicatorDefinition) => { setSelectedDefinition(definition); setDraft(defaultsFor(definition)); setStyleDraft({}); };
  const openSettings = (instance: IndicatorInstanceV1) => {
    setSelectedInstance(instance); setDraft(structuredClone(instance.params)); setStyleDraft(structuredClone(instance.styles)); setMode('settings');
  };
  useEffect(() => {
    const handlePaneSettings = (event: Event) => {
      const id = (event as CustomEvent<string>).detail;
      const instance = document.instances.find(item => item.id === id);
      if (instance) openSettings(instance);
    };
    window.addEventListener('sumi:indicator-settings-request', handlePaneSettings);
    return () => window.removeEventListener('sumi:indicator-settings-request', handlePaneSettings);
  }, [document.instances]);

  return <section data-testid="indicator-manager" aria-label="Indicator Manager" style={{ borderBottom: '1px solid var(--border-color)', background: 'rgba(13,17,23,.96)', padding: '8px 10px', display: 'grid', gap: 7 }}>
    <div style={{ display: 'flex', justifyContent: 'space-between', gap: 8, alignItems: 'center' }}>
      <div><strong style={{ fontSize: 13 }}>Active indicators</strong><span style={{ color: 'var(--text-muted)', fontSize: 11, marginLeft: 8 }}>{document.instances.length} configured</span></div>
      <button type="button" data-testid="open-add-indicator" aria-label="Add indicator" onClick={openAdd}>+ Add Indicator</button>
    </div>
    <div data-testid="active-indicator-list" style={{ display: 'flex', gap: 7, overflowX: 'auto', minHeight: 58 }}>
      {!document.instances.length && <div data-testid="indicator-empty-state" style={{ color: 'var(--text-muted)', fontSize: 12, padding: 12 }}>No active indicators. Add SMA, EMA, RSI, MACD, CCI, Bollinger Bands, ATR, Volume, Volume SMA, MFI, Stochastic, ADX, or Relative Strength.</div>}
      {document.instances.map((instance, index) => {
        const state = runtime[instance.id] ?? { status: 'idle', values: {} };
        const mainColor = Object.values(instance.styles)[0]?.color ?? '#58A6FF';
        return <article key={instance.id} data-testid={`indicator-instance-${instance.id}`} data-definition={instance.definitionId} data-pane={instance.paneId} data-visible={instance.visible} style={{ flex: '0 0 210px', minWidth: 0, border: `1px solid ${instance.visible ? mainColor : 'var(--border-color)'}`, borderRadius: 5, padding: '5px 6px', opacity: instance.visible ? 1 : .58, background: 'var(--bg-panel)' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <span aria-hidden="true" style={{ width: 9, height: 9, borderRadius: 9, background: mainColor }} />
            <strong style={{ fontSize: 12 }}>{instance.label}</strong>
            <span style={{ color: 'var(--text-muted)', fontSize: 10 }}>{instance.placement} pane</span>
            <span data-testid={`indicator-status-${instance.id}`} style={{ marginLeft: 'auto', fontSize: 10, color: state.status === 'error' ? '#FF5252' : 'var(--text-muted)' }}>{instance.visible ? state.status : 'hidden'}</span>
          </div>
          <div style={{ fontSize: 10, color: 'var(--text-muted)', margin: '3px 0' }}>{formatIndicatorParams(instance)}</div>
          <div data-testid={`indicator-values-${instance.id}`} style={{ display: 'flex', flexWrap: 'wrap', gap: 6, fontFamily: 'monospace', fontSize: 10 }}>
            {Object.entries(state.values).map(([name, value]) => <span key={name}>{name}: {valueText(value)}</span>)}
            {!Object.keys(state.values).length && <span>{state.error ?? (instance.visible ? 'waiting for data' : 'settings preserved')}</span>}
          </div>
          <div style={{ display: 'flex', flexWrap: 'nowrap', gap: 3, marginTop: 4 }}>
            <button style={{ padding: '2px 4px', fontSize: 9 }} type="button" data-testid={`toggle-indicator-${instance.id}`} aria-label={`${instance.visible ? 'Hide' : 'Show'} ${instance.label}`} onClick={() => onToggle(instance.id)}>{instance.visible ? 'Hide' : 'Show'}</button>
            <button style={{ padding: '2px 4px', fontSize: 9 }} type="button" data-testid={`indicator-settings-${instance.id}`} aria-label={`Settings for ${instance.label}`} onClick={() => openSettings(instance)}>Settings</button>
            <button style={{ padding: '2px 4px', fontSize: 9 }} type="button" data-testid={`remove-indicator-${instance.id}`} aria-label={`Remove ${instance.label}`} onClick={() => onRemove(instance.id)}>Remove</button>
            <button style={{ padding: '2px 4px', fontSize: 9 }} type="button" aria-label={`Move ${instance.label} up`} disabled={index === 0} onClick={() => onMove(instance.id, -1)}>↑</button>
            <button style={{ padding: '2px 4px', fontSize: 9 }} type="button" aria-label={`Move ${instance.label} down`} disabled={index === document.instances.length - 1} onClick={() => onMove(instance.id, 1)}>↓</button>
          </div>
        </article>;
      })}
    </div>

    {mode && <div role="presentation" style={{ position: 'fixed', inset: 0, zIndex: 200, background: 'rgba(0,0,0,.65)', display: 'grid', placeItems: 'center' }} onMouseDown={event => { if (event.target === event.currentTarget) close(); }}>
      <div ref={setDialogElement} role="dialog" aria-modal="true" aria-labelledby="indicator-dialog-title" data-testid="indicator-dialog" style={{ width: 'min(620px, calc(100vw - 32px))', maxHeight: '85vh', overflow: 'auto', background: '#161B22', border: '1px solid var(--border-color)', borderRadius: 8, padding: 16 }}>
        <div style={{ display: 'flex', justifyContent: 'space-between' }}><h3 id="indicator-dialog-title" style={{ margin: 0 }}>{mode === 'add' ? 'Add Indicator' : `Settings — ${selectedInstance?.label}`}</h3><button type="button" aria-label="Close indicator dialog" onClick={close}>Close</button></div>
        {mode === 'add' && !selectedDefinition && <>
          <label style={{ display: 'grid', gap: 5, margin: '12px 0' }}>Search or category
            <input ref={firstInputRef} data-testid="indicator-search" value={search} onChange={event => setSearch(event.target.value)} placeholder="Search EMA, momentum, volume…" />
          </label>
          <div style={{ display: 'grid', gap: 7 }}>{filtered.map(definition => <button type="button" key={definition.id} data-testid={`add-definition-${definition.id}`} onClick={() => chooseDefinition(definition)} style={{ textAlign: 'left', padding: 9 }}><strong>{definition.label}</strong> <span style={{ color: 'var(--text-muted)' }}>— {definition.category}</span><br/><span style={{ fontSize: 11 }}>{definition.description}</span></button>)}</div>
        </>}
        {activeDefinition && <form onSubmit={event => {
          event.preventDefault(); if (!canSubmit) return;
          if (mode === 'add') onAdd(activeDefinition, validation.params);
          else if (selectedInstance) onUpdate(selectedInstance.id, validation.params, styleDraft);
          close();
        }}>
          <p style={{ color: 'var(--text-muted)', fontSize: 12 }}>{activeDefinition.description}</p>
          {activeDefinition.params.map((param, index) => <label key={param.name} style={{ display: 'grid', gridTemplateColumns: '120px 1fr', alignItems: 'center', gap: 8, margin: '8px 0' }}>{param.name}
            <span><input ref={index === 0 ? firstInputRef : undefined} data-testid={`indicator-param-${param.name}`} type={param.type === 'int' || param.type === 'float' ? 'number' : 'text'} step={param.type === 'int' ? 1 : 'any'} min={param.minimum ?? undefined} max={param.maximum ?? undefined} value={String(draft[param.name] ?? '')} onChange={event => setDraft(previous => ({ ...previous, [param.name]: event.target.value }))} />{validation.errors[param.name] && <span role="alert" style={{ display: 'block', color: '#FF5252', fontSize: 11 }}>{validation.errors[param.name]}</span>}</span>
          </label>)}
          {mode === 'settings' && Object.entries(styleDraft).map(([series, style]) => <label key={series} style={{ display: 'grid', gridTemplateColumns: '120px 1fr', gap: 8, alignItems: 'center', margin: '8px 0' }}>{series} color<input aria-label={`${series} color`} type="color" value={style.color} onChange={event => setStyleDraft(previous => ({ ...previous, [series]: { ...style, color: event.target.value } }))} /></label>)}
          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8, marginTop: 14 }}><button type="button" data-testid="cancel-indicator-dialog" onClick={close}>Cancel</button><button type="submit" data-testid={mode === 'add' ? 'confirm-add-indicator' : 'apply-indicator-settings'} disabled={!canSubmit}>{mode === 'add' ? 'Add Indicator' : 'Apply Settings'}</button></div>
        </form>}
      </div>
    </div>}
  </section>;
};
