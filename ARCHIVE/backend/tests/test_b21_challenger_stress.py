"""Adversarial Stress Test Suite for Batch B21: MarketProfile, Vietnam Calendar & Multi-Lot Settlement.

Challenge Objectives:
1. Multi-Year Holiday Edge Cases:
   - Test calendar behavior across Tết holidays for 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026.
   - Verify that buying on the trading day immediately preceding a 7-day or 9-day Tết closure holds shares locked until the second open trading day after markets reopen.
2. Multi-Lot FIFO Allocation Stress:
   - Create complex buying scenarios: Buy Lot A on Day 1, Buy Lot B on Day 2, Buy Lot C on Day 3.
   - On Day 3: Attempt to sell Lot A (permitted/tradable), while attempting to sell Lot A + B + C should only allow Lot A and reject/lock Lot B + C.
3. Payload Verification:
   - Verify that rejection when selling locked shares contains exact earliest_release_date, blocked_quantity / blocked_qty, available_quantity / available_qty, and settlement_label = "T+1 · về sau 1 phiên".
4. Boundary and Invariance Guarantees:
   - Isolated temporary SQLite in-memory fixtures only; production backend/sumi.db is strictly untouched.
"""

from __future__ import annotations

from datetime import date, datetime, timedelta
from typing import Generator
import numpy as np
import pandas as pd
import pytest
from fastapi import HTTPException
from sqlalchemy import create_engine
from sqlalchemy.orm import Session, sessionmaker

from app.db import Base
from app.domain.backtest.execution import BacktestExecutionKernel, RejectedSettlementLock
from app.domain.market.calendar import TradingCalendar, VIETNAM_HOLIDAYS_V1
from app.domain.market.rules import DEFAULT_VN_MARKET_PROFILE
from app.domain.market.settlement import HoldingLot, SettlementEngine
from app.models.candle import Candle
from app.models.decision import Decision
from app.models.execution import Execution
from app.models.order import Order
from app.models.position import Position
from app.models.replay_session import ReplaySession
from app.schemas.decision_schema import DecisionCreate
from app.services.practice_workflow_service import PracticeWorkflowService
from app.services.trade_lifecycle_service import TradeLifecycleService


@pytest.fixture
def isolated_db(tmp_path) -> Generator[Session, None, None]:
    """Isolated SQLite database session for adversarial stress testing."""
    db_file = tmp_path / "test_b21_challenger_stress.db"
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
# 1. Multi-Year Holiday Edge Cases: Tết Closures (2018–2026)
# ===========================================================================


def test_adv_tet_holidays_multi_year_edge_cases_2018_2026():
    """Verify pre-Tết purchases across 2018-2026 lock shares across 7-9 day closures.

    For all 9 years (2018 to 2026):
    - Identify the final open trading day immediately preceding the Tết holiday window.
    - Verify the total market closure (7 or 9 calendar days).
    - Ensure shares are unsellable throughout the holiday window.
    - Verify on Reopen Day 1 (T+1 trading day): shares remain 100% LOCKED.
    - Verify on Reopen Day 2 (T+2 trading day): shares become tradable in afternoon/close under T+1.5.
    - Verify on Reopen Day 3 (T+3 trading day): shares become tradable at open.
    """
    cal = TradingCalendar()
    engine = SettlementEngine(cal, conservative_mode=True)

    expected_tet_profiles = {
        2018: {"closure_days": 7, "last_pre_tet": date(2018, 2, 13), "reopen_t1": date(2018, 2, 21), "reopen_t2": date(2018, 2, 22), "reopen_t3": date(2018, 2, 23)},
        2019: {"closure_days": 9, "last_pre_tet": date(2019, 2, 1),  "reopen_t1": date(2019, 2, 11), "reopen_t2": date(2019, 2, 12), "reopen_t3": date(2019, 2, 13)},
        2020: {"closure_days": 7, "last_pre_tet": date(2020, 1, 22), "reopen_t1": date(2020, 1, 30), "reopen_t2": date(2020, 1, 31), "reopen_t3": date(2020, 2, 3)},
        2021: {"closure_days": 7, "last_pre_tet": date(2021, 2, 9),  "reopen_t1": date(2021, 2, 17), "reopen_t2": date(2021, 2, 18), "reopen_t3": date(2021, 2, 19)},
        2022: {"closure_days": 9, "last_pre_tet": date(2022, 1, 28), "reopen_t1": date(2022, 2, 7),  "reopen_t2": date(2022, 2, 8),  "reopen_t3": date(2022, 2, 9)},
        2023: {"closure_days": 7, "last_pre_tet": date(2023, 1, 19), "reopen_t1": date(2023, 1, 27), "reopen_t2": date(2023, 1, 30), "reopen_t3": date(2023, 1, 31)},
        2024: {"closure_days": 7, "last_pre_tet": date(2024, 2, 7),  "reopen_t1": date(2024, 2, 15), "reopen_t2": date(2024, 2, 16), "reopen_t3": date(2024, 2, 19)},
        2025: {"closure_days": 9, "last_pre_tet": date(2025, 1, 24), "reopen_t1": date(2025, 2, 3),  "reopen_t2": date(2025, 2, 4),  "reopen_t3": date(2025, 2, 5)},
        2026: {"closure_days": 9, "last_pre_tet": date(2026, 2, 13), "reopen_t1": date(2026, 2, 23), "reopen_t2": date(2026, 2, 24), "reopen_t3": date(2026, 2, 25)},
    }

    for year, exp in expected_tet_profiles.items():
        # Filter Lunar New Year holidays (excluding solar New Year Jan 1-3)
        tet_holidays = sorted([
            d for d in VIETNAM_HOLIDAYS_V1
            if d.year == year and (d.month == 2 or (d.month == 1 and d.day > 10))
        ])
        assert len(tet_holidays) >= 5, f"Year {year} must have at least 5 registered Tết weekdays"

        first_hol = tet_holidays[0]
        last_hol = tet_holidays[-1]

        last_trade_day = cal.previous_trading_day(first_hol)
        first_reopen_day = cal.next_trading_day(last_hol)
        second_reopen_day = cal.next_trading_day(first_reopen_day)
        third_reopen_day = cal.next_trading_day(second_reopen_day)

        assert last_trade_day == exp["last_pre_tet"], f"Mismatch last pre-Tết trading day for {year}"
        assert first_reopen_day == exp["reopen_t1"], f"Mismatch first reopen day (T+1) for {year}"
        assert second_reopen_day == exp["reopen_t2"], f"Mismatch second reopen day (T+2) for {year}"
        assert third_reopen_day == exp["reopen_t3"], f"Mismatch third reopen day (T+3) for {year}"

        closure_days = (first_reopen_day - last_trade_day).days - 1
        assert closure_days == exp["closure_days"], f"Mismatch closure length for {year}: got {closure_days}, expected {exp['closure_days']}"

        # Calendar session arithmetic
        assert cal.add_trading_days(last_trade_day, 1) == first_reopen_day
        assert cal.add_trading_days(last_trade_day, 2) == second_reopen_day
        assert cal.add_trading_days(last_trade_day, 3) == third_reopen_day

        # SettlementEngine calculation
        assert engine.calculate_sellable_date(last_trade_day, session_policy="afternoon") == second_reopen_day
        assert engine.calculate_sellable_date(last_trade_day, session_policy="open_only") == third_reopen_day

        # During closure: unsellable
        cur = last_trade_day
        while cur < first_reopen_day:
            assert engine.is_sellable(last_trade_day, cur, current_session_phase="close") is False
            cur += timedelta(days=1)

        # Reopen Day 1 (T+1): completely locked
        assert engine.is_sellable(last_trade_day, first_reopen_day, current_session_phase="open") is False
        assert engine.is_sellable(last_trade_day, first_reopen_day, current_session_phase="close") is False

        # Reopen Day 2 (T+2): locked at Open in conservative mode, tradable in afternoon/close
        assert engine.is_sellable(last_trade_day, second_reopen_day, current_session_phase="open") is False
        assert engine.is_sellable(last_trade_day, second_reopen_day, current_session_phase="close") is True

        # Reopen Day 3 (T+3): tradable at Open
        assert engine.is_sellable(last_trade_day, third_reopen_day, current_session_phase="open") is True


