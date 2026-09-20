"""Unit tests for SignalBindingAdapter.

Verifies acceptance oracles P1-OR-07 and P1-OR-08.
Tests that unavailable signals return unavailable results even under negation ('not'),
equality-to-false, or 'any' conditions, preventing unknown/missing signals from becoming True.
Verifies rejection of dotted identifiers, unknown aliases, and unsafe syntax.
"""

import pytest

from app.domain.signals.models import SignalOutputPoint, SignalQuality
from app.domain.strategy.rule_evaluator import RuleEvaluationError
from app.domain.strategy.signal_binding import SignalBindingAdapter, SignalBindingResult


def _make_point(
    value: bool | None,
    quality: SignalQuality,
    bar_index: int = 5,
    available_at_index: int | None = None,
    availability_event: str = "BAR_CLOSE",
    output_type: str = "bool",
) -> SignalOutputPoint:
    avail_idx = bar_index if available_at_index is None else available_at_index
    return SignalOutputPoint(
        bar_index=bar_index,
        timestamp="2026-01-06",
        output_type=output_type,
        value=value,
        quality=quality,
        reasons=[quality.value],
        baseline=200.0 if quality == SignalQuality.VALID else None,
        current_volume=400.0 if quality == SignalQuality.VALID else None,
        relative_volume=2.0 if quality == SignalQuality.VALID else None,
        threshold=2.0,
        availability_event=availability_event,
        available_at_index=avail_idx,
        available_at_timestamp="2026-01-06",
    )


def test_p1_or_07_valid_and_unavailable_dsl_evaluation():
    """P1-OR-07: Valid spike true/false and unavailable spike under AST and explicit-object DSL.
    
    Valid cases agree. Unavailable produces unavailable even under 'not', OR with true branch,
    or equality to false; adapter does not call evaluator on missing dependencies.
    """
    # 1. Valid True Spike
    points_true = {"volume__spike": _make_point(True, SignalQuality.VALID)}

    # AST
    res_ast_true = SignalBindingAdapter.evaluate("volume__spike == True", points_true)
    assert res_ast_true.is_valid is True
    assert res_ast_true.value is True

    # DSL
    res_dsl_true = SignalBindingAdapter.evaluate({"eq": ["volume__spike", True]}, points_true)
    assert res_dsl_true.is_valid is True
    assert res_dsl_true.value is True

    # 2. Valid False Spike
    points_false = {"volume__spike": _make_point(False, SignalQuality.VALID)}

    # AST
    res_ast_false = SignalBindingAdapter.evaluate("volume__spike == True", points_false)
    assert res_ast_false.is_valid is True
    assert res_ast_false.value is False

    # DSL
    res_dsl_false = SignalBindingAdapter.evaluate({"eq": ["volume__spike", True]}, points_false)
    assert res_dsl_false.is_valid is True
    assert res_dsl_false.value is False

    # 3. Unavailable Spike (e.g. INSUFFICIENT_HISTORY)
    points_unavail = {"volume__spike": _make_point(None, SignalQuality.INSUFFICIENT_HISTORY)}

    # AST direct equality to false: "volume__spike == False" MUST NOT evaluate to True!
    res_eq_false = SignalBindingAdapter.evaluate("volume__spike == False", points_unavail)
    assert res_eq_false.is_valid is False
    assert res_eq_false.value is None
    assert "DEPENDENCY_UNAVAILABLE" in res_eq_false.reason

    # DSL object: {"eq": ["volume__spike", False]}
    res_dsl_eq_false = SignalBindingAdapter.evaluate({"eq": ["volume__spike", False]}, points_unavail)
    assert res_dsl_eq_false.is_valid is False
    assert res_dsl_eq_false.value is None

    # DSL object with "not": {"not": {"eq": ["volume__spike", True]}} MUST NOT become True!
    res_dsl_not = SignalBindingAdapter.evaluate({"not": {"eq": ["volume__spike", True]}}, points_unavail)
    assert res_dsl_not.is_valid is False
    assert res_dsl_not.value is None

    # DSL object with "any": {"any": [{"eq": ["volume__spike", True]}, True]} MUST NOT become True!
    res_dsl_any = SignalBindingAdapter.evaluate({"any": [{"eq": ["volume__spike", True]}, True]}, points_unavail)
    assert res_dsl_any.is_valid is False
    assert res_dsl_any.value is None

    # Missing dependency altogether
    res_missing = SignalBindingAdapter.evaluate({"eq": ["volume__spike", True]}, {})
    assert res_missing.is_valid is False
    assert res_missing.value is None
    assert "MISSING_SIGNAL_DEPENDENCY" in res_missing.reason


