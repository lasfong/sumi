"""Support and Resistance proximity calculation algorithms (SIG-SR-001).

Strictly pure functions operating on CandleBar primitives.
Causally evaluates proximity to rolling N-bar extrema and key EMAs (20, 50, 200).
Reporting reason codes identify exactly which level was matched.
"""

from typing import Any, Dict, List, Optional

from app.domain.signals.candle_features import calculate_causal_atr
from app.domain.signals.models import (
    CandleBar,
    SignalOutputPoint,
    SignalQuality,
)
from app.domain.signals.technical import compute_causal_ema


def _insufficient_point(bar: CandleBar, reasons: Optional[List[str]] = None) -> SignalOutputPoint:
    return SignalOutputPoint(
        bar_index=bar.index,
        timestamp=bar.timestamp,
        output_type="bool",
        value=None,
        quality=SignalQuality.INSUFFICIENT_HISTORY,
        reasons=reasons or ["INSUFFICIENT_HISTORY"],
        availability_event="BAR_CLOSE",
        available_at_index=bar.index,
        available_at_timestamp=bar.timestamp,
    )


def calculate_near_support(
    candles: List[CandleBar],
    period: int = 20,
    tolerance_atr: float = 0.5,
) -> List[SignalOutputPoint]:
    """SIG-SR-001: Price near support level with source attribution in reasons."""
    closes = [b.close for b in candles]
    ema20 = compute_causal_ema(closes, 20)
    ema50 = compute_causal_ema(closes, 50)
    ema200 = compute_causal_ema(closes, 200)
    atr_series = calculate_causal_atr(candles, 14)

    points: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        if t < max(period, 14) or atr_series[t] is None:
            points.append(_insufficient_point(bar))
            continue

        atr = atr_series[t]
        if atr <= 1e-8:
            points.append(_insufficient_point(bar, ["INVALID_ATR"]))
            continue

        tol = tolerance_atr * atr
        matched_sources: List[str] = []

        # 1. Rolling low (strictly t-period to t)
        prior_lows = [candles[i].low for i in range(t - period, t)]
        rolling_low = min(prior_lows)
        if abs(bar.close - rolling_low) <= tol or abs(bar.low - rolling_low) <= tol:
            matched_sources.append(f"ROLLING_LOW_{period}")

        # 2. EMA20
        if ema20[t] is not None and abs(bar.close - ema20[t]) <= tol:
            matched_sources.append("EMA20")

        # 3. EMA50
        if ema50[t] is not None and abs(bar.close - ema50[t]) <= tol:
            matched_sources.append("EMA50")

        # 4. EMA200
        if ema200[t] is not None and abs(bar.close - ema200[t]) <= tol:
            matched_sources.append("EMA200")

        is_near = len(matched_sources) > 0
        reasons = matched_sources if is_near else ["NOT_NEAR_SUPPORT"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_near,
                quality=SignalQuality.VALID,
                reasons=reasons,
                threshold=tol,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_near_resistance(
    candles: List[CandleBar],
    period: int = 20,
    tolerance_atr: float = 0.5,
) -> List[SignalOutputPoint]:
    """SIG-SR-001: Price near resistance level with source attribution in reasons."""
    closes = [b.close for b in candles]
    ema20 = compute_causal_ema(closes, 20)
    ema50 = compute_causal_ema(closes, 50)
    ema200 = compute_causal_ema(closes, 200)
    atr_series = calculate_causal_atr(candles, 14)

    points: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        if t < max(period, 14) or atr_series[t] is None:
            points.append(_insufficient_point(bar))
            continue

        atr = atr_series[t]
        if atr <= 1e-8:
            points.append(_insufficient_point(bar, ["INVALID_ATR"]))
            continue

        tol = tolerance_atr * atr
        matched_sources: List[str] = []

        # 1. Rolling high (strictly t-period to t)
        prior_highs = [candles[i].high for i in range(t - period, t)]
        rolling_high = max(prior_highs)
        if abs(bar.close - rolling_high) <= tol or abs(bar.high - rolling_high) <= tol:
            matched_sources.append(f"ROLLING_HIGH_{period}")

        # 2. EMA20
        if ema20[t] is not None and abs(bar.close - ema20[t]) <= tol:
            matched_sources.append("EMA20")

        # 3. EMA50
        if ema50[t] is not None and abs(bar.close - ema50[t]) <= tol:
            matched_sources.append("EMA50")

        # 4. EMA200
        if ema200[t] is not None and abs(bar.close - ema200[t]) <= tol:
            matched_sources.append("EMA200")

        is_near = len(matched_sources) > 0
        reasons = matched_sources if is_near else ["NOT_NEAR_RESISTANCE"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_near,
                quality=SignalQuality.VALID,
                reasons=reasons,
                threshold=tol,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points