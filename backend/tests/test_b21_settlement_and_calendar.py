"""Golden Test Suite for Batch B21: MarketProfile, Vietnam Calendar, Settlement & Ingestion Harmonization.

Covers:
- Authoritative MarketProfile & DEFAULT_VN_MARKET_PROFILE in rules.py (single source of truth)
- Zero-slippage fix in Broker (profile.slippage = 0.0 default)
- Cross-module parity across accounting.py, trade_planning.py, broker.py, execution.py
- Vietnam exchange holidays 2018-2026 in TradingCalendar (VIETNAM_HOLIDAYS_V1)
- Non-trading status of compensatory makeup Saturdays (2024-05-04, 2025-04-26, 2026-08-22)
- G-06: Standard weekend settlement (Friday buy -> Monday locked -> Tuesday tradable at open)
- G-07: Reunification/May Day 5-day holiday window (Friday 2024-04-26 buy -> Thursday 2024-05-02 locked -> Friday 2024-05-03 tradable at open)
- G-07: Hung Kings midweek holiday (Wednesday 2024-04-17 buy -> Friday 2024-04-19 locked -> Monday 2024-04-22 tradable at open)
- BacktestExecutionKernel premature sell rejection contract (REJECTED_SETTLEMENT_LOCK / LOT_SETTLEMENT_LOCKED)
- TradeLifecycleService sell rejection contract (400 HTTPException with LOT_SETTLEMENT_LOCKED payload)
- PracticeWorkflowSnapshot settlement_label contract ("T+1 · về sau 1 phiên")
- ImportClassifier float tolerance (math.isclose / are_candles_close) and holiday gap detection
- WeeklyAggregator shortened holiday week aggregation and last-traded session timestamp anchor
- cafef/classifier.py compatibility shim re-exports

Database safety:
All tests run in-memory or on isolated temporary SQLite databases.
Production backend/sumi.db is strictly untouched.
"""

from __future__ import annotations

import math
from datetime import date, datetime, timedelta
from typing import Generator
import pandas as pd
import pytest
from fastapi import HTTPException
from sqlalchemy import create_engine
from sqlalchemy.orm import Session, sessionmaker

from app.db import Base
from app.domain.accounting import BUY_FEE_RATE, SELL_FEE_RATE, SELL_TAX_RATE
from app.domain.backtest.execution import BacktestExecutionKernel, RejectedSettlementLock
from app.domain.engine.broker import BrokerSimulation, SlippageModel
from app.domain.engine.events import EventDispatcher
from app.domain.market.calendar import TradingCalendar, VIETNAM_HOLIDAYS_V1
from app.domain.market.rules import DEFAULT_MARKET_PROFILE, DEFAULT_VN_MARKET_PROFILE, MarketProfile
from app.domain.market.settlement import SettlementEngine
from app.domain.trade_planning import TradePlanInput
from app.models.candle import Candle
from app.models.decision import Decision
from app.models.execution import Execution
from app.models.order import Order
from app.models.position import Position
from app.models.replay_session import ReplaySession
from app.models.trade import Trade
from app.schemas.decision_schema import DecisionCreate
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
from app.services.practice_workflow_service import PracticeWorkflowService
from app.services.trade_lifecycle_service import TradeLifecycleService
from app.services.weekly_aggregator import WeeklyAggregator


@pytest.fixture
def isolated_db(tmp_path) -> Generator[Session, None, None]:
    """Isolated SQLite database session."""
    db_file = tmp_path / "test_b21.db"
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


# ---------------------------------------------------------------------------
# 1. Authoritative MarketProfile & Parity
# ---------------------------------------------------------------------------


def test_b21_authoritative_market_profile_constants():
    """Verify DEFAULT_VN_MARKET_PROFILE is the single canonical source of truth."""
    profile = DEFAULT_VN_MARKET_PROFILE

    assert profile.symbol == "VN"
    assert profile.lot_size == 100
    assert profile.fee_rate == 0.0015
    assert profile.tax_rate == 0.0010
    assert profile.slippage == 0.0
    assert profile.settlement_days == 2
    assert profile.settlement_label == "T+1 · về sau 1 phiên"
    assert profile.settlement_mode == "T+1.5_CALENDAR"
    assert profile.settlement_release_phase == "open"

    # Properties
    assert profile.fee_buy == 0.0015
    assert profile.fee_sell == 0.0015
    assert profile.tax_sell == 0.0010
    assert profile.slippage_rate == 0.0

    # Alias equality
    assert DEFAULT_MARKET_PROFILE is DEFAULT_VN_MARKET_PROFILE