def test_p1_or_08_rejections_and_ast_safety():
    """P1-OR-08: volume.spike (dotted), unknown alias, arbitrary call/import/attribute expression.
    
    Explicit rejection; no widened grammar, silent overwrite or default substitution.
    """
    points = {"volume__spike": _make_point(True, SignalQuality.VALID)}

    # Dotted identifier rejection in validation
    with pytest.raises(RuleEvaluationError, match="Dotted signal names"):
        SignalBindingAdapter.validate_rule("volume.spike == True")

    # Dotted identifier in evaluation raises RuleEvaluationError
    with pytest.raises(RuleEvaluationError, match="Dotted signal names"):
        SignalBindingAdapter.evaluate("volume.spike == True", points)

    # Unknown alias rejection in validation
    with pytest.raises(RuleEvaluationError, match="Unknown identifier"):
        SignalBindingAdapter.validate_rule("unknown__signal == True")

    # Unknown alias in evaluation raises RuleEvaluationError
    with pytest.raises(RuleEvaluationError, match="Unknown identifier"):
        SignalBindingAdapter.evaluate("unknown__signal == True", points)

    # Arbitrary function call / import rejection
    with pytest.raises(RuleEvaluationError):
        SignalBindingAdapter.validate_rule("__import__('os').system('calc')")

    with pytest.raises(RuleEvaluationError):
        SignalBindingAdapter.evaluate("__import__('os').system('calc')", points)

    # Attribute access rejection
    with pytest.raises(RuleEvaluationError):
        SignalBindingAdapter.validate_rule("volume.foo == True")

    with pytest.raises(RuleEvaluationError):
        SignalBindingAdapter.evaluate("volume.foo == True", points)


def _make_rvol_point(
    val: float | None,
    quality: SignalQuality,
    bar_index: int = 5,
    available_at_index: int | None = None,
    availability_event: str = "BAR_CLOSE",
    output_type: str = "float",
) -> SignalOutputPoint:
    avail_idx = bar_index if available_at_index is None else available_at_index
    return SignalOutputPoint(
        bar_index=bar_index,
        timestamp="2026-01-06",
        output_type=output_type,
        value=val,
        quality=quality,
        reasons=[quality.value],
        baseline=200.0 if quality == SignalQuality.VALID else None,
        current_volume=400.0 if quality == SignalQuality.VALID else None,
        relative_volume=val if quality == SignalQuality.VALID else None,
        threshold=None,
        availability_event=availability_event,
        available_at_index=avail_idx,
        available_at_timestamp="2026-01-06",
    )


