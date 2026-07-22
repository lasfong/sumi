import type { MagnetCandle, MagnetMode } from './drawingMagnet';
import type { SumiDrawing, SumiDrawingAnchor, SumiDrawingDocumentV1, DrawingTool } from './drawingDomain';
export type DrawingProviderEvent =
  | { type: 'created'; drawing: SumiDrawing }
  | { type: 'text-placement-requested'; anchor: SumiDrawingAnchor }
  | { type: 'selection-changed'; drawingIds: string[] }
  | { type: 'change-started'; drawingIds: string[] }
  | { type: 'change-preview'; drawings: SumiDrawing[] }
  | { type: 'change-committed'; before: SumiDrawing[]; after: SumiDrawing[] }
  | { type: 'cancelled'; tool: DrawingTool };
export interface DrawingInteractionSnapshot {
  tool: DrawingTool; selectedIds: string[];
  pricePane: { left: number; top: number; width: number; height: number } | null;
  drawings: Array<{ id: string; tool: SumiDrawing['tool']; price: number; coordinate: number | null; anchors: Array<{ time: string; price: number; x: number | null; y: number | null }>;
    handles: Array<{ part: string; x: number | null; y: number | null }>; bounds: { left: number; top: number; right: number; bottom: number } | null;
    visible: boolean; providerVisible: boolean; selected: boolean; geometry: SumiDrawing['geometry'] }>;
  preview: { tool: DrawingTool; anchors: SumiDrawingAnchor[] } | null;
  magnet: { mode: MagnetMode; threshold: number; visibleCandles: Array<{ time: string; x: number | null; prices: Array<{ field: 'open' | 'high' | 'low' | 'close'; price: number; y: number | null }> }> };
  dragging: boolean; dragPart: string | null; primitiveCount: number; listenerCount: number;
}
export interface DrawingProvider {
  setTool(tool: DrawingTool): void; setMagnet(mode: MagnetMode, candles: MagnetCandle[]): void;
  cancel(): void; select(ids: string[]): void; replaceDocument(document: SumiDrawingDocumentV1): void;
  snapshotInteraction(): DrawingInteractionSnapshot;
  subscribe(listener: (event: DrawingProviderEvent) => void): () => void; destroy(): void;
}
