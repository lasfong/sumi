export type DrawingTool = 'select' | DrawingKind;
export type DrawingKind = 'horizontal' | 'trendline' | 'ray' | 'rectangle' | 'fibonacci-retracement' | 'text' | 'risk-reward';
export type LineStyle = 'solid' | 'dashed' | 'dotted';
export type FibonacciDirection = 'start-to-end' | 'end-to-start';

export interface SumiDrawingAnchor { time: string; price: number }
export interface SumiDrawingStyle {
  lineColor: string; lineWidth: number; lineStyle: LineStyle;
  fillColor?: string; fillOpacity?: number; textColor?: string; fontSize?: number;
}
export interface FibonacciLevel { ratio: number; visible: boolean; color?: string }
interface SumiDrawingBase<T extends DrawingKind, A extends SumiDrawingAnchor[], G> {
  id: string; tool: T; paneId: 'price'; order: number; visible: boolean; locked: boolean;
  anchors: A; style: SumiDrawingStyle; geometry: G;
}
export type SumiHorizontalDrawing = SumiDrawingBase<'horizontal', [SumiDrawingAnchor], { kind: 'horizontal' }>;
export type SumiTrendlineDrawing = SumiDrawingBase<'trendline', [SumiDrawingAnchor, SumiDrawingAnchor], { kind: 'trendline' }>;
export type SumiRayDrawing = SumiDrawingBase<'ray', [SumiDrawingAnchor, SumiDrawingAnchor], { kind: 'ray' }>;
export type SumiRectangleDrawing = SumiDrawingBase<'rectangle', [SumiDrawingAnchor, SumiDrawingAnchor], { kind: 'rectangle' }>;
export type SumiFibonacciDrawing = SumiDrawingBase<'fibonacci-retracement', [SumiDrawingAnchor, SumiDrawingAnchor], {
  kind: 'fibonacci-retracement'; levels: FibonacciLevel[]; direction: FibonacciDirection;
}>;
export type SumiTextDrawing = SumiDrawingBase<'text', [SumiDrawingAnchor], { kind: 'text'; text: string }>;
export type SumiRiskRewardDrawing = SumiDrawingBase<'risk-reward', [SumiDrawingAnchor, SumiDrawingAnchor, SumiDrawingAnchor], {
  kind: 'risk-reward'; direction: 'long' | 'short'; riskRewardRatio: number;
}>;
export type SumiDrawing = SumiHorizontalDrawing | SumiTrendlineDrawing | SumiRayDrawing | SumiRectangleDrawing | SumiFibonacciDrawing | SumiTextDrawing | SumiRiskRewardDrawing;
export interface SumiDrawingDocumentV1 {
  schemaVersion: 1; revision: number; sessionId: number; symbol: string; drawings: SumiDrawing[];
}

export const FIBONACCI_RATIOS = [0, 0.236, 0.382, 0.5, 0.618, 0.786, 1] as const;
export const TEXT_MAX_LENGTH = 2000;
export const DEFAULT_DRAWING_STYLE: SumiDrawingStyle = {
  lineColor: '#E056FD', lineWidth: 2, lineStyle: 'solid', fillColor: '#E056FD', fillOpacity: 0.12,
};

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const DATE = /^\d{4}-\d{2}-\d{2}$/;
export const isDrawingDate = (value: string): boolean => {
  if (!DATE.test(value)) return false;
  const parsed = new Date(`${value}T00:00:00Z`);
  return Number.isFinite(parsed.getTime()) && parsed.toISOString().slice(0, 10) === value;
};
export const isRightwardRay = (anchors: readonly SumiDrawingAnchor[]): boolean => anchors.length === 2
  && isDrawingDate(anchors[0].time) && isDrawingDate(anchors[1].time) && anchors[1].time > anchors[0].time;
const exactKeys = (value: Record<string, unknown>, allowed: string[]) => Object.keys(value).every(key => allowed.includes(key));
const isRecord = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value);
const isAnchor = (value: unknown): value is SumiDrawingAnchor => isRecord(value)
  && exactKeys(value, ['time', 'price']) && typeof value.time === 'string' && isDrawingDate(value.time)
  && typeof value.price === 'number' && Number.isFinite(value.price) && value.price > 0;
