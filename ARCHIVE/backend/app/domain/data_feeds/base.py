from abc import ABC, abstractmethod
import pandas as pd
from datetime import datetime
from typing import Optional

class BaseFeed(ABC):
    """
    Base interface for all Data Feed Connectors.
    Following Domain-Driven Design (DDD) to standardize data ingestion.
    """
    
    @abstractmethod
    def fetch_historical_data(
        self, 
        symbol: str, 
        timeframe: str, 
        start_date: datetime, 
        end_date: Optional[datetime] = None
    ) -> pd.DataFrame:
        """
        Fetch historical candle data.
        Must return a pandas DataFrame with standardized columns:
        ['timestamp', 'open', 'high', 'low', 'close', 'volume']
        The timestamp must be timezone-aware.
        """
        pass
    
    @abstractmethod
    def fetch_realtime_data(self, symbol: str) -> dict:
        """
        Fetch a single real-time quote/tick.
        """
        pass

    @abstractmethod
    def get_supported_timeframes(self) -> list[str]:
        """
        Return a list of supported timeframes for this feed (e.g., ['1m', '5m', '1H', '1D']).
        """
        pass
