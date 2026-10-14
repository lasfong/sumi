import pytest
import asyncio
from datetime import date, datetime, timezone
from fastapi import HTTPException
from sqlalchemy.orm import Session

from app.models.candle import Candle
from app.models.symbol import Symbol
from app.models.replay_session import ReplaySession
from app.domain.enums import SessionStatus, SessionMode
from app.schemas.replay_schema import ReplaySessionCreate
from app.schemas.signal_schema import SignalCalculationRequest, SignalCalculationItemRequest
from app.services.replay_service import ReplayService
from app.services.scanner_service import ScannerService
from app.services.signal_service import SignalService
from app.services.backtest_service import BacktestService
from app.services.practice_workflow_service import PracticeWorkflowService
from app.services.weekly_aggregator import WeeklyAggregator
from app.services.readiness_service import ReadinessService
from app.domain.data.adapters.sumi_candle_adapter import SumiCandleAdapter


def _seed_canonical_and_quarantine(db: Session):
    """
    Seeds a controlled dataset:
    - Symbol 'FPT':
        * Canonical 1D candles: 2026-08-03 (Mon, close 100), 2026-08-04 (Tue, close 102)
        * Quarantine 1D candles: 2026-08-05 (Wed, close 999, high 999), 2026-08-06 (Thu, close 999)
    - Symbol 'FAKE':
        * ONLY quarantine 1D candles: 2026-08-03, 2026-08-04
    - Symbol 'CANON':
        * ONLY canonical 1D candles: 2026-08-03, 2026-08-04
    - Symbol 'VNINDEX' (Benchmark):
        * Canonical 1D candle: 2026-08-03 (close 1200)
        * Quarantine 1D candle: 2026-08-04 (close 9999)
    """
    # Symbols
    for sym in ["FPT", "FAKE", "CANON", "VNINDEX"]:
        db.add(Symbol(symbol=sym, is_active=True))

    # FPT Canonical
    db.add(Candle(
        symbol="FPT", timeframe="1D", timestamp=datetime(2026, 8, 3, tzinfo=timezone.utc),
        open=98.0, high=101.0, low=97.0, close=100.0, volume=10000.0,
        source="import:run_1", adjustment_type="unadjusted"
    ))
    db.add(Candle(
        symbol="FPT", timeframe="1D", timestamp=datetime(2026, 8, 4, tzinfo=timezone.utc),
        open=100.0, high=103.0, low=99.0, close=102.0, volume=12000.0,
        source="cafef", adjustment_type="unadjusted"
    ))
    # FPT Quarantine
    db.add(Candle(
        symbol="FPT", timeframe="1D", timestamp=datetime(2026, 8, 5, tzinfo=timezone.utc),
        open=950.0, high=999.0, low=940.0, close=999.0, volume=99999.0,
        source="quarantine:sync:fake_sync_1", adjustment_type="unadjusted"
    ))
    db.add(Candle(
        symbol="FPT", timeframe="1D", timestamp=datetime(2026, 8, 6, tzinfo=timezone.utc),
        open=999.0, high=999.0, low=990.0, close=999.0, volume=99999.0,
        source="quarantine:sync:fake_sync_1", adjustment_type="unadjusted"
    ))

    # FAKE (Quarantine only)
    db.add(Candle(
        symbol="FAKE", timeframe="1D", timestamp=datetime(2026, 8, 3, tzinfo=timezone.utc),
        open=50.0, high=55.0, low=49.0, close=52.0, volume=5000.0,
        source="quarantine:sync:fake_sync_2", adjustment_type="unadjusted"
    ))
    db.add(Candle(
        symbol="FAKE", timeframe="1D", timestamp=datetime(2026, 8, 4, tzinfo=timezone.utc),
        open=52.0, high=56.0, low=51.0, close=54.0, volume=6000.0,
        source="quarantine:sync:fake_sync_2", adjustment_type="unadjusted"
    ))

    # CANON (Canonical only)
    db.add(Candle(
        symbol="CANON", timeframe="1D", timestamp=datetime(2026, 8, 3, tzinfo=timezone.utc),
        open=30.0, high=32.0, low=29.0, close=31.0, volume=3000.0,
        source="import:run_canon", adjustment_type="unadjusted"
    ))
    db.add(Candle(
        symbol="CANON", timeframe="1D", timestamp=datetime(2026, 8, 4, tzinfo=timezone.utc),
        open=31.0, high=33.0, low=30.0, close=32.0, volume=4000.0,
        source="import:run_canon", adjustment_type="unadjusted"
    ))

    # VNINDEX (Benchmark)
    db.add(Candle(
        symbol="VNINDEX", timeframe="1D", timestamp=datetime(2026, 8, 3, tzinfo=timezone.utc),
        open=1200.0, high=1210.0, low=1195.0, close=1205.0, volume=500000.0,
        source="import:run_bench", adjustment_type="unadjusted"
    ))
    db.add(Candle(
        symbol="VNINDEX", timeframe="1D", timestamp=datetime(2026, 8, 4, tzinfo=timezone.utc),
        open=9999.0, high=9999.0, low=9990.0, close=9999.0, volume=999999.0,
        source="quarantine:sync:fake_bench", adjustment_type="unadjusted"
    ))

    db.commit()


