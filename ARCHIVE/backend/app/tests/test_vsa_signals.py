"""Unit tests for Batch P3-VSA-02: Volume Climax, VSA Primitives, Structure, and Composites.

Verifies:
- SIG-VOL-003: Volume Climax Up & Volume Climax Down
- SIG-STR-001: Candle Structure Score and boundary handling
- SIG-VSA-001: Strong Demand
- SIG-VSA-002: Weak Demand / No Demand
- SIG-VSA-003: Upthrust / Simplified Kéo Xả
- SIG-VSA-004: Spring / Shakeout / Đạp Kéo
- SIG-VSA-005: Strong Demand at Support
- SIG-VSA-006: Strong Supply at Resistance
- TEST-CAUSAL-001: Future appending invariance
- TEST-CAUSAL-003: Prior reference exclusion (bar t excluded from lookback extrema)
- SignalRegistry and SignalBindingAdapter AST evaluation across all new aliases
"""

import math
import pytest

from app.domain.signals.candle_features import calculate_causal_atr
from app.domain.signals.models import CandleBar, SignalQuality
from app.domain.signals.registry import SignalRegistry
from app.domain.signals.structure import calculate_candle_structure_score
from app.domain.signals.volume import (
    calculate_volume_climax_down,
    calculate_volume_climax_up,
)
from app.domain.signals.vsa import (
    calculate_spring,
    calculate_strong_demand,
    calculate_strong_demand_at_support,
    calculate_strong_supply_at_resistance,
    calculate_upthrust,
    calculate_weak_demand,
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
        timestamp=f"2026-01-{idx+1:02d}" if idx < 31 else f"2026-02-{idx-30:02d}",
        open=float(open_),
        high=float(high),
        low=float(low),
        close=float(close),
        volume=float(volume),
    )


# ---------------------------------------------------------------------------
# 1. SIG-VOL-003: Volume Climax Up and Down
# ---------------------------------------------------------------------------


def test_volume_climax_up_and_down():
    # 25 baseline bars with volume=1000, daily range ~ 2.0 (ATR ~ 2.0)
    candles = [_bar(i, 100.0, 101.0, 99.0, 100.0, 1000.0) for i in range(25)]

    # Bar 25: Climax Up (Volume=3000 -> RVOL=3.0, Range=10.0 -> Range/ATR ~ 5.0, Close near high 109.0/110.0 -> Close loc=0.90)
    candles.append(_bar(25, 100.0, 110.0, 100.0, 109.0, 3000.0))

    # Bar 26: Climax Down (Volume=3500 -> RVOL=3.5, Range=10.0, Close near low 101.0 -> Close loc=0.10)
    candles.append(_bar(26, 110.0, 110.0, 100.0, 101.0, 3500.0))

    up_pts = calculate_volume_climax_up(candles, period=20, multiplier=2.0, min_range_atr=1.5, min_close_location=0.75)
    down_pts = calculate_volume_climax_down(candles, period=20, multiplier=2.0, min_range_atr=1.5, max_close_location=0.25)

    # Warmup bars
    assert up_pts[10].quality == SignalQuality.INSUFFICIENT_HISTORY
    assert up_pts[10].value is None

    # Normal bar before climax
    assert up_pts[24].quality == SignalQuality.VALID
    assert up_pts[24].value is False
    assert down_pts[24].value is False

    # Bar 25 is Climax Up
    assert up_pts[25].value is True
    assert "VOLUME_CLIMAX_UP" in up_pts[25].reasons
    assert down_pts[25].value is False

    # Bar 26 is Climax Down
    assert up_pts[26].value is False
    assert down_pts[26].value is True
    assert "VOLUME_CLIMAX_DOWN" in down_pts[26].reasons


# ---------------------------------------------------------------------------
# 2. SIG-STR-001: Candle Structure Score
# ---------------------------------------------------------------------------


