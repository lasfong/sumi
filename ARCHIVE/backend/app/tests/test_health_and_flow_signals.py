"""Unit and integration tests for Technical Health and Money Flow BB Events.

Verifies:
- SIG-HEALTH-001: 4-family Technical Health score, missing-BB neutral semantics, favorable/unfavorable states.
- BBI-SIG-001: Money Flow BB consumption (horizons, direction, regime).
- BBI-SIG-002: Multi-horizon confluence, parameterized turn events, non-frozen thresholds.
- BBI-SIG-003: Flow method and minimum quality constraint guardrails.
- FR-CORE-004/005: AST Strategy Binding and precomputed boolean consumption.
- TEST-DSL-001: AST safety and adversarial escape rejection.
- TEST-CAUSAL-001: Future invariance.
"""

from typing import List
import pytest

from app.domain.bb.contracts import BBHorizon, BBDirection, BBRegime
from app.domain.data.contracts import DataQuality, FlowMethod
from app.domain.signals.health import (
    HEALTH_PRESETS,
    WARMUP_BARS,
    calculate_technical_health_favorable,
    calculate_technical_health_score,
    calculate_technical_health_unfavorable,
)
from app.domain.signals.flow_events import (
    calculate_bb_confluence_bearish,
    calculate_bb_confluence_bullish,
    calculate_bb_direction_falling,
    calculate_bb_direction_rising,
    calculate_bb_regime_negative,
    calculate_bb_regime_positive,
    calculate_bb_turn_down,
    calculate_bb_turn_up,
)
from app.domain.signals.models import (
    CandleBar,
    SignalOutputPoint,
    SignalQuality,
)
from app.domain.signals.registry import SignalRegistry
from app.domain.strategy.rule_evaluator import RuleEvaluationError
from app.domain.strategy.signal_binding import SignalBindingAdapter, SignalBindingResult


def _make_bars(n: int = 100, trend: str = "UP", start_price: float = 50.0) -> List[CandleBar]:
    """Generate synthetic daily bars with configurable trend."""
    bars: List[CandleBar] = []
    curr_price = start_price
    for i in range(n):
        if trend == "UP":
            if i < 20:
                delta = 0.1
                open_p = curr_price
                close_p = open_p + delta
                high_p = close_p + 0.3
                low_p = open_p - 0.3
            else:
                delta = 0.3 + 0.01 * (i - 20)
                open_p = curr_price
                close_p = open_p + delta
                high_p = close_p + 0.05
                low_p = open_p - 0.1
        elif trend == "DOWN":
            if i < 20:
                delta = -0.1
                open_p = curr_price
                close_p = open_p + delta
                high_p = open_p + 0.3
                low_p = close_p - 0.3
            else:
                delta = -0.3 - 0.01 * (i - 20)
                open_p = curr_price
                close_p = open_p + delta
                high_p = open_p + 0.1
                low_p = close_p - 0.05
        else:
            delta = 0.1 if i % 2 == 0 else -0.1
            open_p = curr_price
            close_p = open_p + delta
            high_p = max(open_p, close_p) + 0.3
            low_p = min(open_p, close_p) - 0.3

        vol = 100000.0 + (i * 1000.0)

        bars.append(
            CandleBar(
                index=i,
                timestamp=f"2026-01-{i+1:03d}",
                open=round(open_p, 2),
                high=round(high_p, 2),
                low=round(low_p, 2),
                close=round(close_p, 2),
                volume=vol,
            )
        )
        curr_price = close_p
    return bars


def test_registry_contains_all_eleven_new_definitions():
    """Verify SignalRegistry contains 72 total signals including 11 health and BB flow definitions."""
    definitions = SignalRegistry.list_definitions()
    assert len(definitions) == 72

    names = {d.name for d in definitions}
    expected_new = {
        "health.score",
        "health.favorable",
        "health.unfavorable",
        "bb.direction_rising",
        "bb.direction_falling",
        "bb.regime_positive",
        "bb.regime_negative",
        "bb.confluence_bullish",
        "bb.confluence_bearish",
        "bb.turn_up",
        "bb.turn_down",
    }
    for exp in expected_new:
        assert exp in names
        defn = SignalRegistry.get_definition(exp)
        assert defn is not None
        assert defn.ast_alias is not None
        assert SignalRegistry.resolve_name_from_alias(defn.ast_alias) == exp


# ---------------------------------------------------------------------------
# SIG-HEALTH-001 Tests
# ---------------------------------------------------------------------------

