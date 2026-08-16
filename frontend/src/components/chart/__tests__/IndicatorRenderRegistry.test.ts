import { describe, expect, it } from 'vitest';
import type { IndicatorDataPoint } from '../../../api/indicatorsApi';
import type { IndicatorInstanceV1 } from '../../../features/indicators/indicatorDomain';
import { formatBollingerStd, formatFloatParam, IndicatorRenderRegistry } from '../IndicatorRenderRegistry';

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

  it('renders MFI with scale 0..100, reference levels 20/80, and exact column contract', () => {
    const mfiInstance = { ...instance('mfi'), params: { length: 14 } };
    const rendered = IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', MFI_14: 68.5 }], mfiInstance);
    expect(rendered).toHaveLength(1);
    expect(rendered[0]).toMatchObject({
      seriesKey: 'primary',
      name: 'MFI 14',
      scale: { minimum: 0, maximum: 100 },
      references: [{ value: 20, label: 'MFI 20' }, { value: 80, label: 'MFI 80' }],
      data: [{ time: '2026-01-02', value: 68.5 }],
    });

    // Mismatched length fails closed
    expect(IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', MFI_20: 68.5 }], mfiInstance)).toEqual([]);
  });

  it('renders Stochastic with %K and %D series, scale 0..100, reference levels 20/80, and all-or-nothing integrity', () => {
    const stochInstance = { ...instance('stoch'), params: { k: 14, d: 3, smooth_k: 3 } };
    const validData = [{ timestamp: '2026-01-02', STOCHk_14_3_3: 75.2, STOCHd_14_3_3: 70.1 }];
    const rendered = IndicatorRenderRegistry.mapBackendData(validData, stochInstance);
    expect(rendered.map(s => s.seriesKey)).toEqual(['k', 'd']);
    expect(rendered[0]).toMatchObject({
      name: '%K',
      scale: { minimum: 0, maximum: 100 },
      references: [{ value: 20, label: 'Stoch 20' }, { value: 80, label: 'Stoch 80' }],
      data: [{ time: '2026-01-02', value: 75.2 }],
    });
    expect(rendered[1]).toMatchObject({
      name: '%D',
      data: [{ time: '2026-01-02', value: 70.1 }],
    });

    // Partial stochastic (missing %D) must fail closed
    expect(IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', STOCHk_14_3_3: 75.2 }], stochInstance)).toEqual([]);

    // Mismatched parameters fail closed
    expect(IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', STOCHk_20_3_3: 75.2, STOCHd_20_3_3: 70.1 }], stochInstance)).toEqual([]);
  });

  it('renders ADX with ADX, +DI, -DI series, reference levels 20/25, and all-or-nothing integrity', () => {
    const adxInstance = { ...instance('adx'), params: { length: 14 } };
    const validData = [{ timestamp: '2026-01-02', ADX_14: 32.5, DMP_14: 28.1, DMN_14: 15.4 }];
    const rendered = IndicatorRenderRegistry.mapBackendData(validData, adxInstance);
    expect(rendered.map(s => s.seriesKey)).toEqual(['adx', 'dmp', 'dmn']);
    expect(rendered[0]).toMatchObject({
      name: 'ADX',
      references: [{ value: 20, label: 'ADX 20' }, { value: 25, label: 'ADX 25' }],
      data: [{ time: '2026-01-02', value: 32.5 }],
    });
    expect(rendered[1]).toMatchObject({ name: '+DI', data: [{ time: '2026-01-02', value: 28.1 }] });
    expect(rendered[2]).toMatchObject({ name: '-DI', data: [{ time: '2026-01-02', value: 15.4 }] });

    // Partial ADX (missing -DI) must fail closed
    expect(IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', ADX_14: 32.5, DMP_14: 28.1 }], adxInstance)).toEqual([]);

    // Mismatched length fails closed
    expect(IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', ADX_20: 32.5, DMP_20: 28.1, DMN_20: 15.4 }], adxInstance)).toEqual([]);
  });

  it('renders Relative Strength vs VNINDEX with reference level 100 and exact column contract', () => {
    const rsInstance = { ...instance('relative_strength'), params: { length: 20, benchmark: 'VNINDEX' } };
    const rendered = IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', RS_VNINDEX_20: 104.8 }], rsInstance);
    expect(rendered).toHaveLength(1);
    expect(rendered[0]).toMatchObject({
      seriesKey: 'primary',
      name: 'RS (VNINDEX) 20',
      references: [{ value: 100, label: '100' }],
      data: [{ time: '2026-01-02', value: 104.8 }],
    });

    // Mismatched parameters fail closed
    expect(IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', RS_VNINDEX_50: 104.8 }], rsInstance)).toEqual([]);
    expect(IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', RS_VN30_20: 104.8 }], rsInstance)).toEqual([]);
  });

  it('formats float parameters deterministically', () => {
    expect(formatFloatParam(2.0, '2.0')).toBe('2.0');
    expect(formatFloatParam(2, '2.0')).toBe('2.0');
    expect(formatFloatParam(1.5, '2.0')).toBe('1.5');
    expect(formatFloatParam(0.02, '0.02')).toBe('0.02');
    expect(formatFloatParam(0.2, '0.2')).toBe('0.2');
    expect(formatFloatParam(3.0, '3.0')).toBe('3.0');
    expect(formatFloatParam(NaN, '2.0')).toBe('2.0');
  });

  it('renders Keltner Channels with upper, middle, and lower channel series and exact scalar contract', () => {
    const kcInstance: IndicatorInstanceV1 = { ...instance('kc'), params: { length: 20, scalar: 2.0 } };
    const validData = [{ timestamp: '2026-01-02', 'KCUe_20_2.0': 115.0, 'KCBe_20_2.0': 105.0, 'KCLe_20_2.0': 95.0 }];
    const rendered = IndicatorRenderRegistry.mapBackendData(validData, kcInstance);
    expect(rendered.map(s => s.seriesKey)).toEqual(['upper', 'middle', 'lower']);
    expect(rendered[0]).toMatchObject({ name: 'Upper Channel', data: [{ time: '2026-01-02', value: 115.0 }] });
    expect(rendered[1]).toMatchObject({ name: 'Middle Channel', data: [{ time: '2026-01-02', value: 105.0 }] });
    expect(rendered[2]).toMatchObject({ name: 'Lower Channel', data: [{ time: '2026-01-02', value: 95.0 }] });

    // Non-default scalar 1.5
    const kc15Instance: IndicatorInstanceV1 = { ...instance('kc'), params: { length: 20, scalar: 1.5 } };
    const validData15 = [{ timestamp: '2026-01-02', 'KCUe_20_1.5': 112.5, 'KCBe_20_1.5': 105.0, 'KCLe_20_1.5': 97.5 }];
    const rendered15 = IndicatorRenderRegistry.mapBackendData(validData15, kc15Instance);
    expect(rendered15.map(s => s.seriesKey)).toEqual(['upper', 'middle', 'lower']);
    expect(rendered15[0]).toMatchObject({ name: 'Upper Channel', data: [{ time: '2026-01-02', value: 112.5 }] });

    // Partial Keltner Channels (missing lower) must fail closed
    expect(IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', 'KCUe_20_2.0': 115.0, 'KCBe_20_2.0': 105.0 }], kcInstance)).toEqual([]);

    // Mismatched length or scalar fail closed
    expect(IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', 'KCUe_14_2.0': 115.0, 'KCBe_14_2.0': 105.0, 'KCLe_14_2.0': 95.0 }], kcInstance)).toEqual([]);
    expect(IndicatorRenderRegistry.mapBackendData(validData15, kcInstance)).toEqual([]);
  });

  it('renders Parabolic SAR with continuous stop points across trend reversals and exact parameter contract', () => {
    const psarInstance: IndicatorInstanceV1 = { ...instance('psar'), params: { af0: 0.02, af: 0.02, max_af: 0.2 } };
    const multiRowData = [
      { timestamp: '2026-01-01', 'PSARl_0.02_0.2': 100.0, 'PSARs_0.02_0.2': null },
      { timestamp: '2026-01-02', 'PSARl_0.02_0.2': 101.5, 'PSARs_0.02_0.2': null },
      { timestamp: '2026-01-03', 'PSARl_0.02_0.2': null, 'PSARs_0.02_0.2': 108.0 },
    ];
    const rendered = IndicatorRenderRegistry.mapBackendData(multiRowData, psarInstance);
    expect(rendered).toHaveLength(1);
    expect(rendered[0]).toMatchObject({
      seriesKey: 'sar',
      name: 'PSAR 0.02/0.2',
      data: [
        { time: '2026-01-01', value: 100.0 },
        { time: '2026-01-02', value: 101.5 },
        { time: '2026-01-03', value: 108.0 },
      ],
    });

    // Non-default parameters (0.01 / 0.1)
    const psarNonDefault: IndicatorInstanceV1 = { ...instance('psar'), params: { af0: 0.01, af: 0.01, max_af: 0.1 } };
    const nonDefaultData = [{ timestamp: '2026-01-01', 'PSARl_0.01_0.1': 99.0, 'PSARs_0.01_0.1': null }];
    const renderedNonDefault = IndicatorRenderRegistry.mapBackendData(nonDefaultData, psarNonDefault);
    expect(renderedNonDefault[0]).toMatchObject({
      seriesKey: 'sar',
      name: 'PSAR 0.01/0.1',
      data: [{ time: '2026-01-01', value: 99.0 }],
    });

    // Partial PSAR (missing short stop column in response) must fail closed
    expect(IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-01', 'PSARl_0.02_0.2': 100.0 }], psarInstance)).toEqual([]);

    // Mismatched parameters fail closed
    expect(IndicatorRenderRegistry.mapBackendData(nonDefaultData, psarInstance)).toEqual([]);
  });

  it('renders SuperTrend with trend line and direction contract', () => {
    const stInstance: IndicatorInstanceV1 = { ...instance('supertrend'), params: { length: 7, multiplier: 3.0 } };
    const validData = [{ timestamp: '2026-01-02', 'SUPERT_7_3.0': 102.4, 'SUPERTd_7_3.0': 1.0 }];
    const rendered = IndicatorRenderRegistry.mapBackendData(validData, stInstance);
    expect(rendered).toHaveLength(1);
    expect(rendered[0]).toMatchObject({
      seriesKey: 'supertrend',
      name: 'SuperTrend 7/3.0',
      data: [{ time: '2026-01-02', value: 102.4 }],
    });

    // Non-default parameters (length: 10, multiplier: 2.0)
    const stNonDefault: IndicatorInstanceV1 = { ...instance('supertrend'), params: { length: 10, multiplier: 2.0 } };
    const nonDefaultData = [{ timestamp: '2026-01-02', 'SUPERT_10_2.0': 103.1, 'SUPERTd_10_2.0': -1.0 }];
    const renderedNonDefault = IndicatorRenderRegistry.mapBackendData(nonDefaultData, stNonDefault);
    expect(renderedNonDefault[0]).toMatchObject({
      seriesKey: 'supertrend',
      name: 'SuperTrend 10/2.0',
      data: [{ time: '2026-01-02', value: 103.1 }],
    });

    // Partial SuperTrend (missing SUPERTd direction) must fail closed
    expect(IndicatorRenderRegistry.mapBackendData([{ timestamp: '2026-01-02', 'SUPERT_7_3.0': 102.4 }], stInstance)).toEqual([]);

    // Mismatched parameters fail closed
    expect(IndicatorRenderRegistry.mapBackendData(nonDefaultData, stInstance)).toEqual([]);
  });
});