# ==============================================================================
# Q-READ-01: Replay Service & Session Candle Retrieval Excludes Quarantine
# ==============================================================================
def test_q_read_01_replay_excludes_quarantine(db_session):
    _seed_canonical_and_quarantine(db_session)

    # 1. Create session for FPT where canonical & quarantine coexist
    session = ReplayService.create_session(db_session, ReplaySessionCreate(
        symbol="FPT",
        timeframe="1D",
        adjustment_type="unadjusted",
        start_date=date(2026, 8, 1),
        end_date=date(2026, 8, 10),
        initial_cash=100_000_000.0,
        mode=SessionMode.NORMAL,
    ))

    # Advance session to reveal all available candles
    session = ReplayService.next_candle(db_session, session.id, steps=10)
    candles = ReplayService.get_candles(db_session, session.id)

    # Must only return the 2 canonical candles (2026-08-03 and 2026-08-04)
    assert len(candles) == 2, f"Expected 2 canonical candles, got {len(candles)}"
    for c in candles:
        assert not (c.source and c.source.startswith("quarantine:")), (
            f"Quarantine candle leaked into replay: source={c.source}"
        )
        assert c.close < 500.0, f"Contaminated price leaked: {c.close}"

    # 2. Case C: Symbol with ONLY quarantine candles must fail session creation as 'no data'
    with pytest.raises(HTTPException) as exc_info:
        ReplayService.create_session(db_session, ReplaySessionCreate(
            symbol="FAKE",
            timeframe="1D",
            adjustment_type="unadjusted",
            start_date=date(2026, 8, 1),
            end_date=date(2026, 8, 10),
            initial_cash=100_000_000.0,
            mode=SessionMode.NORMAL,
        ))
    assert exc_info.value.status_code == 400
    assert "No candles found" in exc_info.value.detail


# ==============================================================================
# Q-READ-02: Scanner Candle Loading Excludes Quarantine
# ==============================================================================
def test_q_read_02_scanner_excludes_quarantine(db_session):
    _seed_canonical_and_quarantine(db_session)

    scanner = ScannerService()

    # 1. Loading FPT (canonical + quarantine)
    candles = scanner._load_candles(db_session, "FPT", "2026-08-01", "2026-08-10")
    assert len(candles) == 2, f"Scanner loaded {len(candles)} candles instead of 2 canonical"
    for c in candles:
        assert not (c.source and c.source.startswith("quarantine:")), f"Leaked {c.source}"

    # 2. Loading FAKE (only quarantine)
    fake_candles = scanner._load_candles(db_session, "FAKE", "2026-08-01", "2026-08-10")
    assert len(fake_candles) == 0, f"Scanner loaded quarantine candles for FAKE: {len(fake_candles)}"


