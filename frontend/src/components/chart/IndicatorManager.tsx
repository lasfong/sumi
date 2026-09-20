import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { IndicatorDefinition } from '../../api/indicatorsApi';
import {
  defaultsFor, validateIndicatorParams,
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
  runtime?: Record<string, IndicatorRuntimeState>;
  onAdd: (definition: IndicatorDefinition, params: Record<string, unknown>) => void;
  onUpdate: (id: string, params: Record<string, unknown>, styles: Record<string, IndicatorSeriesStyle>) => void;
  onRemove?: (id: string) => void;
  onToggle?: (id: string) => void;
  onMove?: (id: string, direction: -1 | 1) => void;
}

const SERIES_LABELS: Record<string, string> = {
  primary: 'Đường chính',
  macd: 'Đường MACD',
  signal: 'Đường Signal',
  histogram: 'Cột Histogram',
  volume: 'Khối lượng (Volume)',
  upper: 'Dải trên (Upper)',
  middle: 'Dải giữa (Middle)',
  lower: 'Dải dưới (Lower)',
  tenkan: 'Tenkan-sen (Chuyển đổi)',
  kijun: 'Kijun-sen (Tiêu chuẩn)',
  spanA: 'Senkou Span A',
  spanB: 'Senkou Span B',
  chikou: 'Chikou Span (Trễ)',
  cloudUp: 'Mây Kumo Tăng (Bull)',
  cloudDown: 'Mây Kumo Giảm (Bear)',
  sar: 'Chấm SAR',
  supertrend: 'Supertrend',
  bull: 'Xu hướng Tăng',
  bear: 'Xu hướng Giảm',
  k: 'Đường %K',
  d: 'Đường %D',
  adx: 'Đường ADX',
  dmp: 'Đường +DI',
  dmn: 'Đường -DI',
};

export const IndicatorManager: React.FC<Props> = ({ definitions, document, onAdd, onUpdate }) => {
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

  const openAdd = useCallback(() => { close(); setMode('add'); }, [close]);
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
    const handleAddRequest = () => openAdd();
    window.addEventListener('sumi:indicator-settings-request', handlePaneSettings);
    window.addEventListener('sumi:indicator-add-request', handleAddRequest);
    return () => {
      window.removeEventListener('sumi:indicator-settings-request', handlePaneSettings);
      window.removeEventListener('sumi:indicator-add-request', handleAddRequest);
    };
  }, [document.instances, openAdd]);

  return <>
    <section data-testid="indicator-manager" aria-label="Indicator Manager" style={{ display: 'none' }} />
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
          {mode === 'settings' && Object.keys(styleDraft).length > 0 && (
            <div style={{ marginTop: 14, paddingTop: 12, borderTop: '1px solid rgba(255,255,255,0.08)' }}>
              <h4 style={{ margin: '0 0 10px 0', fontSize: 13, color: '#F0F6FC' }}>Tùy chỉnh màu sắc đồ thị</h4>
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(220px, 1fr))', gap: 8 }}>
                {Object.entries(styleDraft).map(([series, style]) => (
                  <label key={series} style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '6px 10px', background: 'rgba(255,255,255,0.03)', borderRadius: 6, border: '1px solid rgba(255,255,255,0.06)' }}>
                    <span style={{ fontSize: 12, color: '#C9D1D9' }}>{SERIES_LABELS[series] || series}</span>
                    <input
                      aria-label={`${series} color`}
                      type="color"
                      value={style.color || '#2962FF'}
                      onChange={event => setStyleDraft(previous => ({ ...previous, [series]: { ...style, color: event.target.value } }))}
                      style={{ width: 32, height: 26, border: 'none', borderRadius: 4, cursor: 'pointer', background: 'transparent' }}
                    />
                  </label>
                ))}
              </div>
            </div>
          )}
          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8, marginTop: 14 }}><button type="button" data-testid="cancel-indicator-dialog" onClick={close}>Cancel</button><button type="submit" data-testid={mode === 'add' ? 'confirm-add-indicator' : 'apply-indicator-settings'} disabled={!canSubmit}>{mode === 'add' ? 'Add Indicator' : 'Apply Settings'}</button></div>
        </form>}
      </div>
    </div>}
  </>;
};
