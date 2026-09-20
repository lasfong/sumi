"""Confirmed Pivots and Four-Way Multi-Indicator Divergence calculation algorithms.

Strictly pure functions operating on CandleBar primitives with zero lookahead.
Implements:
- SIG-DIV-001: Causal Confirmed Pivot detection (left_bars=3, right_bars=3)
- SIG-DIV-002: Four-way divergence across RSI, MACD Histogram, and Stochastic
  * Regular Bullish (Price Lower Low + Oscillator Higher Low)
  * Hidden Bullish (Price Higher Low + Oscillator Lower Low)
  * Regular Bearish (Price Higher High + Oscillator Lower High)
  * Hidden Bearish (Price Lower High + Oscillator Higher High)
- TEST-CAUSAL-002: Non-backdated signal availability at confirmation bar t = p + right_bars
- TEST-CAUSAL-001: Future appending invariance
"""

from dataclasses import dataclass
from typing import Any, Dict, List, Literal, Optional, Tuple

from app.domain.signals.models import (
    CandleBar,
    SignalOutputPoint,
    SignalQuality,
)
from app.domain.signals.technical import (
    compute_causal_macd,
    compute_causal_rsi,
)


@dataclass(frozen=True)
class ConfirmedPivot:
    """Represents a confirmed swing price pivot point."""
    pivot_index: int
    confirmed_index: int
    pivot_type: Literal["HIGH", "LOW"]
    price: float
    oscillator_value: Optional[float]
    timestamp: str
    confirmed_timestamp: str


def compute_causal_stochastic_k(
    candles: List[CandleBar],
    k_period: int = 14,
) -> List[Optional[float]]:
    """Compute causal Fast Stochastic %K without future leakage."""
    n = len(candles)
    stoch_k: List[Optional[float]] = [None] * n
    for t in range(n):
        if t < k_period - 1:
            continue
        window = candles[t - k_period + 1 : t + 1]
        hi = max(b.high for b in window)
        lo = min(b.low for b in window)
        diff = hi - lo
        if diff < 1e-8:
            stoch_k[t] = 50.0
        else:
            stoch_k[t] = round(100.0 * (candles[t].close - lo) / diff, 2)
    return stoch_k


def compute_causal_macd_histogram(
    closes: List[float],
    fast: int = 12,
    slow: int = 26,
    signal_period: int = 9,
) -> List[Optional[float]]:
    """Compute causal MACD histogram = MACD line - Signal line."""
    macd_line, signal_line = compute_causal_macd(closes, fast, slow, signal_period)
    n = len(closes)
    hist: List[Optional[float]] = [None] * n
    for t in range(n):
        m = macd_line[t]
        s = signal_line[t]
        if m is not None and s is not None:
            hist[t] = m - s
    return hist


def _is_pivot_high(
    candles: List[CandleBar],
    p: int,
    left_bars: int,
    right_bars: int,
) -> bool:
    """Check if candidate bar p is a strictly confirmed swing high."""
    if p < left_bars or p + right_bars >= len(candles):
        return False
    hi_p = candles[p].high
    for k in range(1, left_bars + 1):
        if hi_p <= candles[p - k].high:
            return False
    for m in range(1, right_bars + 1):
        if hi_p < candles[p + m].high:
            return False
    # Break ties strictly
    if hi_p <= candles[p + right_bars].high:
        return False
    return True


def _is_pivot_low(
    candles: List[CandleBar],
    p: int,
    left_bars: int,
    right_bars: int,
) -> bool:
    """Check if candidate bar p is a strictly confirmed swing low."""
    if p < left_bars or p + right_bars >= len(candles):
        return False
    lo_p = candles[p].low
    for k in range(1, left_bars + 1):
        if lo_p >= candles[p - k].low:
            return False
    for m in range(1, right_bars + 1):
        if lo_p > candles[p + m].low:
            return False
    # Break ties strictly
    if lo_p >= candles[p + right_bars].low:
        return False
    return True


