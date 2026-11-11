"""Unit tests for pure signal domain models, registry, and volume calculators.

Covers numerical acceptance oracles P1-OR-01, P1-OR-02, P1-OR-03, P1-OR-04,
P1-OR-05, P1-OR-06, and P1-OR-11.
Zero database, FastAPI, or ORM dependencies.
"""

import math
import pytest

from app.domain.signals.models import (
    CandleBar,
    SignalOutputPoint,
    SignalQuality,
    SignalOutputType,
    compute_canonical_params_hash,
)

from app.domain.signals.registry import SignalRegistry
from app.domain.signals.volume import (
    calculate_relative_volume,
    calculate_volume_spike,
    validate_relative_volume_params,
    validate_volume_spike_params,
)


def _make_bars(volumes) -> list[CandleBar]:
    """Helper to create dummy daily CandleBar list with sequential timestamps."""
    bars = []
    for i, v in enumerate(volumes):
        bars.append(
            CandleBar(
                index=i,
                timestamp=f"2026-01-{i+1:02d}",
                open=100.0,
                high=105.0,
                low=95.0,
                close=102.0,
                volume=v,
            )
        )
    return bars


def test_p1_or_01_volume_spike_baseline_and_spike_true():
    """P1-OR-01: Volumes [100,200,300,400,600], period 3, multiplier 2
    
    t=0..2 unavailable (INSUFFICIENT_HISTORY)
    t=3 baseline 200, RVOL 2.0, spike true (VALID)
    t=4 baseline 300, RVOL 2.0, spike true (VALID)
    """
    bars = _make_bars([100, 200, 300, 400, 600])
    points = calculate_volume_spike(bars, period=3, multiplier=2.0)

    assert len(points) == 5

    # t=0..2
    for t in range(3):
        assert points[t].quality == SignalQuality.INSUFFICIENT_HISTORY
        assert points[t].value is None
        assert points[t].baseline is None
        assert points[t].relative_volume is None

    # t=3: window is [100, 200, 300], sum=600, mean=200, curr=400, RVOL=2.0 >= 2.0 -> True
    assert points[3].quality == SignalQuality.VALID
    assert points[3].baseline == 200.0
    assert points[3].relative_volume == 2.0
    assert points[3].value is True
    assert "VOLUME_SPIKE" in points[3].reasons

    # t=4: window is [200, 300, 400], sum=900, mean=300, curr=600, RVOL=2.0 >= 2.0 -> True
    assert points[4].quality == SignalQuality.VALID
    assert points[4].baseline == 300.0
    assert points[4].relative_volume == 2.0
    assert points[4].value is True
    assert "VOLUME_SPIKE" in points[4].reasons


def test_p1_or_02_volume_spike_false_boundary():
    """P1-OR-02: Volumes [100,200,300,399], period 3, multiplier 2
    
    t=3 baseline 200, RVOL 1.995, false (VALID); UI rounding cannot make it true
    """
    bars = _make_bars([100, 200, 300, 399])
    points = calculate_volume_spike(bars, period=3, multiplier=2.0)

    assert len(points) == 4
    assert points[3].quality == SignalQuality.VALID
    assert points[3].baseline == 200.0
    assert points[3].relative_volume == 1.995
    assert points[3].value is False
    assert "NORMAL_VOLUME" in points[3].reasons


def test_p1_or_03_zero_baseline_vs_zero_current_volume():
    """P1-OR-03: Zero baseline vs zero current volume.
    
    [0,0,0,100] -> null / ZERO_BASELINE
    [100,100,100,0] -> RVOL 0 / false / VALID
    """
    # Case 1: [0, 0, 0, 100]
    bars1 = _make_bars([0, 0, 0, 100])
    points1 = calculate_volume_spike(bars1, period=3, multiplier=2.0)
    assert points1[3].quality == SignalQuality.ZERO_BASELINE
    assert points1[3].value is None
    assert points1[3].baseline == 0.0
    assert points1[3].relative_volume is None

    # Case 2: [100, 100, 100, 0]
    bars2 = _make_bars([100, 100, 100, 0])
    points2 = calculate_volume_spike(bars2, period=3, multiplier=2.0)
    assert points2[3].quality == SignalQuality.VALID
    assert points2[3].baseline == 100.0
    assert points2[3].relative_volume == 0.0
    assert points2[3].value is False
    assert "NORMAL_VOLUME" in points2[3].reasons


