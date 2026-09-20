"""Money Flow Blackbox (BB) contracts and boundary guardrails.

Enforces:
- FR-CORE-012: BB request / output point contracts with traceable value source and method metadata.
- FR-CORE-014: Rejection of unvalidated methods (True Flow, Tick Test) in daily symbol BB.
- Section 20 / Invariant: Thresholds (20/30/70/80) remain unvalidated research hypotheses and cannot be hardcoded as production rules.
"""

from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Dict, List, Optional

from app.domain.data.contracts import DataQuality, FlowMethod, ValueSource


class BBHorizon(str, Enum):
    """Standard Blackbox evaluation horizons."""
    T03 = "T03"
    T05 = "T05"
    T10 = "T10"
    T20 = "T20"
    T50 = "T50"
    T200 = "T200"

    @property
    def lookback_bars(self) -> int:
        """Integer lookback length for moving window."""
        return {
            BBHorizon.T03: 3,
            BBHorizon.T05: 5,
            BBHorizon.T10: 10,
            BBHorizon.T20: 20,
            BBHorizon.T50: 50,
            BBHorizon.T200: 200,
        }[self]


@dataclass(frozen=True)
class BBSymbolRequest:
    """Request specification for symbol Blackbox calculation."""
    symbol: str
    as_of: Optional[str] = None
    horizons: List[BBHorizon] = field(default_factory=lambda: [
        BBHorizon.T03, BBHorizon.T05, BBHorizon.T20, BBHorizon.T50, BBHorizon.T200
    ])
    accepted_methods: List[FlowMethod] = field(default_factory=lambda: [FlowMethod.OHLCV_PROXY])
    strict_provenance: bool = True

    def __post_init__(self):
        if not self.symbol or not self.symbol.strip():
            raise ValueError("symbol cannot be empty")

        if not self.horizons:
            raise ValueError("At least one BB horizon must be requested")

        # Invariant: only OHLCV_PROXY is authorized for daily production Symbol BB
        for m in self.accepted_methods:
            if m != FlowMethod.OHLCV_PROXY:
                raise ValueError(
                    f"Method '{m.value}' is not authorized for daily Symbol BB. "
                    f"Only OHLCV_PROXY is validated. True Flow and Tick Test remain R&D capabilities."
                )


@dataclass(frozen=True)
class BBMetricPoint:
    """Calculated Blackbox metric output point for a specific session and horizon."""
    date: str
    horizon: BBHorizon
    bb_value: Optional[float]
    flow_method: FlowMethod
    value_source: ValueSource
    quality: DataQuality
    raw_numerator: Optional[float] = None
    raw_denominator: Optional[float] = None
    is_warmup: bool = False

    def to_dict(self) -> Dict[str, Any]:
        return {
            "date": self.date,
            "horizon": self.horizon.value,
            "bb_value": round(self.bb_value, 2) if self.bb_value is not None else None,
            "flow_method": self.flow_method.value,
            "value_source": self.value_source.value,
            "quality": self.quality.value,
            "raw_numerator": self.raw_numerator,
            "raw_denominator": self.raw_denominator,
            "is_warmup": self.is_warmup,
        }


class BBDirection(str, Enum):
    """Direction of BB score relative to previous valid session."""
    RISING = "RISING"
    FALLING = "FALLING"
    FLAT = "FLAT"
    UNKNOWN = "UNKNOWN"


class BBRegime(str, Enum):
    """Macro regime classification relative to neutral 50.0 baseline."""
    POSITIVE = "POSITIVE"
    NEGATIVE = "NEGATIVE"
    NEUTRAL = "NEUTRAL"
    UNKNOWN = "UNKNOWN"


