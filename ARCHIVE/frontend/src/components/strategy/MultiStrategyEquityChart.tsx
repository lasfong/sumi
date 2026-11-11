import React, { useState, useMemo } from 'react';

export interface StrategyEquitySeries {
  filename: string;
  name: string;
  color: string;
  points: Array<{
    timestamp: string;
    returnPct: number;
    equity: number;
  }>;
}

interface MultiStrategyEquityChartProps {
  series: StrategyEquitySeries[];
  height?: number;
}

export const MultiStrategyEquityChart: React.FC<MultiStrategyEquityChartProps> = ({
  series,
  height = 340,
}) => {
  const [hoverIndex, setHoverIndex] = useState<number | null>(null);

  // Filter series with valid points
  const activeSeries = useMemo(() => series.filter(s => s.points.length > 0), [series]);

  // Find overall min and max return %
  const { minReturn, maxReturn, samplePoints } = useMemo(() => {
    let min = 0;
    let max = 0;
    let longestSeries: StrategyEquitySeries | null = null;

    activeSeries.forEach((s) => {
      if (!longestSeries || s.points.length > longestSeries.points.length) {
        longestSeries = s;
      }
      s.points.forEach((p) => {
        if (p.returnPct < min) min = p.returnPct;
        if (p.returnPct > max) max = p.returnPct;
      });
    });

    // Add 10% breathing room
    const padding = Math.max(5, (max - min) * 0.1);
    return {
      minReturn: Math.floor(min - padding),
      maxReturn: Math.ceil(max + padding),
      samplePoints: longestSeries ? (longestSeries as StrategyEquitySeries).points : [],
    };
  }, [activeSeries]);

  if (activeSeries.length === 0 || samplePoints.length === 0) {
    return (
      <div
        data-testid="multi-strategy-chart-empty"
        style={{
          height,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          background: 'rgba(13, 17, 23, 0.6)',
          border: '1px dashed var(--border-color)',
          borderRadius: '8px',
          color: 'var(--text-muted)',
          fontSize: '13px',
        }}
      >
        Chưa có dữ liệu đường cong lợi nhuận (Equity Curve) để hiển thị.
      </div>
    );
  }

  const svgWidth = 800;
  const svgHeight = height;
  const paddingLeft = 55;
  const paddingRight = 25;
  const paddingTop = 25;
  const paddingBottom = 40;

  const chartWidth = svgWidth - paddingLeft - paddingRight;
  const chartHeight = svgHeight - paddingTop - paddingBottom;

  const getY = (val: number) => {
    const range = maxReturn - minReturn || 1;
    return paddingTop + chartHeight - ((val - minReturn) / range) * chartHeight;
  };

  const zeroY = getY(0);

  // Generate SVG path for a series
  const getPath = (points: StrategyEquitySeries['points']) => {
    if (points.length === 0) return '';
    const step = chartWidth / Math.max(1, points.length - 1);
    return points.reduce((acc, p, i) => {
      const x = paddingLeft + i * step;
      const y = getY(p.returnPct);
      return `${acc} ${i === 0 ? 'M' : 'L'} ${x.toFixed(1)},${y.toFixed(1)}`;
    }, '');
  };

  // Y-axis grid values
  const yTicks = [
    maxReturn,
    Math.round((maxReturn * 2 + minReturn) / 3),
    Math.round((maxReturn + minReturn * 2) / 3),
    minReturn,
  ];

  // Date labels
  const startDate = samplePoints[0]?.timestamp?.slice(0, 10) || '';
  const midDate = samplePoints[Math.floor(samplePoints.length / 2)]?.timestamp?.slice(0, 10) || '';
  const endDate = samplePoints[samplePoints.length - 1]?.timestamp?.slice(0, 10) || '';

  const handleMouseMove = (e: React.MouseEvent<SVGSVGElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const mouseX = e.clientX - rect.left;
    const chartX = mouseX - (paddingLeft / svgWidth) * rect.width;
    const effectiveChartWidth = (chartWidth / svgWidth) * rect.width;
    const ratio = Math.max(0, Math.min(1, chartX / effectiveChartWidth));
    const idx = Math.min(samplePoints.length - 1, Math.round(ratio * (samplePoints.length - 1)));
    setHoverIndex(idx);
  };

  const activeHoverDate = hoverIndex !== null ? samplePoints[hoverIndex]?.timestamp?.slice(0, 10) : null;
  const hoverX = hoverIndex !== null ? paddingLeft + (hoverIndex / Math.max(1, samplePoints.length - 1)) * chartWidth : null;

  return (
    <div
      data-testid="multi-strategy-equity-chart"
      className="panel"
      style={{
        padding: '16px',
        background: 'rgba(13, 17, 23, 0.85)',
        border: '1px solid rgba(88, 166, 255, 0.2)',
        borderRadius: '8px',
        marginBottom: '20px',
      }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '12px' }}>
        <div>
          <h4 style={{ margin: 0, fontSize: '15px', color: 'var(--text-main)', display: 'flex', alignItems: 'center', gap: '8px' }}>
            <span>📈</span>
            <span>So Sánh Tăng Trưởng Vốn (%) — Multi-Strategy Equity Curve</span>
          </h4>
          <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>
            Chuẩn hóa từ mốc 0% để so sánh trực diện hiệu quả sinh lời và độ sụt giảm (Drawdown) qua từng thời kỳ
          </span>
        </div>

        {/* Legend */}
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '14px', alignItems: 'center' }}>
          {activeSeries.map((s) => (
            <div key={s.filename} style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '12px' }}>
              <span style={{ width: '10px', height: '10px', borderRadius: '50%', background: s.color }}></span>
              <span style={{ color: 'var(--text-main)', fontWeight: 500 }}>{s.name}</span>
            </div>
          ))}
        </div>
      </div>

      <div style={{ position: 'relative', width: '100%' }}>
        <svg
          viewBox={`0 0 ${svgWidth} ${svgHeight}`}
          style={{ width: '100%', height: 'auto', display: 'block', overflow: 'visible', cursor: 'crosshair' }}
          onMouseMove={handleMouseMove}
          onMouseLeave={() => setHoverIndex(null)}
        >
          {/* Background Grid & Ticks */}
          {yTicks.map((tick) => {
            const y = getY(tick);
            return (
              <g key={tick}>
                <line
                  x1={paddingLeft}
                  y1={y}
                  x2={svgWidth - paddingRight}
                  y2={y}
                  stroke="rgba(255, 255, 255, 0.07)"
                  strokeDasharray="2,2"
                />
                <text
                  x={paddingLeft - 8}
                  y={y + 4}
                  fill="var(--text-muted)"
                  fontSize="11px"
                  textAnchor="end"
                  fontFamily="monospace"
                >
                  {tick > 0 ? `+${tick}%` : `${tick}%`}
                </text>
              </g>
            );
          })}

          {/* Zero baseline */}
          {zeroY >= paddingTop && zeroY <= paddingTop + chartHeight && (
            <line
              x1={paddingLeft}
              y1={zeroY}
              x2={svgWidth - paddingRight}
              y2={zeroY}
              stroke="rgba(255, 255, 255, 0.3)"
              strokeWidth="1.2"
              strokeDasharray="4,3"
            />
          )}

          {/* Date Ticks */}
          <line
            x1={paddingLeft}
            y1={paddingTop + chartHeight}
            x2={svgWidth - paddingRight}
            y2={paddingTop + chartHeight}
            stroke="rgba(255, 255, 255, 0.15)"
          />
          <text x={paddingLeft} y={svgHeight - 12} fill="var(--text-muted)" fontSize="11px" textAnchor="start">
            {startDate}
          </text>
          <text x={paddingLeft + chartWidth / 2} y={svgHeight - 12} fill="var(--text-muted)" fontSize="11px" textAnchor="middle">
            {midDate}
          </text>
          <text x={svgWidth - paddingRight} y={svgHeight - 12} fill="var(--text-muted)" fontSize="11px" textAnchor="end">
            {endDate}
          </text>

          {/* Strategy Equity Lines */}
          {activeSeries.map((s) => (
            <path
              key={s.filename}
              d={getPath(s.points)}
              fill="none"
              stroke={s.color}
              strokeWidth="2.2"
              strokeLinecap="round"
              strokeLinejoin="round"
              style={{ transition: 'stroke-width 0.15s ease' }}
            />
          ))}

          {/* Hover Crosshair & Tooltip */}
          {hoverX !== null && (
            <>
              <line
                x1={hoverX}
                y1={paddingTop}
                x2={hoverX}
                y2={paddingTop + chartHeight}
                stroke="#58A6FF"
                strokeWidth="1"
                strokeDasharray="3,3"
              />
              {activeSeries.map((s) => {
                const p = s.points[hoverIndex!];
                if (!p) return null;
                const ptY = getY(p.returnPct);
                return (
                  <circle
                    key={s.filename}
                    cx={hoverX}
                    cy={ptY}
                    r="4"
                    fill={s.color}
                    stroke="#0D1117"
                    strokeWidth="2"
                  />
                );
              })}
            </>
          )}
        </svg>

        {/* Floating Tooltip info */}
        {hoverIndex !== null && activeHoverDate && (
          <div
            style={{
              position: 'absolute',
              top: '10px',
              left: `${Math.min(75, Math.max(15, (hoverX! / svgWidth) * 100))}%`,
              transform: 'translateX(-50%)',
              background: 'rgba(22, 27, 34, 0.95)',
              border: '1px solid rgba(88, 166, 255, 0.4)',
              borderRadius: '6px',
              padding: '8px 12px',
              fontSize: '12px',
              pointerEvents: 'none',
              boxShadow: '0 4px 12px rgba(0,0,0,0.5)',
              zIndex: 10,
              minWidth: '160px',
            }}
          >
            <div style={{ fontWeight: 600, color: '#58A6FF', marginBottom: '4px', borderBottom: '1px solid rgba(255,255,255,0.1)', paddingBottom: '2px' }}>
              📅 {activeHoverDate}
            </div>
            {activeSeries.map((s) => {
              const p = s.points[hoverIndex!];
              if (!p) return null;
              return (
                <div key={s.filename} style={{ display: 'flex', justifyContent: 'space-between', gap: '10px', marginTop: '2px' }}>
                  <span style={{ color: s.color, fontWeight: 500 }}>{s.name}:</span>
                  <span style={{ fontWeight: 600, color: p.returnPct >= 0 ? 'var(--color-buy)' : 'var(--color-sell)' }}>
                    {p.returnPct >= 0 ? `+${p.returnPct.toFixed(1)}%` : `${p.returnPct.toFixed(1)}%`}
                  </span>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
};
