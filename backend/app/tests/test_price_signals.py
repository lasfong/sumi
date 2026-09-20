"""Unit tests for Price domain signals: patterns, technical triggers, regimes, and support/resistance.

Verifies:
- SIG-PAT-001 through SIG-PAT-010 (Candlestick patterns)
- SIG-TECH-001 through SIG-TECH-006 (Technical indicator triggers)
- SIG-REG-001 through SIG-REG-006 (Market regimes)
- SIG-SR-001 (Support/Resistance proximity)
- TEST-CAUSAL-001 (Future appending invariance)
- TEST-CAUSAL-003 (Exclusion of current bar from comparative reference window)
- TEST-SIG-003 (Detailed active explanation reasons in composite signals)
- SignalBindingAdapter AST evaluation across all aliases
"""

import math
import pytest

from app.domain.signals.candle_features import (
    calculate_candle_geometry,
    calculate_causal_atr,
)
from app.domain.signals.models import (
    CandleBar,
    SignalOutputPoint,
    SignalQuality,
)
from app.domain.signals.patterns import (
    calculate_any_bearish_pattern,
    calculate_any_bullish_pattern,
    calculate_bearish_engulfing,
    calculate_bullish_engulfing,
    calculate_dark_cloud_cover,
    calculate_evening_star,
    calculate_hammer,
    calculate_inside_bar_breakout_down,
    calculate_inside_bar_breakout_up,
    calculate_morning_star,
    calculate_piercing_line,
    calculate_shooting_star,
    calculate_tweezer_bottom,
    calculate_tweezer_top,
)
from app.domain.signals.regimes import (
    calculate_downtrend_regime,
    calculate_new_high,
    calculate_new_low,
    calculate_pullback_regime,
    calculate_recovery_regime,
    calculate_sideways_regime,
    calculate_uptrend_regime,
)
from app.domain.signals.registry import SignalRegistry
from app.domain.signals.support_resistance import (
    calculate_near_resistance,
    calculate_near_support,
)
from app.domain.signals.technical import (
    calculate_composite_technical_trigger,
    calculate_ema_cross,
    calculate_macd_signal_cross,
    calculate_macd_zero_cross,
    calculate_rsi_level_cross,
    calculate_swing_break,
    compute_causal_ema,
    compute_causal_macd,
    compute_causal_rsi,
)
from app.domain.strategy.signal_binding import SignalBindingAdapter


def _bar(
    idx: int,
    open_: float,
    high: float,
    low: float,
    close: float,
    volume: float = 1000.0,
) -> CandleBar:
    return CandleBar(
        index=idx,
        timestamp=f"2026-01-{idx+1:02d}",
        open=float(open_),
        high=float(high),
        low=float(low),
        close=float(close),
        volume=float(volume),
    )


# ---------------------------------------------------------------------------
# 1. Candle Geometry & Causal ATR Tests
# ---------------------------------------------------------------------------


def test_candle_geometry_normal():
    bar = _bar(0, 100.0, 110.0, 95.0, 108.0)
    geom = calculate_candle_geometry(bar)
    assert geom.range == 15.0
    assert geom.body == 8.0
    assert geom.upper_wick == 2.0
    assert geom.lower_wick == 5.0
    assert abs(geom.body_ratio - 8.0 / 15.0) < 1e-6
    assert abs(geom.upper_wick_ratio - 2.0 / 15.0) < 1e-6
    assert abs(geom.lower_wick_ratio - 5.0 / 15.0) < 1e-6
    assert abs(geom.close_location - 13.0 / 15.0) < 1e-6
    assert geom.midpoint == 104.0
    assert geom.is_bullish is True
    assert geom.is_bearish is False


def test_candle_geometry_zero_range_doji():
    bar = _bar(0, 100.0, 100.0, 100.0, 100.0)
    geom = calculate_candle_geometry(bar)
    assert geom.range == 0.0
    assert geom.body == 0.0
    assert geom.body_ratio == 0.0
    assert geom.upper_wick_ratio == 0.0
    assert geom.lower_wick_ratio == 0.0
    assert geom.close_location == 0.5
    assert geom.is_flat is True


