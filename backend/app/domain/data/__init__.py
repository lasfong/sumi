"""Domain Market Data Layer.

Provides provider-neutral canonical observation contracts, data quality validators,
and port/adapter interfaces for technical analysis, backtesting, and money flow modeling.
"""

from app.domain.data.contracts import (
    AdjustmentType,
    CanonicalBar,
    DataIntegrityError,
    DataQuality,
    FlowMethod,
    MarketDataCapabilityManifest,
    ValueSource,
    calculate_canonical_trading_value,
    validate_candle_bounds,
)
from app.domain.data.ports import MarketDataPort

__all__ = [
    "AdjustmentType",
    "CanonicalBar",
    "DataIntegrityError",
    "DataQuality",
    "FlowMethod",
    "MarketDataCapabilityManifest",
    "MarketDataPort",
    "ValueSource",
    "calculate_canonical_trading_value",
    "validate_candle_bounds",
]
