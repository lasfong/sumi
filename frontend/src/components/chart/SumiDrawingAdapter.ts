import type { DrawingLine, DrawingPoint, DrawingType } from './workspaceTypes';

const DRAWING_TYPES = new Set<DrawingType>(['cursor', 'trendline', 'horizontal', 'fibonacci']);

export class SumiDrawingAdapter {
  static serialize(drawings: DrawingLine[]): string {
    return JSON.stringify(drawings.map(drawing => ({
      id: drawing.id,
      type: drawing.type,
      color: drawing.color,
      points: drawing.points.map(point => ({ time: point.time, price: point.price })),
    })));
  }

  static deserialize(state?: string | null): DrawingLine[] {
    if (!state) return [];
    try {
      const parsed = JSON.parse(state) as unknown;
      if (!Array.isArray(parsed)) return [];
      return parsed.filter(SumiDrawingAdapter.isDrawing);
    } catch {
      return [];
    }
  }

  private static isDrawing(value: unknown): value is DrawingLine {
    if (!value || typeof value !== 'object') return false;
    const drawing = value as Record<string, unknown>;
    return typeof drawing.id === 'string'
      && typeof drawing.type === 'string'
      && DRAWING_TYPES.has(drawing.type as DrawingType)
      && typeof drawing.color === 'string'
      && Array.isArray(drawing.points)
      && drawing.points.every(SumiDrawingAdapter.isPoint);
  }

  private static isPoint(value: unknown): value is DrawingPoint {
    if (!value || typeof value !== 'object') return false;
    const point = value as Record<string, unknown>;
    return (typeof point.time === 'string' || typeof point.time === 'number')
      && typeof point.price === 'number' && Number.isFinite(point.price);
  }
}
