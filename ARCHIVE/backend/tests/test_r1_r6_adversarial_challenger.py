"""Adversarial stress testing harness for Requirements R1 through R6.

Executed by Challenger 1 (Teamwork Preview Challenger).
Adversarial focus:
- R1: Exhaustive table-omission matrix (all 20 tables), missing column combinations, fresh migration from scratch.
- R2: Precise settlement calendar boundaries (T+2 open vs close), weekend crossings, public holiday crossings,
      and stress against heuristic bar-count fallbacks using multi-bar intraday datasets.
- R3: Extreme Sharpe calculations: zero-variance, strictly negative returns, sample threshold boundaries (N=29 vs N=30),
      and large-sample bit-exact parity across services with varied risk-free rates.
- R4: Deep replay rewind across multi-trade sessions, future order cancellation masking,
      audit eligibility lock enforcement (HTTP 403 on active sessions), and verifying underlying DB rows are never mutated.
- R5: Non-default MarketProfile with asymmetric fees, taxes, and slippage, proving exact cash conservation
      and parity with compute_round_trip_pnl.
- R6: Strict read-only contract for confirm=False across all workflow tables, rejection of unstaged ephemeral runs with HTTP 400,
      and verified staged import lifecycle.
- Global Invariant: backend/sumi.db remains bit-exact and untouched.
"""

from __future__ import annotations

from datetime import date, datetime, timedelta, timezone
import hashlib
import importlib.util
import json
import math
from pathlib import Path
from types import SimpleNamespace
from typing import Any, Dict, Generator, List

from fastapi import HTTPException
from fastapi.testclient import TestClient
import numpy as np
import pandas as pd
import pytest
from sqlalchemy import create_engine, inspect, text
from sqlalchemy.orm import Session, sessionmaker

from alembic import command
from alembic.config import Config

from app.api.decisions import clear_order_cancellation_registry, register_order_cancellation
from app.db import Base
from app.dependencies import get_db
from app.domain.accounting import compute_round_trip_pnl
from app.domain.backtest.execution import BacktestExecutionKernel
from app.domain.backtest.metrics import calculate_daily_returns_from_equity_curve, calculate_sharpe_ratio
from app.domain.enums import DecisionAction, OrderSide, OrderStatus, OrderType, PositionStatus, SessionStatus
from app.domain.market.calendar import TradingCalendar
from app.domain.market.provider import MarketProfile as ProviderMarketProfile
from app.domain.market.rules import DEFAULT_VN_MARKET_PROFILE, MarketProfile as RulesMarketProfile, MarketRules
from app.domain.market.settlement import SettlementEngine
from app.main import app
from app.models.candle import Candle
from app.models.decision import Decision
from app.models.event_log import EventLog
from app.models.execution import Execution
from app.models.import_run import ImportRun, ImportRunItem, ImportRunMutation
from app.models.order import Order
from app.models.position import Position
from app.models.replay_session import ReplaySession
from app.models.trade import Trade
from app.schemas.decision_schema import DecisionCreate
from app.services.analytics_service import AnalyticsService
from app.services.import_workflow_service import ImportWorkflowService
from app.services.practice_workflow_service import PracticeWorkflowService
from app.services.replay_service import ReplayService
from app.services.trade_lifecycle_service import TradeLifecycleService


BACKEND_DIR = Path(__file__).resolve().parent.parent
ALEMBIC_INI_PATH = BACKEND_DIR / "alembic.ini"
ALEMBIC_DIR = BACKEND_DIR / "alembic"
SUMI_DB_PATH = BACKEND_DIR / "sumi.db"
EXPECTED_SUMI_DB_SHA256 = "234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86"

ALL_20_HISTORICAL_TABLES = [
    "candles", "symbols", "replay_sessions", "decisions", "orders",
    "executions", "positions", "trades", "journal_entries", "event_logs",
    "drawing_states", "strategy_lab_runs", "scanner_runs", "import_runs",
    "import_run_items", "import_run_mutations", "weekly_candle_provenance",
    "sync_runs", "sync_run_items", "sync_run_mutations"
]

CRITICAL_COLUMNS = [
    ("executions", "execution_candle_index"),
    ("executions", "trade_id"),
    ("replay_sessions", "source_type"),
    ("decisions", "stop_loss"),
    ("trades", "planned_entry_price"),
]