def test_p1_or_04_invalid_volume_handling():
    """P1-OR-04: [100, null, 300, 400] / negative or infinite volume -> null / INVALID_VOLUME."""
    # Subcase A: None in window
    bars_none = _make_bars([100, None, 300, 400])
    points_none = calculate_volume_spike(bars_none, period=3, multiplier=2.0)
    assert points_none[3].quality == SignalQuality.INVALID_VOLUME
    assert points_none[3].value is None

    # Subcase B: negative in window
    bars_neg = _make_bars([100, -50, 300, 400])
    points_neg = calculate_volume_spike(bars_neg, period=3, multiplier=2.0)
    assert points_neg[3].quality == SignalQuality.INVALID_VOLUME
    assert points_neg[3].value is None

    # Subcase C: current volume is infinite
    bars_inf = _make_bars([100, 200, 300, float("inf")])
    points_inf = calculate_volume_spike(bars_inf, period=3, multiplier=2.0)
    assert points_inf[3].quality == SignalQuality.INVALID_VOLUME
    assert points_inf[3].value is None

    # Subcase D: current volume is NaN
    bars_nan = _make_bars([100, 200, 300, float("nan")])
    points_nan = calculate_volume_spike(bars_nan, period=3, multiplier=2.0)
    assert points_nan[3].quality == SignalQuality.INVALID_VOLUME
    assert points_nan[3].value is None


def test_p1_or_05_prefix_invariance_and_no_future_leak():
    """P1-OR-05: Every prefix; append/mutate future bars.
    
    Available values, quality, and reasons match full-series prefix.
    """
    full_volumes = [100, 150, 200, 250, 300, 500, 700, 200, 100, 900]
    full_bars = _make_bars(full_volumes)
    full_points = calculate_volume_spike(full_bars, period=3, multiplier=2.0)

    # Test every prefix from length 1 to 10
    for prefix_len in range(1, len(full_volumes) + 1):
        prefix_bars = _make_bars(full_volumes[:prefix_len])
        prefix_points = calculate_volume_spike(prefix_bars, period=3, multiplier=2.0)

        assert len(prefix_points) == prefix_len
        for k in range(prefix_len):
            assert prefix_points[k].value == full_points[k].value
            assert prefix_points[k].quality == full_points[k].quality
            assert prefix_points[k].baseline == full_points[k].baseline
            assert prefix_points[k].relative_volume == full_points[k].relative_volume
            assert prefix_points[k].reasons == full_points[k].reasons

    # Mutate future bar and verify earlier results do not change
    mutated_volumes = list(full_volumes)
    mutated_volumes[-1] = 9999999
    mutated_bars = _make_bars(mutated_volumes)
    mutated_points = calculate_volume_spike(mutated_bars, period=3, multiplier=2.0)

    for k in range(len(full_volumes) - 1):
        assert mutated_points[k].value == full_points[k].value
        assert mutated_points[k].baseline == full_points[k].baseline


def test_p1_or_06_period_20_warmup_boundary():
    """P1-OR-06: Period 20 with 20 total bars, then bar 21.
    
    First 20 bars unavailable; first calculable result is zero-based index 20.
    """
    vols_20 = [100 + i * 10 for i in range(20)]
    bars_20 = _make_bars(vols_20)
    points_20 = calculate_volume_spike(bars_20, period=20, multiplier=2.0)

    assert len(points_20) == 20
    for p in points_20:
        assert p.quality == SignalQuality.INSUFFICIENT_HISTORY
        assert p.value is None

    # Now add bar 21 (index 20)
    bars_21 = _make_bars(vols_20 + [5000])
    points_21 = calculate_volume_spike(bars_21, period=20, multiplier=2.0)

    assert len(points_21) == 21
    # First 20 bars remain unavailable
    for t in range(20):
        assert points_21[t].quality == SignalQuality.INSUFFICIENT_HISTORY
        assert points_21[t].value is None

    # Bar 21 (index 20) is first valid calculable bar
    assert points_21[20].quality == SignalQuality.VALID
    expected_baseline = sum(vols_20) / 20.0
    assert points_21[20].baseline == expected_baseline
    assert points_21[20].relative_volume == 5000 / expected_baseline
    assert points_21[20].value is True


