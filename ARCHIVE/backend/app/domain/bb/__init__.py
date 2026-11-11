"""Money Flow Blackbox (BB) domain package.

Defines BB horizons, request/response contracts, and capability guardrails.
"""

from app.domain.bb.calculator import ProxyBBCalculator
from app.domain.bb.contracts import (
    BBDirection,
    BBHorizon,
    BBMetricPoint,
    BBRegime,
    BBSymbolHorizonPoint,
    BBSymbolPoint,
    BBSymbolRequest,
    BBSymbolSeriesResult,
    validate_bb_threshold_semantics,
)

__all__ = [
    "BBDirection",
    "BBHorizon",
    "BBMetricPoint",
    "BBRegime",
    "BBSymbolHorizonPoint",
    "BBSymbolPoint",
    "BBSymbolRequest",
    "BBSymbolSeriesResult",
    "ProxyBBCalculator",
    "validate_bb_threshold_semantics",
]
