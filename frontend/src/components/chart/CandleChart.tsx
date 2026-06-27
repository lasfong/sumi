import { useEffect, useRef, useImperativeHandle, forwardRef } from 'react';
import { createChart } from 'lightweight-charts';
import type { IChartApi, ISeriesApi, Time, LineData, SeriesMarker } from 'lightweight-charts';

export type DrawingType = 'cursor' | 'line' | 'trendline' | 'ray' | 'horizontal' | 'fibonacci';

export interface DrawingLine {
  id: string;
  type: DrawingType;
  points: { time: Time, price: number }[];
  color: string;
}

interface CandleData {
  time: Time;
  open: number;
  high: number;
  low: number;
  close: number;
}

interface VolumeData {
  time: Time;
  value: number;
  color: string;
}

interface CandleChartProps {
  data: CandleData[];
  volumeData?: VolumeData[];
  markers?: SeriesMarker<Time>[];
  drawings?: DrawingLine[];
  activeTool?: DrawingType | null;
  onDrawingComplete?: (drawing: DrawingLine) => void;
}

export interface IndicatorSeriesData {
  name: string;
  data: LineData[];
  color?: string;
  type?: 'line' | 'histogram';
}

export interface CandleChartRef {
  addIndicator: (key: string, seriesList: IndicatorSeriesData[], pane?: 'main' | 'oscillator') => void;
  removeIndicator: (key: string) => void;
  clearIndicators: () => void;
  updateCandle: (candle: CandleData, volume?: VolumeData) => void;
}

