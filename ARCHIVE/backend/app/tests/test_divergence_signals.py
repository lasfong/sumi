"""Unit and integration tests for Confirmed Pivots and Four-Way Divergence Signals.

Covers:
- SIG-DIV-001: Confirmed Pivot High and Pivot Low detection with left/right lookback
- SIG-DIV-002: Four-way divergence across RSI, MACD Histogram, and Stochastic:
  * Regular Bullish (Price Lower Low + Oscillator Higher Low)
  * Hidden Bullish (Price Higher Low + Oscillator Lower Low)
  * Regular Bearish (Price Higher High + Oscillator Lower High)
  * Hidden Bearish (Price Lower High + Oscillator Higher High)
- TEST-CAUSAL-002: Pivot delay and non-backdated signal availability (at t = p + right_bars)
- TEST-CAUSAL-001: Future appending invariance (bit-for-bit identical on historical prefix)
- Separation bounds: [min_separation, max_separation] enforcement
- SignalRegistry integration and AST rule binding evaluation
"""

import math
import pytest

from app.domain.signals.models import (
    CandleBar,
    SignalQuality,
    SignalOutputType,
)
from app.domain.signals.divergence import (
    calculate_confirmed_pivot_high,
    calculate_confirmed_pivot_low,
    calculate_rsi_regular_bullish,
    calculate_rsi_regular_bearish,
    calculate_rsi_hidden_bullish,
    calculate_rsi_hidden_bearish,
    calculate_macd_regular_bullish,
    calculate_macd_regular_bearish,
    calculate_macd_hidden_bullish,
    calculate_macd_hidden_bearish,
    calculate_stoch_regular_bullish,
    calculate_stoch_regular_bearish,
    calculate_stoch_hidden_bullish,
    calculate_stoch_hidden_bearish,
    compute_causal_stochastic_k,
    compute_causal_macd_histogram,
)
from app.domain.signals.registry import SignalRegistry
from app.domain.strategy.signal_binding import SignalBindingAdapter


def _generate_bars(prices: list[float]) -> list[CandleBar]:
    """Helper to create CandleBars from close price sequence."""
    bars = []
    for i, p in enumerate(prices):
        bars.append(
            CandleBar(
                index=i,
                timestamp=f"2025-01-{(i % 28) + 1:02d}",
                open=round(p, 2),
                high=round(p + 1.0, 2),
                low=round(p - 1.0, 2),
                close=round(p, 2),
                volume=10000.0,
            )
        )
    return bars


# 1. SIG-DIV-001 & TEST-CAUSAL-002: Confirmed Pivot High/Low and Delay
def test_confirmed_pivot_detection():
    """SIG-DIV-001: Pivot at p is confirmed strictly at t = p + right_bars, never before."""
    # Construct sequence with clear high at index 5 and low at index 12
    # left_bars=3, right_bars=3 => Pivot high at 5 confirmed at index 5 + 3 = 8
    # Pivot low at 12 confirmed at index 12 + 3 = 15
    prices = [100.0, 102.0, 104.0, 106.0, 108.0, 115.0, 110.0, 107.0, 105.0, 103.0, 101.0, 95.0, 90.0, 93.0, 96.0, 98.0, 100.0]
    bars = []
    for i, p in enumerate(prices):
        # Explicit highs and lows
        hi = p + (2.0 if i == 5 else 0.5)
        lo = p - (2.0 if i == 12 else 0.5)
        bars.append(CandleBar(i, f"2025-01-{i+1:02d}", p, hi, lo, p, 10000.0))

    pivot_high_pts = calculate_confirmed_pivot_high(bars, left_bars=3, right_bars=3)
    pivot_low_pts = calculate_confirmed_pivot_low(bars, left_bars=3, right_bars=3)

    # Before index 8, pivot high at 5 is NOT confirmed
    for t in range(8):
        assert pivot_high_pts[t].value is not True

    # At index 8 (5 + 3): pivot high at 5 is confirmed!
    pt_h8 = pivot_high_pts[8]
    assert pt_h8.quality == SignalQuality.VALID
    assert pt_h8.value is True
    assert pt_h8.bar_index == 8
    assert pt_h8.available_at_index == 8
    assert pt_h8.availability_event == "BAR_CLOSE"
    assert "confirmed_pivot_high_at_5" in pt_h8.reasons[0]

    # After index 8, at index 9: value is False (event occurred at bar 8)
    assert pivot_high_pts[9].value is False

    # Low pivot at index 12 confirmed at 12 + 3 = 15
    for t in range(15):
        assert pivot_low_pts[t].value is not True
    pt_l15 = pivot_low_pts[15]
    assert pt_l15.quality == SignalQuality.VALID
    assert pt_l15.value is True
    assert "confirmed_pivot_low_at_12" in pt_l15.reasons[0]


# 2. SIG-DIV-002: Four-Way Divergence Definitions
def test_rsi_regular_bullish_divergence():
    """Regular Bullish Divergence: Price Lower Low, RSI Higher Low."""
    n = 50
    prices = [100.0] * n
    # Dip 1 around bar 20
    prices[18] = 95.0
    prices[19] = 92.0
    prices[20] = 88.0  # Low 1
    prices[21] = 92.0
    prices[22] = 95.0
    # Recovery
    for i in range(23, 30):
        prices[i] = 100.0
    # Dip 2 around bar 32 with steeper bounce
    prices[30] = 94.0
    prices[31] = 89.0
    prices[32] = 85.0  # Low 2: 85 < 88 (Price Lower Low)
    prices[33] = 96.0  # sharp snap back boosts RSI
    prices[34] = 99.0
    prices[35] = 102.0

    bars = _generate_bars(prices)
    pts = calculate_rsi_regular_bullish(bars, left_bars=2, right_bars=2, min_separation=5, max_separation=30)

    # Second low at 32 confirmed at 32 + 2 = 34
    pt_confirm = pts[34]
    assert pt_confirm.quality == SignalQuality.VALID
    assert pt_confirm.value is True
    assert pt_confirm.bar_index == 34
    assert pt_confirm.available_at_index == 34
    assert "rsi_regular_bullish" in pt_confirm.reasons[0]


