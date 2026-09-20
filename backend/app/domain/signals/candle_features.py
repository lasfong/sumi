"""Pure geometric candle features and causal Average True Range (ATR).

No dependencies on ORM, DB, FastAPI, or external execution services.
Protects against zero-range candles (doji / flat bars) and numerical precision issues.
"""

from dataclasses import dataclass
import math
from typing import List, Optional

from app.domain.signals.models import CandleBar


@dataclass(frozen=True)
class CandleGeometry:
    """Calculated geometric properties of a single daily candle bar."""
    bar_index: int
    range: float
    body: float
    upper_wick: float
    lower_wick: float
    body_ratio: float
    upper_wick_ratio: float
    lower_wick_ratio: float
    close_location: float
    midpoint: float
    is_bullish: bool
    is_bearish: bool
    is_flat: bool


def calculate_candle_geometry(bar: CandleBar) -> CandleGeometry:
    """Extract geometric single-candle features with strict zero-range protection."""
    c_range = max(0.0, bar.high - bar.low)
    body = abs(bar.close - bar.open)
    upper_wick = max(0.0, bar.high - max(bar.open, bar.close))
    lower_wick = max(0.0, min(bar.open, bar.close) - bar.low)
    midpoint = (bar.open + bar.close) / 2.0

    is_bullish = bar.close > bar.open
    is_bearish = bar.close < bar.open
    is_flat = bar.close == bar.open

    if c_range <= 1e-8:
        # Zero-range bar / flat candle protection
        body_ratio = 0.0
        upper_wick_ratio = 0.0
        lower_wick_ratio = 0.0
        close_location = 0.5  # Neutral center location
    else:
        body_ratio = min(1.0, max(0.0, body / c_range))
        upper_wick_ratio = min(1.0, max(0.0, upper_wick / c_range))
        lower_wick_ratio = min(1.0, max(0.0, lower_wick / c_range))
        close_location = min(1.0, max(0.0, (bar.close - bar.low) / c_range))

    return CandleGeometry(
        bar_index=bar.index,
        range=c_range,
        body=body,
        upper_wick=upper_wick,
        lower_wick=lower_wick,
        body_ratio=body_ratio,
        upper_wick_ratio=upper_wick_ratio,
        lower_wick_ratio=lower_wick_ratio,
        close_location=close_location,
        midpoint=midpoint,
        is_bullish=is_bullish,
        is_bearish=is_bearish,
        is_flat=is_flat,
    )


def calculate_causal_atr(candles: List[CandleBar], period: int = 14) -> List[Optional[float]]:
    """Calculate causal Average True Range (Wilder's RMA smoothing).
    
    Zero future leakage: ATR at index t depends ONLY on bars 0..t.
    Returns None for warmup bars t < period - 1.
    """
    if not candles:
        return []
    if period < 1:
        raise ValueError(f"ATR period must be >= 1, got {period}")

    n = len(candles)
    tr_series: List[float] = [0.0] * n
    atr_series: List[Optional[float]] = [None] * n

    # Compute True Range series
    for t in range(n):
        curr = candles[t]
        if t == 0:
            tr_series[0] = max(0.0, curr.high - curr.low)
        else:
            prev = candles[t - 1]
            h_l = curr.high - curr.low
            h_pc = abs(curr.high - prev.close)
            l_pc = abs(curr.low - prev.close)
            tr_series[t] = max(h_l, h_pc, l_pc)

    if n < period:
        return atr_series

    # Initial ATR is simple average of first 'period' True Ranges
    initial_atr = sum(tr_series[:period]) / float(period)
    atr_series[period - 1] = initial_atr

    # Subsequent bars use Wilder RMA smoothing
    curr_atr = initial_atr
    for t in range(period, n):
        curr_atr = (curr_atr * (period - 1) + tr_series[t]) / float(period)
        atr_series[t] = curr_atr

    return atr_series