def test_candle_structure_score_and_bounds():
    # 1. Bullish marubozu: open=100, high=110, low=100, close=110
    bullish_bar = _bar(0, 100.0, 110.0, 100.0, 110.0)
    pts_bull = calculate_candle_structure_score([bullish_bar])
    assert pts_bull[0].quality == SignalQuality.VALID
    score_bull = pts_bull[0].value
    assert isinstance(score_bull, float)
    assert score_bull >= 70.0
    assert "GOOD_CANDLE_STRUCTURE" in pts_bull[0].reasons

    # 2. Bearish marubozu: open=110, high=110, low=100, close=100
    bearish_bar = _bar(1, 110.0, 110.0, 100.0, 100.0)
    pts_bear = calculate_candle_structure_score([bearish_bar])
    score_bear = pts_bear[0].value
    assert isinstance(score_bear, float)
    assert score_bear <= -70.0
    assert "POOR_CANDLE_STRUCTURE" in pts_bear[0].reasons

    # 3. Flat candle (zero-range): open=100, high=100, low=100, close=100
    flat_bar = _bar(2, 100.0, 100.0, 100.0, 100.0)
    pts_flat = calculate_candle_structure_score([flat_bar])
    assert pts_flat[0].quality == SignalQuality.VALID
    assert pts_flat[0].value == 0.0
    assert "ZERO_RANGE_CANDLE" in pts_flat[0].reasons


# ---------------------------------------------------------------------------
# 3. SIG-VSA-001: Strong Demand
# ---------------------------------------------------------------------------


def test_vsa_strong_demand():
    candles = [_bar(i, 100.0, 102.0, 98.0, 100.0, 1000.0) for i in range(25)]

    # Strong demand at index 25: close > close[-1], wide spread (range 10.0, ATR ~ 4.0), high RVOL (2.5), body ratio 0.8, close loc 0.9
    candles.append(_bar(25, 101.0, 111.0, 101.0, 110.0, 2500.0))

    pts = calculate_strong_demand(candles, period=20, min_rvol=1.5, min_range_atr=1.2, min_body_ratio=0.50, min_close_location=0.70)
    assert pts[25].value is True
    assert "VSA_STRONG_DEMAND" in pts[25].reasons

    # Sub-threshold demand: low volume (RVOL=1.0)
    candles.append(_bar(26, 110.0, 120.0, 110.0, 119.0, 1000.0))
    pts2 = calculate_strong_demand(candles, period=20, min_rvol=1.5)
    assert pts2[26].value is False
    assert "NOT_STRONG_DEMAND" in pts2[26].reasons


# ---------------------------------------------------------------------------
# 4. SIG-VSA-002: Weak Demand / No Demand
# ---------------------------------------------------------------------------


def test_vsa_weak_demand():
    candles = [_bar(i, 100.0, 105.0, 95.0, 100.0, 1000.0) for i in range(25)]

    # Weak demand: close >= close[-1], narrow range (1.0 vs ATR ~ 10.0), low volume (500 vs baseline 1000 -> RVOL 0.5)
    candles.append(_bar(25, 100.0, 100.8, 99.8, 100.5, 500.0))

    pts = calculate_weak_demand(candles, period=20, max_rvol=0.80, max_range_atr=0.70)
    assert pts[25].value is True
    assert "VSA_WEAK_DEMAND" in pts[25].reasons

    # High volume expansion: should NOT be weak demand
    candles.append(_bar(26, 100.5, 115.0, 100.5, 114.0, 2500.0))
    pts2 = calculate_weak_demand(candles, period=20)
    assert pts2[26].value is False


# ---------------------------------------------------------------------------
# 5. SIG-VSA-003: Upthrust / Kéo Xả
# ---------------------------------------------------------------------------


