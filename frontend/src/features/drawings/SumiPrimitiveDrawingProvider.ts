import type {
  IChartApi, IPrimitivePaneRenderer, IPrimitivePaneView, ISeriesApi, ISeriesPrimitive,
  PrimitiveHoveredItem, SeriesAttachedParameter, Time,
} from 'lightweight-charts';
import type { CanvasRenderingTarget2D } from 'fancy-canvas';
import type { DrawingInteractionSnapshot, DrawingProvider, DrawingProviderEvent } from './DrawingProvider';
import { fibonacciPrice, hitProjectedDrawing, layoutDrawingText, rayEndpoint, rectangleCorners, type ProjectedDrawing, type ScreenPoint } from './drawingGeometry';
import { snapAnchor, type MagnetCandle, type MagnetMode } from './drawingMagnet';
import { createDrawing, isDrawingDate, isRightwardRay, type DrawingTool, type SumiDrawing, type SumiDrawingAnchor, type SumiDrawingDocumentV1 } from './drawingDomain';

const timeKey = (time: Time | null): string | null => {
  if (time === null) return null;
  if (typeof time === 'string') return time.slice(0, 10);
  if (typeof time === 'number') return new Date(time * 1000).toISOString().slice(0, 10);
  return `${time.year}-${String(time.month).padStart(2, '0')}-${String(time.day).padStart(2, '0')}`;
};
const dash = (style: SumiDrawing['style']['lineStyle']) => style === 'dashed' ? [8, 5] : style === 'dotted' ? [2, 4] : [];

