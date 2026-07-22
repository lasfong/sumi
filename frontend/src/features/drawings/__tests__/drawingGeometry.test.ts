import { describe, expect, it } from 'vitest';
import allTools from '../__fixtures__/valid-all-tools-document.json';
import { distanceToSegment, fibonacciPrice, hitProjectedDrawing, layoutDrawingText, pointInRectangle, rayEndpoint, rectangleCorners } from '../drawingGeometry';
import { snapAnchor } from '../drawingMagnet';
import type { SumiDrawingDocumentV1 } from '../drawingDomain';

const document = allTools as SumiDrawingDocumentV1;
describe('drawing geometry and hit semantics', () => {
  it('projects ray to the right and rectangle without mutating directional anchors', () => {
    expect(rayEndpoint({ x: 10, y: 100 }, { x: 20, y: 90 }, 100)).toEqual({ x: 100, y: 10 });
    expect(pointInRectangle({ x: 15, y: 15 }, { x: 20, y: 20 }, { x: 10, y: 10 })).toBe(true);
    expect(distanceToSegment({ x: 15, y: 15 }, { x: 10, y: 10 }, { x: 20, y: 20 })).toBeCloseTo(0);
  });
  it('computes canonical Fibonacci direction and exact standard prices', () => {
    const fib = document.drawings[4]; if (fib.tool !== 'fibonacci-retracement') throw new Error('fixture');
    expect(fibonacciPrice(fib.anchors, 0.618, 'start-to-end')).toBeCloseTo(108.54);
    expect(fibonacciPrice(fib.anchors, 0.618, 'end-to-start')).toBeCloseTo(101.46);
  });
  it('gives anchor handles precedence over drawing bodies', () => {
    const trend = document.drawings[1];
    expect(hitProjectedDrawing({ drawing: trend, anchors: [{ x: 10, y: 10 }, { x: 50, y: 50 }] }, { x: 11, y: 11 }, 100)?.part).toBe('anchor:0');
    expect(hitProjectedDrawing({ drawing: trend, anchors: [{ x: 10, y: 10 }, { x: 50, y: 50 }] }, { x: 30, y: 30 }, 100)?.part).toBe('body');
  });
  it('exposes all four Rectangle corner handles with deterministic indexes', () => {
    const rectangle = document.drawings[3]; const anchors = [{ x: 10, y: 20 }, { x: 80, y: 90 }];
    expect(rectangleCorners(anchors)).toEqual([{ x: 10, y: 20 }, { x: 80, y: 20 }, { x: 80, y: 90 }, { x: 10, y: 90 }]);
    rectangleCorners(anchors).forEach((corner, index) => expect(hitProjectedDrawing({ drawing: rectangle, anchors }, corner, 100)?.part).toBe(`corner:${index}`));
  });
  it('uses the same bounded explicit-newline Text layout for display and hit bounds', () => {
    const layout = layoutDrawingText('First line\nSecond line that wraps across the bounded width', { x: 10, y: 30 }, 14, 100, 4);
    expect(layout.lines.length).toBe(4); expect(layout.truncated).toBe(true); expect(layout.lines[0].text).toBe('First line');
    expect(layout.bounds).toMatchObject({ left: 14, top: 16 });
    const text = document.drawings[5]; if (text.tool !== 'text') throw new Error('fixture');
    expect(hitProjectedDrawing({ drawing: { ...text, geometry: { ...text.geometry, text: 'First\nSecond' } }, anchors: [{ x: 10, y: 30 }] }, { x: 30, y: 45 }, 100)?.part).toBe('body');
  });
});

describe('OHLC magnet', () => {
  const candles = [{ time: '2026-07-01', open: 100, high: 110, low: 90, close: 105 }];
  const projection = { timeToX: () => 50, priceToY: (price: number) => 200 - price };
  it('proves snapped, unsnapped and threshold behavior', () => {
    const raw = { time: '2026-07-02', price: 109.5 };
    expect(snapAnchor(raw, { x: 51, y: 90.5 }, candles, projection, 'ohlc')).toEqual({ time: '2026-07-01', price: 110 });
    expect(snapAnchor(raw, { x: 51, y: 90.5 }, candles, projection, 'off')).toEqual(raw);
    expect(snapAnchor(raw, { x: 61, y: 90.5 }, candles, projection, 'ohlc')).toEqual(raw);
  });
  it('uses deterministic OHLC field priority for exact ties and never sees undisclosed candles', () => {
    const tied = [{ time: '2026-07-01', open: 100, high: 100, low: 90, close: 95 }];
    expect(snapAnchor({ time: 'x', price: 100 }, { x: 50, y: 100 }, tied, projection, 'ohlc')).toEqual({ time: '2026-07-01', price: 100 });
    expect(snapAnchor({ time: '2026-07-09', price: 150 }, { x: 50, y: 50 }, [], projection, 'ohlc')).toEqual({ time: '2026-07-09', price: 150 });
  });
});
