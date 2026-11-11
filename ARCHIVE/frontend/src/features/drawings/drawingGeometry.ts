import type { FibonacciDirection, SumiDrawing, SumiDrawingAnchor } from './drawingDomain';

export interface ScreenPoint { x: number; y: number }
export interface ProjectedDrawing { drawing: SumiDrawing; anchors: ScreenPoint[] }
export interface HitResult { drawingId: string; part: 'body' | `anchor:${number}` | `corner:${number}`; distance: number }
export interface TextLayout { lines: Array<{ text: string; x: number; y: number }>; bounds: { left: number; top: number; right: number; bottom: number }; truncated: boolean }

export const distanceToSegment = (point: ScreenPoint, start: ScreenPoint, end: ScreenPoint): number => {
  const dx = end.x - start.x; const dy = end.y - start.y;
  if (dx === 0 && dy === 0) return Math.hypot(point.x - start.x, point.y - start.y);
  const t = Math.max(0, Math.min(1, ((point.x - start.x) * dx + (point.y - start.y) * dy) / (dx * dx + dy * dy)));
  return Math.hypot(point.x - (start.x + t * dx), point.y - (start.y + t * dy));
};
export const pointInRectangle = (point: ScreenPoint, a: ScreenPoint, b: ScreenPoint, padding = 0) => point.x >= Math.min(a.x, b.x) - padding
  && point.x <= Math.max(a.x, b.x) + padding && point.y >= Math.min(a.y, b.y) - padding && point.y <= Math.max(a.y, b.y) + padding;
export const rayEndpoint = (start: ScreenPoint, through: ScreenPoint, right: number): ScreenPoint => {
  if (through.x === start.x) return through;
  const scale = (right - start.x) / (through.x - start.x);
  return { x: right, y: start.y + ((through.y - start.y) * scale) };
};
export const rectangleCorners = (anchors: readonly ScreenPoint[]): [ScreenPoint, ScreenPoint, ScreenPoint, ScreenPoint] => {
  const [a, b] = anchors; return [a, { x: b.x, y: a.y }, b, { x: a.x, y: b.y }];
};
export const layoutDrawingText = (text: string, anchor: ScreenPoint, fontSize: number, maxWidth = 240, maxLines = 12): TextLayout => {
  const charWidth = fontSize * 0.6; const maxChars = Math.max(1, Math.floor(maxWidth / charWidth)); const expanded: string[] = [];
  for (const explicitLine of text.split('\n')) {
    if (!explicitLine.length) { expanded.push(''); continue; }
    for (let offset = 0; offset < explicitLine.length; offset += maxChars) expanded.push(explicitLine.slice(offset, offset + maxChars));
  }
  const truncated = expanded.length > maxLines; const visible = expanded.slice(0, maxLines);
  if (truncated && visible.length) visible[visible.length - 1] = `${visible[visible.length - 1].slice(0, Math.max(0, maxChars - 1))}…`;
  const lineHeight = fontSize * 1.35; const x = anchor.x + 4;
  const lines = visible.map((line, index) => ({ text: line, x, y: anchor.y + (index * lineHeight) }));
  const width = Math.min(maxWidth, Math.max(charWidth, ...visible.map(line => line.length * charWidth)));
  return { lines, truncated, bounds: { left: x, top: anchor.y - fontSize, right: x + width, bottom: anchor.y + ((Math.max(1, visible.length) - 1) * lineHeight) + (fontSize * 0.3) } };
};
export const fibonacciPrice = (anchors: [SumiDrawingAnchor, SumiDrawingAnchor], ratio: number, direction: FibonacciDirection) => {
  const [start, end] = direction === 'start-to-end' ? anchors : [anchors[1], anchors[0]];
  return start.price + ((end.price - start.price) * ratio);
};
export const hitProjectedDrawing = (projected: ProjectedDrawing, point: ScreenPoint, width: number): Omit<HitResult, 'drawingId'> | null => {
  const { drawing, anchors } = projected;
  if (drawing.tool === 'rectangle') {
    const corners = rectangleCorners(anchors);
    for (let index = corners.length - 1; index >= 0; index -= 1) {
      const distance = Math.hypot(point.x - corners[index].x, point.y - corners[index].y);
      if (distance <= 9) return { part: `corner:${index}`, distance };
    }
  }
  for (let index = anchors.length - 1; index >= 0; index -= 1) {
    const distance = Math.hypot(point.x - anchors[index].x, point.y - anchors[index].y);
    if (distance <= 9) return { part: `anchor:${index}`, distance };
  }
  if (drawing.tool === 'horizontal') { const distance = Math.abs(point.y - anchors[0].y); return distance <= 8 ? { part: 'body', distance } : null; }
  if (drawing.tool === 'rectangle') return pointInRectangle(point, anchors[0], anchors[1], 5) ? { part: 'body', distance: 0 } : null;
  if (drawing.tool === 'text') { const bounds = layoutDrawingText(drawing.geometry.text, anchors[0], drawing.style.fontSize ?? 14).bounds;
    return point.x >= bounds.left - 4 && point.x <= bounds.right + 4 && point.y >= bounds.top - 4 && point.y <= bounds.bottom + 4 ? { part: 'body', distance: 0 } : null; }
  if (drawing.tool === 'fibonacci-retracement' || drawing.tool === 'risk-reward') {
    const top = Math.min(...anchors.map(a => a.y)); const bottom = Math.max(...anchors.map(a => a.y));
    return point.x >= Math.min(...anchors.map(anchor => anchor.x)) - 5 && point.x <= width && point.y >= top - 5 && point.y <= bottom + 5 ? { part: 'body', distance: 0 } : null;
  }
  const end = drawing.tool === 'ray' ? rayEndpoint(anchors[0], anchors[1], width) : anchors[1];
  const distance = distanceToSegment(point, anchors[0], end); return distance <= 8 ? { part: 'body', distance } : null;
};