def calculate_confirmed_pivot_high(
    candles: List[CandleBar],
    left_bars: int = 3,
    right_bars: int = 3,
) -> List[SignalOutputPoint]:
    """Calculate confirmed Pivot High event at bar t = p + right_bars."""
    results: List[SignalOutputPoint] = []
    warmup_needed = left_bars + right_bars

    for t, bar in enumerate(candles):
        if t < warmup_needed:
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

        p = t - right_bars
        is_pivot = _is_pivot_high(candles, p, left_bars, right_bars)
        reasons = [f"confirmed_pivot_high_at_{p}", f"price_{candles[p].high:.2f}"] if is_pivot else []

        results.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_pivot,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return results


def calculate_confirmed_pivot_low(
    candles: List[CandleBar],
    left_bars: int = 3,
    right_bars: int = 3,
) -> List[SignalOutputPoint]:
    """Calculate confirmed Pivot Low event at bar t = p + right_bars."""
    results: List[SignalOutputPoint] = []
    warmup_needed = left_bars + right_bars

    for t, bar in enumerate(candles):
        if t < warmup_needed:
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

        p = t - right_bars
        is_pivot = _is_pivot_low(candles, p, left_bars, right_bars)
        reasons = [f"confirmed_pivot_low_at_{p}", f"price_{candles[p].low:.2f}"] if is_pivot else []

        results.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_pivot,
                quality=SignalQuality.VALID,
                reasons=reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )
    return results


def _calculate_divergence_core(
    candles: List[CandleBar],
    oscillator_series: List[Optional[float]],
    divergence_type: Literal[
        "regular_bullish",
        "hidden_bullish",
        "regular_bearish",
        "hidden_bearish",
    ],
    left_bars: int = 3,
    right_bars: int = 3,
    min_separation: int = 5,
    max_separation: int = 60,
    oscillator_label: str = "osc",
) -> List[SignalOutputPoint]:
    """Pure core divergence engine pairing consecutive confirmed pivots."""
    n = len(candles)
    is_bullish_div = "bullish" in divergence_type
    target_pivot_type: Literal["HIGH", "LOW"] = "LOW" if is_bullish_div else "HIGH"

    confirmed_pivots: List[ConfirmedPivot] = []
    results: List[SignalOutputPoint] = []
    warmup_needed = left_bars + right_bars

    for t, bar in enumerate(candles):
        if t < warmup_needed:
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

        p = t - right_bars
        new_pivot_found = False

        if target_pivot_type == "LOW":
            if _is_pivot_low(candles, p, left_bars, right_bars):
                lo_p = candles[p].low
                osc_p = oscillator_series[p]
                new_pivot_found = True
                confirmed_pivots.append(
                    ConfirmedPivot(
                        pivot_index=p,
                        confirmed_index=t,
                        pivot_type="LOW",
                        price=lo_p,
                        oscillator_value=osc_p,
                        timestamp=candles[p].timestamp,
                        confirmed_timestamp=bar.timestamp,
                    )
                )
        else:
            if _is_pivot_high(candles, p, left_bars, right_bars):
                hi_p = candles[p].high
                osc_p = oscillator_series[p]
                new_pivot_found = True
                confirmed_pivots.append(
                    ConfirmedPivot(
                        pivot_index=p,
                        confirmed_index=t,
                        pivot_type="HIGH",
                        price=hi_p,
                        oscillator_value=osc_p,
                        timestamp=candles[p].timestamp,
                        confirmed_timestamp=bar.timestamp,
                    )
                )

        # Check divergence on the current confirmation bar t
        is_divergent = False
        active_reasons: List[str] = []

        if new_pivot_found and len(confirmed_pivots) >= 2:
            p2 = confirmed_pivots[-1]
            p1 = confirmed_pivots[-2]
            sep = p2.pivot_index - p1.pivot_index

            if min_separation <= sep <= max_separation:
                if p1.oscillator_value is not None and p2.oscillator_value is not None:
                    price1, price2 = p1.price, p2.price
                    osc1, osc2 = p1.oscillator_value, p2.oscillator_value

                    if divergence_type == "regular_bullish":
                        # Price Lower Low, Oscillator Higher Low
                        if price2 < price1 and osc2 > osc1:
                            is_divergent = True
                    elif divergence_type == "hidden_bullish":
                        # Price Higher Low, Oscillator Lower Low
                        if price2 > price1 and osc2 < osc1:
                            is_divergent = True
                    elif divergence_type == "regular_bearish":
                        # Price Higher High, Oscillator Lower High
                        if price2 > price1 and osc2 < osc1:
                            is_divergent = True
                    elif divergence_type == "hidden_bearish":
                        # Price Lower High, Oscillator Higher High
                        if price2 < price1 and osc2 > osc1:
                            is_divergent = True

                    if is_divergent:
                        active_reasons = [
                            f"{oscillator_label}_{divergence_type}",
                            f"pivot1_at_{p1.pivot_index}_p_{price1:.2f}_{oscillator_label}_{osc1:.2f}",
                            f"pivot2_at_{p2.pivot_index}_p_{price2:.2f}_{oscillator_label}_{osc2:.2f}",
                            f"separation_{sep}_bars",
                            f"confirmed_at_{t}",
                        ]

        results.append(
            SignalOutputPoint(
                bar_index=bar.index,
                timestamp=bar.timestamp,
                output_type="bool",
                value=is_divergent,
                quality=SignalQuality.VALID,
                reasons=active_reasons,
                availability_event="BAR_CLOSE",
                available_at_index=bar.index,
                available_at_timestamp=bar.timestamp,
            )
        )

    return results