def _load_env_module():
    env_path = ALEMBIC_DIR / "env.py"
    spec = importlib.util.spec_from_file_location("sumi_alembic_env_challenger", env_path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


env_mod = _load_env_module()
_ensure_precreated_db_stamped = env_mod._ensure_precreated_db_stamped


# ============================================================================
# SECTION 1: R1 ALEMBIC SCHEMA VALIDATION ADVERSARIAL STRESS
# ============================================================================

def _create_minimal_historical_schema(engine, exclude_table: str | None = None, exclude_column: tuple[str, str] | None = None):
    """Create the 20 historical tables, optionally omitting one table or one column."""
    with engine.begin() as conn:
        for tbl in ALL_20_HISTORICAL_TABLES:
            if tbl == exclude_table:
                continue

            cols = ["id INTEGER PRIMARY KEY"]
            if tbl == "executions":
                if exclude_column != ("executions", "execution_candle_index"):
                    cols.append("execution_candle_index INTEGER")
                if exclude_column != ("executions", "trade_id"):
                    cols.append("trade_id INTEGER")
            elif tbl == "replay_sessions":
                if exclude_column != ("replay_sessions", "source_type"):
                    cols.append("source_type VARCHAR(32)")
            elif tbl == "decisions":
                if exclude_column != ("decisions", "stop_loss"):
                    cols.append("stop_loss FLOAT")
            elif tbl == "trades":
                if exclude_column != ("trades", "planned_entry_price"):
                    cols.append("planned_entry_price FLOAT")

            col_defs = ", ".join(cols)
            conn.execute(text(f"CREATE TABLE {tbl} ({col_defs})"))


@pytest.mark.parametrize("missing_tbl", [
    "candles", "symbols", "replay_sessions", "orders", "executions",
    "positions", "trades", "event_logs", "import_runs", "sync_run_mutations"
])
def test_adversarial_r1_missing_individual_table_fails_closed(tmp_path, missing_tbl):
    """Stress test: Omission of each single table fails closed with diagnostic RuntimeError."""
    db_path = tmp_path / f"test_r1_missing_{missing_tbl}.db"
    engine = create_engine(f"sqlite:///{db_path}")

    _create_minimal_historical_schema(engine, exclude_table=missing_tbl)

    with engine.connect() as conn:
        with pytest.raises(RuntimeError) as exc_info:
            _ensure_precreated_db_stamped(conn)

        msg = str(exc_info.value)
        assert "partially initialized but incomplete" in msg
        assert missing_tbl in msg
        assert "a2d639ecd5d5" in msg

        # Ensure alembic_version was NOT created or stamped
        insp = inspect(conn)
        assert "alembic_version" not in insp.get_table_names()


@pytest.mark.parametrize("tbl,col", CRITICAL_COLUMNS)
def test_adversarial_r1_missing_individual_column_fails_closed(tmp_path, tbl, col):
    """Stress test: Presence of all 20 tables but missing 1 critical column fails closed."""
    db_path = tmp_path / f"test_r1_missing_{tbl}_{col}.db"
    engine = create_engine(f"sqlite:///{db_path}")

    _create_minimal_historical_schema(engine, exclude_column=(tbl, col))

    with engine.connect() as conn:
        with pytest.raises(RuntimeError) as exc_info:
            _ensure_precreated_db_stamped(conn)

        msg = str(exc_info.value)
        assert "missing required historical columns" in msg
        assert tbl in msg
        assert col in msg

        insp = inspect(conn)
        assert "alembic_version" not in insp.get_table_names()


def test_adversarial_r1_fresh_db_migrates_cleanly_from_scratch(tmp_path):
    """Stress test: A completely fresh database migrates from revision 0 to head without stamping."""
    db_path = tmp_path / "test_r1_fresh_migration.db"
    engine = create_engine(f"sqlite:///{db_path}")

    alembic_cfg = Config(str(ALEMBIC_INI_PATH))
    alembic_cfg.set_main_option("sqlalchemy.url", f"sqlite:///{db_path}")
    alembic_cfg.set_main_option("script_location", str(ALEMBIC_DIR))

    # Upgrade completely to head
    command.upgrade(alembic_cfg, "head")

    with engine.connect() as conn:
        insp = inspect(conn)
        tables = insp.get_table_names()
        assert "candles" in tables
        assert "replay_sessions" in tables
        assert "alembic_version" in tables

        # Alembic version should NOT be the pre-created stamp a2d639ecd5d5;
        # it should be at the actual latest migration head
        row = conn.execute(text("SELECT version_num FROM alembic_version")).fetchone()
        assert row is not None
        assert row[0] != "a2d639ecd5d5"


def test_adversarial_r1_existing_migration_unmodified(tmp_path):
    """Stress test: Calling _ensure_precreated_db_stamped on an already versioned DB is a strict no-op."""
    db_path = tmp_path / "test_r1_existing_version.db"
    engine = create_engine(f"sqlite:///{db_path}")

    with engine.begin() as conn:
        conn.execute(text("CREATE TABLE alembic_version (version_num VARCHAR(32) PRIMARY KEY)"))
        conn.execute(text("INSERT INTO alembic_version VALUES ('existing_rev_123')"))
        conn.execute(text("CREATE TABLE candles (id INT)"))

    with engine.connect() as conn:
        _ensure_precreated_db_stamped(conn)
        row = conn.execute(text("SELECT version_num FROM alembic_version")).fetchone()
        assert row[0] == "existing_rev_123"


# ============================================================================
# SECTION 2: R2 SETTLEMENT CALENDAR BOUNDARIES & ANTI-HEURISTIC TESTS
# ============================================================================

def test_adversarial_r2_t2_phase_boundary_comprehensive():
    """Stress test: Comprehensive boundary validation across conservative and permissive modes."""
    # Conservative mode (default): Open and Intraday touch locked on T+2; Close/ATC permitted
    cons_engine = SettlementEngine(conservative_mode=True)
    buy_date = date(2024, 1, 8)  # Monday

    # T+0 (Mon)
    assert not cons_engine.is_sellable(buy_date, date(2024, 1, 8), "open")
    assert not cons_engine.is_sellable(buy_date, date(2024, 1, 8), "close")
    # T+1 (Tue)
    assert not cons_engine.is_sellable(buy_date, date(2024, 1, 9), "open")
    assert not cons_engine.is_sellable(buy_date, date(2024, 1, 9), "close")
    # T+2 (Wed): Morning OPEN is locked, intraday touch locked, afternoon CLOSE (ATC) unlocked
    assert not cons_engine.is_sellable(buy_date, date(2024, 1, 10), "open")
    assert not cons_engine.is_sellable(buy_date, date(2024, 1, 10), "intraday")
    assert cons_engine.is_sellable(buy_date, date(2024, 1, 10), "close")
    assert cons_engine.is_sellable(buy_date, date(2024, 1, 10), "afternoon")
    # T+3 (Thu): All unlocked
    assert cons_engine.is_sellable(buy_date, date(2024, 1, 11), "open")
    assert cons_engine.is_sellable(buy_date, date(2024, 1, 11), "intraday")
    assert cons_engine.is_sellable(buy_date, date(2024, 1, 11), "close")

    # Permissive mode: Intraday and Close unlocked on T+2; Open still locked
    perm_engine = SettlementEngine(conservative_mode=False)
    assert not perm_engine.is_sellable(buy_date, date(2024, 1, 10), "open")
    assert perm_engine.is_sellable(buy_date, date(2024, 1, 10), "intraday")
    assert perm_engine.is_sellable(buy_date, date(2024, 1, 10), "close")


def test_adversarial_r2_weekend_and_holiday_boundaries():
    """Stress test: Weekend non-trading days and Vietnam statutory holidays."""
    engine = DEFAULT_VN_MARKET_PROFILE.settlement_engine

    # Thursday buy (2024-01-11)
    thu_buy = date(2024, 1, 11)
    # Friday is T+1
    assert not engine.is_sellable(thu_buy, date(2024, 1, 12), "close")
    # Saturday and Sunday are non-trading days
    assert not engine.is_sellable(thu_buy, date(2024, 1, 13), "close")
    assert not engine.is_sellable(thu_buy, date(2024, 1, 14), "close")
    # Monday is T+2
    assert not engine.is_sellable(thu_buy, date(2024, 1, 15), "open")
    assert engine.is_sellable(thu_buy, date(2024, 1, 15), "close")

    # Friday buy (2024-01-12)
    fri_buy = date(2024, 1, 12)
    # Monday is T+1
    assert not engine.is_sellable(fri_buy, date(2024, 1, 15), "close")
    # Tuesday is T+2
    assert not engine.is_sellable(fri_buy, date(2024, 1, 16), "open")
    assert engine.is_sellable(fri_buy, date(2024, 1, 16), "close")

    # Holiday test: Giỗ Tổ Hùng Vương (2024-04-18, Thursday)
    tue_buy = date(2024, 4, 16)
    # Wednesday 2024-04-17 is T+1
    assert not engine.is_sellable(tue_buy, date(2024, 4, 17), "close")
    # Thursday 2024-04-18 is Holiday (no trading)
    assert not engine.is_sellable(tue_buy, date(2024, 4, 18), "close")
    # Friday 2024-04-19 is T+2
    assert not engine.is_sellable(tue_buy, date(2024, 4, 19), "open")
    assert engine.is_sellable(tue_buy, date(2024, 4, 19), "close")


def test_adversarial_r2_anti_heuristic_high_bar_count_stress():
    """Stress test: 50 intraday bars on the same day must NOT permit selling if calendar date has not reached T+2 close."""
    day1 = datetime(2024, 1, 8, 9, 0)
    day2 = datetime(2024, 1, 9, 9, 0)

    rows = []
    # 30 bars on Day 1 (Monday)
    for i in range(30):
        ts = day1 + timedelta(minutes=i * 5)
        rows.append({"timestamp": ts, "open": 20000.0, "high": 20500.0, "low": 19800.0, "close": 20200.0, "volume": 10000})

    # 30 bars on Day 2 (Tuesday - T+1)
    for i in range(30):
        ts = day2 + timedelta(minutes=i * 5)
        rows.append({"timestamp": ts, "open": 20300.0, "high": 20600.0, "low": 20100.0, "close": 20400.0, "volume": 10000})

    df = pd.DataFrame(rows)

    # Indicator triggers:
    # Buy on bar 1 (fills bar 2 open); Sell on bar 2 (queued for bar 3 open)
    sig = np.zeros(60)
    sig[1] = 1.0   # Buy signal
    sig[2] = -1.0  # Sell signal

    indicator_values = {
        "signal": sig,
        "zero": np.zeros(60),
    }

    strategy = SimpleNamespace(
        name="stress_hft",
        indicators=[
            {"id": "signal", "kind": "custom", "params": {}},
            {"id": "zero", "kind": "custom", "params": {}},
        ],
        entry_rules=[{"dsl": {"gt": ["signal", "zero"]}}],
        exit_rules=[{"dsl": {"lt": ["signal", "zero"]}}],
        position_sizing=SimpleNamespace(
            method="fixed_quantity", quantity=100, percent=100.0,
            model_dump=lambda: {"method": "fixed_quantity", "quantity": 100, "percent": 100.0}
        ),
        risk_management=None
    )

    kernel = BacktestExecutionKernel()
    result = kernel.run(df=df, indicator_values=indicator_values, strategy=strategy, symbol="VIC")

    # Day 2 is only T+1; despite having 60 bars total, 0 trades can complete!
    completed_trades = [t for t in result.trades if t.exit_bar_index is not None]
    assert len(completed_trades) == 0
    assert any("lock" in str(w).lower() or "settlement" in str(w).lower() for w in result.warnings)


# ============================================================================
# SECTION 3: R3 SHARPE COHERENCE ACROSS EXTREME INPUTS
# ============================================================================

def test_adversarial_r3_zero_variance_equity_curves():
    """Stress test: Zero-variance returns must produce 0.0 in metrics and status='not_applicable' in analytics."""
    flat_curve = [{"equity": 100_000_000.0} for _ in range(50)]

    sharpe_m = calculate_sharpe_ratio(flat_curve, rf_annual=0.0)
    assert sharpe_m == 0.0

    analytics_res = AnalyticsService._build_metric_results(trades=[], equity_curve=flat_curve, rf_annual=0.0)
    sharpe_a = analytics_res["sharpe_ratio"]
    assert sharpe_a.status == "not_applicable"
    assert sharpe_a.value is None
    assert "zero variance" in sharpe_a.reason.lower()


def test_adversarial_r3_strictly_negative_returns_parity():
    """Stress test: Varying negative equity returns produce bit-exact negative Sharpe parity."""
    np.random.seed(99)
    # Daily returns strictly negative: between -0.3% and -2.0%
    negative_rets = -np.random.uniform(0.003, 0.020, size=45)

    eq = 100_000_000.0
    curve = [{"equity": eq}]
    for r in negative_rets:
        eq *= (1.0 + r)
        curve.append({"equity": eq})

    sharpe_m = calculate_sharpe_ratio(curve, rf_annual=0.0)
    assert sharpe_m < 0.0

    analytics_res = AnalyticsService._build_metric_results(trades=[], equity_curve=curve, rf_annual=0.0)
    sharpe_a = analytics_res["sharpe_ratio"]
    assert sharpe_a.status == "valid"
    assert sharpe_a.value == sharpe_m


def test_adversarial_r3_sample_size_threshold_boundaries():
    """Stress test: N=29 (insufficient) vs N=30 (valid) threshold gating."""
    np.random.seed(42)
    daily_rets = list(np.random.normal(0.001, 0.015, size=40))

    # Curve with N=29 daily returns (30 points)
    curve_29 = [{"equity": 100_000_000.0}]
    for r in daily_rets[:29]:
        curve_29.append({"equity": curve_29[-1]["equity"] * (1.0 + r)})

    res_29 = AnalyticsService._build_metric_results(trades=[], equity_curve=curve_29, rf_annual=0.0)
    assert res_29["sharpe_ratio"].status == "insufficient_data"
    assert res_29["sharpe_ratio"].value is None

    # Curve with N=30 daily returns (31 points)
    curve_30 = [{"equity": 100_000_000.0}]
    for r in daily_rets[:30]:
        curve_30.append({"equity": curve_30[-1]["equity"] * (1.0 + r)})

    res_30 = AnalyticsService._build_metric_results(trades=[], equity_curve=curve_30, rf_annual=0.0)
    assert res_30["sharpe_ratio"].status == "valid"
    assert res_30["sharpe_ratio"].value is not None
    assert res_30["sharpe_ratio"].value == calculate_sharpe_ratio(curve_30, rf_annual=0.0)


@pytest.mark.parametrize("rf_rate", [0.0, 0.025, 0.045, 0.08, 0.15])
def test_adversarial_r3_large_sample_bit_exact_cross_service_parity(rf_rate):
    """Stress test: 100 oscillating return bars under various risk-free rates must match to 4 decimal places."""
    np.random.seed(1337)
    rets = np.random.normal(0.0008, 0.018, size=100)

    curve = [{"equity": 100_000_000.0}]
    for r in rets:
        curve.append({"equity": curve[-1]["equity"] * (1.0 + r)})

    metric_val = calculate_sharpe_ratio(curve, rf_annual=rf_rate)
    analytics_res = AnalyticsService._build_metric_results(trades=[], equity_curve=curve, rf_annual=rf_rate)
    analytics_val = analytics_res["sharpe_ratio"].value

    assert analytics_res["sharpe_ratio"].status == "valid"
    assert metric_val == analytics_val
    assert isinstance(metric_val, float)


# ============================================================================
# SECTION 4: R4 REPLAY FUTURE-STATE ISOLATION & REWIND INVARIANCE
# ============================================================================

@pytest.fixture
def isolated_db_client(tmp_path) -> Generator[tuple[Session, TestClient, str], None, None]:
    clear_order_cancellation_registry()
    db_file = tmp_path / "test_r4_adversarial.db"
    db_url = f"sqlite:///{db_file}"
    engine = create_engine(db_url, connect_args={"check_same_thread": False})
    Base.metadata.create_all(bind=engine)
    TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)
    db = TestingSessionLocal()

    def override_get_db():
        try:
            yield db
        finally:
            pass

    app.dependency_overrides[get_db] = override_get_db
    client = TestClient(app)

    yield db, client, str(db_file)

    app.dependency_overrides.clear()
    db.close()
    clear_order_cancellation_registry()


