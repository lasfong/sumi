"""Adversarial Challenger Stress Suite for Batch B21 (Ingestion & Float Classifier).

Author: Challenger B21-2 (Ingestion & Float Classifier Challenger)
Scope:
1. Float Precision Tolerance (GAP-DB-08):
   - Direct unit testing of `are_candles_close` across micro-noise scales (1e-12, 1e-10, 1e-8) and real deviations (1e-5, 1e-4, 1e-3, 0.01).
   - Relative vs absolute tolerance behavior across equity price tiers (penny stocks ~1.0, typical ~30.0, high ~150.0, and VND-scaled ~100,000.0).
   - Verification across all candle components (open, high, low, close, volume).
   - Validation that `ImportClassifier.classify_records` accepts micro float noise (<1e-9) as duplicate/parsed while real deviations (>1e-6) trigger conflicting and block acceptance.
   - Compatibility verification with `cafef.classifier` re-export shim.
2. Holiday Gap vs Missing Day Gap:
   - Contiguous daily candle sequences spanning official exchange holidays (Hung Kings, 30/4 Reunification Day & 1/5 Labor Day, National Day 2/9, Lunar New Year / Tết).
   - Empirical verification that exchange holidays are NEVER flagged as missing trading days.
   - Verification that true omitted trading sessions ARE reliably flagged as `missing` with accurate date metadata.
   - Per-symbol isolation in missing-day detection.
   - Database-aware missing day resolution (existing candles in DB satisfy gap).
3. Weekly Aggregator Shortened Week Stress (GAP-DB-11):
   - 1-day trading week (e.g. Friday post-Tết 2023-01-27).
   - 2-day trading week with holiday at week start (e.g. Thursday-Friday post-30/4 2024-05-02..05-03).
   - 2-day trading week with holiday at week end (e.g. Monday-Tuesday pre-holiday).
   - 3-day trading week (e.g. Wednesday-Friday post-National Day 2024-09-04..09-06, and Monday-Wednesday pre-Tết 2024-02-05..02-07).
   - Midweek single-day holiday (e.g. Hung Kings 2024-04-18, 4 trading days).
   - Full 5-day holiday week (0 trading days, e.g. Tết 2019-02-04..02-08, 2022-01-31..02-04) yielding 0 weekly candles.
   - Dynamic week anchor updating (incremental additions from partial week to full week).
   - Strict holiday candle rejection (holiday candles in 1D are excluded from 1W aggregation).
   - Quarantine vs canonical provenance isolation (ADR-PROV-001).
4. Database Safety:
   - All tests execute on temporary SQLite instances (`tmp_path`).
   - Production `backend/sumi.db` is strictly untouched.
"""

from __future__ import annotations

import math
from datetime import date, datetime, timedelta
from typing import Generator
import pandas as pd
import pytest
from sqlalchemy import create_engine
from sqlalchemy.orm import Session, sessionmaker

from app.db import Base
from app.domain.market.calendar import TradingCalendar, VIETNAM_HOLIDAYS_V1
from app.models.candle import Candle
from app.models.import_run import WeeklyCandleProvenance
from app.services.cafef.classifier import (
    ClassifiedRow as CafeFClassifiedRow,
    ImportClassifier as CafeFImportClassifier,
    are_candles_close as cafef_are_candles_close,
)
from app.services.import_classifier import (
    ClassifiedRow,
    ImportClassifier,
    are_candles_close,
)
from app.services.weekly_aggregator import WeeklyAggregator


@pytest.fixture
def isolated_challenger_db(tmp_path) -> Generator[Session, None, None]:
    """Isolated SQLite database session for empirical stress testing."""
    db_file = tmp_path / "test_b21_challenger.db"
    engine = create_engine(
        f"sqlite:///{db_file}",
        connect_args={"check_same_thread": False},
    )
    Base.metadata.create_all(bind=engine)
    TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)
    session = TestingSessionLocal()
    try:
        yield session
    finally:
        session.close()
        engine.dispose()


# ===========================================================================
# 1. Float Precision Tolerance (GAP-DB-08)
# ===========================================================================


