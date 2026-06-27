import React, { useEffect, useRef } from 'react';
import { createChart } from 'lightweight-charts';
import type { IChartApi, Time } from 'lightweight-charts';

interface EquityPoint {
  timestamp: string;
  equity: number;
  drawdown: number;
}

interface EquityChartProps {
  data: EquityPoint[];
}

export const EquityChart: React.FC<EquityChartProps> = ({ data }) => {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);

  useEffect(() => {
    if (!chartContainerRef.current || data.length === 0) return;

    const chart = createChart(chartContainerRef.current, {
      layout: {
        background: { color: 'transparent' },
        textColor: '#F0F6FC',
      },
      grid: {
        vertLines: { color: 'rgba(255,255,255,0.05)' },
        horzLines: { color: 'rgba(255,255,255,0.05)' },
      },
      rightPriceScale: {
        borderColor: 'rgba(255,255,255,0.1)',
      },
      timeScale: {
        borderColor: 'rgba(255,255,255,0.1)',
      },
      crosshair: {
        mode: 1,
      },
    });

    chartRef.current = chart;

    const equitySeries = chart.addAreaSeries({
      lineColor: '#2962FF',
      topColor: 'rgba(41, 98, 255, 0.4)',
      bottomColor: 'rgba(41, 98, 255, 0.0)',
      lineWidth: 2,
    });

    const drawdownSeries = chart.addHistogramSeries({
      color: 'rgba(255, 23, 68, 0.5)',
      priceFormat: {
        type: 'volume',
      },
      priceScaleId: '', // overlay
    });
    drawdownSeries.priceScale().applyOptions({
      scaleMargins: {
        top: 0.8,
        bottom: 0,
      },
    });

    // Format data
    const sortedData = [...data].sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime());
    
    // Add base line
    const chartData = sortedData.map(d => ({
      time: d.timestamp.split('T')[0] as Time,
      value: d.equity
    }));
    
    // De-duplicate times (lightweight-charts requires unique times)
    const uniqueChartData = [];
    const seenTimes = new Set();
    for (const item of chartData) {
      if (!seenTimes.has(item.time)) {
        seenTimes.add(item.time);
        uniqueChartData.push(item);
      }
    }

    const ddData = sortedData.map(d => ({
      time: d.timestamp.split('T')[0] as Time,
      value: d.drawdown,
      color: 'rgba(255, 23, 68, 0.5)'
    }));
    
    const uniqueDdData = [];
    const seenDdTimes = new Set();
    for (const item of ddData) {
      if (!seenDdTimes.has(item.time)) {
        seenDdTimes.add(item.time);
        uniqueDdData.push(item);
      }
    }

    if (uniqueChartData.length > 0) {
      equitySeries.setData(uniqueChartData);
      drawdownSeries.setData(uniqueDdData);
      chart.timeScale().fitContent();
    }

    const handleResize = () => {
      if (chartContainerRef.current) {
        chart.applyOptions({
          width: chartContainerRef.current.clientWidth,
          height: chartContainerRef.current.clientHeight,
        });
      }
    };
    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      chart.remove();
    };
  }, [data]);

  return <div ref={chartContainerRef} style={{ width: '100%', height: '300px' }} />;
};