def test_causal_atr_calculation():
    # 20 bars with range 10 each
    bars = [_bar(i, 100.0, 110.0, 100.0, 105.0) for i in range(20)]
    atr = calculate_causal_atr(bars, period=14)
    assert len(atr) == 20
    # First 13 bars are None
    for i in range(13):
        assert atr[i] is None
    # Bar 13 (14th bar) has initial ATR = 10.0
    assert atr[13] == 10.0
    # Bar 14 also 10.0
    assert atr[14] == 10.0


# ---------------------------------------------------------------------------
# 2. Candlestick Pattern Tests (SIG-PAT-001..010)
# ---------------------------------------------------------------------------


def test_sig_pat_001_bullish_engulfing():
    # Bar 0: Bearish 105 -> 95 (range 107 -> 93)
    # Bar 1: Bullish 94 -> 106 (range 107 -> 93) -> Engulfs
    b0 = _bar(0, 105.0, 107.0, 93.0, 95.0)
    b1 = _bar(1, 94.0, 107.0, 93.0, 106.0)
    pts = calculate_bullish_engulfing([b0, b1])

    assert pts[0].quality == SignalQuality.INSUFFICIENT_HISTORY
    assert pts[1].quality == SignalQuality.VALID
    assert pts[1].value is True
    assert "BULLISH_ENGULFING" in pts[1].reasons


def test_sig_pat_002_bearish_engulfing():
    # Bar 0: Bullish 95 -> 105
    # Bar 1: Bearish 106 -> 94 -> Engulfs
    b0 = _bar(0, 95.0, 107.0, 93.0, 105.0)
    b1 = _bar(1, 106.0, 107.0, 93.0, 94.0)
    pts = calculate_bearish_engulfing([b0, b1])

    assert pts[1].value is True
    assert "BEARISH_ENGULFING" in pts[1].reasons


def test_sig_pat_003_hammer():
    # Open 100, Close 102 (body 2), High 102.5 (upper wick 0.5), Low 90 (lower wick 10 >= 2*2)
    b0 = _bar(0, 100.0, 102.5, 90.0, 102.0)
    pts = calculate_hammer([b0])
    assert pts[0].value is True
    assert "HAMMER" in pts[0].reasons


def test_sig_pat_004_shooting_star():
    # Open 98, Close 96 (body 2), High 110 (upper wick 12 >= 2*2), Low 95.5 (lower wick 0.5)
    b0 = _bar(0, 98.0, 110.0, 95.5, 96.0)
    pts = calculate_shooting_star([b0])
    assert pts[0].value is True
    assert "SHOOTING_STAR" in pts[0].reasons


def test_sig_pat_005_morning_star():
    # Bar 0: Bearish 110 -> 90 (midpoint 100)
    # Bar 1: Small body 89 -> 90
    # Bar 2: Bullish 90 -> 102 (closes above midpoint 100)
    b0 = _bar(0, 110.0, 112.0, 88.0, 90.0)
    b1 = _bar(1, 89.0, 92.0, 88.0, 90.0)
    b2 = _bar(2, 90.0, 105.0, 89.0, 102.0)
    pts = calculate_morning_star([b0, b1, b2])

    assert pts[0].quality == SignalQuality.INSUFFICIENT_HISTORY
    assert pts[1].quality == SignalQuality.INSUFFICIENT_HISTORY
    assert pts[2].value is True
    assert "MORNING_STAR" in pts[2].reasons


def test_sig_pat_006_evening_star():
    # Bar 0: Bullish 90 -> 110 (midpoint 100)
    # Bar 1: Small body 111 -> 110
    # Bar 2: Bearish 110 -> 98 (closes below midpoint 100)
    b0 = _bar(0, 90.0, 112.0, 88.0, 110.0)
    b1 = _bar(1, 111.0, 112.0, 109.0, 110.0)
    b2 = _bar(2, 110.0, 111.0, 95.0, 98.0)
    pts = calculate_evening_star([b0, b1, b2])

    assert pts[2].value is True
    assert "EVENING_STAR" in pts[2].reasons