class TestFloatPrecisionTolerance:
    """Stress-test candle float comparisons and noise handling."""

    @pytest.mark.parametrize(
        "noise",
        [1e-15, 1e-13, 1e-12, 1e-11, 1e-10, 5e-10, 1e-9],
    )
    def test_are_candles_close_accepts_micro_noise(self, noise: float):
        """Micro float differences (<1e-9) must be accepted as matches across all OHLCV fields."""
        base = (30.0, 32.0, 29.0, 31.5, 150000.0)

        # Perturb each field individually by positive and negative noise
        for i in range(5):
            t_pos = list(base)
            t_pos[i] += noise
            assert are_candles_close(base, tuple(t_pos)) is True
            assert cafef_are_candles_close(base, tuple(t_pos)) is True

            t_neg = list(base)
            t_neg[i] -= noise
            assert are_candles_close(base, tuple(t_neg)) is True
            assert cafef_are_candles_close(base, tuple(t_neg)) is True

        # Perturb all fields simultaneously
        t_all = (
            base[0] + noise,
            base[1] - noise,
            base[2] + noise,
            base[3] - noise,
            base[4] + noise,
        )
        assert are_candles_close(base, t_all) is True

    @pytest.mark.parametrize(
        "deviation",
        [1e-5, 5e-5, 1e-4, 1e-3, 0.01, 0.1, 1.0],
    )
    def test_are_candles_close_rejects_real_deviations(self, deviation: float):
        """Real price deviations (>1e-6) must NOT match and return False."""
        base = (30.0, 32.0, 29.0, 31.5, 150000.0)

        # Real deviation on any price field must return False
        for i in range(4):
            t_dev = list(base)
            t_dev[i] += deviation
            assert are_candles_close(base, tuple(t_dev)) is False
            assert cafef_are_candles_close(base, tuple(t_dev)) is False

    @pytest.mark.parametrize("price_scale", [1.5, 10.0, 30.0, 100.0, 250.0])
    def test_are_candles_close_price_scale_invariance(self, price_scale: float):
        """Verify behavior across price levels (penny stock to large cap)."""
        base = (price_scale, price_scale * 1.05, price_scale * 0.95, price_scale * 1.02, 50000.0)

        # Micro noise (<1e-9) must pass
        micro_perturbed = (
            base[0] + 1e-11,
            base[1] - 1e-11,
            base[2] + 1e-11,
            base[3] - 1e-11,
            base[4],
        )
        assert are_candles_close(base, micro_perturbed) is True

        # Real deviation (1e-5) on price scale must fail
        macro_perturbed = (
            base[0] + 1e-5,
            base[1],
            base[2],
            base[3],
            base[4],
        )
        assert are_candles_close(base, macro_perturbed) is False

    def test_are_candles_close_length_mismatch(self):
        """Tuple length mismatch must safely return False without exceptions."""
        t4 = (30.0, 32.0, 29.0, 31.5)
        t5 = (30.0, 32.0, 29.0, 31.5, 100000.0)
        t6 = (30.0, 32.0, 29.0, 31.5, 100000.0, 999.0)

        assert are_candles_close(t4, t5) is False
        assert are_candles_close(t5, t6) is False
        assert are_candles_close((), t5) is False

    def test_classifier_intrafile_duplicate_with_micro_noise(self):
        """Duplicate rows within the same file with float epsilon must be accepted as duplicate, not conflict."""
        df = pd.DataFrame([
            {"symbol": "HPG", "timestamp": "2024-03-01", "open": 28.5000000000, "high": 29.2000000000, "low": 28.1000000000, "close": 29.0000000000, "volume": 500000.0},
            {"symbol": "HPG", "timestamp": "2024-03-01", "open": 28.5000000001, "high": 29.1999999999, "low": 28.1000000001, "close": 28.9999999999, "volume": 500000.0},
        ])
        items, counts, can_accept, block_reason = ImportClassifier.classify_records(df, {})

        assert can_accept is True
        assert block_reason is None
        assert counts["parsed"] == 1
        assert counts["duplicate"] == 1
        assert counts["conflicting"] == 0

    def test_classifier_intrafile_conflict_with_real_deviation(self):
        """Different rows for same date in same file exceeding tolerance must trigger conflicting and block."""
        df = pd.DataFrame([
            {"symbol": "HPG", "timestamp": "2024-03-01", "open": 28.5, "high": 29.2, "low": 28.1, "close": 29.0, "volume": 500000.0},
            {"symbol": "HPG", "timestamp": "2024-03-01", "open": 28.50005, "high": 29.2, "low": 28.1, "close": 29.0, "volume": 500000.0},
        ])
        items, counts, can_accept, block_reason = ImportClassifier.classify_records(df, {})

        assert can_accept is False
        assert counts["conflicting"] == 1
        assert "xung đột" in str(block_reason).lower()

    def test_classifier_cross_db_duplicate_with_micro_noise(self):
        """Incoming candle matching DB candle within micro float tolerance is marked duplicate and permitted."""
        existing_map = {
            ("VNM", "1D", date(2024, 6, 3), "unadjusted"): (68.5, 69.2, 68.0, 68.8, 200000.0)
        }
        df = pd.DataFrame([
            {"symbol": "VNM", "timestamp": "2024-06-03", "open": 68.5000000002, "high": 69.1999999998, "low": 68.0000000001, "close": 68.8000000001, "volume": 200000.0},
        ])
        items, counts, can_accept, block_reason = ImportClassifier.classify_records(df, existing_map)

        assert can_accept is True
        assert counts["duplicate"] == 1
        assert counts["conflicting"] == 0

    def test_classifier_cross_db_conflict_blocks_acceptance(self):
        """Incoming candle with real price deviation vs DB is marked conflicting and blocks import."""
        existing_map = {
            ("VNM", "1D", date(2024, 6, 3), "unadjusted"): (68.5, 69.2, 68.0, 68.8, 200000.0)
        }
        df = pd.DataFrame([
            {"symbol": "VNM", "timestamp": "2024-06-03", "open": 68.5, "high": 69.8, "low": 68.0, "close": 68.8, "volume": 200000.0},
        ])
        items, counts, can_accept, block_reason = ImportClassifier.classify_records(df, existing_map)

        assert can_accept is False
        assert counts["conflicting"] == 1
        assert "xung đột" in str(block_reason).lower()


