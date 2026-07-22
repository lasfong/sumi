import { describe, expect, it, vi } from 'vitest';
import type { IChartApi, ISeriesApi } from 'lightweight-charts';
import { createDrawing, emptyDrawingDocument, type SumiDrawing, type SumiDrawingDocumentV1 } from '../drawingDomain';
import { SumiPrimitiveDrawingProvider } from '../SumiPrimitiveDrawingProvider';

const drawingDocument = (): SumiDrawingDocumentV1 => ({
  schemaVersion: 1, revision: 1, sessionId: 1, symbol: 'FPT', drawings: [{
    id: '123e4567-e89b-42d3-a456-426614174000', tool: 'horizontal', paneId: 'price', order: 0,
    visible: true, locked: false, anchors: [{ time: '2026-07-15', price: 100 }],
    style: { lineColor: '#E056FD', lineWidth: 2, lineStyle: 'solid' }, geometry: { kind: 'horizontal' },
  }],
});

const setup = (documentState = drawingDocument(), conversion?: { coordinateToTime?: (x: number) => string | null; coordinateToPrice?: (y: number) => number | null }) => {
  const container = document.createElement('div');
  const pane = document.createElement('div');
  pane.getBoundingClientRect = () => ({ x: 10, y: 100, top: 100, left: 10, right: 810, bottom: 400, width: 800, height: 300, toJSON: () => ({}) });
  const attachPrimitive = vi.fn(); const detachPrimitive = vi.fn();
  const series = { attachPrimitive, detachPrimitive, coordinateToPrice: conversion?.coordinateToPrice ?? ((y: number) => y), priceToCoordinate: (price: number) => price } as unknown as ISeriesApi<'Candlestick'>;
  const applyOptions = vi.fn(); const chart = { applyOptions, paneSize: () => ({ width: 800, height: 300 }), timeScale: () => ({
    coordinateToTime: conversion?.coordinateToTime ?? ((x: number) => x < 230 ? '2026-07-15' : x < 310 ? '2026-07-16' : x < 390 ? '2026-07-17' : '2026-07-18'),
    timeToCoordinate: (time: string) => ({ '2026-07-15': 190, '2026-07-16': 270, '2026-07-17': 350, '2026-07-18': 430 })[time] ?? null,
  }) } as unknown as IChartApi;
  const capture = vi.fn(); const release = vi.fn(); const hasCapture = vi.fn(() => true);
  container.setPointerCapture = capture; container.releasePointerCapture = release; container.hasPointerCapture = hasCapture;
  const provider = new SumiPrimitiveDrawingProvider(chart, series, container, documentState, () => '2026-07-15', () => pane);
  provider.setMagnet('off', [
    { time: '2026-07-15', open: 90, high: 110, low: 80, close: 100 },
    { time: '2026-07-16', open: 140, high: 170, low: 130, close: 160 },
    { time: '2026-07-17', open: 150, high: 180, low: 140, close: 170 },
    { time: '2026-07-18', open: 160, high: 190, low: 150, close: 180 },
  ]);
  const dispatch = (type: string, x: number, y: number) => container.dispatchEvent(new MouseEvent(type, { button: 0, clientX: x, clientY: y, bubbles: true }));
  return { provider, container, dispatch, attachPrimitive, detachPrimitive, applyOptions, capture, release };
};