def test_b21_cross_module_profile_parity():
    """Verify accounting, trade planning, and broker align with canonical profile."""
    # accounting.py
    assert BUY_FEE_RATE == DEFAULT_VN_MARKET_PROFILE.fee_buy
    assert SELL_FEE_RATE == DEFAULT_VN_MARKET_PROFILE.fee_sell
    assert SELL_TAX_RATE == DEFAULT_VN_MARKET_PROFILE.tax_sell

    # trade_planning.py
    plan_input = TradePlanInput(entry_price=10.0, stop_loss=9.0, target_price=12.0)
    assert plan_input.lot_size == DEFAULT_VN_MARKET_PROFILE.lot_size
    assert plan_input.fee_rate == DEFAULT_VN_MARKET_PROFILE.fee_rate
    assert plan_input.tax_rate == DEFAULT_VN_MARKET_PROFILE.tax_rate

    # broker.py default slippage must be 0.0 (fixing the 0.001 bug)
    assert SlippageModel().pct == 0.0
    broker = BrokerSimulation(dispatcher=EventDispatcher())
    assert broker.profile.slippage == 0.0
    assert broker.profile.lot_size == 100
    assert broker.profile.fee_buy == 0.0015
    assert broker.profile.tax_sell == 0.0010
    assert broker.slippage_model.pct == 0.0


# ---------------------------------------------------------------------------
# 2. Vietnam Trading Calendar & Holidays 2018–2026
# ---------------------------------------------------------------------------


def test_b21_vietnam_calendar_version_and_holidays_2018_2026():
    """Verify calendar version and holiday fixture coverage 2018-2026."""
    cal = TradingCalendar()
    assert TradingCalendar.VERSION == "VN_CALENDAR_2018_2026_V1"

    # Check 2018 key holidays
    assert date(2018, 1, 1) in VIETNAM_HOLIDAYS_V1  # New Year
    assert date(2018, 2, 14) in VIETNAM_HOLIDAYS_V1  # Tet 29
    assert date(2018, 2, 15) in VIETNAM_HOLIDAYS_V1  # Tet 30
    assert date(2018, 2, 16) in VIETNAM_HOLIDAYS_V1  # Tet Mung 1
    assert date(2018, 2, 19) in VIETNAM_HOLIDAYS_V1  # Tet makeup
    assert date(2018, 2, 20) in VIETNAM_HOLIDAYS_V1  # Tet makeup
    assert date(2018, 4, 25) in VIETNAM_HOLIDAYS_V1  # Hung Kings
    assert date(2018, 4, 30) in VIETNAM_HOLIDAYS_V1  # 30/4
    assert date(2018, 5, 1) in VIETNAM_HOLIDAYS_V1   # 1/5
    assert date(2018, 9, 3) in VIETNAM_HOLIDAYS_V1   # 2/9 makeup

    # Check 2019 key holidays
    assert date(2019, 1, 1) in VIETNAM_HOLIDAYS_V1
    assert date(2019, 2, 4) in VIETNAM_HOLIDAYS_V1  # Tet 30
    assert date(2019, 2, 5) in VIETNAM_HOLIDAYS_V1  # Tet Mung 1
    assert date(2019, 2, 6) in VIETNAM_HOLIDAYS_V1  # Tet Mung 2
    assert date(2019, 2, 7) in VIETNAM_HOLIDAYS_V1  # Tet Mung 3
    assert date(2019, 2, 8) in VIETNAM_HOLIDAYS_V1  # Tet Mung 4
    assert date(2019, 4, 15) in VIETNAM_HOLIDAYS_V1 # Hung Kings makeup
    assert date(2019, 4, 29) in VIETNAM_HOLIDAYS_V1 # Bridge swap day
    assert date(2019, 4, 30) in VIETNAM_HOLIDAYS_V1
    assert date(2019, 5, 1) in VIETNAM_HOLIDAYS_V1
    assert date(2019, 9, 2) in VIETNAM_HOLIDAYS_V1

    # Verify is_trading_day rejects holidays
    for hol in [date(2018, 2, 15), date(2019, 4, 29), date(2024, 4, 30), date(2026, 2, 18)]:
        assert cal.is_trading_day(hol) is False