def test_p1_or_11_canonical_params_hash_deterministic():
    """P1-OR-11: Registry defaults omitted vs explicit produce identical hash."""
    # Omitted defaults
    resolved_omitted = SignalRegistry.resolve_and_validate_params("volume.spike", {})
    hash_omitted = compute_canonical_params_hash("volume.spike", "1.0.0", resolved_omitted)

    # Explicit defaults
    resolved_explicit = SignalRegistry.resolve_and_validate_params("volume.spike", {"period": 20, "multiplier": 2.0})
    hash_explicit = compute_canonical_params_hash("volume.spike", "1.0.0", resolved_explicit)

    assert resolved_omitted == resolved_explicit
    assert hash_omitted == hash_explicit

    # Changing params alters hash
    resolved_diff = SignalRegistry.resolve_and_validate_params("volume.spike", {"period": 21, "multiplier": 2.0})
    hash_diff = compute_canonical_params_hash("volume.spike", "1.0.0", resolved_diff)
    assert hash_diff != hash_omitted

    # Fixed expected canonical hash golden fixture
    # Payload: {"name":"volume.spike","params":{"multiplier":2.0,"period":20},"version":"1.0.0"}
    # Verify exact SHA-256
    import hashlib, json
    fixture_json = '{"name":"volume.spike","params":{"multiplier":2.0,"period":20},"version":"1.0.0"}'
    expected_fixture_hash = hashlib.sha256(fixture_json.encode("utf-8")).hexdigest()
    assert hash_omitted == expected_fixture_hash


def test_parameter_validation_rejections():
    """Verify strict rejection of invalid parameter types and ranges."""
    # Reject bool as int
    with pytest.raises(ValueError, match="strict integer"):
        validate_volume_spike_params({"period": True})

    with pytest.raises(ValueError, match="strict integer"):
        validate_relative_volume_params({"period": False})

    # Reject bool as multiplier
    with pytest.raises(ValueError, match="finite number"):
        validate_volume_spike_params({"multiplier": True})

    # Reject unknown fields
    with pytest.raises(ValueError, match="Unknown parameter"):
        validate_volume_spike_params({"period": 20, "extra_field": 123})

    with pytest.raises(ValueError, match="Unknown parameter"):
        validate_relative_volume_params({"invalid_key": 20})

    # Reject null
    with pytest.raises(ValueError, match="cannot be null"):
        validate_volume_spike_params({"period": None})

    with pytest.raises(ValueError, match="cannot be null"):
        validate_volume_spike_params({"multiplier": None})

    # Reject NaN / Infinity
    with pytest.raises(ValueError, match="cannot be NaN or Infinity"):
        validate_volume_spike_params({"multiplier": float("nan")})

    with pytest.raises(ValueError, match="cannot be NaN or Infinity"):
        validate_volume_spike_params({"multiplier": float("inf")})

    # Reject out of range
    with pytest.raises(ValueError, match="between 1 and 252"):
        validate_volume_spike_params({"period": 0})

    with pytest.raises(ValueError, match="between 1 and 252"):
        validate_volume_spike_params({"period": 253})

    with pytest.raises(ValueError, match="> 0 and <= 100"):
        validate_volume_spike_params({"multiplier": 0})

    with pytest.raises(ValueError, match="> 0 and <= 100"):
        validate_volume_spike_params({"multiplier": 100.1})


def test_relative_volume_calculation():
    """Test Relative Volume pure calculation."""
    bars = _make_bars([100, 200, 300, 400])
    points = calculate_relative_volume(bars, period=3)

    assert len(points) == 4
    assert points[3].quality == SignalQuality.VALID
    assert points[3].baseline == 200.0
    assert points[3].relative_volume == 2.0
    assert points[3].value == 2.0
    assert points[3].output_type == "float"


def test_registry_metadata():
    """Verify SignalRegistry definition integrity."""
    defs = SignalRegistry.list_definitions()
    assert len(defs) == 72

    rvol = SignalRegistry.get_definition("volume.relative_volume")
    assert rvol is not None
    assert rvol.output_type == SignalOutputType.FLOAT
    assert rvol.ast_alias == "volume__relative_volume"

    spike = SignalRegistry.get_definition("volume.spike")
    assert spike is not None
    assert spike.output_type == SignalOutputType.BOOL
    assert spike.ast_alias == "volume__spike"

    assert SignalRegistry.resolve_name_from_alias("volume__spike") == "volume.spike"
    assert SignalRegistry.get_alias_for_name("volume.spike") == "volume__spike"


