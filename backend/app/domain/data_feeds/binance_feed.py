import requests
import pandas as pd
from datetime import datetime
from typing import Optional
import pytz

from app.domain.data_feeds.base import BaseFeed

class BinanceFeed(BaseFeed):
    """
    Binance connector for cryptocurrency pairs.
    Uses Binance public API, no API key required for public market data.
    """
    BASE_URL = "https://api.binance.com/api/v3"
    
    # Binance kline intervals: 1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 6h, 8h, 12h, 1d, 3d, 1w, 1M
    TIMEFRAME_MAP = {
        '1m': '1m',
        '5m': '5m',
        '15m': '15m',
        '1H': '1h',
        '4H': '4h',
        '1D': '1d',
        '1W': '1w'
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
            raise ValueError(f"Timeframe {timeframe} not supported by BinanceFeed")

        # Convert to milliseconds timestamp
        start_ts = int(start_date.timestamp() * 1000)
        end_ts = int(end_date.timestamp() * 1000) if end_date else int(datetime.now(pytz.UTC).timestamp() * 1000)

        # Binance limit per request is 1000
        limit = 1000
        all_klines = []
        
        current_start = start_ts
        while current_start < end_ts:
            params = {
                "symbol": symbol.upper().replace("/", ""),
                "interval": interval,
                "startTime": current_start,
                "endTime": end_ts,
                "limit": limit
            }
            
            response = requests.get(f"{self.BASE_URL}/klines", params=params)
            response.raise_for_status()
            
            klines = response.json()
            if not klines:
                break
                
            all_klines.extend(klines)
            
            # Update start time for next iteration (last timestamp + 1ms)
            current_start = klines[-1][0] + 1
            
            # Avoid too many requests, limit reached
            if len(klines) < limit:
                break

        if not all_klines:
            return pd.DataFrame(columns=['timestamp', 'open', 'high', 'low', 'close', 'volume'])

        # Columns returned by Binance
        columns = ['timestamp', 'open', 'high', 'low', 'close', 'volume', 
                  'close_time', 'quote_asset_volume', 'number_of_trades',
                  'taker_buy_base_asset_volume', 'taker_buy_quote_asset_volume', 'ignore']
        
        df = pd.DataFrame(all_klines, columns=columns)
        
        # Convert timestamp (ms) to timezone-aware datetime
        df['timestamp'] = pd.to_datetime(df['timestamp'], unit='ms').dt.tz_localize('UTC')
        
        # Convert string values to numeric
        for col in ['open', 'high', 'low', 'close', 'volume']:
            df[col] = pd.to_numeric(df[col])
            
        return df[['timestamp', 'open', 'high', 'low', 'close', 'volume']]

    def fetch_realtime_data(self, symbol: str) -> dict:
        params = {"symbol": symbol.upper().replace("/", "")}
        response = requests.get(f"{self.BASE_URL}/ticker/price", params=params)
        response.raise_for_status()
        data = response.json()
        
        return {
            "symbol": symbol,
            "price": float(data['price']),
            "volume": 0.0, # Ticket/price endpoint doesn't return volume. We'd need 24hr ticker for that
            "timestamp": datetime.now(pytz.UTC)
        }

    def get_supported_timeframes(self) -> list[str]:
        return list(self.TIMEFRAME_MAP.keys())