# ==============================================================================
# Q-READ-03: Signal / Indicator Cannot Consume Quarantine Candles
# ==============================================================================
def test_q_read_03_signal_and_indicator_exclude_quarantine(client, db_session):
    _seed_canonical_and_quarantine(db_session)

    # 1. Direct indicator API for FPT
    resp = client.get("/api/indicators/FPT?indicator=sma&timeframe=1D&length=2")
    assert resp.status_code == 200
    # Indicator for FAKE (quarantine only) must return 404
    resp_fake = client.get("/api/indicators/FAKE?indicator=sma&timeframe=1D&length=2")
    assert resp_fake.status_code == 404

    # 2. Signal service calculation on FPT session
    session = ReplayService.create_session(db_session, ReplaySessionCreate(
        symbol="FPT",
        timeframe="1D",
        adjustment_type="unadjusted",
        start_date=date(2026, 8, 1),
        end_date=date(2026, 8, 10),
        initial_cash=100_000_000.0,
        mode=SessionMode.NORMAL,
    ))
    ReplayService.next_candle(db_session, session.id, steps=5)

    sig_resp = SignalService.calculate_replay_signals(
        db_session,
        session.id,
        SignalCalculationRequest(
            signals=[SignalCalculationItemRequest(name="volume.spike", params={})]
        )
    )
    assert sig_resp.session_id == session.id
    # Observed index cannot exceed canonical candle count - 1
    assert sig_resp.observed_current_index <= 1


# ==============================================================================
# Q-READ-04: Backtest / Strategy Lab Excludes Quarantine Candles
# ==============================================================================
def test_q_read_04_backtest_excludes_quarantine(db_session):
    _seed_canonical_and_quarantine(db_session)

    backtest_svc = BacktestService()

    # 1. Backtest on FAKE (quarantine only) must fail with NO_CANDLES
    config_fake = {
        "symbol": "FAKE",
        "start_date": "2026-08-01",
        "end_date": "2026-08-10",
        "strategy": {
            "name": "test_strat",
            "indicators": [],
            "entry_rules": [],
            "exit_rules": [],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        },
        "parameters": {},
        "benchmark_symbol": "VNINDEX",
    }
    result_fake = asyncio.run(backtest_svc.run_backtest(db_session, config_fake))
    assert result_fake["status"] == "failed"
    assert result_fake["error_code"] == "NO_CANDLES"

    # 2. Backtest on FPT (canonical + quarantine) must only see 2 candles
    config_fpt = {
        "symbol": "FPT",
        "start_date": "2026-08-01",
        "end_date": "2026-08-10",
        "strategy": {
            "name": "test_strat",
            "indicators": [],
            "entry_rules": [],
            "exit_rules": [],
            "position_sizing": {"method": "fixed_quantity", "quantity": 100},
        },
        "parameters": {},
        "benchmark_symbol": "VNINDEX",
    }
    result_fpt = asyncio.run(backtest_svc.run_backtest(db_session, config_fpt))
    assert result_fpt["total_candles"] == 2, (
        f"Expected 2 total_candles, got {result_fpt['total_candles']}"
    )


# ==============================================================================
# Q-READ-05: Practice Workflow Does Not Consume Quarantine Candles
# ==============================================================================
def test_q_read_05_practice_workflow_excludes_quarantine(db_session):
    _seed_canonical_and_quarantine(db_session)

    session = ReplayService.create_session(db_session, ReplaySessionCreate(
        symbol="FPT",
        timeframe="1D",
        adjustment_type="unadjusted",
        start_date=date(2026, 8, 1),
        end_date=date(2026, 8, 10),
        initial_cash=100_000_000.0,
        mode=SessionMode.NORMAL,
    ))

    candles = PracticeWorkflowService._candles(db_session, session)
    assert len(candles) == 2, f"Expected 2 canonical candles, got {len(candles)}"
    for c in candles:
        assert not (c.source and c.source.startswith("quarantine:"))

    state = PracticeWorkflowService.get_snapshot(db_session, session.id)
    assert state.total_bars == 2, f"Expected total_bars == 2, got {state.total_bars}"


