"""Regression and verification suite for REL-001 (CafeF Raw Data Import Correctness).

Covers Section 6 requirements of SUMI-REL001-REMEDIATION-001:
1. Original HPG 2024 reproduction & full 250-session recovery.
2. Benign precision mismatch normalization (4-decimal split-adjusted factor truncation).
3. Real OHLC corruption rejection (1-tick error, Low > High, Open corruption, unadjusted tick violation, index violation).
4. Exact OHLC boundary cases (Open=Low, Close=High, flat bar, 1-ULP boundary, 6-bps/25-VND boundary, sub-tick step boundary).
5. Invalid negative, zero, or non-finite (NaN/Inf) prices and volumes.
6. Unit conversion (1,000x kVND->VND for equities, 1x points for indices, volume preservation).
7. Duplicate imports (intra-file and cross-run DB duplicates).
8. Repeatability and idempotency of normalization and import workflow.
9. Already-clean CSV import (TCB 2024: 250 rows, 0 corrected, 0 rejected).
10. Partial-import warnings and explicit disclosure on preview and accept.
11. Import rollback and auditable raw OHLC provenance preservation.
12. Broader raw CafeF multi-symbol spot checks (10 Core HSX symbols in 2024 = 2,500 rows) and downstream Catalog / Chart / Replay / Backtest flow.
"""

from __future__ import annotations

import asyncio
import math
from datetime import date, datetime
from io import BytesIO
from pathlib import Path
from typing import Generator

import pandas as pd
import pytest
from sqlalchemy import create_engine
from sqlalchemy.orm import Session, sessionmaker

from app.db import Base
from app.models.candle import Candle
from app.models.import_run import ImportRun, ImportRunItem, ImportRunMutation
from app.models.symbol import Symbol
from app.schemas.replay_schema import ReplaySessionCreate
from app.services.backtest_service import BacktestService
from app.services.cafef_importer import CafeFImporter
from app.services.import_classifier import ImportClassifier
from app.services.import_workflow_service import ImportWorkflowService
from app.services.replay_service import ReplayService

REPO_ROOT = Path(__file__).resolve().parents[2]
RAW_HSX_CSV = REPO_ROOT / "docs" / "CafeF.SolieuGD.Upto05102026" / "CafeF.HSX.Upto05.10.2026.csv"
CLEAN_HSX_CSV = REPO_ROOT / "test-results" / "p0b" / "data" / "CafeF.HSX.clean.csv"


@pytest.fixture
def isolated_db(tmp_path) -> Generator[Session, None, None]:
    """Isolated temporary SQLite database session."""
    db_path = tmp_path / "test_rel001_isolated.db"
    engine = create_engine(
        f"sqlite:///{db_path}",
        connect_args={"check_same_thread": False},
    )
    Base.metadata.create_all(bind=engine)
    TestingSession = sessionmaker(autocommit=False, autoflush=False, bind=engine)
    session = TestingSession()
    try:
        yield session
    finally:
        session.close()
        engine.dispose()


@pytest.fixture(scope="module")
def raw_hsx_2024_df() -> pd.DataFrame:
    """Load 2024 subset of raw CafeF HSX archive once per module."""
    assert RAW_HSX_CSV.exists(), f"Missing source CSV: {RAW_HSX_CSV}"
    df = pd.read_csv(RAW_HSX_CSV)
    return df[(df["<DTYYYYMMDD>"] >= 20240101) & (df["<DTYYYYMMDD>"] <= 20241231)].copy()


def _df_to_cafef_bytes(df: pd.DataFrame) -> bytes:
    buf = BytesIO()
    df.to_csv(buf, index=False)
    return buf.getvalue()


