"""Unit and integration tests for Causal Ichimoku Signals.

Covers:
- SIG-ICHI-001: Four core causal factors, Kijun slope, score normalization [-100, 100], bullish/bearish states, reasons
- TEST-ICHI-001: Visible cloud displacement D=26 fixture and zero lookahead proof
- TEST-CAUSAL-001: Future appending invariance (bit-for-bit identical on historical prefix)
- Transition signals: TK crossover up/down, Kumo breakout up/down
- SignalRegistry integration and AST rule binding evaluation
- Replay indicator API projection tagging (is_projected)
"""

from datetime import date, timedelta
import math
import pytest
import pandas as pd
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from sqlalchemy.pool import StaticPool
from fastapi.testclient import TestClient

from app.domain.signals.models import (
    CandleBar,
    SignalQuality,
    SignalOutputType,
)
from app.domain.signals.ichimoku import (
    calculate_ichimoku_score,
    calculate_ichimoku_bullish,
    calculate_ichimoku_bearish,
    calculate_ichimoku_tk_cross_bullish,
    calculate_ichimoku_tk_cross_bearish,
    calculate_ichimoku_kumo_breakout_bullish,
    calculate_ichimoku_kumo_breakout_bearish,
    _compute_ichimoku_series,
)
from app.domain.signals.registry import SignalRegistry
from app.domain.strategy.signal_binding import SignalBindingAdapter
from app.main import app
from app.dependencies import get_db
from app.db import Base
from app.models.candle import Candle
from app.models.replay_session import ReplaySession


def _generate_deterministic_bars(count: int, trend: float = 0.5) -> list[CandleBar]:
    """Generate synthetic deterministic bars for testing."""
    bars = []
    base_price = 100.0
    for i in range(count):
        close = base_price + i * trend + math.sin(i / 5.0) * 5.0
        high = close + 2.0
        low = close - 2.0
        open_p = close - 0.5
        bars.append(
            CandleBar(
                index=i,
                timestamp=f"2025-01-{(i % 28) + 1:02d}",
                open=round(open_p, 2),
                high=round(high, 2),
                low=round(low, 2),
                close=round(close, 2),
                volume=10000.0 + i * 100.0,
            )
        )
    return bars


# 1. Oracle Verification of Tenkan, Kijun, and Spans
def test_ichimoku_series_oracle():
    """Verify rolling midpoints for Tenkan (9), Kijun (26), Raw Span A, Raw Span B (52)."""
    bars = _generate_deterministic_bars(90)
    tenkan, kijun, raw_a, raw_b, vis_top, vis_bottom = _compute_ichimoku_series(bars)

    # Tenkan requires 9 bars (index 0..8)
    for i in range(8):
        assert tenkan[i] is None
    w9 = bars[0:9]
    expected_t8 = (max(b.high for b in w9) + min(b.low for b in w9)) / 2.0
    assert tenkan[8] == pytest.approx(expected_t8)

    # Kijun requires 26 bars (index 0..25)
    for i in range(25):
        assert kijun[i] is None
    w26 = bars[0:26]
    expected_k25 = (max(b.high for b in w26) + min(b.low for b in w26)) / 2.0
    assert kijun[25] == pytest.approx(expected_k25)

    # Raw Span A at index 25 is average of Tenkan and Kijun
    assert raw_a[25] == pytest.approx((tenkan[25] + kijun[25]) / 2.0)

    # Raw Span B requires 52 bars (index 0..51)
    for i in range(51):
        assert raw_b[i] is None
    w52 = bars[0:52]
    expected_b51 = (max(b.high for b in w52) + min(b.low for b in w52)) / 2.0
    assert raw_b[51] == pytest.approx(expected_b51)


