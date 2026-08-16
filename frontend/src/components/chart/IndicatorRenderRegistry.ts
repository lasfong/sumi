import type { IndicatorDataPoint } from '../../api/indicatorsApi';
import type { Time } from 'lightweight-charts';
import { SUPPORTED_INDICATORS, type IndicatorInstanceV1 } from '../../features/indicators/indicatorDomain';
import { toDateKey } from '../../utils/date';
import type { IndicatorSeriesData, VolumeData } from './workspaceTypes';

const finitePoints = (data: IndicatorDataPoint[], column: string) => data.flatMap(row => {
  const raw = row[column]; if (raw === null || raw === '') return [];
  const value = typeof raw === 'number' ? raw : Number(raw);
  return Number.isFinite(value) ? [{ time: (toDateKey(row.timestamp) || row.timestamp) as Time, value }] : [];
});

const columns = (data: IndicatorDataPoint[]) => [...new Set(data.flatMap(row => Object.keys(row).filter(key => key !== 'timestamp')))];
const primaryColor = (instance: IndicatorInstanceV1) => instance.styles.primary?.color ?? '#58A6FF';
const paramsLabel = (instance: IndicatorInstanceV1) => Object.values(instance.params).filter(value => value !== 0).join('/');

const getIntParam = (instance: IndicatorInstanceV1, name: string, defaultValue: number): number => {
  const val = instance.params[name];
  const num = typeof val === 'number' ? val : Number(val);
  return Number.isFinite(num) && num > 0 ? Math.round(num) : defaultValue;
};

const getFloatParam = (instance: IndicatorInstanceV1, name: string, defaultValue: number): number => {
  const val = instance.params[name];
  const num = typeof val === 'number' ? val : Number(val);
  return Number.isFinite(num) && num > 0 ? num : defaultValue;
};

export const formatFloatParam = (val: number, defaultVal = '2.0'): string => {
  if (!Number.isFinite(val) || val <= 0) {
    return defaultVal;
  }
  return Number.isInteger(val) ? val.toFixed(1) : String(val);
};

export const formatBollingerStd = (std: number): string => formatFloatParam(std, '2.0');

export class IndicatorRenderRegistry {
  static referencesFor(definitionId: IndicatorInstanceV1['definitionId']): Array<{ value: number; label: string }> {
    if (definitionId === 'rsi') return [30, 50, 70].map(value => ({ value, label: `RSI ${value}` }));
    if (definitionId === 'cci') return [-100, 0, 100].map(value => ({ value, label: `CCI ${value}` }));
    if (definitionId === 'mfi') return [20, 80].map(value => ({ value, label: `MFI ${value}` }));
    if (definitionId === 'stoch') return [20, 80].map(value => ({ value, label: `Stoch ${value}` }));
    if (definitionId === 'adx') return [20, 25].map(value => ({ value, label: `ADX ${value}` }));
    if (definitionId === 'relative_strength') return [{ value: 100, label: '100' }];
    if (definitionId === 'macd') return [{ value: 0, label: 'Zero' }];
    return [];
  }

