import type { SumiDrawingAnchor } from './drawingDomain';
export type MagnetMode = 'off' | 'ohlc';
export interface MagnetCandle { time: string; open: number; high: number; low: number; close: number }
export interface MagnetProjection { timeToX: (time: string) => number | null; priceToY: (price: number) => number | null }
const FIELDS = ['open', 'high', 'low', 'close'] as const;
export const snapAnchor = (raw: SumiDrawingAnchor, rawPoint: { x: number; y: number }, candles: MagnetCandle[], projection: MagnetProjection, mode: MagnetMode, threshold = 10): SumiDrawingAnchor => {
  if (mode === 'off') return raw;
  const candidates = candles.flatMap((candle, candleIndex) => {
    const x = projection.timeToX(candle.time); if (x === null || Math.abs(x - rawPoint.x) > threshold) return [];
    return FIELDS.flatMap((field, fieldIndex) => {
      const y = projection.priceToY(candle[field]); if (y === null || Math.abs(y - rawPoint.y) > threshold) return [];
      return [{ candle, price: candle[field], candleIndex, fieldIndex, distance: ((x - rawPoint.x) ** 2) + ((y - rawPoint.y) ** 2) }];
    });
  }).sort((a, b) => a.distance - b.distance || a.candleIndex - b.candleIndex || a.fieldIndex - b.fieldIndex);
  const winner = candidates[0]; return winner ? { time: winner.candle.time, price: winner.price } : raw;
};