def test_registry_multiplier_metadata_and_validation_agreement():
    """Verify registry multiplier metadata expresses exclusive zero and agrees with validation at 0.001, 0, and 100."""
    spike_defn = SignalRegistry.get_definition("volume.spike")
    assert spike_defn is not None
    mult_schema = spike_defn.parameters_schema["multiplier"]

    assert mult_schema.get("exclusiveMinimum") == 0
    assert mult_schema.get("maximum") == 100
    assert "min" not in mult_schema
    assert "0.01" not in str(mult_schema)
    assert "0.1" not in str(mult_schema)

    # Validation agreement at 0.001, 0, 100
    res_small = validate_volume_spike_params({"period": 20, "multiplier": 0.001})
    assert res_small["multiplier"] == 0.001

    res_100 = validate_volume_spike_params({"period": 20, "multiplier": 100})
    assert res_100["multiplier"] == 100.0

    with pytest.raises(ValueError, match="> 0 and <= 100"):
        validate_volume_spike_params({"period": 20, "multiplier": 0})

    with pytest.raises(ValueError, match="> 0 and <= 100"):
        validate_volume_spike_params({"period": 20, "multiplier": 100.001})


def test_extreme_finite_inputs_cannot_produce_valid_or_serialize_nan_inf():
    """Extreme finite inputs cannot produce a VALID point containing or hiding non-finite derived data."""
    # Subcase 1: Prior baseline overflows float64 to inf
    bars_baseline_overflow = _make_bars([1e308, 1e308, 1e308, 100.0])
    pts_rvol = calculate_relative_volume(bars_baseline_overflow, period=3)
    p_rvol = pts_rvol[3]

    assert p_rvol.quality == SignalQuality.INVALID_VOLUME
    assert p_rvol.value is None
    assert "NON_FINITE_DERIVED_VALUE" in p_rvol.reasons
    assert p_rvol.baseline is None

    dict_rvol = p_rvol.to_dict()
    assert dict_rvol["value"] is None
    assert dict_rvol["baseline"] is None
    assert dict_rvol["quality"] == "INVALID_VOLUME"
    # Never serializes NaN or Infinity
    import json
    json_str_rvol = json.dumps(dict_rvol)
    assert "NaN" not in json_str_rvol
    assert "Infinity" not in json_str_rvol

    pts_spike = calculate_volume_spike(bars_baseline_overflow, period=3, multiplier=2.0)
    p_spike = pts_spike[3]
    assert p_spike.quality == SignalQuality.INVALID_VOLUME
    assert p_spike.value is None
    assert "NON_FINITE_DERIVED_VALUE" in p_spike.reasons
    assert p_spike.baseline is None
    dict_spike = p_spike.to_dict()
    assert dict_spike["value"] is None
    assert dict_spike["baseline"] is None
    json_str_spike = json.dumps(dict_spike)
    assert "NaN" not in json_str_spike
    assert "Infinity" not in json_str_spike

    # Subcase 2: Baseline tiny, current volume huge -> RVOL overflows float64 to inf
    bars_rvol_overflow = _make_bars([1e-308, 1e-308, 1e-308, 1e308])
    pts_rvol2 = calculate_relative_volume(bars_rvol_overflow, period=3)
    p_rvol2 = pts_rvol2[3]

    assert p_rvol2.quality == SignalQuality.INVALID_VOLUME
    assert p_rvol2.value is None
    assert "NON_FINITE_DERIVED_VALUE" in p_rvol2.reasons
    assert p_rvol2.relative_volume is None

    dict_rvol2 = p_rvol2.to_dict()
    assert dict_rvol2["value"] is None
    assert dict_rvol2["relative_volume"] is None
    json_str_rvol2 = json.dumps(dict_rvol2)
    assert "NaN" not in json_str_rvol2
    assert "Infinity" not in json_str_rvol2

    pts_spike2 = calculate_volume_spike(bars_rvol_overflow, period=3, multiplier=2.0)
    p_spike2 = pts_spike2[3]
    assert p_spike2.quality == SignalQuality.INVALID_VOLUME
    assert p_spike2.value is None
    assert "NON_FINITE_DERIVED_VALUE" in p_spike2.reasons
    assert p_spike2.relative_volume is None
    dict_spike2 = p_spike2.to_dict()
    assert dict_spike2["value"] is None
    assert dict_spike2["relative_volume"] is None
    json_str_spike2 = json.dumps(dict_spike2)
    assert "NaN" not in json_str_spike2
    assert "Infinity" not in json_str_spike2