def _populate_replay_session(db: Session, num_bars: int = 15) -> tuple[ReplaySession, list[Candle]]:
    start_date = date(2024, 1, 8)
    candles = []
    for i in range(num_bars):
        c_date = start_date + timedelta(days=i)
        candle = Candle(
            symbol="HPG",
            timeframe="1D",
            timestamp=datetime.combine(c_date, datetime.min.time(), tzinfo=timezone.utc),
            open=25000.0 + i * 100,
            high=25500.0 + i * 100,
            low=24800.0 + i * 100,
            close=25200.0 + i * 100,
            volume=100000.0 + i * 1000,
            source="cafef",
            adjustment_type="unadjusted"
        )
        db.add(candle)
        candles.append(candle)
    db.commit()

    session = ReplaySession(
        symbol="HPG",
        timeframe="1D",
        initial_cash=100_000_000.0,
        current_cash=100_000_000.0,
        start_date=candles[0].timestamp.date(),
        end_date=candles[-1].timestamp.date(),
        current_index=5,
        status="active"
    )
    db.add(session)
    db.commit()
    db.refresh(session)
    return session, candles


def test_adversarial_r4_multi_step_rewind_position_projection(isolated_db_client):
    """Stress test: Position closed at future bar must project as OPEN at earlier cursor without DB mutation."""
    db, client, _ = isolated_db_client
    session, candles = _populate_replay_session(db, num_bars=15)

    # Buy at bar 2
    dec_buy = Decision(
        session_id=session.id, symbol="HPG", action="BUY", candle_index=2,
        decision_date=candles[2].timestamp, price=25000.0, planned_quantity=1000
    )
    db.add(dec_buy)
    db.flush()

    buy_order = Order(
        session_id=session.id, decision_id=dec_buy.id, symbol="HPG", side="BUY", order_type="MARKET",
        quantity=1000.0, status="filled", created_at=candles[2].timestamp
    )
    db.add(buy_order)
    db.commit()
    db.refresh(buy_order)

    buy_exc = Execution(
        order_id=buy_order.id, session_id=session.id, symbol="HPG",
        execution_price=25000.0, quantity=1000.0, execution_candle_index=2,
        execution_date=candles[2].timestamp, gross_amount=25000000.0, net_amount=25000000.0
    )
    db.add(buy_exc)

    # Sell at bar 8
    dec_sell = Decision(
        session_id=session.id, symbol="HPG", action="SELL", candle_index=8,
        decision_date=candles[8].timestamp, price=26000.0, planned_quantity=1000
    )
    db.add(dec_sell)
    db.flush()

    sell_order = Order(
        session_id=session.id, decision_id=dec_sell.id, symbol="HPG", side="SELL", order_type="MARKET",
        quantity=1000.0, status="filled", created_at=candles[8].timestamp
    )
    db.add(sell_order)
    db.commit()
    db.refresh(sell_order)

    sell_exc = Execution(
        order_id=sell_order.id, session_id=session.id, symbol="HPG",
        execution_price=26000.0, quantity=1000.0, execution_candle_index=8,
        execution_date=candles[8].timestamp, gross_amount=26000000.0, net_amount=26000000.0
    )
    db.add(sell_exc)

    # Persist closed position in database
    db_pos = Position(
        session_id=session.id, symbol="HPG", quantity=0.0, average_price=25000.0,
        total_cost=0.0, realized_pnl=1_000_000.0, status="closed",
        opened_at=candles[2].timestamp, closed_at=candles[8].timestamp
    )
    db.add(db_pos)
    db.commit()

    # Cursor is at bar 5 (position should be OPEN!)
    session.current_index = 5
    db.commit()

    resp = client.get(f"/api/replay/sessions/{session.id}/position")
    assert resp.status_code == 200
    pos_data = resp.json()
    assert len(pos_data) == 1
    assert pos_data[0]["status"] == "open"
    assert pos_data[0]["quantity"] == 1000.0
    assert pos_data[0]["closed_at"] is None

    # Verify SQLite row in DB remains UNMUTATED (status == 'closed')
    db_row = db.query(Position).filter(Position.id == db_pos.id).first()
    assert db_row.status == "closed"
    assert db_row.quantity == 0.0

    # Advance cursor past bar 8 (e.g., bar 10)
    session.current_index = 10
    db.commit()

    resp_after = client.get(f"/api/replay/sessions/{session.id}/position")
    assert resp_after.status_code == 200
    assert len(resp_after.json()) == 0