def test_adv_backtest_kernel_tet_holiday_closure_lock():
    """Verify BacktestExecutionKernel rejects premature sells across a 7-day Tết closure."""
    from types import SimpleNamespace

    # Dates spanning Tết 2024:
    # 2024-02-05 (Mon, bar 0)
    # 2024-02-06 (Tue, bar 1) -> BUY signal triggers at close
    # 2024-02-07 (Wed, bar 2) -> BUY fills at open (pre-Tết); SELL signal triggers at close
    # [Tết closure 2024-02-08 to 2024-02-14: 7 calendar days]
    # 2024-02-15 (Thu, bar 3) -> Reopen Day 1 (T+1): SELL order rejected (T+1 lock); SELL signal triggers at close
    # 2024-02-16 (Fri, bar 4) -> Reopen Day 2 (T+2): SELL order rejected (T+2 open lock); SELL signal triggers at close
    # 2024-02-19 (Mon, bar 5) -> Reopen Day 3 (T+3): SELL order fills at open!
    dates = pd.to_datetime(["2024-02-05", "2024-02-06", "2024-02-07", "2024-02-15", "2024-02-16", "2024-02-19"])
    df = pd.DataFrame({
        "timestamp": dates,
        "open": [20.0, 20.0, 20.5, 21.0, 21.5, 22.0],
        "high": [20.5, 20.8, 20.9, 21.6, 21.8, 22.5],
        "low": [19.5, 19.8, 20.2, 20.8, 21.2, 21.7],
        "close": [20.0, 20.5, 20.8, 21.4, 21.6, 22.2],
        "volume": [10000, 10000, 12000, 15000, 11000, 13000],
    }, index=dates)

    fast_ma = np.array([10.0, 20.0, 5.0, 5.0, 5.0, 5.0])
    slow_ma = np.array([10.0, 10.0, 15.0, 15.0, 15.0, 15.0])
    indicator_values = {"fast_ma": fast_ma, "slow_ma": slow_ma}

    strategy = SimpleNamespace(
        name="adv_tet_strat",
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
        symbol="HPG",
        initial_cash=100_000.0,
    )

    orders = result.ledger.orders
    assert len(orders) == 4

    # Order 0: BUY on bar 1 (2024-02-06 close) filled on bar 2 (2024-02-07 open, pre-Tết)
    assert orders[0].status == "FILLED"
    assert orders[0].side == "BUY"

    # Order 1: Attempted SELL on bar 3 (2024-02-15, Reopen Day 1 T+1) -> REJECTED_SETTLEMENT_LOCK
    assert orders[1].status == "REJECTED_SETTLEMENT_LOCK"
    assert "LOT_SETTLEMENT_LOCKED" in orders[1].rejection_reason
    assert "2024-02-16" in orders[1].rejection_reason

    # Order 2: Attempted SELL on bar 4 (2024-02-16, Reopen Day 2 T+2 Open) -> REJECTED_SETTLEMENT_LOCK (open locked)
    assert orders[2].status == "REJECTED_SETTLEMENT_LOCK"
    assert "LOT_SETTLEMENT_LOCKED" in orders[2].rejection_reason

    # Order 3: Attempted SELL on bar 5 (2024-02-19, Reopen Day 3 T+3 Open) -> FILLED!
    assert orders[3].status == "FILLED"
    assert orders[3].side == "SELL"


# ===========================================================================
# 2. Multi-Lot FIFO Allocation Stress
# ===========================================================================


def test_adv_multi_lot_fifo_settlement_stress():
    """Verify discrete multi-lot FIFO allocation and consumption rules in SettlementEngine.

    Scenarios:
    - Lot A: 100 shares bought on Day 1 (Fri 2024-03-01)
    - Lot B: 200 shares bought on Day 2 (Mon 2024-03-04)
    - Lot C: 300 shares bought on Day 3 (Tue 2024-03-05)

    Lifecycle:
    - Day 3 Close:
      - Lot A (100) is sellable.
      - Lot B (200) and Lot C (300) are locked.
      - Selling 100 shares: completely consumes Lot A. Remaining: Lot B (200), Lot C (300).
      - Selling 600 shares: only 100 shares sold; remaining 500 shares remain in lots.
    - Day 4 Close:
      - Lot A (100) + Lot B (200) are sellable (total 300). Lot C (300) remains locked.
      - Selling 250 shares: consumes all 100 of Lot A + 150 of Lot B. Remaining: Lot B (50), Lot C (300).
    - Day 5 Close:
      - All lots (A, B, C) are sellable.
    """
    cal = TradingCalendar()
    engine = SettlementEngine(cal, conservative_mode=True)

    d1 = date(2024, 3, 1)  # Fri
    d2 = date(2024, 3, 4)  # Mon
    d3 = date(2024, 3, 5)  # Tue
    d4 = date(2024, 3, 6)  # Wed
    d5 = date(2024, 3, 7)  # Thu

    def make_fresh_lots():
        return [
            HoldingLot(quantity=100.0, acquisition_date=d1, price=10.0, lot_id="lot_a"),
            HoldingLot(quantity=200.0, acquisition_date=d2, price=11.0, lot_id="lot_b"),
            HoldingLot(quantity=300.0, acquisition_date=d3, price=12.0, lot_id="lot_c"),
        ]

    # --- Day 3 (Tue 2024-03-05) ---
    lots = make_fresh_lots()
    # At Open: all locked
    assert engine.get_sellable_quantity(lots, d3, "open") == 0.0

    # At Close: only Lot A is sellable
    assert engine.get_sellable_quantity(lots, d3, "close") == 100.0

    # Attempt to sell 600 (Lot A + B + C): only Lot A should be sold
    sold_qty, remaining = engine.allocate_sell(lots, 600.0, d3, "close")
    assert sold_qty == 100.0
    assert len(remaining) == 2
    assert remaining[0].lot_id == "lot_b" and remaining[0].quantity == 200.0
    assert remaining[1].lot_id == "lot_c" and remaining[1].quantity == 300.0

    # Attempt to sell exact Lot A (100.0)
    lots = make_fresh_lots()
    sold_qty, remaining = engine.allocate_sell(lots, 100.0, d3, "close")
    assert sold_qty == 100.0
    assert len(remaining) == 2
    assert remaining[0].lot_id == "lot_b" and remaining[0].quantity == 200.0
    assert remaining[1].lot_id == "lot_c" and remaining[1].quantity == 300.0

    # --- Day 4 (Wed 2024-03-06) ---
    lots = make_fresh_lots()
    # At Open: Lot A is T+3 (sellable), Lot B is T+2 (locked at open), Lot C is T+1 (locked)
    assert engine.get_sellable_quantity(lots, d4, "open") == 100.0
    # At Close: Lot A (100) + Lot B (200) = 300.0 sellable
    assert engine.get_sellable_quantity(lots, d4, "close") == 300.0

    # FIFO partial consumption: sell 250 shares
    sold_qty, remaining = engine.allocate_sell(lots, 250.0, d4, "close")
    assert sold_qty == 250.0
    assert len(remaining) == 2
    # Lot A consumed, Lot B has 50 left, Lot C intact
    assert remaining[0].lot_id == "lot_b" and remaining[0].quantity == 50.0
    assert remaining[1].lot_id == "lot_c" and remaining[1].quantity == 300.0

    # --- Day 5 (Thu 2024-03-07) ---
    lots = make_fresh_lots()
    # At Close: all 600 shares sellable
    assert engine.get_sellable_quantity(lots, d5, "close") == 600.0
    sold_qty, remaining = engine.allocate_sell(lots, 600.0, d5, "close")
    assert sold_qty == 600.0
    assert len(remaining) == 0


