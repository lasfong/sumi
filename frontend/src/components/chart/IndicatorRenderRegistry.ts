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

export const formatBollingerStd = (std: number): string => {
  if (!Number.isFinite(std) || std <= 0) {
    return '2.0';
  }
  return Number.isInteger(std) ? std.toFixed(1) : String(std);
};

export class IndicatorRenderRegistry {
  static referencesFor(definitionId: IndicatorInstanceV1['definitionId']): Array<{ value: number; label: string }> {
    if (definitionId === 'rsi') return [30, 50, 70].map(value => ({ value, label: `RSI ${value}` }));
    if (definitionId === 'cci') return [-100, 0, 100].map(value => ({ value, label: `CCI ${value}` }));
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
    // Single-series released definitions: find exact single pinned output contract
    const findExactColumn = (defId: string): string | undefined => {
      const defaultLen = (defId === 'rsi' || defId === 'atr') ? 14 : 20;
      const length = getIntParam(instance, 'length', defaultLen);

      if (defId === 'sma') return available.find(c => c === `SMA_${length}`);
      if (defId === 'ema') return available.find(c => c === `EMA_${length}`);
      if (defId === 'rsi') return available.find(c => c === `RSI_${length}`);
      if (defId === 'cci') return available.find(c => c === `CCI_${length}_0.015`);
      if (defId === 'atr') return available.find(c => c === `ATRr_${length}`);
      if (defId === 'volume_sma') return available.find(c => c === `VOLUME_SMA_${length}`);
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
    return [];
  }

  static mapVolume(data: VolumeData[], instance: IndicatorInstanceV1): IndicatorSeriesData[] {
    return [{ seriesKey: 'raw-volume', name: 'Volume', type: 'histogram', data, color: instance.styles.volume?.color ?? '#58A6FF' }];
  }

  static currentValues(series: IndicatorSeriesData[]): Record<string, number | null> {
    return Object.fromEntries(series.map(item => [item.seriesKey, item.data.length ? Number(item.data[item.data.length - 1].value) : null]));
  }
}