# --- Public RSI Divergence Functions ---

def calculate_rsi_regular_bullish(
    candles: List[CandleBar],
    rsi_period: int = 14,
    left_bars: int = 3,
    right_bars: int = 3,
    min_separation: int = 5,
    max_separation: int = 60,
) -> List[SignalOutputPoint]:
    closes = [b.close for b in candles]
    rsi = compute_causal_rsi(closes, length=rsi_period)
    return _calculate_divergence_core(
        candles, rsi, "regular_bullish", left_bars, right_bars, min_separation, max_separation, "rsi"
    )


def calculate_rsi_regular_bearish(
    candles: List[CandleBar],
    rsi_period: int = 14,
    left_bars: int = 3,
    right_bars: int = 3,
    min_separation: int = 5,
    max_separation: int = 60,
) -> List[SignalOutputPoint]:
    closes = [b.close for b in candles]
    rsi = compute_causal_rsi(closes, length=rsi_period)
    return _calculate_divergence_core(
        candles, rsi, "regular_bearish", left_bars, right_bars, min_separation, max_separation, "rsi"
    )


def calculate_rsi_hidden_bullish(
    candles: List[CandleBar],
    rsi_period: int = 14,
    left_bars: int = 3,
    right_bars: int = 3,
    min_separation: int = 5,
    max_separation: int = 60,
) -> List[SignalOutputPoint]:
    closes = [b.close for b in candles]
    rsi = compute_causal_rsi(closes, length=rsi_period)
    return _calculate_divergence_core(
        candles, rsi, "hidden_bullish", left_bars, right_bars, min_separation, max_separation, "rsi"
    )


def calculate_rsi_hidden_bearish(
    candles: List[CandleBar],
    rsi_period: int = 14,
    left_bars: int = 3,
    right_bars: int = 3,
    min_separation: int = 5,
    max_separation: int = 60,
) -> List[SignalOutputPoint]:
    closes = [b.close for b in candles]
    rsi = compute_causal_rsi(closes, length=rsi_period)
    return _calculate_divergence_core(
        candles, rsi, "hidden_bearish", left_bars, right_bars, min_separation, max_separation, "rsi"
    )


# --- Public MACD Histogram Divergence Functions ---

