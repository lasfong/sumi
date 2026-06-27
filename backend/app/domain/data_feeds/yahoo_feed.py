import yfinance as yf
import pandas as pd
from datetime import datetime
from typing import Optional
import pytz

from app.domain.data_feeds.base import BaseFeed

class YahooFinanceFeed(BaseFeed):
    """
    Yahoo Finance connector.
    Good for daily data and some intraday (max 7 days for 1m, 60 days for 5m/15m).
    """
    
    # Mapping our timeframes to yfinance intervals
    TIMEFRAME_MAP = {
        '1m': '1m',
        '5m': '5m',
        '15m': '15m',
        '1H': '1h',
        '1D': '1d',
        '1W': '1wk'
    }

    def fetch_historical_data(
        self, 
        symbol: str, 
        timeframe: str, 
        start_date: datetime, 
        end_date: Optional[datetime] = None
    ) -> pd.DataFrame:
        interval = self.TIMEFRAME_MAP.get(timeframe)
        if not interval:
            raise ValueError(f"Timeframe {timeframe} not supported by YahooFinanceFeed")
            
        ticker = yf.Ticker(symbol)
        
        # yfinance expects date strings or datetime
        end_param = end_date if end_date else datetime.now(pytz.UTC)
        
        df = ticker.history(start=start_date, end=end_param, interval=interval)
        
        if df.empty:
            return pd.DataFrame(columns=['timestamp', 'open', 'high', 'low', 'close', 'volume'])
            
        # Standardize columns
        df = df.reset_index()
        # Yahoo finance index could be 'Date' or 'Datetime'
        time_col = 'Datetime' if 'Datetime' in df.columns else 'Date'
        
        # Ensure timezone-aware
        if df[time_col].dt.tz is None:
            df[time_col] = df[time_col].dt.tz_localize('UTC')
        else:
            df[time_col] = df[time_col].dt.tz_convert('UTC')
            
        df = df.rename(columns={
            time_col: 'timestamp',
            'Open': 'open',
            'High': 'high',
            'Low': 'low',
            'Close': 'close',
            'Volume': 'volume'
        })
        
        # Keep only required columns
        df = df[['timestamp', 'open', 'high', 'low', 'close', 'volume']]
        return df

    def fetch_realtime_data(self, symbol: str) -> dict:
        ticker = yf.Ticker(symbol)
        data = ticker.fast_info
        return {
            "symbol": symbol,
            "price": data.last_price,
            "volume": data.last_volume,
            "timestamp": datetime.now(pytz.UTC)
        }

    def get_supported_timeframes(self) -> list[str]:
        return list(self.TIMEFRAME_MAP.keys())
