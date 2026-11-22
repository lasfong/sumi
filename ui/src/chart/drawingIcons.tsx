/**
 * Crisp 20x20 SVG Drawing Tool Icons for all 37+ tools in the 42px Left Drawing Rail.
 * Geometric & Fibonacci/Wave motifs inspired by klinecharts-pro (Apache-2.0) & TradingView Supercharts.
 */
import React from 'react'

export interface DrawingToolItem {
  key: string
  labelVi: string
  shortcut?: string
}

export interface DrawingToolGroup {
  id: string
  labelVi: string
  defaultTool: string
  items: DrawingToolItem[]
}

/**
 * Complete 37-tool Drawing Rail groups:
 *  - 16 KLineChart v10 built-in overlays
 *  - 17 upgraded klinecharts-pro overlays
 *  - 4 new TradingView signature overlays (longPosition, shortPosition, priceDateRange, brokerOrderLine)
 */
export const DRAWING_TOOL_GROUPS: DrawingToolGroup[] = [
  {
    id: 'lines',
    labelVi: 'Đường xu hướng & Ngang/Dọc',
    defaultTool: 'segment',
    items: [
      { key: 'segment', labelVi: 'Đoạn xu hướng (Trend Line)', shortcut: 'Alt+T' },
      { key: 'rayLine', labelVi: 'Tia xu hướng (Ray)' },
      { key: 'straightLine', labelVi: 'Đường thẳng kéo dài' },
      { key: 'arrow', labelVi: 'Mũi tên xu hướng (Arrow)' },
      { key: 'horizontalStraightLine', labelVi: 'Đường ngang (H-Line)', shortcut: 'Alt+H' },
      { key: 'horizontalRayLine', labelVi: 'Tia ngang (H-Ray)', shortcut: 'Alt+J' },
      { key: 'horizontalSegment', labelVi: 'Đoạn ngang' },
      { key: 'verticalStraightLine', labelVi: 'Đường dọc (V-Line)', shortcut: 'Alt+V' },
      { key: 'verticalRayLine', labelVi: 'Tia dọc' },
      { key: 'verticalSegment', labelVi: 'Đoạn dọc' },
      { key: 'priceLine', labelVi: 'Đường giá (Price Line)' },
    ],
  },
  {
    id: 'channels',
    labelVi: 'Kênh giá & Song song',
    defaultTool: 'priceChannelLine',
    items: [
      { key: 'priceChannelLine', labelVi: 'Kênh giá song song (Price Channel)' },
      { key: 'parallelStraightLine', labelVi: 'Đường thẳng song song' },
    ],
  },
  {
    id: 'fibonacci',
    labelVi: 'Fibonacci & Gann',
    defaultTool: 'fibonacciLine',
    items: [
      { key: 'fibonacciLine', labelVi: 'Fibonacci Thoái lui (Retracement)', shortcut: 'Alt+F' },
      { key: 'fibonacciSegment', labelVi: 'Fibonacci Đoạn (Segment)' },
      { key: 'fibonacciExtension', labelVi: 'Fibonacci Mở rộng (Extension)' },
      { key: 'fibonacciCircle', labelVi: 'Vòng tròn Fibonacci' },
      { key: 'fibonacciSpiral', labelVi: 'Xoắn ốc Fibonacci' },
      { key: 'fibonacciSpeedResistanceFan', labelVi: 'Quạt kháng cự Fibonacci' },
      { key: 'gannBox', labelVi: 'Hộp Gann (Gann Box)' },
    ],
  },
  {
    id: 'waves',
    labelVi: 'Sóng Elliott & Mô hình Harmonic',
    defaultTool: 'fiveWaves',
    items: [
      { key: 'xabcd', labelVi: 'Mô hình Harmonic XABCD' },
      { key: 'abcd', labelVi: 'Mô hình ABCD' },
      { key: 'threeWaves', labelVi: 'Sóng điều chỉnh ABC (3 sóng)' },
      { key: 'fiveWaves', labelVi: 'Sóng đẩy Elliott (5 sóng)' },
      { key: 'eightWaves', labelVi: 'Chu kỳ Elliott đầy đủ (8 sóng)' },
      { key: 'anyWaves', labelVi: 'Đa tuyến sóng tự do (Any Waves)' },
    ],
  },
  {
    id: 'shapes',
    labelVi: 'Hình học & Vẽ tự do',
    defaultTool: 'rect',
    items: [
      { key: 'rect', labelVi: 'Hình chữ nhật (Rectangle)' },
      { key: 'circle', labelVi: 'Hình tròn (Circle)' },
      { key: 'triangle', labelVi: 'Hình tam giác (Triangle)' },
      { key: 'parallelogram', labelVi: 'Hình bình hành (Parallelogram)' },
      { key: 'brush', labelVi: 'Cọ vẽ tự do (Brush)' },
    ],
  },
  {
    id: 'annotations',
    labelVi: 'Ghi chú & Nhãn giá',
    defaultTool: 'simpleAnnotation',
    items: [
      { key: 'simpleAnnotation', labelVi: 'Ghi chú văn bản (Annotation)' },
      { key: 'simpleTag', labelVi: 'Nhãn giá trên trục (Price Tag)' },
    ],
  },
  {
    id: 'trading',
    labelVi: 'Dự phóng R:R & Lệnh trên biểu đồ',
    defaultTool: 'longPosition',
    items: [
      { key: 'longPosition', labelVi: 'Vị thế Mua — Khung R:R (Long Position)' },
      { key: 'shortPosition', labelVi: 'Vị thế Bán — Khung R:R (Short Position)' },
      { key: 'brokerOrderLine', labelVi: 'Đường Lệnh / Vị thế (Order Line)' },
    ],
  },
  {
    id: 'measure',
    labelVi: 'Thước đo Giá & Thời gian',
    defaultTool: 'priceDateRange',
    items: [
      { key: 'priceDateRange', labelVi: 'Thước đo Giá & Thời gian (Measure)', shortcut: 'Shift+M' },
    ],
  },
]