def test_b21_compensatory_saturdays_non_trading():
    """Verify government compensatory work Saturdays never trade exchange."""
    cal = TradingCalendar()

    # 2019 swap Saturday 04/05/2019 did not trade
    assert cal.is_trading_day(date(2019, 5, 4)) is False

    # 2024 swap Saturday 04/05/2024 did not trade
    assert cal.is_trading_day(date(2024, 5, 4)) is False

    # 2025 swap Saturday 26/04/2025 does not trade
    assert cal.is_trading_day(date(2025, 4, 26)) is False

    # 2026 swap Saturday 22/08/2026 does not trade
    assert cal.is_trading_day(date(2026, 8, 22)) is False


# ---------------------------------------------------------------------------
# 3. G-06 & G-07 Settlement Tests (SettlementEngine)
# ---------------------------------------------------------------------------


def test_b21_g06_standard_weekend_settlement():
    """G-06: Friday buy -> Monday locked -> Tuesday tradable at open."""
    engine = SettlementEngine()
    buy_date = date(2024, 3, 1)  # Friday

    # Friday itself: T+0 (same bar) -> Locked
    assert engine.is_sellable(buy_date, buy_date, current_session_phase="close") is False

    # Saturday 2024-03-02 & Sunday 2024-03-03 -> Weekend, not trading days
    assert engine.calendar.is_trading_day(date(2024, 3, 2)) is False
    assert engine.calendar.is_trading_day(date(2024, 3, 3)) is False

    # Monday 2024-03-04: T+1 trading day -> Locked all day
    assert engine.is_sellable(buy_date, date(2024, 3, 4), current_session_phase="open") is False
    assert engine.is_sellable(buy_date, date(2024, 3, 4), current_session_phase="close") is False

    # Tuesday 2024-03-05: T+2 trading day -> In conservative T+1.5 mode, morning open is locked, afternoon/close is sellable!
    assert engine.is_sellable(buy_date, date(2024, 3, 5), current_session_phase="open") is False
    assert engine.is_sellable(buy_date, date(2024, 3, 5), current_session_phase="close") is True

    # Wednesday 2024-03-06: T+3 trading day -> Fully sellable across all sessions
    assert engine.is_sellable(buy_date, date(2024, 3, 6), current_session_phase="open") is True
    assert engine.is_sellable(buy_date, date(2024, 3, 6), current_session_phase="close") is True

    # Exact sellable date check (afternoon policy)
    assert engine.calculate_sellable_date(buy_date) == date(2024, 3, 5)


def test_b21_g07_reunification_may_day_5day_holiday_window():
    """G-07: Friday 2024-04-26 buy -> 5-day holiday window skipped -> Thursday locked -> Friday tradable at close."""
    engine = SettlementEngine()
    buy_date = date(2024, 4, 26)  # Friday before long holiday

    # Holiday calendar verification
    assert engine.calendar.is_trading_day(date(2024, 4, 27)) is False  # Sat
    assert engine.calendar.is_trading_day(date(2024, 4, 28)) is False  # Sun
    assert engine.calendar.is_trading_day(date(2024, 4, 29)) is False  # Mon (bridge holiday)
    assert engine.calendar.is_trading_day(date(2024, 4, 30)) is False  # Tue (30/4)
    assert engine.calendar.is_trading_day(date(2024, 5, 1)) is False   # Wed (1/5)

    # Thursday 2024-05-02 is T+1 trading day -> Locked
    assert engine.calendar.is_trading_day(date(2024, 5, 2)) is True
    assert engine.is_sellable(buy_date, date(2024, 5, 2), current_session_phase="open") is False
    assert engine.is_sellable(buy_date, date(2024, 5, 2), current_session_phase="close") is False

    # Friday 2024-05-03 is T+2 trading day -> Tradable in afternoon/close!
    assert engine.calendar.is_trading_day(date(2024, 5, 3)) is True
    assert engine.is_sellable(buy_date, date(2024, 5, 3), current_session_phase="open") is False
    assert engine.is_sellable(buy_date, date(2024, 5, 3), current_session_phase="close") is True

    # Monday 2024-05-06 is T+3 trading day -> Tradable at Open!
    assert engine.is_sellable(buy_date, date(2024, 5, 6), current_session_phase="open") is True

    # Compensatory Saturday 2024-05-04 does not trade exchange
    assert engine.calendar.is_trading_day(date(2024, 5, 4)) is False

    # Exact sellable date calculation
    assert engine.calculate_sellable_date(buy_date) == date(2024, 5, 3)


