"""Volume Spread Analysis (VSA) and Price-Volume technical inference signals.

Strictly pure functions operating on CandleBar primitives.
Implements:
- SIG-VSA-001: Strong Demand
- SIG-VSA-002: Weak Demand / No Demand
- SIG-VSA-003: Upthrust / Simplified Kéo Xả
- SIG-VSA-004: Spring / Shakeout / Đạp Kéo
- SIG-VSA-005: Strong Demand at Support
- SIG-VSA-006: Strong Supply at Resistance

Strict adherence to non-overclaiming: All metrics are technical inferences
based strictly on price geometry and volume distributions, with zero claims
regarding institutional intent or unverified order-flow telemetry.
"""

import math
from typing import Any, Dict, List, Optional, Tuple

from app.domain.signals.candle_features import (
    calculate_candle_geometry,
    calculate_causal_atr,
)
from app.domain.signals.models import (
    CandleBar,
    SignalOutputPoint,
    SignalQuality,
)
from app.domain.signals.support_resistance import (
    calculate_near_resistance,
    calculate_near_support,
)


def _is_invalid_vol(v: Optional[float]) -> bool:
    if v is None or isinstance(v, bool) or not isinstance(v, (int, float)):
        return True
    return math.isnan(v) or math.isinf(v) or v < 0


def _compute_rvol_and_baselines(
    candles: List[CandleBar],
    period: int,
) -> List[Tuple[Optional[float], Optional[float], Optional[str]]]:
    """Compute (rvol, baseline, error_reason) for each bar t.
    
    Window t - period .. t strictly excludes bar t.
    """
    results = []
    for t, bar in enumerate(candles):
        if t < period:
            results.append((None, None, "INSUFFICIENT_HISTORY"))
            continue

        curr_vol = bar.volume
        if _is_invalid_vol(curr_vol):
            results.append((None, None, "INVALID_VOLUME_CURRENT_BAR"))
            continue

        win = candles[t - period : t]
        if any(_is_invalid_vol(b.volume) for b in win):
            results.append((None, None, "INVALID_VOLUME_IN_WINDOW"))
            continue

        baseline = sum(float(b.volume) for b in win) / period
        if baseline <= 0:
            results.append((None, 0.0, "ZERO_BASELINE"))
            continue

        rvol = float(curr_vol) / baseline
        results.append((rvol, baseline, None))
    return results


def _insufficient_point(
    bar: CandleBar,
    quality: SignalQuality = SignalQuality.INSUFFICIENT_HISTORY,
    reasons: Optional[List[str]] = None,
) -> SignalOutputPoint:
    return SignalOutputPoint(
        bar_index=bar.index,
        timestamp=bar.timestamp,
        output_type="bool",
        value=None,
        quality=quality,
        reasons=reasons or ["INSUFFICIENT_HISTORY"],
        availability_event="BAR_CLOSE",
        available_at_index=bar.index,
        available_at_timestamp=bar.timestamp,
    )


def calculate_strong_demand(
    candles: List[CandleBar],
    period: int = 20,
    min_rvol: float = 1.5,
    min_range_atr: float = 1.2,
    min_body_ratio: float = 0.50,
    min_close_location: float = 0.70,
) -> List[SignalOutputPoint]:
    """SIG-VSA-001: Strong Demand.
    
    Identifies high-volume bullish expansion bars closing near highs:
    - close > close[-1]
    - range / ATR14 >= min_range_atr (default 1.2)
    - body_ratio >= min_body_ratio (default 0.50)
    - close_location >= min_close_location (default 0.70)
    - relative_volume >= min_rvol (default 1.5)
    """
    atr_series = calculate_causal_atr(candles, 14)
    rvol_data = _compute_rvol_and_baselines(candles, period)
    points: List[SignalOutputPoint] = []
    warmup_needed = max(period, 14)

    for t, bar in enumerate(candles):
        if t < warmup_needed or atr_series[t] is None:
            points.append(_insufficient_point(bar))
            continue

        rvol, baseline, err = rvol_data[t]
        if err is not None:
            q = (
                SignalQuality.ZERO_BASELINE
                if err == "ZERO_BASELINE"
                else SignalQuality.INVALID_VOLUME
            )
            points.append(_insufficient_point(bar, quality=q, reasons=[err]))
            continue

        atr = atr_series[t]
        if atr is None or atr <= 1e-8:
            points.append(_insufficient_point(bar, reasons=["INVALID_ATR"]))
            continue

        geom = calculate_candle_geometry(bar)
        prev_bar = candles[t - 1]
        range_atr = geom.range / atr

        is_positive_return = bar.close > prev_bar.close
        is_wide_spread = range_atr >= min_range_atr
        is_strong_body = geom.body_ratio >= min_body_ratio
        is_high_close = geom.close_location >= min_close_location
        is_high_volume = rvol >= min_rvol

        is_demand = bool(
            is_positive_return
            and is_wide_spread
            and is_strong_body
            and is_high_close
            and is_high_volume
        )

        reasons = (
            [
                "VSA_STRONG_DEMAND",
                f"RVOL_{rvol:.2f}",
                f"RANGE_ATR_{range_atr:.2f}",
                f"BODY_{geom.body_ratio:.2f}",
                f"CLOSE_LOC_{geom.close_location:.2f}",
            ]
            if is_demand
            else ["NOT_STRONG_DEMAND"]
        )

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_demand,
                quality=SignalQuality.VALID,
                reasons=reasons,
                baseline=baseline,
                current_volume=float(bar.volume),
                relative_volume=rvol,
                threshold=min_rvol,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return points