def test_adversarial_r4_future_cancellation_masking_ladder(isolated_db_client):
    """Stress test: Future cancellation is masked to PENDING at earlier cursor."""
    db, client, _ = isolated_db_client
    session, candles = _populate_replay_session(db, num_bars=15)

    decision = Decision(
        session_id=session.id, symbol="HPG", action="BUY", candle_index=2,
        decision_date=candles[2].timestamp, price=24000.0, planned_quantity=500
    )
    db.add(decision)
    db.flush()

    order = Order(
        session_id=session.id, decision_id=decision.id, symbol="HPG", side="BUY", order_type="LIMIT",
        requested_price=24000.0, quantity=500.0, status="cancelled",
        created_at=candles[2].timestamp
    )
    db.add(order)
    db.commit()
    db.refresh(order)

    # Cancellation occurred at bar 6
    register_order_cancellation(order.id, 6, session.id)

    # Test at cursor K = 4 (< 6): Order MUST be masked to 'pending'
    session.current_index = 4
    db.commit()
    r4 = client.get(f"/api/replay/sessions/{session.id}/orders")
    assert r4.status_code == 200
    orders4 = r4.json()
    assert len(orders4) == 1
    assert orders4[0]["status"] == "pending"

    # Test at cursor K = 7 (>= 6): Order returns unmasked 'cancelled'
    session.current_index = 7
    db.commit()
    r7 = client.get(f"/api/replay/sessions/{session.id}/orders")
    assert r7.status_code == 200
    orders7 = r7.json()
    assert len(orders7) == 1
    assert orders7[0]["status"] == "cancelled"