class SumiDrawingDocumentPrimitive implements ISeriesPrimitive<Time> {
  private requestUpdate: (() => void) | null = null;
  private readonly series: ISeriesApi<'Candlestick'>; private readonly chart: IChartApi;
  private document: SumiDrawingDocumentV1; private selectedIds: string[]; private preview: SumiDrawing | null;
  constructor(series: ISeriesApi<'Candlestick'>, chart: IChartApi, document: SumiDrawingDocumentV1, selectedIds: string[], preview: SumiDrawing | null) {
    this.series = series; this.chart = chart; this.document = document; this.selectedIds = selectedIds; this.preview = preview;
  }
  attached(param: SeriesAttachedParameter<Time>): void { this.requestUpdate = param.requestUpdate; }
  detached(): void { this.requestUpdate = null; }
  replace(document: SumiDrawingDocumentV1, selectedIds: string[], preview: SumiDrawing | null): void {
    this.document = document; this.selectedIds = selectedIds; this.preview = preview; this.requestUpdate?.();
  }
  private project(drawing: SumiDrawing): ProjectedDrawing | null {
    const anchors = drawing.anchors.map(anchor => ({
      x: this.chart.timeScale().timeToCoordinate(anchor.time as Time), y: this.series.priceToCoordinate(anchor.price),
    }));
    return anchors.every(anchor => anchor.x !== null && anchor.y !== null)
      ? { drawing, anchors: anchors as ScreenPoint[] } : null;
  }
  paneViews(): readonly IPrimitivePaneView[] {
    const renderer: IPrimitivePaneRenderer = { draw: (target: CanvasRenderingTarget2D) => target.useMediaCoordinateSpace(({ context, mediaSize }) => {
      const items = [...this.document.drawings.filter(drawing => drawing.visible), ...(this.preview ? [this.preview] : [])];
      for (const drawing of items) {
        const projected = this.project(drawing); if (!projected) continue;
        const [a, b] = projected.anchors; const selected = this.selectedIds.includes(drawing.id); const preview = drawing === this.preview;
        context.save(); context.strokeStyle = drawing.style.lineColor; context.fillStyle = drawing.style.fillColor ?? drawing.style.lineColor;
        context.lineWidth = drawing.style.lineWidth; context.setLineDash(preview ? [5, 4] : dash(drawing.style.lineStyle));
        if (drawing.tool === 'horizontal') { context.beginPath(); context.moveTo(0, a.y); context.lineTo(mediaSize.width, a.y); context.stroke(); }
        if (drawing.tool === 'trendline' || drawing.tool === 'ray') {
          const end = drawing.tool === 'ray' ? rayEndpoint(a, b, mediaSize.width) : b;
          context.beginPath(); context.moveTo(a.x, a.y); context.lineTo(end.x, end.y); context.stroke();
        }
        if (drawing.tool === 'rectangle') {
          const x = Math.min(a.x, b.x); const y = Math.min(a.y, b.y); const width = Math.abs(a.x - b.x); const height = Math.abs(a.y - b.y);
          context.save(); context.globalAlpha = drawing.style.fillOpacity ?? 0.12; context.fillRect(x, y, width, height); context.restore(); context.strokeRect(x, y, width, height);
        }
        if (drawing.tool === 'fibonacci-retracement') {
          for (const level of drawing.geometry.levels.filter(level => level.visible)) {
            const price = fibonacciPrice(drawing.anchors, level.ratio, drawing.geometry.direction);
            const y = this.series.priceToCoordinate(price); if (y === null) continue;
            context.strokeStyle = level.color ?? drawing.style.lineColor; context.beginPath(); context.moveTo(Math.min(a.x, b.x), y); context.lineTo(mediaSize.width, y); context.stroke();
            context.fillStyle = drawing.style.textColor ?? '#F0F6FC'; context.font = `${drawing.style.fontSize ?? 12}px sans-serif`;
            context.fillText(`${level.ratio.toFixed(3).replace(/0+$/, '').replace(/\.$/, '')}  ${price.toFixed(2)}`, Math.min(a.x, b.x) + 5, y - 3);
          }
        }
        if (drawing.tool === 'risk-reward' && projected.anchors.length === 3) {
          const [entryPt, stopPt, targetPt] = projected.anchors;
          const minX = Math.min(entryPt.x, stopPt.x, targetPt.x);
          const maxX = Math.max(entryPt.x, stopPt.x, targetPt.x, mediaSize.width);
          const width = Math.max(10, maxX - minX);

          // 1. Target (profit) zone
          const targetTop = Math.min(entryPt.y, targetPt.y);
          const targetHeight = Math.abs(entryPt.y - targetPt.y);
          context.save();
          context.fillStyle = 'rgba(38, 166, 154, 0.18)';
          context.fillRect(minX, targetTop, width, targetHeight);
          context.strokeStyle = '#26A69A';
          context.lineWidth = 1.5;
          context.strokeRect(minX, targetTop, width, targetHeight);
          context.restore();

          // 2. Stop (loss) zone
          const stopTop = Math.min(entryPt.y, stopPt.y);
          const stopHeight = Math.abs(entryPt.y - stopPt.y);
          context.save();
          context.fillStyle = 'rgba(239, 83, 80, 0.18)';
          context.fillRect(minX, stopTop, width, stopHeight);
          context.strokeStyle = '#EF5350';
          context.lineWidth = 1.5;
          context.strokeRect(minX, stopTop, width, stopHeight);
          context.restore();

          // 3. Entry line
          context.save();
          context.strokeStyle = '#58A6FF';
          context.lineWidth = 2;
          context.setLineDash([4, 4]);
          context.beginPath();
          context.moveTo(minX, entryPt.y);
          context.lineTo(minX + width, entryPt.y);
          context.stroke();
          context.restore();

          // 4. Text labels
          context.fillStyle = '#F0F6FC';
          context.font = '11px sans-serif';
          const targetPrice = drawing.anchors[2].price;
          const entryPrice = drawing.anchors[0].price;
          const stopPrice = drawing.anchors[1].price;
          const ratio = drawing.geometry.riskRewardRatio;
          context.fillText(`Target: ${targetPrice.toFixed(2)}`, minX + 8, targetPt.y + (targetPt.y < entryPt.y ? 14 : -4));
          context.fillText(`Entry: ${entryPrice.toFixed(2)} (R:R ${ratio.toFixed(2)})`, minX + 8, entryPt.y - 4);
          context.fillText(`Stop: ${stopPrice.toFixed(2)}`, minX + 8, stopPt.y + (stopPt.y > entryPt.y ? -4 : 14));
        }
        if (drawing.tool === 'text') {
          context.fillStyle = drawing.style.textColor ?? drawing.style.lineColor; context.font = `${drawing.style.fontSize ?? 14}px sans-serif`;
          const layout = layoutDrawingText(drawing.geometry.text, a, drawing.style.fontSize ?? 14);
          layout.lines.forEach(line => context.fillText(line.text, line.x, line.y));
        }
        if (selected) {
          context.strokeStyle = '#2962ff'; context.fillStyle = '#ffffff'; context.lineWidth = 2; context.setLineDash([]);
          const handles = drawing.tool === 'rectangle' ? rectangleCorners(projected.anchors) : projected.anchors;
          handles.forEach(anchor => { context.beginPath(); context.arc(anchor.x, anchor.y, 6, 0, Math.PI * 2); context.fill(); context.stroke(); });
          if (drawing.tool === 'rectangle') context.strokeRect(Math.min(a.x, b.x), Math.min(a.y, b.y), Math.abs(a.x - b.x), Math.abs(a.y - b.y));
          if (drawing.tool === 'text') { const bounds = layoutDrawingText(drawing.geometry.text, a, drawing.style.fontSize ?? 14).bounds;
            context.strokeRect(bounds.left, bounds.top, bounds.right - bounds.left, bounds.bottom - bounds.top); }
        }
        context.restore();
      }
    }) };
    return [{ zOrder: () => 'top', renderer: () => renderer }];
  }
  hitTest(x: number, y: number): PrimitiveHoveredItem | null {
    const width = this.chart.paneSize().width;
    for (const drawing of [...this.document.drawings].reverse()) {
      if (!drawing.visible || drawing.locked) continue;
      const projected = this.project(drawing); if (!projected) continue;
      const hit = hitProjectedDrawing(projected, { x, y }, width);
      if (hit) return { externalId: `${drawing.id}:${hit.part}`, distance: hit.distance, hitTestPriority: hit.part.startsWith('anchor') ? 2 : 1, cursorStyle: hit.part === 'body' ? 'move' : 'crosshair', zOrder: 'top' };
    }
    return null;
  }
}