def test_adv_practice_workflow_multi_lot_fifo_lifecycle_stress(isolated_db: Session):
    """Stress-test multi-lot buying and selling across sequential sessions in Practice/Replay workflow."""
    db = isolated_db

    # Candle setup: 5 days
    candles = [
        Candle(symbol="FPT", timeframe="1D", timestamp=datetime(2024, 3, 1), open=100.0, high=102.0, low=99.0, close=101.0, volume=1000),
        Candle(symbol="FPT", timeframe="1D", timestamp=datetime(2024, 3, 4), open=101.0, high=103.0, low=100.0, close=102.0, volume=1000),
        Candle(symbol="FPT", timeframe="1D", timestamp=datetime(2024, 3, 5), open=102.0, high=104.0, low=101.0, close=103.0, volume=1000),
        Candle(symbol="FPT", timeframe="1D", timestamp=datetime(2024, 3, 6), open=103.0, high=105.0, low=102.0, close=104.0, volume=1000),
        Candle(symbol="FPT", timeframe="1D", timestamp=datetime(2024, 3, 7), open=104.0, high=106.0, low=103.0, close=105.0, volume=1000),
    ]
    db.add_all(candles)
    db.commit()

    session = ReplaySession(
        symbol="FPT",
        timeframe="1D",
        start_date=datetime(2024, 3, 1),
        end_date=datetime(2024, 3, 7),
        current_index=2,  # Day 3 (2024-03-05)
        initial_cash=100_000_000.0,
        current_cash=30_000_000.0,
    )
    db.add(session)
    db.commit()

    # Buy Lot A: 100 shares on Day 1 (bar 0)
    d1 = Decision(session_id=session.id, symbol="FPT", action="BUY", candle_index=0, decision_date=datetime(2024, 3, 1), price=100.0, planned_quantity=100)
    db.add(d1); db.commit()
    o1 = Order(session_id=session.id, decision_id=d1.id, symbol="FPT", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(o1); db.commit()
    e1 = Execution(session_id=session.id, order_id=o1.id, symbol="FPT", execution_candle_index=0, execution_date=datetime(2024, 3, 1), execution_price=100.0, quantity=100, gross_amount=10000.0, net_amount=10015.0)
    db.add(e1); db.commit()

    # Buy Lot B: 200 shares on Day 2 (bar 1)
    d2 = Decision(session_id=session.id, symbol="FPT", action="BUY", candle_index=1, decision_date=datetime(2024, 3, 4), price=101.0, planned_quantity=200)
    db.add(d2); db.commit()
    o2 = Order(session_id=session.id, decision_id=d2.id, symbol="FPT", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=200, status="executed")
    db.add(o2); db.commit()
    e2 = Execution(session_id=session.id, order_id=o2.id, symbol="FPT", execution_candle_index=1, execution_date=datetime(2024, 3, 4), execution_price=101.0, quantity=200, gross_amount=20200.0, net_amount=20230.3)
    db.add(e2); db.commit()

    # Buy Lot C: 300 shares on Day 3 (bar 2)
    d3 = Decision(session_id=session.id, symbol="FPT", action="BUY", candle_index=2, decision_date=datetime(2024, 3, 5), price=102.0, planned_quantity=300)
    db.add(d3); db.commit()
    o3 = Order(session_id=session.id, decision_id=d3.id, symbol="FPT", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=300, status="executed")
    db.add(o3); db.commit()
    e3 = Execution(session_id=session.id, order_id=o3.id, symbol="FPT", execution_candle_index=2, execution_date=datetime(2024, 3, 5), execution_price=102.0, quantity=300, gross_amount=30600.0, net_amount=30645.9)
    db.add(e3); db.commit()

    pos = Position(session_id=session.id, symbol="FPT", quantity=600.0, average_price=101.33, total_cost=60800.0, opened_at=datetime(2024, 3, 1), status="open")
    db.add(pos)
    db.commit()

    # --- Phase 1: On Day 3 (bar 2) ---
    snap = PracticeWorkflowService.get_snapshot(db, session.id)
    assert snap.available_quantity == 100.0
    assert snap.blocked_quantity == 500.0
    assert snap.settlement_label == "T+1 · về sau 1 phiên"
    assert snap.earliest_release_date.date() == date(2024, 3, 6)

    # Attempt to sell 600 shares (Lot A + B + C): MUST REJECT with 400
    dec_sell_600 = DecisionCreate(action="SELL", candle_index=2, price=103.0, quantity=600)
    with pytest.raises(HTTPException) as exc_info:
        TradeLifecycleService.process_decision(db, session.id, dec_sell_600)
    assert exc_info.value.status_code == 400
    detail = exc_info.value.detail
    assert detail["error_code"] == "LOT_SETTLEMENT_LOCKED"
    assert detail["status"] == "REJECTED_SETTLEMENT_LOCK"
    assert detail["settlement_label"] == "T+1 · về sau 1 phiên"
    assert detail["earliest_release_date"].startswith("2024-03-06")

    # Attempt to sell 100 shares (Lot A): MUST SUCCEED
    dec_sell_100 = DecisionCreate(action="SELL", candle_index=2, price=103.0, quantity=100)
    TradeLifecycleService.process_decision(db, session.id, dec_sell_100)
    db.refresh(pos)
    assert pos.quantity == 500.0

    # Attempt to sell another 100 shares on Day 3: MUST REJECT because available is 0
    with pytest.raises(HTTPException) as exc_info2:
        TradeLifecycleService.process_decision(db, session.id, dec_sell_100)
    assert exc_info2.value.status_code == 400
    detail2 = exc_info2.value.detail
    available_val = detail2.get("available_quantity", detail2.get("available_qty"))
    blocked_val = detail2.get("blocked_quantity", detail2.get("blocked_qty"))
    assert available_val == 0.0
    assert blocked_val == 500.0

    # --- Phase 2: Advance to Day 4 (bar 3) ---
    session.current_index = 3
    db.commit()

    snap_d4 = PracticeWorkflowService.get_snapshot(db, session.id)
    assert snap_d4.available_quantity == 200.0  # Lot B now available
    assert snap_d4.blocked_quantity == 300.0    # Lot C still locked
    assert snap_d4.earliest_release_date.date() == date(2024, 3, 7)

    # Sell 200 shares (Lot B): MUST SUCCEED
    dec_sell_200 = DecisionCreate(action="SELL", candle_index=3, price=104.0, quantity=200)
    TradeLifecycleService.process_decision(db, session.id, dec_sell_200)
    db.refresh(pos)
    assert pos.quantity == 300.0

    # --- Phase 3: Advance to Day 5 (bar 4) ---
    session.current_index = 4
    db.commit()

    snap_d5 = PracticeWorkflowService.get_snapshot(db, session.id)
    assert snap_d5.available_quantity == 300.0  # Lot C now available
    assert snap_d5.blocked_quantity == 0.0      # Zero blocked
    assert snap_d5.earliest_release_date is None

    # Sell remaining 300 shares (Lot C): MUST SUCCEED and close position
    dec_sell_300 = DecisionCreate(action="SELL", candle_index=4, price=105.0, quantity=300)
    TradeLifecycleService.process_decision(db, session.id, dec_sell_300)
    db.refresh(pos)
    assert pos.quantity == 0.0
    assert pos.status == "closed"


# ===========================================================================
# 3. Payload Verification
# ===========================================================================


def test_adv_payload_verification_exact_fields(isolated_db: Session):
    """Verify rejection and snapshot payload fields conform to the contract."""
    db = isolated_db

    c1 = Candle(symbol="TCB", timeframe="1D", timestamp=datetime(2024, 4, 1), open=40.0, high=41.0, low=39.5, close=40.5, volume=5000)
    c2 = Candle(symbol="TCB", timeframe="1D", timestamp=datetime(2024, 4, 2), open=40.5, high=41.5, low=40.0, close=41.0, volume=6000)
    c3 = Candle(symbol="TCB", timeframe="1D", timestamp=datetime(2024, 4, 3), open=41.0, high=42.0, low=40.5, close=41.5, volume=7000)
    db.add_all([c1, c2, c3])
    db.commit()

    session = ReplaySession(
        symbol="TCB",
        timeframe="1D",
        start_date=datetime(2024, 4, 1),
        end_date=datetime(2024, 4, 3),
        current_index=1,  # Day 2 (T+1)
        initial_cash=50_000_000.0,
        current_cash=45_000_000.0,
    )
    db.add(session); db.commit()

    # Buy on Day 1 (bar 0)
    dec = Decision(session_id=session.id, symbol="TCB", action="BUY", candle_index=0, decision_date=datetime(2024, 4, 1), price=40.0, planned_quantity=100)
    db.add(dec); db.commit()
    order = Order(session_id=session.id, decision_id=dec.id, symbol="TCB", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(order); db.commit()
    exc = Execution(session_id=session.id, order_id=order.id, symbol="TCB", execution_candle_index=0, execution_date=datetime(2024, 4, 1), execution_price=40.0, quantity=100, gross_amount=4000.0, net_amount=4006.0)
    db.add(exc); db.commit()
    pos = Position(session_id=session.id, symbol="TCB", quantity=100.0, average_price=40.0, total_cost=4000.0, opened_at=datetime(2024, 4, 1), status="open")
    db.add(pos); db.commit()

    # Snapshot contract verification
    snapshot = PracticeWorkflowService.get_snapshot(db, session.id)
    assert hasattr(snapshot, "earliest_release_date")
    assert hasattr(snapshot, "blocked_quantity")
    assert hasattr(snapshot, "available_quantity")
    assert hasattr(snapshot, "settlement_label")

    assert snapshot.settlement_label == "T+1 · về sau 1 phiên"
    assert snapshot.blocked_quantity == 100.0
    assert snapshot.available_quantity == 0.0
    assert snapshot.earliest_release_date is not None
    assert snapshot.earliest_release_date.date() == date(2024, 4, 3)

    # TradeLifecycle rejection payload verification
    dec_sell = DecisionCreate(action="SELL", candle_index=1, price=41.0, quantity=100)
    with pytest.raises(HTTPException) as exc_info:
        TradeLifecycleService.process_decision(db, session.id, dec_sell)

    assert exc_info.value.status_code == 400
    detail = exc_info.value.detail
    assert isinstance(detail, dict)

    # Mandatory contract fields
    assert detail["error_code"] == "LOT_SETTLEMENT_LOCKED"
    assert detail["status"] == "REJECTED_SETTLEMENT_LOCK"
    assert detail["settlement_label"] == "T+1 · về sau 1 phiên"
    assert "earliest_release_date" in detail
    assert detail["earliest_release_date"].startswith("2024-04-03")

    # Quantities: verify values regardless of whether named _quantity or _qty
    avail_val = detail.get("available_quantity") if "available_quantity" in detail else detail.get("available_qty")
    blocked_val = detail.get("blocked_quantity") if "blocked_quantity" in detail else detail.get("blocked_qty")
    assert avail_val == 0.0
    assert blocked_val == 100.0


# ===========================================================================
# 4. Boundary and Invariance Guarantees
# ===========================================================================


def test_adv_boundary_extreme_cases():
    """Stress-test boundary parameters and extreme inputs in SettlementEngine and TradingCalendar."""
    cal = TradingCalendar()
    engine = SettlementEngine(cal)

    # 1. Trading days between identical dates is 0
    d = date(2024, 3, 1)
    assert cal.trading_days_between(d, d) == 0

    # 2. Inverted dates produce negative count
    assert cal.trading_days_between(date(2024, 3, 5), date(2024, 3, 1)) == -cal.trading_days_between(date(2024, 3, 1), date(2024, 3, 5))

    # 3. Add negative trading days moves backward
    assert cal.add_trading_days(date(2024, 3, 5), -2) == date(2024, 3, 1)

    # 4. Weekend date passed to is_trading_day
    assert cal.is_trading_day(date(2024, 3, 2)) is False  # Saturday
    assert cal.is_trading_day(date(2024, 3, 3)) is False  # Sunday

    # 5. Empty lot list returns 0.0 sellable
    assert engine.get_sellable_quantity([], date(2024, 3, 5), "close") == 0.0
    sold, rem = engine.allocate_sell([], 100.0, date(2024, 3, 5), "close")
    assert sold == 0.0 and len(rem) == 0

    # 6. Negative or zero sell quantity returns 0 sold
    lot = HoldingLot(quantity=100.0, acquisition_date=date(2024, 3, 1))
    sold, rem = engine.allocate_sell([lot], 0.0, date(2024, 3, 5), "close")
    assert sold == 0.0 and len(rem) == 1 and rem[0].quantity == 100.0


# ===========================================================================
# 5. Targeted Challenger Probes for Discovered Defects
# ===========================================================================


def test_adv_payload_strict_quantity_keys(isolated_db: Session):
    """Verify whether TradeLifecycleService HTTPException detail contains exact 'blocked_quantity' and 'available_quantity' keys."""
    db = isolated_db

    c1 = Candle(symbol="TCB", timeframe="1D", timestamp=datetime(2024, 4, 1), open=40.0, high=41.0, low=39.5, close=40.5, volume=5000)
    c2 = Candle(symbol="TCB", timeframe="1D", timestamp=datetime(2024, 4, 2), open=40.5, high=41.5, low=40.0, close=41.0, volume=6000)
    db.add_all([c1, c2]); db.commit()

    session = ReplaySession(
        symbol="TCB",
        timeframe="1D",
        start_date=datetime(2024, 4, 1),
        end_date=datetime(2024, 4, 2),
        current_index=1,
        initial_cash=50_000_000.0,
        current_cash=45_000_000.0,
    )
    db.add(session); db.commit()

    dec = Decision(session_id=session.id, symbol="TCB", action="BUY", candle_index=0, decision_date=datetime(2024, 4, 1), price=40.0, planned_quantity=100)
    db.add(dec); db.commit()
    order = Order(session_id=session.id, decision_id=dec.id, symbol="TCB", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(order); db.commit()
    exc = Execution(session_id=session.id, order_id=order.id, symbol="TCB", execution_candle_index=0, execution_date=datetime(2024, 4, 1), execution_price=40.0, quantity=100, gross_amount=4000.0, net_amount=4006.0)
    db.add(exc); db.commit()
    pos = Position(session_id=session.id, symbol="TCB", quantity=100.0, average_price=40.0, total_cost=4000.0, opened_at=datetime(2024, 4, 1), status="open")
    db.add(pos); db.commit()

    dec_sell = DecisionCreate(action="SELL", candle_index=1, price=41.0, quantity=100)
    with pytest.raises(HTTPException) as exc_info:
        TradeLifecycleService.process_decision(db, session.id, dec_sell)

    detail = exc_info.value.detail
    # Challenger probe: strictly verifies exact schema keys requested by prompt
    assert "blocked_quantity" in detail, f"Schema mismatch: expected 'blocked_quantity' in detail, but only found keys {list(detail.keys())}"
    assert "available_quantity" in detail, f"Schema mismatch: expected 'available_quantity' in detail, but only found keys {list(detail.keys())}"


def test_adv_horizon_clamping_release_date_none_when_beyond_end_date(isolated_db: Session):
    """Probe whether earliest_release_date in PracticeWorkflowSnapshot is clamped to None when release falls beyond session.end_date."""
    db = isolated_db
    symbol = "CLAMP_ADV"
    start_d = datetime(2023, 1, 1)

    # 10 daily candles
    candles = [
        Candle(symbol=symbol, timeframe="1D", timestamp=start_d + timedelta(days=i), open=100.0 + i, high=102.0 + i, low=99.0 + i, close=101.0 + i, volume=1000)
        for i in range(10)
    ]
    db.add_all(candles); db.commit()

    # Session ends on candle 4 (2023-01-05)
    session = ReplaySession(
        symbol=symbol,
        timeframe="1D",
        start_date=start_d,
        end_date=start_d + timedelta(days=4),
        current_index=4,  # Final candle of the session
        initial_cash=100_000_000.0,
        current_cash=90_000_000.0,
    )
    db.add(session); db.commit()

    # Buy on final candle 4 (2023-01-05). Release bar would be bar 6 (2023-01-07), which is > session.end_date (2023-01-05)
    dec = Decision(session_id=session.id, symbol=symbol, action="BUY", candle_index=4, decision_date=start_d + timedelta(days=4), price=104.0, planned_quantity=100)
    db.add(dec); db.commit()
    order = Order(session_id=session.id, decision_id=dec.id, symbol=symbol, side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(order); db.commit()
    exc = Execution(session_id=session.id, order_id=order.id, symbol=symbol, execution_candle_index=4, execution_date=start_d + timedelta(days=4), execution_price=104.0, quantity=100, gross_amount=10400.0, net_amount=10415.6)
    db.add(exc); db.commit()
    pos = Position(session_id=session.id, symbol=symbol, quantity=100.0, average_price=104.0, total_cost=10400.0, opened_at=start_d + timedelta(days=4), status="open")
    db.add(pos); db.commit()

    snap = PracticeWorkflowService.get_snapshot(db, session.id)
    # Per B10 / Train 02 anti-lookahead horizon specification, earliest_release_date must be None when beyond session.end_date
    assert snap.earliest_release_date is None, (
        f"Horizon clamping failure: earliest_release_date was {snap.earliest_release_date}, "
        f"which leaks beyond session.end_date {session.end_date}"
    )


def test_adv_horizon_clamping_multi_session_lengths(isolated_db: Session):
    """Stress-test horizon clamping across 1-day, 2-day, 5-day, and 120-day replay sessions.

    Invariants enforced:
    - Buying on the final candle of any session MUST yield earliest_release_date = None.
    - Buying when release falls outside the session horizon or candle timeline MUST yield None.
    - Buying when release falls strictly inside the session horizon and candle timeline MUST yield valid release datetime.
    """
    db = isolated_db
    cal = TradingCalendar()

    # 1. 1-day session (N = 1 candle): Buy on final (and only) bar 0
    c_1d = Candle(symbol="CLAMP_1D", timeframe="1D", timestamp=datetime(2024, 1, 2), open=10.0, high=11.0, low=9.0, close=10.5, volume=100)
    db.add(c_1d); db.commit()
    s_1d = ReplaySession(symbol="CLAMP_1D", timeframe="1D", start_date=date(2024, 1, 2), end_date=date(2024, 1, 2), current_index=0, initial_cash=10_000_000, current_cash=9_000_000)
    db.add(s_1d); db.commit()
    dec = Decision(session_id=s_1d.id, symbol="CLAMP_1D", action="BUY", candle_index=0, decision_date=datetime(2024, 1, 2), price=10.0, planned_quantity=100)
    db.add(dec); db.commit()
    order = Order(session_id=s_1d.id, decision_id=dec.id, symbol="CLAMP_1D", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(order); db.commit()
    exc = Execution(session_id=s_1d.id, order_id=order.id, symbol="CLAMP_1D", execution_candle_index=0, execution_date=datetime(2024, 1, 2), execution_price=10.0, quantity=100, gross_amount=1000.0, net_amount=1001.5)
    db.add(exc); db.commit()
    pos = Position(session_id=s_1d.id, symbol="CLAMP_1D", quantity=100.0, average_price=10.0, total_cost=1000.0, opened_at=datetime(2024, 1, 2), status="open")
    db.add(pos); db.commit()

    snap_1d = PracticeWorkflowService.get_snapshot(db, s_1d.id)
    assert snap_1d.earliest_release_date is None, "1-day session buy on bar 0 must clamp earliest_release_date to None"
    assert snap_1d.positions[0].earliest_release_date is None

    # 2. 2-day session (N = 2 candles: Tue 2024-01-02, Wed 2024-01-03)
    c_2d_1 = Candle(symbol="CLAMP_2D", timeframe="1D", timestamp=datetime(2024, 1, 2), open=10.0, high=11.0, low=9.0, close=10.5, volume=100)
    c_2d_2 = Candle(symbol="CLAMP_2D", timeframe="1D", timestamp=datetime(2024, 1, 3), open=10.5, high=11.5, low=10.0, close=11.0, volume=100)
    db.add_all([c_2d_1, c_2d_2]); db.commit()
    s_2d = ReplaySession(symbol="CLAMP_2D", timeframe="1D", start_date=date(2024, 1, 2), end_date=date(2024, 1, 3), current_index=1, initial_cash=10_000_000, current_cash=9_000_000)
    db.add(s_2d); db.commit()
    dec = Decision(session_id=s_2d.id, symbol="CLAMP_2D", action="BUY", candle_index=1, decision_date=datetime(2024, 1, 3), price=11.0, planned_quantity=100)
    db.add(dec); db.commit()
    order = Order(session_id=s_2d.id, decision_id=dec.id, symbol="CLAMP_2D", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(order); db.commit()
    exc = Execution(session_id=s_2d.id, order_id=order.id, symbol="CLAMP_2D", execution_candle_index=1, execution_date=datetime(2024, 1, 3), execution_price=11.0, quantity=100, gross_amount=1100.0, net_amount=1101.65)
    db.add(exc); db.commit()
    pos = Position(session_id=s_2d.id, symbol="CLAMP_2D", quantity=100.0, average_price=11.0, total_cost=1100.0, opened_at=datetime(2024, 1, 3), status="open")
    db.add(pos); db.commit()

    snap_2d = PracticeWorkflowService.get_snapshot(db, s_2d.id)
    assert snap_2d.earliest_release_date is None, "2-day session buy on final bar 1 must clamp earliest_release_date to None"
    assert snap_2d.positions[0].earliest_release_date is None

    # 3. 5-day session (Mon 2024-03-04 to Fri 2024-03-08)
    candles_5d = [
        Candle(symbol="CLAMP_5D", timeframe="1D", timestamp=datetime(2024, 3, 4 + i), open=20.0 + i, high=21.0 + i, low=19.0 + i, close=20.5 + i, volume=500)
        for i in range(5)
    ]
    db.add_all(candles_5d); db.commit()

    # 3a. Buy on bar 0 (Mon 2024-03-04): Release date is Wed 2024-03-06 (within 5-day session)
    s_5d_0 = ReplaySession(symbol="CLAMP_5D", timeframe="1D", start_date=date(2024, 3, 4), end_date=date(2024, 3, 8), current_index=0, initial_cash=10_000_000, current_cash=8_000_000)
    db.add(s_5d_0); db.commit()
    dec = Decision(session_id=s_5d_0.id, symbol="CLAMP_5D", action="BUY", candle_index=0, decision_date=datetime(2024, 3, 4), price=20.0, planned_quantity=100)
    db.add(dec); db.commit()
    order = Order(session_id=s_5d_0.id, decision_id=dec.id, symbol="CLAMP_5D", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(order); db.commit()
    exc = Execution(session_id=s_5d_0.id, order_id=order.id, symbol="CLAMP_5D", execution_candle_index=0, execution_date=datetime(2024, 3, 4), execution_price=20.0, quantity=100, gross_amount=2000.0, net_amount=2003.0)
    db.add(exc); db.commit()
    pos = Position(session_id=s_5d_0.id, symbol="CLAMP_5D", quantity=100.0, average_price=20.0, total_cost=2000.0, opened_at=datetime(2024, 3, 4), status="open")
    db.add(pos); db.commit()

    snap_5d_0 = PracticeWorkflowService.get_snapshot(db, s_5d_0.id)
    assert snap_5d_0.earliest_release_date == datetime(2024, 3, 6, 0, 0), "Buy on Monday of 5-day session must have release date Wednesday within horizon"
    assert snap_5d_0.positions[0].earliest_release_date == datetime(2024, 3, 6, 0, 0)

    # 3b. Buy on bar 4 (Fri 2024-03-08, final bar of 5-day session): Release date Tue 2024-03-12 (past horizon)
    s_5d_4 = ReplaySession(symbol="CLAMP_5D", timeframe="1D", start_date=date(2024, 3, 4), end_date=date(2024, 3, 8), current_index=4, initial_cash=10_000_000, current_cash=8_000_000)
    db.add(s_5d_4); db.commit()
    dec = Decision(session_id=s_5d_4.id, symbol="CLAMP_5D", action="BUY", candle_index=4, decision_date=datetime(2024, 3, 8), price=24.0, planned_quantity=100)
    db.add(dec); db.commit()
    order = Order(session_id=s_5d_4.id, decision_id=dec.id, symbol="CLAMP_5D", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(order); db.commit()
    exc = Execution(session_id=s_5d_4.id, order_id=order.id, symbol="CLAMP_5D", execution_candle_index=4, execution_date=datetime(2024, 3, 8), execution_price=24.0, quantity=100, gross_amount=2400.0, net_amount=2403.6)
    db.add(exc); db.commit()
    pos = Position(session_id=s_5d_4.id, symbol="CLAMP_5D", quantity=100.0, average_price=24.0, total_cost=2400.0, opened_at=datetime(2024, 3, 8), status="open")
    db.add(pos); db.commit()

    snap_5d_4 = PracticeWorkflowService.get_snapshot(db, s_5d_4.id)
    assert snap_5d_4.earliest_release_date is None, "Buy on Friday (final bar) of 5-day session must clamp earliest_release_date to None"
    assert snap_5d_4.positions[0].earliest_release_date is None

    # 4. 120-day session
    d_cur = date(2023, 1, 3)
    trading_days = []
    while len(trading_days) < 125:
        if cal.is_trading_day(d_cur):
            trading_days.append(d_cur)
        d_cur = d_cur + timedelta(days=1)

    candles_120 = [
        Candle(
            symbol="CLAMP_120D",
            timeframe="1D",
            timestamp=datetime.combine(td, datetime.min.time()),
            open=50.0 + i * 0.1,
            high=51.0 + i * 0.1,
            low=49.0 + i * 0.1,
            close=50.5 + i * 0.1,
            volume=10000,
        )
        for i, td in enumerate(trading_days[:120])
    ]
    db.add_all(candles_120); db.commit()

    # 4a. Buy on final bar 119
    s_120_119 = ReplaySession(symbol="CLAMP_120D", timeframe="1D", start_date=trading_days[0], end_date=trading_days[119], current_index=119, initial_cash=100_000_000.0, current_cash=90_000_000.0)
    db.add(s_120_119); db.commit()
    dec = Decision(session_id=s_120_119.id, symbol="CLAMP_120D", action="BUY", candle_index=119, decision_date=datetime.combine(trading_days[119], datetime.min.time()), price=60.0, planned_quantity=100)
    db.add(dec); db.commit()
    order = Order(session_id=s_120_119.id, decision_id=dec.id, symbol="CLAMP_120D", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(order); db.commit()
    exc = Execution(session_id=s_120_119.id, order_id=order.id, symbol="CLAMP_120D", execution_candle_index=119, execution_date=datetime.combine(trading_days[119], datetime.min.time()), execution_price=60.0, quantity=100, gross_amount=6000.0, net_amount=6009.0)
    db.add(exc); db.commit()
    pos = Position(session_id=s_120_119.id, symbol="CLAMP_120D", quantity=100.0, average_price=60.0, total_cost=6000.0, opened_at=datetime.combine(trading_days[119], datetime.min.time()), status="open")
    db.add(pos); db.commit()

    snap_120_119 = PracticeWorkflowService.get_snapshot(db, s_120_119.id)
    assert snap_120_119.earliest_release_date is None, "120-day session buy on final bar 119 must clamp earliest_release_date to None"
    assert snap_120_119.positions[0].earliest_release_date is None

    # 4b. Buy on penultimate bar 118 (release is 2 trading days later, which is bar 120, outside session)
    s_120_118 = ReplaySession(symbol="CLAMP_120D", timeframe="1D", start_date=trading_days[0], end_date=trading_days[119], current_index=118, initial_cash=100_000_000.0, current_cash=90_000_000.0)
    db.add(s_120_118); db.commit()
    dec = Decision(session_id=s_120_118.id, symbol="CLAMP_120D", action="BUY", candle_index=118, decision_date=datetime.combine(trading_days[118], datetime.min.time()), price=60.0, planned_quantity=100)
    db.add(dec); db.commit()
    order = Order(session_id=s_120_118.id, decision_id=dec.id, symbol="CLAMP_120D", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(order); db.commit()
    exc = Execution(session_id=s_120_118.id, order_id=order.id, symbol="CLAMP_120D", execution_candle_index=118, execution_date=datetime.combine(trading_days[118], datetime.min.time()), execution_price=60.0, quantity=100, gross_amount=6000.0, net_amount=6009.0)
    db.add(exc); db.commit()
    pos = Position(session_id=s_120_118.id, symbol="CLAMP_120D", quantity=100.0, average_price=60.0, total_cost=6000.0, opened_at=datetime.combine(trading_days[118], datetime.min.time()), status="open")
    db.add(pos); db.commit()

    snap_120_118 = PracticeWorkflowService.get_snapshot(db, s_120_118.id)
    assert snap_120_118.earliest_release_date is None, "120-day session buy on bar 118 must clamp earliest_release_date to None"
    assert snap_120_118.positions[0].earliest_release_date is None

    # 4c. Buy on bar 117 (release is bar 119, which is inside session)
    s_120_117 = ReplaySession(symbol="CLAMP_120D", timeframe="1D", start_date=trading_days[0], end_date=trading_days[119], current_index=117, initial_cash=100_000_000.0, current_cash=90_000_000.0)
    db.add(s_120_117); db.commit()
    dec = Decision(session_id=s_120_117.id, symbol="CLAMP_120D", action="BUY", candle_index=117, decision_date=datetime.combine(trading_days[117], datetime.min.time()), price=60.0, planned_quantity=100)
    db.add(dec); db.commit()
    order = Order(session_id=s_120_117.id, decision_id=dec.id, symbol="CLAMP_120D", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(order); db.commit()
    exc = Execution(session_id=s_120_117.id, order_id=order.id, symbol="CLAMP_120D", execution_candle_index=117, execution_date=datetime.combine(trading_days[117], datetime.min.time()), execution_price=60.0, quantity=100, gross_amount=6000.0, net_amount=6009.0)
    db.add(exc); db.commit()
    pos = Position(session_id=s_120_117.id, symbol="CLAMP_120D", quantity=100.0, average_price=60.0, total_cost=6000.0, opened_at=datetime.combine(trading_days[117], datetime.min.time()), status="open")
    db.add(pos); db.commit()

    snap_120_117 = PracticeWorkflowService.get_snapshot(db, s_120_117.id)
    assert snap_120_117.earliest_release_date == datetime.combine(trading_days[119], datetime.min.time()), "Buy on bar 117 must resolve to candle 119 release datetime within horizon"
    assert snap_120_117.positions[0].earliest_release_date == datetime.combine(trading_days[119], datetime.min.time())


def test_adv_horizon_clamping_asymmetric_boundaries(isolated_db: Session):
    """Stress-test asymmetric boundaries between session.end_date and candle availability.

    Case A: rel_d > max_candle_date but rel_d <= session.end_date (truncated candle history).
    Case B: rel_d <= max_candle_date but rel_d > session.end_date (narrow session window in wide dataset).
    In both cases, earliest_release_date MUST be strictly None!
    """
    db = isolated_db

    # Case A: Truncated candle history
    # Session allows up to Jan 31, but database only has 2 candles (Jan 2, Jan 3)
    c1 = Candle(symbol="ASYMM_A", timeframe="1D", timestamp=datetime(2024, 1, 2), open=10.0, high=11.0, low=9.0, close=10.5, volume=100)
    c2 = Candle(symbol="ASYMM_A", timeframe="1D", timestamp=datetime(2024, 1, 3), open=10.5, high=11.5, low=10.0, close=11.0, volume=100)
    db.add_all([c1, c2]); db.commit()

    s_a = ReplaySession(symbol="ASYMM_A", timeframe="1D", start_date=date(2024, 1, 2), end_date=date(2024, 1, 31), current_index=1, initial_cash=10_000_000, current_cash=9_000_000)
    db.add(s_a); db.commit()

    dec_a = Decision(session_id=s_a.id, symbol="ASYMM_A", action="BUY", candle_index=1, decision_date=datetime(2024, 1, 3), price=11.0, planned_quantity=100)
    db.add(dec_a); db.commit()
    order_a = Order(session_id=s_a.id, decision_id=dec_a.id, symbol="ASYMM_A", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(order_a); db.commit()
    exc_a = Execution(session_id=s_a.id, order_id=order_a.id, symbol="ASYMM_A", execution_candle_index=1, execution_date=datetime(2024, 1, 3), execution_price=11.0, quantity=100, gross_amount=1100.0, net_amount=1101.65)
    db.add(exc_a); db.commit()
    pos_a = Position(session_id=s_a.id, symbol="ASYMM_A", quantity=100.0, average_price=11.0, total_cost=1100.0, opened_at=datetime(2024, 1, 3), status="open")
    db.add(pos_a); db.commit()

    snap_a = PracticeWorkflowService.get_snapshot(db, s_a.id)
    # rel_d = 2024-01-05 <= session.end_date (2024-01-31), but rel_d > candles[-1].date() (2024-01-03)
    assert snap_a.earliest_release_date is None, "When rel_d > max_candle_date, earliest_release_date must be None"
    assert snap_a.positions[0].earliest_release_date is None

    # Case B: Narrow session window in wide dataset
    # 10 candles exist (Jan 2 to Jan 15), but session is clamped to end on Jan 3
    start_d = datetime(2024, 1, 2)
    candles_b = [
        Candle(symbol="ASYMM_B", timeframe="1D", timestamp=start_d + timedelta(days=i), open=10.0 + i, high=11.0 + i, low=9.0 + i, close=10.5 + i, volume=100)
        for i in range(10)
    ]
    db.add_all(candles_b); db.commit()

    s_b = ReplaySession(symbol="ASYMM_B", timeframe="1D", start_date=date(2024, 1, 2), end_date=date(2024, 1, 3), current_index=1, initial_cash=10_000_000, current_cash=9_000_000)
    db.add(s_b); db.commit()

    dec_b = Decision(session_id=s_b.id, symbol="ASYMM_B", action="BUY", candle_index=1, decision_date=datetime(2024, 1, 3), price=11.0, planned_quantity=100)
    db.add(dec_b); db.commit()
    order_b = Order(session_id=s_b.id, decision_id=dec_b.id, symbol="ASYMM_B", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(order_b); db.commit()
    exc_b = Execution(session_id=s_b.id, order_id=order_b.id, symbol="ASYMM_B", execution_candle_index=1, execution_date=datetime(2024, 1, 3), execution_price=11.0, quantity=100, gross_amount=1100.0, net_amount=1101.65)
    db.add(exc_b); db.commit()
    pos_b = Position(session_id=s_b.id, symbol="ASYMM_B", quantity=100.0, average_price=11.0, total_cost=1100.0, opened_at=datetime(2024, 1, 3), status="open")
    db.add(pos_b); db.commit()

    snap_b = PracticeWorkflowService.get_snapshot(db, s_b.id)
    # rel_d = 2024-01-05 <= candles[-1].date() (2024-01-11), but rel_d > session.end_date (2024-01-03)
    assert snap_b.earliest_release_date is None, "When rel_d > session.end_date, earliest_release_date must be None"
    assert snap_b.positions[0].earliest_release_date is None


def test_adv_horizon_clamping_tet_holiday_boundary(isolated_db: Session):
    """Stress-test horizon clamping across 2024 Tết holiday boundary.

    Buying on Wednesday 2024-02-07 (last trading day before Tết).
    Reopen day 1: 2024-02-15 (Thu) -> LOCKED
    Reopen day 2: 2024-02-16 (Fri) -> TRADABLE (release date)

    If session ends on 2024-02-07 (pre-holiday): earliest_release_date MUST be None.
    If session ends on 2024-02-15 (reopen day 1): earliest_release_date MUST be None.
    If session ends on 2024-02-16 (reopen day 2): earliest_release_date MUST be datetime(2024, 2, 16, 0, 0).
    """
    db = isolated_db

    # Create candles: 2024-02-07, 2024-02-15, 2024-02-16
    c1 = Candle(symbol="TET_CLAMP", timeframe="1D", timestamp=datetime(2024, 2, 7), open=50.0, high=52.0, low=49.0, close=51.0, volume=1000)
    c2 = Candle(symbol="TET_CLAMP", timeframe="1D", timestamp=datetime(2024, 2, 15), open=51.0, high=53.0, low=50.0, close=52.0, volume=1000)
    c3 = Candle(symbol="TET_CLAMP", timeframe="1D", timestamp=datetime(2024, 2, 16), open=52.0, high=54.0, low=51.0, close=53.0, volume=1000)
    db.add_all([c1, c2, c3]); db.commit()

    # Session A: Ends on 2024-02-07 (pre-holiday). Release date (2024-02-16) is beyond session.
    s_a = ReplaySession(symbol="TET_CLAMP", timeframe="1D", start_date=date(2024, 2, 7), end_date=date(2024, 2, 7), current_index=0, initial_cash=10_000_000, current_cash=9_000_000)
    db.add(s_a); db.commit()
    dec_a = Decision(session_id=s_a.id, symbol="TET_CLAMP", action="BUY", candle_index=0, decision_date=datetime(2024, 2, 7), price=51.0, planned_quantity=100)
    db.add(dec_a); db.commit()
    order_a = Order(session_id=s_a.id, decision_id=dec_a.id, symbol="TET_CLAMP", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(order_a); db.commit()
    exc_a = Execution(session_id=s_a.id, order_id=order_a.id, symbol="TET_CLAMP", execution_candle_index=0, execution_date=datetime(2024, 2, 7), execution_price=51.0, quantity=100, gross_amount=5100.0, net_amount=5107.65)
    db.add(exc_a); db.commit()
    pos_a = Position(session_id=s_a.id, symbol="TET_CLAMP", quantity=100.0, average_price=51.0, total_cost=5100.0, opened_at=datetime(2024, 2, 7), status="open")
    db.add(pos_a); db.commit()

    snap_a = PracticeWorkflowService.get_snapshot(db, s_a.id)
    assert snap_a.earliest_release_date is None, "Pre-Tết buy when session ends pre-holiday must have earliest_release_date = None"

    # Session B: Ends on 2024-02-15 (reopen day 1). Release date (2024-02-16) is still beyond session.
    s_b = ReplaySession(symbol="TET_CLAMP", timeframe="1D", start_date=date(2024, 2, 7), end_date=date(2024, 2, 15), current_index=1, initial_cash=10_000_000, current_cash=9_000_000)
    db.add(s_b); db.commit()
    dec_b = Decision(session_id=s_b.id, symbol="TET_CLAMP", action="BUY", candle_index=0, decision_date=datetime(2024, 2, 7), price=51.0, planned_quantity=100)
    db.add(dec_b); db.commit()
    order_b = Order(session_id=s_b.id, decision_id=dec_b.id, symbol="TET_CLAMP", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(order_b); db.commit()
    exc_b = Execution(session_id=s_b.id, order_id=order_b.id, symbol="TET_CLAMP", execution_candle_index=0, execution_date=datetime(2024, 2, 7), execution_price=51.0, quantity=100, gross_amount=5100.0, net_amount=5107.65)
    db.add(exc_b); db.commit()
    pos_b = Position(session_id=s_b.id, symbol="TET_CLAMP", quantity=100.0, average_price=51.0, total_cost=5100.0, opened_at=datetime(2024, 2, 7), status="open")
    db.add(pos_b); db.commit()

    snap_b = PracticeWorkflowService.get_snapshot(db, s_b.id)
    assert snap_b.earliest_release_date is None, "Pre-Tết buy when session ends on reopen day 1 must have earliest_release_date = None"

    # Session C: Ends on 2024-02-16 (reopen day 2). Release date is within session horizon.
    s_c = ReplaySession(symbol="TET_CLAMP", timeframe="1D", start_date=date(2024, 2, 7), end_date=date(2024, 2, 16), current_index=1, initial_cash=10_000_000, current_cash=9_000_000)
    db.add(s_c); db.commit()
    dec_c = Decision(session_id=s_c.id, symbol="TET_CLAMP", action="BUY", candle_index=0, decision_date=datetime(2024, 2, 7), price=51.0, planned_quantity=100)
    db.add(dec_c); db.commit()
    order_c = Order(session_id=s_c.id, decision_id=dec_c.id, symbol="TET_CLAMP", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(order_c); db.commit()
    exc_c = Execution(session_id=s_c.id, order_id=order_c.id, symbol="TET_CLAMP", execution_candle_index=0, execution_date=datetime(2024, 2, 7), execution_price=51.0, quantity=100, gross_amount=5100.0, net_amount=5107.65)
    db.add(exc_c); db.commit()
    pos_c = Position(session_id=s_c.id, symbol="TET_CLAMP", quantity=100.0, average_price=51.0, total_cost=5100.0, opened_at=datetime(2024, 2, 7), status="open")
    db.add(pos_c); db.commit()

    snap_c = PracticeWorkflowService.get_snapshot(db, s_c.id)
    assert snap_c.earliest_release_date == datetime(2024, 2, 16, 0, 0), "Pre-Tết buy when session includes reopen day 2 must yield 2024-02-16"


def test_adv_sell_rejection_detail_beyond_horizon(isolated_db: Session):
    """Verify that attempting to sell shares locked beyond the session horizon raises LOT_SETTLEMENT_LOCKED with strict schema."""
    db = isolated_db

    # 2-day session
    c1 = Candle(symbol="SELL_ADV", timeframe="1D", timestamp=datetime(2024, 1, 2), open=10.0, high=11.0, low=9.0, close=10.5, volume=100)
    c2 = Candle(symbol="SELL_ADV", timeframe="1D", timestamp=datetime(2024, 1, 3), open=10.5, high=11.5, low=10.0, close=11.0, volume=100)
    db.add_all([c1, c2]); db.commit()

    s = ReplaySession(symbol="SELL_ADV", timeframe="1D", start_date=date(2024, 1, 2), end_date=date(2024, 1, 3), current_index=1, initial_cash=10_000_000, current_cash=9_000_000)
    db.add(s); db.commit()

    # Buy on bar 1 (Wed Jan 3, final bar of session)
    dec = Decision(session_id=s.id, symbol="SELL_ADV", action="BUY", candle_index=1, decision_date=datetime(2024, 1, 3), price=11.0, planned_quantity=100)
    db.add(dec); db.commit()
    order = Order(session_id=s.id, decision_id=dec.id, symbol="SELL_ADV", side="BUY", order_type="MARKET_NEXT_OPEN", quantity=100, status="executed")
    db.add(order); db.commit()
    exc = Execution(session_id=s.id, order_id=order.id, symbol="SELL_ADV", execution_candle_index=1, execution_date=datetime(2024, 1, 3), execution_price=11.0, quantity=100, gross_amount=1100.0, net_amount=1101.65)
    db.add(exc); db.commit()
    pos = Position(session_id=s.id, symbol="SELL_ADV", quantity=100.0, average_price=11.0, total_cost=1100.0, opened_at=datetime(2024, 1, 3), status="open")
    db.add(pos); db.commit()

    # Attempt sell on bar 1
    dec_sell = DecisionCreate(action="SELL", candle_index=1, price=11.0, quantity=100)
    with pytest.raises(HTTPException) as exc_info:
        TradeLifecycleService.process_decision(db, s.id, dec_sell)

    assert exc_info.value.status_code == 400
    detail = exc_info.value.detail
    assert detail["error_code"] == "LOT_SETTLEMENT_LOCKED"
    assert detail["status"] == "REJECTED_SETTLEMENT_LOCK"
    assert detail["available_quantity"] == 0.0
    assert detail["blocked_quantity"] == 100.0
    assert detail["settlement_label"] == "T+1 · về sau 1 phiên"
    assert "blocked_quantity" in detail
    assert "blocked_qty" in detail
    assert "available_quantity" in detail
    assert "available_qty" in detail
    assert "T+2 constraint" in detail




