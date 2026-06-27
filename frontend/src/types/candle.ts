/** OHLCV candle data from the backend */
export interface Candle {
  id: number;
  symbol: string;
  timeframe: string;
  timestamp: string;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  source: string;
  adjustment_type: string;
}

/** Lightweight Charts format for candlestick rendering */
export interface ChartCandle {
  time: string;
  open: number;
  high: number;
  low: number;
  close: number;
}

/** Lightweight Charts format for volume histogram */
export interface ChartVolume {
  time: string;
  value: number;
  color: string;
}