export class SumiPrimitiveDrawingProvider implements DrawingProvider {
  private tool: DrawingTool = 'select'; private document: SumiDrawingDocumentV1; private selectedIds: string[] = [];
  private listeners = new Set<(event: DrawingProviderEvent) => void>(); private primitive: SumiDrawingDocumentPrimitive;
  private destroyed = false; private preview: SumiDrawing | null = null;
  private creationAnchor: SumiDrawingAnchor | null = null;
  private creationAnchors: SumiDrawingAnchor[] = [];
  private magnetMode: MagnetMode = 'off'; private candles: MagnetCandle[] = [];
  private drag: { drawing: SumiDrawing; pointerId: number; part: string; start: ScreenPoint } | null = null;
  private readonly chart: IChartApi; private readonly series: ISeriesApi<'Candlestick'>; private readonly container: HTMLElement;
  private readonly getPricePaneElement: () => HTMLElement | null;
  constructor(chart: IChartApi, series: ISeriesApi<'Candlestick'>, container: HTMLElement,
    document: SumiDrawingDocumentV1, _currentTime: () => string, getPricePaneElement: () => HTMLElement | null) {
    this.chart = chart; this.series = series; this.container = container; this.getPricePaneElement = getPricePaneElement;
    this.document = structuredClone(document); this.primitive = new SumiDrawingDocumentPrimitive(series, chart, this.document, this.selectedIds, null);
    series.attachPrimitive(this.primitive);
    container.addEventListener('pointerdown', this.onPointerDown); container.addEventListener('pointermove', this.onPointerMove);
    container.addEventListener('pointerup', this.onPointerUp); container.addEventListener('pointercancel', this.onPointerCancel);
    container.addEventListener('lostpointercapture', this.onLostPointerCapture);
    container.addEventListener('sumi:drawing-snapshot-request', this.onSnapshotRequest); this.publishSnapshot();
  }
  setTool(tool: DrawingTool): void { if (tool !== this.tool) this.clearTransaction(true); this.tool = tool; if (tool !== 'select') this.select([]); this.redraw(); }
  setMagnet(mode: MagnetMode, candles: MagnetCandle[]): void { this.magnetMode = mode; this.candles = structuredClone(candles); this.publishSnapshot(); }
  cancel(): void { if (this.destroyed) return; const previous = this.tool; this.clearTransaction(true); this.tool = 'select'; this.emit({ type: 'cancelled', tool: previous }); this.redraw(); }
  select(ids: string[]): void { const next = ids.filter(id => this.document.drawings.some(d => d.id === id)); if (JSON.stringify(next) === JSON.stringify(this.selectedIds)) return; this.selectedIds = next; this.redraw(); this.emit({ type: 'selection-changed', drawingIds: [...next] }); }
  replaceDocument(document: SumiDrawingDocumentV1): void { this.document = structuredClone(document); this.selectedIds = this.selectedIds.filter(id => document.drawings.some(d => d.id === id)); this.redraw(); }
  snapshotInteraction(): DrawingInteractionSnapshot {
    const rect = this.getPricePaneElement()?.getBoundingClientRect();
    return { tool: this.tool, selectedIds: [...this.selectedIds], pricePane: rect ? { left: rect.left, top: rect.top, width: rect.width, height: rect.height } : null,
      drawings: this.document.drawings.map(drawing => { const anchors = drawing.anchors.map(anchor => ({ ...anchor, x: this.chart.timeScale().timeToCoordinate(anchor.time as Time), y: this.series.priceToCoordinate(anchor.price) }));
        const handles = drawing.tool === 'rectangle' && anchors.every(anchor => anchor.x !== null && anchor.y !== null)
          ? rectangleCorners(anchors as ScreenPoint[]).map((point, index) => ({ part: `corner:${index}`, ...point }))
          : anchors.map((anchor, index) => ({ part: `anchor:${index}`, x: anchor.x, y: anchor.y }));
        const bounds = drawing.tool === 'text' && anchors[0].x !== null && anchors[0].y !== null
          ? layoutDrawingText(drawing.geometry.text, anchors[0] as ScreenPoint, drawing.style.fontSize ?? 14).bounds : null;
        return { id: drawing.id, tool: drawing.tool, price: drawing.anchors[0].price, coordinate: this.series.priceToCoordinate(drawing.anchors[0].price), visible: drawing.visible,
          providerVisible: drawing.visible && anchors.every(anchor => anchor.x !== null && anchor.y !== null), selected: this.selectedIds.includes(drawing.id), geometry: structuredClone(drawing.geometry), anchors, handles, bounds }; }),
      preview: this.preview ? { tool: this.preview.tool, anchors: structuredClone(this.preview.anchors) } : this.creationAnchors.length > 0 ? { tool: this.tool, anchors: structuredClone(this.creationAnchors) } : this.creationAnchor ? { tool: this.tool, anchors: [structuredClone(this.creationAnchor)] } : null,
      magnet: { mode: this.magnetMode, threshold: 10, visibleCandles: this.candles.map(candle => ({ time: candle.time, x: this.chart.timeScale().timeToCoordinate(candle.time as Time), prices: (['open', 'high', 'low', 'close'] as const).map(field => ({ field, price: candle[field], y: this.series.priceToCoordinate(candle[field]) })) })) },
      dragging: this.drag !== null, dragPart: this.drag?.part ?? null, primitiveCount: this.destroyed ? 0 : 1, listenerCount: this.destroyed ? 0 : 6 };
  }
  subscribe(listener: (event: DrawingProviderEvent) => void): () => void { this.listeners.add(listener); return () => this.listeners.delete(listener); }
  destroy(): void { if (this.destroyed) return; this.clearTransaction(true); this.destroyed = true; this.listeners.clear();
    this.container.removeEventListener('pointerdown', this.onPointerDown); this.container.removeEventListener('pointermove', this.onPointerMove);
    this.container.removeEventListener('pointerup', this.onPointerUp); this.container.removeEventListener('pointercancel', this.onPointerCancel);
    this.container.removeEventListener('lostpointercapture', this.onLostPointerCapture);
    this.container.removeEventListener('sumi:drawing-snapshot-request', this.onSnapshotRequest); this.series.detachPrimitive(this.primitive); delete this.container.dataset.drawingInteractionState; }
  private emit(event: DrawingProviderEvent): void { this.listeners.forEach(listener => listener(event)); }
  private redraw(): void { this.primitive.replace(this.document, this.selectedIds, this.preview); this.publishSnapshot(); }
  private publishSnapshot(): void { if (!this.destroyed) this.container.dataset.drawingInteractionState = JSON.stringify(this.snapshotInteraction()); }
  private onSnapshotRequest = () => this.publishSnapshot();
  private releaseCapture(pointerId: number): void { try { if (!this.container.hasPointerCapture || this.container.hasPointerCapture(pointerId)) this.container.releasePointerCapture?.(pointerId); } catch { /* already released */ } }
  private clearTransaction(rollback: boolean): void { if (this.drag) { const { drawing, pointerId } = this.drag; this.drag = null;
      if (rollback) { this.document = { ...this.document, drawings: this.document.drawings.map(item => item.id === drawing.id ? structuredClone(drawing) : item) }; this.emit({ type: 'change-preview', drawings: [structuredClone(drawing)] }); }
      this.releaseCapture(pointerId); }
    this.preview = null; this.creationAnchor = null; this.creationAnchors = []; this.chart.applyOptions({ handleScroll: true }); }
  private panePoint(event: PointerEvent): ScreenPoint | null { const rect = this.getPricePaneElement()?.getBoundingClientRect(); if (!rect || event.clientX < rect.left || event.clientX > rect.right || event.clientY < rect.top || event.clientY > rect.bottom) return null; return { x: event.clientX - rect.left, y: event.clientY - rect.top }; }
  private anchorAt(point: ScreenPoint): SumiDrawingAnchor | null { const price = this.series.coordinateToPrice(point.y); const time = timeKey(this.chart.timeScale().coordinateToTime(point.x)); if (price === null || !Number.isFinite(price) || price <= 0 || !time || !isDrawingDate(time)) return null;
    const raw = { time, price }; return snapAnchor(raw, point, this.candles, { timeToX: value => this.chart.timeScale().timeToCoordinate(value as Time), priceToY: value => this.series.priceToCoordinate(value) }, this.magnetMode); }
  private projected(drawing: SumiDrawing): ProjectedDrawing | null { const anchors = drawing.anchors.map(anchor => ({ x: this.chart.timeScale().timeToCoordinate(anchor.time as Time), y: this.series.priceToCoordinate(anchor.price) })); return anchors.every(anchor => anchor.x !== null && anchor.y !== null) ? { drawing, anchors: anchors as ScreenPoint[] } : null; }
  private hit(point: ScreenPoint): { drawing: SumiDrawing; part: string } | null { const width = this.chart.paneSize().width; for (const drawing of [...this.document.drawings].reverse()) { if (!drawing.visible || drawing.locked) continue; const projected = this.projected(drawing); const result = projected && hitProjectedDrawing(projected, point, width); if (result) return { drawing, part: result.part }; } return null; }
  private translateBody(original: SumiDrawing, point: ScreenPoint): SumiDrawingAnchor[] | null {
    const projected = this.projected(original); if (!projected) return null;
    const dx = point.x - this.drag!.start.x; const dy = point.y - this.drag!.start.y;
    const failure = this.container.dataset.sumiDrawingBodyConversionFailure ?? '';
    const converted = projected.anchors.map((anchor, index) => {
      const rawPrice = failure === `price:${index}` ? null : this.series.coordinateToPrice(anchor.y + dy);
      const rawTime = failure === `time:${index}` ? null : this.chart.timeScale().coordinateToTime(anchor.x + dx);
      const time = timeKey(rawTime); const price = rawPrice;
      return price !== null && Number.isFinite(price) && price > 0 && time && isDrawingDate(time) ? { time, price } : null;
    });
    if (converted.some(anchor => anchor === null)) return null;
    const anchors = converted as SumiDrawingAnchor[];
    if (anchors.length < 2) return anchors;
    const candleIndex = new Map(this.candles.map((candle, index) => [candle.time, index]));
    const logicalDeltas = anchors.map((anchor, index) => {
      const beforeIndex = candleIndex.get(original.anchors[index].time); const afterIndex = candleIndex.get(anchor.time);
      return beforeIndex === undefined || afterIndex === undefined ? null : afterIndex - beforeIndex;
    });
    if (logicalDeltas.some(delta => delta === null) || !logicalDeltas.every(delta => delta === logicalDeltas[0])) return null;
    const priceDeltas = anchors.map((anchor, index) => anchor.price - original.anchors[index].price);
    if (!priceDeltas.every(delta => Number.isFinite(delta)) || !priceDeltas.every(delta => Math.abs(delta - priceDeltas[0]) <= 1e-7)) return null;
    const priceDelta = priceDeltas[0];
    return anchors.map((anchor, index) => ({ time: anchor.time, price: original.anchors[index].price + priceDelta }));
  }
  private rejectDrag(): void {
    if (!this.drag) return; const { drawing, pointerId } = this.drag; this.drag = null;
    this.document = { ...this.document, drawings: this.document.drawings.map(item => item.id === drawing.id ? structuredClone(drawing) : item) };
    this.emit({ type: 'change-preview', drawings: [structuredClone(drawing)] }); this.releaseCapture(pointerId);
    this.chart.applyOptions({ handleScroll: true }); this.redraw();
  }
  private updateDrag(point: ScreenPoint): SumiDrawing | null { if (!this.drag) return null; const original = this.drag.drawing; let anchors: SumiDrawingAnchor[];
    if (this.drag.part.startsWith('anchor:')) { const index = Number(this.drag.part.split(':')[1]); const anchor = this.anchorAt(point); if (!anchor) return null; anchors = original.anchors.map((item, itemIndex) => itemIndex === index ? anchor : item); }
    else if (this.drag.part.startsWith('corner:') && original.tool === 'rectangle') { const index = Number(this.drag.part.split(':')[1]); const anchor = this.anchorAt(point); if (!anchor) return null;
      const [a, b] = original.anchors; anchors = index === 0 ? [anchor, b] : index === 1 ? [{ ...a, price: anchor.price }, { ...b, time: anchor.time }]
        : index === 2 ? [a, anchor] : [{ ...a, time: anchor.time }, { ...b, price: anchor.price }]; }
    else { const translated = this.translateBody(original, point); if (!translated) return null; anchors = translated; }
    if (original.tool === 'ray' && !isRightwardRay(anchors)) return null;
    return { ...original, anchors } as SumiDrawing;
  }
  private onPointerDown = (event: PointerEvent): void => { if (event.button !== 0 || this.destroyed) return; const point = this.panePoint(event); if (!point) return; const anchor = this.anchorAt(point); if (!anchor) return;
    if (this.tool !== 'select') { event.preventDefault();
      if (this.tool === 'horizontal') { const drawing = createDrawing(this.tool, [anchor], this.document.drawings.length); this.emit({ type: 'created', drawing }); this.tool = 'select'; this.redraw(); return; }
      if (this.tool === 'text') { this.emit({ type: 'text-placement-requested', anchor }); this.tool = 'select'; this.redraw(); return; }
      if (this.tool === 'risk-reward') {
        if (this.creationAnchors.length === 0) { this.creationAnchors = [anchor]; this.chart.applyOptions({ handleScroll: false }); this.redraw(); return; }
        if (this.creationAnchors.length === 1) { this.creationAnchors.push(anchor); this.redraw(); return; }
        const drawing = createDrawing(this.tool, [this.creationAnchors[0], this.creationAnchors[1], anchor], this.document.drawings.length);
        this.creationAnchors = []; this.creationAnchor = null; this.preview = null; this.chart.applyOptions({ handleScroll: true });
        this.emit({ type: 'created', drawing }); this.tool = 'select'; this.redraw(); return;
      }
      if (!this.creationAnchor) { this.creationAnchor = anchor; this.chart.applyOptions({ handleScroll: false }); this.redraw(); return; }
      if (this.tool === 'ray' && !isRightwardRay([this.creationAnchor, anchor])) { this.preview = null; this.redraw(); return; }
      const drawing = createDrawing(this.tool, [this.creationAnchor, anchor], this.document.drawings.length); this.creationAnchor = null; this.preview = null; this.chart.applyOptions({ handleScroll: true }); this.emit({ type: 'created', drawing }); this.tool = 'select'; this.redraw(); return; }
    const hit = this.hit(point); if (!hit) { this.select([]); return; } event.preventDefault(); this.select([hit.drawing.id]);
    this.drag = { drawing: structuredClone(hit.drawing), pointerId: event.pointerId, part: hit.part, start: point }; this.container.setPointerCapture?.(event.pointerId); this.chart.applyOptions({ handleScroll: false }); this.emit({ type: 'change-started', drawingIds: [hit.drawing.id] }); this.publishSnapshot(); };
  private onPointerMove = (event: PointerEvent): void => { if (this.destroyed) return; const point = this.panePoint(event); if (!point) return;
    if (this.tool === 'risk-reward' && this.creationAnchors.length > 0) { const anchor = this.anchorAt(point); if (!anchor) return;
      if (this.creationAnchors.length === 1) {
        const defaultTarget = { time: anchor.time, price: this.creationAnchors[0].price + Math.abs(this.creationAnchors[0].price - anchor.price) * 2 };
        this.preview = createDrawing(this.tool, [this.creationAnchors[0], anchor, defaultTarget], this.document.drawings.length);
      } else {
        this.preview = createDrawing(this.tool, [this.creationAnchors[0], this.creationAnchors[1], anchor], this.document.drawings.length);
      }
      this.redraw(); return; }
    if (this.creationAnchor && this.tool !== 'select' && this.tool !== 'horizontal' && this.tool !== 'text' && this.tool !== 'risk-reward') { const anchor = this.anchorAt(point); if (!anchor) return;
      if (this.tool === 'ray' && !isRightwardRay([this.creationAnchor, anchor])) { this.preview = null; this.redraw(); return; }
      this.preview = createDrawing(this.tool, [this.creationAnchor, anchor], this.document.drawings.length); this.redraw(); return; }
    if (!this.drag || event.pointerId !== this.drag.pointerId) return; const part = this.drag.part; const changed = this.updateDrag(point);
    if (!changed) { if (part === 'body') this.rejectDrag(); return; }
    this.document = { ...this.document, drawings: this.document.drawings.map(drawing => drawing.id === changed.id ? changed : drawing) }; this.redraw(); this.emit({ type: 'change-preview', drawings: [structuredClone(changed)] }); };
  private onPointerUp = (event: PointerEvent): void => { if (!this.drag || event.pointerId !== this.drag.pointerId) return; const before = this.drag.drawing; const after = this.document.drawings.find(drawing => drawing.id === before.id) ?? before; this.drag = null; this.releaseCapture(event.pointerId); this.chart.applyOptions({ handleScroll: true }); if (JSON.stringify(after.anchors) !== JSON.stringify(before.anchors)) this.emit({ type: 'change-committed', before: [before], after: [structuredClone(after)] }); this.publishSnapshot(); };
  private onPointerCancel = (event: PointerEvent): void => { if (this.destroyed) return; if (this.drag && event.pointerId === this.drag.pointerId) { this.clearTransaction(true); this.redraw(); } else if (this.creationAnchor || this.creationAnchors.length > 0) { const previous = this.tool; this.clearTransaction(false); this.tool = 'select'; this.emit({ type: 'cancelled', tool: previous }); this.redraw(); } };
  private onLostPointerCapture = (event: PointerEvent): void => { if (this.destroyed || !this.drag || event.pointerId !== this.drag.pointerId) return; this.clearTransaction(true); this.redraw(); };
}