@dataclass(frozen=True)
class BBSymbolHorizonPoint:
    """Calculated metric for a single horizon on a specific calendar session."""
    horizon: BBHorizon
    bb_value: Optional[float]
    oib_raw: Optional[float] = None
    raw_numerator: Optional[float] = None
    raw_denominator: Optional[float] = None
    is_warmup: bool = False
    direction: BBDirection = BBDirection.UNKNOWN
    regime: BBRegime = BBRegime.UNKNOWN
    regime_run_length: int = 0
    value_source: ValueSource = ValueSource.UNAVAILABLE
    quality: DataQuality = DataQuality.HIGH

    def to_dict(self) -> Dict[str, Any]:
        return {
            "horizon": self.horizon.value,
            "bb_value": round(self.bb_value, 2) if self.bb_value is not None else None,
            "oib_raw": round(self.oib_raw, 4) if self.oib_raw is not None else None,
            "raw_numerator": round(self.raw_numerator, 2) if self.raw_numerator is not None else None,
            "raw_denominator": round(self.raw_denominator, 2) if self.raw_denominator is not None else None,
            "is_warmup": self.is_warmup,
            "direction": self.direction.value,
            "regime": self.regime.value,
            "regime_run_length": self.regime_run_length,
            "value_source": self.value_source.value,
            "quality": self.quality.value,
        }


@dataclass(frozen=True)
class BBSymbolPoint:
    """Composite Blackbox observation across all evaluated horizons for a session."""
    date: str
    horizons: Dict[str, BBSymbolHorizonPoint]
    daily_pressure: float
    daily_trading_value: float
    daily_value_source: ValueSource
    daily_quality: DataQuality

    def to_dict(self) -> Dict[str, Any]:
        return {
            "date": self.date,
            "horizons": {h: pt.to_dict() for h, pt in self.horizons.items()},
            "daily_pressure": round(self.daily_pressure, 4),
            "daily_trading_value": round(self.daily_trading_value, 2),
            "daily_value_source": self.daily_value_source.value,
            "daily_quality": self.daily_quality.value,
        }


@dataclass(frozen=True)
class BBSymbolSeriesResult:
    """Complete multi-horizon Blackbox series result for a symbol."""
    symbol: str
    flow_method: FlowMethod = FlowMethod.OHLCV_PROXY
    methodology_version: str = "bb_v1_ohlcv_proxy"
    as_of: Optional[str] = None
    requested_horizons: List[BBHorizon] = field(default_factory=list)
    points: List[BBSymbolPoint] = field(default_factory=list)
    total_bars: int = 0
    coverage_ratio: float = 1.0

    def to_dict(self) -> Dict[str, Any]:
        return {
            "symbol": self.symbol,
            "flow_method": self.flow_method.value,
            "methodology_version": self.methodology_version,
            "as_of": self.as_of,
            "requested_horizons": [h.value for h in self.requested_horizons],
            "total_bars": self.total_bars,
            "coverage_ratio": round(self.coverage_ratio, 4),
            "points": [p.to_dict() for p in self.points],
        }


def validate_bb_threshold_semantics(threshold: float) -> None:
    """Guardrail preventing hardcoding production trading semantics for 20/30/70/80 thresholds.
    
    Per Master Spec and Audit recommendations, thresholds remain R&D hypotheses until
    measurement validation (P6-MEASURE-01) empirically establishes their statistical behavior.
    """
    if threshold in (20.0, 30.0, 70.0, 80.0):
        raise ValueError(
            f"Threshold {threshold} cannot be hardcoded as an authoritative production signal rule. "
            f"Measurement validation in P6-MEASURE-01 is required before freezing threshold semantics."
        )


class MarketPublicationStatus(str, Enum):
    """Publication gate status for whole-market and aggregate metrics."""
    CANONICAL_PUBLISHED = "CANONICAL_PUBLISHED"
    RESEARCH_RETROSPECTIVE = "RESEARCH_RETROSPECTIVE"
    UNAVAILABLE_DEGRADED = "UNAVAILABLE_DEGRADED"