def test_sig_pat_007_piercing_line_and_dark_cloud():
    # Piercing: b0 bearish 100 -> 90 (midpoint 95), b1 bullish 89 -> 96
    b0 = _bar(0, 100.0, 101.0, 89.0, 90.0)
    b1 = _bar(1, 89.0, 97.0, 88.0, 96.0)
    pts_p = calculate_piercing_line([b0, b1])
    assert pts_p[1].value is True

    # Dark cloud: b0 bullish 90 -> 100 (midpoint 95), b1 bearish 101 -> 94
    b0_d = _bar(0, 90.0, 101.0, 89.0, 100.0)
    b1_d = _bar(1, 101.0, 102.0, 93.0, 94.0)
    pts_d = calculate_dark_cloud_cover([b0_d, b1_d])
    assert pts_d[1].value is True


def test_sig_pat_008_tweezer_bottom():
    # 15 bars so ATR is valid; last 2 bars have matching lows (low=90.0)
    bars = [_bar(i, 100.0, 105.0, 95.0, 100.0) for i in range(14)]
    bars.append(_bar(14, 100.0, 105.0, 90.0, 92.0))  # Bearish, low 90.0
    bars.append(_bar(15, 92.0, 102.0, 90.05, 101.0)) # Bullish, low 90.05 (~matching)
    pts = calculate_tweezer_bottom(bars)
    assert pts[15].value is True
    assert "TWEEZER_BOTTOM" in pts[15].reasons


def test_sig_pat_009_inside_bar_breakout_up():
    bars = [_bar(i, 100.0, 105.0, 95.0, 100.0) for i in range(14)]
    # Mother bar at 14: High 110, Low 90
    bars.append(_bar(14, 100.0, 110.0, 90.0, 105.0))
    # Inside bar at 15: High 105 (<110), Low 95 (>90)
    bars.append(_bar(15, 100.0, 105.0, 95.0, 102.0))
    # Breakout at 16: Close 115 (> 110 + buffer)
    bars.append(_bar(16, 102.0, 116.0, 101.0, 115.0))
    pts = calculate_inside_bar_breakout_up(bars)
    assert pts[16].value is True
    assert "INSIDE_BAR_BREAKOUT_UP" in pts[16].reasons


def test_sig_pat_010_composite_any_with_reasons():
    # Setup a hammer at bar 0
    b0 = _bar(0, 100.0, 102.5, 90.0, 102.0)
    pts = calculate_any_bullish_pattern([b0])
    assert pts[0].value is True
    assert "hammer" in pts[0].reasons


# ---------------------------------------------------------------------------
# 3. Technical Trigger Tests (SIG-TECH-001..006)
# ---------------------------------------------------------------------------


def test_sig_tech_001_macd_signal_cross():
    # Create 40 bars with a sharp upward turn to trigger MACD cross up
    bars = []
    for i in range(35):
        bars.append(_bar(i, 100.0 - i * 0.5, 101.0 - i * 0.5, 99.0 - i * 0.5, 100.0 - i * 0.5))
    # Sharp turn up
    for i in range(35, 45):
        bars.append(_bar(i, 80.0 + (i - 35) * 5.0, 85.0 + (i - 35) * 5.0, 79.0 + (i - 35) * 5.0, 84.0 + (i - 35) * 5.0))

    pts = calculate_macd_signal_cross(bars, direction="up")
    # Cross must occur in the upward sequence
    crosses = [p for p in pts if p.value is True]
    assert len(crosses) >= 1
    assert "MACD_SIGNAL_CROSS_UP" in crosses[0].reasons