def calculate_macd_regular_bullish(
    candles: List[CandleBar],
    fast: int = 12,
    slow: int = 26,
    signal_period: int = 9,
    left_bars: int = 3,
    right_bars: int = 3,
    min_separation: int = 5,
    max_separation: int = 60,
) -> List[SignalOutputPoint]:
    closes = [b.close for b in candles]
    hist = compute_causal_macd_histogram(closes, fast, slow, signal_period)
    return _calculate_divergence_core(
        candles, hist, "regular_bullish", left_bars, right_bars, min_separation, max_separation, "macd"
    )


def calculate_macd_regular_bearish(
    candles: List[CandleBar],
    fast: int = 12,
    slow: int = 26,
    signal_period: int = 9,
    left_bars: int = 3,
    right_bars: int = 3,
    min_separation: int = 5,
    max_separation: int = 60,
) -> List[SignalOutputPoint]:
    closes = [b.close for b in candles]
    hist = compute_causal_macd_histogram(closes, fast, slow, signal_period)
    return _calculate_divergence_core(
        candles, hist, "regular_bearish", left_bars, right_bars, min_separation, max_separation, "macd"
    )


def calculate_macd_hidden_bullish(
    candles: List[CandleBar],
    fast: int = 12,
    slow: int = 26,
    signal_period: int = 9,
    left_bars: int = 3,
    right_bars: int = 3,
    min_separation: int = 5,
    max_separation: int = 60,
) -> List[SignalOutputPoint]:
    closes = [b.close for b in candles]
    hist = compute_causal_macd_histogram(closes, fast, slow, signal_period)
    return _calculate_divergence_core(
        candles, hist, "hidden_bullish", left_bars, right_bars, min_separation, max_separation, "macd"
    )


def calculate_macd_hidden_bearish(
    candles: List[CandleBar],
    fast: int = 12,
    slow: int = 26,
    signal_period: int = 9,
    left_bars: int = 3,
    right_bars: int = 3,
    min_separation: int = 5,
    max_separation: int = 60,
) -> List[SignalOutputPoint]:
    closes = [b.close for b in candles]
    hist = compute_causal_macd_histogram(closes, fast, slow, signal_period)
    return _calculate_divergence_core(
        candles, hist, "hidden_bearish", left_bars, right_bars, min_separation, max_separation, "macd"
    )


# --- Public Stochastic Divergence Functions ---

def calculate_stoch_regular_bullish(
    candles: List[CandleBar],
    k_period: int = 14,
    left_bars: int = 3,
    right_bars: int = 3,
    min_separation: int = 5,
    max_separation: int = 60,
) -> List[SignalOutputPoint]:
    stoch = compute_causal_stochastic_k(candles, k_period)
    return _calculate_divergence_core(
        candles, stoch, "regular_bullish", left_bars, right_bars, min_separation, max_separation, "stoch"
    )


def calculate_stoch_regular_bearish(
    candles: List[CandleBar],
    k_period: int = 14,
    left_bars: int = 3,
    right_bars: int = 3,
    min_separation: int = 5,
    max_separation: int = 60,
) -> List[SignalOutputPoint]:
    stoch = compute_causal_stochastic_k(candles, k_period)
    return _calculate_divergence_core(
        candles, stoch, "regular_bearish", left_bars, right_bars, min_separation, max_separation, "stoch"
    )


def calculate_stoch_hidden_bullish(
    candles: List[CandleBar],
    k_period: int = 14,
    left_bars: int = 3,
    right_bars: int = 3,
    min_separation: int = 5,
    max_separation: int = 60,
) -> List[SignalOutputPoint]:
    stoch = compute_causal_stochastic_k(candles, k_period)
    return _calculate_divergence_core(
        candles, stoch, "hidden_bullish", left_bars, right_bars, min_separation, max_separation, "stoch"
    )


def calculate_stoch_hidden_bearish(
    candles: List[CandleBar],
    k_period: int = 14,
    left_bars: int = 3,
    right_bars: int = 3,
    min_separation: int = 5,
    max_separation: int = 60,
) -> List[SignalOutputPoint]:
    stoch = compute_causal_stochastic_k(candles, k_period)
    return _calculate_divergence_core(
        candles, stoch, "hidden_bearish", left_bars, right_bars, min_separation, max_separation, "stoch"
    )
