import { describe, expect, it } from 'vitest';
import type { IndicatorInstanceV1 } from '../../../features/indicators/indicatorDomain';
import { IndicatorRenderRegistry } from '../IndicatorRenderRegistry';

const instance = (definitionId: IndicatorInstanceV1['definitionId']): IndicatorInstanceV1 => ({
  id: `00000000-0000-4000-8000-00000000000${definitionId.length}`,
  definitionId,
  label: definitionId.toUpperCase(),
  params: definitionId === 'macd' ? { fast: 12, slow: 26, signal: 9 } : { length: 20 },
  placement: definitionId === 'ema' ? 'price' : 'oscillator',
  paneId: definitionId === 'ema' ? 'price' : `indicator:test-${definitionId}`,
  visible: true,
  order: 0,
  styles: {
    primary: { color: '#58A6FF' }, macd: { color: '#58A6FF' }, signal: { color: '#F59E0B' },
    histogram: { color: '#A78BFA' }, volume: { color: '#58A6FF' },
  },
});

describe('IndicatorRenderRegistry', () => {
  it('renders semantic MACD line, signal, histogram and zero reference', () => {
    const rendered = IndicatorRenderRegistry.mapBackendData([{
      timestamp: '2026-01-02T00:00:00', MACD_12_26_9: 1.2, MACDs_12_26_9: 0.8, MACDh_12_26_9: 0.4,
    }], instance('macd'));

    expect(rendered.map(series => series.seriesKey)).toEqual(['macd', 'signal', 'histogram']);
    expect(rendered[2]).toMatchObject({ type: 'histogram', references: [{ value: 0, label: 'Zero' }] });
    expect(rendered.every(series => series.data[0].time === '2026-01-02')).toBe(true);
  });

  it('provides professional RSI and CCI scale/reference semantics', () => {
    const rsi = IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', RSI_20: 55 }], instance('rsi'))[0];
    const cci = IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', CCI_20: -12 }], instance('cci'))[0];
    expect(rsi.scale).toEqual({ minimum: 0, maximum: 100 });
    expect(rsi.references?.map(line => line.value)).toEqual([30, 50, 70]);
    expect(cci.references?.map(line => line.value)).toEqual([-100, 0, 100]);
  });

  it.each(['ema', 'rsi', 'cci', 'macd'] as const)('does not turn %s backend warm-up nulls into zero', definitionId => {
    const column = definitionId === 'macd' ? 'MACD_12_26_9' : `${definitionId.toUpperCase()}_20`;
    const rendered = IndicatorRenderRegistry.mapBackendData([
      { timestamp: '2026-01-01', [column]: null },
      { timestamp: '2026-01-02', [column]: 101.5 },
    ], instance(definitionId));
    expect(rendered[0].data).toEqual([{ time: '2026-01-02', value: 101.5 }]);
  });
});