const isStyle = (value: unknown): value is SumiDrawingStyle => {
  if (!isRecord(value) || !exactKeys(value, ['lineColor', 'lineWidth', 'lineStyle', 'fillColor', 'fillOpacity', 'textColor', 'fontSize'])) return false;
  if (typeof value.lineColor !== 'string' || value.lineColor.length < 1 || !Number.isInteger(value.lineWidth)
    || Number(value.lineWidth) < 1 || Number(value.lineWidth) > 8 || !['solid', 'dashed', 'dotted'].includes(String(value.lineStyle))) return false;
  return (value.fillColor === undefined || (typeof value.fillColor === 'string' && value.fillColor.length > 0))
    && (value.fillOpacity === undefined || (typeof value.fillOpacity === 'number' && Number.isFinite(value.fillOpacity) && value.fillOpacity >= 0 && value.fillOpacity <= 1))
    && (value.textColor === undefined || (typeof value.textColor === 'string' && value.textColor.length > 0))
    && (value.fontSize === undefined || (Number.isInteger(value.fontSize) && Number(value.fontSize) >= 8 && Number(value.fontSize) <= 72));
};
const validFibGeometry = (geometry: Record<string, unknown>) => {
  if (!exactKeys(geometry, ['kind', 'levels', 'direction']) || geometry.kind !== 'fibonacci-retracement'
    || !['start-to-end', 'end-to-start'].includes(String(geometry.direction)) || !Array.isArray(geometry.levels)
    || geometry.levels.length !== FIBONACCI_RATIOS.length) return false;
  return geometry.levels.every((level, index) => isRecord(level) && exactKeys(level, ['ratio', 'visible', 'color'])
    && level.ratio === FIBONACCI_RATIOS[index] && typeof level.visible === 'boolean'
    && (level.color === undefined || (typeof level.color === 'string' && level.color.length > 0)));
};
const isRiskRewardGeometry = (geometry: Record<string, unknown>) =>
  exactKeys(geometry, ['kind', 'direction', 'riskRewardRatio']) && geometry.kind === 'risk-reward'
  && ['long', 'short'].includes(String(geometry.direction))
  && typeof geometry.riskRewardRatio === 'number' && Number.isFinite(geometry.riskRewardRatio) && geometry.riskRewardRatio >= 0;

export const emptyDrawingDocument = (sessionId: number, symbol: string): SumiDrawingDocumentV1 => ({
  schemaVersion: 1, revision: 0, sessionId, symbol, drawings: [],
});

export const validateDrawingDocument = (value: unknown): value is SumiDrawingDocumentV1 => {
  if (!isRecord(value) || !exactKeys(value, ['schemaVersion', 'revision', 'sessionId', 'symbol', 'drawings'])
    || value.schemaVersion !== 1 || !Number.isInteger(value.revision) || Number(value.revision) < 0
    || !Number.isInteger(value.sessionId) || Number(value.sessionId) < 1
    || typeof value.symbol !== 'string' || value.symbol.length < 1 || value.symbol.length > 32 || !Array.isArray(value.drawings)) return false;
  const ids = new Set<string>();
  return value.drawings.every((item, index) => {
    if (!isRecord(item) || !exactKeys(item, ['id', 'tool', 'paneId', 'order', 'visible', 'locked', 'anchors', 'style', 'geometry'])
      || typeof item.id !== 'string' || !UUID.test(item.id) || ids.has(item.id) || item.paneId !== 'price'
      || item.order !== index || typeof item.visible !== 'boolean' || typeof item.locked !== 'boolean'
      || !Array.isArray(item.anchors) || !item.anchors.every(isAnchor) || !isStyle(item.style) || !isRecord(item.geometry)) return false;
    ids.add(item.id);
    if (item.tool === 'horizontal') return item.anchors.length === 1 && exactKeys(item.geometry, ['kind']) && item.geometry.kind === 'horizontal';
    if (item.tool === 'trendline') return item.anchors.length === 2 && exactKeys(item.geometry, ['kind']) && item.geometry.kind === 'trendline';
    if (item.tool === 'ray') return item.anchors.length === 2 && exactKeys(item.geometry, ['kind']) && item.geometry.kind === 'ray' && isRightwardRay(item.anchors);
    if (item.tool === 'rectangle') return item.anchors.length === 2 && exactKeys(item.geometry, ['kind']) && item.geometry.kind === 'rectangle';
    if (item.tool === 'fibonacci-retracement') return item.anchors.length === 2 && validFibGeometry(item.geometry);
    if (item.tool === 'risk-reward') return item.anchors.length === 3 && isRiskRewardGeometry(item.geometry);
    if (item.tool === 'text') return item.anchors.length === 1 && exactKeys(item.geometry, ['kind', 'text']) && item.geometry.kind === 'text'
      && typeof item.geometry.text === 'string' && item.geometry.text.length <= TEXT_MAX_LENGTH
      && item.geometry.text.trim().length > 0;
    return false;
  });
};