  static mapBackendData(data: IndicatorDataPoint[], instance: IndicatorInstanceV1): IndicatorSeriesData[] {
    if (!(SUPPORTED_INDICATORS as readonly string[]).includes(instance.definitionId)) {
      return [];
    }
    const available = columns(data);
    if (instance.definitionId === 'macd') {
      const fast = getIntParam(instance, 'fast', 12);
      const slow = getIntParam(instance, 'slow', 26);
      const signalParam = getIntParam(instance, 'signal', 9);
      const expectedLine = `MACD_${fast}_${slow}_${signalParam}`;
      const expectedSignal = `MACDs_${fast}_${slow}_${signalParam}`;
      const expectedHist = `MACDh_${fast}_${slow}_${signalParam}`;

      const line = available.find(c => c === expectedLine);
      const signal = available.find(c => c === expectedSignal);
      const histogram = available.find(c => c === expectedHist);

      // All-or-nothing: line, signal, and histogram must all be present
      if (!line || !signal || !histogram) {
        return [];
      }

      const linePoints = finitePoints(data, line);
      const signalPoints = finitePoints(data, signal);
      const histPoints = finitePoints(data, histogram);

      if (!linePoints.length || !signalPoints.length || !histPoints.length) {
        return [];
      }

      return [
        { seriesKey: 'macd', name: 'MACD', data: linePoints, color: instance.styles.macd?.color },
        { seriesKey: 'signal', name: 'Signal', data: signalPoints, color: instance.styles.signal?.color },
        { seriesKey: 'histogram', name: 'Histogram', type: 'histogram', data: histPoints, color: instance.styles.histogram?.color, references: this.referencesFor('macd') },
      ];
    }
    if (instance.definitionId === 'bbands') {
      const length = getIntParam(instance, 'length', 20);
      const std = getFloatParam(instance, 'std', 2.0);
      const stdStr = formatBollingerStd(std);

      const expectedUpper = `BBU_${length}_${stdStr}_${stdStr}`;
      const expectedMiddle = `BBM_${length}_${stdStr}_${stdStr}`;
      const expectedLower = `BBL_${length}_${stdStr}_${stdStr}`;

      const upper = available.find(c => c === expectedUpper);
      const middle = available.find(c => c === expectedMiddle);
      const lower = available.find(c => c === expectedLower);

      // All-or-nothing: upper, middle, and lower must all be present
      if (!upper || !middle || !lower) {
        return [];
      }

      const upperPoints = finitePoints(data, upper);
      const middlePoints = finitePoints(data, middle);
      const lowerPoints = finitePoints(data, lower);

      if (!upperPoints.length || !middlePoints.length || !lowerPoints.length) {
        return [];
      }

      return [
        { seriesKey: 'upper', name: 'Upper Band', data: upperPoints, color: instance.styles.upper?.color ?? '#00E5FF' },
        { seriesKey: 'middle', name: 'Middle Band', data: middlePoints, color: instance.styles.middle?.color ?? '#FFD166' },
        { seriesKey: 'lower', name: 'Lower Band', data: lowerPoints, color: instance.styles.lower?.color ?? '#00E5FF' },
      ];
    }
    if (instance.definitionId === 'kc') {
      const length = getIntParam(instance, 'length', 20);
      const scalar = getFloatParam(instance, 'scalar', 2.0);
      const scalarStr = formatFloatParam(scalar, '2.0');

      const expectedUpper = `KCUe_${length}_${scalarStr}`;
      const expectedMiddle = `KCBe_${length}_${scalarStr}`;
      const expectedLower = `KCLe_${length}_${scalarStr}`;

      const upper = available.find(c => c === expectedUpper);
      const middle = available.find(c => c === expectedMiddle);
      const lower = available.find(c => c === expectedLower);

      // All-or-nothing: upper, middle, and lower must all be present
      if (!upper || !middle || !lower) {
        return [];
      }

      const upperPoints = finitePoints(data, upper);
      const middlePoints = finitePoints(data, middle);
      const lowerPoints = finitePoints(data, lower);

      if (!upperPoints.length || !middlePoints.length || !lowerPoints.length) {
        return [];
      }

      return [
        { seriesKey: 'upper', name: 'Upper Channel', data: upperPoints, color: instance.styles.upper?.color ?? '#00E5FF' },
        { seriesKey: 'middle', name: 'Middle Channel', data: middlePoints, color: instance.styles.middle?.color ?? '#FFD166' },
        { seriesKey: 'lower', name: 'Lower Channel', data: lowerPoints, color: instance.styles.lower?.color ?? '#00E5FF' },
      ];
    }
    if (instance.definitionId === 'psar') {
      const af0 = getFloatParam(instance, 'af0', 0.02);
      const maxAf = getFloatParam(instance, 'max_af', 0.2);
      const af0Str = formatFloatParam(af0, '0.02');
      const maxAfStr = formatFloatParam(maxAf, '0.2');

      const expectedLong = `PSARl_${af0Str}_${maxAfStr}`;
      const expectedShort = `PSARs_${af0Str}_${maxAfStr}`;

      const longCol = available.find(c => c === expectedLong);
      const shortCol = available.find(c => c === expectedShort);

      // All-or-nothing: both long and short stop columns must be present in response
      if (!longCol || !shortCol) {
        return [];
      }

      const sarPoints = data.flatMap(row => {
        const l = row[longCol];
        const s = row[shortCol];
        const raw = (l !== null && l !== '' && l !== undefined) ? l : s;
        if (raw === null || raw === '' || raw === undefined) return [];
        const value = typeof raw === 'number' ? raw : Number(raw);
        return Number.isFinite(value) ? [{ time: (toDateKey(row.timestamp) || row.timestamp) as Time, value }] : [];
      });

      if (!sarPoints.length) {
        return [];
      }

      return [
        { seriesKey: 'sar', name: `PSAR ${af0Str}/${maxAfStr}`, data: sarPoints, color: instance.styles.sar?.color ?? instance.styles.primary?.color ?? '#E040FB' },
      ];
    }
    if (instance.definitionId === 'supertrend') {
      const length = getIntParam(instance, 'length', 7);
      const multiplier = getFloatParam(instance, 'multiplier', 3.0);
      const multStr = formatFloatParam(multiplier, '3.0');

      const expectedSt = `SUPERT_${length}_${multStr}`;
      const expectedDir = `SUPERTd_${length}_${multStr}`;

      const stCol = available.find(c => c === expectedSt);
      const dirCol = available.find(c => c === expectedDir);

      // All-or-nothing: both SuperTrend line and direction columns must be present
      if (!stCol || !dirCol) {
        return [];
      }

      const stPoints = finitePoints(data, stCol);

      if (!stPoints.length) {
        return [];
      }

      return [
        { seriesKey: 'supertrend', name: `SuperTrend ${length}/${multStr}`, data: stPoints, color: instance.styles.supertrend?.color ?? instance.styles.bull?.color ?? '#26A69A' },
      ];
    }
    if (instance.definitionId === 'stoch') {
      const k = getIntParam(instance, 'k', 14);
      const d = getIntParam(instance, 'd', 3);
      const smoothK = getIntParam(instance, 'smooth_k', 3);

      const expectedK = `STOCHk_${k}_${d}_${smoothK}`;
      const expectedD = `STOCHd_${k}_${d}_${smoothK}`;

      const kCol = available.find(c => c === expectedK);
      const dCol = available.find(c => c === expectedD);

      // All-or-nothing: both %K and %D must be present
      if (!kCol || !dCol) {
        return [];
      }

      const kPoints = finitePoints(data, kCol);
      const dPoints = finitePoints(data, dCol);

      if (!kPoints.length || !dPoints.length) {
        return [];
      }

      return [
        { seriesKey: 'k', name: '%K', data: kPoints, color: instance.styles.k?.color ?? '#58A6FF', scale: { minimum: 0, maximum: 100 }, references: this.referencesFor('stoch') },
        { seriesKey: 'd', name: '%D', data: dPoints, color: instance.styles.d?.color ?? '#FF8A00' },
      ];
    }
    if (instance.definitionId === 'adx') {
      const length = getIntParam(instance, 'length', 14);

      const expectedAdx = `ADX_${length}`;
      const expectedDmp = `DMP_${length}`;
      const expectedDmn = `DMN_${length}`;

      const adxCol = available.find(c => c === expectedAdx);
      const dmpCol = available.find(c => c === expectedDmp);
      const dmnCol = available.find(c => c === expectedDmn);

      // All-or-nothing: ADX, +DI, and -DI must all be present
      if (!adxCol || !dmpCol || !dmnCol) {
        return [];
      }

      const adxPoints = finitePoints(data, adxCol);
      const dmpPoints = finitePoints(data, dmpCol);
      const dmnPoints = finitePoints(data, dmnCol);

      if (!adxPoints.length || !dmpPoints.length || !dmnPoints.length) {
        return [];
      }

      return [
        { seriesKey: 'adx', name: 'ADX', data: adxPoints, color: instance.styles.adx?.color ?? '#FFD166', references: this.referencesFor('adx') },
        { seriesKey: 'dmp', name: '+DI', data: dmpPoints, color: instance.styles.dmp?.color ?? '#26A69A' },
        { seriesKey: 'dmn', name: '-DI', data: dmnPoints, color: instance.styles.dmn?.color ?? '#EF5350' },
      ];
    }
    if (instance.definitionId === 'ichimoku') {
      const tenkan = getIntParam(instance, 'tenkan', 9);
      const kijun = getIntParam(instance, 'kijun', 26);

      const expectedTenkan = `ITS_${tenkan}`;
      const expectedKijun = `IKS_${kijun}`;
      const expectedSpanA = `ISA_${tenkan}`;
      const expectedSpanB = `ISB_${kijun}`;
      const expectedChikou = `ICS_${kijun}`;

      const tenkanCol = available.find(c => c === expectedTenkan);
      const kijunCol = available.find(c => c === expectedKijun);
      const spanACol = available.find(c => c === expectedSpanA);
      const spanBCol = available.find(c => c === expectedSpanB);
      const chikouCol = available.find(c => c === expectedChikou);

      // All-or-nothing: tenkan, kijun, spanA, spanB, chikou columns must all be present
      if (!tenkanCol || !kijunCol || !spanACol || !spanBCol || !chikouCol) {
        return [];
      }

      const tenkanPoints = finitePoints(data, tenkanCol);
      const kijunPoints = finitePoints(data, kijunCol);
      const spanAPoints = finitePoints(data, spanACol);
      const spanBPoints = finitePoints(data, spanBCol);
      const chikouPoints = finitePoints(data, chikouCol);

      if (!tenkanPoints.length && !kijunPoints.length && !spanAPoints.length && !spanBPoints.length && !chikouPoints.length) {
        return [];
      }

      return [
        { seriesKey: 'tenkan', name: `Tenkan (${tenkan})`, data: tenkanPoints, color: instance.styles.tenkan?.color ?? '#26A69A' },
        { seriesKey: 'kijun', name: `Kijun (${kijun})`, data: kijunPoints, color: instance.styles.kijun?.color ?? '#EF5350' },
        { seriesKey: 'spanA', name: 'Span A', data: spanAPoints, color: instance.styles.spanA?.color ?? '#00E5FF' },
        { seriesKey: 'spanB', name: 'Span B', data: spanBPoints, color: instance.styles.spanB?.color ?? '#FF8A00' },
        { seriesKey: 'chikou', name: `Chikou (${kijun})`, data: chikouPoints, color: instance.styles.chikou?.color ?? '#E040FB' },
      ];
    }
    // Single-series released definitions: find exact single pinned output contract
    const findExactColumn = (defId: string): string | undefined => {
      const defaultLen = (defId === 'rsi' || defId === 'atr' || defId === 'mfi') ? 14 : 20;
      const length = getIntParam(instance, 'length', defaultLen);

      if (defId === 'sma') return available.find(c => c === `SMA_${length}`);
      if (defId === 'ema') return available.find(c => c === `EMA_${length}`);
      if (defId === 'rsi') return available.find(c => c === `RSI_${length}`);
      if (defId === 'cci') return available.find(c => c === `CCI_${length}_0.015`);
      if (defId === 'atr') return available.find(c => c === `ATRr_${length}`);
      if (defId === 'volume_sma') return available.find(c => c === `VOLUME_SMA_${length}`);
      if (defId === 'mfi') return available.find(c => c === `MFI_${length}`);
      if (defId === 'relative_strength') {
        const benchmark = String(instance.params.benchmark ?? 'VNINDEX').toUpperCase();
        return available.find(c => c === `RS_${benchmark}_${length}`);
      }
      return undefined;
    };

    const column = findExactColumn(instance.definitionId);
    if (!column) return [];

    const points = finitePoints(data, column);
    if (!points.length) return [];
    const length = paramsLabel(instance);
    if (instance.definitionId === 'rsi') return [{
      seriesKey: 'primary', name: `RSI ${length}`, data: points, color: primaryColor(instance),
      scale: { minimum: 0, maximum: 100 }, references: this.referencesFor('rsi'),
    }];
    if (instance.definitionId === 'cci') return [{
      seriesKey: 'primary', name: `CCI ${length}`, data: points, color: primaryColor(instance),
      references: this.referencesFor('cci'),
    }];
    if (instance.definitionId === 'ema') return [{
      seriesKey: 'primary', name: `EMA ${length}`, data: points, color: primaryColor(instance),
    }];
    if (instance.definitionId === 'sma') return [{
      seriesKey: 'primary', name: `SMA ${length}`, data: points, color: instance.styles.primary?.color ?? '#FFD166',
    }];
    if (instance.definitionId === 'atr') return [{
      seriesKey: 'primary', name: `ATR ${length}`, data: points, color: instance.styles.primary?.color ?? '#E040FB',
    }];
    if (instance.definitionId === 'volume_sma') return [{
      seriesKey: 'primary', name: `Volume SMA ${length}`, type: 'line', data: points, color: instance.styles.primary?.color ?? '#FF8A00',
    }];
    if (instance.definitionId === 'mfi') return [{
      seriesKey: 'primary', name: `MFI ${length}`, data: points, color: instance.styles.primary?.color ?? '#26A69A',
      scale: { minimum: 0, maximum: 100 }, references: this.referencesFor('mfi'),
    }];
    if (instance.definitionId === 'relative_strength') {
      const benchmark = String(instance.params.benchmark ?? 'VNINDEX').toUpperCase();
      const lengthVal = getIntParam(instance, 'length', 20);
      return [{
        seriesKey: 'primary', name: `RS (${benchmark}) ${lengthVal}`, data: points, color: instance.styles.primary?.color ?? '#00E5FF',
        references: this.referencesFor('relative_strength'),
      }];
    }
    return [];
  }

  static mapVolume(data: VolumeData[], instance: IndicatorInstanceV1): IndicatorSeriesData[] {
    return [{ seriesKey: 'raw-volume', name: 'Volume', type: 'histogram', data, color: instance.styles.volume?.color ?? '#58A6FF' }];
  }

  static currentValues(series: IndicatorSeriesData[]): Record<string, number | null> {
    return Object.fromEntries(series.map(item => [item.seriesKey, item.data.length ? Number(item.data[item.data.length - 1].value) : null]));
  }
}
