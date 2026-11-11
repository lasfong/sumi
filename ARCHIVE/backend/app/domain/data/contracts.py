"""Provider-neutral canonical observation contracts and data quality rules.

Enforces:
- FR-CORE-001: Strict domain model isolation from vendor schemas and SQLite ORMs.
- FR-CORE-012: Explicit value-source precedence (ACTUAL_MATCHED_VALUE vs ESTIMATED_TP_X_VOLUME).
- FR-CORE-014: Capability tagging and method classification (OHLCV_PROXY).
- DATA-CANDLE-001/002/003: Physical price and volume boundary validation.
- NFR-DQ-001: Missing-vs-zero semantics and non-silent value fallback.
"""

from dataclasses import dataclass, field
from datetime import date, datetime
from enum import Enum
import math
from typing import Any, Dict, List, Optional, Tuple


class ValueSource(str, Enum):
    """Provenance and calculation method of trading value."""
    ACTUAL_MATCHED_VALUE = "ACTUAL_MATCHED_VALUE"
    ESTIMATED_TP_X_VOLUME = "ESTIMATED_TP_X_VOLUME"
    ESTIMATED_CLOSE_X_VOLUME = "ESTIMATED_CLOSE_X_VOLUME"
    UNAVAILABLE = "UNAVAILABLE"


class FlowMethod(str, Enum):
    """Classification of order flow methodology based on capability audit."""
    OHLCV_PROXY = "OHLCV_PROXY"
    TICK_TEST_ESTIMATE_RESEARCH_ONLY = "TICK_TEST_ESTIMATE_RESEARCH_ONLY"
    CLASSIFIED_ORDER_FLOW = "CLASSIFIED_ORDER_FLOW"
    EXECUTED_ORDER_FLOW = "EXECUTED_ORDER_FLOW"
    UNKNOWN = "UNKNOWN"


class DataQuality(str, Enum):
    """Grading of observation completeness and integrity."""
    HIGH = "HIGH"
    MEDIUM = "MEDIUM"
    DEGRADED = "DEGRADED"
    SUSPECT = "SUSPECT"
    INVALID = "INVALID"


class AdjustmentType(str, Enum):
    """Corporate action adjustment policy."""
    UNADJUSTED = "UNADJUSTED"
    DIVIDEND_ADJUSTED = "DIVIDEND_ADJUSTED"
    SPLIT_ADJUSTED = "SPLIT_ADJUSTED"
    FULLY_ADJUSTED = "FULLY_ADJUSTED"


class DataIntegrityError(ValueError):
    """Raised when an observation violates non-negotiable candle price or volume invariants."""
    pass


def validate_candle_bounds(
    open_price: float,
    high_price: float,
    low_price: float,
    close_price: float,
    volume: float,
) -> Tuple[bool, List[str]]:
    """Validate physical price and volume boundaries (DATA-CANDLE-001/002/003).
    
    Invariants:
    1. high >= low
    2. high >= open and high >= close
    3. low <= open and low <= close
    4. volume >= 0
    5. prices must be positive non-NaN finite numbers
    """
    errors: List[str] = []

    # Check finite numbers
    for name, val in [("open", open_price), ("high", high_price), ("low", low_price), ("close", close_price)]:
        if val is None or math.isnan(val) or math.isinf(val) or val <= 0:
            errors.append(f"{name} price must be a positive finite number, got {val}")

    if volume is None or math.isnan(volume) or math.isinf(volume):
        errors.append(f"volume must be a finite number, got {volume}")
    elif volume < 0:
        errors.append(f"volume cannot be negative, got {volume}")

    if not errors:
        if high_price < low_price:
            errors.append(f"high price ({high_price}) cannot be less than low price ({low_price})")
        if high_price < open_price:
            errors.append(f"high price ({high_price}) cannot be less than open price ({open_price})")
        if high_price < close_price:
            errors.append(f"high price ({high_price}) cannot be less than close price ({close_price})")
        if low_price > open_price:
            errors.append(f"low price ({low_price}) cannot be greater than open price ({open_price})")
        if low_price > close_price:
            errors.append(f"low price ({low_price}) cannot be greater than close price ({close_price})")

    return (len(errors) == 0, errors)