# ===========================================================================
# 2. Holiday Gap vs Missing Day Gap Detection
# ===========================================================================


class TestHolidayGapVsMissingDayGap:
    """Empirically test holiday recognition vs true trading session omissions."""

    def test_reunification_and_may_day_holiday_window_no_false_missing(self):
        """Sequence spanning 2024 30/4 - 1/5 holiday window must NOT report missing trading sessions."""
        # 2024-04-26 (Fri) -> [04-27 Sat, 04-28 Sun, 04-29 Mon holiday, 04-30 Tue holiday, 05-01 Wed holiday] -> 2024-05-02 (Thu) -> 2024-05-03 (Fri)
        df = pd.DataFrame([
            {"symbol": "SSI", "timestamp": "2024-04-26", "open": 35.0, "high": 36.0, "low": 34.5, "close": 35.5, "volume": 100000},
            {"symbol": "SSI", "timestamp": "2024-05-02", "open": 35.5, "high": 36.5, "low": 35.0, "close": 36.0, "volume": 120000},
            {"symbol": "SSI", "timestamp": "2024-05-03", "open": 36.0, "high": 37.0, "low": 35.8, "close": 36.5, "volume": 130000},
        ])
        items, counts, can_accept, block_reason = ImportClassifier.classify_records(df, {})

        assert can_accept is True
        assert counts["missing"] == 0
        missing_items = [it for it in items if it.classification == "missing"]
        assert len(missing_items) == 0

    def test_reunification_window_with_true_missing_session_detected(self):
        """When Thursday 2024-05-02 is omitted from the sequence, it MUST be detected as missing."""
        # Missing 2024-05-02 (trading day), but 04-27..05-01 are weekend/holidays and must NOT be flagged
        df = pd.DataFrame([
            {"symbol": "SSI", "timestamp": "2024-04-26", "open": 35.0, "high": 36.0, "low": 34.5, "close": 35.5, "volume": 100000},
            {"symbol": "SSI", "timestamp": "2024-05-03", "open": 36.0, "high": 37.0, "low": 35.8, "close": 36.5, "volume": 130000},
        ])
        items, counts, can_accept, block_reason = ImportClassifier.classify_records(df, {})

        assert counts["missing"] == 1
        missing_items = [it for it in items if it.classification == "missing"]
        assert len(missing_items) == 1
        assert missing_items[0].timestamp == date(2024, 5, 2)
        assert "2024-05-02" in missing_items[0].reject_reason

    def test_hung_kings_midweek_holiday_no_false_missing(self):
        """Hung Kings holiday (2024-04-18 Thursday) must NOT be flagged when spanning Wed 04-17 to Fri 04-19."""
        df = pd.DataFrame([
            {"symbol": "TCB", "timestamp": "2024-04-17", "open": 45.0, "high": 46.0, "low": 44.5, "close": 45.5, "volume": 80000},
            {"symbol": "TCB", "timestamp": "2024-04-19", "open": 45.5, "high": 46.5, "low": 45.0, "close": 46.0, "volume": 85000},
        ])
        items, counts, can_accept, block_reason = ImportClassifier.classify_records(df, {})

        assert can_accept is True
        assert counts["missing"] == 0

    def test_hung_kings_span_with_missing_friday_flagged(self):
        """When Friday 2024-04-19 is omitted, only 04-19 is flagged as missing (04-18 is recognized as holiday)."""
        df = pd.DataFrame([
            {"symbol": "TCB", "timestamp": "2024-04-17", "open": 45.0, "high": 46.0, "low": 44.5, "close": 45.5, "volume": 80000},
            {"symbol": "TCB", "timestamp": "2024-04-22", "open": 46.0, "high": 47.0, "low": 45.8, "close": 46.5, "volume": 90000},
        ])
        items, counts, can_accept, block_reason = ImportClassifier.classify_records(df, {})

        assert counts["missing"] == 1
        missing_items = [it for it in items if it.classification == "missing"]
        assert len(missing_items) == 1
        assert missing_items[0].timestamp == date(2024, 4, 19)

    def test_national_day_holiday_window_no_false_missing(self):
        """National Day 2024 (2024-09-02 Mon & 2024-09-03 Tue) spanning Friday 08-30 to Wednesday 09-04."""
        df = pd.DataFrame([
            {"symbol": "FPT", "timestamp": "2024-08-30", "open": 130.0, "high": 133.0, "low": 129.0, "close": 132.0, "volume": 200000},
            {"symbol": "FPT", "timestamp": "2024-09-04", "open": 132.5, "high": 135.0, "low": 132.0, "close": 134.0, "volume": 220000},
        ])
        items, counts, can_accept, block_reason = ImportClassifier.classify_records(df, {})

        assert can_accept is True
        assert counts["missing"] == 0

    def test_national_day_with_omitted_session_flagged(self):
        """When Wednesday 2024-09-04 is omitted, only 09-04 is flagged as missing."""
        df = pd.DataFrame([
            {"symbol": "FPT", "timestamp": "2024-08-30", "open": 130.0, "high": 133.0, "low": 129.0, "close": 132.0, "volume": 200000},
            {"symbol": "FPT", "timestamp": "2024-09-05", "open": 134.0, "high": 136.0, "low": 133.0, "close": 135.5, "volume": 210000},
        ])
        items, counts, can_accept, block_reason = ImportClassifier.classify_records(df, {})

        assert counts["missing"] == 1
        missing_items = [it for it in items if it.classification == "missing"]
        assert len(missing_items) == 1
        assert missing_items[0].timestamp == date(2024, 9, 4)

    def test_tet_holiday_multi_day_window_no_false_missing(self):
        """Tết 2024 (2024-02-08 Thu to 2024-02-14 Wed) spanning 2024-02-07 to 2024-02-15."""
        df = pd.DataFrame([
            {"symbol": "MWG", "timestamp": "2024-02-07", "open": 48.0, "high": 49.0, "low": 47.5, "close": 48.5, "volume": 150000},
            {"symbol": "MWG", "timestamp": "2024-02-15", "open": 49.0, "high": 50.5, "low": 48.8, "close": 50.0, "volume": 180000},
        ])
        items, counts, can_accept, block_reason = ImportClassifier.classify_records(df, {})

        assert can_accept is True
        assert counts["missing"] == 0

    def test_multi_symbol_missing_gap_isolation(self):
        """Missing session on one symbol must not affect other clean symbols in the same import batch."""
        df = pd.DataFrame([
            # Symbol VCB is continuous
            {"symbol": "VCB", "timestamp": "2024-03-04", "open": 90.0, "high": 92.0, "low": 89.0, "close": 91.0, "volume": 1000},
            {"symbol": "VCB", "timestamp": "2024-03-05", "open": 91.0, "high": 93.0, "low": 90.0, "close": 92.0, "volume": 1200},
            # Symbol BID is missing 2024-03-05
            {"symbol": "BID", "timestamp": "2024-03-04", "open": 50.0, "high": 51.0, "low": 49.5, "close": 50.5, "volume": 2000},
            {"symbol": "BID", "timestamp": "2024-03-06", "open": 51.0, "high": 52.0, "low": 50.5, "close": 51.5, "volume": 2200},
        ])
        items, counts, can_accept, block_reason = ImportClassifier.classify_records(df, {})

        assert counts["missing"] == 1
        missing_bid = [it for it in items if it.classification == "missing" and it.symbol == "BID"]
        missing_vcb = [it for it in items if it.classification == "missing" and it.symbol == "VCB"]
        assert len(missing_bid) == 1
        assert missing_bid[0].timestamp == date(2024, 3, 5)
        assert len(missing_vcb) == 0

    def test_db_existing_candles_satisfy_missing_gap(self):
        """A day missing in the uploaded file that already exists in the database must not be flagged missing."""
        existing_map = {
            ("VIC", "1D", date(2024, 3, 5), "unadjusted"): (44.0, 45.0, 43.5, 44.5, 100000.0)
        }
        df = pd.DataFrame([
            {"symbol": "VIC", "timestamp": "2024-03-04", "open": 43.0, "high": 44.0, "low": 42.5, "close": 43.5, "volume": 90000},
            {"symbol": "VIC", "timestamp": "2024-03-06", "open": 44.5, "high": 45.5, "low": 44.0, "close": 45.0, "volume": 95000},
        ])
        items, counts, can_accept, block_reason = ImportClassifier.classify_records(df, existing_map)

        assert counts["missing"] == 0


