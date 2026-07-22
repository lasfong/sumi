import type { IndicatorDataPoint } from '../../api/indicatorsApi';
import type { Time } from 'lightweight-charts';
import type { IndicatorInstanceV1 } from '../../features/indicators/indicatorDomain';
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

export class IndicatorRenderRegistry {
  static referencesFor(definitionId: IndicatorInstanceV1['definitionId']): Array<{ value: number; label: string }> {
    if (definitionId === 'rsi') return [30, 50, 70].map(value => ({ value, label: `RSI ${value}` }));
    if (definitionId === 'cci') return [-100, 0, 100].map(value => ({ value, label: `CCI ${value}` }));
    if (definitionId === 'macd') return [{ value: 0, label: 'Zero' }];
    return [];
  }

  static mapBackendData(data: IndicatorDataPoint[], instance: IndicatorInstanceV1): IndicatorSeriesData[] {
    const available = columns(data);
    if (instance.definitionId === 'macd') {
      const pick = (test: (name: string) => boolean) => available.find(test);
      const histogram = pick(name => name.toLowerCase().startsWith('macdh'));
      const signal = pick(name => name.toLowerCase().startsWith('macds'));
      const line = pick(name => name.toLowerCase().startsWith('macd') && name !== histogram && name !== signal);
      const series: IndicatorSeriesData[] = [];
      if (line) series.push({ seriesKey: 'macd', name: 'MACD', data: finitePoints(data, line), color: instance.styles.macd?.color });
      if (signal) series.push({ seriesKey: 'signal', name: 'Signal', data: finitePoints(data, signal), color: instance.styles.signal?.color });
      if (histogram) series.push({
        seriesKey: 'histogram', name: 'Histogram', type: 'histogram', data: finitePoints(data, histogram),
        color: instance.styles.histogram?.color, references: this.referencesFor('macd'),
      });
      return series.filter(item => item.data.length > 0);
    }
    const column = available[0]; if (!column) return [];
    const points = finitePoints(data, column); if (!points.length) return [];
    const length = paramsLabel(instance);
    if (instance.definitionId === 'rsi') return [{
      seriesKey: 'primary', name: `RSI ${length}`, data: points, color: primaryColor(instance),
      scale: { minimum: 0, maximum: 100 }, references: this.referencesFor('rsi'),
    }];
    if (instance.definitionId === 'cci') return [{
      seriesKey: 'primary', name: `CCI ${length}`, data: points, color: primaryColor(instance),
      references: this.referencesFor('cci'),
    }];
    return [{ seriesKey: 'primary', name: `EMA ${length}`, data: points, color: primaryColor(instance) }];
  }

  static mapVolume(data: VolumeData[], instance: IndicatorInstanceV1): IndicatorSeriesData[] {
    return [{ seriesKey: 'volume', name: 'Volume', type: 'histogram', data, color: instance.styles.volume?.color ?? '#58A6FF' }];
  }

  static currentValues(series: IndicatorSeriesData[]): Record<string, number | null> {
    return Object.fromEntries(series.map(item => [item.seriesKey, item.data.length ? Number(item.data[item.data.length - 1].value) : null]));
  }
}
