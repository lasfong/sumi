import type { IndicatorDataPoint } from '../../api/indicatorsApi';
import type { Time } from 'lightweight-charts';
import { toDateKey } from '../../utils/date';
import type { IndicatorSeriesData, PaneId } from './workspaceTypes';

type Style = { color: string; type?: 'line' | 'histogram'; skip?: boolean };

const styleForColumn = (column: string, fallback?: string): Style => {
  const key = column.toLowerCase();
  if (key.startsWith('macdh')) return { color: 'rgba(0, 230, 118, 0.55)', type: 'histogram' };
  if (key.startsWith('macds')) return { color: '#FF8A00' };
  if (key.startsWith('macd')) return { color: '#58A6FF' };
  if (key.startsWith('bbl') || key.startsWith('bbu')) return { color: 'rgba(88, 166, 255, 0.7)' };
  if (key.startsWith('bbm')) return { color: '#FF8A00' };
  if (key.startsWith('isa')) return { color: '#00C853' };
  if (key.startsWith('isb')) return { color: '#FF5252' };
  if (key.startsWith('its')) return { color: '#58A6FF' };
  if (key.startsWith('iks')) return { color: '#FF5252' };
  if (key.startsWith('ics')) return { color: '#00E676' };
  if (key.startsWith('adx')) return { color: '#F0F6FC' };
  if (key.startsWith('dmp')) return { color: '#00E676' };
  if (key.startsWith('dmn')) return { color: '#FF5252' };
  if (key.startsWith('supertd') || key.startsWith('supertl') || key.startsWith('superts')) return { color: '', skip: true };
  if (key.startsWith('supert_')) return { color: '#00E676' };
  if (key.startsWith('stochk')) return { color: '#58A6FF' };
  if (key.startsWith('stochd')) return { color: '#FF8A00' };
  return { color: fallback ?? '#B388FF' };
};

export class IndicatorRenderRegistry {
  static paneFor(indicatorId: string, declaredPane: 'main' | 'oscillator'): PaneId {
    if (declaredPane === 'main') return 'price';
    return `indicator:${indicatorId}`;
  }

  static mapBackendData(data: IndicatorDataPoint[], fallbackColor?: string): IndicatorSeriesData[] {
    if (data.length === 0) return [];
    return Object.keys(data[0]).filter(column => column !== 'timestamp').flatMap(column => {
      const style = styleForColumn(column, fallbackColor);
      if (style.skip) return [];
      const points = data.flatMap(row => {
        const rawValue = row[column];
        if (rawValue === null || rawValue === '') return [];
        const value = typeof rawValue === 'number' ? rawValue : Number(rawValue);
        return Number.isFinite(value)
          ? [{ time: (toDateKey(row.timestamp) || row.timestamp) as Time, value }]
          : [];
      });
      return points.length ? [{ name: column, data: points, color: style.color, type: style.type }] : [];
    });
  }
}