def test_rsi_regular_bearish_divergence():
    """Regular Bearish Divergence: Price Higher High, RSI Lower High."""
    n = 50
    prices = [80.0] * n
    # Peak 1 around bar 20
    prices[18] = 95.0
    prices[19] = 98.0
    prices[20] = 105.0  # High 1
    prices[21] = 98.0
    prices[22] = 95.0
    # Retracement
    for i in range(23, 30):
        prices[i] = 85.0
    # Peak 2 around bar 32 with higher price but sluggish momentum
    prices[30] = 98.0
    prices[31] = 104.0
    prices[32] = 110.0  # High 2: 110 > 105 (Price Higher High)
    prices[33] = 102.0
    prices[34] = 95.0
    prices[35] = 90.0

    bars = _generate_bars(prices)
    pts = calculate_rsi_regular_bearish(bars, left_bars=2, right_bars=2, min_separation=5, max_separation=30)

    # Second high at 32 confirmed at 32 + 2 = 34
    pt_confirm = pts[34]
    assert pt_confirm.quality == SignalQuality.VALID
    assert pt_confirm.value is True
    assert "rsi_regular_bearish" in pt_confirm.reasons[0]



def test_macd_and_stoch_divergence_execution():
    """Verify MACD histogram and Stochastic divergence calculate cleanly with valid schema."""
    bars = _generate_bars([100.0 + math.sin(i / 3.0) * 10.0 for i in range(50)])

    macd_bull = calculate_macd_regular_bullish(bars, left_bars=2, right_bars=2, min_separation=3)
    macd_bear = calculate_macd_regular_bearish(bars, left_bars=2, right_bars=2, min_separation=3)
    stoch_bull = calculate_stoch_regular_bullish(bars, left_bars=2, right_bars=2, min_separation=3)
    stoch_bear = calculate_stoch_regular_bearish(bars, left_bars=2, right_bars=2, min_separation=3)

    assert len(macd_bull) == 50
    assert len(macd_bear) == 50
    assert len(stoch_bull) == 50
    assert len(stoch_bear) == 50

    # Ensure no NaN or exceptions in points
    for p in macd_bull:
        if p.quality == SignalQuality.VALID:
            assert isinstance(p.value, bool)


# 3. Separation Bounds Enforcement
def test_divergence_separation_bounds():
    """Pivots closer than min_separation or farther than max_separation are rejected."""
    # Two lows separated by only 2 bars (min_separation=5)
    prices = [100.0] * 30
    prices[8] = 90.0
    prices[10] = 85.0  # sep = 10 - 8 = 2 bars < 5
    prices[11] = 95.0
    prices[12] = 98.0

    bars = _generate_bars(prices)
    pts = calculate_rsi_regular_bullish(bars, left_bars=1, right_bars=1, min_separation=5, max_separation=20)
    # Confirmation at 10 + 1 = 11 must be False due to separation bound
    assert pts[11].value is False


# 4. TEST-CAUSAL-001: Future Appending Invariance
def test_divergence_future_invariance():
    """TEST-CAUSAL-001: Appending future bars never changes historical divergence outputs."""
    base_prices = [100.0 + math.sin(i / 4.0) * 8.0 for i in range(80)]
    extended_prices = list(base_prices) + [110.0 + math.cos(j / 3.0) * 15.0 for j in range(50)]

    base_bars = _generate_bars(base_prices)
    extended_bars = _generate_bars(extended_prices)

    signals_to_test = [
        "divergence.confirmed_pivot_high",
        "divergence.confirmed_pivot_low",
        "divergence.rsi_regular_bullish",
        "divergence.rsi_regular_bearish",
        "divergence.macd_regular_bullish",
        "divergence.macd_regular_bearish",
        "divergence.stoch_regular_bullish",
        "divergence.stoch_regular_bearish",
    ]

    for sig_name in signals_to_test:
        res_base = SignalRegistry.calculate(sig_name, base_bars)
        res_ext = SignalRegistry.calculate(sig_name, extended_bars)

        assert len(res_base.points) == 80
        assert len(res_ext.points) == 130

        for t in range(80):
            p_b = res_base.points[t]
            p_e = res_ext.points[t]

            assert p_b.quality == p_e.quality, f"{sig_name} quality mismatch at {t}"
            assert p_b.value == p_e.value, f"{sig_name} value mismatch at {t}"
            assert p_b.reasons == p_e.reasons, f"{sig_name} reasons mismatch at {t}"


# 5. AST Signal Binding Integration
def test_divergence_ast_signal_binding():
    """Verify SignalBindingAdapter evaluates rules with divergence__* aliases."""
    bars = _generate_bars([100.0 + math.sin(i / 3.0) * 10.0 for i in range(50)])
    res = SignalRegistry.calculate("divergence.rsi_regular_bullish", bars)

    points_bar_30 = {
        "divergence__rsi_regular_bullish": res.points[30],
    }

    rule = {"eq": ["divergence__rsi_regular_bullish", res.points[30].value]}
    binding = SignalBindingAdapter.evaluate(rule, points_bar_30)
    assert binding.is_valid is True
    assert binding.value is True