def test_vsa_upthrust_keo_xa():
    # 25 bars with resistance at 105.0
    candles = [_bar(i, 100.0, 105.0, 98.0, 100.0, 1000.0) for i in range(25)]

    # Bar 25: Breaches 105.0 up to 110.0 in session, but drops back to close at 102.0
    # Upper wick: 110 - max(100, 102) = 8.0 / 12.0 = 0.67 (>= 0.35)
    # Close location: (102 - 98) / 12.0 = 0.33 (<= 0.45)
    # Volume: 2500 (RVOL = 2.5 >= 1.5)
    candles.append(_bar(25, 100.0, 110.0, 98.0, 102.0, 2500.0))

    pts = calculate_upthrust(candles, period=20, min_rvol=1.5, min_upper_wick_ratio=0.35, max_close_location=0.45)
    assert pts[25].value is True
    assert "VSA_UPTHRUST" in pts[25].reasons
    assert any("PRIOR_RESISTANCE_105.00" in r for r in pts[25].reasons)


# ---------------------------------------------------------------------------
# 6. SIG-VSA-004: Spring / Shakeout / Đạp Kéo
# ---------------------------------------------------------------------------


def test_vsa_spring_dap_keo():
    # 25 bars with support at 95.0
    candles = [_bar(i, 100.0, 105.0, 95.0, 100.0, 1000.0) for i in range(25)]

    # Bar 25: Drops below 95.0 to 90.0 intra-session, but rallies back to close at 103.0
    # Lower wick: min(100, 103) - 90 = 10.0 / 15.0 = 0.67 (>= 0.35)
    # Close location: (103 - 90) / 15.0 = 0.87 (>= 0.70)
    # Volume: 2000 (RVOL = 2.0 >= 1.30)
    candles.append(_bar(25, 100.0, 105.0, 90.0, 103.0, 2000.0))

    pts = calculate_spring(candles, period=20, min_rvol=1.30, min_lower_wick_ratio=0.35, min_close_location=0.70)
    assert pts[25].value is True
    assert "VSA_SPRING" in pts[25].reasons
    assert any("PRIOR_SUPPORT_95.00" in r for r in pts[25].reasons)


# ---------------------------------------------------------------------------
# 7. SIG-VSA-005 & SIG-VSA-006: Composites at Support & Resistance
# ---------------------------------------------------------------------------


def test_vsa_strong_demand_at_support():
    # 25 baseline bars establishing support at 95.0
    candles = [_bar(i, 100.0, 105.0, 95.0, 100.0, 1000.0) for i in range(25)]

    # Bar 25: Spring at support (close 96.0, low 90.0, high 100.0, open 94.0, vol 2000)
    # Near support: abs(96 - 95) = 1.0 <= tolerance_atr * ATR
    candles.append(_bar(25, 94.0, 100.0, 90.0, 98.0, 2000.0))

    pts = calculate_strong_demand_at_support(candles, period=20, tolerance_atr=1.5, min_rvol=1.30)
    assert pts[25].value is True
    assert "STRONG_DEMAND_AT_SUPPORT" in pts[25].reasons
    assert any("ROLLING_LOW_20" in r for r in pts[25].reasons)


def test_vsa_strong_supply_at_resistance():
    # 25 baseline bars establishing resistance at 105.0
    candles = [_bar(i, 100.0, 105.0, 95.0, 100.0, 1000.0) for i in range(25)]

    # Bar 25: Upthrust at resistance (high=110 breaches 105, close=102 with weak close loc 0.33 and high upper wick)
    candles.append(_bar(25, 100.0, 110.0, 98.0, 102.0, 2500.0))

    pts = calculate_strong_supply_at_resistance(candles, period=20, tolerance_atr=1.5, min_rvol=1.30)
    assert pts[25].value is True
    assert "STRONG_SUPPLY_AT_RESISTANCE" in pts[25].reasons
    assert any("ROLLING_HIGH_20" in r for r in pts[25].reasons)


# ---------------------------------------------------------------------------
# 8. TEST-CAUSAL-001: Future Appending Invariance
# ---------------------------------------------------------------------------