export const CandleChart = forwardRef<CandleChartRef, CandleChartProps>(({ data, volumeData, markers, drawings = [], activeTool = null, onDrawingComplete }, ref) => {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const candlestickSeriesRef = useRef<ISeriesApi<"Candlestick"> | null>(null);
  const volumeSeriesRef = useRef<ISeriesApi<"Histogram"> | null>(null);

  // Store multiple series per indicator key
  const indicatorSeriesRef = useRef<{ [key: string]: ISeriesApi<any>[] }>({});
  
  // Track how many oscillators we have to adjust margins
  const numOscillators = useRef<number>(0);

  const updateMargins = () => {
    if (!chartRef.current) return;
    const oscCount = numOscillators.current;
    
    // Main chart margin
    const mainBottom = oscCount > 0 ? (oscCount * 0.2) + 0.1 : 0.15;
    chartRef.current.priceScale('right').applyOptions({
      scaleMargins: { top: 0.1, bottom: mainBottom },
    });
    
    // Volume margin (always below main chart)
    if (volumeSeriesRef.current) {
      volumeSeriesRef.current.priceScale().applyOptions({
        scaleMargins: { top: 1 - mainBottom, bottom: mainBottom - 0.15 },
      });
    }
  };

  useImperativeHandle(ref, () => ({
    addIndicator: (key: string, seriesList: IndicatorSeriesData[], pane: 'main' | 'oscillator' = 'main') => {
      if (!chartRef.current) return;
      
      // Update if exists
      if (indicatorSeriesRef.current[key]) {
        indicatorSeriesRef.current[key].forEach((s, idx) => {
          if (seriesList[idx]) {
            s.setData(seriesList[idx].data);
          }
        });
        return;
      }
      
      const newSeries: ISeriesApi<any>[] = [];
      let priceScaleId = 'right'; // Default main pane
      
      if (pane === 'oscillator') {
        numOscillators.current += 1;
        priceScaleId = key; // Use the key as unique priceScaleId for this oscillator
        
        // Calculate position based on how many oscillators exist
        const index = numOscillators.current - 1;
        const height = 0.2; // 20% of screen per oscillator
        const top = 1 - (index + 1) * height;
        const bottom = index * height;
        
        chartRef.current.priceScale(priceScaleId).applyOptions({
          scaleMargins: { top, bottom },
        });
        
        updateMargins();
      }
      
      seriesList.forEach(sData => {
        let series;
        if (sData.type === 'histogram') {
          series = chartRef.current!.addHistogramSeries({
            color: sData.color || '#2962FF',
            priceScaleId,
          });
        } else {
          series = chartRef.current!.addLineSeries({
            color: sData.color || '#2962FF',
            lineWidth: 2,
            crosshairMarkerVisible: false,
            priceScaleId,
          });
        }
        series.setData(sData.data);
        newSeries.push(series);
      });
      
      indicatorSeriesRef.current[key] = newSeries;
    },
    removeIndicator: (key: string) => {
      if (chartRef.current && indicatorSeriesRef.current[key]) {
        indicatorSeriesRef.current[key].forEach(s => chartRef.current?.removeSeries(s));
        
        // Check if it was an oscillator by checking if its priceScaleId was added
        // Simplification: if we have oscillators, and we remove one, just decrement
        // For a robust app we'd track the pane of each key.
        if (chartRef.current.priceScale(key)) {
           numOscillators.current = Math.max(0, numOscillators.current - 1);
           updateMargins();
        }
        
        delete indicatorSeriesRef.current[key];
      }
    },
    clearIndicators: () => {
      if (!chartRef.current) return;
      Object.values(indicatorSeriesRef.current).forEach(seriesList => {
        seriesList.forEach(s => chartRef.current?.removeSeries(s));
      });
      indicatorSeriesRef.current = {};
      numOscillators.current = 0;
      updateMargins();
    },
    updateCandle: (candle: CandleData, volume?: VolumeData) => {
      if (candlestickSeriesRef.current) {
        candlestickSeriesRef.current.update(candle);
      }
      if (volumeSeriesRef.current && volume) {
        volumeSeriesRef.current.update(volume);
      }
    }
  }));

  useEffect(() => {
    if (!chartContainerRef.current) return;

    const chart = createChart(chartContainerRef.current, {
      layout: {
        background: { color: 'transparent' },
        textColor: '#F0F6FC',
      },
      grid: {
        vertLines: { color: 'rgba(255,255,255,0.05)' },
        horzLines: { color: 'rgba(255,255,255,0.05)' },
      },
      crosshair: {
        mode: 1, // Magnet
      },
      rightPriceScale: {
        borderColor: 'rgba(255,255,255,0.1)',
      },
      timeScale: {
        borderColor: 'rgba(255,255,255,0.1)',
        timeVisible: false,
        shiftVisibleRangeOnNewBar: true,
      },
      width: chartContainerRef.current.clientWidth,
      height: chartContainerRef.current.clientHeight || 500,
    });

    chartRef.current = chart;

    const candlestickSeries = chart.addCandlestickSeries({
      upColor: '#00E676',
      downColor: '#FF1744',
      borderVisible: false,
      wickUpColor: '#00E676',
      wickDownColor: '#FF1744',
    });
    
    candlestickSeriesRef.current = candlestickSeries;

    // Always create volume series as overlay on the main pane
    const volumeSeries = chart.addHistogramSeries({
      color: 'rgba(0, 230, 118, 0.5)',
      priceFormat: {
        type: 'volume',
      },
      priceScaleId: '', // overlay on main chart
    });
    volumeSeries.priceScale().applyOptions({
      scaleMargins: {
        top: 0.8,
        bottom: 0,
      },
    });
    volumeSeriesRef.current = volumeSeries;

    const handleResize = () => {
      if (chartContainerRef.current) {
        chart.applyOptions({
          width: chartContainerRef.current.clientWidth,
          height: chartContainerRef.current.clientHeight,
        });
      }
    };

    const resizeObserver = new ResizeObserver(handleResize);
    resizeObserver.observe(chartContainerRef.current);
    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      resizeObserver.disconnect();
      chart.remove();
    };
  }, []);

  // Handle Drawing Tools Input
  const pendingDrawingRef = useRef<{ type: DrawingType; points: {time: Time, price: number}[]; series?: ISeriesApi<"Line"> } | null>(null);
  const drawingsSeriesRef = useRef<{ [id: string]: any }>({});

  useEffect(() => {
    if (!chartRef.current || !candlestickSeriesRef.current) return;
    const chart = chartRef.current;
    const mainSeries = candlestickSeriesRef.current;

    const getSnappedPrice = (param: any, fallbackPrice: number): number => {
      if (!param.point || !param.seriesData) return fallbackPrice;
      const data = param.seriesData.get(mainSeries) as any;
      if (!data) return fallbackPrice;

      const threshold = 10; // Magnet threshold in pixels
      const mouseY = param.point.y;
      
      const prices = [data.open, data.high, data.low, data.close].filter(p => p !== undefined);
      if (prices.length === 0) return fallbackPrice;
      
      let closestPrice = fallbackPrice;
      let minDistance = Infinity;

      for (const p of prices) {
        const y = mainSeries.priceToCoordinate(p);
        if (y !== null) {
          const dist = Math.abs(y - mouseY);
          if (dist < minDistance && dist <= threshold) {
            minDistance = dist;
            closestPrice = p;
          }
        }
      }

      return closestPrice;
    };

    const handleClick = (param: any) => {
      if (!param.point || !param.time || !activeTool || activeTool === 'cursor') return;
      
      let price = mainSeries.coordinateToPrice(param.point.y) as number | null;
      if (price === null) return;
      price = getSnappedPrice(param, price);

      if (activeTool === 'horizontal') {
        const id = Math.random().toString(36).substr(2, 9);
        const newDrawing: DrawingLine = {
          id,
          type: 'horizontal',
          points: [{ time: param.time, price }],
          color: '#e056fd'
        };
        onDrawingComplete?.(newDrawing);
        return;
      }

      if (activeTool === 'trendline' || activeTool === 'fibonacci') {
        if (!pendingDrawingRef.current) {
          const series = chart.addLineSeries({ color: '#e056fd', lineWidth: 2, lastValueVisible: false, priceLineVisible: false });
          pendingDrawingRef.current = {
            type: activeTool,
            points: [{ time: param.time, price }],
            series
          };
        } else {
          const startPt = pendingDrawingRef.current.points[0];
          const id = Math.random().toString(36).substr(2, 9);
          const newDrawing: DrawingLine = {
            id,
            type: activeTool,
            points: [startPt, { time: param.time, price }],
            color: '#e056fd'
          };
          if (pendingDrawingRef.current.series) chart.removeSeries(pendingDrawingRef.current.series);
          pendingDrawingRef.current = null;
          onDrawingComplete?.(newDrawing);
        }
      }
    };

    const handleMouseMove = (param: any) => {
      if (!param.point || !param.time || !pendingDrawingRef.current || (pendingDrawingRef.current.type !== 'trendline' && pendingDrawingRef.current.type !== 'fibonacci')) return;
      
      let price = mainSeries.coordinateToPrice(param.point.y) as number | null;
      if (price === null) return;
      price = getSnappedPrice(param, price);
      
      const series = pendingDrawingRef.current.series;
      if (series) {
        const start = pendingDrawingRef.current.points[0];
        const current = { time: param.time, value: price };
        
        const startTimestamp = typeof start.time === 'string' ? new Date(start.time).getTime() : start.time as number;
        const currentTimestamp = typeof param.time === 'string' ? new Date(param.time).getTime() : param.time as number;
        
        let data = [];
        if (startTimestamp < currentTimestamp) {
          data = [{ time: start.time, value: start.price }, current];
        } else if (startTimestamp > currentTimestamp) {
          data = [current, { time: start.time, value: start.price }];
        } else {
           data = [{ time: start.time, value: start.price }];
        }
        series.setData(data);
      }
    };

    chart.subscribeClick(handleClick);
    chart.subscribeCrosshairMove(handleMouseMove);

    return () => {
      chart.unsubscribeClick(handleClick);
      chart.unsubscribeCrosshairMove(handleMouseMove);
      if (pendingDrawingRef.current?.series && chartRef.current) {
        try { chartRef.current.removeSeries(pendingDrawingRef.current.series); } catch(e){}
      }
    };
  }, [activeTool, onDrawingComplete]);

  // Render Drawings from Props
  useEffect(() => {
    if (!chartRef.current || !candlestickSeriesRef.current || !drawings) return;
    const chart = chartRef.current;
    const mainSeries = candlestickSeriesRef.current;

    Object.values(drawingsSeriesRef.current).forEach(item => {
      if (item.type === 'horizontal') {
        try { mainSeries.removePriceLine(item.ref); } catch(e){}
      } else if (item.type === 'trendline') {
        try { chart.removeSeries(item.ref); } catch(e){}
      } else if (item.type === 'fibonacci') {
        if (Array.isArray(item.ref)) {
          item.ref.forEach((r: any) => { try { chart.removeSeries(r); } catch(e){} });
        }
      }
    });
    drawingsSeriesRef.current = {};

    drawings.forEach(d => {
      if (d.type === 'horizontal') {
        const line = mainSeries.createPriceLine({
          price: d.points[0].price,
          color: d.color,
          lineStyle: 0,
          lineWidth: 2,
          axisLabelVisible: true,
        });
        drawingsSeriesRef.current[d.id] = { type: 'horizontal', ref: line };
      } else if (d.type === 'trendline') {
        const series = chart.addLineSeries({
          color: d.color,
          lineWidth: 2,
          crosshairMarkerVisible: false,
          lastValueVisible: false,
          priceLineVisible: false
        });
        const sorted = [...d.points].sort((a, b) => {
           const ta = typeof a.time === 'string' ? new Date(a.time).getTime() : a.time as number;
           const tb = typeof b.time === 'string' ? new Date(b.time).getTime() : b.time as number;
           return ta - tb;
        });
        series.setData(sorted.map(pt => ({ time: pt.time, value: pt.price })));
        drawingsSeriesRef.current[d.id] = { type: 'trendline', ref: series };
      } else if (d.type === 'fibonacci' && d.points.length >= 2) {
        // Draw the anchor trendline
        const anchorSeries = chart.addLineSeries({ color: 'rgba(224, 86, 253, 0.3)', lineStyle: 2, lineWidth: 1, crosshairMarkerVisible: false, lastValueVisible: false, priceLineVisible: false });
        const sorted = [...d.points].sort((a, b) => {
           const ta = typeof a.time === 'string' ? new Date(a.time).getTime() : a.time as number;
           const tb = typeof b.time === 'string' ? new Date(b.time).getTime() : b.time as number;
           return ta - tb;
        });
        anchorSeries.setData(sorted.map(pt => ({ time: pt.time, value: pt.price })));
        
        // Draw the levels
        const p1 = d.points[0].price;
        const p2 = d.points[1].price;
        const diff = p1 - p2;
        const levels = [0, 0.236, 0.382, 0.5, 0.618, 0.786, 1];
        
        const refs: any[] = [anchorSeries];
        levels.forEach(lvl => {
          const lvlPrice = p1 - diff * lvl;
          const series = chart.addLineSeries({
            color: d.color,
            lineWidth: 1,
            crosshairMarkerVisible: false,
            lastValueVisible: false,
            priceLineVisible: false
          });
          series.setData([
            { time: sorted[0].time, value: lvlPrice },
            { time: sorted[1].time, value: lvlPrice }
          ]);
          refs.push(series);
        });
        drawingsSeriesRef.current[d.id] = { type: 'fibonacci', ref: refs };
      }
    });
  }, [drawings]);

  useEffect(() => {
    if (candlestickSeriesRef.current && data.length > 0) {
      candlestickSeriesRef.current.setData(data);
      if (markers) {
        candlestickSeriesRef.current.setMarkers(markers);
      }
    }
  }, [data, markers]);

  useEffect(() => {
    if (volumeSeriesRef.current && volumeData && volumeData.length > 0) {
      volumeSeriesRef.current.setData(volumeData);
    }
  }, [volumeData]);

  return <div ref={chartContainerRef} style={{ width: '100%', height: '100%' }} />;
});
