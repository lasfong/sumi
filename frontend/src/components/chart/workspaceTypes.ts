import type { HistogramData, LineData, SeriesMarker, Time } from 'lightweight-charts';
import type { DrawingInteractionSnapshot, DrawingProviderEvent } from '../../features/drawings/DrawingProvider';
import type { DrawingTool as PrimitiveDrawingTool, SumiDrawingDocumentV1 } from '../../features/drawings/drawingDomain';
import type { MagnetMode } from '../../features/drawings/drawingMagnet';

export type PaneId = 'price' | 'volume' | `indicator:${string}`;
export type DrawingType = 'cursor' | 'trendline' | 'horizontal' | 'fibonacci';

export interface DrawingPoint {
  time: Time;
  price: number;
}

export interface DrawingLine {
  id: string;
  type: DrawingType;
  points: DrawingPoint[];
  color: string;
}

export interface CandleData {
  time: Time;
  open: number;
  high: number;
  low: number;
  close: number;
}

export interface VolumeData {
  time: Time;
  value: number;
  color: string;
}

export interface IndicatorSeriesData {
  seriesKey: string;
  name: string;
  data: Array<LineData | HistogramData>;
  color?: string;
  type?: 'line' | 'histogram';
  references?: Array<{ value: number; label: string; color?: string }>;
  scale?: { minimum: number; maximum: number };
}

export interface IndicatorRenderInput {
  instanceId: string;
  paneId: PaneId;
  series: IndicatorSeriesData[];
  paneOrder?: PaneId[];
}

export interface IndicatorChartSnapshot {
  keys: string[];
  candleInput: { count: number; maxDate: string | null };
  panes: Array<{ id: PaneId; index: number; height: number; stretchFactor: number; seriesCount: number }>;
  instances: Array<{ id: string; paneId: PaneId; series: string[]; seriesMaxDates: Record<string, string | null>; references: string[] }>;
}

export interface ChartWorkspaceProps {
  data: CandleData[];
  volumeData?: VolumeData[];
  markers?: SeriesMarker<Time>[];
  drawings?: DrawingLine[];
  drawingDocument?: SumiDrawingDocumentV1;
  drawingTool?: PrimitiveDrawingTool;
  drawingSelection?: string[];
  currentDrawingTime?: string;
  onDrawingProviderEvent?: (event: DrawingProviderEvent) => void;
  drawingMagnetMode?: MagnetMode;
  minimumHeight?: number;
}

export interface ChartWorkspaceRef {
  addIndicator: (input: IndicatorRenderInput) => void;
  removeIndicator: (key: string) => void;
  clearIndicators: () => void;
  setIndicatorOrder: (paneIds: PaneId[]) => void;
  getIndicatorState: () => IndicatorChartSnapshot | null;
  updateCandle: (candle: CandleData, volume?: VolumeData) => void;
  cancelDrawing: () => void;
  getDrawingInteractionState: () => DrawingInteractionSnapshot | null;
}