export interface DrawingDocumentIdentity { sessionId: number; symbol: string }
export const validateDrawingDocumentSemantics = (value: unknown, identity?: DrawingDocumentIdentity): value is SumiDrawingDocumentV1 =>
  validateDrawingDocument(value) && (!identity || (value.sessionId === identity.sessionId && value.symbol === identity.symbol));

export const parseDrawingDocument = (raw: string | null | undefined): SumiDrawingDocumentV1 | null => {
  if (!raw) return null;
  try { const value: unknown = JSON.parse(raw); return validateDrawingDocument(value) ? value : null; } catch { return null; }
};
export const normalizeDrawingOrder = (drawings: SumiDrawing[]): SumiDrawing[] => drawings.map((drawing, order) => ({ ...drawing, order }));
const base = <T extends DrawingKind>(tool: T, anchors: SumiDrawingAnchor[], order: number) => ({
  id: globalThis.crypto.randomUUID(), tool, paneId: 'price' as const, order, visible: true, locked: false,
  anchors, style: structuredClone(DEFAULT_DRAWING_STYLE),
});
export const createDrawing = (tool: Exclude<DrawingTool, 'select'>, anchors: SumiDrawingAnchor[], order: number, text = ''): SumiDrawing => {
  if (tool === 'horizontal' && anchors.length === 1) return { ...base(tool, anchors, order), anchors: [anchors[0]], geometry: { kind: tool } };
  if (tool === 'trendline' && anchors.length === 2) return { ...base(tool, anchors, order), anchors: [anchors[0], anchors[1]], geometry: { kind: tool } };
  if (tool === 'ray' && isRightwardRay(anchors)) return { ...base(tool, anchors, order), anchors: [anchors[0], anchors[1]], geometry: { kind: tool } };
  if (tool === 'rectangle' && anchors.length === 2) return { ...base(tool, anchors, order), anchors: [anchors[0], anchors[1]], geometry: { kind: tool } };
  if (tool === 'fibonacci-retracement' && anchors.length === 2) return { ...base(tool, anchors, order), anchors: [anchors[0], anchors[1]], geometry: {
    kind: tool, levels: FIBONACCI_RATIOS.map(ratio => ({ ratio, visible: true })), direction: 'start-to-end',
  } };
  if (tool === 'risk-reward' && anchors.length === 3) {
    const risk = Math.abs(anchors[0].price - anchors[1].price);
    const reward = Math.abs(anchors[2].price - anchors[0].price);
    const riskRewardRatio = risk > 0 ? reward / risk : 0;
    const direction = anchors[2].price >= anchors[0].price ? 'long' : 'short';
    return {
      ...base(tool, anchors, order),
      anchors: [anchors[0], anchors[1], anchors[2]],
      style: {
        ...structuredClone(DEFAULT_DRAWING_STYLE),
        lineColor: '#2962FF',
        fillColor: '#26A69A',
      },
      geometry: {
        kind: tool,
        direction,
        riskRewardRatio: Number(riskRewardRatio.toFixed(4)),
      },
    };
  }
  if (tool === 'text' && anchors.length === 1) return { ...base(tool, anchors, order), anchors: [anchors[0]], style: {
    ...structuredClone(DEFAULT_DRAWING_STYLE), textColor: '#F0F6FC', fontSize: 14,
  }, geometry: { kind: tool, text: text.trim() } };
  throw new TypeError(`Invalid anchors for ${tool}`);
};
export const createHorizontalDrawing = (time: string, price: number, order: number): SumiHorizontalDrawing =>
  createDrawing('horizontal', [{ time, price }], order) as SumiHorizontalDrawing;