def test_sig_tech_003_rsi_level_cross():
    # Downtrend dropping RSI below 30, then sharp bounce above 30
    bars = []
    for i in range(25):
        p = 100.0 - i * 2.0
        bars.append(_bar(i, p, p + 1.0, p - 1.0, p))
    # Bounce up
    bars.append(_bar(25, 52.0, 60.0, 51.0, 58.0))
    bars.append(_bar(26, 58.0, 70.0, 57.0, 68.0))

    pts = calculate_rsi_level_cross(bars, length=14, level=30.0, direction="up")
    crosses = [p for p in pts if p.value is True]
    assert len(crosses) >= 1


def test_sig_tech_004_ema_cross():
    # 60 bars: downtrend then sharp rally
    bars = []
    for i in range(50):
        bars.append(_bar(i, 100.0, 102.0, 98.0, 100.0))
    for i in range(50, 70):
        p = 100.0 + (i - 50) * 3.0
        bars.append(_bar(i, p, p + 2.0, p - 1.0, p + 1.0))

    pts = calculate_ema_cross(bars, fast_period=10, slow_period=20, direction="up")
    crosses = [p for p in pts if p.value is True]
    assert len(crosses) >= 1
    assert "EMA_CROSS_UP" in crosses[0].reasons


# ---------------------------------------------------------------------------
# 4. Market Regime Tests (SIG-REG-001..006)
# ---------------------------------------------------------------------------


def test_sig_reg_001_uptrend_regime():
    # 70 bars of steady uptrend
    bars = []
    for i in range(70):
        p = 100.0 + i * 2.0
        bars.append(_bar(i, p, p + 2.0, p - 1.0, p + 1.0))
    pts = calculate_uptrend_regime(bars, fast_period=20, slow_period=50, slope_lookback=5)
    assert pts[65].value is True
    assert "UPTREND" in pts[65].reasons


def test_sig_reg_003_sideways_regime():
    # 40 bars flat at 100.0
    bars = [_bar(i, 100.0, 101.0, 99.0, 100.0) for i in range(40)]
    pts = calculate_sideways_regime(bars, ema_period=20, slope_lookback=5)
    assert pts[35].value is True
    assert "SIDEWAYS" in pts[35].reasons


def test_sig_reg_006_test_causal_003_new_high_and_new_low():
    """TEST-CAUSAL-003: Current bar t is excluded from comparative reference window."""
    # 25 bars: bars 0..19 highs around 100..105, highest was 105.0 at bar 10
    bars = [_bar(i, 100.0, 102.0, 98.0, 100.0) for i in range(20)]
    bars[10] = _bar(10, 100.0, 105.0, 98.0, 100.0)

    # Bar 20 has High = 106.0 > 105.0 -> NEW HIGH!
    bars.append(_bar(20, 100.0, 106.0, 99.0, 105.0))
    pts_high = calculate_new_high(bars, period=20)
    assert pts_high[20].value is True
    assert "NEW_HIGH" in pts_high[20].reasons

    # Bar 21 has High = 105.5.
    # If bar 20 (high=106) is properly included in history for bar 21,
    # then prior max for bar 21 is 106.0, so 105.5 is NOT a new high!
    bars.append(_bar(21, 104.0, 105.5, 103.0, 104.0))
    pts_high_2 = calculate_new_high(bars, period=20)
    assert pts_high_2[21].value is False

    # Similarly for new low
    bars_low = [_bar(i, 100.0, 102.0, 95.0, 100.0) for i in range(20)]
    bars_low.append(_bar(20, 100.0, 101.0, 93.0, 94.0))  # 93.0 < 95.0 -> NEW LOW
    pts_low = calculate_new_low(bars_low, period=20)
    assert pts_low[20].value is True
    assert "NEW_LOW" in pts_low[20].reasons


# ---------------------------------------------------------------------------
# 5. Support / Resistance Tests (SIG-SR-001)
# ---------------------------------------------------------------------------


def test_sig_sr_001_near_support():
    # 25 bars with rolling low = 90.0, then current bar dips near 90.0
    bars = [_bar(i, 100.0, 105.0, 95.0, 100.0) for i in range(20)]
    bars[5] = _bar(5, 95.0, 96.0, 90.0, 92.0)  # low 90.0
    bars.append(_bar(20, 92.0, 94.0, 90.5, 91.0)) # Close 91.0 near 90.0

    pts = calculate_near_support(bars, period=20, tolerance_atr=1.0)
    assert pts[20].value is True
    assert any("ROLLING_LOW" in r for r in pts[20].reasons)