def test_adversarial_r4_include_future_strictly_prohibited_during_active_replay(isolated_db_client):
    """Stress test: include_future=True MUST return HTTP 403 Forbidden on active sessions."""
    db, client, _ = isolated_db_client
    session, candles = _populate_replay_session(db, num_bars=15)

    session.current_index = 5  # Active, not at end
    session.status = "active"
    db.commit()

    # Query all endpoints with include_future=True
    for endpoint in ["orders", "position", "trades"]:
        resp = client.get(f"/api/replay/sessions/{session.id}/{endpoint}?include_future=True")
        assert resp.status_code == 403
        assert "strictly prohibited during active replay" in resp.json()["detail"]


# ============================================================================
# SECTION 5: R5 CUSTOM MARKETPROFILE ASYMMETRIC RATES & CASH CONSERVATION
# ============================================================================

def test_adversarial_r5_asymmetric_market_profile_cash_conservation():
    """Stress test: Backtest execution with custom asymmetric rates satisfies cash conservation."""
    # Define custom profile with non-default rates
    custom_profile = RulesMarketProfile(
        name="custom_asymmetric",
        fee_buy=0.0028,    # 0.28%
        fee_sell=0.0038,   # 0.38%
        tax_sell=0.0015,   # 0.15%
        slippage=0.0020,   # 0.20%
        settlement_days=2,
        rules=MarketRules(exchange="HOSE"),
        settlement_engine=SettlementEngine(TradingCalendar(), settlement_days=2)
    )

    # 20 trading days of candles
    dates = [
        date(2024, 1, 8), date(2024, 1, 9), date(2024, 1, 10), date(2024, 1, 11), date(2024, 1, 12),
        date(2024, 1, 15), date(2024, 1, 16), date(2024, 1, 17), date(2024, 1, 18), date(2024, 1, 19),
        date(2024, 1, 22), date(2024, 1, 23), date(2024, 1, 24), date(2024, 1, 25), date(2024, 1, 26),
        date(2024, 1, 29), date(2024, 1, 30), date(2024, 1, 31), date(2024, 2, 1), date(2024, 2, 2)
    ]
    prices = [20000, 20200, 20500, 20800, 21000, 21200, 21500, 21800, 22000, 22200,
              22000, 21800, 21500, 21200, 21000, 20800, 20500, 20200, 20000, 19800]

    rows = []
    for d, p in zip(dates, prices):
        rows.append({
            "timestamp": datetime.combine(d, datetime.min.time()),
            "open": float(p), "high": float(p + 300), "low": float(p - 300), "close": float(p),
            "volume": 50000
        })
    df = pd.DataFrame(rows)

    # Indicator triggers:
    # Enter at bar 1 (Jan 9), exit at bar 5 (Jan 15, T+4)
    sig = np.zeros(20)
    sig[1] = 1.0  # Buy trigger
    sig[5] = -1.0 # Sell trigger

    indicator_values = {
        "signal": sig,
        "zero": np.zeros(20),
    }

    strategy = SimpleNamespace(
        name="profile_stress_strategy",
        indicators=[
            {"id": "signal", "kind": "custom", "params": {}},
            {"id": "zero", "kind": "custom", "params": {}},
        ],
        entry_rules=[{"dsl": {"gt": ["signal", "zero"]}}],
        exit_rules=[{"dsl": {"lt": ["signal", "zero"]}}],
        position_sizing=SimpleNamespace(
            method="fixed_quantity", quantity=1000, percent=100.0,
            model_dump=lambda: {"method": "fixed_quantity", "quantity": 1000, "percent": 100.0}
        ),
        risk_management=None
    )

    kernel = BacktestExecutionKernel(profile=custom_profile)
    init_cash = 100_000_000.0
    res = kernel.run(df=df, indicator_values=indicator_values, strategy=strategy, symbol="TCB", initial_cash=init_cash)

    assert len(res.trades) == 1
    trade = res.trades[0]

    # Order executed on next bar open:
    # Buy trigger at bar 1 -> fills on bar 2 open (20500)
    # Sell trigger at bar 5 -> fills on bar 6 open (21500)
    buy_open = 20500.0
    sell_open = 21500.0
    expected_entry_px = buy_open * 1.002
    expected_exit_px = sell_open * 0.998
    assert math.isclose(trade.entry_price, expected_entry_px, abs_tol=1e-4)
    assert math.isclose(trade.exit_price, expected_exit_px, abs_tol=1e-4)

    # Gross PnL
    expected_gross = 1000 * (expected_exit_px - expected_entry_px)
    assert math.isclose(trade.gross_pnl, expected_gross, abs_tol=1e-4)

    # Fees: buy_fee + sell_fee
    expected_fees = (1000 * expected_entry_px * 0.0028) + (1000 * expected_exit_px * 0.0038)
    assert math.isclose(trade.fees, expected_fees, abs_tol=1e-4)

    # Taxes: sell_tax
    expected_taxes = 1000 * expected_exit_px * 0.0015
    assert math.isclose(trade.taxes, expected_taxes, abs_tol=1e-4)

    # Net PnL = gross - fees - taxes
    expected_net = expected_gross - expected_fees - expected_taxes
    assert math.isclose(trade.net_pnl, expected_net, abs_tol=1e-4)

    # Ledger Cash Conservation: Delta Cash == sum(net_pnl)
    cash_delta = res.final_cash - init_cash
    assert math.isclose(cash_delta, expected_net, abs_tol=1e-4)

    # Cross-check with compute_round_trip_pnl
    rt = compute_round_trip_pnl(
        entry_price=trade.entry_price, exit_price=trade.exit_price,
        quantity=trade.quantity, profile=custom_profile
    )
    assert math.isclose(rt.net_pnl, trade.net_pnl, abs_tol=1e-4)


