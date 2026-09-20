"""Technical Health Score and State Calculation (SIG-HEALTH-001).

Goal: Summarize four distinct, non-overlapping information families:
1. Trend: EMA alignment (Close vs EMA20 / EMA50) and EMA20 slope.
2. Momentum: RSI (Wilder 14).
3. Transition: MACD Histogram state (expanding vs contracting).
4. Participation: Relative Volume OR BB flow confirmation if available.

BB Absence Semantics:
- Missing BB is NOT negative evidence.
- If BB is absent (bb_points is None or missing for bar), participation uses Relative Volume.
- If neither is available, participation is neutral (0.0).
- Explicit reason 'bb_absent_neutral' is emitted for full auditability.

Presets:
- Default versioned preset: 'health_v1_balanced', version '1.0.0'.
- Default weights: {'trend': 0.35, 'momentum': 0.25, 'transition': 0.20, 'participation': 0.20}.
- Composite score normalized to [-100.0, +100.0].
- Favorable threshold: score >= +35.0.
- Unfavorable threshold: score <= -35.0.

Causality:
- Strict point-in-time calculation with zero future leakage.
- Warmup requirement: 50 bars. Prior bars return quality=INSUFFICIENT_HISTORY, value=None.
"""

from typing import Any, Dict, List, Optional, Sequence

from app.domain.signals.models import (
    CandleBar,
    SignalOutputPoint,
    SignalQuality,
    SignalSeriesResult,
    compute_canonical_params_hash,
)
from app.domain.signals.technical import (
    compute_causal_ema,
    compute_causal_macd,
    compute_causal_rsi,
)


def compute_causal_volume_baseline(volumes: Sequence[Optional[float]], period: int = 20) -> List[Optional[float]]:
    """Compute causal rolling average volume baseline over strictly prior bars."""
    n = len(volumes)
    baselines: List[Optional[float]] = [None] * n
    for t in range(period, n):
        win = volumes[t - period : t]
        if any(v is None or v < 0 for v in win):
            continue
        valid_vols = [float(v) for v in win if v is not None]
        if len(valid_vols) == period:
            baselines[t] = sum(valid_vols) / float(period)
    return baselines


HEALTH_PRESETS: Dict[str, Dict[str, float]] = {
    "health_v1_balanced": {
        "trend": 0.35,
        "momentum": 0.25,
        "transition": 0.20,
        "participation": 0.20,
    },
    "health_v1_trend_focused": {
        "trend": 0.50,
        "momentum": 0.20,
        "transition": 0.15,
        "participation": 0.15,
    },
}

DEFAULT_PRESET_NAME = "health_v1_balanced"
HEALTH_METHODOLOGY_VERSION = "health_v1_composite"
DEFAULT_FAVORABLE_THRESHOLD = 35.0
DEFAULT_UNFAVORABLE_THRESHOLD = -35.0
WARMUP_BARS = 50


def _evaluate_trend_family(
    close: float,
    ema20_curr: float,
    ema20_prev: Optional[float],
    ema50_curr: float,
) -> tuple[float, List[str]]:
    """Evaluate Trend family score in [-1.0, +1.0] and audit reasons."""
    reasons: List[str] = []
    
    # Alignment: Close vs EMA20 vs EMA50
    alignment_score = 0.0
    if close > ema20_curr and ema20_curr > ema50_curr:
        alignment_score = 0.5
        reasons.append("trend_ema_bullish_aligned")
    elif close < ema20_curr and ema20_curr < ema50_curr:
        alignment_score = -0.5
        reasons.append("trend_ema_bearish_aligned")
    elif close > ema20_curr:
        alignment_score = 0.25
        reasons.append("trend_above_ema20")
    elif close < ema20_curr:
        alignment_score = -0.25
        reasons.append("trend_below_ema20")

    # Slope: EMA20[t] vs EMA20[t-1]
    slope_score = 0.0
    if ema20_prev is not None:
        if ema20_curr > ema20_prev:
            slope_score = 0.5
            reasons.append("trend_ema20_slope_up")
        elif ema20_curr < ema20_prev:
            slope_score = -0.5
            reasons.append("trend_ema20_slope_down")

    total_trend = max(-1.0, min(1.0, alignment_score + slope_score))
    return total_trend, reasons


def _evaluate_momentum_family(rsi_val: float) -> tuple[float, List[str]]:
    """Evaluate Momentum family score in [-1.0, +1.0] and audit reasons."""
    if rsi_val >= 65.0:
        return 1.0, ["rsi_bullish_strong"]
    elif rsi_val >= 55.0:
        return 0.5, ["rsi_bullish_moderate"]
    elif rsi_val <= 35.0:
        return -1.0, ["rsi_bearish_strong"]
    elif rsi_val <= 45.0:
        return -0.5, ["rsi_bearish_moderate"]
    else:
        return 0.0, ["rsi_neutral"]