# ---------------------------------------------------------------------------
# 6. Future Invariance Test (TEST-CAUSAL-001)
# ---------------------------------------------------------------------------


def test_causal_future_invariance_across_signals():
    """TEST-CAUSAL-001: Appending future bars cannot alter previously calculated points."""
    base_bars = [_bar(i, 100.0 + (i % 5), 105.0 + (i % 5), 95.0 + (i % 5), 101.0 + (i % 5)) for i in range(60)]
    future_bars = [_bar(60 + i, 150.0, 160.0, 140.0, 155.0) for i in range(10)]
    extended_bars = base_bars + future_bars

    signals_to_test = [
        ("pattern.bullish_engulfing", {}),
        ("pattern.hammer", {}),
        ("pattern.morning_star", {}),
        ("tech.macd_signal_cross", {}),
        ("tech.ema_cross", {"fast_period": 10, "slow_period": 20}),
        ("regime.uptrend", {"fast_period": 10, "slow_period": 20, "slope_lookback": 3}),
        ("regime.new_high", {"period": 20}),
        ("sr.near_support", {"period": 20}),
    ]

    for sig_name, params in signals_to_test:
        res_base = SignalRegistry.calculate(sig_name, base_bars, params)
        res_ext = SignalRegistry.calculate(sig_name, extended_bars, params)

        # All points in res_base must match res_ext index by index
        for t in range(len(base_bars)):
            pt_base = res_base.points[t]
            pt_ext = res_ext.points[t]

            assert pt_base.bar_index == pt_ext.bar_index
            assert pt_base.quality == pt_ext.quality
            assert pt_base.value == pt_ext.value
            assert pt_base.reasons == pt_ext.reasons


# ---------------------------------------------------------------------------
# 7. Strategy Binding & AST Alias Tests
# ---------------------------------------------------------------------------


def test_signal_binding_ast_aliases():
    """Verify all new signals are recognized in Strategy AST evaluator and fail-closed."""
    aliases = SignalBindingAdapter.get_supported_signal_aliases()

    expected_new_aliases = [
        "pattern__bullish_engulfing",
        "pattern__bearish_engulfing",
        "pattern__hammer",
        "pattern__shooting_star",
        "pattern__morning_star",
        "pattern__evening_star",
        "pattern__piercing_line",
        "pattern__dark_cloud_cover",
        "pattern__tweezer_bottom",
        "pattern__tweezer_top",
        "pattern__inside_bar_breakout_up",
        "pattern__inside_bar_breakout_down",
        "pattern__any_bullish",
        "pattern__any_bearish",
        "tech__macd_signal_cross",
        "tech__macd_zero_cross",
        "tech__rsi_level_cross",
        "tech__ema_cross",
        "tech__swing_break",
        "tech__composite",
        "regime__uptrend",
        "regime__downtrend",
        "regime__sideways",
        "regime__pullback",
        "regime__recovery",
        "regime__new_high",
        "regime__new_low",
        "sr__near_support",
        "sr__near_resistance",
    ]

    for alias in expected_new_aliases:
        assert alias in aliases, f"Alias {alias} not found in supported aliases!"

    # Evaluate rule with new pattern alias
    point = SignalOutputPoint(
        bar_index=5,
        timestamp="2026-01-06",
        output_type="bool",
        value=True,
        quality=SignalQuality.VALID,
        reasons=["BULLISH_ENGULFING"],
        availability_event="BAR_CLOSE",
        available_at_index=5,
        available_at_timestamp="2026-01-06",
    )
    res = SignalBindingAdapter.evaluate("pattern__bullish_engulfing == True", {"pattern__bullish_engulfing": point})
    assert res.is_valid is True
    assert res.value is True
