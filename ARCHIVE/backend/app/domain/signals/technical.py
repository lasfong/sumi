"""Technical Indicator Trigger calculation algorithms (SIG-TECH-001 through SIG-TECH-006).

Strictly pure functions operating on CandleBar primitives.
Authoritative pure causal math: EMA, MACD, RSI, and confirmed Swing Breaks.
No lookahead, zero dependencies on DB, ORM, or FastAPI.
"""

from typing import Any, Dict, List, Optional, Tuple

from app.domain.signals.candle_features import calculate_causal_atr
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


def compute_causal_ema(values: List[float], length: int) -> List[Optional[float]]:
    """Compute causal Exponential Moving Average without future leakage."""
    n = len(values)
    if n == 0 or length < 1:
        return [None] * n

    ema: List[Optional[float]] = [None] * n
    if n < length:
        return ema

    # Seed initial EMA with SMA of first length bars
    k = 2.0 / (length + 1.0)
    sma = sum(values[:length]) / float(length)
    ema[length - 1] = sma

    curr = sma
    for t in range(length, n):
        curr = values[t] * k + curr * (1.0 - k)
        ema[t] = curr

    return ema


def compute_causal_macd(
    closes: List[float],
    fast: int = 12,
    slow: int = 26,
    signal_period: int = 9,
) -> Tuple[List[Optional[float]], List[Optional[float]]]:
    """Compute causal MACD line and Signal line."""
    n = len(closes)
    ema_fast = compute_causal_ema(closes, fast)
    ema_slow = compute_causal_ema(closes, slow)

    macd_line: List[Optional[float]] = [None] * n
    for t in range(n):
        if ema_fast[t] is not None and ema_slow[t] is not None:
            macd_line[t] = ema_fast[t] - ema_slow[t]

    # Signal line is EMA of MACD line over signal_period
    valid_macd_indices = [t for t in range(n) if macd_line[t] is not None]
    signal_line: List[Optional[float]] = [None] * n

    if len(valid_macd_indices) >= signal_period:
        valid_values = [macd_line[t] for t in valid_macd_indices]
        valid_sig = compute_causal_ema(valid_values, signal_period)
        for idx, t in enumerate(valid_macd_indices):
            signal_line[t] = valid_sig[idx]

    return macd_line, signal_line


def compute_causal_rsi(closes: List[float], length: int = 14) -> List[Optional[float]]:
    """Compute causal Relative Strength Index using Wilder smoothing."""
    n = len(closes)
    rsi: List[Optional[float]] = [None] * n
    if n <= length or length < 1:
        return rsi

    gains: List[float] = [0.0] * n
    losses: List[float] = [0.0] * n

    for t in range(1, n):
        diff = closes[t] - closes[t - 1]
        if diff > 0:
            gains[t] = diff
        else:
            losses[t] = -diff

    # Initial SMA of gains and losses
    avg_gain = sum(gains[1 : length + 1]) / float(length)
    avg_loss = sum(losses[1 : length + 1]) / float(length)

    if avg_loss == 0.0:
        rsi[length] = 100.0
    else:
        rs = avg_gain / avg_loss
        rsi[length] = 100.0 - (100.0 / (1.0 + rs))

    # Wilder smoothing for t > length
    for t in range(length + 1, n):
        avg_gain = (avg_gain * (length - 1) + gains[t]) / float(length)
        avg_loss = (avg_loss * (length - 1) + losses[t]) / float(length)

        if avg_loss == 0.0:
            rsi[t] = 100.0
        else:
            rs = avg_gain / avg_loss
            rsi[t] = 100.0 - (100.0 / (1.0 + rs))

    return rsi