def calculate_weak_demand(
    candles: List[CandleBar],
    period: int = 20,
    max_rvol: float = 0.80,
    max_range_atr: float = 0.70,
) -> List[SignalOutputPoint]:
    """SIG-VSA-002: Weak Demand / No Demand.
    
    Identifies low-volume narrow-range upward or flat sessions:
    - close >= close[-1]
    - range / ATR14 <= max_range_atr (default 0.70)
    - relative_volume <= max_rvol (default 0.80)
    """
    atr_series = calculate_causal_atr(candles, 14)
    rvol_data = _compute_rvol_and_baselines(candles, period)
    points: List[SignalOutputPoint] = []
    warmup_needed = max(period, 14)

    for t, bar in enumerate(candles):
        if t < warmup_needed or atr_series[t] is None:
            points.append(_insufficient_point(bar))
            continue

        rvol, baseline, err = rvol_data[t]
        if err is not None:
            q = (
                SignalQuality.ZERO_BASELINE
                if err == "ZERO_BASELINE"
                else SignalQuality.INVALID_VOLUME
            )
            points.append(_insufficient_point(bar, quality=q, reasons=[err]))
            continue

        atr = atr_series[t]
        if atr is None or atr <= 1e-8:
            points.append(_insufficient_point(bar, reasons=["INVALID_ATR"]))
            continue

        geom = calculate_candle_geometry(bar)
        prev_bar = candles[t - 1]
        range_atr = geom.range / atr

        is_flat_or_up = bar.close >= prev_bar.close
        is_narrow_spread = range_atr <= max_range_atr
        is_low_volume = rvol <= max_rvol

        is_weak = bool(is_flat_or_up and is_narrow_spread and is_low_volume)

        reasons = (
            [
                "VSA_WEAK_DEMAND",
                f"RVOL_{rvol:.2f}",
                f"RANGE_ATR_{range_atr:.2f}",
            ]
            if is_weak
            else ["NOT_WEAK_DEMAND"]
        )

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_weak,
                quality=SignalQuality.VALID,
                reasons=reasons,
                baseline=baseline,
                current_volume=float(bar.volume),
                relative_volume=rvol,
                threshold=max_rvol,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return points