def test_b21_g07_hung_kings_midweek_holiday():
    """G-07: Wednesday 2024-04-17 buy -> Thursday holiday skipped -> Friday locked -> Monday tradable at close."""
    engine = SettlementEngine()
    buy_date = date(2024, 4, 17)  # Wednesday

    # Thursday 2024-04-18 is Hung Kings public holiday
    assert engine.calendar.is_trading_day(date(2024, 4, 18)) is False

    # Friday 2024-04-19 is T+1 trading day -> Locked
    assert engine.calendar.is_trading_day(date(2024, 4, 19)) is True
    assert engine.is_sellable(buy_date, date(2024, 4, 19), current_session_phase="open") is False
    assert engine.is_sellable(buy_date, date(2024, 4, 19), current_session_phase="close") is False

    # Weekend 2024-04-20 & 2024-04-21 skipped
    assert engine.calendar.is_trading_day(date(2024, 4, 20)) is False
    assert engine.calendar.is_trading_day(date(2024, 4, 21)) is False

    # Monday 2024-04-22 is T+2 trading day -> Tradable in afternoon/close!
    assert engine.calendar.is_trading_day(date(2024, 4, 22)) is True
    assert engine.is_sellable(buy_date, date(2024, 4, 22), current_session_phase="open") is False
    assert engine.is_sellable(buy_date, date(2024, 4, 22), current_session_phase="close") is True

    # Tuesday 2024-04-23 is T+3 trading day -> Tradable at Open!
    assert engine.is_sellable(buy_date, date(2024, 4, 23), current_session_phase="open") is True

    # Exact sellable date calculation
    assert engine.calculate_sellable_date(buy_date) == date(2024, 4, 22)


# ---------------------------------------------------------------------------
# 4. BacktestExecutionKernel Settlement Rejection Payload
# ---------------------------------------------------------------------------


def test_b21_backtest_kernel_settlement_rejection():
    """Premature sell in BacktestExecutionKernel produces REJECTED_SETTLEMENT_LOCK."""
    from types import SimpleNamespace
    import numpy as np

    dates = pd.date_range("2024-03-01", periods=6, freq="D")
    df = pd.DataFrame({
        "timestamp": dates,
        "open": [10.0, 10.5, 11.0, 11.2, 11.5, 12.0],
        "high": [10.8, 10.9, 11.4, 11.6, 11.8, 12.2],
        "low": [9.8, 10.2, 10.8, 11.0, 11.2, 11.7],
        "close": [10.5, 10.8, 11.2, 11.4, 11.6, 12.0],
        "volume": [10000, 12000, 15000, 11000, 13000, 14000],
    }, index=dates)

    fast_ma = np.array([10.0, 20.0, 5.0, 5.0, 5.0, 5.0])
    slow_ma = np.array([10.0, 10.0, 15.0, 15.0, 15.0, 15.0])
    indicator_values = {"fast_ma": fast_ma, "slow_ma": slow_ma}

    strategy = SimpleNamespace(
        name="test_strat",
        indicators=[
            {"id": "fast_ma", "kind": "sma", "params": {"length": 1}},
            {"id": "slow_ma", "kind": "sma", "params": {"length": 2}},
        ],
        entry_rules=[{"dsl": {"gt": ["fast_ma", "slow_ma"]}}],
        exit_rules=[{"dsl": {"lt": ["fast_ma", "slow_ma"]}}],
        position_sizing=SimpleNamespace(
            method="fixed_quantity",
            quantity=100,
            percent=100.0,
            model_dump=lambda: {"method": "fixed_quantity", "quantity": 100, "percent": 100.0},
        ),
        risk_management=None,
    )

    kernel = BacktestExecutionKernel(
        profile=DEFAULT_VN_MARKET_PROFILE,
        min_holding_bars=2,
    )
    result = kernel.run(
        df=df,
        indicator_values=indicator_values,
        strategy=strategy,
        symbol="TCB",
        initial_cash=100_000.0,
    )

    rejected_orders = [o for o in result.ledger.orders if o.status == "REJECTED_SETTLEMENT_LOCK"]
    assert len(rejected_orders) >= 1

    rej_order = rejected_orders[0]
    # Backward compatibility: must also equal "REJECTED"
    assert rej_order.status == "REJECTED"
    assert rej_order.status == "REJECTED_SETTLEMENT_LOCK"
    assert "LOT_SETTLEMENT_LOCKED" in rej_order.rejection_reason
    assert "UNSETTLED_HOLDING_T2" in rej_order.rejection_reason


