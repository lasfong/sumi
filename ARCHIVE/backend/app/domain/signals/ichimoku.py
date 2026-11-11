"""Causal Ichimoku signal calculations and visible cloud boundary evaluation.

Strictly pure functions operating on CandleBar primitives with zero lookahead leak.
Implements:
- SIG-ICHI-001: Ichimoku Score and Bullish/Bearish state
- TEST-ICHI-001: Historical visible cloud honoring displacement D=26
- TEST-CAUSAL-001: Future appending invariance

All factors at bar t strictly utilize market information known at or before bar t.
"""

from typing import Any, Dict, List, Optional, Tuple

from app.domain.signals.models import (
    CandleBar,
    SignalOutputPoint,
    SignalQuality,
)


def _compute_midpoints(
    candles: List[CandleBar],
    period: int,
) -> List[Optional[float]]:
    """Compute (highest high + lowest low) / 2 over rolling window of length `period` ending at bar t (inclusive).
    
    Returns None for indices where history is insufficient (t < period - 1).
    """
    midpoints: List[Optional[float]] = []
    for t in range(len(candles)):
        if t < period - 1:
            midpoints.append(None)
            continue
        window = candles[t - period + 1 : t + 1]
        hi = max(b.high for b in window)
        lo = min(b.low for b in window)
        midpoints.append((hi + lo) / 2.0)
    return midpoints


def _compute_ichimoku_series(
    candles: List[CandleBar],
    tenkan_period: int = 9,
    kijun_period: int = 26,
    senkou_period: int = 52,
    displacement: int = 26,
) -> Tuple[
    List[Optional[float]],  # tenkan
    List[Optional[float]],  # kijun
    List[Optional[float]],  # raw_span_a
    List[Optional[float]],  # raw_span_b
    List[Optional[float]],  # visible_cloud_top
    List[Optional[float]],  # visible_cloud_bottom
]:
    """Compute all raw and visible Ichimoku series causally up to each bar t."""
    n = len(candles)
    tenkan = _compute_midpoints(candles, tenkan_period)
    kijun = _compute_midpoints(candles, kijun_period)

    raw_span_a: List[Optional[float]] = []
    for t in range(n):
        tk = tenkan[t]
        kj = kijun[t]
        if tk is not None and kj is not None:
            raw_span_a.append((tk + kj) / 2.0)
        else:
            raw_span_a.append(None)

    raw_span_b = _compute_midpoints(candles, senkou_period)

    visible_cloud_top: List[Optional[float]] = []
    visible_cloud_bottom: List[Optional[float]] = []

    for t in range(n):
        past_idx = t - displacement
        if past_idx < 0:
            visible_cloud_top.append(None)
            visible_cloud_bottom.append(None)
            continue

        sa_past = raw_span_a[past_idx]
        sb_past = raw_span_b[past_idx]

        if sa_past is not None and sb_past is not None:
            visible_cloud_top.append(max(sa_past, sb_past))
            visible_cloud_bottom.append(min(sa_past, sb_past))
        else:
            visible_cloud_top.append(None)
            visible_cloud_bottom.append(None)

    return tenkan, kijun, raw_span_a, raw_span_b, visible_cloud_top, visible_cloud_bottom