# 2. TEST-ICHI-001: Visible Cloud Displacement Fixture (D=26)
def test_ichimoku_visible_cloud_displacement_fixture():
    """TEST-ICHI-001: Visible cloud at bar t strictly matches raw spans at t - 26."""
    bars = _generate_deterministic_bars(120)
    _, _, raw_a, raw_b, vis_top, vis_bottom = _compute_ichimoku_series(bars, displacement=26)

    # Warmup requirement: raw_b is first valid at 51, shifted by 26 => visible at 51 + 26 = 77
    for t in range(77):
        assert vis_top[t] is None
        assert vis_bottom[t] is None

    # At t=77: visible cloud must equal max/min of raw spans at t - 26 = 51
    assert raw_a[51] is not None
    assert raw_b[51] is not None
    assert vis_top[77] == pytest.approx(max(raw_a[51], raw_b[51]))
    assert vis_bottom[77] == pytest.approx(min(raw_a[51], raw_b[51]))

    # At t=100: visible cloud must equal max/min of raw spans at t - 26 = 74
    assert vis_top[100] == pytest.approx(max(raw_a[74], raw_b[74]))
    assert vis_bottom[100] == pytest.approx(min(raw_a[74], raw_b[74]))


# 3. SIG-ICHI-001: Score Evaluation, Four Core Factors, and Active Reasons
def test_ichimoku_score_four_factors_and_reasons():
    """SIG-ICHI-001: Score normalized in [-100, 100], 4 factors evaluated causally."""
    # Create strongly bullish setup: strong uptrend
    bullish_bars = _generate_deterministic_bars(100, trend=2.0)
    score_pts = calculate_ichimoku_score(bullish_bars)

    # Before warmup (t < 77)
    for t in range(77):
        assert score_pts[t].quality == SignalQuality.INSUFFICIENT_HISTORY
        assert score_pts[t].value is None

    # At t=78 (after warmup) in a strong linear uptrend:
    # 1. Price is above visible cloud (cloud formed 26 bars ago when price was lower) -> price_above_cloud
    # 2. Tenkan > Kijun -> tk_bullish
    # 3. Close[t] > Close[t - 26] -> chikou_above_price
    # 4. Raw Span A > Raw Span B -> future_cloud_bullish
    pt_bull = score_pts[80]
    assert pt_bull.quality == SignalQuality.VALID
    assert pt_bull.value == 100.0  # all 4 factors bullish: 100 * (4 - 0) / 4
    assert "price_above_cloud" in pt_bull.reasons
    assert "tk_bullish" in pt_bull.reasons
    assert "chikou_above_price" in pt_bull.reasons
    assert "future_cloud_bullish" in pt_bull.reasons

    # Create strongly bearish setup: strong downtrend
    bearish_bars = _generate_deterministic_bars(100, trend=-2.0)
    bear_pts = calculate_ichimoku_score(bearish_bars)
    pt_bear = bear_pts[80]
    assert pt_bear.quality == SignalQuality.VALID
    assert pt_bear.value == -100.0  # all 4 factors bearish: 100 * (0 - 4) / 4
    assert "price_below_cloud" in pt_bear.reasons
    assert "tk_bearish" in pt_bear.reasons
    assert "chikou_below_price" in pt_bear.reasons
    assert "future_cloud_bearish" in pt_bear.reasons


def test_ichimoku_optional_kijun_slope():
    """Verify 5-factor mode when include_kijun_slope=True."""
    bars = _generate_deterministic_bars(100, trend=1.5)
    score_5 = calculate_ichimoku_score(bars, include_kijun_slope=True)
    pt = score_5[80]
    assert pt.quality == SignalQuality.VALID
    # 5 bullish factors: 100 * (5 - 0) / 5 = 100.0
    assert pt.value == 100.0
    assert "kijun_slope_positive" in pt.reasons


