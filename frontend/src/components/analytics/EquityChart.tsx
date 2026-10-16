import React, { useEffect, useRef } from 'react';
import type { Time } from 'lightweight-charts';
import { toDateKey } from '../../utils/date';
import { ChartKernel, type EquityKernelHandle, type EquitySeriesPoint } from '../chart/ChartKernel';

interface EquityPoint {
  timestamp: string;
  equity: number;
  cash: number;
  holdings_value: number;
  drawdown: number;
  drawdown_pct: number;
}

interface EquityChartProps {
  data: EquityPoint[];
}

export const EquityChart: React.FC<EquityChartProps> = ({ data }) => {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const kernelRef = useRef<EquityKernelHandle | null>(null);

  useEffect(() => {
    if (!chartContainerRef.current || data.length === 0) return;

    const kernel = ChartKernel.createEquityKernel(chartContainerRef.current);
    kernelRef.current = kernel;

    const sortedData = [...data].sort(
      (a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime(),
    );

    const chartData: EquitySeriesPoint[] = sortedData.map(d => ({
      time: (toDateKey(d.timestamp) || d.timestamp) as Time,
      value: d.equity,
    }));

    const uniqueChartData: EquitySeriesPoint[] = [];
    const seenTimes = new Set<Time>();
    for (const item of chartData) {
      if (!seenTimes.has(item.time)) {
        seenTimes.add(item.time);
        uniqueChartData.push(item);
      }
    }

    const ddData: EquitySeriesPoint[] = sortedData.map(d => ({
      time: (toDateKey(d.timestamp) || d.timestamp) as Time,
      value: d.drawdown,
      color: 'rgba(255, 23, 68, 0.5)',
    }));

    const uniqueDdData: EquitySeriesPoint[] = [];
    const seenDdTimes = new Set<Time>();
    for (const item of ddData) {
      if (!seenDdTimes.has(item.time)) {
        seenDdTimes.add(item.time);
        uniqueDdData.push(item);
      }
    }

    if (uniqueChartData.length > 0) {
      kernel.setData(uniqueChartData, uniqueDdData);
    }

    const handleResize = () => {
      if (chartContainerRef.current) {
        kernel.resize(
          chartContainerRef.current.clientWidth,
          chartContainerRef.current.clientHeight,
        );
      }
    };
    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      kernel.destroy();
      kernelRef.current = null;
    };
  }, [data]);

  return <div ref={chartContainerRef} style={{ width: '100%', height: '300px' }} />;
};