def calculate_ichimoku_score(
    candles: List[CandleBar],
    tenkan_period: int = 9,
    kijun_period: int = 26,
    senkou_period: int = 52,
    displacement: int = 26,
    include_kijun_slope: bool = False,
) -> List[SignalOutputPoint]:
    """Calculate Ichimoku multi-factor score in [-100.0, 100.0].
    
    Four core causal factors evaluated at bar t:
    1. Close vs visible Kumo top/bottom (shifted by displacement D)
    2. Tenkan vs Kijun alignment
    3. Chikou clearance: Close[t] vs Close[t - displacement]
    4. Future Kumo orientation known at t: Raw Span A[t] vs Raw Span B[t]
    5. Optional: Kijun slope (Kijun[t] vs Kijun[t - 1])
    """
    (
        tenkan,
        kijun,
        raw_span_a,
        raw_span_b,
        vis_top,
        vis_bottom,
    ) = _compute_ichimoku_series(
        candles,
        tenkan_period=tenkan_period,
        kijun_period=kijun_period,
        senkou_period=senkou_period,
        displacement=displacement,
    )

    total_factors = 5 if include_kijun_slope else 4
    results: List[SignalOutputPoint] = []

    for t, bar in enumerate(candles):
        # Warmup requires valid visible cloud and past close for Chikou clearance
        top = vis_top[t]
        bottom = vis_bottom[t]
        tk = tenkan[t]
        kj = kijun[t]
        sa = raw_span_a[t]
        sb = raw_span_b[t]
        past_idx = t - displacement

        if (
            top is None
            or bottom is None
            or tk is None
            or kj is None
            or sa is None
            or sb is None
            or past_idx < 0
        ):
            results.append(
                SignalOutputPoint(
                    bar_index=bar.index,
                    timestamp=bar.timestamp,
                    output_type="float",
                    value=None,
                    quality=SignalQuality.INSUFFICIENT_HISTORY,
                    reasons=["INSUFFICIENT_HISTORY"],
                    availability_event="BAR_CLOSE",
                    available_at_index=bar.index,
                    available_at_timestamp=bar.timestamp,
                )
            )
            continue

        bullish_reasons: List[str] = []
        bearish_reasons: List[str] = []

        # 1. Price relative to visible Kumo
        close = bar.close
        if close > top:
            bullish_reasons.append("price_above_cloud")
        elif close < bottom:
            bearish_reasons.append("price_below_cloud")

        # 2. Tenkan vs Kijun
        if tk > kj:
            bullish_reasons.append("tk_bullish")
        elif tk < kj:
            bearish_reasons.append("tk_bearish")

        # 3. Chikou clearance: causal comparison against close D bars ago
        past_close = candles[past_idx].close
        if close > past_close:
            bullish_reasons.append("chikou_above_price")
        elif close < past_close:
            bearish_reasons.append("chikou_below_price")

        # 4. Future Kumo orientation known at t
        if sa > sb:
            bullish_reasons.append("future_cloud_bullish")
        elif sa < sb:
            bearish_reasons.append("future_cloud_bearish")

        # 5. Optional Kijun slope
        if include_kijun_slope:
            prev_kj = kijun[t - 1] if t > 0 else None
            if prev_kj is not None:
                if kj > prev_kj:
                    bullish_reasons.append("kijun_slope_positive")
                elif kj < prev_kj:
                    bearish_reasons.append("kijun_slope_negative")

        bullish_count = len(bullish_reasons)
        bearish_count = len(bearish_reasons)
        score = round(100.0 * (bullish_count - bearish_count) / float(total_factors), 2)

        # Active reasons reflect active directional forces
        active_reasons: List[str] = []
        if score > 0:
            active_reasons = list(bullish_reasons)
        elif score < 0:
            active_reasons = list(bearish_reasons)
        else:
            active_reasons = ["neutral_balance"]

        results.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="float",
                value=score,
                quality=SignalQuality.VALID,
                reasons=active_reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return results


def calculate_ichimoku_bullish(
    candles: List[CandleBar],
    tenkan_period: int = 9,
    kijun_period: int = 26,
    senkou_period: int = 52,
    displacement: int = 26,
    include_kijun_slope: bool = False,
    threshold: float = 50.0,
) -> List[SignalOutputPoint]:
    """Calculate Ichimoku bullish state (score >= threshold, default 50.0)."""
    score_points = calculate_ichimoku_score(
        candles,
        tenkan_period=tenkan_period,
        kijun_period=kijun_period,
        senkou_period=senkou_period,
        displacement=displacement,
        include_kijun_slope=include_kijun_slope,
    )

    results: List[SignalOutputPoint] = []
    for pt in score_points:
        if pt.quality != SignalQuality.VALID or pt.value is None:
            results.append(
                SignalOutputPoint(
                    bar_index=pt.bar_index,
                    timestamp=pt.timestamp,
                    output_type="bool",
                    value=None,
                    quality=pt.quality,
                    reasons=list(pt.reasons),
                    threshold=threshold,
                    availability_event=pt.availability_event,
                    available_at_index=pt.available_at_index,
                    available_at_timestamp=pt.available_at_timestamp,
                )
            )
        else:
            is_bullish = bool(pt.value >= threshold)
            reasons = list(pt.reasons) if is_bullish else []
            results.append(
                SignalOutputPoint(
                    bar_index=pt.bar_index,
                    timestamp=pt.timestamp,
                    output_type="bool",
                    value=is_bullish,
                    quality=SignalQuality.VALID,
                    reasons=reasons,
                    threshold=threshold,
                    availability_event=pt.availability_event,
                    available_at_index=pt.available_at_index,
                    available_at_timestamp=pt.available_at_timestamp,
                )
            )
    return results


def calculate_ichimoku_bearish(
    candles: List[CandleBar],
    tenkan_period: int = 9,
    kijun_period: int = 26,
    senkou_period: int = 52,
    displacement: int = 26,
    include_kijun_slope: bool = False,
    threshold: float = -50.0,
) -> List[SignalOutputPoint]:
    """Calculate Ichimoku bearish state (score <= threshold, default -50.0)."""
    score_points = calculate_ichimoku_score(
        candles,
        tenkan_period=tenkan_period,
        kijun_period=kijun_period,
        senkou_period=senkou_period,
        displacement=displacement,
        include_kijun_slope=include_kijun_slope,
    )

    results: List[SignalOutputPoint] = []
    for pt in score_points:
        if pt.quality != SignalQuality.VALID or pt.value is None:
            results.append(
                SignalOutputPoint(
                    bar_index=pt.bar_index,
                    timestamp=pt.timestamp,
                    output_type="bool",
                    value=None,
                    quality=pt.quality,
                    reasons=list(pt.reasons),
                    threshold=threshold,
                    availability_event=pt.availability_event,
                    available_at_index=pt.available_at_index,
                    available_at_timestamp=pt.available_at_timestamp,
                )
            )
        else:
            is_bearish = bool(pt.value <= threshold)
            reasons = list(pt.reasons) if is_bearish else []
            results.append(
                SignalOutputPoint(
                    bar_index=pt.bar_index,
                    timestamp=pt.timestamp,
                    output_type="bool",
                    value=is_bearish,
                    quality=SignalQuality.VALID,
                    reasons=reasons,
                    threshold=threshold,
                    availability_event=pt.availability_event,
                    available_at_index=pt.available_at_index,
                    available_at_timestamp=pt.available_at_timestamp,
                )
            )
    return results


