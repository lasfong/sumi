export interface IndicatorCatalogEntry {
  name: string
  titleVi: string
  category: 'Xu hướng' | 'Dao động' | 'Khối lượng' | 'Biến động' | 'Sumi Quant'
  isMain: boolean
  defaultParams: number[]
  paramLabels: string[]
  description: string
}

export const INDICATOR_CATALOG: IndicatorCatalogEntry[] = [
  // Sumi Core Quantitative Indicators (Custom registered)
  {
    name: 'ICHIMOKU',
    titleVi: 'Mây Ichimoku Kinko Hyo',
    category: 'Sumi Quant',
    isMain: true,
    defaultParams: [9, 26, 52],
    paramLabels: ['Tenkan', 'Kijun', 'Senkou B'],
    description: 'Đám mây Kumo, Tenkan-sen, Kijun-sen, Chikou Span chuẩn Nhật Bản',
  },
  {
    name: 'SUPERTREND',
    titleVi: 'SuperTrend ATR',
    category: 'Sumi Quant',
    isMain: true,
    defaultParams: [10, 3],
    paramLabels: ['ATR Period', 'Hệ số'],
    description: 'Theo dõi xu hướng động dựa trên ATR (xanh tăng / đỏ giảm)',
  },
  {
    name: 'VWAP',
    titleVi: 'VWAP & Dải độ lệch ±1σ/±2σ',
    category: 'Sumi Quant',
    isMain: true,
    defaultParams: [1, 2],
    paramLabels: ['Dải 1 (σ)', 'Dải 2 (σ)'],
    description: 'Giá bình quân gia quyền khối lượng phiên kèm băng chuẩn độ lệch',
  },
  {
    name: 'ATR',
    titleVi: 'Average True Range (ATR)',
    category: 'Sumi Quant',
    isMain: false,
    defaultParams: [14],
    paramLabels: ['Chu kỳ'],
    description: 'Đo lường biên độ dao động thực trung bình (Wilder ATR)',
  },
  {
    name: 'STOCHRSI',
    titleVi: 'Stochastic RSI',
    category: 'Sumi Quant',
    isMain: false,
    defaultParams: [14, 14, 3, 3],
    paramLabels: ['RSI', 'Stoch', '%K', '%D'],
    description: 'Bộ dao động Stochastic áp dụng trên đường RSI (quá mua 80 / quá bán 20)',
  },

  // Main-pane Overlay Built-in Indicators (KLineChart v10)
  { name: 'MA', titleVi: 'Đường Trung Bình Động (MA)', category: 'Xu hướng', isMain: true, defaultParams: [5, 10, 20, 60], paramLabels: ['MA1', 'MA2', 'MA3', 'MA4'], description: 'Simple Moving Average đa chu kỳ' },
  { name: 'EMA', titleVi: 'Trung Bình Lũy Thừa (EMA)', category: 'Xu hướng', isMain: true, defaultParams: [12, 26, 50], paramLabels: ['EMA1', 'EMA2', 'EMA3'], description: 'Exponential Moving Average nhạy hơn với giá gần nhất' },
  { name: 'SMA', titleVi: 'Smoothed Moving Average (SMA)', category: 'Xu hướng', isMain: true, defaultParams: [12, 2], paramLabels: ['Chu kỳ', 'Trọng số'], description: 'Trung bình trượt mượt' },
  { name: 'BOLL', titleVi: 'Dải Bollinger Bands (BOLL)', category: 'Biến động', isMain: true, defaultParams: [20, 2], paramLabels: ['Chu kỳ', 'Độ lệch chuẩn'], description: 'Dải biến động trên/giữa/dưới theo độ lệch chuẩn' },
  { name: 'BBI', titleVi: 'Chỉ Số Bò & Gấu (BBI)', category: 'Xu hướng', isMain: true, defaultParams: [3, 6, 12, 24], paramLabels: ['P1', 'P2', 'P3', 'P4'], description: 'Bull and Bear Index tổng hợp 4 đường trung bình' },
  { name: 'SAR', titleVi: 'Parabolic SAR', category: 'Xu hướng', isMain: true, defaultParams: [2, 2, 20], paramLabels: ['Khởi đầu', 'Bước', 'Tối đa'], description: 'Điểm dừng và đảo chiều xu hướng Parabolic' },

  // Sub-pane Built-in Indicators (All 21 remaining KLineChart v10 indicators -> 27 built-in + 5 custom = 32!)
  { name: 'VOL', titleVi: 'Khối Lượng Giao Dịch (VOL)', category: 'Khối lượng', isMain: false, defaultParams: [5, 10, 20], paramLabels: ['MA1', 'MA2', 'MA3'], description: 'Cột khối lượng kèm đường trung bình KL' },
  { name: 'MACD', titleVi: 'MACD (12, 26, 9)', category: 'Dao động', isMain: false, defaultParams: [12, 26, 9], paramLabels: ['Nhanh', 'Chậm', 'Tín hiệu'], description: 'Hội tụ phân kỳ đường trung bình động' },
  { name: 'RSI', titleVi: 'Chỉ Số Sức Mạnh Tương Đối (RSI)', category: 'Dao động', isMain: false, defaultParams: [6, 14, 24], paramLabels: ['RSI1', 'RSI2', 'RSI3'], description: 'Đo xung lượng quá mua / quá bán' },
  { name: 'KDJ', titleVi: 'Stochastic KDJ', category: 'Dao động', isMain: false, defaultParams: [9, 3, 3], paramLabels: ['Chu kỳ', 'K', 'D'], description: 'Chỉ báo dao động ngẫu nhiên K, D, J' },
  { name: 'OBV', titleVi: 'Khối Lượng Cân Bằng (OBV)', category: 'Khối lượng', isMain: false, defaultParams: [30], paramLabels: ['MA'], description: 'On-Balance Volume đo dòng tiền tích lũy' },
  { name: 'DMI', titleVi: 'Chỉ Số Định Hướng (DMI / ADX)', category: 'Xu hướng', isMain: false, defaultParams: [14, 6], paramLabels: ['DI', 'ADX'], description: 'PDI, MDI và sức mạnh xu hướng ADX' },
  { name: 'CCI', titleVi: 'Commodity Channel Index (CCI)', category: 'Dao động', isMain: false, defaultParams: [20], paramLabels: ['Chu kỳ'], description: 'Đo độ lệch giá so với mức trung bình thống kê' },
  { name: 'WR', titleVi: 'Williams %R (WR)', category: 'Dao động', isMain: false, defaultParams: [6, 10, 14], paramLabels: ['WR1', 'WR2', 'WR3'], description: 'Động lượng quá mua/bán của Larry Williams' },
  { name: 'BIAS', titleVi: 'Độ Lệch Giá (BIAS)', category: 'Dao động', isMain: false, defaultParams: [6, 12, 24], paramLabels: ['P1', 'P2', 'P3'], description: 'Khoảng cách phần trăm giữa giá và đường MA' },
  { name: 'BRAR', titleVi: 'Tâm Lý Thị Trường (BRAR)', category: 'Dao động', isMain: false, defaultParams: [26], paramLabels: ['Chu kỳ'], description: 'Đo ý chí mua bán & mức độ hưng phấn' },
  { name: 'CR', titleVi: 'Chỉ Số Năng Lượng (CR)', category: 'Biến động', isMain: false, defaultParams: [26, 10, 20, 40, 60], paramLabels: ['CR', 'MA1', 'MA2', 'MA3', 'MA4'], description: 'Đo năng lượng tăng giảm quanh giá trung bình' },
  { name: 'DMA', titleVi: 'Hiệu Đường Trung Bình (DMA)', category: 'Xu hướng', isMain: false, defaultParams: [10, 50, 10], paramLabels: ['Ngắn', 'Dài', 'AMA'], description: 'Chênh lệch giữa hai đường trung bình động' },
  { name: 'EMV', titleVi: 'Ease of Movement (EMV)', category: 'Khối lượng', isMain: false, defaultParams: [14, 9], paramLabels: ['Chu kỳ', 'MA'], description: 'Tương quan giữa biến động giá và thanh khoản' },
  { name: 'MTM', titleVi: 'Xung Lượng Giá (Momentum)', category: 'Dao động', isMain: false, defaultParams: [12, 6], paramLabels: ['Chu kỳ', 'MA'], description: 'Tốc độ thay đổi giá tuyệt đối' },
  { name: 'PSY', titleVi: 'Đường Tâm Lý (PSY)', category: 'Dao động', isMain: false, defaultParams: [12, 6], paramLabels: ['Chu kỳ', 'MA'], description: 'Tỷ lệ phiên tăng trong N phiên gần nhất' },
  { name: 'ROC', titleVi: 'Tỷ Suất Thay Đổi (ROC)', category: 'Dao động', isMain: false, defaultParams: [12, 6], paramLabels: ['Chu kỳ', 'MA'], description: 'Phần trăm thay đổi giá so với N phiên trước' },
  { name: 'TRIX', titleVi: 'Triple EMA Oscillator (TRIX)', category: 'Xu hướng', isMain: false, defaultParams: [12, 9], paramLabels: ['TRIX', 'MA'], description: 'Lọc nhiễu xu hướng bằng EMA ba lớp' },
  { name: 'VR', titleVi: 'Tỷ Lệ Khối Lượng (VR)', category: 'Khối lượng', isMain: false, defaultParams: [26, 6], paramLabels: ['Chu kỳ', 'MA'], description: 'Volume Ratio giữa phiên tăng và phiên giảm' },
  { name: 'AO', titleVi: 'Awesome Oscillator (AO)', category: 'Dao động', isMain: false, defaultParams: [5, 34], paramLabels: ['Ngắn', 'Dài'], description: 'Động lượng Bill Williams (SMA 5 - SMA 34 của HL2)' },
  { name: 'PVT', titleVi: 'Xu Hướng Giá & Khối Lượng (PVT)', category: 'Khối lượng', isMain: false, defaultParams: [], paramLabels: [], description: 'Price Volume Trend tích lũy theo % thay đổi' },
  { name: 'AVP', titleVi: 'Giá Bình Quân (AVP)', category: 'Khối lượng', isMain: false, defaultParams: [], paramLabels: [], description: 'Average Price từ giá trị giao dịch / khối lượng' },
]

export function findIndicatorMeta(name: string): IndicatorCatalogEntry | undefined {
  return INDICATOR_CATALOG.find((item) => item.name === name)
}