# 4. Boolean Signals: ichimoku.bullish and ichimoku.bearish
def test_ichimoku_bullish_and_bearish_states():
    """Verify boolean state transitions with threshold defaults (+50 and -50)."""
    bullish_bars = _generate_deterministic_bars(100, trend=2.0)
    bull_signals = calculate_ichimoku_bullish(bullish_bars, threshold=50.0)
    bear_signals = calculate_ichimoku_bearish(bullish_bars, threshold=-50.0)

    # Before warmup
    assert bull_signals[10].quality == SignalQuality.INSUFFICIENT_HISTORY
    assert bull_signals[10].value is None

    # After warmup in strong uptrend
    assert bull_signals[80].quality == SignalQuality.VALID
    assert bull_signals[80].value is True
    assert len(bull_signals[80].reasons) > 0

    assert bear_signals[80].quality == SignalQuality.VALID
    assert bear_signals[80].value is False
    assert bear_signals[80].reasons == []


# 5. Transition Signals: TK Cross and Kumo Breakout
# 5. Transition Signals: TK Cross and Kumo Breakout
def test_ichimoku_tk_cross():
    """Verify TK bullish and bearish crossover detection."""
    bars = []
    # Bars 0..17: high 100, low 50
    for i in range(18):
        bars.append(CandleBar(i, f"2025-01-{(i%28)+1:02d}", 80.0, 100.0, 50.0, 80.0, 1000.0))
    # Bars 18..25: high 85, low 80 (low is higher, so bar 17 low 50 remains Kijun's min)
    for i in range(18, 26):
        bars.append(CandleBar(i, f"2025-01-{(i%28)+1:02d}", 82.0, 85.0, 80.0, 82.0, 1000.0))
    # Bar 26: price rallies to 120, low 80
    # Bar 17 (low 50) drops out of Tenkan's 9-period window (bars 18..26).
    # Tenkan = (120 + 80)/2 = 100. Kijun = (120 + 50)/2 = 85.
    bars.append(CandleBar(26, "2025-02-01", 85.0, 120.0, 80.0, 118.0, 5000.0))

    tk_up = calculate_ichimoku_tk_cross_bullish(bars)
    assert tk_up[26].quality == SignalQuality.VALID
    assert tk_up[26].value is True
    assert "tk_cross_bullish" in tk_up[26].reasons

    tk_dn = calculate_ichimoku_tk_cross_bearish(bars)
    assert tk_dn[26].value is False


def test_ichimoku_kumo_breakout():
    """Verify Kumo breakout above visible cloud."""
    bars = _generate_deterministic_bars(85, trend=0.0)
    # At bar 80, price spikes high above cloud
    bars[80] = CandleBar(80, "2025-03-20", 120.0, 150.0, 120.0, 148.0, 50000.0)

    breakout_pts = calculate_ichimoku_kumo_breakout_bullish(bars)
    # Bar 80 must trigger breakout
    assert breakout_pts[80].quality == SignalQuality.VALID
    assert breakout_pts[80].value is True
    assert "kumo_breakout_bullish" in breakout_pts[80].reasons


# 6. TEST-CAUSAL-001: Future Appending Invariance
def test_ichimoku_future_invariance():
    """TEST-CAUSAL-001: Appending future bars never changes historical signal values."""
    base_bars = _generate_deterministic_bars(90, trend=0.8)
    extended_bars = _generate_deterministic_bars(140, trend=-1.2)  # different trend in future

    # Override first 90 bars of extended to match base_bars exactly
    for i in range(90):
        extended_bars[i] = base_bars[i]

    signals_to_test = [
        "ichimoku.score",
        "ichimoku.bullish",
        "ichimoku.bearish",
        "ichimoku.tk_cross_bullish",
        "ichimoku.tk_cross_bearish",
        "ichimoku.kumo_breakout_bullish",
        "ichimoku.kumo_breakout_bearish",
    ]

    for sig_name in signals_to_test:
        base_res = SignalRegistry.calculate(sig_name, base_bars)
        ext_res = SignalRegistry.calculate(sig_name, extended_bars)

        assert len(base_res.points) == 90
        assert len(ext_res.points) == 140

        for t in range(90):
            p_base = base_res.points[t]
            p_ext = ext_res.points[t]

            assert p_base.quality == p_ext.quality, f"{sig_name} quality mismatch at {t}"
            assert p_base.value == p_ext.value, f"{sig_name} value mismatch at {t}"
            assert p_base.reasons == p_ext.reasons, f"{sig_name} reasons mismatch at {t}"