def test_cross_up_and_cross_down_valid_evaluation():
    """Verify valid cross_up and cross_down over current (bar 5) and previous (bar 4) Relative Volume points."""
    # cross_up: previous=1.8 <= 2.0 (bar 4), current=2.2 > 2.0 (bar 5) -> True
    curr_points = {"volume__relative_volume": _make_rvol_point(2.2, SignalQuality.VALID, bar_index=5)}
    prev_points = {"volume__relative_volume": _make_rvol_point(1.8, SignalQuality.VALID, bar_index=4)}

    rule_cross_up = {"cross_up": ["volume__relative_volume", 2.0]}
    res_up_true = SignalBindingAdapter.evaluate(
        rule=rule_cross_up,
        signal_points=curr_points,
        previous_signal_points=prev_points,
    )
    assert res_up_true.is_valid is True
    assert res_up_true.value is True
    assert res_up_true.quality == SignalQuality.VALID

    # cross_up: previous=2.2 (already above, bar 4), current=2.5 (bar 5) -> False
    curr_points_high = {"volume__relative_volume": _make_rvol_point(2.5, SignalQuality.VALID, bar_index=5)}
    prev_points_high = {"volume__relative_volume": _make_rvol_point(2.2, SignalQuality.VALID, bar_index=4)}
    res_up_false = SignalBindingAdapter.evaluate(
        rule=rule_cross_up,
        signal_points=curr_points_high,
        previous_signal_points=prev_points_high,
    )
    assert res_up_false.is_valid is True
    assert res_up_false.value is False

    # cross_down: previous=2.2 >= 2.0 (bar 4), current=1.8 < 2.0 (bar 5) -> True
    curr_points_down = {"volume__relative_volume": _make_rvol_point(1.8, SignalQuality.VALID, bar_index=5)}
    prev_points_down = {"volume__relative_volume": _make_rvol_point(2.2, SignalQuality.VALID, bar_index=4)}
    rule_cross_down = {"cross_down": ["volume__relative_volume", 2.0]}
    res_down_true = SignalBindingAdapter.evaluate(
        rule=rule_cross_down,
        signal_points=curr_points_down,
        previous_signal_points=prev_points_down,
    )
    assert res_down_true.is_valid is True
    assert res_down_true.value is True

    # cross_down: previous=1.8 (already below, bar 4), current=1.5 (bar 5) -> False
    curr_points_low = {"volume__relative_volume": _make_rvol_point(1.5, SignalQuality.VALID, bar_index=5)}
    res_down_false = SignalBindingAdapter.evaluate(
        rule=rule_cross_down,
        signal_points=curr_points_low,
        previous_signal_points=prev_points,
    )
    assert res_down_false.is_valid is True
    assert res_down_false.value is False


def test_cross_missing_or_invalid_previous_point_fails_closed():
    """Verify missing or invalid previous signal point returns nullable/fail-closed and never enters boolean evaluation."""
    curr_points = {"volume__relative_volume": _make_rvol_point(2.2, SignalQuality.VALID, bar_index=5)}
    rule_cross_up = {"cross_up": ["volume__relative_volume", 2.0]}

    # 1. previous_signal_points is None
    res_none = SignalBindingAdapter.evaluate(
        rule=rule_cross_up,
        signal_points=curr_points,
        previous_signal_points=None,
    )
    assert res_none.is_valid is False
    assert res_none.value is None
    assert "MISSING_PREVIOUS_SIGNAL_DEPENDENCY" in res_none.reason

    # 2. previous_signal_points is empty dict
    res_empty = SignalBindingAdapter.evaluate(
        rule=rule_cross_up,
        signal_points=curr_points,
        previous_signal_points={},
    )
    assert res_empty.is_valid is False
    assert res_empty.value is None
    assert "MISSING_PREVIOUS_SIGNAL_DEPENDENCY" in res_empty.reason

    # 3. previous_signal_points has INSUFFICIENT_HISTORY (bar 4)
    prev_unavail = {"volume__relative_volume": _make_rvol_point(None, SignalQuality.INSUFFICIENT_HISTORY, bar_index=4)}
    res_unavail = SignalBindingAdapter.evaluate(
        rule=rule_cross_up,
        signal_points=curr_points,
        previous_signal_points=prev_unavail,
    )
    assert res_unavail.is_valid is False
    assert res_unavail.value is None
    assert "DEPENDENCY_UNAVAILABLE_previous_volume__relative_volume_INSUFFICIENT_HISTORY" in res_unavail.reason

    # 4. previous_signal_points has ZERO_BASELINE (bar 4)
    prev_zero = {"volume__relative_volume": _make_rvol_point(None, SignalQuality.ZERO_BASELINE, bar_index=4)}
    res_zero = SignalBindingAdapter.evaluate(
        rule=rule_cross_up,
        signal_points=curr_points,
        previous_signal_points=prev_zero,
    )
    assert res_zero.is_valid is False
    assert res_zero.value is None
    assert res_zero.quality == SignalQuality.ZERO_BASELINE

    # 5. previous_signal_points has INVALID_VOLUME (bar 4)
    prev_invalid = {"volume__relative_volume": _make_rvol_point(None, SignalQuality.INVALID_VOLUME, bar_index=4)}
    res_invalid = SignalBindingAdapter.evaluate(
        rule=rule_cross_up,
        signal_points=curr_points,
        previous_signal_points=prev_invalid,
    )
    assert res_invalid.is_valid is False
    assert res_invalid.value is None
    assert res_invalid.quality == SignalQuality.INVALID_VOLUME

    # 6. 'not' condition MUST NOT become True on missing/invalid previous point
    rule_not_cross = {"not": {"cross_up": ["volume__relative_volume", 2.0]}}
    res_not = SignalBindingAdapter.evaluate(
        rule=rule_not_cross,
        signal_points=curr_points,
        previous_signal_points=None,
    )
    assert res_not.is_valid is False
    assert res_not.value is None

    # 7. 'any' with a True branch MUST NOT become True on missing/invalid previous point
    rule_any_cross = {"any": [{"cross_up": ["volume__relative_volume", 2.0]}, True]}
    res_any = SignalBindingAdapter.evaluate(
        rule=rule_any_cross,
        signal_points=curr_points,
        previous_signal_points=None,
    )
    assert res_any.is_valid is False
    assert res_any.value is None

    # 8. Current signal point is invalid, even if previous point is valid
    curr_invalid = {"volume__relative_volume": _make_rvol_point(None, SignalQuality.INVALID_VOLUME, bar_index=5)}
    prev_valid = {"volume__relative_volume": _make_rvol_point(1.8, SignalQuality.VALID, bar_index=4)}
    res_curr_inv = SignalBindingAdapter.evaluate(
        rule=rule_cross_up,
        signal_points=curr_invalid,
        previous_signal_points=prev_valid,
    )
    assert res_curr_inv.is_valid is False
    assert res_curr_inv.value is None
    assert "DEPENDENCY_UNAVAILABLE_volume__relative_volume_INVALID_VOLUME" in res_curr_inv.reason


