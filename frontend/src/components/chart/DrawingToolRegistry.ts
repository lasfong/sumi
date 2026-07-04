import {
  LineSeries,
  type IChartApi,
  type IPriceLine,
  type ISeriesApi,
  type LineData,
  type Time,
} from 'lightweight-charts';
import type { DrawingLine } from './workspaceTypes';

type DrawingRef =
  | { type: 'horizontal'; lines: IPriceLine[] }
  | { type: 'series'; series: ISeriesApi<'Line'>[] };

const timeValue = (time: Time): number => typeof time === 'string' ? new Date(time).getTime() : Number(time);

export class DrawingToolRegistry {
  private readonly refs = new Map<string, DrawingRef>();
  private readonly chart: IChartApi;
  private readonly priceSeries: ISeriesApi<'Candlestick'>;
  private readonly paneIndex: number;

  constructor(
    chart: IChartApi,
    priceSeries: ISeriesApi<'Candlestick'>,
    paneIndex: number,
  ) {
    this.chart = chart;
    this.priceSeries = priceSeries;
    this.paneIndex = paneIndex;
  }

  render(drawings: DrawingLine[]): void {
    this.clear();
    drawings.forEach(drawing => this.renderOne(drawing));
  }

  clear(): void {
    this.refs.forEach(ref => {
      if (ref.type === 'horizontal') ref.lines.forEach(line => this.priceSeries.removePriceLine(line));
      else ref.series.forEach(series => this.chart.removeSeries(series));
    });
    this.refs.clear();
  }

  createPreview(color = '#E056FD'): ISeriesApi<'Line'> {
    return this.chart.addSeries(LineSeries, {
      color, lineWidth: 2, lastValueVisible: false, priceLineVisible: false,
      crosshairMarkerVisible: false,
    }, this.paneIndex);
  }

  removePreview(series: ISeriesApi<'Line'>): void { this.chart.removeSeries(series); }

  private renderOne(drawing: DrawingLine): void {
    if (drawing.type === 'horizontal' && drawing.points[0]) {
      const line = this.priceSeries.createPriceLine({
        price: drawing.points[0].price, color: drawing.color, lineWidth: 2,
        axisLabelVisible: true, title: 'Drawing',
      });
      this.refs.set(drawing.id, { type: 'horizontal', lines: [line] });
      return;
    }
    if ((drawing.type === 'trendline' || drawing.type === 'fibonacci') && drawing.points.length >= 2) {
      const points = [...drawing.points].sort((a, b) => timeValue(a.time) - timeValue(b.time));
      const refs: ISeriesApi<'Line'>[] = [];
      if (drawing.type === 'trendline') {
        refs.push(this.addLine(points.map(point => ({ time: point.time, value: point.price })), drawing.color, 2));
      } else {
        refs.push(this.addLine(points.map(point => ({ time: point.time, value: point.price })), 'rgba(224, 86, 253, 0.35)', 1));
        const delta = points[0].price - points[1].price;
        [0, 0.236, 0.382, 0.5, 0.618, 0.786, 1].forEach(level => {
          const value = points[0].price - delta * level;
          refs.push(this.addLine([
            { time: points[0].time, value }, { time: points[1].time, value },
          ], drawing.color, 1));
        });
      }
      this.refs.set(drawing.id, { type: 'series', series: refs });
    }
  }

  private addLine(data: LineData[], color: string, lineWidth: 1 | 2): ISeriesApi<'Line'> {
    const series = this.chart.addSeries(LineSeries, {
      color, lineWidth, crosshairMarkerVisible: false, lastValueVisible: false, priceLineVisible: false,
    }, this.paneIndex);
    series.setData(data);
    return series;
  }
}
