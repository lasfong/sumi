"""Relative Volume and Volume Spike signal calculation algorithms.

Strictly pure functions operating on CandleBar primitives.
No dependencies on ORM, DB, FastAPI, or external execution services.
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


def validate_relative_volume_params(params: Dict[str, Any]) -> Dict[str, int]:
    """Validate and normalize Relative Volume parameters.
    
    Allowed parameters:
    - period: strict integer, default 20, range 1-252.
    """
    allowed_keys = {"period"}
    unknown = set(params.keys()) - allowed_keys
    if unknown:
        raise ValueError(f"Unknown parameter(s) for volume.relative_volume: {', '.join(sorted(unknown))}")

    period_val = params.get("period", 20)
    if period_val is None:
        raise ValueError("Parameter 'period' cannot be null")
    if isinstance(period_val, bool) or not isinstance(period_val, int):
        raise ValueError(f"Parameter 'period' must be a strict integer, got {type(period_val).__name__}")
    if period_val < 1 or period_val > 252:
        raise ValueError(f"Parameter 'period' must be between 1 and 252, got {period_val}")

    return {"period": period_val}


def validate_volume_spike_params(params: Dict[str, Any]) -> Dict[str, Any]:
    """Validate and normalize Volume Spike parameters.
    
    Allowed parameters:
    - period: strict integer, default 20, range 1-252.
    - multiplier: finite positive number, default 2.0, range > 0 and <= 100.
    """
    allowed_keys = {"period", "multiplier"}
    unknown = set(params.keys()) - allowed_keys
    if unknown:
        raise ValueError(f"Unknown parameter(s) for volume.spike: {', '.join(sorted(unknown))}")

    # Validate period
    period_val = params.get("period", 20)
    if period_val is None:
        raise ValueError("Parameter 'period' cannot be null")
    if isinstance(period_val, bool) or not isinstance(period_val, int):
        raise ValueError(f"Parameter 'period' must be a strict integer, got {type(period_val).__name__}")
    if period_val < 1 or period_val > 252:
        raise ValueError(f"Parameter 'period' must be between 1 and 252, got {period_val}")

    # Validate multiplier
    multiplier_val = params.get("multiplier", 2.0)
    if multiplier_val is None:
        raise ValueError("Parameter 'multiplier' cannot be null")
    if isinstance(multiplier_val, bool) or not isinstance(multiplier_val, (int, float)):
        raise ValueError(f"Parameter 'multiplier' must be a finite number, got {type(multiplier_val).__name__}")
    if math.isnan(multiplier_val) or math.isinf(multiplier_val):
        raise ValueError("Parameter 'multiplier' cannot be NaN or Infinity")
    if multiplier_val <= 0 or multiplier_val > 100:
        raise ValueError(f"Parameter 'multiplier' must be > 0 and <= 100, got {multiplier_val}")

    return {"period": period_val, "multiplier": float(multiplier_val)}


def _is_invalid_volume(v: Optional[float]) -> bool:
    """Check if a volume observation is missing, negative, or non-finite."""
    if v is None:
        return True
    if isinstance(v, bool):
        return True
    if not isinstance(v, (int, float)):
        return True
    if math.isnan(v) or math.isinf(v):
        return True
    if v < 0:
        return True
    return False


def calculate_relative_volume(candles: List[CandleBar], period: int) -> List[SignalOutputPoint]:
    """Calculate Relative Volume series across chronologically ordered candle bars.
    
    Formula at index t:
    - If t < period: INSUFFICIENT_HISTORY, null value.
    - Window = candles[t-period : t] (strictly excludes current bar t).
    - If current volume or any window volume is invalid/negative/non-finite: INVALID_VOLUME, null value.
    - Baseline = mean(volume in window).
    - If baseline == 0: ZERO_BASELINE, null value.
    - If baseline > 0: VALID, RVOL = current_volume / baseline.
    """
    points: List[SignalOutputPoint] = []

    for t, bar in enumerate(candles):
        curr_vol = bar.volume

        if t < period:
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="float",
                    value=None,
                    quality=SignalQuality.INSUFFICIENT_HISTORY,
                    reasons=["INSUFFICIENT_HISTORY"],
                    baseline=None,
                    current_volume=None if _is_invalid_volume(curr_vol) else float(curr_vol),
                    relative_volume=None,
                    threshold=None,
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        # Prior window check
        window = candles[t - period : t]
        window_invalid = any(_is_invalid_volume(c.volume) for c in window)
        curr_invalid = _is_invalid_volume(curr_vol)

        if window_invalid or curr_invalid:
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="float",
                    value=None,
                    quality=SignalQuality.INVALID_VOLUME,
                    reasons=["INVALID_VOLUME"],
                    baseline=None,
                    current_volume=None if curr_invalid else float(curr_vol),
                    relative_volume=None,
                    threshold=None,
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        try:
            baseline = sum(float(c.volume) for c in window) / float(period)
        except OverflowError:
            baseline = float("inf")

        if not math.isfinite(baseline):
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="float",
                    value=None,
                    quality=SignalQuality.INVALID_VOLUME,
                    reasons=["NON_FINITE_DERIVED_VALUE"],
                    baseline=None,
                    current_volume=float(curr_vol) if math.isfinite(curr_vol) else None,
                    relative_volume=None,
                    threshold=None,
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        if baseline == 0.0:
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="float",
                    value=None,
                    quality=SignalQuality.ZERO_BASELINE,
                    reasons=["ZERO_BASELINE"],
                    baseline=0.0,
                    current_volume=float(curr_vol),
                    relative_volume=None,
                    threshold=None,
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        try:
            rvol = float(curr_vol) / baseline
        except OverflowError:
            rvol = float("inf")

        if not math.isfinite(rvol):
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="float",
                    value=None,
                    quality=SignalQuality.INVALID_VOLUME,
                    reasons=["NON_FINITE_DERIVED_VALUE"],
                    baseline=baseline if math.isfinite(baseline) else None,
                    current_volume=float(curr_vol) if math.isfinite(curr_vol) else None,
                    relative_volume=None,
                    threshold=None,
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="float",
                value=rvol,
                quality=SignalQuality.VALID,
                reasons=["VALID"],
                baseline=baseline,
                current_volume=float(curr_vol),
                relative_volume=rvol,
                threshold=None,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return points


def calculate_volume_spike(candles: List[CandleBar], period: int, multiplier: float) -> List[SignalOutputPoint]:
    """Calculate Volume Spike series across chronologically ordered candle bars.
    
    Formula at index t:
    - Same window and quality rules as Relative Volume.
    - If quality is VALID:
        spike = bool(relative_volume >= multiplier) (no rounding before comparison).
        value = spike
        reasons = ["VOLUME_SPIKE"] if spike else ["NORMAL_VOLUME"]
        threshold = multiplier
    """
    points: List[SignalOutputPoint] = []

    for t, bar in enumerate(candles):
        curr_vol = bar.volume

        if t < period:
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.INSUFFICIENT_HISTORY,
                    reasons=["INSUFFICIENT_HISTORY"],
                    baseline=None,
                    current_volume=None if _is_invalid_volume(curr_vol) else float(curr_vol),
                    relative_volume=None,
                    threshold=multiplier,
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        # Prior window check
        window = candles[t - period : t]
        window_invalid = any(_is_invalid_volume(c.volume) for c in window)
        curr_invalid = _is_invalid_volume(curr_vol)

        if window_invalid or curr_invalid:
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.INVALID_VOLUME,
                    reasons=["INVALID_VOLUME"],
                    baseline=None,
                    current_volume=None if curr_invalid else float(curr_vol),
                    relative_volume=None,
                    threshold=multiplier,
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        try:
            baseline = sum(float(c.volume) for c in window) / float(period)
        except OverflowError:
            baseline = float("inf")

        if not math.isfinite(baseline):
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.INVALID_VOLUME,
                    reasons=["NON_FINITE_DERIVED_VALUE"],
                    baseline=None,
                    current_volume=float(curr_vol) if math.isfinite(curr_vol) else None,
                    relative_volume=None,
                    threshold=multiplier if math.isfinite(multiplier) else None,
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        if baseline == 0.0:
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.ZERO_BASELINE,
                    reasons=["ZERO_BASELINE"],
                    baseline=0.0,
                    current_volume=float(curr_vol),
                    relative_volume=None,
                    threshold=multiplier,
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        try:
            rvol = float(curr_vol) / baseline
        except OverflowError:
            rvol = float("inf")

        if not math.isfinite(rvol):
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.INVALID_VOLUME,
                    reasons=["NON_FINITE_DERIVED_VALUE"],
                    baseline=baseline if math.isfinite(baseline) else None,
                    current_volume=float(curr_vol) if math.isfinite(curr_vol) else None,
                    relative_volume=None,
                    threshold=multiplier if math.isfinite(multiplier) else None,
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        is_spike = bool(rvol >= multiplier)

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_spike,
                quality=SignalQuality.VALID,
                reasons=["VOLUME_SPIKE"] if is_spike else ["NORMAL_VOLUME"],
                baseline=baseline,
                current_volume=float(curr_vol),
                relative_volume=rvol,
                threshold=multiplier,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return points


def calculate_volume_climax_up(
    candles: List[CandleBar],
    period: int = 20,
    multiplier: float = 2.0,
    min_range_atr: float = 1.5,
    min_close_location: float = 0.75,
) -> List[SignalOutputPoint]:
    """SIG-VOL-003: Volume Climax Up.
    
    Identifies wide-range bars on high relative volume closing in the upper quartile:
    - relative_volume >= multiplier (default 2.0)
    - range / ATR14 >= min_range_atr (default 1.5)
    - close_location >= min_close_location (default 0.75)
    """
    atr_series = calculate_causal_atr(candles, 14)
    points: List[SignalOutputPoint] = []
    warmup_needed = max(period, 14)

    for t, bar in enumerate(candles):
        if t < warmup_needed or atr_series[t] is None:
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.INSUFFICIENT_HISTORY,
                    reasons=["INSUFFICIENT_HISTORY"],
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        curr_vol = bar.volume
        if _is_invalid_volume(curr_vol):
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.INVALID_VOLUME,
                    reasons=["INVALID_VOLUME_CURRENT_BAR"],
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        window_bars = candles[t - period : t]
        has_invalid_win = any(_is_invalid_volume(b.volume) for b in window_bars)
        if has_invalid_win:
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.INVALID_VOLUME,
                    reasons=["INVALID_VOLUME_IN_WINDOW"],
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        baseline = sum(float(b.volume) for b in window_bars) / period
        if baseline <= 0:
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.ZERO_BASELINE,
                    reasons=["ZERO_BASELINE"],
                    baseline=0.0,
                    current_volume=float(curr_vol),
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        rvol = float(curr_vol) / baseline
        atr = atr_series[t]
        if atr is None or atr <= 1e-8:
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.INSUFFICIENT_HISTORY,
                    reasons=["INVALID_ATR"],
                    baseline=baseline,
                    current_volume=float(curr_vol),
                    relative_volume=rvol,
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        geom = calculate_candle_geometry(bar)
        range_atr = geom.range / atr
        is_climax = bool(
            rvol >= multiplier
            and range_atr >= min_range_atr
            and geom.close_location >= min_close_location
        )

        reasons = (
            [
                "VOLUME_CLIMAX_UP",
                f"RVOL_{rvol:.2f}",
                f"RANGE_ATR_{range_atr:.2f}",
                f"CLOSE_LOC_{geom.close_location:.2f}",
            ]
            if is_climax
            else ["NOT_VOLUME_CLIMAX_UP"]
        )

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_climax,
                quality=SignalQuality.VALID,
                reasons=reasons,
                baseline=baseline,
                current_volume=float(curr_vol),
                relative_volume=rvol,
                threshold=multiplier,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return points


def calculate_volume_climax_down(
    candles: List[CandleBar],
    period: int = 20,
    multiplier: float = 2.0,
    min_range_atr: float = 1.5,
    max_close_location: float = 0.25,
) -> List[SignalOutputPoint]:
    """SIG-VOL-003: Volume Climax Down.
    
    Identifies wide-range bars on high relative volume closing in the lower quartile:
    - relative_volume >= multiplier (default 2.0)
    - range / ATR14 >= min_range_atr (default 1.5)
    - close_location <= max_close_location (default 0.25)
    """
    atr_series = calculate_causal_atr(candles, 14)
    points: List[SignalOutputPoint] = []
    warmup_needed = max(period, 14)

    for t, bar in enumerate(candles):
        if t < warmup_needed or atr_series[t] is None:
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.INSUFFICIENT_HISTORY,
                    reasons=["INSUFFICIENT_HISTORY"],
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        curr_vol = bar.volume
        if _is_invalid_volume(curr_vol):
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.INVALID_VOLUME,
                    reasons=["INVALID_VOLUME_CURRENT_BAR"],
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        window_bars = candles[t - period : t]
        has_invalid_win = any(_is_invalid_volume(b.volume) for b in window_bars)
        if has_invalid_win:
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.INVALID_VOLUME,
                    reasons=["INVALID_VOLUME_IN_WINDOW"],
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        baseline = sum(float(b.volume) for b in window_bars) / period
        if baseline <= 0:
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.ZERO_BASELINE,
                    reasons=["ZERO_BASELINE"],
                    baseline=0.0,
                    current_volume=float(curr_vol),
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        rvol = float(curr_vol) / baseline
        atr = atr_series[t]
        if atr is None or atr <= 1e-8:
            points.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="bool",
                    value=None,
                    quality=SignalQuality.INSUFFICIENT_HISTORY,
                    reasons=["INVALID_ATR"],
                    baseline=baseline,
                    current_volume=float(curr_vol),
                    relative_volume=rvol,
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        geom = calculate_candle_geometry(bar)
        range_atr = geom.range / atr
        is_climax = bool(
            rvol >= multiplier
            and range_atr >= min_range_atr
            and geom.close_location <= max_close_location
        )

        reasons = (
            [
                "VOLUME_CLIMAX_DOWN",
                f"RVOL_{rvol:.2f}",
                f"RANGE_ATR_{range_atr:.2f}",
                f"CLOSE_LOC_{geom.close_location:.2f}",
            ]
            if is_climax
            else ["NOT_VOLUME_CLIMAX_DOWN"]
        )

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_climax,
                quality=SignalQuality.VALID,
                reasons=reasons,
                baseline=baseline,
                current_volume=float(curr_vol),
                relative_volume=rvol,
                threshold=multiplier,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return points