def _evaluate_transition_family(
    hist_curr: float,
    hist_prev: Optional[float],
) -> tuple[float, List[str]]:
    """Evaluate Transition family score in [-1.0, +1.0] and audit reasons."""
    if hist_prev is None:
        if hist_curr > 0:
            return 0.5, ["macd_hist_positive"]
        elif hist_curr < 0:
            return -0.5, ["macd_hist_negative"]
        return 0.0, ["macd_hist_neutral"]

    if hist_curr > 0:
        if hist_curr >= hist_prev:
            return 1.0, ["macd_hist_bullish_expanding"]
        else:
            return 0.5, ["macd_hist_bullish_contracting"]
    elif hist_curr < 0:
        if hist_curr >= hist_prev:
            return -0.5, ["macd_hist_bearish_contracting"]
        else:
            return -1.0, ["macd_hist_bearish_expanding"]
    else:
        return 0.0, ["macd_hist_neutral"]


def _evaluate_participation_family(
    bar: CandleBar,
    volume_baseline: Optional[float],
    bb_value: Optional[float],
) -> tuple[float, List[str]]:
    """Evaluate Participation family score in [-1.0, +1.0] and audit reasons.
    
    Enforces BB absence neutral semantics: BB absence is NOT negative evidence.
    """
    reasons: List[str] = []

    # Priority 1: BB Flow confirmation if supplied and valid
    if bb_value is not None:
        if bb_value > 55.0:
            return 1.0, ["bb_flow_bullish"]
        elif bb_value < 45.0:
            return -1.0, ["bb_flow_bearish"]
        else:
            return 0.0, ["bb_flow_neutral"]

    # Priority 2: Fallback to Relative Volume
    reasons.append("bb_absent_neutral")
    curr_vol = bar.volume
    if curr_vol is not None and volume_baseline is not None and volume_baseline > 0:
        rvol = curr_vol / volume_baseline
        if rvol >= 1.3:
            reasons.append("volume_participation_high")
            return 1.0, reasons
        elif rvol >= 1.0:
            reasons.append("volume_participation_normal")
            return 0.5, reasons
        elif rvol <= 0.7:
            reasons.append("volume_participation_low")
            return -0.5, reasons
        else:
            reasons.append("volume_participation_neutral")
            return 0.0, reasons

    # If neither is available, neutral 0.0
    reasons.append("participation_neutral")
    return 0.0, reasons