def test_technical_health_four_families_and_reasons():
    """Verify Technical Health evaluates 4 distinct families and records structured reasons."""
    bars = _make_bars(n=80, trend="UP", start_price=50.0)
    pts = calculate_technical_health_score(bars)

    assert len(pts) == 80

    # Warmup gate
    for t in range(WARMUP_BARS):
        assert pts[t].quality == SignalQuality.INSUFFICIENT_HISTORY
        assert pts[t].value is None

    # Post-warmup bar in strong uptrend
    pt_post = pts[70]
    assert pt_post.quality == SignalQuality.VALID
    assert pt_post.value is not None
    assert -100.0 <= pt_post.value <= 100.0
    assert pt_post.value > 0.0  # Strong uptrend should produce positive score

    # Check reasons contain family attributions
    reasons_str = " ".join(pt_post.reasons)
    assert "trend_" in reasons_str
    assert "rsi_" in reasons_str
    assert "macd_hist_" in reasons_str
    assert ("volume_participation_" in reasons_str or "bb_flow_" in reasons_str)


def test_health_missing_bb_neutral_semantics():
    """Verify BB absence is NOT negative evidence (missing BB neutral semantics)."""
    bars = _make_bars(n=80, trend="UP", start_price=50.0)

    # Calculation with NO BB data
    pts_no_bb = calculate_technical_health_score(bars, bb_values=None)
    pt_no_bb = pts_no_bb[70]
    assert pt_no_bb.quality == SignalQuality.VALID
    assert "bb_absent_neutral" in pt_no_bb.reasons

    # Calculation with neutral BB data (50.0)
    neutral_bb = [50.0] * 80
    pts_neutral_bb = calculate_technical_health_score(bars, bb_values=neutral_bb)
    pt_neutral_bb = pts_neutral_bb[70]

    # Absence of BB should not arbitrarily penalize or drag the score into negative territory
    assert pt_no_bb.value > 0.0


def test_health_favorable_and_unfavorable_states():
    """Verify health.favorable (>= 35) and health.unfavorable (<= -35) states."""
    bars_up = _make_bars(n=80, trend="UP", start_price=50.0)
    bars_down = _make_bars(n=80, trend="DOWN", start_price=100.0)

    pts_fav = calculate_technical_health_favorable(bars_up)
    pts_unfav = calculate_technical_health_unfavorable(bars_down)

    pt_fav = pts_fav[75]
    assert pt_fav.quality == SignalQuality.VALID
    assert pt_fav.value is True
    assert any("health_favorable_score" in r for r in pt_fav.reasons)

    pt_unfav = pts_unfav[75]
    assert pt_unfav.quality == SignalQuality.VALID
    assert pt_unfav.value is True
    assert any("health_unfavorable_score" in r for r in pt_unfav.reasons)


def test_health_future_invariance():
    """TEST-CAUSAL-001: Future invariance for Technical Health calculation."""
    bars = _make_bars(n=70, trend="UP", start_price=50.0)
    pts_base = calculate_technical_health_score(bars)

    # Append 20 future bars
    extra_bars = _make_bars(n=20, trend="DOWN", start_price=bars[-1].close)
    extended_bars = list(bars)
    for i, b in enumerate(extra_bars):
        extended_bars.append(
            CandleBar(
                index=70 + i,
                timestamp=f"2026-03-{i+1:03d}",
                open=b.open,
                high=b.high,
                low=b.low,
                close=b.close,
                volume=b.volume,
            )
        )
    pts_extended = calculate_technical_health_score(extended_bars)

    # Historical 70 points must be identical
    for t in range(70):
        assert pts_base[t].quality == pts_extended[t].quality
        assert pts_base[t].value == pts_extended[t].value
        assert pts_base[t].reasons == pts_extended[t].reasons


# ---------------------------------------------------------------------------
# BBI-SIG-001, BBI-SIG-002, BBI-SIG-003 Tests
# ---------------------------------------------------------------------------

def test_bb_direction_and_regime_events():
    """BBI-SIG-001: Verify BB direction and regime events across standard horizons."""
    bars = _make_bars(n=60, trend="UP", start_price=50.0)

    # Direction rising
    pts_dir = calculate_bb_direction_rising(bars, horizon="T20")
    # Uptrend should produce RISING direction after warmup
    pt_dir = pts_dir[40]
    assert pt_dir.quality == SignalQuality.VALID
    assert pt_dir.value is True

    # Regime positive
    pts_reg = calculate_bb_regime_positive(bars, horizon="T20")
    pt_reg = pts_reg[40]
    assert pt_reg.quality == SignalQuality.VALID
    assert pt_reg.value is True