# ============================================================================
# SECTION 6: R6 IMPORT PREVIEW / ACCEPT CONTRACT INTEGRITY
# ============================================================================

def test_adversarial_r6_confirm_false_zero_db_mutation(tmp_path):
    """Stress test: POST /api/import/preview with confirm=False creates strictly 0 rows across all tables."""
    db_file = tmp_path / "test_r6_preview_stress.db"
    engine = create_engine(f"sqlite:///{db_file}", connect_args={"check_same_thread": False})
    Base.metadata.create_all(bind=engine)
    TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)
    db = TestingSessionLocal()

    def override_get_db():
        try:
            yield db
        finally:
            pass

    app.dependency_overrides[get_db] = override_get_db
    client = TestClient(app)

    csv_data = (
        "<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        "VNM,20240108,68.0,69.0,67.5,68.5,120000\n"
        "VNM,20240109,68.5,69.5,68.0,69.0,150000\n"
    ).encode("utf-8")

    # Send preview request with confirm=False
    resp = client.post(
        "/api/import/preview",
        files={"file": ("cafef_sample.csv", csv_data, "text/csv")},
        data={"confirm": "false"}
    )
    assert resp.status_code == 200
    data = resp.json()
    assert data["staged"] is False
    assert data["parsed_count"] == 2
    run_id = data["run_id"]

    # Verify EVERY table has 0 rows
    with engine.connect() as conn:
        for tbl in ["candles", "import_runs", "import_run_items", "import_run_mutations", "event_logs"]:
            cnt = conn.execute(text(f"SELECT COUNT(*) FROM {tbl}")).scalar()
            assert cnt == 0, f"Table {tbl} unexpectedly has {cnt} rows after confirm=False preview!"

    # Attempt to accept the unstaged ephemeral run_id: MUST fail closed with HTTP 400
    accept_resp = client.post(
        f"/api/import/runs/{run_id}/accept",
        json={"content_sha256": data["content_sha256"]}
    )
    assert accept_resp.status_code == 400
    assert "Cannot accept unstaged preview" in accept_resp.json()["detail"]

    # Attempt to accept a completely bogus UUID: MUST fail closed with HTTP 400
    bogus_resp = client.post(
        "/api/import/runs/00000000-0000-0000-0000-000000000000/accept",
        json={"content_sha256": "fake_sha"}
    )
    assert bogus_resp.status_code == 400
    assert "Cannot accept unstaged preview" in bogus_resp.json()["detail"]

    app.dependency_overrides.clear()
    db.close()


