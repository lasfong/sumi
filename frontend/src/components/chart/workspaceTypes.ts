import type { LineData, SeriesMarker, Time } from 'lightweight-charts';

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
  name: string;
  data: LineData[];
  color?: string;
  type?: 'line' | 'histogram';
}

export interface IndicatorRenderInput {
  id: string;
  key: string;
  pane: 'main' | 'oscillator';
  series: IndicatorSeriesData[];
}

export interface ChartWorkspaceProps {
  data: CandleData[];
  volumeData?: VolumeData[];
  markers?: SeriesMarker<Time>[];
  drawings?: DrawingLine[];
  activeTool?: DrawingType | null;
  onDrawingComplete?: (drawing: DrawingLine) => void;
}

export interface ChartWorkspaceRef {
  addIndicator: (input: IndicatorRenderInput) => void;
  removeIndicator: (key: string) => void;
  clearIndicators: () => void;
  updateCandle: (candle: CandleData, volume?: VolumeData) => void;
}