def test_cross_same_bar_future_and_non_adjacent_previous_points_fail_closed():
    """Verify same-bar, future, and non-adjacent previous points fail closed with INSUFFICIENT_HISTORY."""
    curr_points = {"volume__relative_volume": _make_rvol_point(2.2, SignalQuality.VALID, bar_index=5)}
    rule_cross = {"cross_up": ["volume__relative_volume", 2.0]}

    # 1. Same-bar previous point (bar 5 vs current bar 5)
    prev_same_bar = {"volume__relative_volume": _make_rvol_point(1.8, SignalQuality.VALID, bar_index=5)}
    res_same = SignalBindingAdapter.evaluate(
        rule=rule_cross,
        signal_points=curr_points,
        previous_signal_points=prev_same_bar,
    )
    assert res_same.is_valid is False
    assert res_same.value is None
    assert res_same.quality == SignalQuality.INSUFFICIENT_HISTORY
    assert "NON_ADJACENT_PREVIOUS_SIGNAL" in res_same.reason

    # 2. Future previous point (bar 6 vs current bar 5)
    prev_future = {"volume__relative_volume": _make_rvol_point(1.8, SignalQuality.VALID, bar_index=6)}
    res_future = SignalBindingAdapter.evaluate(
        rule=rule_cross,
        signal_points=curr_points,
        previous_signal_points=prev_future,
    )
    assert res_future.is_valid is False
    assert res_future.value is None
    assert res_future.quality == SignalQuality.INSUFFICIENT_HISTORY
    assert "NON_ADJACENT_PREVIOUS_SIGNAL" in res_future.reason

    # 3. Non-adjacent previous point (bar 3 vs current bar 5, gap of 2 bars)
    prev_gap = {"volume__relative_volume": _make_rvol_point(1.8, SignalQuality.VALID, bar_index=3)}
    res_gap = SignalBindingAdapter.evaluate(
        rule=rule_cross,
        signal_points=curr_points,
        previous_signal_points=prev_gap,
    )
    assert res_gap.is_valid is False
    assert res_gap.value is None
    assert res_gap.quality == SignalQuality.INSUFFICIENT_HISTORY
    assert "NON_ADJACENT_PREVIOUS_SIGNAL" in res_gap.reason