# ===========================================================================
# 3. Weekly Aggregator Shortened Week Stress (GAP-DB-11)
# ===========================================================================


class TestWeeklyAggregatorShortenedWeekStress:
    """Stress test weekly aggregation on weeks with 1, 2, or 3 trading days due to holidays."""

    def test_1_day_trading_week_aggregation(self, isolated_challenger_db: Session):
        """Week with only 1 trading session (e.g. Friday 2023-01-27 post-Tết) forms valid 1W candle."""
        db = isolated_challenger_db

        c = Candle(
            symbol="HPG",
            timeframe="1D",
            timestamp=datetime(2023, 1, 27),  # Friday (Mon-Thu were Tết holidays)
            open=21.0,
            high=22.5,
            low=20.8,
            close=22.0,
            volume=450000.0,
            adjustment_type="unadjusted",
        )
        db.add(c)
        db.commit()

        count = WeeklyAggregator.derive_weekly_candles(db, {"HPG"}, {"unadjusted"})
        assert count == 1

        w = db.query(Candle).filter(Candle.symbol == "HPG", Candle.timeframe == "1W").first()
        assert w is not None
        # Anchored to the single traded session
        assert w.timestamp.date() == date(2023, 1, 27)
        assert w.open == 21.0
        assert w.high == 22.5
        assert w.low == 20.8
        assert w.close == 22.0
        assert w.volume == 450000.0

        prov = db.query(WeeklyCandleProvenance).filter(WeeklyCandleProvenance.symbol == "HPG").first()
        assert prov is not None
        assert prov.week_start_date == "2023-01-23"  # Monday of the week
        assert prov.daily_member_keys_json == '["2023-01-27"]'

    def test_2_day_trading_week_start_holiday(self, isolated_challenger_db: Session):
        """2-day week where Mon-Wed are holidays and Thu-Fri trade (2024-05-02..05-03)."""
        db = isolated_challenger_db

        c1 = Candle(
            symbol="VNM",
            timeframe="1D",
            timestamp=datetime(2024, 5, 2),  # Thursday
            open=65.0,
            high=67.0,
            low=64.5,
            close=66.0,
            volume=300000.0,
            adjustment_type="unadjusted",
        )
        c2 = Candle(
            symbol="VNM",
            timeframe="1D",
            timestamp=datetime(2024, 5, 3),  # Friday
            open=66.2,
            high=68.5,
            low=65.8,
            close=68.0,
            volume=350000.0,
            adjustment_type="unadjusted",
        )
        db.add_all([c1, c2])
        db.commit()

        count = WeeklyAggregator.derive_weekly_candles(db, {"VNM"}, {"unadjusted"})
        assert count == 1

        w = db.query(Candle).filter(Candle.symbol == "VNM", Candle.timeframe == "1W").first()
        assert w is not None
        # Anchored to Friday (last traded day)
        assert w.timestamp.date() == date(2024, 5, 3)
        assert w.open == 65.0  # From Thursday (first traded day)
        assert w.close == 68.0  # From Friday (last traded day)
        assert w.high == 68.5  # max(67.0, 68.5)
        assert w.low == 64.5   # min(64.5, 65.8)
        assert w.volume == 650000.0  # 300k + 350k

        prov = db.query(WeeklyCandleProvenance).filter(WeeklyCandleProvenance.symbol == "VNM").first()
        assert prov is not None
        assert prov.week_start_date == "2024-04-29"

    def test_2_day_trading_week_end_holiday(self, isolated_challenger_db: Session):
        """2-day week where Mon-Tue trade and Wed-Fri are holidays (e.g. Tết Eve)."""
        db = isolated_challenger_db

        c1 = Candle(
            symbol="FPT",
            timeframe="1D",
            timestamp=datetime(2021, 2, 8),  # Monday (traded)
            open=70.0,
            high=72.0,
            low=69.5,
            close=71.5,
            volume=500000.0,
            adjustment_type="unadjusted",
        )
        c2 = Candle(
            symbol="FPT",
            timeframe="1D",
            timestamp=datetime(2021, 2, 9),  # Tuesday (traded)
            open=71.8,
            high=74.0,
            low=71.0,
            close=73.5,
            volume=600000.0,
            adjustment_type="unadjusted",
        )
        # 2021-02-10, 11, 12 are official Tết holidays
        db.add_all([c1, c2])
        db.commit()

        count = WeeklyAggregator.derive_weekly_candles(db, {"FPT"}, {"unadjusted"})
        assert count == 1

        w = db.query(Candle).filter(Candle.symbol == "FPT", Candle.timeframe == "1W").first()
        assert w is not None
        # Anchored to Tuesday 2021-02-09 because Wed-Fri were holidays
        assert w.timestamp.date() == date(2021, 2, 9)
        assert w.open == 70.0  # Monday open
        assert w.close == 73.5  # Tuesday close
        assert w.high == 74.0
        assert w.low == 69.5
        assert w.volume == 1100000.0

    def test_3_day_trading_week_post_holiday(self, isolated_challenger_db: Session):
        """3-day week: Mon-Tue holidays (National Day 2024-09-02..03), Wed-Fri traded (2024-09-04..06)."""
        db = isolated_challenger_db

        c1 = Candle(symbol="SSI", timeframe="1D", timestamp=datetime(2024, 9, 4), open=32.0, high=33.0, low=31.8, close=32.5, volume=100000, adjustment_type="unadjusted")
        c2 = Candle(symbol="SSI", timeframe="1D", timestamp=datetime(2024, 9, 5), open=32.6, high=34.0, low=32.2, close=33.8, volume=150000, adjustment_type="unadjusted")
        c3 = Candle(symbol="SSI", timeframe="1D", timestamp=datetime(2024, 9, 6), open=33.9, high=34.5, low=33.5, close=34.2, volume=120000, adjustment_type="unadjusted")
        db.add_all([c1, c2, c3])
        db.commit()

        count = WeeklyAggregator.derive_weekly_candles(db, {"SSI"}, {"unadjusted"})
        assert count == 1

        w = db.query(Candle).filter(Candle.symbol == "SSI", Candle.timeframe == "1W").first()
        assert w is not None
        assert w.timestamp.date() == date(2024, 9, 6)  # Friday
        assert w.open == 32.0  # Wednesday open
        assert w.close == 34.2  # Friday close
        assert w.high == 34.5
        assert w.low == 31.8
        assert w.volume == 370000.0

    def test_3_day_trading_week_pre_holiday(self, isolated_challenger_db: Session):
        """3-day week: Mon-Wed traded (2024-02-05..07), Thu-Fri holidays (Tết 2024-02-08..09)."""
        db = isolated_challenger_db

        c1 = Candle(symbol="TCB", timeframe="1D", timestamp=datetime(2024, 2, 5), open=35.0, high=36.0, low=34.8, close=35.5, volume=200000, adjustment_type="unadjusted")
        c2 = Candle(symbol="TCB", timeframe="1D", timestamp=datetime(2024, 2, 6), open=35.5, high=36.5, low=35.2, close=36.2, volume=220000, adjustment_type="unadjusted")
        c3 = Candle(symbol="TCB", timeframe="1D", timestamp=datetime(2024, 2, 7), open=36.3, high=37.0, low=36.0, close=36.8, volume=250000, adjustment_type="unadjusted")
        db.add_all([c1, c2, c3])
        db.commit()

        count = WeeklyAggregator.derive_weekly_candles(db, {"TCB"}, {"unadjusted"})
        assert count == 1

        w = db.query(Candle).filter(Candle.symbol == "TCB", Candle.timeframe == "1W").first()
        assert w is not None
        # Anchored to Wednesday 2024-02-07 (last traded day before holiday)
        assert w.timestamp.date() == date(2024, 2, 7)
        assert w.open == 35.0
        assert w.close == 36.8
        assert w.high == 37.0
        assert w.low == 34.8
        assert w.volume == 670000.0

    def test_full_holiday_week_yields_zero_weekly_candles(self, isolated_challenger_db: Session):
        """A full week of official holidays (e.g. Tết 2019-02-04..08 or 2022-01-31..02-04) produces no 1W candle."""
        db = isolated_challenger_db

        # Attempt to insert candles for full holiday week (say dirty source)
        cal = TradingCalendar()
        for d in [date(2022, 1, 31), date(2022, 2, 1), date(2022, 2, 2), date(2022, 2, 3), date(2022, 2, 4)]:
            assert cal.is_trading_day(d) is False
            db.add(Candle(symbol="VIC", timeframe="1D", timestamp=datetime.combine(d, datetime.min.time()), open=100.0, high=101.0, low=99.0, close=100.5, volume=1000, adjustment_type="unadjusted"))
        db.commit()

        count = WeeklyAggregator.derive_weekly_candles(db, {"VIC"}, {"unadjusted"})
        # None of the candles are on valid trading days -> 0 weekly candles created
        assert count == 0
        w_candles = db.query(Candle).filter(Candle.symbol == "VIC", Candle.timeframe == "1W").all()
        assert len(w_candles) == 0

    def test_incremental_aggregation_updates_anchor_and_cleans_old_candle(self, isolated_challenger_db: Session):
        """Adding days progressively to an incomplete week shifts anchor and leaves exactly 1 1W candle."""
        db = isolated_challenger_db

        # Step 1: Ingest Monday & Tuesday of 2024-03-04 week
        c1 = Candle(symbol="MWG", timeframe="1D", timestamp=datetime(2024, 3, 4), open=45.0, high=46.0, low=44.0, close=45.5, volume=100000, adjustment_type="unadjusted")
        c2 = Candle(symbol="MWG", timeframe="1D", timestamp=datetime(2024, 3, 5), open=45.6, high=47.0, low=45.0, close=46.5, volume=120000, adjustment_type="unadjusted")
        db.add_all([c1, c2])
        db.commit()

        WeeklyAggregator.derive_weekly_candles(db, {"MWG"}, {"unadjusted"})
        w1 = db.query(Candle).filter(Candle.symbol == "MWG", Candle.timeframe == "1W").all()
        assert len(w1) == 1
        assert w1[0].timestamp.date() == date(2024, 3, 5)  # Anchored at Tuesday
        assert w1[0].close == 46.5
        assert w1[0].volume == 220000.0

        # Step 2: Now ingest Wednesday, Thursday, Friday
        c3 = Candle(symbol="MWG", timeframe="1D", timestamp=datetime(2024, 3, 6), open=46.5, high=47.5, low=46.0, close=47.0, volume=110000, adjustment_type="unadjusted")
        c4 = Candle(symbol="MWG", timeframe="1D", timestamp=datetime(2024, 3, 7), open=47.0, high=48.0, low=46.8, close=47.5, volume=130000, adjustment_type="unadjusted")
        c5 = Candle(symbol="MWG", timeframe="1D", timestamp=datetime(2024, 3, 8), open=47.5, high=49.0, low=47.2, close=48.8, volume=150000, adjustment_type="unadjusted")
        db.add_all([c3, c4, c5])
        db.commit()

        WeeklyAggregator.derive_weekly_candles(db, {"MWG"}, {"unadjusted"})
        w_all = db.query(Candle).filter(Candle.symbol == "MWG", Candle.timeframe == "1W").all()
        # Old Tuesday candle MUST be cleaned up, exactly ONE 1W candle exists
        assert len(w_all) == 1
        w_final = w_all[0]
        assert w_final.timestamp.date() == date(2024, 3, 8)  # Migrated to Friday
        assert w_final.open == 45.0
        assert w_final.close == 48.8
        assert w_final.high == 49.0
        assert w_final.low == 44.0
        assert w_final.volume == 610000.0

    def test_quarantine_isolation_in_weekly_aggregation(self, isolated_challenger_db: Session):
        """Quarantine 1D candles produce quarantine 1W candles and do not pollute canonical 1W candles."""
        db = isolated_challenger_db

        # Insert 1D candles marked with quarantine source
        c1 = Candle(symbol="ACB", timeframe="1D", timestamp=datetime(2024, 3, 4), open=25.0, high=26.0, low=24.5, close=25.5, volume=1000, source="quarantine:import_run_1", adjustment_type="unadjusted")
        c2 = Candle(symbol="ACB", timeframe="1D", timestamp=datetime(2024, 3, 5), open=25.5, high=26.5, low=25.0, close=26.0, volume=1200, source="quarantine:import_run_1", adjustment_type="unadjusted")
        db.add_all([c1, c2])
        db.commit()

        WeeklyAggregator.derive_weekly_candles(db, {"ACB"}, {"unadjusted"})

        w_candle = db.query(Candle).execution_options(include_quarantine=True).filter(Candle.symbol == "ACB", Candle.timeframe == "1W").first()
        assert w_candle is not None
        assert w_candle.source.startswith("quarantine:")

        prov = db.query(WeeklyCandleProvenance).filter(WeeklyCandleProvenance.symbol == "ACB").first()
        assert prov is not None
        assert prov.rule_version.startswith("quarantine:")

    def test_multi_week_consecutive_aggregation_with_shortened_weeks(self, isolated_challenger_db: Session):
        """Aggregate 3 consecutive weeks: full 5-day week, 2-day holiday week, 3-day holiday week."""
        db = isolated_challenger_db

        candles = []
        # Week 1: 2024-04-22 to 2024-04-26 (Full 5 days)
        for i in range(5):
            d = date(2024, 4, 22) + timedelta(days=i)
            candles.append(Candle(symbol="MBB", timeframe="1D", timestamp=datetime.combine(d, datetime.min.time()), open=22.0 + i * 0.2, high=23.0 + i * 0.2, low=21.8 + i * 0.2, close=22.5 + i * 0.2, volume=100000, adjustment_type="unadjusted"))

        # Week 2: 2024-04-29 to 2024-05-03 (2-day week: Mon-Wed holiday, Thu-Fri traded)
        candles.append(Candle(symbol="MBB", timeframe="1D", timestamp=datetime(2024, 5, 2), open=23.5, high=24.5, low=23.2, close=24.0, volume=150000, adjustment_type="unadjusted"))
        candles.append(Candle(symbol="MBB", timeframe="1D", timestamp=datetime(2024, 5, 3), open=24.1, high=25.0, low=23.8, close=24.8, volume=180000, adjustment_type="unadjusted"))

        # Week 3: 2024-05-06 to 2024-05-10 (Full 5 days)
        for i in range(5):
            d = date(2024, 5, 6) + timedelta(days=i)
            candles.append(Candle(symbol="MBB", timeframe="1D", timestamp=datetime.combine(d, datetime.min.time()), open=25.0 + i * 0.1, high=25.8 + i * 0.1, low=24.6 + i * 0.1, close=25.4 + i * 0.1, volume=120000, adjustment_type="unadjusted"))

        db.add_all(candles)
        db.commit()

        count = WeeklyAggregator.derive_weekly_candles(db, {"MBB"}, {"unadjusted"})
        assert count == 3

        w_candles = db.query(Candle).filter(Candle.symbol == "MBB", Candle.timeframe == "1W").order_by(Candle.timestamp.asc()).all()
        assert len(w_candles) == 3

        # Week 1: Anchored 2024-04-26 (Friday)
        assert w_candles[0].timestamp.date() == date(2024, 4, 26)
        assert w_candles[0].open == 22.0
        assert w_candles[0].volume == 500000.0

        # Week 2: Anchored 2024-05-03 (Friday) - 2 trading days
        assert w_candles[1].timestamp.date() == date(2024, 5, 3)
        assert w_candles[1].open == 23.5  # From Thursday 05-02
        assert w_candles[1].close == 24.8  # From Friday 05-03
        assert w_candles[1].high == 25.0
        assert w_candles[1].low == 23.2
        assert w_candles[1].volume == 330000.0

        # Week 3: Anchored 2024-05-10 (Friday)
        assert w_candles[2].timestamp.date() == date(2024, 5, 10)
        assert w_candles[2].open == 25.0
        assert w_candles[2].volume == 600000.0

    def test_weekly_aggregator_ignores_weekend_candles(self, isolated_challenger_db: Session):
        """Weekend candles erroneously present in 1D table must be excluded from 1W aggregation."""
        db = isolated_challenger_db

        c_fri = Candle(symbol="CTG", timeframe="1D", timestamp=datetime(2024, 3, 1), open=30.0, high=31.0, low=29.5, close=30.5, volume=100000, adjustment_type="unadjusted")
        # Erroneous Saturday candle
        c_sat = Candle(symbol="CTG", timeframe="1D", timestamp=datetime(2024, 3, 2), open=99.0, high=999.0, low=1.0, close=99.0, volume=9999999, adjustment_type="unadjusted")
        db.add_all([c_fri, c_sat])
        db.commit()

        count = WeeklyAggregator.derive_weekly_candles(db, {"CTG"}, {"unadjusted"})
        assert count == 1

        w = db.query(Candle).filter(Candle.symbol == "CTG", Candle.timeframe == "1W").first()
        assert w is not None
        assert w.timestamp.date() == date(2024, 3, 1)  # Anchored to Friday, Saturday ignored!
        assert w.high == 31.0
        assert w.low == 29.5
        assert w.volume == 100000.0

    def test_weekly_aggregator_deletes_when_all_1d_removed(self, isolated_challenger_db: Session):
        """When 1D candles are wiped, derive_weekly_candles cleans up all 1W candles and provenance."""
        db = isolated_challenger_db

        c = Candle(symbol="KDH", timeframe="1D", timestamp=datetime(2024, 3, 1), open=30.0, high=31.0, low=29.5, close=30.5, volume=100000, adjustment_type="unadjusted")
        db.add(c)
        db.commit()

        WeeklyAggregator.derive_weekly_candles(db, {"KDH"}, {"unadjusted"})
        assert db.query(Candle).filter(Candle.symbol == "KDH", Candle.timeframe == "1W").count() == 1
        assert db.query(WeeklyCandleProvenance).filter(WeeklyCandleProvenance.symbol == "KDH").count() == 1

        # Delete all 1D candles
        db.query(Candle).filter(Candle.symbol == "KDH", Candle.timeframe == "1D").delete()
        db.commit()

        WeeklyAggregator.derive_weekly_candles(db, {"KDH"}, {"unadjusted"})
        assert db.query(Candle).filter(Candle.symbol == "KDH", Candle.timeframe == "1W").count() == 0
        assert db.query(WeeklyCandleProvenance).filter(WeeklyCandleProvenance.symbol == "KDH").count() == 0