describe('SumiPrimitiveDrawingProvider contract', () => {
  it('registers/removes listeners, emits no events after destroy, and destroys idempotently', () => {
    const container = document.createElement('div');
    const add = vi.spyOn(container, 'addEventListener'); const remove = vi.spyOn(container, 'removeEventListener');
    const pane = document.createElement('div'); pane.getBoundingClientRect = () => ({ x: 0, y: 0, top: 0, left: 0, right: 800, bottom: 300, width: 800, height: 300, toJSON: () => ({}) });
    const detachPrimitive = vi.fn();
    const series = { attachPrimitive: vi.fn(), detachPrimitive, coordinateToPrice: (y: number) => y, priceToCoordinate: (price: number) => price } as unknown as ISeriesApi<'Candlestick'>;
    const provider = new SumiPrimitiveDrawingProvider({ applyOptions: vi.fn() } as unknown as IChartApi, series, container, emptyDrawingDocument(1, 'FPT'), () => '2026-07-15', () => pane);
    const events: string[] = []; provider.subscribe(event => events.push(event.type)); provider.destroy(); provider.destroy();
    container.dispatchEvent(new MouseEvent('pointerdown', { button: 0, clientX: 20, clientY: 20 }));
    expect(add).toHaveBeenCalledTimes(6); expect(remove).toHaveBeenCalledTimes(6);
    expect(events).toEqual([]); expect(detachPrimitive).toHaveBeenCalledOnce();
  });

  it('accepts create only inside the official price pane', () => {
    const { provider, dispatch } = setup(emptyDrawingDocument(1, 'FPT'));
    const events: string[] = []; provider.subscribe(event => events.push(event.type)); provider.setTool('horizontal');
    dispatch('pointerdown', 200, 450); expect(events).not.toContain('created');
    dispatch('pointerdown', 200, 220); expect(events.filter(type => type === 'created')).toHaveLength(1);
  });

  it('hit-selects and coalesces a drag into one committed event', () => {
    const { provider, dispatch, applyOptions, capture, release } = setup();
    const events: string[] = []; provider.subscribe(event => events.push(event.type));
    dispatch('pointerdown', 200, 200); dispatch('pointermove', 200, 230); dispatch('pointermove', 200, 240); dispatch('pointerup', 200, 240);
    expect(provider.snapshotInteraction().selectedIds).toEqual(['123e4567-e89b-42d3-a456-426614174000']);
    expect(events.filter(type => type === 'change-committed')).toHaveLength(1);
    expect(events.filter(type => type === 'change-preview')).toHaveLength(2);
    expect(capture).toHaveBeenCalledOnce(); expect(release).toHaveBeenCalledOnce();
    expect(applyOptions).toHaveBeenNthCalledWith(1, { handleScroll: false });
    expect(applyOptions).toHaveBeenLastCalledWith({ handleScroll: true });
  });

  it('cancel rolls a drag back without commit and restores pointer/scroll state', () => {
    const { provider, dispatch, applyOptions, release } = setup();
    const events: Array<{ type: string; drawings?: Array<{ anchors: Array<{ price: number }> }> }> = [];
    provider.subscribe(event => events.push(event));
    dispatch('pointerdown', 200, 200); dispatch('pointermove', 200, 250); provider.cancel();
    expect(events.filter(event => event.type === 'change-committed')).toHaveLength(0);
    expect(events.findLast(event => event.type === 'change-preview')?.drawings?.[0].anchors[0].price).toBe(100);
    expect(provider.snapshotInteraction()).toMatchObject({ selectedIds: ['123e4567-e89b-42d3-a456-426614174000'], dragging: false });
    expect(release).toHaveBeenCalledOnce(); expect(applyOptions).toHaveBeenLastCalledWith({ handleScroll: true });
  });

  it('native pointercancel rolls back exactly, never commits, cleans up, and stays inert after destroy', () => {
    const { provider, dispatch, applyOptions, release } = setup();
    const events: Array<{ type: string; drawings?: Array<{ anchors: Array<{ price: number }> }> }> = [];
    provider.subscribe(event => events.push(event));
    dispatch('pointerdown', 200, 200); dispatch('pointermove', 200, 260);
    expect(provider.snapshotInteraction().drawings[0].price).toBe(160);
    dispatch('pointercancel', 200, 260);
    expect(provider.snapshotInteraction()).toMatchObject({
      selectedIds: ['123e4567-e89b-42d3-a456-426614174000'], dragging: false,
      drawings: [{ id: '123e4567-e89b-42d3-a456-426614174000', price: 100 }],
    });
    expect(events.filter(event => event.type === 'change-committed')).toHaveLength(0);
    expect(events.findLast(event => event.type === 'change-preview')?.drawings?.[0].anchors[0].price).toBe(100);
    expect(release).toHaveBeenCalledOnce(); expect(applyOptions).toHaveBeenLastCalledWith({ handleScroll: true });
    const eventCount = events.length; provider.destroy(); dispatch('pointercancel', 200, 260);
    expect(events).toHaveLength(eventCount);
  });

  it('creates every multi-anchor tool once and cancels incomplete previews without a commit', () => {
    for (const tool of ['trendline', 'ray', 'rectangle', 'fibonacci-retracement'] as const) {
      const cancelled = setup(emptyDrawingDocument(1, 'FPT')); const cancelledEvents: string[] = []; cancelled.provider.subscribe(event => cancelledEvents.push(event.type));
      cancelled.provider.setTool(tool); cancelled.dispatch('pointerdown', 200, 220); expect(cancelled.provider.snapshotInteraction().preview?.anchors).toHaveLength(1);
      cancelled.provider.cancel(); expect(cancelled.provider.snapshotInteraction()).toMatchObject({ tool: 'select', preview: null, dragging: false }); expect(cancelledEvents).not.toContain('created');
      const completed = setup(emptyDrawingDocument(1, 'FPT')); const drawings: Array<{ tool: string }> = []; completed.provider.subscribe(event => { if (event.type === 'created') drawings.push(event.drawing); });
      completed.provider.setTool(tool); completed.dispatch('pointerdown', 200, 220); completed.dispatch('pointermove', 260, 250); completed.dispatch('pointerdown', 280, 260);
      expect(drawings).toHaveLength(1); expect(drawings[0].tool).toBe(tool); expect(completed.provider.snapshotInteraction().preview).toBeNull();
    }
  });

  it('requests explicit Text entry without creating a provider orphan', () => {
    const { provider, dispatch } = setup(emptyDrawingDocument(1, 'FPT')); const events: string[] = []; provider.subscribe(event => events.push(event.type));
    provider.setTool('text'); dispatch('pointerdown', 200, 220);
    expect(events).toEqual(['text-placement-requested']); expect(provider.snapshotInteraction()).toMatchObject({ tool: 'select', preview: null });
  });
  it('rolls back active drag and two-anchor preview before switching tools', () => {
    const active = setup(); const events: Array<{ type: string; drawings?: Array<{ anchors: Array<{ price: number }> }> }> = [];
    active.provider.subscribe(event => events.push(event)); active.dispatch('pointerdown', 200, 200); active.dispatch('pointermove', 200, 250);
    active.provider.setTool('trendline');
    expect(active.provider.snapshotInteraction()).toMatchObject({ tool: 'trendline', dragging: false, drawings: [{ price: 100 }] });
    expect(events.filter(event => event.type === 'change-committed')).toHaveLength(0);
    const preview = setup(emptyDrawingDocument(1, 'FPT')); const previewEvents: string[] = []; preview.provider.subscribe(event => previewEvents.push(event.type));
    preview.provider.setTool('trendline'); preview.dispatch('pointerdown', 200, 220); preview.dispatch('pointermove', 280, 250); preview.provider.setTool('rectangle');
    expect(preview.provider.snapshotInteraction()).toMatchObject({ tool: 'rectangle', preview: null, dragging: false }); expect(previewEvents).not.toContain('created');
  });
  it('rejects null official time/price conversion without dirty interaction state', () => {
    for (const conversion of [{ coordinateToTime: () => null }, { coordinateToPrice: () => null }]) {
      const { provider, dispatch } = setup(emptyDrawingDocument(1, 'FPT'), conversion); const events: string[] = []; provider.subscribe(event => events.push(event.type));
      provider.setTool('horizontal'); dispatch('pointerdown', 200, 220);
      expect(events).not.toContain('created'); expect(provider.snapshotInteraction()).toMatchObject({ preview: null, dragging: false });
    }
  });
  it('rejects leftward and equal Ray direction and preserves an active preview', () => {
    const left = setup(emptyDrawingDocument(1, 'FPT')); const events: string[] = []; left.provider.subscribe(event => events.push(event.type));
    left.provider.setTool('ray'); left.dispatch('pointerdown', 280, 220); left.dispatch('pointerdown', 200, 240);
    expect(events).not.toContain('created'); expect(left.provider.snapshotInteraction().preview?.anchors).toHaveLength(1);
    const equal = setup(emptyDrawingDocument(1, 'FPT')); const equalEvents: string[] = []; equal.provider.subscribe(event => equalEvents.push(event.type));
    equal.provider.setTool('ray'); equal.dispatch('pointerdown', 200, 220); equal.dispatch('pointerdown', 210, 240); expect(equalEvents).not.toContain('created');
  });
  it('maps an actual mixed Rectangle corner drag to only its semantic time and price fields', () => {
    const rectangle: SumiDrawingDocumentV1 = { schemaVersion: 1, revision: 1, sessionId: 1, symbol: 'FPT', drawings: [{
      id: '223e4567-e89b-42d3-a456-426614174000', tool: 'rectangle', paneId: 'price', order: 0, visible: true, locked: false,
      anchors: [{ time: '2026-07-15', price: 100 }, { time: '2026-07-16', price: 160 }], style: { lineColor: '#fff', lineWidth: 2, lineStyle: 'solid' }, geometry: { kind: 'rectangle' },
    }] };
    const { provider, dispatch } = setup(rectangle); let committed: SumiDrawing[] | null = null;
    provider.subscribe(event => { if (event.type === 'change-committed') committed = event.after; });
    dispatch('pointerdown', 280, 200); expect(provider.snapshotInteraction().dragPart).toBe('corner:1'); dispatch('pointermove', 280, 230); dispatch('pointerup', 280, 230);
    expect((committed as SumiDrawing[] | null)?.[0].anchors).toEqual([{ time: '2026-07-15', price: 130 }, { time: '2026-07-16', price: 160 }]);
  });
  const multiAnchorDocument = (tool: 'trendline' | 'ray' | 'rectangle' | 'fibonacci-retracement'): SumiDrawingDocumentV1 => {
    const drawing = createDrawing(tool, [{ time: '2026-07-15', price: 100 }, { time: '2026-07-16', price: 160 }], 0);
    drawing.id = '223e4567-e89b-42d3-a456-426614174000';
    return { schemaVersion: 1, revision: 1, sessionId: 1, symbol: 'FPT', drawings: [drawing] };
  };
  for (const tool of ['trendline', 'ray', 'rectangle', 'fibonacci-retracement'] as const) {
    for (const failureKind of ['time', 'price'] as const) {
      it(`atomically rejects ${tool} body when one of two ${failureKind} conversions fails`, () => {
        const { provider, container, dispatch, release, applyOptions } = setup(multiAnchorDocument(tool));
        const commits: SumiDrawing[][] = []; provider.subscribe(event => { if (event.type === 'change-committed') commits.push(event.after); });
        container.dataset.sumiDrawingBodyConversionFailure = `${failureKind}:1`;
        dispatch('pointerdown', 240, 230); expect(provider.snapshotInteraction().dragPart).toBe('body');
        dispatch('pointermove', 320, 240); dispatch('pointerup', 320, 240);
        expect(provider.snapshotInteraction()).toMatchObject({ dragging: false, drawings: [{ anchors: [
          { time: '2026-07-15', price: 100 }, { time: '2026-07-16', price: 160 },
        ] }] });
        expect(commits).toEqual([]); expect(release).toHaveBeenCalledOnce(); expect(applyOptions).toHaveBeenLastCalledWith({ handleScroll: true });
      });
    }
    it(`moves ${tool} body with exact equal logical-index and price deltas`, () => {
      const { provider, dispatch } = setup(multiAnchorDocument(tool)); let committed: SumiDrawing | null = null;
      provider.subscribe(event => { if (event.type === 'change-committed') committed = event.after[0]; });
      dispatch('pointerdown', 240, 230); dispatch('pointermove', 320, 240); dispatch('pointerup', 320, 240);
      const after = (committed as SumiDrawing | null)?.anchors;
      expect(after).toEqual([{ time: '2026-07-16', price: 110 }, { time: '2026-07-17', price: 170 }]);
      expect(after?.map((anchor, index) => anchor.price - ([100, 160][index] ?? 0))).toEqual([10, 10]);
    });
  }
  it('rolls back on pointer-capture loss and remains inert after idempotent destroy', () => {
    const { provider, container, dispatch } = setup(); const events: string[] = []; provider.subscribe(event => events.push(event.type));
    dispatch('pointerdown', 200, 200); dispatch('pointermove', 200, 250); dispatch('lostpointercapture', 200, 250);
    expect(provider.snapshotInteraction()).toMatchObject({ dragging: false, drawings: [{ price: 100 }] }); expect(events).not.toContain('change-committed');
    const count = events.length; provider.destroy(); provider.destroy(); container.dispatchEvent(new MouseEvent('pointerdown', { button: 0, clientX: 200, clientY: 220 })); expect(events).toHaveLength(count);
  });
});
