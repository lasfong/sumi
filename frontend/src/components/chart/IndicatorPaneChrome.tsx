import React from 'react';
import { formatIndicatorParams, type IndicatorDocumentV1 } from '../../features/indicators/indicatorDomain';
import { IndicatorRenderRegistry } from './IndicatorRenderRegistry';
import type { IndicatorRuntimeState } from './IndicatorManager';

interface Props {
  document: IndicatorDocumentV1;
  runtime: Record<string, IndicatorRuntimeState>;
  onSettings: (id: string) => void;
  onToggle: (id: string) => void;
  onRemove: (id: string) => void;
}

const valueText = (value: number | null | undefined) => value == null
  ? 'warming' : value.toLocaleString(undefined, { maximumFractionDigits: 2 });

export const IndicatorPaneChrome: React.FC<Props> = ({ document, runtime, onSettings, onToggle, onRemove }) => {
  const visible = document.instances.filter(instance => instance.visible && instance.placement !== 'price');
  if (!visible.length) return null;
  return <div data-testid="indicator-pane-chrome-layer" aria-label="Visible indicator pane controls" style={{ position: 'absolute', inset: '0 68px 26px 0', zIndex: 6, pointerEvents: 'none', display: 'flex', flexDirection: 'column' }}>
    <div aria-hidden="true" style={{ flex: '4 1 0', minHeight: 240 }} />
    {visible.map(instance => {
      const state = runtime[instance.id] ?? { status: 'idle', values: {} };
      const references = IndicatorRenderRegistry.referencesFor(instance.definitionId);
      const components = instance.definitionId === 'macd' ? ['macd', 'signal', 'histogram'] : Object.keys(state.values);
      return <section key={instance.id} data-testid={`indicator-pane-${instance.definitionId}`} data-pane-instance-id={instance.id} data-pane-id={instance.paneId} data-status={state.status} aria-label={`${instance.label} pane`} style={{ flex: '1 1 0', minHeight: 60, position: 'relative', borderTop: '1px solid rgba(255,255,255,.12)' }}>
        <div data-testid={`indicator-pane-chrome-${instance.id}`} style={{ pointerEvents: 'auto', position: 'absolute', top: 3, left: 5, maxWidth: 'calc(100% - 10px)', display: 'flex', flexWrap: 'wrap', alignItems: 'center', gap: '2px 7px', padding: '2px 5px', borderRadius: 4, background: 'rgba(13,17,23,.78)', color: '#F0F6FC', fontSize: 10, lineHeight: 1.35 }}>
          <strong>{instance.label}</strong>
          <span data-testid={`pane-params-${instance.id}`} style={{ color: '#9DA7B3' }}>{formatIndicatorParams(instance)}</span>
          <span data-testid={instance.definitionId === 'macd' ? 'macd-components' : `pane-values-${instance.id}`} style={{ display: 'flex', gap: 6, fontFamily: 'monospace' }}>
            {components.map(name => <span key={name}>{name}: {valueText(state.values[name])}</span>)}
            {!components.length && <span>{state.status}</span>}
          </span>
          {!!references.length && <span aria-label={`${instance.label} reference levels`} style={{ display: 'flex', gap: 4, color: '#FFD166' }}>
            <span>{instance.definitionId === 'macd' ? 'MACD ref: Zero' : `${instance.definitionId.toUpperCase()} refs:`}</span>
            <span data-testid={`${instance.definitionId}-reference-lines`}>{references.map(reference => reference.value).join(',')}</span>
          </span>}
          <span style={{ display: 'flex', gap: 3 }}>
            <button type="button" data-testid={`pane-settings-${instance.id}`} aria-label={`Pane settings for ${instance.label}`} onClick={() => onSettings(instance.id)} style={{ padding: '1px 4px', fontSize: 9 }}>Settings</button>
            <button type="button" data-testid={`pane-toggle-${instance.id}`} aria-label={`Hide ${instance.label} pane`} onClick={() => onToggle(instance.id)} style={{ padding: '1px 4px', fontSize: 9 }}>Hide</button>
            <button type="button" data-testid={`pane-remove-${instance.id}`} aria-label={`Close ${instance.label} pane`} onClick={() => onRemove(instance.id)} style={{ padding: '1px 4px', fontSize: 9 }}>Close</button>
          </span>
          {state.status === 'error' && <span role="alert" style={{ color: '#FF7B72' }}>{state.error}</span>}
        </div>
      </section>;
    })}
  </div>;
};