class TestFloatToleranceExactBoundaries:
    """Stress test boundary transitions and VND price scale float precision."""

    @pytest.mark.parametrize(
        "noise,expected",
        [
            (1e-12, True),
            (1e-10, True),
            (1e-8, True),
            (1e-7, True),
            (9e-7, True),       # Strictly inside 1e-6 abs_tol
            (2e-6, False),      # Strictly outside 1e-6 abs_tol
            (1e-5, False),      # Real deviation (>1e-6)
            (1e-4, False),
        ],
    )
    def test_float_noise_transition_boundary(self, noise: float, expected: bool):
        """Exact transition boundary: <1e-6 noise passes, >1e-6 deviation fails for standard ~30.0 price."""
        base = (30.0, 31.0, 29.5, 30.5, 100000.0)
        perturbed = (base[0] + noise, base[1], base[2], base[3], base[4])
        assert are_candles_close(base, perturbed) is expected

    def test_ieee754_boundary_rounding_behavior(self):
        """Demonstrate empirical IEEE-754 rounding at exactly 1e-6:
        - For 1.0 + 1e-6: diff rounds down to 9.999999999177334e-7 <= 1e-6 -> True.
        - For 30.0 + 1e-6: diff rounds up to 1.0000000010279564e-6 > 1e-6 -> False.
        This confirms why the specification defines <1e-9 as safe micro float noise
        and >1e-6 (e.g. 1e-5) as definitive conflict.
        """
        assert are_candles_close((1.0, 2.0, 0.5, 1.5, 1000.0), (1.0 + 1e-6, 2.0, 0.5, 1.5, 1000.0)) is True
        assert are_candles_close((30.0, 31.0, 29.5, 30.5, 1000.0), (30.0 + 1e-6, 31.0, 29.5, 30.5, 1000.0)) is False

    def test_vnd_scaled_price_tolerance(self):
        """VND quoted equity prices (~61,900 to 100,000 VND) relative tolerance behavior."""
        base = (61900.0, 62500.0, 61500.0, 62200.0, 1000000.0)

        # Micro noise (<1e-9 rel diff = 6.19e-5): 1e-6, 1e-5 are micro float noise on 61,900 VND
        assert are_candles_close(base, (base[0] + 1e-6, base[1], base[2], base[3], base[4])) is True
        assert are_candles_close(base, (base[0] + 1e-5, base[1], base[2], base[3], base[4])) is True

        # Real price deviations: 0.1 VND, 1.0 VND, 50.0 VND
        assert are_candles_close(base, (base[0] + 0.1, base[1], base[2], base[3], base[4])) is False
        assert are_candles_close(base, (base[0] + 1.0, base[1], base[2], base[3], base[4])) is False
        assert are_candles_close(base, (base[0] + 50.0, base[1], base[2], base[3], base[4])) is False

