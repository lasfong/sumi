"""Market Regime calculation algorithms (SIG-REG-001 through SIG-REG-006).

Strictly pure functions operating on CandleBar primitives.
Complies with TEST-CAUSAL-003: Comparative extrema strictly exclude the current bar t.
No lookahead, zero dependencies on DB, ORM, or FastAPI.
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


def calculate_uptrend_regime(
    candles: List[CandleBar],
    fast_period: int = 20,
    slow_period: int = 50,
    slope_lookback: int = 5,
) -> List[SignalOutputPoint]:
    """SIG-REG-001: Uptrend Regime."""
    closes = [b.close for b in candles]
    ema_fast = compute_causal_ema(closes, fast_period)
    ema_slow = compute_causal_ema(closes, slow_period)

    points: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        if (
            t < slow_period + slope_lookback
            or ema_fast[t] is None
            or ema_slow[t] is None
            or ema_fast[t - slope_lookback] is None
        ):
            points.append(_insufficient_point(bar))
            continue

        alignment_ok = bar.close > ema_fast[t] > ema_slow[t]
        slope_ok = ema_fast[t] > ema_fast[t - slope_lookback]

        is_uptrend = alignment_ok and slope_ok
        reasons = ["UPTREND"] if is_uptrend else ["NOT_UPTREND"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_uptrend,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_downtrend_regime(
    candles: List[CandleBar],
    fast_period: int = 20,
    slow_period: int = 50,
    slope_lookback: int = 5,
) -> List[SignalOutputPoint]:
    """SIG-REG-002: Downtrend Regime."""
    closes = [b.close for b in candles]
    ema_fast = compute_causal_ema(closes, fast_period)
    ema_slow = compute_causal_ema(closes, slow_period)

    points: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        if (
            t < slow_period + slope_lookback
            or ema_fast[t] is None
            or ema_slow[t] is None
            or ema_fast[t - slope_lookback] is None
        ):
            points.append(_insufficient_point(bar))
            continue

        alignment_ok = bar.close < ema_fast[t] < ema_slow[t]
        slope_ok = ema_fast[t] < ema_fast[t - slope_lookback]

        is_downtrend = alignment_ok and slope_ok
        reasons = ["DOWNTREND"] if is_downtrend else ["NOT_DOWNTREND"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_downtrend,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_sideways_regime(
    candles: List[CandleBar],
    ema_period: int = 20,
    slope_lookback: int = 5,
    slope_threshold: float = 0.008,
) -> List[SignalOutputPoint]:
    """SIG-REG-003: Sideways / Consolidation Regime."""
    closes = [b.close for b in candles]
    ema = compute_causal_ema(closes, ema_period)

    points: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        if t < ema_period + slope_lookback or ema[t] is None or ema[t - slope_lookback] is None:
            points.append(_insufficient_point(bar))
            continue

        prev_val = ema[t - slope_lookback]
        if prev_val <= 1e-8:
            points.append(_insufficient_point(bar, ["ZERO_EMA_BASELINE"]))
            continue

        slope = abs(ema[t] - prev_val) / prev_val
        is_sideways = slope < slope_threshold
        reasons = ["SIDEWAYS"] if is_sideways else ["TRENDING"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_sideways,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_pullback_regime(
    candles: List[CandleBar],
    lookback: int = 20,
    min_drawdown: float = 0.02,
    max_drawdown: float = 0.12,
    tolerance_atr: float = 0.5,
) -> List[SignalOutputPoint]:
    """SIG-REG-004: Pullback / Correction in an Uptrend."""
    closes = [b.close for b in candles]
    ema_fast = compute_causal_ema(closes, 20)
    ema_slow = compute_causal_ema(closes, 50)
    atr_series = calculate_causal_atr(candles, 14)

    points: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        if (
            t < max(50, lookback)
            or ema_fast[t] is None
            or ema_slow[t] is None
            or atr_series[t] is None
        ):
            points.append(_insufficient_point(bar))
            continue

        atr = atr_series[t]
        uptrend_context = ema_fast[t] > ema_slow[t]

        # Prior high excluding current bar t
        window_highs = [candles[i].high for i in range(t - lookback, t)]
        recent_high = max(window_highs)

        if recent_high <= 1e-8:
            points.append(_insufficient_point(bar, ["ZERO_RECENT_HIGH"]))
            continue

        drawdown = (recent_high - bar.close) / recent_high
        dd_ok = min_drawdown <= drawdown <= max_drawdown
        support_level = ema_slow[t] - tolerance_atr * atr
        trend_not_broken = bar.close >= support_level

        is_pullback = uptrend_context and dd_ok and trend_not_broken
        reasons = ["PULLBACK"] if is_pullback else ["NOT_PULLBACK"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_pullback,
                quality=SignalQuality.VALID,
                reasons=reasons,
                threshold=support_level,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_recovery_regime(
    candles: List[CandleBar],
    lookback: int = 20,
) -> List[SignalOutputPoint]:
    """SIG-REG-005: Early Recovery from Downtrend."""
    closes = [b.close for b in candles]
    ema_fast = compute_causal_ema(closes, 20)
    ema_slow = compute_causal_ema(closes, 50)

    points: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        if t < max(50, lookback) or ema_fast[t] is None or ema_slow[t] is None or ema_fast[t - 3] is None:
            points.append(_insufficient_point(bar))
            continue

        downtrend_context = ema_fast[t] < ema_slow[t]
        c1 = bar.close > ema_fast[t]
        c2 = ema_fast[t] > ema_fast[t - 3]
        window_lows = [candles[i].low for i in range(t - lookback, t)]
        recent_low = min(window_lows)
        c3 = bar.close > recent_low * 1.03

        confirmations = sum([c1, c2, c3])
        is_recovery = downtrend_context and confirmations >= 2
        reasons = [f"RECOVERY_CONFIRMATIONS_{confirmations}"] if is_recovery else ["NOT_RECOVERY"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_recovery,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_new_high(
    candles: List[CandleBar],
    period: int = 20,
    buffer: float = 0.0,
) -> List[SignalOutputPoint]:
    """SIG-REG-006 / TEST-CAUSAL-003: New High Signal.
    
    Acceptance Invariant:
    Reference window candles[t-period : t] STRICTLY EXCLUDES the current bar t.
    """
    points: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        if t < period:
            points.append(_insufficient_point(bar))
            continue

        # Invariant: prior window excludes current bar t
        prior_high_window = [candles[i].high for i in range(t - period, t)]
        prior_high = max(prior_high_window)

        is_new_high = bar.high > prior_high + buffer
        reasons = ["NEW_HIGH"] if is_new_high else ["NOT_NEW_HIGH"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_new_high,
                quality=SignalQuality.VALID,
                reasons=reasons,
                threshold=prior_high + buffer,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_new_low(
    candles: List[CandleBar],
    period: int = 20,
    buffer: float = 0.0,
) -> List[SignalOutputPoint]:
    """SIG-REG-006 / TEST-CAUSAL-003: New Low Signal.
    
    Acceptance Invariant:
    Reference window candles[t-period : t] STRICTLY EXCLUDES the current bar t.
    """
    points: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        if t < period:
            points.append(_insufficient_point(bar))
            continue

        # Invariant: prior window excludes current bar t
        prior_low_window = [candles[i].low for i in range(t - period, t)]
        prior_low = min(prior_low_window)

        is_new_low = bar.low < prior_low - buffer
        reasons = ["NEW_LOW"] if is_new_low else ["NOT_NEW_LOW"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_new_low,
                quality=SignalQuality.VALID,
                reasons=reasons,
                threshold=prior_low - buffer,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points