# ==============================================================================
# Q-READ-06: Weekly Aggregation and Readers Exclude Quarantine
# ==============================================================================
def test_q_read_06_weekly_aggregation_and_readers_exclude_quarantine(db_session):
    _seed_canonical_and_quarantine(db_session)

    # Trigger weekly aggregation for FPT and FAKE
    WeeklyAggregator.derive_weekly_candles(db_session, {"FPT", "FAKE"}, {"unadjusted"})

    # Case D: FPT has Mon, Tue canonical (high=103) and Wed, Thu quarantine (high=999)
    # The derived canonical 1W candle must have high=103, NOT 999!
    fpt_weekly = db_session.query(Candle).filter(
        Candle.symbol == "FPT",
        Candle.timeframe == "1W"
    ).first()
    assert fpt_weekly is not None
    assert fpt_weekly.source == "VN_TRADING_WEEK_V1"
    assert float(fpt_weekly.high) == 103.0, (
        f"Weekly high was contaminated by quarantine candle! Expected 103.0, got {fpt_weekly.high}"
    )

    # Case E: FAKE has only quarantine 1D candles.
    # Canonical 1W reader must NOT expose quarantine weekly data!
    fake_weekly = db_session.query(Candle).filter(
        Candle.symbol == "FAKE",
        Candle.timeframe == "1W"
    ).all()
    assert len(fake_weekly) == 0, (
        f"Quarantine 1W candle leaked to canonical query: {[c.source for c in fake_weekly]}"
    )


# ==============================================================================
# SumiCandleAdapter & ReadinessService
# ==============================================================================
def test_sumi_candle_adapter_and_readiness_exclude_quarantine(db_session):
    _seed_canonical_and_quarantine(db_session)

    # SumiCandleAdapter
    adapter = SumiCandleAdapter(db_session)
    bars = adapter.get_canonical_bars("FPT", start_date="2026-08-01", end_date="2026-08-10")
    assert len(bars) == 2, f"Expected 2 bars from adapter, got {len(bars)}"
    for b in bars:
        assert b.close < 500.0

    fake_bars = adapter.get_canonical_bars("FAKE", start_date="2026-08-01", end_date="2026-08-10")
    assert len(fake_bars) == 0

    # ReadinessService
    readiness = ReadinessService.get_readiness(db_session)
    # Total candles must only count canonical candles:
    # FPT (2) + CANON (2) + VNINDEX (1) = 5 canonical candles
    assert readiness.total_candles == 5, (
        f"Expected 5 canonical candles in readiness, got {readiness.total_candles}"
    )
    assert "FAKE" not in readiness.symbols_with_candles


# ==============================================================================
# Adversarial Checks: Cases A, B, C, D, E
# ==============================================================================
def test_adversarial_case_a_only_canonical_candles(client, db_session):
    """Case A: Only canonical candles exist. Normal behavior unchanged."""
    _seed_canonical_and_quarantine(db_session)

    # Replay on CANON works normally
    session = ReplayService.create_session(db_session, ReplaySessionCreate(
        symbol="CANON",
        timeframe="1D",
        adjustment_type="unadjusted",
        start_date=date(2026, 8, 1),
        end_date=date(2026, 8, 10),
        initial_cash=100_000_000.0,
        mode=SessionMode.NORMAL,
    ))
    session = ReplayService.next_candle(db_session, session.id, steps=5)
    candles = ReplayService.get_candles(db_session, session.id)
    assert len(candles) == 2
    assert candles[0].symbol == "CANON"

    # Indicator works normally
    resp = client.get("/api/indicators/CANON?indicator=sma&timeframe=1D&length=2")
    assert resp.status_code == 200