def test_vsa_future_invariance():
    base_candles = [_bar(i, 100.0 + i*0.2, 105.0 + i*0.2, 98.0 + i*0.2, 103.0 + i*0.2, 1000.0 + i*50) for i in range(30)]
    future_candles = base_candles + [
        _bar(30, 110.0, 120.0, 108.0, 118.0, 3000.0),
        _bar(31, 118.0, 125.0, 115.0, 116.0, 4000.0),
    ]

    # Test across all VSA and structure signals
    signals_to_test = [
        "volume.climax_up",
        "volume.climax_down",
        "structure.candle_score",
        "vsa.strong_demand",
        "vsa.weak_demand",
        "vsa.upthrust",
        "vsa.spring",
        "vsa.strong_demand_at_support",
        "vsa.strong_supply_at_resistance",
    ]

    for sig_name in signals_to_test:
        res_base = SignalRegistry.calculate(sig_name, base_candles)
        res_future = SignalRegistry.calculate(sig_name, future_candles)

        assert len(res_base.points) == len(base_candles)
        assert len(res_future.points) == len(future_candles)

        for t in range(len(base_candles)):
            p_base = res_base.points[t]
            p_fut = res_future.points[t]
            assert p_base.value == p_fut.value, f"Signal {sig_name} future leakage at index {t}"
            assert p_base.quality == p_fut.quality, f"Signal {sig_name} quality drift at index {t}"


# ---------------------------------------------------------------------------
# 9. TEST-CAUSAL-003: Prior Reference Exclusion
# ---------------------------------------------------------------------------


def test_vsa_reference_window_causal_exclusion():
    # 20 bars with maximum high 100.0
    candles = [_bar(i, 90.0, 100.0, 85.0, 95.0, 1000.0) for i in range(20)]

    # Bar 20: Reaches 120.0. If bar 20 was mistakenly included in prior resistance,
    # prior resistance would be 120.0 and it could never breach 120.0.
    # Because bar 20 is strictly excluded, prior resistance is 100.0, so 120.0 breaches it!
    candles.append(_bar(20, 95.0, 120.0, 94.0, 98.0, 3000.0))

    pts = calculate_upthrust(candles, period=20, min_rvol=1.5)
    assert pts[20].value is True
    assert any("PRIOR_RESISTANCE_100.00" in r for r in pts[20].reasons)


# ---------------------------------------------------------------------------
# 10. SignalRegistry and AST Binding Adapter Integration
# ---------------------------------------------------------------------------


def test_vsa_registry_and_binding():
    candles = [_bar(i, 100.0, 102.0, 98.0, 100.0, 1000.0) for i in range(25)]
    candles.append(_bar(25, 101.0, 111.0, 101.0, 110.0, 2500.0))

    # Calculate through registry
    res_demand = SignalRegistry.calculate("vsa.strong_demand", candles)
    assert res_demand.points[25].value is True

    # Evaluate via SignalBindingAdapter DSL
    adapter = SignalBindingAdapter()
    snapshots = {
        "vsa__strong_demand": res_demand.points[25],
    }

    eval_res = SignalBindingAdapter.evaluate(
        rule="vsa__strong_demand == True",
        signal_points=snapshots,
    )
    assert eval_res.is_valid is True
    assert eval_res.value is True



    # Test AST alias resolution
    assert SignalRegistry.resolve_name_from_alias("vsa__strong_demand") == "vsa.strong_demand"
    assert SignalRegistry.resolve_name_from_alias("vsa__upthrust") == "vsa.upthrust"
    assert SignalRegistry.resolve_name_from_alias("vsa__spring") == "vsa.spring"
    assert SignalRegistry.resolve_name_from_alias("structure__candle_score") == "structure.candle_score"
    assert SignalRegistry.resolve_name_from_alias("volume__climax_up") == "volume.climax_up"