def calculate_upthrust(
    candles: List[CandleBar],
    period: int = 20,
    min_rvol: float = 1.5,
    min_upper_wick_ratio: float = 0.35,
    max_close_location: float = 0.45,
    breach_buffer_atr: float = 0.0,
) -> List[SignalOutputPoint]:
    """SIG-VSA-003: Upthrust / Simplified Kéo Xả.
    
    Identifies intra-session breakout above prior resistance that failed by the close:
    - high > prior_resistance + breach_buffer_atr * ATR
    - close < prior_resistance
    - upper_wick_ratio >= min_upper_wick_ratio (default 0.35)
    - close_location <= max_close_location (default 0.45)
    - relative_volume >= min_rvol (default 1.5)
    
    Reference resistance strictly uses prior window candles[t-period : t] (TEST-CAUSAL-003).
    """
    atr_series = calculate_causal_atr(candles, 14)
    rvol_data = _compute_rvol_and_baselines(candles, period)
    points: List[SignalOutputPoint] = []
    warmup_needed = max(period, 14)

    for t, bar in enumerate(candles):
        if t < warmup_needed or atr_series[t] is None:
            points.append(_insufficient_point(bar))
            continue

        rvol, baseline, err = rvol_data[t]
        if err is not None:
            q = (
                SignalQuality.ZERO_BASELINE
                if err == "ZERO_BASELINE"
                else SignalQuality.INVALID_VOLUME
            )
            points.append(_insufficient_point(bar, quality=q, reasons=[err]))
            continue

        atr = atr_series[t]
        if atr is None or atr <= 1e-8:
            points.append(_insufficient_point(bar, reasons=["INVALID_ATR"]))
            continue

        # Prior resistance from historical window excluding bar t
        prior_highs = [candles[i].high for i in range(t - period, t)]
        prior_resistance = max(prior_highs)

        geom = calculate_candle_geometry(bar)
        buffer_dist = breach_buffer_atr * atr

        breached = bar.high > (prior_resistance + buffer_dist)
        failed = bar.close < prior_resistance
        wick_rejection = geom.upper_wick_ratio >= min_upper_wick_ratio
        weak_close = geom.close_location <= max_close_location
        high_volume = rvol >= min_rvol

        is_upthrust = bool(
            breached and failed and wick_rejection and weak_close and high_volume
        )

        reasons = (
            [
                "VSA_UPTHRUST",
                f"PRIOR_RESISTANCE_{prior_resistance:.2f}",
                f"RVOL_{rvol:.2f}",
                f"UPPER_WICK_{geom.upper_wick_ratio:.2f}",
                f"CLOSE_LOC_{geom.close_location:.2f}",
            ]
            if is_upthrust
            else ["NOT_UPTHRUST"]
        )

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_upthrust,
                quality=SignalQuality.VALID,
                reasons=reasons,
                baseline=baseline,
                current_volume=float(bar.volume),
                relative_volume=rvol,
                threshold=min_rvol,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return points


def calculate_spring(
    candles: List[CandleBar],
    period: int = 20,
    min_rvol: float = 1.30,
    min_lower_wick_ratio: float = 0.35,
    min_close_location: float = 0.70,
    breach_buffer_atr: float = 0.0,
) -> List[SignalOutputPoint]:
    """SIG-VSA-004: Spring / Shakeout / Đạp Kéo.
    
    Identifies intra-session breakdown below prior support that was rejected and closed back above:
    - low < prior_support - breach_buffer_atr * ATR
    - close > prior_support
    - lower_wick_ratio >= min_lower_wick_ratio (default 0.35)
    - close_location >= min_close_location (default 0.70)
    - relative_volume >= min_rvol (default 1.30)
    
    Reference support strictly uses prior window candles[t-period : t] (TEST-CAUSAL-003).
    """
    atr_series = calculate_causal_atr(candles, 14)
    rvol_data = _compute_rvol_and_baselines(candles, period)
    points: List[SignalOutputPoint] = []
    warmup_needed = max(period, 14)

    for t, bar in enumerate(candles):
        if t < warmup_needed or atr_series[t] is None:
            points.append(_insufficient_point(bar))
            continue

        rvol, baseline, err = rvol_data[t]
        if err is not None:
            q = (
                SignalQuality.ZERO_BASELINE
                if err == "ZERO_BASELINE"
                else SignalQuality.INVALID_VOLUME
            )
            points.append(_insufficient_point(bar, quality=q, reasons=[err]))
            continue

        atr = atr_series[t]
        if atr is None or atr <= 1e-8:
            points.append(_insufficient_point(bar, reasons=["INVALID_ATR"]))
            continue

        # Prior support from historical window excluding bar t
        prior_lows = [candles[i].low for i in range(t - period, t)]
        prior_support = min(prior_lows)

        geom = calculate_candle_geometry(bar)
        buffer_dist = breach_buffer_atr * atr

        breached = bar.low < (prior_support - buffer_dist)
        reclaimed = bar.close > prior_support
        wick_rejection = geom.lower_wick_ratio >= min_lower_wick_ratio
        strong_close = geom.close_location >= min_close_location
        high_volume = rvol >= min_rvol

        is_spring = bool(
            breached and reclaimed and wick_rejection and strong_close and high_volume
        )

        reasons = (
            [
                "VSA_SPRING",
                f"PRIOR_SUPPORT_{prior_support:.2f}",
                f"RVOL_{rvol:.2f}",
                f"LOWER_WICK_{geom.lower_wick_ratio:.2f}",
                f"CLOSE_LOC_{geom.close_location:.2f}",
            ]
            if is_spring
            else ["NOT_SPRING"]
        )

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_spring,
                quality=SignalQuality.VALID,
                reasons=reasons,
                baseline=baseline,
                current_volume=float(bar.volume),
                relative_volume=rvol,
                threshold=min_rvol,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return points