def test_adversarial_case_b_canonical_and_quarantine_coexist(client, db_session):
    """Case B: Canonical + quarantine candles coexist. Canonical readers see only canonical."""
    _seed_canonical_and_quarantine(db_session)

    # Scanner only sees canonical candles
    scanner_candles = ScannerService()._load_candles(db_session, "FPT", "2026-08-01", "2026-08-10")
    assert len(scanner_candles) == 2
    for c in scanner_candles:
        assert not (c.source and c.source.startswith("quarantine:"))

    # Practice workflow sees only canonical
    session = ReplayService.create_session(db_session, ReplaySessionCreate(
        symbol="FPT",
        timeframe="1D",
        adjustment_type="unadjusted",
        start_date=date(2026, 8, 1),
        end_date=date(2026, 8, 10),
        initial_cash=100_000_000.0,
        mode=SessionMode.NORMAL,
    ))
    candles = PracticeWorkflowService._candles(db_session, session)
    assert len(candles) == 2


def test_adversarial_case_c_only_quarantine_candles(client, db_session):
    """Case C: Only quarantine candles exist. Canonical readers behave as 'no canonical data', not valid history."""
    _seed_canonical_and_quarantine(db_session)

    # Replay creation rejects session as no candles found
    with pytest.raises(HTTPException) as exc_info:
        ReplayService.create_session(db_session, ReplaySessionCreate(
            symbol="FAKE",
            timeframe="1D",
            adjustment_type="unadjusted",
            start_date=date(2026, 8, 1),
            end_date=date(2026, 8, 10),
            initial_cash=100_000_000.0,
            mode=SessionMode.NORMAL,
        ))
    assert exc_info.value.status_code == 400
    assert "No candles found" in exc_info.value.detail

    # Scanner returns empty list
    scanner_candles = ScannerService()._load_candles(db_session, "FAKE", "2026-08-01", "2026-08-10")
    assert len(scanner_candles) == 0

    # Indicator API returns 404
    resp = client.get("/api/indicators/FAKE?indicator=sma&timeframe=1D&length=2")
    assert resp.status_code == 404


def test_adversarial_case_d_canonical_and_quarantine_1d_same_week(db_session):
    """Case D: Canonical 1D + quarantine 1D same week. Canonical 1W uses only canonical members."""
    _seed_canonical_and_quarantine(db_session)

    WeeklyAggregator.derive_weekly_candles(db_session, {"FPT"}, {"unadjusted"})

    weekly_candles = db_session.query(Candle).filter(
        Candle.symbol == "FPT",
        Candle.timeframe == "1W"
    ).all()
    assert len(weekly_candles) == 1
    w = weekly_candles[0]
    assert w.source == "VN_TRADING_WEEK_V1"
    # Canonical members max high is 103.0; quarantine had 999.0
    assert float(w.high) == 103.0
    # Canonical members volume sum is 10000 + 12000 = 22000; quarantine had 99999 + 99999
    assert float(w.volume) == 22000.0


def test_adversarial_case_e_only_quarantine_1d_exists(db_session):
    """Case E: Only quarantine 1D exists. Canonical 1W reader must not expose quarantine weekly data."""
    _seed_canonical_and_quarantine(db_session)

    WeeklyAggregator.derive_weekly_candles(db_session, {"FAKE"}, {"unadjusted"})

    # Canonical 1W reader sees 0 weekly candles
    canonical_1w = db_session.query(Candle).filter(
        Candle.symbol == "FAKE",
        Candle.timeframe == "1W"
    ).all()
    assert len(canonical_1w) == 0

    # Explicit quarantine reader CAN see the derived quarantine weekly candle
    quarantine_1w = db_session.query(Candle).execution_options(include_quarantine=True).filter(
        Candle.symbol == "FAKE",
        Candle.timeframe == "1W"
    ).all()
    assert len(quarantine_1w) == 1
    assert quarantine_1w[0].source == "quarantine:VN_TRADING_WEEK_V1"