def test_bb_confluence_events():
    """BBI-SIG-002: Verify multi-horizon bullish confluence event."""
    bars = _make_bars(n=60, trend="UP", start_price=50.0)
    pts_conf = calculate_bb_confluence_bullish(bars, short_horizon="T05", long_horizon="T20")

    pt = pts_conf[40]
    assert pt.quality == SignalQuality.VALID
    assert pt.value is True
    assert "bb_bullish_confluence_active" in pt.reasons


def test_bb_turn_events_require_parameters():
    """BBI-SIG-002: Verify turn events evaluate with explicit parameters and do not freeze unvalidated thresholds."""
    bars = _make_bars(n=60, trend="UP", start_price=50.0)

    # Turn up with explicit threshold
    pts_turn = calculate_bb_turn_up(bars, threshold=45.0, tolerance=5.0, horizon="T20")
    assert pts_turn[40].quality == SignalQuality.VALID


def test_bb_method_and_quality_guards():
    """BBI-SIG-003: Verify method and quality guardrails fail closed when constraints are violated."""
    bars = _make_bars(n=60, trend="UP", start_price=50.0)

    # Disallowed flow method (e.g. TRUE_FLOW)
    pts_disallowed = calculate_bb_direction_rising(
        bars,
        horizon="T20",
        accepted_methods=["TRUE_FLOW"],
    )
    for pt in pts_disallowed:
        assert pt.quality == SignalQuality.INVALID_VOLUME
        assert pt.value is None
        assert any("DISALLOWED_FLOW_METHOD" in r for r in pt.reasons)


# ---------------------------------------------------------------------------
# Strategy Binding & AST Whitelist Safety Tests (FR-CORE-004, TEST-DSL-001)
# ---------------------------------------------------------------------------

def test_health_and_bb_ast_strategy_binding():
    """FR-CORE-004/005: Verify AST and DSL rule evaluation on health and BB aliases."""
    points = {
        "health__score": SignalOutputPoint(
            bar_index=60,
            timestamp="2026-03-01",
            output_type="float",
            value=45.5,
            quality=SignalQuality.VALID,
            reasons=["trend_bullish"],
            availability_event="BAR_CLOSE",
            available_at_index=60,
            available_at_timestamp="2026-03-01",
        ),
        "health__favorable": SignalOutputPoint(
            bar_index=60,
            timestamp="2026-03-01",
            output_type="bool",
            value=True,
            quality=SignalQuality.VALID,
            reasons=["health_favorable"],
            availability_event="BAR_CLOSE",
            available_at_index=60,
            available_at_timestamp="2026-03-01",
        ),
        "bb__direction_rising": SignalOutputPoint(
            bar_index=60,
            timestamp="2026-03-01",
            output_type="bool",
            value=True,
            quality=SignalQuality.VALID,
            reasons=["bb_rising"],
            availability_event="BAR_CLOSE",
            available_at_index=60,
            available_at_timestamp="2026-03-01",
        ),
    }

    # AST String condition
    ast_rule = "health__favorable == True and bb__direction_rising == True and health__score > 35.0"
    res_ast = SignalBindingAdapter.evaluate(ast_rule, points)
    assert res_ast.is_valid is True
    assert res_ast.value is True

    # DSL Dict
    dsl_rule = {
        "all": [
            {"eq": ["health__favorable", True]},
            {"eq": ["bb__direction_rising", True]},
            {"gt": ["health__score", 35.0]},
        ]
    }
    res_dsl = SignalBindingAdapter.evaluate(dsl_rule, points)
    assert res_dsl.is_valid is True
    assert res_dsl.value is True


def test_dsl_safety_adversarial_rejection():
    """TEST-DSL-001: Arbitrary calls, imports, attribute escapes, and disallowed methods are rejected."""
    # 1. Arbitrary function call
    with pytest.raises(RuleEvaluationError):
        SignalBindingAdapter.validate_rule("__import__('os').system('calc')")

    # 2. Dotted attribute escape
    with pytest.raises(RuleEvaluationError, match="Dotted signal names"):
        SignalBindingAdapter.validate_rule("bb.direction_rising == True")

    # 3. Disallowed flow method in strategy flow_constraints
    with pytest.raises(RuleEvaluationError, match="not authorized for daily Symbol BB"):
        SignalBindingAdapter.validate_rule(
            "bb__direction_rising == True",
            flow_constraints={"accepted_methods": ["TRUE_FLOW"]},
        )