# ---------------------------------------------------------------------------
# 5. TradeLifecycleService Settlement Rejection Contract
# ---------------------------------------------------------------------------


def test_b21_trade_lifecycle_sell_rejection_http_exception(isolated_db: Session):
    """Premature sell in TradeLifecycleService raises 400 with LOT_SETTLEMENT_LOCKED."""
    db = isolated_db

    # Create candles: Fri 2024-03-01, Mon 2024-03-04, Tue 2024-03-05
    c1 = Candle(symbol="VCB", timeframe="1D", timestamp=datetime(2024, 3, 1), open=90.0, high=92.0, low=89.0, close=91.0, volume=1000)
    c2 = Candle(symbol="VCB", timeframe="1D", timestamp=datetime(2024, 3, 4), open=91.0, high=93.0, low=90.0, close=92.0, volume=1200)
    c3 = Candle(symbol="VCB", timeframe="1D", timestamp=datetime(2024, 3, 5), open=92.0, high=94.0, low=91.0, close=93.0, volume=1500)
    db.add_all([c1, c2, c3])
    db.commit()

    # Create replay session
    session = ReplaySession(
        symbol="VCB",
        timeframe="1D",
        start_date=datetime(2024, 3, 1),
        end_date=datetime(2024, 3, 5),
        current_index=1,  # Mon 2024-03-04
        initial_cash=100_000_000.0,
        current_cash=90_000_000.0,
    )
    db.add(session)
    db.commit()

    # Create a BUY execution on Fri 2024-03-01 (bar 0)
    dec = Decision(session_id=session.id, symbol="VCB", action="BUY", candle_index=0, decision_date=datetime(2024, 3, 1), price=90.0, planned_quantity=100)
    db.add(dec)
    db.commit()

    order = Order(session_id=session.id, decision_id=dec.id, symbol="VCB", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(order)
    db.commit()

    exc = Execution(
        session_id=session.id,
        order_id=order.id,
        symbol="VCB",
        execution_candle_index=0,
        execution_date=datetime(2024, 3, 1),
        execution_price=90.0,
        quantity=100,
        gross_amount=9000.0,
        net_amount=9000.0 + (9000.0 * 0.0015),
    )
    db.add(exc)
    db.commit()

    # Position exists
    pos = Position(session_id=session.id, symbol="VCB", quantity=100, average_price=90.0, total_cost=9000.0, opened_at=datetime(2024, 3, 1))
    db.add(pos)
    db.commit()

    # Now attempt a SELL on Mon 2024-03-04 (bar 1). Since buy was Fri 2024-03-01, sellable date is Tue 2024-03-05!
    # Mon 2024-03-04 is UNSETTLED!
    sell_dec = Decision(session_id=session.id, symbol="VCB", action="SELL", candle_index=1, decision_date=datetime(2024, 3, 4), price=92.0, planned_quantity=100)
    db.add(sell_dec)
    db.commit()

    dec_in = DecisionCreate(action="SELL", candle_index=1, price=92.0, quantity=100)
    with pytest.raises(HTTPException) as exc_info:
        TradeLifecycleService._execute_sell(
            db=db,
            session=session,
            decision_id=sell_dec.id,
            decision_in=dec_in,
            symbol="VCB",
            date=datetime(2024, 3, 4),
            price=92.0,
            qty=100.0,
        )

    assert exc_info.value.status_code == 400
    detail = exc_info.value.detail
    assert isinstance(detail, dict)
    assert detail["error_code"] == "LOT_SETTLEMENT_LOCKED"
    assert detail["status"] == "REJECTED_SETTLEMENT_LOCK"
    assert detail["settlement_label"] == "T+1 · về sau 1 phiên"
    assert "earliest_release_date" in detail
    assert detail["earliest_release_date"].startswith("2024-03-05")
    # Verify backward compatibility with string searches
    assert "Cannot sell: T+2 constraint" in str(detail)


# ---------------------------------------------------------------------------
# 6. PracticeWorkflowSnapshot Contract
# ---------------------------------------------------------------------------


def test_b21_practice_workflow_snapshot_contract(isolated_db: Session):
    """Verify PracticeWorkflowSnapshot exposes settlement_label and proper quantities."""
    db = isolated_db

    c1 = Candle(symbol="VCB", timeframe="1D", timestamp=datetime(2024, 3, 1), open=90.0, high=92.0, low=89.0, close=91.0, volume=1000)
    c2 = Candle(symbol="VCB", timeframe="1D", timestamp=datetime(2024, 3, 4), open=91.0, high=93.0, low=90.0, close=92.0, volume=1200)
    c3 = Candle(symbol="VCB", timeframe="1D", timestamp=datetime(2024, 3, 5), open=92.0, high=94.0, low=91.0, close=93.0, volume=1100)
    db.add_all([c1, c2, c3])
    db.commit()

    session = ReplaySession(
        symbol="VCB",
        timeframe="1D",
        start_date=datetime(2024, 3, 1),
        end_date=datetime(2024, 3, 5),
        current_index=1,  # Mon 03/04
        initial_cash=100_000_000.0,
        current_cash=90_000_000.0,
    )
    db.add(session)
    db.commit()

    # Buy on bar 0
    dec = Decision(session_id=session.id, symbol="VCB", action="BUY", candle_index=0, decision_date=datetime(2024, 3, 1), price=90.0, planned_quantity=100)
    db.add(dec)
    db.commit()
    order = Order(session_id=session.id, decision_id=dec.id, symbol="VCB", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(order)
    db.commit()
    exc = Execution(session_id=session.id, order_id=order.id, symbol="VCB", execution_candle_index=0, execution_date=datetime(2024, 3, 1), execution_price=90.0, quantity=100, gross_amount=9000.0, net_amount=9013.5)
    db.add(exc)
    pos = Position(session_id=session.id, symbol="VCB", quantity=100, average_price=90.0, total_cost=9000.0, opened_at=datetime(2024, 3, 1))
    db.add(pos)
    db.commit()

    snapshot = PracticeWorkflowService.get_snapshot(db, session.id)

    assert hasattr(snapshot, "settlement_label")
    assert snapshot.settlement_label == "T+1 · về sau 1 phiên"
    # On Mon 2024-03-04 (bar 1), shares bought on Fri 2024-03-01 are STILL UNSETTLED (T+1 trading day)
    assert snapshot.available_quantity == 0
    assert snapshot.blocked_quantity == 100
    assert snapshot.earliest_release_date is not None
    assert snapshot.earliest_release_date.date() == date(2024, 3, 5)


# ---------------------------------------------------------------------------
# 7. ImportClassifier Float Tolerance & Holiday Gaps
# ---------------------------------------------------------------------------


def test_b21_import_classifier_float_tolerance():
    """Verify are_candles_close allows float imprecision and detects real conflicts."""
    t1 = (30.0, 31.0, 29.5, 30.5, 100000.0)
    # Negligible IEEE-754 / float epsilon diff
    t2 = (30.0000000001, 31.0000000002, 29.4999999999, 30.5000000001, 100000.0)
    assert are_candles_close(t1, t2) is True
    assert cafef_are_candles_close(t1, t2) is True

    # Real price change
    t3 = (30.0, 31.5, 29.5, 30.5, 100000.0)
    assert are_candles_close(t1, t3) is False

    # Within classify_records: duplicate when within tolerance
    existing_map = {
        ("SSI", "1D", date(2026, 1, 5), "unadjusted"): t1
    }
    df_imprecise = pd.DataFrame([
        {"symbol": "SSI", "timestamp": "2026-01-05", "open": 30.0000000001, "high": 31.0000000002, "low": 29.4999999999, "close": 30.5000000001, "volume": 100000.0},
    ])
    items, counts, can_accept, block_reason = ImportClassifier.classify_records(df_imprecise, existing_map)
    assert counts["duplicate"] == 1
    assert counts["conflicting"] == 0


def test_b21_import_classifier_holiday_gap_not_missing():
    """Gap spanning official public holiday is recognized and NOT flagged as missing trading day."""
    # 2024-04-30 (Tue) and 2024-05-01 (Wed) are official holidays
    # Import Monday 2024-04-29 and Thursday 2024-05-02
    df = pd.DataFrame([
        {"symbol": "SSI", "timestamp": "2024-04-29", "open": 30.0, "high": 31.0, "low": 29.5, "close": 30.5, "volume": 100000},
        {"symbol": "SSI", "timestamp": "2024-05-02", "open": 30.5, "high": 31.5, "low": 30.0, "close": 31.0, "volume": 110000},
    ])
    items, counts, can_accept, block_reason = ImportClassifier.classify_records(df, {})

    # Neither 2024-04-30 nor 2024-05-01 should be marked as "missing"
    missing_items = [it for it in items if it.classification == "missing"]
    assert len(missing_items) == 0
    assert counts["missing"] == 0


def test_b21_cafef_classifier_reexports():
    """Verify cafef/classifier.py compatibility shim properly re-exports canonical symbols."""
    assert CafeFImportClassifier is ImportClassifier
    assert CafeFClassifiedRow is ClassifiedRow
    assert cafef_are_candles_close is are_candles_close


# ---------------------------------------------------------------------------
# 8. WeeklyAggregator Shortened Trading Week
# ---------------------------------------------------------------------------


def test_b21_weekly_aggregator_shortened_week(isolated_db: Session):
    """WeeklyAggregator anchors 1W candle to final traded session of a shortened week."""
    db = isolated_db

    # April 2024 week:
    # 2024-04-15 (Mon), 2024-04-16 (Tue), 2024-04-17 (Wed), 2024-04-19 (Fri)
    # 2024-04-18 is Hung Kings holiday!
    c1 = Candle(symbol="FPT", timeframe="1D", timestamp=datetime(2024, 4, 15), open=100.0, high=102.0, low=99.0, close=101.0, volume=1000, adjustment_type="unadjusted")
    c2 = Candle(symbol="FPT", timeframe="1D", timestamp=datetime(2024, 4, 16), open=101.0, high=103.0, low=100.0, close=102.0, volume=1200, adjustment_type="unadjusted")
    c3 = Candle(symbol="FPT", timeframe="1D", timestamp=datetime(2024, 4, 17), open=102.0, high=104.0, low=101.0, close=103.0, volume=1500, adjustment_type="unadjusted")
    c4 = Candle(symbol="FPT", timeframe="1D", timestamp=datetime(2024, 4, 19), open=103.0, high=106.0, low=102.0, close=105.0, volume=2000, adjustment_type="unadjusted")

    db.add_all([c1, c2, c3, c4])
    db.commit()

    count = WeeklyAggregator.derive_weekly_candles(db, {"FPT"}, {"unadjusted"})
    assert count == 1

    weekly_candle = db.query(Candle).filter(Candle.symbol == "FPT", Candle.timeframe == "1W").first()
    assert weekly_candle is not None
    # Anchors to final traded session: 2024-04-19
    assert weekly_candle.timestamp.date() == date(2024, 4, 19)
    assert weekly_candle.open == 100.0
    assert weekly_candle.high == 106.0
    assert weekly_candle.low == 99.0
    assert weekly_candle.close == 105.0
    assert weekly_candle.volume == 5700.0
