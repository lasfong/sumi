"""Candlestick Pattern calculation algorithms (SIG-PAT-001 through SIG-PAT-010).

Strictly pure functions operating on CandleBar primitives.
No lookahead, zero dependencies on DB, ORM, or FastAPI.
All patterns return List[SignalOutputPoint] with quality and explanation reasons.
"""

import math
from typing import Any, Dict, List, Optional

from app.domain.signals.candle_features import (
    calculate_candle_geometry,
    calculate_causal_atr,
)
from app.domain.signals.models import (
    CandleBar,
    SignalOutputPoint,
    SignalQuality,
)


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


def calculate_bullish_engulfing(
    candles: List[CandleBar],
    min_body_ratio: float = 0.55,
    min_close_location: float = 0.70,
    require_quality: bool = True,
) -> List[SignalOutputPoint]:
    """SIG-PAT-001: Bullish Engulfing pattern."""
    points: List[SignalOutputPoint] = []
    if not candles:
        return points

    geoms = [calculate_candle_geometry(b) for b in candles]

    for t, bar in enumerate(candles):
        if t < 1:
            points.append(_insufficient_point(bar))
            continue

        prev_geom = geoms[t - 1]
        curr_geom = geoms[t]
        prev_bar = candles[t - 1]

        prev_bearish = prev_geom.is_bearish
        curr_bullish = curr_geom.is_bullish
        body_engulf = bar.open <= prev_bar.close and bar.close >= prev_bar.open

        quality_ok = (
            curr_geom.body_ratio >= min_body_ratio
            and curr_geom.close_location >= min_close_location
        )

        is_signal = prev_bearish and curr_bullish and body_engulf
        if require_quality:
            is_signal = is_signal and quality_ok

        reasons = []
        if is_signal:
            reasons.append("BULLISH_ENGULFING")
            if quality_ok:
                reasons.append("HIGH_QUALITY")
        else:
            reasons.append("NO_PATTERN")

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_signal,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_bearish_engulfing(
    candles: List[CandleBar],
    min_body_ratio: float = 0.55,
    min_close_location: float = 0.70,
    require_quality: bool = True,
) -> List[SignalOutputPoint]:
    """SIG-PAT-002: Bearish Engulfing pattern."""
    points: List[SignalOutputPoint] = []
    if not candles:
        return points

    geoms = [calculate_candle_geometry(b) for b in candles]

    for t, bar in enumerate(candles):
        if t < 1:
            points.append(_insufficient_point(bar))
            continue

        prev_geom = geoms[t - 1]
        curr_geom = geoms[t]
        prev_bar = candles[t - 1]

        prev_bullish = prev_geom.is_bullish
        curr_bearish = curr_geom.is_bearish
        body_engulf = bar.open >= prev_bar.close and bar.close <= prev_bar.open

        quality_ok = (
            curr_geom.body_ratio >= min_body_ratio
            and curr_geom.close_location <= (1.0 - min_close_location)
        )

        is_signal = prev_bullish and curr_bearish and body_engulf
        if require_quality:
            is_signal = is_signal and quality_ok

        reasons = []
        if is_signal:
            reasons.append("BEARISH_ENGULFING")
            if quality_ok:
                reasons.append("HIGH_QUALITY")
        else:
            reasons.append("NO_PATTERN")

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_signal,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_hammer(
    candles: List[CandleBar],
    wick_multiplier: float = 2.0,
    max_upper_wick_ratio: float = 0.25,
    min_close_location: float = 0.65,
    max_body_ratio: float = 0.50,
) -> List[SignalOutputPoint]:
    """SIG-PAT-003: Hammer / Bullish Pinbar."""
    points: List[SignalOutputPoint] = []
    if not candles:
        return points

    for bar in candles:
        geom = calculate_candle_geometry(bar)
        epsilon = 1e-6
        lower_wick_ok = geom.lower_wick >= wick_multiplier * max(geom.body, epsilon)
        upper_wick_ok = geom.upper_wick_ratio <= max_upper_wick_ratio
        close_loc_ok = geom.close_location >= min_close_location
        body_ratio_ok = geom.body_ratio <= max_body_ratio

        is_signal = lower_wick_ok and upper_wick_ok and close_loc_ok and body_ratio_ok

        reasons = ["HAMMER"] if is_signal else ["NO_PATTERN"]
        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_signal,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_shooting_star(
    candles: List[CandleBar],
    wick_multiplier: float = 2.0,
    max_lower_wick_ratio: float = 0.25,
    max_close_location: float = 0.35,
    max_body_ratio: float = 0.50,
) -> List[SignalOutputPoint]:
    """SIG-PAT-004: Shooting Star / Bearish Pinbar."""
    points: List[SignalOutputPoint] = []
    if not candles:
        return points

    for bar in candles:
        geom = calculate_candle_geometry(bar)
        epsilon = 1e-6
        upper_wick_ok = geom.upper_wick >= wick_multiplier * max(geom.body, epsilon)
        lower_wick_ok = geom.lower_wick_ratio <= max_lower_wick_ratio
        close_loc_ok = geom.close_location <= max_close_location
        body_ratio_ok = geom.body_ratio <= max_body_ratio

        is_signal = upper_wick_ok and lower_wick_ok and close_loc_ok and body_ratio_ok

        reasons = ["SHOOTING_STAR"] if is_signal else ["NO_PATTERN"]
        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_signal,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_morning_star(
    candles: List[CandleBar],
    min_first_body_ratio: float = 0.55,
    max_middle_body_ratio: float = 0.40,
    min_close_location: float = 0.65,
) -> List[SignalOutputPoint]:
    """SIG-PAT-005: Morning Star pattern (3-bar)."""
    points: List[SignalOutputPoint] = []
    if not candles:
        return points

    geoms = [calculate_candle_geometry(b) for b in candles]

    for t, bar in enumerate(candles):
        if t < 2:
            points.append(_insufficient_point(bar))
            continue

        b0 = candles[t - 2]
        g0 = geoms[t - 2]
        g1 = geoms[t - 1]
        g2 = geoms[t]

        # 1. First bar bearish with solid body
        bar0_ok = g0.is_bearish and g0.body_ratio >= min_first_body_ratio
        # 2. Middle bar small body
        bar1_ok = g1.body <= max_middle_body_ratio * max(g0.body, 1e-6)
        # 3. Third bar bullish, closes >= midpoint of first bar
        bar2_ok = g2.is_bullish and bar.close >= g0.midpoint and g2.close_location >= min_close_location

        is_signal = bar0_ok and bar1_ok and bar2_ok
        reasons = ["MORNING_STAR"] if is_signal else ["NO_PATTERN"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_signal,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_evening_star(
    candles: List[CandleBar],
    min_first_body_ratio: float = 0.55,
    max_middle_body_ratio: float = 0.40,
    min_close_location: float = 0.65,
) -> List[SignalOutputPoint]:
    """SIG-PAT-006: Evening Star pattern (3-bar)."""
    points: List[SignalOutputPoint] = []
    if not candles:
        return points

    geoms = [calculate_candle_geometry(b) for b in candles]

    for t, bar in enumerate(candles):
        if t < 2:
            points.append(_insufficient_point(bar))
            continue

        b0 = candles[t - 2]
        g0 = geoms[t - 2]
        g1 = geoms[t - 1]
        g2 = geoms[t]

        # 1. First bar bullish with solid body
        bar0_ok = g0.is_bullish and g0.body_ratio >= min_first_body_ratio
        # 2. Middle bar small body
        bar1_ok = g1.body <= max_middle_body_ratio * max(g0.body, 1e-6)
        # 3. Third bar bearish, closes <= midpoint of first bar
        bar2_ok = g2.is_bearish and bar.close <= g0.midpoint and g2.close_location <= (1.0 - min_close_location)

        is_signal = bar0_ok and bar1_ok and bar2_ok
        reasons = ["EVENING_STAR"] if is_signal else ["NO_PATTERN"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_signal,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_piercing_line(candles: List[CandleBar]) -> List[SignalOutputPoint]:
    """SIG-PAT-007: Piercing Line pattern (2-bar)."""
    points: List[SignalOutputPoint] = []
    if not candles:
        return points

    geoms = [calculate_candle_geometry(b) for b in candles]

    for t, bar in enumerate(candles):
        if t < 1:
            points.append(_insufficient_point(bar))
            continue

        prev_bar = candles[t - 1]
        g_prev = geoms[t - 1]
        g_curr = geoms[t]

        prev_bearish = g_prev.is_bearish
        curr_bullish = g_curr.is_bullish
        open_gap_down = bar.open <= prev_bar.close
        rebound = bar.close >= g_prev.midpoint and bar.close < prev_bar.open
        quality_ok = g_curr.close_location >= 0.60

        is_signal = prev_bearish and curr_bullish and open_gap_down and rebound and quality_ok
        reasons = ["PIERCING_LINE"] if is_signal else ["NO_PATTERN"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_signal,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_dark_cloud_cover(candles: List[CandleBar]) -> List[SignalOutputPoint]:
    """SIG-PAT-007: Dark Cloud Cover pattern (2-bar)."""
    points: List[SignalOutputPoint] = []
    if not candles:
        return points

    geoms = [calculate_candle_geometry(b) for b in candles]

    for t, bar in enumerate(candles):
        if t < 1:
            points.append(_insufficient_point(bar))
            continue

        prev_bar = candles[t - 1]
        g_prev = geoms[t - 1]
        g_curr = geoms[t]

        prev_bullish = g_prev.is_bullish
        curr_bearish = g_curr.is_bearish
        open_gap_up = bar.open >= prev_bar.close
        pullback = bar.close <= g_prev.midpoint and bar.close > prev_bar.open
        quality_ok = g_curr.close_location <= 0.40

        is_signal = prev_bullish and curr_bearish and open_gap_up and pullback and quality_ok
        reasons = ["DARK_CLOUD_COVER"] if is_signal else ["NO_PATTERN"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_signal,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_tweezer_bottom(
    candles: List[CandleBar],
    tolerance_atr: float = 0.15,
) -> List[SignalOutputPoint]:
    """SIG-PAT-008: Tweezer Bottom pattern."""
    points: List[SignalOutputPoint] = []
    if not candles:
        return points

    geoms = [calculate_candle_geometry(b) for b in candles]
    atr_series = calculate_causal_atr(candles, period=14)

    for t, bar in enumerate(candles):
        if t < 14:
            points.append(_insufficient_point(bar))
            continue

        prev_bar = candles[t - 1]
        g_prev = geoms[t - 1]
        g_curr = geoms[t]
        atr = atr_series[t]

        if atr is None or atr <= 1e-8:
            points.append(_insufficient_point(bar, ["INVALID_ATR"]))
            continue

        low_diff = abs(bar.low - prev_bar.low)
        within_tolerance = low_diff <= tolerance_atr * atr
        candle_types_ok = (g_prev.is_bearish or g_prev.is_flat) and g_curr.is_bullish

        is_signal = within_tolerance and candle_types_ok
        reasons = ["TWEEZER_BOTTOM"] if is_signal else ["NO_PATTERN"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_signal,
                quality=SignalQuality.VALID,
                reasons=reasons,
                threshold=tolerance_atr * atr,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_tweezer_top(
    candles: List[CandleBar],
    tolerance_atr: float = 0.15,
) -> List[SignalOutputPoint]:
    """SIG-PAT-008: Tweezer Top pattern."""
    points: List[SignalOutputPoint] = []
    if not candles:
        return points

    geoms = [calculate_candle_geometry(b) for b in candles]
    atr_series = calculate_causal_atr(candles, period=14)

    for t, bar in enumerate(candles):
        if t < 14:
            points.append(_insufficient_point(bar))
            continue

        prev_bar = candles[t - 1]
        g_prev = geoms[t - 1]
        g_curr = geoms[t]
        atr = atr_series[t]

        if atr is None or atr <= 1e-8:
            points.append(_insufficient_point(bar, ["INVALID_ATR"]))
            continue

        high_diff = abs(bar.high - prev_bar.high)
        within_tolerance = high_diff <= tolerance_atr * atr
        candle_types_ok = (g_prev.is_bullish or g_prev.is_flat) and g_curr.is_bearish

        is_signal = within_tolerance and candle_types_ok
        reasons = ["TWEEZER_TOP"] if is_signal else ["NO_PATTERN"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_signal,
                quality=SignalQuality.VALID,
                reasons=reasons,
                threshold=tolerance_atr * atr,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_inside_bar_breakout_up(
    candles: List[CandleBar],
    breakout_buffer_atr: float = 0.05,
) -> List[SignalOutputPoint]:
    """SIG-PAT-009: Inside Bar Breakout Up."""
    points: List[SignalOutputPoint] = []
    if not candles:
        return points

    atr_series = calculate_causal_atr(candles, period=14)

    for t, bar in enumerate(candles):
        if t < 14:
            points.append(_insufficient_point(bar))
            continue

        atr = atr_series[t]
        if atr is None or atr <= 1e-8:
            points.append(_insufficient_point(bar, ["INVALID_ATR"]))
            continue

        # Inside bar at t-1 relative to mother bar at t-2
        mother_bar = candles[t - 2]
        inside_bar = candles[t - 1]

        is_inside = inside_bar.high < mother_bar.high and inside_bar.low > mother_bar.low
        breakout = bar.close > mother_bar.high + breakout_buffer_atr * atr

        is_signal = is_inside and breakout
        reasons = ["INSIDE_BAR_BREAKOUT_UP"] if is_signal else ["NO_PATTERN"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_signal,
                quality=SignalQuality.VALID,
                reasons=reasons,
                threshold=mother_bar.high + breakout_buffer_atr * atr,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_inside_bar_breakout_down(
    candles: List[CandleBar],
    breakout_buffer_atr: float = 0.05,
) -> List[SignalOutputPoint]:
    """SIG-PAT-009: Inside Bar Breakout Down."""
    points: List[SignalOutputPoint] = []
    if not candles:
        return points

    atr_series = calculate_causal_atr(candles, period=14)

    for t, bar in enumerate(candles):
        if t < 14:
            points.append(_insufficient_point(bar))
            continue

        atr = atr_series[t]
        if atr is None or atr <= 1e-8:
            points.append(_insufficient_point(bar, ["INVALID_ATR"]))
            continue

        mother_bar = candles[t - 2]
        inside_bar = candles[t - 1]

        is_inside = inside_bar.high < mother_bar.high and inside_bar.low > mother_bar.low
        breakdown = bar.close < mother_bar.low - breakout_buffer_atr * atr

        is_signal = is_inside and breakdown
        reasons = ["INSIDE_BAR_BREAKOUT_DOWN"] if is_signal else ["NO_PATTERN"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_signal,
                quality=SignalQuality.VALID,
                reasons=reasons,
                threshold=mother_bar.low - breakout_buffer_atr * atr,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_any_bullish_pattern(candles: List[CandleBar]) -> List[SignalOutputPoint]:
    """SIG-PAT-010: Aggregate Bullish Patterns with explicit child reasons."""
    engulf = calculate_bullish_engulfing(candles)
    hammer = calculate_hammer(candles)
    m_star = calculate_morning_star(candles)
    pierce = calculate_piercing_line(candles)
    tweezer = calculate_tweezer_bottom(candles)
    inside_up = calculate_inside_bar_breakout_up(candles)

    points: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        active_children = []
        # Check available patterns
        if engulf[t].quality == SignalQuality.VALID and engulf[t].value is True:
            active_children.append("bullish_engulfing")
        if hammer[t].quality == SignalQuality.VALID and hammer[t].value is True:
            active_children.append("hammer")
        if m_star[t].quality == SignalQuality.VALID and m_star[t].value is True:
            active_children.append("morning_star")
        if pierce[t].quality == SignalQuality.VALID and pierce[t].value is True:
            active_children.append("piercing_line")
        if tweezer[t].quality == SignalQuality.VALID and tweezer[t].value is True:
            active_children.append("tweezer_bottom")
        if inside_up[t].quality == SignalQuality.VALID and inside_up[t].value is True:
            active_children.append("inside_bar_breakout_up")

        has_any = len(active_children) > 0
        reasons = active_children if has_any else ["NO_BULLISH_PATTERN"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=has_any,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_any_bearish_pattern(candles: List[CandleBar]) -> List[SignalOutputPoint]:
    """SIG-PAT-010: Aggregate Bearish Patterns with explicit child reasons."""
    engulf = calculate_bearish_engulfing(candles)
    shooting = calculate_shooting_star(candles)
    e_star = calculate_evening_star(candles)
    dark_cloud = calculate_dark_cloud_cover(candles)
    tweezer = calculate_tweezer_top(candles)
    inside_down = calculate_inside_bar_breakout_down(candles)

    points: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        active_children = []
        if engulf[t].quality == SignalQuality.VALID and engulf[t].value is True:
            active_children.append("bearish_engulfing")
        if shooting[t].quality == SignalQuality.VALID and shooting[t].value is True:
            active_children.append("shooting_star")
        if e_star[t].quality == SignalQuality.VALID and e_star[t].value is True:
            active_children.append("evening_star")
        if dark_cloud[t].quality == SignalQuality.VALID and dark_cloud[t].value is True:
            active_children.append("dark_cloud_cover")
        if tweezer[t].quality == SignalQuality.VALID and tweezer[t].value is True:
            active_children.append("tweezer_top")
        if inside_down[t].quality == SignalQuality.VALID and inside_down[t].value is True:
            active_children.append("inside_bar_breakout_down")

        has_any = len(active_children) > 0
        reasons = active_children if has_any else ["NO_BEARISH_PATTERN"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=has_any,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points