def test_canonical_params_hash_rejects_bool_nan_inf_null_and_types():
    """Hash utility rejects bool, null, strings, containers, NaN, and Infinity; preserves golden hash."""
    # Reject boolean
    with pytest.raises(TypeError, match="boolean"):
        compute_canonical_params_hash("volume.spike", "1.0.0", {"period": True, "multiplier": 2.0})

    with pytest.raises(TypeError, match="boolean"):
        compute_canonical_params_hash("volume.spike", "1.0.0", {"period": 20, "multiplier": False})

    # Reject null
    with pytest.raises(TypeError, match="null"):
        compute_canonical_params_hash("volume.spike", "1.0.0", {"period": None, "multiplier": 2.0})

    # Reject string
    with pytest.raises(TypeError):
        compute_canonical_params_hash("volume.spike", "1.0.0", {"period": "20", "multiplier": 2.0})

    # Reject list/container
    with pytest.raises(TypeError):
        compute_canonical_params_hash("volume.spike", "1.0.0", {"period": [20], "multiplier": 2.0})

    # Reject NaN
    with pytest.raises(ValueError, match="NaN or Infinity"):
        compute_canonical_params_hash("volume.spike", "1.0.0", {"period": 20, "multiplier": float("nan")})

    # Reject Infinity
    with pytest.raises(ValueError, match="NaN or Infinity"):
        compute_canonical_params_hash("volume.spike", "1.0.0", {"period": 20, "multiplier": float("inf")})

    # Non-dict payload
    with pytest.raises(TypeError):
        compute_canonical_params_hash("volume.spike", "1.0.0", [20, 2.0])  # type: ignore


def test_quality_enum_removes_complete():
    """Verify SignalQuality enum contains exactly the 4 frozen values and COMPLETE is removed."""
    assert not hasattr(SignalQuality, "COMPLETE")
    values = [q.value for q in SignalQuality]
    assert set(values) == {"VALID", "INSUFFICIENT_HISTORY", "INVALID_VOLUME", "ZERO_BASELINE"}
    assert len(values) == 4