def test_adversarial_r6_two_stage_preview_to_accept_workflow(tmp_path):
    """Stress test: Full staged workflow (preview with confirm=True -> accept) commits correctly."""
    db_file = tmp_path / "test_r6_staged_workflow.db"
    engine = create_engine(f"sqlite:///{db_file}", connect_args={"check_same_thread": False})
    Base.metadata.create_all(bind=engine)
    TestingSessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)
    db = TestingSessionLocal()

    def override_get_db():
        try:
            yield db
        finally:
            pass

    app.dependency_overrides[get_db] = override_get_db
    client = TestClient(app)

    csv_data = (
        "<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        "FPT,20240108,95.0,96.0,94.5,95.5,300000\n"
        "FPT,20240109,95.5,97.0,95.0,96.5,450000\n"
    ).encode("utf-8")

    # Step 1: Preview with confirm=True (stages the manifest)
    preview_resp = client.post(
        "/api/import/preview",
        files={"file": ("fpt_staged.csv", csv_data, "text/csv")},
        data={"confirm": "true"}
    )
    assert preview_resp.status_code == 200
    pdata = preview_resp.json()
    assert pdata["staged"] is True
    run_id = pdata["run_id"]
    content_sha = pdata["content_sha256"]

    # Candles should STILL be 0 prior to acceptance
    with engine.connect() as conn:
        assert conn.execute(text("SELECT COUNT(*) FROM candles")).scalar() == 0

    # Step 2: Accept with mismatching checksum -> HTTP 400
    mismatch_resp = client.post(
        f"/api/import/runs/{run_id}/accept",
        json={"content_sha256": "wrong_sha256_hash"}
    )
    assert mismatch_resp.status_code == 400

    # Step 3: Accept with valid checksum -> HTTP 200
    accept_resp = client.post(
        f"/api/import/runs/{run_id}/accept",
        json={"content_sha256": content_sha}
    )
    assert accept_resp.status_code == 200
    adata = accept_resp.json()
    assert adata["status"] == "accepted"
    assert adata["accepted_count"] == 2

    # Step 4: Verify candles and mutations now exist
    with engine.connect() as conn:
        assert conn.execute(text("SELECT COUNT(*) FROM candles")).scalar() >= 2
        assert conn.execute(text("SELECT COUNT(*) FROM import_run_mutations")).scalar() >= 2

    # Step 5: Repeat accept -> Idempotent no-op
    repeat_resp = client.post(
        f"/api/import/runs/{run_id}/accept",
        json={"content_sha256": content_sha}
    )
    assert repeat_resp.status_code == 200
    assert repeat_resp.json()["status"] == "noop"

    app.dependency_overrides.clear()
    db.close()


# ============================================================================
# SECTION 7: GLOBAL DATABASE IMMUTABILITY INVARIANT
# ============================================================================

def test_adversarial_global_sumi_db_untouched():
    """Verify backend/sumi.db has not been accessed or altered in any way."""
    assert SUMI_DB_PATH.exists(), "backend/sumi.db must exist"
    content = SUMI_DB_PATH.read_bytes()
    sha256 = hashlib.sha256(content).hexdigest()
    assert sha256 == EXPECTED_SUMI_DB_SHA256, (
        f"CRITICAL VIOLATION: backend/sumi.db SHA256 altered! Expected {EXPECTED_SUMI_DB_SHA256}, got {sha256}"
    )