def calculate_canonical_trading_value(
    open_price: float,
    high_price: float,
    low_price: float,
    close_price: float,
    volume: float,
    reported_value: Optional[float] = None,
) -> Tuple[Optional[float], ValueSource]:
    """Determine authoritative trading value and tag its provenance.
    
    Precedence:
    1. If reported_value is non-null and > 0 -> ACTUAL_MATCHED_VALUE.
    2. Else if volume > 0 and high/low/close are valid:
       - Typical Price (H+L+C)/3 * volume -> ESTIMATED_TP_X_VOLUME.
    3. Else if volume > 0 and close is valid:
       - Close * volume -> ESTIMATED_CLOSE_X_VOLUME.
    4. Else (volume == 0 or negative or invalid):
       - 0.0 if volume == 0, else None -> UNAVAILABLE.
       
    Never silently impute missing value as 0 without tagging ValueSource!
    """
    if reported_value is not None and not math.isnan(reported_value) and reported_value > 0:
        return float(reported_value), ValueSource.ACTUAL_MATCHED_VALUE

    if volume is not None and not math.isnan(volume) and volume > 0:
        # Check Typical Price feasibility
        if (
            high_price is not None and not math.isnan(high_price) and high_price > 0 and
            low_price is not None and not math.isnan(low_price) and low_price > 0 and
            close_price is not None and not math.isnan(close_price) and close_price > 0
        ):
            tp = (high_price + low_price + close_price) / 3.0
            return float(tp * volume), ValueSource.ESTIMATED_TP_X_VOLUME

        if close_price is not None and not math.isnan(close_price) and close_price > 0:
            return float(close_price * volume), ValueSource.ESTIMATED_CLOSE_X_VOLUME

    if volume == 0:
        return 0.0, ValueSource.UNAVAILABLE

    return None, ValueSource.UNAVAILABLE


@dataclass(frozen=True)
class CanonicalBar:
    """Standardized provider-neutral market bar observation."""
    symbol: str
    timestamp: datetime
    session_date: date
    open: float
    high: float
    low: float
    close: float
    volume: float
    trading_value: Optional[float] = None
    value_source: ValueSource = ValueSource.UNAVAILABLE
    flow_method: FlowMethod = FlowMethod.OHLCV_PROXY
    adjustment_type: AdjustmentType = AdjustmentType.UNADJUSTED
    quality: DataQuality = DataQuality.HIGH
    source_provider: str = "unknown"
    raw_metadata: Dict[str, Any] = field(default_factory=dict)

    def __post_init__(self):
        """Validate candle bounds upon instantiation unless explicitly marked INVALID."""
        if not self.symbol or not self.symbol.strip():
            raise ValueError("symbol cannot be empty")

        if self.quality != DataQuality.INVALID:
            is_valid, errors = validate_candle_bounds(
                self.open, self.high, self.low, self.close, self.volume
            )
            if not is_valid:
                raise DataIntegrityError(f"Candle bounds violation for {self.symbol} on {self.session_date}: {'; '.join(errors)}")

    @property
    def typical_price(self) -> float:
        """Calculate unrounded Typical Price (H + L + C) / 3."""
        return (self.high + self.low + self.close) / 3.0

    def to_dict(self) -> Dict[str, Any]:
        """Serialize canonical bar to dictionary."""
        return {
            "symbol": self.symbol,
            "timestamp": self.timestamp.isoformat(),
            "session_date": self.session_date.isoformat(),
            "open": self.open,
            "high": self.high,
            "low": self.low,
            "close": self.close,
            "volume": self.volume,
            "trading_value": self.trading_value,
            "value_source": self.value_source.value,
            "flow_method": self.flow_method.value,
            "adjustment_type": self.adjustment_type.value,
            "quality": self.quality.value,
            "source_provider": self.source_provider,
            "raw_metadata": dict(self.raw_metadata),
        }


@dataclass(frozen=True)
class MarketDataCapabilityManifest:
    """Documents verified capability and rights boundary of a data provider."""
    provider_name: str
    supported_flow_methods: List[FlowMethod]
    supported_value_sources: List[ValueSource]
    supported_symbols: Optional[List[str]] = None
    start_date: Optional[str] = None
    end_date: Optional[str] = None
    is_point_in_time: bool = False
    notes: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "provider_name": self.provider_name,
            "supported_flow_methods": [m.value for m in self.supported_flow_methods],
            "supported_value_sources": [s.value for s in self.supported_value_sources],
            "supported_symbols": list(self.supported_symbols) if self.supported_symbols else None,
            "start_date": self.start_date,
            "end_date": self.end_date,
            "is_point_in_time": self.is_point_in_time,
            "notes": self.notes,
        }
