import { describe, expect, it } from 'vitest';
import { IndicatorRenderRegistry } from '../IndicatorRenderRegistry';

describe('IndicatorRenderRegistry', () => {
  it('renders MACD line, signal, and histogram from backend columns', () => {
    const rendered = IndicatorRenderRegistry.mapBackendData([{
      timestamp: '2026-01-02T00:00:00',
      MACD_12_26_9: 1.2,
      MACDs_12_26_9: 0.8,
      MACDh_12_26_9: 0.4,
    }]);

    expect(rendered).toHaveLength(3);
    expect(rendered.find(series => series.name.startsWith('MACDh'))?.type).toBe('histogram');
    expect(rendered.every(series => series.data[0].time === '2026-01-02')).toBe(true);
  });

  it('assigns overlays to price and oscillators to dedicated panes', () => {
    expect(IndicatorRenderRegistry.paneFor('ema', 'main')).toBe('price');
    expect(IndicatorRenderRegistry.paneFor('rsi', 'oscillator')).toBe('indicator:rsi');
    expect(IndicatorRenderRegistry.paneFor('cci', 'oscillator')).toBe('indicator:cci');
    expect(IndicatorRenderRegistry.paneFor('macd', 'oscillator')).toBe('indicator:macd');
  });

  it('does not turn backend warm-up nulls into zero values', () => {
    const rendered = IndicatorRenderRegistry.mapBackendData([
      { timestamp: '2026-01-01', EMA_20: null },
      { timestamp: '2026-01-02', EMA_20: 101.5 },
    ]);

    expect(rendered[0].data).toEqual([{ time: '2026-01-02', value: 101.5 }]);
  });
});