def calculate_strong_demand_at_support(
    candles: List[CandleBar],
    period: int = 20,
    tolerance_atr: float = 0.5,
    min_rvol: float = 1.30,
) -> List[SignalOutputPoint]:
    """SIG-VSA-005: Strong Demand at Support composite.
    
    Composite logic: near_support AND (strong_demand OR spring).
    """
    near_supp_points = calculate_near_support(candles, period=period, tolerance_atr=tolerance_atr)
    strong_dem_points = calculate_strong_demand(candles, period=period, min_rvol=min_rvol)
    spring_points = calculate_spring(candles, period=period, min_rvol=min_rvol)

    points: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        p_sup = near_supp_points[t]
        p_dem = strong_dem_points[t]
        p_spr = spring_points[t]

        if p_sup.value is None or p_dem.value is None or p_spr.value is None:
            points.append(_insufficient_point(bar))
            continue

        is_near_sup = bool(p_sup.value)
        is_dem = bool(p_dem.value)
        is_spr = bool(p_spr.value)

        is_active = is_near_sup and (is_dem or is_spr)
        reasons: List[str] = []

        if is_active:
            reasons.append("STRONG_DEMAND_AT_SUPPORT")
            # Include matched support source from p_sup reasons
            matched_sources = [r for r in p_sup.reasons if r != "NOT_NEAR_SUPPORT"]
            reasons.extend(matched_sources)
            if is_dem:
                reasons.append("TRIGGER_STRONG_DEMAND")
            if is_spr:
                reasons.append("TRIGGER_SPRING")
        else:
            reasons.append("NOT_DEMAND_AT_SUPPORT")

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_active,
                quality=SignalQuality.VALID,
                reasons=reasons,
                threshold=tolerance_atr,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return points


def calculate_strong_supply_at_resistance(
    candles: List[CandleBar],
    period: int = 20,
    tolerance_atr: float = 0.5,
    min_rvol: float = 1.30,
) -> List[SignalOutputPoint]:
    """SIG-VSA-006: Strong Supply at Resistance composite.
    
    Composite logic: near_resistance AND (upthrust OR strong_bearish_rejection).
    """
    near_res_points = calculate_near_resistance(candles, period=period, tolerance_atr=tolerance_atr)
    upthrust_points = calculate_upthrust(candles, period=period, min_rvol=min_rvol)
    rvol_data = _compute_rvol_and_baselines(candles, period)
    warmup_needed = max(period, 14)

    points: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        if t < warmup_needed:
            points.append(_insufficient_point(bar))
            continue

        p_res = near_res_points[t]
        p_up = upthrust_points[t]
        rvol, baseline, err = rvol_data[t]

        if p_res.value is None or p_up.value is None or rvol is None:
            points.append(_insufficient_point(bar))
            continue

        geom = calculate_candle_geometry(bar)
        prev_bar = candles[t - 1]

        # Strong bearish rejection definition: high volume, negative close, upper wick rejection, weak close location
        is_bearish_rejection = (
            rvol >= min_rvol
            and bar.close < prev_bar.close
            and geom.upper_wick_ratio >= 0.30
            and geom.close_location <= 0.40
        )

        is_near_res = bool(p_res.value)
        is_up = bool(p_up.value)

        is_active = is_near_res and (is_up or is_bearish_rejection)
        reasons: List[str] = []

        if is_active:
            reasons.append("STRONG_SUPPLY_AT_RESISTANCE")
            matched_sources = [r for r in p_res.reasons if r != "NOT_NEAR_RESISTANCE"]
            reasons.extend(matched_sources)
            if is_up:
                reasons.append("TRIGGER_UPTHRUST")
            if is_bearish_rejection:
                reasons.append("TRIGGER_BEARISH_REJECTION")
        else:
            reasons.append("NOT_SUPPLY_AT_RESISTANCE")

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_active,
                quality=SignalQuality.VALID,
                reasons=reasons,
                threshold=tolerance_atr,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return points