@dataclass(frozen=True)
class MarketBBHorizonPoint:
    """Calculated whole-market Blackbox metric for a single horizon on a session."""
    horizon: BBHorizon
    market_bb_value: Optional[float]
    market_buy_value: float
    market_sell_value: float
    market_total_value: float
    market_oib: Optional[float] = None
    is_warmup: bool = False
    direction: BBDirection = BBDirection.UNKNOWN
    regime: BBRegime = BBRegime.UNKNOWN
    quality: DataQuality = DataQuality.HIGH

    def to_dict(self) -> Dict[str, Any]:
        return {
            "horizon": self.horizon.value,
            "market_bb_value": round(self.market_bb_value, 2) if self.market_bb_value is not None else None,
            "market_buy_value": round(self.market_buy_value, 2),
            "market_sell_value": round(self.market_sell_value, 2),
            "market_total_value": round(self.market_total_value, 2),
            "market_oib": round(self.market_oib, 4) if self.market_oib is not None else None,
            "is_warmup": self.is_warmup,
            "direction": self.direction.value,
            "regime": self.regime.value,
            "quality": self.quality.value,
        }


@dataclass(frozen=True)
class FlowBreadthHorizonPoint:
    """Flow breadth metrics across universe constituents for a specific horizon."""
    horizon: BBHorizon
    positive_count: int
    neutral_count: int
    negative_count: int
    total_count: int
    positive_count_ratio: float
    neutral_count_ratio: float
    negative_count_ratio: float
    positive_value: float
    neutral_value: float
    negative_value: float
    total_value: float
    positive_value_ratio: float
    neutral_value_ratio: float
    negative_value_ratio: float
    buffer: float = 0.0

    def to_dict(self) -> Dict[str, Any]:
        return {
            "horizon": self.horizon.value,
            "positive_count": self.positive_count,
            "neutral_count": self.neutral_count,
            "negative_count": self.negative_count,
            "total_count": self.total_count,
            "positive_count_ratio": round(self.positive_count_ratio, 4),
            "neutral_count_ratio": round(self.neutral_count_ratio, 4),
            "negative_count_ratio": round(self.negative_count_ratio, 4),
            "positive_value": round(self.positive_value, 2),
            "neutral_value": round(self.neutral_value, 2),
            "negative_value": round(self.negative_value, 2),
            "total_value": round(self.total_value, 2),
            "positive_value_ratio": round(self.positive_value_ratio, 4),
            "neutral_value_ratio": round(self.neutral_value_ratio, 4),
            "negative_value_ratio": round(self.negative_value_ratio, 4),
            "buffer": self.buffer,
        }


@dataclass(frozen=True)
class MarketBBSessionPoint:
    """Whole-market session point across all requested horizons and breadth."""
    date: str
    horizons: Dict[str, MarketBBHorizonPoint]
    breadth: Dict[str, FlowBreadthHorizonPoint]
    symbol_coverage: float
    value_coverage: float
    reporting_symbols_count: int
    total_target_count: int
    coverage_status: str

    def to_dict(self) -> Dict[str, Any]:
        return {
            "date": self.date,
            "horizons": {h: pt.to_dict() for h, pt in self.horizons.items()},
            "breadth": {h: pt.to_dict() for h, pt in self.breadth.items()},
            "symbol_coverage": round(self.symbol_coverage, 4),
            "value_coverage": round(self.value_coverage, 4),
            "reporting_symbols_count": self.reporting_symbols_count,
            "total_target_count": self.total_target_count,
            "coverage_status": self.coverage_status,
        }


@dataclass(frozen=True)
class MarketBBSeriesResult:
    """Complete multi-horizon Market BB series result and publication gate state."""
    universe_id: str
    universe_version: str
    universe_mode: str
    is_point_in_time: bool
    publication_status: MarketPublicationStatus
    flow_method: FlowMethod = FlowMethod.OHLCV_PROXY
    methodology_version: str = "bb_v1_market_aggregate"
    as_of: Optional[str] = None
    survivor_bias_caveat: Optional[str] = None
    points: List[MarketBBSessionPoint] = field(default_factory=list)
    gate_reasons: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "universe_id": self.universe_id,
            "universe_version": self.universe_version,
            "universe_mode": self.universe_mode,
            "is_point_in_time": self.is_point_in_time,
            "publication_status": self.publication_status.value,
            "flow_method": self.flow_method.value,
            "methodology_version": self.methodology_version,
            "as_of": self.as_of,
            "survivor_bias_caveat": self.survivor_bias_caveat,
            "gate_reasons": list(self.gate_reasons),
            "total_sessions": len(self.points),
            "points": [p.to_dict() for p in self.points],
        }


