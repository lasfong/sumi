import { describe, expect, it, vi } from 'vitest';
import type { IChartApi, IPaneApi, Time } from 'lightweight-charts';
import { PaneManager } from '../PaneManager';
import type { PaneId } from '../workspaceTypes';

interface FakePane {
  id: string;
  factor: number;
  series: unknown[];
  api: IPaneApi<Time>;
}

const chartFixture = () => {
  const order: FakePane[] = [];
  const makePane = (id: string): FakePane => {
    const item = { id, factor: 1, series: [] as unknown[] } as FakePane;
    item.api = {
      paneIndex: vi.fn(() => order.indexOf(item)),
      moveTo: vi.fn((target: number) => { order.splice(order.indexOf(item), 1); order.splice(target, 0, item); }),
      setStretchFactor: vi.fn((factor: number) => { item.factor = factor; }),
      getStretchFactor: vi.fn(() => item.factor),
      getHeight: vi.fn(() => Math.round(600 * item.factor / order.reduce((sum, pane) => sum + pane.factor, 0))),
      getSeries: vi.fn(() => item.series),
    } as unknown as IPaneApi<Time>;
    return item;
  };
  order.push(makePane('price'));
  const chart = {
    panes: vi.fn(() => order.map(item => item.api)),
    addPane: vi.fn(() => { const pane = makePane(`created-${order.length}`); order.push(pane); return pane.api; }),
    removePane: vi.fn((index: number) => { order.splice(index, 1); }),
  } as unknown as IChartApi;
  return { chart, order };
};

const createInOrder = (creationOrder: PaneId[]) => {
  const fixture = chartFixture();
  const manager = new PaneManager(fixture.chart);
  const desired: PaneId[] = ['volume', 'indicator:rsi', 'indicator:macd', 'indicator:cci'];
  manager.layout(desired, 600);
  creationOrder.forEach(id => manager.get(id));
  manager.layout(desired, 600);
  return manager.snapshot();
};

describe('PaneManager deterministic fixed layout', () => {
  it.each([
    [['volume', 'indicator:rsi', 'indicator:macd', 'indicator:cci']],
    [['indicator:cci', 'indicator:macd', 'volume', 'indicator:rsi']],
    [['indicator:rsi', 'volume', 'indicator:cci', 'indicator:macd']],
  ] as Array<[PaneId[]]>)('reconciles asynchronous creation order %j to document order and 4:1 factors', creationOrder => {
    const snapshot = createInOrder(creationOrder);
    expect(snapshot.map(pane => pane.id)).toEqual(['price', 'volume', 'indicator:rsi', 'indicator:macd', 'indicator:cci']);
    expect(snapshot.map(pane => pane.stretchFactor)).toEqual([4, 1, 1, 1, 1]);
    expect(snapshot.map(pane => pane.height)).toEqual([300, 75, 75, 75, 75]);
  });

  it('keeps exact order/factors across repeated layout, resize, and removal', () => {
    const fixture = chartFixture();
    const manager = new PaneManager(fixture.chart);
    manager.layout(['indicator:rsi', 'indicator:cci'], 480);
    manager.get('indicator:cci'); manager.get('indicator:rsi');
    for (let cycle = 0; cycle < 10; cycle += 1) {
      manager.layout(['indicator:rsi', 'indicator:cci'], cycle % 2 ? 480 : 720);
      manager.resize(cycle % 2 ? 720 : 480);
    }
    expect(manager.snapshot().map(pane => [pane.id, pane.stretchFactor])).toEqual([
      ['price', 4], ['indicator:rsi', 1], ['indicator:cci', 1],
    ]);
    manager.removeIfEmpty('indicator:rsi');
    expect(manager.snapshot().map(pane => pane.id)).toEqual(['price', 'indicator:cci']);
    expect(manager.snapshot().map(pane => pane.stretchFactor)).toEqual([4, 1]);
  });
});
