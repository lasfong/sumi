from abc import ABC, abstractmethod
from datetime import date, datetime
from typing import List, Dict, Any, Optional
from pydantic import BaseModel

class ProviderCandleDTO(BaseModel):
    """Normalized, vendor-neutral candle transfer object."""
    symbol: str
    timeframe: str = "1D"  # Must be '1D' (1W is derived locally by WeeklyAggregator)
    timestamp: date
    open: float
    high: float
    low: float
    close: float
    volume: float
    adjustment_type: str = "unadjusted"  # 'unadjusted' | 'adjusted'


class ProviderMetadata(BaseModel):
    """Metadata describing provider capabilities and authentication requirements."""
    provider_id: str
    display_name: str
    is_official: bool
    requires_auth: bool
    supported_timeframes: List[str] = ["1D"]
    supported_adjustments: List[str] = ["unadjusted", "adjusted"]
    rate_limit_rps: int = 10
    description: str
    auth_fields: List[str] = []


class ProviderException(Exception):
    """Base exception for all market data provider errors."""
    pass


class ProviderAuthError(ProviderException):
    """Authentication or authorization failure."""
    pass


class ProviderRateLimitError(ProviderException):
    """Rate limit exceeded."""
    pass


class ProviderNetworkError(ProviderException):
    """Network connection or timeout error."""
    pass


class ProviderDataError(ProviderException):
    """Malformed or invalid data payload received from provider."""
    pass


class MarketDataProviderAdapter(ABC):
    """Authoritative abstract interface for all market data providers (ADR-002)."""

    @abstractmethod
    def get_metadata(self) -> ProviderMetadata:
        """Returns provider capabilities and constraints."""
        pass

    @abstractmethod
    def test_connection(self, credentials: Optional[Dict[str, Any]] = None) -> bool:
        """Validates credentials and API connectivity without mutating local data."""
        pass

    @abstractmethod
    def fetch_daily_candles(
        self,
        symbol: str,
        start_date: date,
        end_date: date,
        adjustment_type: str = "unadjusted",
        credentials: Optional[Dict[str, Any]] = None
    ) -> List[ProviderCandleDTO]:
        """
        Fetches and normalizes Daily candles into ProviderCandleDTO objects.
        Must raise ProviderRateLimitError, ProviderAuthError, ProviderNetworkError, or ProviderDataError on failure.
        """
        pass

    @abstractmethod
    def fetch_benchmark_indices(
        self,
        benchmark: str,  # 'VNINDEX' | 'VN30' | 'HNX30'
        start_date: date,
        end_date: date,
        credentials: Optional[Dict[str, Any]] = None
    ) -> List[ProviderCandleDTO]:
        """Fetches market index candles for Relative Strength and market regime analysis."""
        pass
