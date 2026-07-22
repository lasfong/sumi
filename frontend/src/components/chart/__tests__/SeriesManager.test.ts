import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { IChartApi, ISeriesApi } from 'lightweight-charts';
import type { PaneManager } from '../PaneManager';
import { SeriesManager } from '../SeriesManager';

vi.mock('lightweight-charts', () => ({
  CandlestickSeries: 'candles', HistogramSeries: 'histogram', LineSeries: 'line', LineStyle: { Dashed: 2 },
  createSeriesMarkers: () => ({ setMarkers: vi.fn() }),
}));

const makeSeries = () => ({ setData: vi.fn(), update: vi.fn(), createPriceLine: vi.fn() });

describe('SeriesManager indicator ownership', () => {
  let created: ReturnType<typeof makeSeries>[];
  let chart: { addSeries: ReturnType<typeof vi.fn>; removeSeries: ReturnType<typeof vi.fn> };
  let panes: Pick<PaneManager, 'index' | 'removeIfEmpty' | 'snapshot' | 'layout' | 'resize'>;

  beforeEach(() => {
    created = [];
    chart = {
      addSeries: vi.fn(() => { const series = makeSeries(); created.push(series); return series as unknown as ISeriesApi<'Line'>; }),
      removeSeries: vi.fn(),
    };
    panes = { index: vi.fn(() => 1), removeIfEmpty: vi.fn(), snapshot: vi.fn(() => []), layout: vi.fn(), resize: vi.fn() };
  });

  it('keys duplicate same-type series by stable instance and cleans exactly one owner', () => {
    const manager = new SeriesManager(chart as unknown as IChartApi, panes as PaneManager);
    manager.setIndicator('uuid-a', 'indicator:uuid-a', [{ seriesKey: 'primary', name: 'RSI 14', data: [] }]);
    manager.setIndicator('uuid-b', 'indicator:uuid-b', [{ seriesKey: 'primary', name: 'RSI 21', data: [] }]);
    expect(manager.snapshot().keys).toEqual(['uuid-a', 'uuid-b']);
    manager.removeIndicator('uuid-a');
    expect(manager.snapshot().keys).toEqual(['uuid-b']);
    expect(chart.removeSeries).toHaveBeenCalledTimes(1);
    expect(panes.removeIfEmpty).toHaveBeenCalledWith('indicator:uuid-a');
  });

  it('replaces a stable instance without orphaning its old component series', () => {
    const manager = new SeriesManager(chart as unknown as IChartApi, panes as PaneManager);
    manager.setIndicator('uuid-macd', 'indicator:uuid-macd', [
      { seriesKey: 'macd', name: 'MACD', data: [] }, { seriesKey: 'signal', name: 'Signal', data: [] },
      { seriesKey: 'histogram', name: 'Histogram', type: 'histogram', data: [], references: [{ value: 0, label: 'Zero' }] },
    ]);
    manager.setIndicator('uuid-macd', 'indicator:uuid-macd', [{ seriesKey: 'macd', name: 'MACD', data: [] }]);
    expect(chart.removeSeries).toHaveBeenCalledTimes(3);
    expect(manager.snapshot().instances).toEqual([{ id: 'uuid-macd', paneId: 'indicator:uuid-macd', series: ['macd'], seriesMaxDates: { macd: null }, references: [] }]);
  });

  it('rolls back staged series and retains prior valid ownership when chart data application fails', () => {
    const manager = new SeriesManager(chart as unknown as IChartApi, panes as PaneManager);
    manager.setIndicator('uuid-rsi', 'indicator:uuid-rsi', [{ seriesKey: 'primary', name: 'RSI 14', data: [] }]);
    chart.addSeries.mockImplementationOnce(() => {
      const failing = makeSeries(); failing.setData.mockImplementationOnce(() => { throw new Error('setData failed'); });
      created.push(failing); return failing as unknown as ISeriesApi<'Line'>;
    });
    expect(() => manager.setIndicator('uuid-rsi', 'indicator:uuid-rsi', [{ seriesKey: 'primary', name: 'RSI 21', data: [] }])).toThrow('setData failed');
    expect(manager.snapshot().instances[0].series).toEqual(['primary']);
    expect(chart.removeSeries).toHaveBeenCalledTimes(1);
  });

  it('rolls back all staged components and preserves the prior instance when layout fails', () => {
    const manager = new SeriesManager(chart as unknown as IChartApi, panes as PaneManager);
    manager.setIndicator('uuid-macd', 'indicator:uuid-macd', [{ seriesKey: 'macd', name: 'MACD', data: [] }]);
    vi.mocked(panes.layout).mockImplementationOnce(() => { throw new Error('layout failed'); });
    expect(() => manager.setIndicator('uuid-macd', 'indicator:uuid-macd', [
      { seriesKey: 'macd', name: 'MACD', data: [] }, { seriesKey: 'signal', name: 'Signal', data: [] },
    ], ['indicator:uuid-macd'])).toThrow('layout failed');
    expect(manager.snapshot().instances[0].series).toEqual(['macd']);
    expect(chart.removeSeries).toHaveBeenCalledTimes(2);
  });
});
