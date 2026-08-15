import { describe, expect, it } from 'vitest';
import type { IndicatorDataPoint } from '../../../api/indicatorsApi';
import type { IndicatorInstanceV1 } from '../../../features/indicators/indicatorDomain';
import { formatBollingerStd, IndicatorRenderRegistry } from '../IndicatorRenderRegistry';

const instance = (definitionId: IndicatorInstanceV1['definitionId']): IndicatorInstanceV1 => ({
  id: `00000000-0000-4000-8000-00000000000${definitionId.length}`,
  definitionId,
  label: definitionId.toUpperCase(),
  params: definitionId === 'macd' ? { fast: 12, slow: 26, signal: 9 } : (definitionId === 'atr' || definitionId === 'rsi' ? { length: 14 } : { length: 20 }),
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
  it('formats Bollinger standard deviations deterministically', () => {
    expect(formatBollingerStd(2.0)).toBe('2.0');
    expect(formatBollingerStd(2)).toBe('2.0');
    expect(formatBollingerStd(3)).toBe('3.0');
    expect(formatBollingerStd(10)).toBe('10.0');
    expect(formatBollingerStd(2.25)).toBe('2.25');
    expect(formatBollingerStd(1.15)).toBe('1.15');
    expect(formatBollingerStd(2.5)).toBe('2.5');
    expect(formatBollingerStd(0.1)).toBe('0.1');
    expect(formatBollingerStd(NaN)).toBe('2.0');
    expect(formatBollingerStd(-1)).toBe('2.0');
  });

  it('renders semantic MACD line, signal, histogram and zero reference', () => {
    const rendered = IndicatorRenderRegistry.mapBackendData([{
      timestamp: '2026-01-02T00:00:00', MACD_12_26_9: 1.2, MACDs_12_26_9: 0.8, MACDh_12_26_9: 0.4,
    }], instance('macd'));

    expect(rendered.map(series => series.seriesKey)).toEqual(['macd', 'signal', 'histogram']);
    expect(rendered[2]).toMatchObject({ type: 'histogram', references: [{ value: 0, label: 'Zero' }] });
    expect(rendered.every(series => series.data[0].time === '2026-01-02')).toBe(true);
  });

  it('provides professional RSI and CCI scale/reference semantics', () => {
    const rsi = IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', RSI_14: 55 }], instance('rsi'))[0];
    const cci = IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', 'CCI_20_0.015': -12 }], instance('cci'))[0];
    expect(rsi.scale).toEqual({ minimum: 0, maximum: 100 });
    expect(rsi.references?.map(line => line.value)).toEqual([30, 50, 70]);
    expect(cci.references?.map(line => line.value)).toEqual([-100, 0, 100]);
  });

  it('renders SMA, ATR, and Volume SMA with correct labels and series types', () => {
    const sma = IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', SMA_20: 105.4 }], instance('sma'))[0];
    const atr = IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', ATRr_14: 2.3 }], instance('atr'))[0];
    const volumeSma = IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', VOLUME_SMA_20: 50000 }], instance('volume_sma'))[0];

    expect(sma).toMatchObject({ seriesKey: 'primary', name: 'SMA 20', data: [{ time: '2026-01-02', value: 105.4 }] });
    expect(atr).toMatchObject({ seriesKey: 'primary', name: 'ATR 14', data: [{ time: '2026-01-02', value: 2.3 }] });
    expect(volumeSma).toMatchObject({ seriesKey: 'primary', name: 'Volume SMA 20', type: 'line', data: [{ time: '2026-01-02', value: 50000 }] });
  });

  it('renders Bollinger Bands with upper, middle, and lower series', () => {
    const bbands = IndicatorRenderRegistry.mapBackendData([{
      timestamp: '2026-01-02', 'BBU_20_2.0_2.0': 110.0, 'BBM_20_2.0_2.0': 100.0, 'BBL_20_2.0_2.0': 90.0,
    }], instance('bbands'));

    expect(bbands.map(s => s.seriesKey)).toEqual(['upper', 'middle', 'lower']);
    expect(bbands[0]).toMatchObject({ name: 'Upper Band', data: [{ time: '2026-01-02', value: 110.0 }] });
    expect(bbands[1]).toMatchObject({ name: 'Middle Band', data: [{ time: '2026-01-02', value: 100.0 }] });
    expect(bbands[2]).toMatchObject({ name: 'Lower Band', data: [{ time: '2026-01-02', value: 90.0 }] });
  });

  it('renders non-default Bollinger Bands (std=2.25) with exact fractional column contract', () => {
    const bbandsInstance: IndicatorInstanceV1 = {
      ...instance('bbands'),
      params: { length: 20, std: 2.25 },
    };
    const bbands = IndicatorRenderRegistry.mapBackendData([{
      timestamp: '2026-01-02', 'BBU_20_2.25_2.25': 112.5, 'BBM_20_2.25_2.25': 100.0, 'BBL_20_2.25_2.25': 87.5,
    }], bbandsInstance);

    expect(bbands.map(s => s.seriesKey)).toEqual(['upper', 'middle', 'lower']);
    expect(bbands[0]).toMatchObject({ name: 'Upper Band', data: [{ time: '2026-01-02', value: 112.5 }] });
    expect(bbands[1]).toMatchObject({ name: 'Middle Band', data: [{ time: '2026-01-02', value: 100.0 }] });
    expect(bbands[2]).toMatchObject({ name: 'Lower Band', data: [{ time: '2026-01-02', value: 87.5 }] });
  });

  it('fails closed when non-default Bollinger Bands receive rounded, default, or mismatched columns', () => {
    const bbands225: IndicatorInstanceV1 = {
      ...instance('bbands'),
      params: { length: 20, std: 2.25 },
    };

    // Rounded to 1 decimal place (toFixed(1) defect) must fail closed
    const roundedCols = [{ timestamp: '2026-01-02', 'BBU_20_2.2_2.2': 112.5, 'BBM_20_2.2_2.2': 100.0, 'BBL_20_2.2_2.2': 87.5 }];
    expect(IndicatorRenderRegistry.mapBackendData(roundedCols, bbands225)).toEqual([]);

    // Default std columns (std=2.0) must fail closed when 2.25 was requested
    const defaultCols = [{ timestamp: '2026-01-02', 'BBU_20_2.0_2.0': 110.0, 'BBM_20_2.0_2.0': 100.0, 'BBL_20_2.0_2.0': 90.0 }];
    expect(IndicatorRenderRegistry.mapBackendData(defaultCols, bbands225)).toEqual([]);

    // Alias without lower/upper std suffix must fail closed
    const aliasCols = [{ timestamp: '2026-01-02', 'BBU_20_2.25': 112.5, 'BBM_20_2.25': 100.0, 'BBL_20_2.25': 87.5 }];
    expect(IndicatorRenderRegistry.mapBackendData(aliasCols, bbands225)).toEqual([]);
  });

  it('fails closed on unknown or unreleased definitions without falling back to EMA', () => {
    const unknownInstance = { ...instance('ema'), definitionId: 'ichimoku' as unknown as IndicatorInstanceV1['definitionId'] };
    const rendered = IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', UNKNOWN_20: 100 }], unknownInstance);
    expect(rendered).toEqual([]);
  });

  it.each(['ema', 'rsi', 'cci', 'macd', 'sma', 'atr', 'volume_sma'] as const)('does not turn %s backend warm-up nulls into zero', definitionId => {
    const len = (definitionId === 'rsi' || definitionId === 'atr') ? 14 : 20;
    const row1: Record<string, unknown> = { timestamp: '2026-01-01' };
    const row2: Record<string, unknown> = { timestamp: '2026-01-02' };

    if (definitionId === 'macd') {
      row1['MACD_12_26_9'] = null; row1['MACDs_12_26_9'] = null; row1['MACDh_12_26_9'] = null;
      row2['MACD_12_26_9'] = 101.5; row2['MACDs_12_26_9'] = 98.0; row2['MACDh_12_26_9'] = 3.5;
    } else if (definitionId === 'cci') {
      row1['CCI_20_0.015'] = null; row2['CCI_20_0.015'] = 101.5;
    } else if (definitionId === 'atr') {
      row1[`ATRr_${len}`] = null; row2[`ATRr_${len}`] = 101.5;
    } else {
      row1[`${definitionId.toUpperCase()}_${len}`] = null; row2[`${definitionId.toUpperCase()}_${len}`] = 101.5;
    }

    const rendered = IndicatorRenderRegistry.mapBackendData([row1, row2] as IndicatorDataPoint[], instance(definitionId));
    expect(rendered[0].data).toEqual([{ time: '2026-01-02', value: 101.5 }]);
  });

  it('ignores prepended or irrelevant response columns and selects exact semantic output', () => {
    const dataWithExtra = [{ timestamp: '2026-01-02', IRRELEVANT_COL: 999.9, SMA_20: 105.4 }];
    const sma = IndicatorRenderRegistry.mapBackendData(dataWithExtra, instance('sma'))[0];
    expect(sma).toMatchObject({ seriesKey: 'primary', name: 'SMA 20', data: [{ time: '2026-01-02', value: 105.4 }] });

    const atrData = [{ timestamp: '2026-01-02', UNWANTED: 12.3, ATRr_14: 2.3 }];
    const atr = IndicatorRenderRegistry.mapBackendData(atrData, instance('atr'))[0];
    expect(atr).toMatchObject({ seriesKey: 'primary', name: 'ATR 14', data: [{ time: '2026-01-02', value: 2.3 }] });

    const vmaData = [{ timestamp: '2026-01-02', EXTRA_HEADER: 88, VOLUME_SMA_20: 50000 }];
    const vma = IndicatorRenderRegistry.mapBackendData(vmaData, instance('volume_sma'))[0];
    expect(vma).toMatchObject({ seriesKey: 'primary', name: 'Volume SMA 20', data: [{ time: '2026-01-02', value: 50000 }] });
  });

  it('fails closed when response column parameters do not match requested instance parameters', () => {
    // Mismatched length
    expect(IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', SMA_10: 100 }], instance('sma'))).toEqual([]);
    expect(IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', EMA_50: 100 }], instance('ema'))).toEqual([]);
    expect(IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', RSI_7: 50 }], instance('rsi'))).toEqual([]);
    expect(IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', 'CCI_14_0.015': 50 }], instance('cci'))).toEqual([]);
    expect(IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', ATRr_20: 2.3 }], instance('atr'))).toEqual([]);
    expect(IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', VOLUME_SMA_50: 50000 }], instance('volume_sma'))).toEqual([]);

    // Mismatched Bollinger std / length
    const wrongBbands = [{ timestamp: '2026-01-02', 'BBU_20_1.5_1.5': 110, 'BBM_20_1.5_1.5': 100, 'BBL_20_1.5_1.5': 90 }];
    expect(IndicatorRenderRegistry.mapBackendData(wrongBbands, instance('bbands'))).toEqual([]);

    // Mismatched MACD parameters
    const wrongMacd = [{ timestamp: '2026-01-02', MACD_10_20_5: 1.2, MACDs_10_20_5: 0.8, MACDh_10_20_5: 0.4 }];
    expect(IndicatorRenderRegistry.mapBackendData(wrongMacd, instance('macd'))).toEqual([]);
  });

  it('rejects CCI/ATR aliases and alternate Bollinger name spellings', () => {
    // CCI alias without _0.015 suffix must fail closed
    expect(IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', CCI_20: 50 }], instance('cci'))).toEqual([]);

    // ATR alias without 'r' prefix must fail closed
    expect(IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', ATR_14: 2.3 }], instance('atr'))).toEqual([]);

    // Bollinger alternate spelling BBU_20_2_2 without float decimals must fail closed
    const altBbands = [{ timestamp: '2026-01-02', BBU_20_2_2: 110, BBM_20_2_2: 100, BBL_20_2_2: 90 }];
    expect(IndicatorRenderRegistry.mapBackendData(altBbands, instance('bbands'))).toEqual([]);
  });

  it('enforces all-or-nothing rendering for MACD and Bollinger Bands', () => {
    // Partial MACD (missing histogram) must fail closed as a whole
    const partialMacd = [{ timestamp: '2026-01-02', MACD_12_26_9: 1.2, MACDs_12_26_9: 0.8 }];
    expect(IndicatorRenderRegistry.mapBackendData(partialMacd, instance('macd'))).toEqual([]);

    // Partial Bollinger (missing middle band) must fail closed as a whole
    const partialBbands = [{ timestamp: '2026-01-02', 'BBU_20_2.0_2.0': 110, 'BBL_20_2.0_2.0': 90 }];
    expect(IndicatorRenderRegistry.mapBackendData(partialBbands, instance('bbands'))).toEqual([]);
  });
});