def calculate_ichimoku_tk_cross_bullish(
    candles: List[CandleBar],
    tenkan_period: int = 9,
    kijun_period: int = 26,
) -> List[SignalOutputPoint]:
    """Calculate bullish Tenkan/Kijun crossover event (Tenkan crosses strictly above Kijun)."""
    tenkan = _compute_midpoints(candles, tenkan_period)
    kijun = _compute_midpoints(candles, kijun_period)

    results: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        tk = tenkan[t]
        kj = kijun[t]
        prev_tk = tenkan[t - 1] if t > 0 else None
        prev_kj = kijun[t - 1] if t > 0 else None

        if tk is None or kj is None or prev_tk is None or prev_kj is None:
            results.append(
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

        is_cross = bool(tk > kj and prev_tk <= prev_kj)
        results.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_cross,
                quality=SignalQuality.VALID,
                reasons=["tk_cross_bullish"] if is_cross else [],
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return results


def calculate_ichimoku_tk_cross_bearish(
    candles: List[CandleBar],
    tenkan_period: int = 9,
    kijun_period: int = 26,
) -> List[SignalOutputPoint]:
    """Calculate bearish Tenkan/Kijun crossunder event (Tenkan crosses strictly below Kijun)."""
    tenkan = _compute_midpoints(candles, tenkan_period)
    kijun = _compute_midpoints(candles, kijun_period)

    results: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        tk = tenkan[t]
        kj = kijun[t]
        prev_tk = tenkan[t - 1] if t > 0 else None
        prev_kj = kijun[t - 1] if t > 0 else None

        if tk is None or kj is None or prev_tk is None or prev_kj is None:
            results.append(
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

        is_cross = bool(tk < kj and prev_tk >= prev_kj)
        results.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_cross,
                quality=SignalQuality.VALID,
                reasons=["tk_cross_bearish"] if is_cross else [],
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return results


def calculate_ichimoku_kumo_breakout_bullish(
    candles: List[CandleBar],
    tenkan_period: int = 9,
    kijun_period: int = 26,
    senkou_period: int = 52,
    displacement: int = 26,
) -> List[SignalOutputPoint]:
    """Calculate bullish Kumo breakout event (Close crosses strictly above visible cloud top)."""
    (
        _,
        _,
        _,
        _,
        vis_top,
        _,
    ) = _compute_ichimoku_series(
        candles,
        tenkan_period=tenkan_period,
        kijun_period=kijun_period,
        senkou_period=senkou_period,
        displacement=displacement,
    )

    results: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        top = vis_top[t]
        prev_top = vis_top[t - 1] if t > 0 else None

        if top is None or prev_top is None or t == 0:
            results.append(
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

        curr_above = bar.close > top
        prev_above = candles[t - 1].close > prev_top
        is_breakout = bool(curr_above and not prev_above)

        results.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_breakout,
                quality=SignalQuality.VALID,
                reasons=["kumo_breakout_bullish"] if is_breakout else [],
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return results


def calculate_ichimoku_kumo_breakout_bearish(
    candles: List[CandleBar],
    tenkan_period: int = 9,
    kijun_period: int = 26,
    senkou_period: int = 52,
    displacement: int = 26,
) -> List[SignalOutputPoint]:
    """Calculate bearish Kumo breakout event (Close crosses strictly below visible cloud bottom)."""
    (
        _,
        _,
        _,
        _,
        _,
        vis_bottom,
    ) = _compute_ichimoku_series(
        candles,
        tenkan_period=tenkan_period,
        kijun_period=kijun_period,
        senkou_period=senkou_period,
        displacement=displacement,
    )

    results: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        bottom = vis_bottom[t]
        prev_bottom = vis_bottom[t - 1] if t > 0 else None

        if bottom is None or prev_bottom is None or t == 0:
            results.append(
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

        curr_below = bar.close < bottom
        prev_below = candles[t - 1].close < prev_bottom
        is_breakout = bool(curr_below and not prev_below)

        results.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_breakout,
                quality=SignalQuality.VALID,
                reasons=["kumo_breakout_bearish"] if is_breakout else [],
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return results