# 7. AST Signal Binding Integration
def test_ichimoku_ast_signal_binding():
    """Verify SignalBindingAdapter evaluates rules with ichimoku__* aliases."""
    bars = _generate_deterministic_bars(90, trend=2.0)
    score_res = SignalRegistry.calculate("ichimoku.score", bars)
    bull_res = SignalRegistry.calculate("ichimoku.bullish", bars)

    points_bar_80 = {
        "ichimoku__score": score_res.points[80],
        "ichimoku__bullish": bull_res.points[80],
    }

    # At bar 80 (strong bull)
    rule_score = {"gt": ["ichimoku__score", 50.0]}
    binding_score = SignalBindingAdapter.evaluate(rule_score, points_bar_80)
    assert binding_score.is_valid is True
    assert binding_score.value is True

    rule_bull = {"eq": ["ichimoku__bullish", True]}
    binding_bull = SignalBindingAdapter.evaluate(rule_bull, points_bar_80)
    assert binding_bull.is_valid is True
    assert binding_bull.value is True



# 8. Replay Indicator API is_projected Tagging
def test_replay_indicator_projection_tagging():
    """Verify replay indicator endpoint explicitly tags is_projected for future cloud points."""
    engine = create_engine(
        "sqlite:///:memory:",
        connect_args={"check_same_thread": False},
        poolclass=StaticPool,
    )
    TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)
    Base.metadata.create_all(bind=engine)
    db = TestingSessionLocal()

    symbol = "TEST_ICHI_PROJ"
    base_date = date(2025, 1, 1)

    # Seed 100 candles
    candles = []
    for i in range(100):
        c_date = base_date + timedelta(days=i)
        candle = Candle(
            symbol=symbol,
            timeframe="1D",
            adjustment_type="unadjusted",
            timestamp=c_date,
            open=100.0,
            high=105.0,
            low=95.0,
            close=102.0,
            volume=1000.0,
        )
        db.add(candle)
        candles.append(candle)
    db.commit()

    def override_get_db():
        try:
            yield db
        finally:
            pass

    app.dependency_overrides[get_db] = override_get_db
    client = TestClient(app)

    try:
        # Create session with 80 candles visible
        session_resp = client.post("/api/replay/sessions", json={
            "symbol": symbol,
            "timeframe": "1D",
            "adjustment_type": "unadjusted",
            "start_date": str(base_date),
            "end_date": str(base_date + timedelta(days=100)),
            "initial_cash": 100_000_000,
            "mode": "normal",
        })
        assert session_resp.status_code == 200
        session_id = session_resp.json()["id"]

        # Advance to step 79 (80 visible candles)
        adv_resp = client.post(f"/api/replay/sessions/{session_id}/next", params={"steps": 79})
        assert adv_resp.status_code == 200

        # Request ichimoku indicator
        res = client.get(
            f"/api/replay/sessions/{session_id}/indicators",
            params={"indicator": "ichimoku", "tenkan": 9, "kijun": 26, "senkou": 52},
        )
        assert res.status_code == 200, res.text
        data = res.json()["data"]

        # Assert data points have is_projected
        assert len(data) > 0
        visible_points = [p for p in data if not p.get("is_projected")]
        projected_points = [p for p in data if p.get("is_projected")]

        # Visible points must equal 80 (visible candles count)
        assert len(visible_points) == 80

        # Future projected points must exist (pandas-ta extends Span A & B forward by 26 bars)
        assert len(projected_points) == 26
        for p in projected_points:
            assert p.get("is_projected") is True


    finally:
        app.dependency_overrides.pop(get_db, None)