def test_signal_output_point_to_dict_rejects_malformed_valid_and_preserves_clean_serialization():
    """Verify SignalOutputPoint.to_dict explicitly rejects malformed VALID points instead of emitting VALID + null,
    while preserving clean valid and non-VALID null serialization.
    """
    # 1. Malformed VALID points raise ValueError
    # Float with NaN
    p_nan = SignalOutputPoint(
        bar_index=5, timestamp="2026-01-06", output_type="float",
        value=float("nan"), quality=SignalQuality.VALID, available_at_index=5,
    )
    with pytest.raises(ValueError, match="finite numeric value"):
        p_nan.to_dict()

    # Float with Infinity
    p_inf = SignalOutputPoint(
        bar_index=5, timestamp="2026-01-06", output_type="float",
        value=float("inf"), quality=SignalQuality.VALID, available_at_index=5,
    )
    with pytest.raises(ValueError, match="finite numeric value"):
        p_inf.to_dict()

    # Float with null
    p_null_float = SignalOutputPoint(
        bar_index=5, timestamp="2026-01-06", output_type="float",
        value=None, quality=SignalQuality.VALID, available_at_index=5,
    )
    with pytest.raises(ValueError, match="cannot have null value"):
        p_null_float.to_dict()

    # Float with bool value
    p_bool_as_float = SignalOutputPoint(
        bar_index=5, timestamp="2026-01-06", output_type="float",
        value=True, quality=SignalQuality.VALID, available_at_index=5,
    )
    with pytest.raises(ValueError, match="finite numeric value"):
        p_bool_as_float.to_dict()

    # Float with string value
    p_str_as_float = SignalOutputPoint(
        bar_index=5, timestamp="2026-01-06", output_type="float",
        value="2.0", quality=SignalQuality.VALID, available_at_index=5,
    )
    with pytest.raises(ValueError, match="finite numeric value"):
        p_str_as_float.to_dict()

    # Bool with null
    p_null_bool = SignalOutputPoint(
        bar_index=5, timestamp="2026-01-06", output_type="bool",
        value=None, quality=SignalQuality.VALID, available_at_index=5,
    )
    with pytest.raises(ValueError, match="cannot have null value"):
        p_null_bool.to_dict()

    # Bool with int value
    p_int_as_bool = SignalOutputPoint(
        bar_index=5, timestamp="2026-01-06", output_type="bool",
        value=1, quality=SignalQuality.VALID, available_at_index=5,
    )
    with pytest.raises(ValueError, match="must have bool value"):
        p_int_as_bool.to_dict()

    # Bool with string value
    p_str_as_bool = SignalOutputPoint(
        bar_index=5, timestamp="2026-01-06", output_type="bool",
        value="True", quality=SignalQuality.VALID, available_at_index=5,
    )
    with pytest.raises(ValueError, match="must have bool value"):
        p_str_as_bool.to_dict()

    # Float with oversized int (10**10000)
    p_huge_val = SignalOutputPoint(
        bar_index=5, timestamp="2026-01-06", output_type="float",
        value=10**10000, quality=SignalQuality.VALID, available_at_index=5,
    )
    with pytest.raises(ValueError, match="finite numeric value"):
        p_huge_val.to_dict()

    # Enum with bool value
    p_enum_bool = SignalOutputPoint(
        bar_index=5, timestamp="2026-01-06", output_type="enum",
        value=True, quality=SignalQuality.VALID, available_at_index=5,
    )
    with pytest.raises(ValueError, match="string value"):
        p_enum_bool.to_dict()

    # Enum with number value
    p_enum_num = SignalOutputPoint(
        bar_index=5, timestamp="2026-01-06", output_type="enum",
        value=42, quality=SignalQuality.VALID, available_at_index=5,
    )
    with pytest.raises(ValueError, match="string value"):
        p_enum_num.to_dict()

    # Unknown output_type
    p_unknown = SignalOutputPoint(
        bar_index=5, timestamp="2026-01-06", output_type="bogus",
        value="x", quality=SignalQuality.VALID, available_at_index=5,
    )
    with pytest.raises(ValueError, match="unknown output_type"):
        p_unknown.to_dict()

    p_unknown_nonvalid = SignalOutputPoint(
        bar_index=5, timestamp="2026-01-06", output_type="bogus",
        value=None, quality=SignalQuality.INSUFFICIENT_HISTORY, available_at_index=5,
    )
    with pytest.raises(ValueError, match="unknown output_type"):
        p_unknown_nonvalid.to_dict()

    # 2. Clean valid serialization remains unchanged
    p_valid_float = SignalOutputPoint(
        bar_index=5, timestamp="2026-01-06", output_type="float",
        value=2.5, quality=SignalQuality.VALID, baseline=100.0,
        current_volume=250.0, relative_volume=2.5, available_at_index=5,
    )
    d_f = p_valid_float.to_dict()
    assert d_f["value"] == 2.5
    assert d_f["quality"] == "VALID"
    assert d_f["baseline"] == 100.0
    assert d_f["relative_volume"] == 2.5

    # Oversized explanation fields sanitize to None
    p_huge_exp = SignalOutputPoint(
        bar_index=5, timestamp="2026-01-06", output_type="float",
        value=2.5, quality=SignalQuality.VALID, baseline=10**10000,
        current_volume=10**10000, relative_volume=10**10000, threshold=10**10000,
        available_at_index=5,
    )
    d_huge_exp = p_huge_exp.to_dict()
    assert d_huge_exp["value"] == 2.5
    assert d_huge_exp["baseline"] is None
    assert d_huge_exp["current_volume"] is None
    assert d_huge_exp["relative_volume"] is None
    assert d_huge_exp["threshold"] is None

    # Enum string succeeds
    p_enum_str = SignalOutputPoint(
        bar_index=5, timestamp="2026-01-06", output_type="enum",
        value="STRONG_BUY", quality=SignalQuality.VALID, available_at_index=5,
    )
    d_enum = p_enum_str.to_dict()
    assert d_enum["value"] == "STRONG_BUY"
    assert d_enum["output_type"] == "enum"
    assert d_enum["quality"] == "VALID"

    p_valid_bool = SignalOutputPoint(
        bar_index=5, timestamp="2026-01-06", output_type="bool",
        value=False, quality=SignalQuality.VALID, available_at_index=5,
    )
    d_b = p_valid_bool.to_dict()
    assert d_b["value"] is False
    assert d_b["quality"] == "VALID"

    # 3. Clean non-VALID null serialization remains unchanged
    p_inval = SignalOutputPoint(
        bar_index=5, timestamp="2026-01-06", output_type="float",
        value=None, quality=SignalQuality.INVALID_VOLUME, available_at_index=5,
    )
    d_inv = p_inval.to_dict()
    assert d_inv["value"] is None
    assert d_inv["quality"] == "INVALID_VOLUME"

    p_hist = SignalOutputPoint(
        bar_index=5, timestamp="2026-01-06", output_type="bool",
        value=None, quality=SignalQuality.INSUFFICIENT_HISTORY, available_at_index=5,
    )
    d_hist = p_hist.to_dict()
    assert d_hist["value"] is None
    assert d_hist["quality"] == "INSUFFICIENT_HISTORY"