def calculate_macd_signal_cross(
    candles: List[CandleBar],
    fast: int = 12,
    slow: int = 26,
    signal_period: int = 9,
    direction: str = "up",
) -> List[SignalOutputPoint]:
    """SIG-TECH-001: MACD line crossing Signal line."""
    closes = [b.close for b in candles]
    macd, sig = compute_causal_macd(closes, fast, slow, signal_period)

    points: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        if t < 1 or macd[t] is None or sig[t] is None or macd[t - 1] is None or sig[t - 1] is None:
            points.append(_insufficient_point(bar))
            continue

        prev_diff = macd[t - 1] - sig[t - 1]
        curr_diff = macd[t] - sig[t]

        if direction == "up":
            is_cross = prev_diff <= 0.0 and curr_diff > 0.0
            reasons = ["MACD_SIGNAL_CROSS_UP"] if is_cross else ["NO_CROSS"]
        else:
            is_cross = prev_diff >= 0.0 and curr_diff < 0.0
            reasons = ["MACD_SIGNAL_CROSS_DOWN"] if is_cross else ["NO_CROSS"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_cross,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_macd_zero_cross(
    candles: List[CandleBar],
    fast: int = 12,
    slow: int = 26,
    direction: str = "up",
) -> List[SignalOutputPoint]:
    """SIG-TECH-002: MACD line crossing Zero."""
    closes = [b.close for b in candles]
    macd, _ = compute_causal_macd(closes, fast, slow, 9)

    points: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        if t < 1 or macd[t] is None or macd[t - 1] is None:
            points.append(_insufficient_point(bar))
            continue

        if direction == "up":
            is_cross = macd[t - 1] <= 0.0 and macd[t] > 0.0
            reasons = ["MACD_ZERO_CROSS_UP"] if is_cross else ["NO_CROSS"]
        else:
            is_cross = macd[t - 1] >= 0.0 and macd[t] < 0.0
            reasons = ["MACD_ZERO_CROSS_DOWN"] if is_cross else ["NO_CROSS"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_cross,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_rsi_level_cross(
    candles: List[CandleBar],
    length: int = 14,
    level: float = 30.0,
    direction: str = "up",
) -> List[SignalOutputPoint]:
    """SIG-TECH-003: RSI level cross."""
    closes = [b.close for b in candles]
    rsi = compute_causal_rsi(closes, length)

    points: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        if t < 1 or rsi[t] is None or rsi[t - 1] is None:
            points.append(_insufficient_point(bar))
            continue

        if direction == "up":
            is_cross = rsi[t - 1] <= level and rsi[t] > level
            reasons = [f"RSI_CROSS_UP_{int(level)}"] if is_cross else ["NO_CROSS"]
        else:
            is_cross = rsi[t - 1] >= level and rsi[t] < level
            reasons = [f"RSI_CROSS_DOWN_{int(level)}"] if is_cross else ["NO_CROSS"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_cross,
                quality=SignalQuality.VALID,
                reasons=reasons,
                threshold=level,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_ema_cross(
    candles: List[CandleBar],
    fast_period: int = 20,
    slow_period: int = 50,
    direction: str = "up",
) -> List[SignalOutputPoint]:
    """SIG-TECH-004: Fast EMA crossing Slow EMA."""
    closes = [b.close for b in candles]
    ema_fast = compute_causal_ema(closes, fast_period)
    ema_slow = compute_causal_ema(closes, slow_period)

    points: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        if t < 1 or ema_fast[t] is None or ema_slow[t] is None or ema_fast[t - 1] is None or ema_slow[t - 1] is None:
            points.append(_insufficient_point(bar))
            continue

        prev_diff = ema_fast[t - 1] - ema_slow[t - 1]
        curr_diff = ema_fast[t] - ema_slow[t]

        if direction == "up":
            is_cross = prev_diff <= 0.0 and curr_diff > 0.0
            reasons = ["EMA_CROSS_UP"] if is_cross else ["NO_CROSS"]
        else:
            is_cross = prev_diff >= 0.0 and curr_diff < 0.0
            reasons = ["EMA_CROSS_DOWN"] if is_cross else ["NO_CROSS"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_cross,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_swing_break(
    candles: List[CandleBar],
    swing_window: int = 3,
    buffer_atr: float = 0.05,
    direction: str = "up",
) -> List[SignalOutputPoint]:
    """SIG-TECH-005: Confirmed swing reference breakout/breakdown.
    
    Invariant: A swing pivot at bar k is confirmed only after swing_window bars have passed (at k + swing_window).
    At bar t, only pivots confirmed on or before t-1 are valid reference targets.
    """
    atr_series = calculate_causal_atr(candles, period=14)
    points: List[SignalOutputPoint] = []
    n = len(candles)

    # Track confirmed swing levels causally
    confirmed_swing_high: Optional[float] = None
    confirmed_swing_low: Optional[float] = None

    for t in range(n):
        bar = candles[t]
        atr = atr_series[t]

        # Check if a new swing was confirmed at t - 1
        # Pivot candidate at idx_pivot = (t - 1) - swing_window
        idx_pivot = t - 1 - swing_window
        if idx_pivot >= swing_window:
            # Check if idx_pivot was a local high
            cand_high = candles[idx_pivot].high
            is_high = all(candles[i].high <= cand_high for i in range(idx_pivot - swing_window, idx_pivot + swing_window + 1))
            if is_high:
                confirmed_swing_high = cand_high

            cand_low = candles[idx_pivot].low
            is_low = all(candles[i].low >= cand_low for i in range(idx_pivot - swing_window, idx_pivot + swing_window + 1))
            if is_low:
                confirmed_swing_low = cand_low

        if t < 14 or atr is None or atr <= 1e-8:
            points.append(_insufficient_point(bar))
            continue

        if direction == "up":
            if confirmed_swing_high is None:
                points.append(_insufficient_point(bar, ["NO_CONFIRMED_SWING_HIGH"]))
                continue
            threshold = confirmed_swing_high + buffer_atr * atr
            is_break = bar.close > threshold
            reasons = ["SWING_HIGH_BREAK"] if is_break else ["NO_BREAK"]
        else:
            if confirmed_swing_low is None:
                points.append(_insufficient_point(bar, ["NO_CONFIRMED_SWING_LOW"]))
                continue
            threshold = confirmed_swing_low - buffer_atr * atr
            is_break = bar.close < threshold
            reasons = ["SWING_LOW_BREAK"] if is_break else ["NO_BREAK"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_break,
                quality=SignalQuality.VALID,
                reasons=reasons,
                threshold=threshold,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points


def calculate_composite_technical_trigger(
    candles: List[CandleBar],
    minimum_confirmations: int = 2,
    direction: str = "up",
) -> List[SignalOutputPoint]:
    """SIG-TECH-006: Bullish/Bearish composite technical trigger with minimum confirmations."""
    macd_sig = calculate_macd_signal_cross(candles, direction=direction)
    macd_zero = calculate_macd_zero_cross(candles, direction=direction)
    rsi_cross = calculate_rsi_level_cross(candles, level=50.0, direction=direction)
    ema_cross = calculate_ema_cross(candles, direction=direction)
    swing = calculate_swing_break(candles, direction=direction)

    points: List[SignalOutputPoint] = []
    for t, bar in enumerate(candles):
        active_triggers = []
        if macd_sig[t].quality == SignalQuality.VALID and macd_sig[t].value is True:
            active_triggers.append("macd_signal_cross")
        if macd_zero[t].quality == SignalQuality.VALID and macd_zero[t].value is True:
            active_triggers.append("macd_zero_cross")
        if rsi_cross[t].quality == SignalQuality.VALID and rsi_cross[t].value is True:
            active_triggers.append("rsi_level_cross")
        if ema_cross[t].quality == SignalQuality.VALID and ema_cross[t].value is True:
            active_triggers.append("ema_cross")
        if swing[t].quality == SignalQuality.VALID and swing[t].value is True:
            active_triggers.append("swing_break")

        is_confirmed = len(active_triggers) >= minimum_confirmations
        reasons = [f"COUNT_{len(active_triggers)}"] + active_triggers if is_confirmed else ["INSUFFICIENT_CONFIRMATIONS"]

        points.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_confirmed,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return points