def calculate_technical_health_score(
    candles: Sequence[CandleBar],
    preset: str = DEFAULT_PRESET_NAME,
    trend_weight: float = 0.35,
    momentum_weight: float = 0.25,
    transition_weight: float = 0.20,
    participation_weight: float = 0.20,
    bb_values: Optional[Sequence[Optional[float]]] = None,
) -> List[SignalOutputPoint]:
    """Calculate composite Technical Health score in [-100.0, +100.0] (SIG-HEALTH-001)."""
    weights = HEALTH_PRESETS.get(preset, HEALTH_PRESETS[DEFAULT_PRESET_NAME])
    w_trend = trend_weight if trend_weight != 0.35 else weights["trend"]
    w_mom = momentum_weight if momentum_weight != 0.25 else weights["momentum"]
    w_trans = transition_weight if transition_weight != 0.20 else weights["transition"]
    w_part = participation_weight if participation_weight != 0.20 else weights["participation"]

    n = len(candles)
    if n == 0:
        return []

    closes = [b.close for b in candles]
    volumes = [b.volume for b in candles]

    # Precalculate pure causal indicators
    ema20 = compute_causal_ema(closes, 20)
    ema50 = compute_causal_ema(closes, 50)
    rsi = compute_causal_rsi(closes, 14)
    macd_line, signal_line = compute_causal_macd(closes, 12, 26, 9)
    vol_baselines = compute_causal_volume_baseline(volumes, 20)

    points: List[SignalOutputPoint] = []

    for t in range(n):
        bar = candles[t]

        # Warmup gate
        if (
            t < WARMUP_BARS
            or ema20[t] is None
            or ema50[t] is None
            or rsi[t] is None
            or macd_line[t] is None
            or signal_line[t] is None
        ):
            points.append(
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

        hist_curr = macd_line[t] - signal_line[t]
        hist_prev = (macd_line[t - 1] - signal_line[t - 1]) if (t > 0 and macd_line[t - 1] is not None and signal_line[t - 1] is not None) else None

        bb_val = bb_values[t] if (bb_values is not None and t < len(bb_values)) else None

        # Evaluate 4 families
        s_trend, r_trend = _evaluate_trend_family(bar.close, ema20[t], ema20[t - 1], ema50[t])
        s_mom, r_mom = _evaluate_momentum_family(rsi[t])
        s_trans, r_trans = _evaluate_transition_family(hist_curr, hist_prev)
        s_part, r_part = _evaluate_participation_family(bar, vol_baselines[t], bb_val)

        # Composite score calculation
        composite_raw = (
            w_trend * s_trend
            + w_mom * s_mom
            + w_trans * s_trans
            + w_part * s_part
        )
        score = round(max(-100.0, min(100.0, composite_raw * 100.0)), 2)

        reasons = r_trend + r_mom + r_trans + r_part

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="float",
                value=score,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return points


def calculate_technical_health_favorable(
    candles: Sequence[CandleBar],
    preset: str = DEFAULT_PRESET_NAME,
    favorable_threshold: float = DEFAULT_FAVORABLE_THRESHOLD,
    trend_weight: float = 0.35,
    momentum_weight: float = 0.25,
    transition_weight: float = 0.20,
    participation_weight: float = 0.20,
    bb_values: Optional[Sequence[Optional[float]]] = None,
) -> List[SignalOutputPoint]:
    """Calculate boolean state: Technical Health is favorable (score >= threshold)."""
    score_points = calculate_technical_health_score(
        candles=candles,
        preset=preset,
        trend_weight=trend_weight,
        momentum_weight=momentum_weight,
        transition_weight=transition_weight,
        participation_weight=participation_weight,
        bb_values=bb_values,
    )
    points: List[SignalOutputPoint] = []
    for pt in score_points:
        if pt.quality != SignalQuality.VALID or pt.value is None:
            points.append(
                SignalOutputPoint(
                    bar_index=pt.bar_index,
                    timestamp=pt.timestamp,
                    output_type="bool",
                    value=None,
                    quality=pt.quality,
                    reasons=pt.reasons,
                    availability_event=pt.availability_event,
                    available_at_index=pt.available_at_index,
                    available_at_timestamp=pt.available_at_timestamp,
                )
            )
        else:
            is_fav = bool(pt.value >= favorable_threshold)
            reasons = list(pt.reasons)
            if is_fav:
                reasons.append(f"health_favorable_score_{pt.value}")
            points.append(
                SignalOutputPoint(
                    bar_index=pt.bar_index,
                    timestamp=pt.timestamp,
                    output_type="bool",
                    value=is_fav,
                    quality=SignalQuality.VALID,
                    reasons=reasons,
                    threshold=favorable_threshold,
                    availability_event=pt.availability_event,
                    available_at_index=pt.available_at_index,
                    available_at_timestamp=pt.available_at_timestamp,
                )
            )

    return points


def calculate_technical_health_unfavorable(
    candles: Sequence[CandleBar],
    preset: str = DEFAULT_PRESET_NAME,
    unfavorable_threshold: float = DEFAULT_UNFAVORABLE_THRESHOLD,
    trend_weight: float = 0.35,
    momentum_weight: float = 0.25,
    transition_weight: float = 0.20,
    participation_weight: float = 0.20,
    bb_values: Optional[Sequence[Optional[float]]] = None,
) -> List[SignalOutputPoint]:
    """Calculate boolean state: Technical Health is unfavorable (score <= threshold)."""
    score_points = calculate_technical_health_score(
        candles=candles,
        preset=preset,
        trend_weight=trend_weight,
        momentum_weight=momentum_weight,
        transition_weight=transition_weight,
        participation_weight=participation_weight,
        bb_values=bb_values,
    )
    points: List[SignalOutputPoint] = []
    for pt in score_points:
        if pt.quality != SignalQuality.VALID or pt.value is None:
            points.append(
                SignalOutputPoint(
                    bar_index=pt.bar_index,
                    timestamp=pt.timestamp,
                    output_type="bool",
                    value=None,
                    quality=pt.quality,
                    reasons=pt.reasons,
                    availability_event=pt.availability_event,
                    available_at_index=pt.available_at_index,
                    available_at_timestamp=pt.available_at_timestamp,
                )
            )
        else:
            is_unfav = bool(pt.value <= unfavorable_threshold)
            reasons = list(pt.reasons)
            if is_unfav:
                reasons.append(f"health_unfavorable_score_{pt.value}")
            points.append(
                SignalOutputPoint(
                    bar_index=pt.bar_index,
                    timestamp=pt.timestamp,
                    output_type="bool",
                    value=is_unfav,
                    quality=SignalQuality.VALID,
                    reasons=reasons,
                    threshold=unfavorable_threshold,
                    availability_event=pt.availability_event,
                    available_at_index=pt.available_at_index,
                    available_at_timestamp=pt.available_at_timestamp,
                )
            )

    return points