def test_float_snapshot_with_nan_inf_fails_closed_before_evaluation():
    """Verify current or previous float snapshot with NaN/Infinity fails closed before evaluation."""
    prev_valid = {"volume__relative_volume": _make_rvol_point(1.8, SignalQuality.VALID, bar_index=4)}
    curr_valid = {"volume__relative_volume": _make_rvol_point(2.2, SignalQuality.VALID, bar_index=5)}
    rule_cross = {"cross_up": ["volume__relative_volume", 2.0]}
    rule_direct = "volume__relative_volume > 2.0"

    # Current with NaN
    curr_nan = {"volume__relative_volume": _make_rvol_point(float("nan"), SignalQuality.VALID, bar_index=5)}
    res_c_nan_dir = SignalBindingAdapter.evaluate(rule_direct, curr_nan)
    assert res_c_nan_dir.is_valid is False
    assert res_c_nan_dir.value is None
    assert res_c_nan_dir.quality == SignalQuality.INVALID_VOLUME
    assert "MALFORMED_SIGNAL_VALUE" in res_c_nan_dir.reason

    res_c_nan_cross = SignalBindingAdapter.evaluate(rule_cross, curr_nan, previous_signal_points=prev_valid)
    assert res_c_nan_cross.is_valid is False
    assert res_c_nan_cross.value is None
    assert res_c_nan_cross.quality == SignalQuality.INVALID_VOLUME

    # Current with Infinity
    curr_inf = {"volume__relative_volume": _make_rvol_point(float("inf"), SignalQuality.VALID, bar_index=5)}
    res_c_inf = SignalBindingAdapter.evaluate(rule_direct, curr_inf)
    assert res_c_inf.is_valid is False
    assert res_c_inf.value is None
    assert res_c_inf.quality == SignalQuality.INVALID_VOLUME

    # Previous with NaN
    prev_nan = {"volume__relative_volume": _make_rvol_point(float("nan"), SignalQuality.VALID, bar_index=4)}
    res_p_nan = SignalBindingAdapter.evaluate(rule_cross, curr_valid, previous_signal_points=prev_nan)
    assert res_p_nan.is_valid is False
    assert res_p_nan.value is None
    assert res_p_nan.quality == SignalQuality.INVALID_VOLUME
    assert "MALFORMED_SIGNAL_VALUE_previous" in res_p_nan.reason

    # Previous with Infinity
    prev_inf = {"volume__relative_volume": _make_rvol_point(float("inf"), SignalQuality.VALID, bar_index=4)}
    res_p_inf = SignalBindingAdapter.evaluate(rule_cross, curr_valid, previous_signal_points=prev_inf)
    assert res_p_inf.is_valid is False
    assert res_p_inf.value is None
    assert res_p_inf.quality == SignalQuality.INVALID_VOLUME
    assert "MALFORMED_SIGNAL_VALUE_previous" in res_p_inf.reason


def test_type_mismatched_runtime_values_fail_closed():
    """Verify float signal with bool/string and bool signal with numeric/string fail closed."""
    # Float signal with bool value True
    p_float_bool = _make_rvol_point(True, SignalQuality.VALID, bar_index=5)  # type: ignore
    res_fb = SignalBindingAdapter.evaluate("volume__relative_volume > 1.0", {"volume__relative_volume": p_float_bool})
    assert res_fb.is_valid is False
    assert res_fb.value is None
    assert res_fb.quality == SignalQuality.INVALID_VOLUME
    assert "MALFORMED_SIGNAL_VALUE_volume__relative_volume_EXPECTED_FLOAT" in res_fb.reason

    # Float signal with string value "2.5"
    p_float_str = _make_rvol_point("2.5", SignalQuality.VALID, bar_index=5)  # type: ignore
    res_fs = SignalBindingAdapter.evaluate("volume__relative_volume > 1.0", {"volume__relative_volume": p_float_str})
    assert res_fs.is_valid is False
    assert res_fs.value is None
    assert res_fs.quality == SignalQuality.INVALID_VOLUME

    # Bool signal with int value 1
    p_bool_int = _make_point(1, SignalQuality.VALID, bar_index=5)  # type: ignore
    res_bi = SignalBindingAdapter.evaluate("volume__spike == True", {"volume__spike": p_bool_int})
    assert res_bi.is_valid is False
    assert res_bi.value is None
    assert res_bi.quality == SignalQuality.INVALID_VOLUME
    assert "MALFORMED_SIGNAL_VALUE_volume__spike_EXPECTED_BOOL" in res_bi.reason

    # Bool signal with string value "True"
    p_bool_str = _make_point("True", SignalQuality.VALID, bar_index=5)  # type: ignore
    res_bs = SignalBindingAdapter.evaluate("volume__spike == True", {"volume__spike": p_bool_str})
    assert res_bs.is_valid is False
    assert res_bs.value is None
    assert res_bs.quality == SignalQuality.INVALID_VOLUME