class TestRel001CafeFImportCorrectness:
    """Comprehensive verification for REL-001 remediation."""

    def test_01_hpg_2024_full_session_recovery_and_clean_reference_parity(
        self, isolated_db: Session, raw_hsx_2024_df: pd.DataFrame
    ):
        """HPG 2024 raw CafeF import must recover all 250 trading sessions (22 normalized, 0 rejected, 0 missing)."""
        hpg_df = raw_hsx_2024_df[raw_hsx_2024_df["<Ticker>"] == "HPG"].copy()
        assert len(hpg_df) == 250

        csv_bytes = _df_to_cafef_bytes(hpg_df)
        preview = ImportWorkflowService.preview_import(
            db=isolated_db,
            file_content=csv_bytes,
            filename="hpg_2024_raw.csv",
            source_type="cafef",
            timeframe="1D",
            adjustment_type="unadjusted",
            confirm=True,
        )

        assert preview.can_accept is True
        assert preview.status == "previewed"
        assert preview.parsed_count == 250
        assert preview.corrected_count == 22
        assert preview.rejected_count == 0
        assert preview.duplicate_count == 0
        assert preview.conflicting_count == 0
        assert preview.missing_count == 0
        assert preview.out_of_order_count == 0
        assert preview.is_partial is False
        assert preview.partial_warning is None
        assert preview.normalization_warning is not None
        assert "22 dòng" in preview.normalization_warning

        corrected_items = [it for it in preview.items if it.is_corrected]
        assert len(corrected_items) == 22
        for it in corrected_items:
            assert it.classification == "parsed"
            assert it.reject_reason is not None
            assert it.reject_reason.startswith(CafeFImporter.NORMALIZED_PROVENANCE_PREFIX)
            assert it.raw_high is not None and it.raw_close is not None
            assert it.raw_close > it.raw_high
            assert it.high == it.close == it.raw_close
            assert it.open == it.raw_open
            assert it.low == it.raw_low

        accept_res = ImportWorkflowService.accept_import(
            db=isolated_db,
            run_id=preview.run_id,
            content_sha256=preview.content_sha256,
        )
        assert accept_res.status == "accepted"
        assert accept_res.accepted_count == 250
        assert accept_res.corrected_count == 22
        assert accept_res.rejected_count == 0
        assert accept_res.is_partial is False

        candles_1d = (
            isolated_db.query(Candle)
            .filter(Candle.symbol == "HPG", Candle.timeframe == "1D")
            .order_by(Candle.timestamp.asc())
            .all()
        )
        candles_1w = (
            isolated_db.query(Candle)
            .filter(Candle.symbol == "HPG", Candle.timeframe == "1W")
            .order_by(Candle.timestamp.asc())
            .all()
        )
        assert len(candles_1d) == 250
        assert len(candles_1w) == 53

        # Verify 100% parity with canonical clean reference dataset if present
        if CLEAN_HSX_CSV.exists():
            clean_df = pd.read_csv(CLEAN_HSX_CSV)
            clean_hpg = clean_df[
                (clean_df["<Ticker>"] == "HPG")
                & (clean_df["<DTYYYYMMDD>"] >= 20240101)
                & (clean_df["<DTYYYYMMDD>"] <= 20241231)
            ].sort_values("<DTYYYYMMDD>")
            assert len(clean_hpg) == 250
            for db_c, (_, ref_row) in zip(candles_1d, clean_hpg.iterrows()):
                assert math.isclose(db_c.open, float(ref_row["<Open>"]) * 1000.0, abs_tol=1e-4)
                assert math.isclose(db_c.high, float(ref_row["<High>"]) * 1000.0, abs_tol=1e-4)
                assert math.isclose(db_c.low, float(ref_row["<Low>"]) * 1000.0, abs_tol=1e-4)
                assert math.isclose(db_c.close, float(ref_row["<Close>"]) * 1000.0, abs_tol=1e-4)
                assert math.isclose(db_c.volume, float(ref_row["<Volume>"]), abs_tol=1e-4)

    def test_02_benign_precision_mismatch_across_price_tiers(self, isolated_db: Session):
        """Benign 4-decimal split-factor truncation on low, mid, and high-priced equities is safely normalized."""
        csv_data = (
            b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
            # HPG (~20.9 kVND, delta = 2.8 VND)
            b"HPG,20240119,20.6907,20.9298,20.6907,20.9326,28759500\n"
            # FPT (~115 kVND, delta = 10.9 VND)
            b"FPT,20240529,112.4046,115.6955,111.7295,115.7064,6154200\n"
            # VNM (~61.2 kVND, delta = 13.0 VND)
            b"VNM,20190912,59.729,61.2194,59.729,61.2324,1931290\n"
            # Close slightly below Low by 1 ULP (0.1 VND = 0.0001 kVND) on split-adjusted row
            b"SSI,20240304,26.5432,26.8123,26.3112,26.3111,12500000\n"
        )
        preview = CafeFImporter.preview(isolated_db, csv_data, "precision_tiers.csv")
        assert preview.can_accept is True
        assert preview.parsed_count == 4
        assert preview.corrected_count == 4
        assert preview.rejected_count == 0

        items_by_sym = {it.symbol: it for it in preview.items}
        assert items_by_sym["HPG"].high == 20932.6
        assert items_by_sym["HPG"].raw_high == 20929.8
        assert items_by_sym["FPT"].high == 115706.4
        assert items_by_sym["FPT"].raw_high == 115695.5
        assert items_by_sym["VNM"].high == 61232.4
        assert items_by_sym["VNM"].raw_high == 61219.4
        assert items_by_sym["SSI"].low == 26311.1
        assert items_by_sym["SSI"].raw_low == 26311.2

    def test_03_real_ohlc_corruption_strictly_rejected(self, isolated_db: Session):
        """Legitimately malformed or economically inconsistent OHLC rows must be rejected, never normalized."""
        csv_data = (
            b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
            # 1. Low > High
            b"FPT,20240102,95.0,93.0,96.0,94.0,1000000\n"
            # 2. Genuine 1-tick Close > High corruption from HSX 2024 (BTP 20240118: delta = 42.4 VND = 31.3 bps)
            b"BTP,20240118,13.3614,13.5274,13.3614,13.5698,64200\n"
            # 3. Genuine Open > High corruption from Core 10 (VNM 20190913: Open exceeds High by 54.4 VND)
            b"VNM,20190913,67.3501,67.2957,66.2194,67.1972,1500000\n"
            # 4. Unadjusted equity price (multiples of 10/100 VND) with Close > High by 10 VND
            b"VNZ,20240110,1020.0,1020.0,1020.0,1020.01,1500\n"
            # 5. Market Index (VNINDEX) with Close > High by 0.0004 points (indices are unadjusted points)
            b"VNINDEX,20240110,1150.1234,1155.1234,1148.1234,1155.1238,500000000\n"
        )
        preview = CafeFImporter.preview(isolated_db, csv_data, "corrupted_ohlc.csv")
        assert preview.can_accept is False
        assert preview.parsed_count == 0
        assert preview.corrected_count == 0
        assert preview.rejected_count == 5

    def test_04_exact_ohlc_boundary_cases(self):
        """Test exact mathematical boundaries of evaluate_and_normalize_ohlc."""
        # Case A: Open == Low and Close == High (already valid -> is_corrected=False)
        o, h, l, c, corr, reason = CafeFImporter.evaluate_and_normalize_ohlc(
            "HPG", 20690.7, 20932.6, 20690.7, 20932.6, 100000
        )
        assert corr is False
        assert reason is None
        assert (o, h, l, c) == (20690.7, 20932.6, 20690.7, 20932.6)

        # Case B: Flat bar Open == High == Low == Close (already valid -> is_corrected=False)
        o, h, l, c, corr, reason = CafeFImporter.evaluate_and_normalize_ohlc(
            "HPG", 20690.7, 20690.7, 20690.7, 20690.7, 100000
        )
        assert corr is False

        # Case C: Flat O=H=L with C exceeding H by 0.4 VND (4-decimal truncation on limit-up/single-price session)
        o, h, l, c, corr, reason = CafeFImporter.evaluate_and_normalize_ohlc(
            "HPG", 20690.7, 20690.7, 20690.7, 20691.1, 100000
        )
        assert corr is True
        assert h == 20691.1
        assert l == 20690.7

        # Case D: Relative delta just inside 6 bps (0.00059) vs just outside 6 bps (0.00061)
        # H = 50,000.0 VND (with sub-10-VND Open = 49,500.3). 6 bps of 50,000 = 30 VND, capped at 25.0 VND.
        # At H = 30,000.3 VND: 6 bps = 18.00018 VND.
        _, h_in, _, _, corr_in, _ = CafeFImporter.evaluate_and_normalize_ohlc(
            "HPG", 29500.3, 30000.3, 29500.3, 30018.0, 100000
        )
        assert corr_in is True
        assert h_in == 30018.0

        _, h_out, _, _, corr_out, _ = CafeFImporter.evaluate_and_normalize_ohlc(
            "HPG", 29500.3, 30000.3, 29500.3, 30019.0, 100000
        )
        assert corr_out is False
        assert h_out == 30000.3

        # Case E: Sub-tick step violation (delta >= 0.5 * min_step) must be rejected
        # Here H - O = 4.0 VND, and C - H = 2.5 VND (>= 0.5 * 4.0 VND)
        _, _, _, _, corr_step, _ = CafeFImporter.evaluate_and_normalize_ohlc(
            "HPG", 20926.3, 20930.3, 20690.7, 20932.8, 100000
        )
        assert corr_step is False

    def test_05_invalid_negative_zero_and_non_finite_prices(self, isolated_db: Session):
        """Negative, zero, NaN, Inf, or non-numeric prices/volumes must be rejected."""
        df = pd.DataFrame([
            {"symbol": "HPG", "timestamp": "2024-01-02", "open": -20.5, "high": 21.0, "low": 20.0, "close": 20.5, "volume": 1000},
            {"symbol": "HPG", "timestamp": "2024-01-03", "open": 0.0, "high": 21.0, "low": 0.0, "close": 20.5, "volume": 1000},
            {"symbol": "HPG", "timestamp": "2024-01-04", "open": 20.5, "high": 21.0, "low": 20.0, "close": 20.5, "volume": -50},
            {"symbol": "HPG", "timestamp": "2024-01-05", "open": float("nan"), "high": 21.0, "low": 20.0, "close": 20.5, "volume": 1000},
            {"symbol": "HPG", "timestamp": "2024-01-08", "open": 20.5, "high": float("inf"), "low": 20.0, "close": 20.5, "volume": 1000},
            {"symbol": "HPG", "timestamp": "2024-01-09", "open": "bad", "high": 21.0, "low": 20.0, "close": 20.5, "volume": 1000},
        ])
        items, counts, can_accept, _ = ImportClassifier.classify_records(df, {})
        assert can_accept is False
        assert counts["parsed"] == 0
        assert counts["corrected"] == 0
        assert counts["rejected"] == 6

    def test_06_unit_conversion_and_volume_preservation(self, isolated_db: Session):
        """Equities scale 1,000x (including normalized High/Low) while indices stay 1x and volume is untouched."""
        csv_data = (
            b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
            b"HPG,20240119,20.6907,20.9298,20.6907,20.9326,28759500\n"
            b"VNINDEX,20240119,1169.0,1182.5,1168.2,1181.5,650123400\n"
        )
        preview = CafeFImporter.preview(isolated_db, csv_data, "units.csv")
        assert preview.can_accept is True
        assert preview.parsed_count == 2
        assert preview.corrected_count == 1

        accept_res = CafeFImporter.accept(isolated_db, preview.run_id, preview.content_sha256)
        assert accept_res.accepted_count == 2

        hpg = isolated_db.query(Candle).filter_by(symbol="HPG", timeframe="1D").one()
        assert hpg.open == 20690.7
        assert hpg.high == 20932.6
        assert hpg.low == 20690.7
        assert hpg.close == 20932.6
        assert hpg.volume == 28759500.0

        vni = isolated_db.query(Candle).filter_by(symbol="VNINDEX", timeframe="1D").one()
        assert vni.open == 1169.0
        assert vni.high == 1182.5
        assert vni.low == 1168.2
        assert vni.close == 1181.5
        assert vni.volume == 650123400.0

    def test_07_duplicate_imports_and_idempotency(self, isolated_db: Session):
        """Normalized rows must be idempotent on repeat parse and detected as duplicates on repeat import."""
        csv_data = (
            b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
            b"HPG,20240119,20.6907,20.9298,20.6907,20.9326,28759500\n"
            b"HPG,20240119,20.6907,20.9298,20.6907,20.9326,28759500\n"
        )
        # 1. Intra-file duplicate of normalized row
        p1 = CafeFImporter.preview(isolated_db, csv_data, "dup_intra.csv")
        assert p1.can_accept is True
        assert p1.parsed_count == 1
        assert p1.corrected_count == 1
        assert p1.duplicate_count == 1
        assert p1.conflicting_count == 0

        res1 = CafeFImporter.accept(isolated_db, p1.run_id, p1.content_sha256)
        assert res1.accepted_count == 1

        # 2. Repeat import of the same raw CSV against DB with accepted normalized candle
        p2 = CafeFImporter.preview(isolated_db, csv_data, "dup_repeat.csv")
        assert p2.can_accept is True
        assert p2.parsed_count == 0
        assert p2.duplicate_count == 2
        assert p2.conflicting_count == 0

        res2 = CafeFImporter.accept(isolated_db, p2.run_id, p2.content_sha256)
        assert res2.status == "noop"
        assert res2.accepted_count == 0

        # 3. Direct idempotency of evaluate_and_normalize_ohlc on already normalized values
        o1, h1, l1, c1, corr1, _ = CafeFImporter.evaluate_and_normalize_ohlc(
            "HPG", 20690.7, 20929.8, 20690.7, 20932.6, 28759500
        )
        assert corr1 is True
        o2, h2, l2, c2, corr2, _ = CafeFImporter.evaluate_and_normalize_ohlc(
            "HPG", o1, h1, l1, c1, 28759500
        )
        assert corr2 is False
        assert (o2, h2, l2, c2) == (o1, h1, l1, c1)

    def test_08_already_clean_csv_untouched(self, isolated_db: Session):
        """An already-clean CafeF CSV (CafeF.HSX.clean.csv HPG 2024, 250 rows) imports with 250 parsed, 0 corrected, 0 rejected."""
        clean_df = pd.read_csv(CLEAN_HSX_CSV)
        clean_hpg_2024 = clean_df[
            (clean_df["<Ticker>"] == "HPG")
            & (clean_df["<DTYYYYMMDD>"] >= 20240101)
            & (clean_df["<DTYYYYMMDD>"] <= 20241231)
        ].copy()
        assert len(clean_hpg_2024) == 250

        csv_bytes = _df_to_cafef_bytes(clean_hpg_2024)
        preview = CafeFImporter.preview(isolated_db, csv_bytes, "hpg_2024_clean.csv")
        assert preview.can_accept is True
        assert preview.parsed_count == 250
        assert preview.corrected_count == 0
        assert preview.rejected_count == 0
        assert preview.missing_count == 0
        assert preview.is_partial is False
        assert preview.partial_warning is None
        assert preview.normalization_warning is None

    def test_09_partial_import_warning_disclosure(self, isolated_db: Session):
        """When a file has both valid/normalized rows and genuinely invalid rows, partial import warnings are surfaced."""
        csv_data = (
            b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
            b"HPG,20240118,20.5,20.8,20.4,20.6907,15000000\n"
            b"HPG,20240119,20.6907,20.9298,20.6907,20.9326,28759500\n"
            # Genuine corruption on 2024-01-22 (Monday): Low > High -> rejected, creating 1 missing weekday gap
            b"HPG,20240122,21.0,20.5,21.5,21.0,10000000\n"
            b"HPG,20240123,20.8,21.1,20.7,20.9,18000000\n"
        )
        preview = CafeFImporter.preview(isolated_db, csv_data, "partial_hpg.csv")
        assert preview.can_accept is True
        assert preview.parsed_count == 3
        assert preview.corrected_count == 1
        assert preview.rejected_count == 1
        assert preview.missing_count == 1
        assert preview.is_partial is True
        assert preview.partial_warning is not None
        assert "Partial Import" in preview.partial_warning
        assert "1 dòng không hợp lệ" in preview.partial_warning

        accept_res = CafeFImporter.accept(isolated_db, preview.run_id, preview.content_sha256)
        assert accept_res.status == "accepted"
        assert accept_res.accepted_count == 3
        assert accept_res.corrected_count == 1
        assert accept_res.rejected_count == 1
        assert accept_res.is_partial is True
        assert accept_res.partial_warning is not None
        assert "CẢNH BÁO NHẬP MỘT PHẦN" in accept_res.message

    def test_10_import_rollback_and_provenance_audit(self, isolated_db: Session):
        """Normalized rows persist raw OHLC provenance in ImportRunItem and roll back cleanly."""
        csv_data = (
            b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
            b"HPG,20240118,20.5,20.8,20.4,20.6907,15000000\n"
            b"HPG,20240119,20.6907,20.9298,20.6907,20.9326,28759500\n"
        )
        preview = CafeFImporter.preview(isolated_db, csv_data, "rollback_test.csv")
        accept_res = CafeFImporter.accept(isolated_db, preview.run_id, preview.content_sha256)
        assert accept_res.accepted_count == 2

        # Verify persisted provenance in ImportRunItem
        db_items = (
            isolated_db.query(ImportRunItem)
            .filter(ImportRunItem.run_id == preview.run_id)
            .order_by(ImportRunItem.row_index.asc())
            .all()
        )
        assert len(db_items) == 2
        assert db_items[0].reject_reason is None
        assert db_items[1].classification == "parsed"
        assert db_items[1].reject_reason is not None
        assert db_items[1].reject_reason.startswith(CafeFImporter.NORMALIZED_PROVENANCE_PREFIX)
        assert "raw_kvnd=(20.6907, 20.9298, 20.6907, 20.9326)" in db_items[1].reject_reason
        assert "raw_high=20929.8" in db_items[1].reject_reason
        assert "high=20932.6" in db_items[1].reject_reason
        assert "delta_vnd=2.8" in db_items[1].reject_reason

        # Verify ImportRunMutation records normalized after-image
        mutations = (
            isolated_db.query(ImportRunMutation)
            .filter(ImportRunMutation.run_id == preview.run_id)
            .all()
        )
        assert len(mutations) == 2

        # Roll back and verify clean restoration
        rb_res = ImportWorkflowService.rollback_import(isolated_db, preview.run_id)
        assert rb_res.status == "rolled_back"
        assert rb_res.restored_mutations_count == 2
        assert isolated_db.query(Candle).filter(Candle.symbol == "HPG").count() == 0

    def test_11_core10_symbols_2024_spot_check_and_downstream_flows(
        self, isolated_db: Session, raw_hsx_2024_df: pd.DataFrame
    ):
        """Spot-check all 10 Core HSX symbols in 2024 (2,500 rows) and verify Catalog, Market, Replay, and Backtest."""
        core_symbols = ["FPT", "HPG", "MBB", "MWG", "SSI", "TCB", "VCB", "VHM", "VIC", "VNM"]
        core_df = raw_hsx_2024_df[raw_hsx_2024_df["<Ticker>"].isin(core_symbols)].copy()
        assert len(core_df) == 2500

        csv_bytes = _df_to_cafef_bytes(core_df)
        preview = ImportWorkflowService.preview_import(
            db=isolated_db,
            file_content=csv_bytes,
            filename="core10_2024.csv",
            source_type="cafef",
            timeframe="1D",
            adjustment_type="unadjusted",
            confirm=True,
        )
        assert preview.can_accept is True
        assert preview.parsed_count == 2500
        assert preview.corrected_count == 202
        assert preview.rejected_count == 0
        assert preview.missing_count == 0
        assert preview.is_partial is False

        accept_res = ImportWorkflowService.accept_import(
            db=isolated_db,
            run_id=preview.run_id,
            content_sha256=preview.content_sha256,
        )
        assert accept_res.accepted_count == 2500
        assert accept_res.corrected_count == 202

        # 1. Data Catalog check
        catalog = ImportWorkflowService.get_catalog(isolated_db)
        hpg_cat_1d = next(c for c in catalog if c.symbol == "HPG" and c.timeframe == "1D")
        hpg_cat_1w = next(c for c in catalog if c.symbol == "HPG" and c.timeframe == "1W")
        assert hpg_cat_1d.row_count == 250
        assert hpg_cat_1d.start_date == "2024-01-02"
        assert hpg_cat_1d.end_date == "2024-12-31"
        assert hpg_cat_1w.row_count == 53

        # 2. Chart / Candle query check
        market_candles = (
            isolated_db.query(Candle)
            .filter(Candle.symbol == "HPG", Candle.timeframe == "1D")
            .order_by(Candle.timestamp.asc())
            .all()
        )
        assert len(market_candles) == 250
        jan03 = next(c for c in market_candles if c.timestamp.strftime("%Y-%m-%d") == "2024-01-03")
        assert jan03.high == 18801.0
        assert jan03.close == 18801.0

        # 3. ReplayService check (create session & step across normalized 2024-01-03 and 2024-01-16 bars)
        replay_session = ReplayService.create_session(
            isolated_db,
            ReplaySessionCreate(
                symbol="HPG",
                timeframe="1D",
                adjustment_type="unadjusted",
                start_date=datetime(2024, 1, 2),
                end_date=datetime(2024, 12, 31),
                initial_cash=100_000_000.0,
            ),
        )
        assert replay_session is not None
        serialized = ReplayService.serialize_session(isolated_db, replay_session)
        assert serialized.available_candles == 250
        stepped_session = ReplayService.next_candle(isolated_db, replay_session.id, steps=15)
        assert stepped_session.current_index == 15
        replay_candles = ReplayService.get_candles(isolated_db, replay_session.id)
        assert len(replay_candles) == 16
        assert any(c.timestamp.strftime("%Y-%m-%d") == "2024-01-03" and c.high == 18801.0 for c in replay_candles)
        assert any(c.timestamp.strftime("%Y-%m-%d") == "2024-01-16" and c.high == 18801.0 for c in replay_candles)

        # 4. BacktestService check across full 250-bar 2024 HPG series
        bt_config = {
            "symbol": "HPG",
            "timeframe": "1D",
            "adjustment_type": "unadjusted",
            "start_date": "2024-01-01",
            "end_date": "2024-12-31",
            "strategy": {
                "name": "rel001_verify_strat",
                "indicators": [],
                "entry_rules": [],
                "exit_rules": [],
                "position_sizing": {"method": "fixed_quantity", "quantity": 100},
            },
            "parameters": {},
            "benchmark_symbol": "VNINDEX",
        }
        bt_res = asyncio.run(BacktestService().run_backtest(isolated_db, bt_config))
        assert bt_res is not None
        assert bt_res["total_candles"] == 250