export const ALL_DRAWING_TOOLS_COUNT = DRAWING_TOOL_GROUPS.reduce((sum, g) => sum + g.items.length, 0)

export function DrawingToolIcon({ name, className = 'w-[18px] h-[18px]' }: { name: string; className?: string }): React.ReactElement {
  const common = {
    viewBox: '0 0 22 22',
    fill: 'none',
    stroke: 'currentColor',
    strokeWidth: 1.5,
    strokeLinecap: 'round' as const,
    strokeLinejoin: 'round' as const,
    className,
    'aria-hidden': true,
  }

  switch (name) {
    case 'cursor':
      return (
        <svg {...common}>
          <path d="M11 3v16M3 11h16" strokeDasharray="2.5 2.5" />
          <circle cx="11" cy="11" r="1.5" fill="currentColor" stroke="none" />
        </svg>
      )
    case 'segment':
      return (
        <svg {...common}>
          <line x1="4" y1="17" x2="18" y2="5" />
          <circle cx="4" cy="17" r="1.8" fill="var(--tv-bg-panel)" />
          <circle cx="18" cy="5" r="1.8" fill="var(--tv-bg-panel)" />
        </svg>
      )
    case 'rayLine':
      return (
        <svg {...common}>
          <line x1="4" y1="17" x2="19" y2="4" />
          <circle cx="4" cy="17" r="1.8" fill="var(--tv-bg-panel)" />
          <circle cx="12" cy="10" r="1.8" fill="var(--tv-bg-panel)" />
        </svg>
      )
    case 'straightLine':
      return (
        <svg {...common}>
          <line x1="2" y1="19" x2="20" y2="3" />
          <circle cx="8" cy="13.5" r="1.6" fill="var(--tv-bg-panel)" />
          <circle cx="14" cy="8.2" r="1.6" fill="var(--tv-bg-panel)" />
        </svg>
      )
    case 'arrow':
      return (
        <svg {...common}>
          <line x1="4" y1="17" x2="18" y2="5" />
          <polyline points="12,5 18,5 18,11" />
        </svg>
      )
    case 'horizontalStraightLine':
    case 'horizontalRayLine':
    case 'horizontalSegment':
      return (
        <svg {...common}>
          <line x1="3" y1="11" x2="19" y2="11" />
          <circle cx="11" cy="11" r="1.8" fill="var(--tv-bg-panel)" />
        </svg>
      )
    case 'verticalStraightLine':
    case 'verticalRayLine':
    case 'verticalSegment':
      return (
        <svg {...common}>
          <line x1="11" y1="3" x2="11" y2="19" />
          <circle cx="11" cy="11" r="1.8" fill="var(--tv-bg-panel)" />
        </svg>
      )
    case 'priceLine':
      return (
        <svg {...common}>
          <line x1="3" y1="13" x2="19" y2="13" />
          <rect x="4" y="6" width="7" height="4" rx="1" />
        </svg>
      )
    case 'priceChannelLine':
    case 'parallelStraightLine':
      return (
        <svg {...common}>
          <line x1="3" y1="13" x2="16" y2="4" />
          <line x1="6" y1="18" x2="19" y2="9" />
          <circle cx="9.5" cy="8.5" r="1.5" fill="var(--tv-bg-panel)" />
        </svg>
      )
    case 'fibonacciLine':
    case 'fibonacciSegment':
    case 'fibonacciExtension':
      return (
        <svg {...common}>
          <line x1="3" y1="5" x2="19" y2="5" />
          <line x1="3" y1="9.5" x2="19" y2="9.5" strokeDasharray="2 2" />
          <line x1="3" y1="13.5" x2="19" y2="13.5" strokeDasharray="2 2" />
          <line x1="3" y1="17.5" x2="19" y2="17.5" />
        </svg>
      )
    case 'fibonacciCircle':
      return (
        <svg {...common}>
          <circle cx="11" cy="11" r="3" />
          <circle cx="11" cy="11" r="6" />
          <circle cx="11" cy="11" r="8.5" strokeDasharray="2 2" />
        </svg>
      )
    case 'fibonacciSpiral':
      return (
        <svg {...common}>
          <path d="M11 11a2 2 0 1 1 2 2a4 4 0 1 1-4-4a7 7 0 1 1 7 7" />
        </svg>
      )
    case 'fibonacciSpeedResistanceFan':
      return (
        <svg {...common}>
          <line x1="4" y1="18" x2="19" y2="4" />
          <line x1="4" y1="18" x2="19" y2="10" />
          <line x1="4" y1="18" x2="19" y2="15" />
          <line x1="4" y1="18" x2="12" y2="4" />
        </svg>
      )
    case 'gannBox':
      return (
        <svg {...common}>
          <rect x="4" y="4" width="14" height="14" />
          <line x1="4" y1="18" x2="18" y2="4" />
          <line x1="4" y1="4" x2="18" y2="18" />
          <line x1="4" y1="11" x2="18" y2="11" strokeDasharray="2 2" />
        </svg>
      )
    case 'xabcd':
    case 'abcd':
    case 'threeWaves':
    case 'fiveWaves':
    case 'eightWaves':
    case 'anyWaves':
      return (
        <svg {...common}>
          <polyline points="3,16 7,7 11,13 15,5 19,14" />
          <circle cx="7" cy="7" r="1.4" fill="var(--tv-bg-panel)" />
          <circle cx="15" cy="5" r="1.4" fill="var(--tv-bg-panel)" />
        </svg>
      )
    case 'rect':
      return (
        <svg {...common}>
          <rect x="4" y="5" width="14" height="12" rx="1" fill="currentColor" fillOpacity="0.14" />
          <circle cx="4" cy="5" r="1.5" fill="var(--tv-bg-panel)" />
          <circle cx="18" cy="17" r="1.5" fill="var(--tv-bg-panel)" />
        </svg>
      )
    case 'circle':
      return (
        <svg {...common}>
          <circle cx="11" cy="11" r="7" fill="currentColor" fillOpacity="0.14" />
          <circle cx="11" cy="11" r="1.5" fill="var(--tv-bg-panel)" />
        </svg>
      )
    case 'triangle':
      return (
        <svg {...common}>
          <polygon points="11,4 19,18 3,18" fill="currentColor" fillOpacity="0.14" />
        </svg>
      )
    case 'parallelogram':
      return (
        <svg {...common}>
          <polygon points="7,5 19,5 15,17 3,17" fill="currentColor" fillOpacity="0.14" />
        </svg>
      )
    case 'brush':
      return (
        <svg {...common}>
          <path d="M4 17c2-4 5-10 10-12 1.5-.6 3 .6 2.3 2.1-1.6 3.5-5.2 6.8-8.3 9.2" />
          <path d="M4 17l3 1" />
        </svg>
      )
    case 'simpleAnnotation':
    case 'simpleTag':
      return (
        <svg {...common}>
          <path d="M5 5h12M11 5v12M8 17h6" />
        </svg>
      )
    case 'longPosition':
      return (
        <svg {...common}>
          <rect x="4" y="4" width="14" height="7" fill="#089981" fillOpacity="0.32" stroke="#089981" />
          <rect x="4" y="11" width="14" height="7" fill="#F23645" fillOpacity="0.32" stroke="#F23645" />
          <line x1="3" y1="11" x2="19" y2="11" stroke="currentColor" />
        </svg>
      )
    case 'shortPosition':
      return (
        <svg {...common}>
          <rect x="4" y="4" width="14" height="7" fill="#F23645" fillOpacity="0.32" stroke="#F23645" />
          <rect x="4" y="11" width="14" height="7" fill="#089981" fillOpacity="0.32" stroke="#089981" />
          <line x1="3" y1="11" x2="19" y2="11" stroke="currentColor" />
        </svg>
      )
    case 'brokerOrderLine':
      return (
        <svg {...common}>
          <line x1="3" y1="11" x2="12" y2="11" strokeDasharray="2 2" />
          <rect x="12" y="8" width="7" height="6" rx="1.5" fill="#2962FF" stroke="#2962FF" />
        </svg>
      )
    case 'priceDateRange':
      return (
        <svg {...common}>
          <rect x="4" y="5" width="14" height="12" rx="1" fill="currentColor" fillOpacity="0.14" />
          <path d="M7 11h8M11 8v6" />
        </svg>
      )
    default:
      return (
        <svg {...common}>
          <line x1="4" y1="17" x2="18" y2="5" />
        </svg>
      )
  }
}
