"""Automated verification tests for P6-MEASURE-01 BB behavior measurement study.

Verifies:
- Audited fixture integrity (FPT, HPG, SSI with 673 daily sessions).
- Statistical consistency: Monotonic percentiles (min <= p05 <= p25 <= p50 <= p75 <= p95 <= max).
- Smoothing decay: Lag-1 autocorrelation on T50 > T03.
- Deterministic reproducibility bit-for-bit (NFR-DET-001).
- Threshold verdict guardrail: NOT_PRODUCTION_SEMANTICS_YET (TEST-BB-002, BBI-SIG-002).
"""

import json
from pathlib import Path
import pytest

from app.domain.bb.contracts import validate_bb_threshold_semantics
from scripts.measure_bb_behavior import (
    BACKEND_ROOT,
    load_audited_fixture,
    run_bb_measurement,
)


def test_audited_fixture_integrity():
    """Verify the audited fixture contains 673 sessions for FPT, HPG, and SSI."""
    fixture_path = BACKEND_ROOT / "app" / "tests" / "fixtures" / "real_audited_sample_fpt_hpg_ssi.json"
    assert fixture_path.exists(), f"Fixture file missing: {fixture_path}"

    with open(fixture_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    assert set(data.keys()) == {"FPT", "HPG", "SSI"}
    for sym, rows in data.items():
        assert len(rows) == 673, f"Expected 673 rows for {sym}, got {len(rows)}"
        # Verify date range
        assert rows[0]["trading_date"] == "2024-01-02"
        assert rows[-1]["trading_date"] == "2026-09-18"


def test_measurement_statistical_monotonicity_and_consistency():
    """Verify statistical distributions are mathematically sound and monotonic."""
    report = run_bb_measurement()

    pooled = report["pooled"]
    horizons = ["T03", "T05", "T10", "T20", "T50", "T200"]

    for h in horizons:
        h_data = pooled[h]
        p = h_data["percentiles"]

        # Percentile monotonicity: min <= p05 <= p10 <= p25 <= p50 <= p75 <= p90 <= p95 <= max
        assert h_data["min"] <= p["p05"] <= p["p10"] <= p["p25"] <= p["p50"] <= p["p75"] <= p["p90"] <= p["p95"] <= h_data["max"]
        assert 0.0 <= h_data["min"] <= 100.0
        assert 0.0 <= h_data["max"] <= 100.0

        # Autocorrelation bounded in [-1, 1]
        autocorr = h_data["smoothness"]["autocorrelation_lag1"]
        assert -1.0 <= autocorr <= 1.0

        # Zone occupancies sum to approximately 100%
        z = h_data["zone_occupancy_pct"]
        # le_30 + corridor_30_70 + ge_70 ~ 100% (accounting for values exactly at 30.0 or 70.0)
        total_coverage = z["le_30"] + z["corridor_30_70"] + z["ge_70"]
        assert 98.0 <= total_coverage <= 102.0

    # Smoothing monotonicity: Longer horizons have higher lag-1 autocorrelation than shorter
    assert pooled["T50"]["smoothness"]["autocorrelation_lag1"] > pooled["T03"]["smoothness"]["autocorrelation_lag1"]
    assert pooled["T20"]["smoothness"]["autocorrelation_lag1"] > pooled["T03"]["smoothness"]["autocorrelation_lag1"]


def test_measurement_threshold_verdict_and_guardrail():
    """Verify threshold evaluation confirms NOT_PRODUCTION_SEMANTICS_YET and blocks hardcoding."""
    report = run_bb_measurement()
    thresh_eval = report["threshold_evaluation"]

    # 1. Formal verdict is NOT_PRODUCTION_SEMANTICS_YET
    assert thresh_eval["verdict"] == "NOT_PRODUCTION_SEMANTICS_YET"
    assert "not correspond to stable" in thresh_eval["verdict_rationale"]

    # 2. Empirical proof: On T20, time >= 70 is less than 1% (actually 0.0%)
    t20_ge_70 = thresh_eval["hypothesis_20_30_70_80"]["empirical_occupancy_T20_ge_70_pct"]
    assert t20_ge_70 < 1.0, f"Expected <1.0% above 70 on T20, got {t20_ge_70}%"

    # 3. Guardrail prevents hardcoding 20, 30, 70, 80 as production thresholds
    for thr in [20.0, 30.0, 70.0, 80.0]:
        with pytest.raises(ValueError, match="cannot be hardcoded"):
            validate_bb_threshold_semantics(thr)


def test_measurement_determinism():
    """Verify bit-for-bit deterministic output across independent runs (NFR-DET-001)."""
    run1 = run_bb_measurement()
    run2 = run_bb_measurement()

    # Convert to JSON string and compare bit-for-bit
    str1 = json.dumps(run1, sort_keys=True)
    str2 = json.dumps(run2, sort_keys=True)
    assert str1 == str2