def test_mismatched_output_type_and_invalid_availability_metadata_fail_closed():
    """Verify mismatched output_type and invalid availability metadata fail closed."""
    # Mismatched output_type (volume.spike has output_type float instead of bool)
    p_bad_type = _make_point(True, SignalQuality.VALID, bar_index=5, output_type="float")
    res_type = SignalBindingAdapter.evaluate("volume__spike == True", {"volume__spike": p_bad_type})
    assert res_type.is_valid is False
    assert res_type.value is None
    assert res_type.quality == SignalQuality.INVALID_VOLUME
    assert "MISMATCHED_OUTPUT_TYPE" in res_type.reason

    # Invalid availability index (available_at_index != bar_index)
    p_bad_idx = _make_point(True, SignalQuality.VALID, bar_index=5, available_at_index=4)
    res_idx = SignalBindingAdapter.evaluate("volume__spike == True", {"volume__spike": p_bad_idx})
    assert res_idx.is_valid is False
    assert res_idx.value is None
    assert res_idx.quality == SignalQuality.INVALID_VOLUME
    assert "INVALID_AVAILABILITY_METADATA" in res_idx.reason

    # Invalid availability event (not BAR_CLOSE)
    p_bad_evt = _make_point(True, SignalQuality.VALID, bar_index=5, availability_event="INTRADAY_TICK")
    res_evt = SignalBindingAdapter.evaluate("volume__spike == True", {"volume__spike": p_bad_evt})
    assert res_evt.is_valid is False
    assert res_evt.value is None
    assert res_evt.quality == SignalQuality.INVALID_VOLUME
    assert "INVALID_AVAILABILITY_METADATA" in res_evt.reason

    # Previous point with invalid availability metadata
    curr_ok = {"volume__relative_volume": _make_rvol_point(2.2, SignalQuality.VALID, bar_index=5)}
    prev_bad_meta = {"volume__relative_volume": _make_rvol_point(1.8, SignalQuality.VALID, bar_index=4, available_at_index=3)}
    res_prev_meta = SignalBindingAdapter.evaluate(
        rule={"cross_up": ["volume__relative_volume", 2.0]},
        signal_points=curr_ok,
        previous_signal_points=prev_bad_meta,
    )
    assert res_prev_meta.is_valid is False
    assert res_prev_meta.value is None
    assert res_prev_meta.quality == SignalQuality.INVALID_VOLUME
    assert "INVALID_AVAILABILITY_METADATA_previous" in res_prev_meta.reason


def test_float_snapshot_with_oversized_int_fails_closed_before_evaluation():
    """Verify current and previous float snapshots containing 10**10000 fail closed without entering evaluator."""
    # 1. Current float snapshot with 10**10000
    p_huge_curr = _make_rvol_point(10**10000, SignalQuality.VALID, bar_index=5)  # type: ignore
    res_curr = SignalBindingAdapter.evaluate(
        rule="volume__relative_volume > 1.0",
        signal_points={"volume__relative_volume": p_huge_curr},
    )
    assert res_curr.is_valid is False
    assert res_curr.value is None
    assert res_curr.quality == SignalQuality.INVALID_VOLUME
    assert "MALFORMED_SIGNAL_VALUE_volume__relative_volume_EXPECTED_FLOAT" in res_curr.reason

    # 2. Previous float snapshot with 10**10000
    curr_ok = {"volume__relative_volume": _make_rvol_point(2.5, SignalQuality.VALID, bar_index=5)}
    p_huge_prev = _make_rvol_point(10**10000, SignalQuality.VALID, bar_index=4)  # type: ignore
    res_prev = SignalBindingAdapter.evaluate(
        rule={"cross_up": ["volume__relative_volume", 2.0]},
        signal_points=curr_ok,
        previous_signal_points={"volume__relative_volume": p_huge_prev},
    )
    assert res_prev.is_valid is False
    assert res_prev.value is None
    assert res_prev.quality == SignalQuality.INVALID_VOLUME
    assert "MALFORMED_SIGNAL_VALUE_previous_volume__relative_volume_EXPECTED_FLOAT" in res_prev.reason
