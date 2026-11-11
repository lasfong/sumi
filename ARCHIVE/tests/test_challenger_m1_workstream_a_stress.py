import sys
from pathlib import Path
from datetime import date, datetime, timedelta, timezone
from types import SimpleNamespace
import pandas as pd
import pytest
from fastapi import HTTPException
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from sqlalchemy.pool import StaticPool

# Add backend directory to sys.path
_backend_dir = Path(__file__).resolve().parent.parent / "backend"
if str(_backend_dir) not in sys.path:
    sys.path.insert(0, str(_backend_dir))

from tests_safety import install_database_safeguard
install_database_safeguard()

from app.db import Base
import app.models  # noqa: F401
from app.models.candle import Candle
from app.models.symbol import Symbol
from app.models.order import Order
from app.models.position import Position
from app.models.trade import Trade
from app.models.execution import Execution
from app.models.event_log import EventLog
from app.models.replay_session import ReplaySession
from app.services.replay_service import ReplayService
from app.services.trade_lifecycle_service import TradeLifecycleService
from app.services.strategy_lab_service import StrategyLabService
from app.services.analytics_service import AnalyticsService
from app.schemas.replay_schema import ReplaySessionCreate
from app.schemas.decision_schema import DecisionCreate
from app.domain.enums import DecisionAction, OrderType, OrderStatus, OrderSide, SessionStatus
from app.api.decisions import get_orders, clear_order_rejection_registry
from app.domain.market.calendar import TradingCalendar
from app.domain.accounting import calculate_buy_amounts, calculate_sell_amounts, calculate_pnl_percent
from app.domain.strategy.strategy_loader import load_strategy_from_dict


@pytest.fixture(autouse=True)
def clean_registry():
    clear_order_rejection_registry()
    yield
    clear_order_rejection_registry()


@pytest.fixture
def test_db():
    engine = create_engine(
        "sqlite:///:memory:",
        connect_args={"check_same_thread": False},
        poolclass=StaticPool,
    )
    Base.metadata.create_all(bind=engine)
    SessionClass = sessionmaker(autocommit=False, autoflush=False, bind=engine)
    db = SessionClass()
    try:
        yield db
    finally:
        db.close()
        Base.metadata.drop_all(bind=engine)


def _seed_candles(db, symbol: str = "FPT", count: int = 30, base_price: float = 90_000.0):
    sym = Symbol(
        symbol=symbol,
        exchange="HOSE",
        company_name=f"{symbol} Corp",
        sector="Technology",
        industry="Software",
        asset_type="stock",
        is_active=True,
    )
    db.merge(sym)
    db.flush()

    cal = TradingCalendar()
    cur = date(2024, 1, 2)
    added = 0
    while added < count:
        if cal.is_trading_day(cur):
            db.add(
                Candle(
                    symbol=symbol,
                    timeframe="1D",
                    timestamp=datetime(cur.year, cur.month, cur.day, tzinfo=timezone.utc),
                    open=base_price,
                    high=base_price + 2_000.0,
                    low=base_price - 1_000.0,
                    close=base_price + 1_000.0,
                    volume=1_000_000 + added * 10_000,
                    source="cafef",
                    adjustment_type="unadjusted",
                )
            )
            added += 1
        cur += timedelta(days=1)
    db.commit()


# ============================================================================
# STRESS TEST 1: Adversarial Multi-Order Queue with Mixed Affordability
# ============================================================================
def test_adversarial_mixed_order_queue(test_db):
    """Stress test: 5 orders queued at index 0 with mixed affordability and actions.

    Order 1: Unaffordable BUY (9B VND) -> must be REJECTED
    Order 2: Affordable BUY (200 shares ~= 18M VND) -> must be EXECUTED
    Order 3: Unaffordable BUY (1,000 shares ~= 90M VND vs ~82M cash left) -> must be REJECTED
    Order 4: Affordable BUY (100 shares ~= 9M VND) -> must be EXECUTED
    Order 5: Unaffordable SELL (500 shares, but only 300 bought) -> must be REJECTED

    Verifies:
    - Independent order transitions without transaction leakage or wedging
    - Accurate cash deductions for executed orders only
    - Exact position quantity (300 shares)
    - Replay advances cleanly to next index
    """
    _seed_candles(test_db, "FPT", count=15, base_price=90_000.0)

    session_in = ReplaySessionCreate(
        symbol="FPT",
        timeframe="1D",
        start_date=date(2024, 1, 2),
        end_date=date(2024, 2, 15),
        initial_cash=100_000_000.0,
    )
    session = ReplayService.create_session(test_db, session_in)
    assert session.current_cash == 100_000_000.0

    # 1. Place 5 orders as MARKET_NEXT_OPEN at index 0
    # Order 1: 100,000 shares @ 90,000 ~= 9 Billion VND (> 100M)
    d1 = TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
        action=DecisionAction.BUY, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=100_000.0, price=90_000.0
    ))
    # Order 2: 200 shares @ 90,000 ~= 18M VND (< 100M)
    d2 = TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
        action=DecisionAction.BUY, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=200.0, price=90_000.0
    ))
    # Order 3: 1,000 shares @ 90,000 ~= 90.13M VND (> 82M remaining)
    d3 = TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
        action=DecisionAction.BUY, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=1_000.0, price=90_000.0
    ))
    # Order 4: 100 shares @ 90,000 ~= 9.01M VND (< 82M remaining)
    d4 = TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
        action=DecisionAction.BUY, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=100.0, price=90_000.0
    ))
    # Order 5: SELL 500 shares (only 300 will be held)
    d5 = TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
        action=DecisionAction.SELL, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=500.0, price=90_000.0
    ))

    pending = test_db.query(Order).filter(Order.session_id == session.id, Order.status == OrderStatus.PENDING.value).all()
    assert len(pending) == 5

    # Step to index 1: match all 5 pending orders
    next_session = ReplayService.next_candle(test_db, session.id, steps=1)
    assert next_session.current_index == 1

    # Check order outcomes
    orders_by_decision = {o.decision_id: o for o in test_db.query(Order).filter(Order.session_id == session.id).all()}
    assert orders_by_decision[d1.id].status == OrderStatus.REJECTED.value, "Order 1 should be REJECTED"
    assert orders_by_decision[d2.id].status == OrderStatus.EXECUTED.value, "Order 2 should be EXECUTED"
    assert orders_by_decision[d3.id].status == OrderStatus.REJECTED.value, "Order 3 should be REJECTED"
    assert orders_by_decision[d4.id].status == OrderStatus.EXECUTED.value, "Order 4 should be EXECUTED"
    assert orders_by_decision[d5.id].status == OrderStatus.REJECTED.value, "Order 5 should be REJECTED"

    # Verify positions: exactly 300 shares
    pos = test_db.query(Position).filter(Position.session_id == session.id, Position.status == "open").first()
    assert pos is not None
    assert pos.quantity == 300.0

    # Verify cash: 100M - cost(200) - cost(100)
    cost2 = calculate_buy_amounts(90_000.0, 200.0).net_amount
    cost4 = calculate_buy_amounts(90_000.0, 100.0).net_amount
    expected_cash = 100_000_000.0 - cost2 - cost4
    assert next_session.current_cash == pytest.approx(expected_cash, abs=1e-2)

    # Verify stepping continues smoothly
    s2 = ReplayService.next_candle(test_db, session.id, steps=1)
    assert s2.current_index == 2
    s3 = ReplayService.next_candle(test_db, session.id, steps=1)
    assert s3.current_index == 3


# ============================================================================
# STRESS TEST 2: Zero Initial Cash & Replay Advancement
# ============================================================================
def test_near_zero_cash_boundary_replay(test_db):
    """Stress test: Near-zero cash session (100.0 VND initial cash).

    Pending order must be rejected cleanly with no division by zero or exception crash,
    and cursor must advance freely.
    """
    _seed_candles(test_db, "FPT", count=10, base_price=90_000.0)

    session_in = ReplaySessionCreate(
        symbol="FPT",
        timeframe="1D",
        start_date=date(2024, 1, 2),
        end_date=date(2024, 1, 30),
        initial_cash=100.0,
    )
    session = ReplayService.create_session(test_db, session_in)
    assert session.current_cash == 100.0

    # Place pending MARKET_NEXT_OPEN BUY (cost ~9.01M >> 100 VND)
    TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
        action=DecisionAction.BUY, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=100.0, price=90_000.0
    ))

    # Advance to next candle
    s1 = ReplayService.next_candle(test_db, session.id, steps=1)
    assert s1.current_index == 1
    assert s1.current_cash == 100.0

    order = test_db.query(Order).filter(Order.session_id == session.id).first()
    assert order.status == OrderStatus.REJECTED.value

    # Subsequent advance works normally
    s2 = ReplayService.next_candle(test_db, session.id, steps=1)
    assert s2.current_index == 2


def test_zero_cash_with_open_position_replay(test_db):
    """Stress test: 0 cash remaining while holding an open position (equity > 0).

    Pending order must be rejected cleanly for insufficient cash without wedging.
    """
    _seed_candles(test_db, "FPT", count=10, base_price=90_000.0)

    # Calculate exact cost for 100 shares @ 90,000 VND
    cost = calculate_buy_amounts(90_000.0, 100.0).net_amount
    session = ReplayService.create_session(test_db, ReplaySessionCreate(
        symbol="FPT", timeframe="1D", start_date=date(2024, 1, 2), end_date=date(2024, 1, 30), initial_cash=cost
    ))

    # Buy 100 shares immediately at market close
    TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
        action=DecisionAction.BUY, order_type=OrderType.MARKET_AT_CLOSE.value, quantity=100.0, price=90_000.0
    ))
    test_db.refresh(session)
    assert session.current_cash == pytest.approx(0.0, abs=1e-4)

    # Now with 0 cash remaining, place pending MARKET_NEXT_OPEN BUY for another 100 shares
    TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
        action=DecisionAction.BUY, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=100.0, price=90_000.0
    ))

    # Advance to next candle
    s1 = ReplayService.next_candle(test_db, session.id, steps=1)
    assert s1.current_index == 1
    assert s1.current_cash == pytest.approx(0.0, abs=1e-4)

    # Pending order must be rejected cleanly
    orders = test_db.query(Order).filter(Order.session_id == session.id, Order.order_type == OrderType.MARKET_NEXT_OPEN.value).all()
    assert len(orders) == 1
    assert orders[0].status == OrderStatus.REJECTED.value

    # Position is intact (100 shares)
    pos = test_db.query(Position).filter(Position.session_id == session.id, Position.status == "open").first()
    assert pos.quantity == 100.0

    # Advance further
    s2 = ReplayService.next_candle(test_db, session.id, steps=1)
    assert s2.current_index == 2


# ============================================================================
# STRESS TEST 3: Multi-Step Jump Advancement with Pending Orders
# ============================================================================
def test_rapid_multi_step_jump_with_unaffordable_orders(test_db):
    """Stress test: Stepping 10 bars forward in a single call (steps=10) with pending orders.

    Both MARKET_NEXT_OPEN and LIMIT orders must transition on their appropriate triggering bars,
    and cursor must land cleanly at index 10 without getting wedged mid-jump.
    """
    _seed_candles(test_db, "FPT", count=25, base_price=90_000.0)

    session = ReplayService.create_session(test_db, ReplaySessionCreate(
        symbol="FPT", timeframe="1D", start_date=date(2024, 1, 2), end_date=date(2024, 2, 28), initial_cash=10_000_000.0
    ))

    # Place unaffordable MARKET_NEXT_OPEN BUY (cost ~90M > 10M)
    d_mkt = TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
        action=DecisionAction.BUY, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=1000.0, price=90_000.0
    ))

    # Place unaffordable LIMIT BUY (limit price 89,500, triggered on low=89,000, cost ~44.8M > 10M)
    d_lim = TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
        action=DecisionAction.BUY, order_type=OrderType.LIMIT.value, quantity=500.0, price=89_500.0
    ))

    # Jump 10 steps in one call!
    jump_session = ReplayService.next_candle(test_db, session.id, steps=10)
    assert jump_session.current_index == 10
    assert jump_session.current_cash == 10_000_000.0

    orders = test_db.query(Order).filter(Order.session_id == session.id).all()
    for o in orders:
        assert o.status == OrderStatus.REJECTED.value


# ============================================================================
# STRESS TEST 4: Rewind and Re-Advance Oscillation (Zero Order Duplication)
# ============================================================================
def test_oscillation_rewind_readvance_no_duplicate_executions(test_db):
    """Stress test: Oscillate back and forth across rejection point.

    Bar 0 -> Bar 1: place unaffordable order
    Bar 1 -> Bar 2: order rejected
    Bar 2 -> Bar 5: advance
    Bar 5 -> Bar 1: rewind (check masked as PENDING)
    Bar 1 -> Bar 0: rewind (check masked as non-existent)
    Bar 0 -> Bar 2: advance back
    Bar 2 -> Bar 6: advance forward

    Verifies:
    - Order is masked properly at every cursor
    - Re-advancing does NOT duplicate rejection logs or executions
    - No phantom positions created
    """
    _seed_candles(test_db, "FPT", count=15, base_price=90_000.0)

    session = ReplayService.create_session(test_db, ReplaySessionCreate(
        symbol="FPT", timeframe="1D", start_date=date(2024, 1, 2), end_date=date(2024, 2, 15), initial_cash=50_000_000.0
    ))

    # Advance to bar 1
    ReplayService.next_candle(test_db, session.id, steps=1)

    # Place unaffordable order at bar 1
    TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
        action=DecisionAction.BUY, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=5_000.0, price=90_000.0
    ))

    # Advance to bar 2: rejected
    ReplayService.next_candle(test_db, session.id, steps=1)
    orders_at_2 = get_orders(session.id, db=test_db)
    assert len(orders_at_2) == 1
    assert orders_at_2[0].status == OrderStatus.REJECTED

    # Advance to bar 5
    ReplayService.next_candle(test_db, session.id, steps=3)
    test_db.refresh(session)
    assert session.current_index == 5

    # Rewind to bar 1
    ReplayService.previous_candle(test_db, session.id, steps=4)
    test_db.refresh(session)
    assert session.current_index == 1
    orders_at_1 = get_orders(session.id, db=test_db)
    assert len(orders_at_1) == 1
    assert orders_at_1[0].status == OrderStatus.PENDING

    # Rewind to bar 0
    ReplayService.previous_candle(test_db, session.id, steps=1)
    test_db.refresh(session)
    assert session.current_index == 0
    orders_at_0 = get_orders(session.id, db=test_db)
    assert len(orders_at_0) == 0

    # Re-advance to bar 2
    ReplayService.next_candle(test_db, session.id, steps=2)
    test_db.refresh(session)
    assert session.current_index == 2
    orders_at_2_again = get_orders(session.id, db=test_db)
    assert len(orders_at_2_again) == 1
    assert orders_at_2_again[0].status == OrderStatus.REJECTED

    # Total order rows in DB should still be exactly 1!
    total_orders_in_db = test_db.query(Order).filter(Order.session_id == session.id).count()
    assert total_orders_in_db == 1

    total_executions_in_db = test_db.query(Execution).filter(Execution.session_id == session.id).count()
    assert total_executions_in_db == 0


# ============================================================================
# STRESS TEST 5: SQA-013 Strategy Lab Win Rate Precision & Complex Fractions
# ============================================================================
def test_strategy_lab_win_rate_complex_fractions_oracle():
    """Oracle test: Generate multiple trade distribution scenarios including odd fractions

    (1/3, 2/7, 5/9, 99/100, 0/10, 10/10) and verify exact scale consistency (0..1 fraction)
    and that frontend formatPercent produces exact formatting without double scaling.
    """
    svc = StrategyLabService()
    strategy_obj = load_strategy_from_dict({
        "name": "OracleStrategy",
        "indicators": [], "entry_rules": [], "exit_rules": [],
        "position_sizing": {"method": "fixed_quantity", "quantity": 100},
    })
    df = pd.DataFrame([
        {"timestamp": datetime(2024, 1, 2, tzinfo=timezone.utc), "open": 100.0, "high": 105.0, "low": 98.0, "close": 102.0, "volume": 1000},
        {"timestamp": datetime(2024, 1, 3, tzinfo=timezone.utc), "open": 102.0, "high": 108.0, "low": 101.0, "close": 106.0, "volume": 1500},
    ])
    candles = [
        Candle(symbol="FPT", timeframe="1D", adjustment_type="unadjusted", timestamp=datetime(2024, 1, 2, tzinfo=timezone.utc), open=100.0, high=105.0, low=98.0, close=102.0, volume=1000),
        Candle(symbol="FPT", timeframe="1D", adjustment_type="unadjusted", timestamp=datetime(2024, 1, 3, tzinfo=timezone.utc), open=102.0, high=108.0, low=101.0, close=106.0, volume=1500),
    ]

    def frontend_format_percent(val):
        return f"{(val * 100):.2f}%" if val is not None else "0.00%"

    test_cases = [
        # (wins, losses, expected_fraction_round4, expected_frontend_str)
        (1, 2, 0.3333, "33.33%"),   # 1/3 = 33.33%
        (2, 5, 0.2857, "28.57%"),   # 2/7 = 28.57%
        (5, 4, 0.5556, "55.56%"),   # 5/9 = 55.56%
        (99, 1, 0.9900, "99.00%"),  # 99/100 = 99.00%
        (0, 10, 0.0000, "0.00%"),   # 0/10 = 0.00%
        (10, 0, 1.0000, "100.00%"), # 10/10 = 100.00%
    ]

    for wins, losses, exp_frac, exp_str in test_cases:
        trades = (
            [SimpleNamespace(net_pnl=100_000.0, status="closed") for _ in range(wins)]
            + [SimpleNamespace(net_pnl=-50_000.0, status="closed") for _ in range(losses)]
        )
        resp = svc._format_kernel_response(
            SimpleNamespace(trades=trades),
            strategy_obj, "FPT", df, candles, 100_000_000.0, "VNINDEX"
        )
        metric_val = resp["analytics"]["metrics"]["win_rate"]["value"]
        assert metric_val == pytest.approx(exp_frac, abs=1e-4)
        assert resp["summary"]["win_rate"] == pytest.approx(wins / (wins + losses), abs=1e-4)
        assert resp["analytics"]["win_rate"] == pytest.approx(wins / (wins + losses), abs=1e-4)

        rendered = frontend_format_percent(metric_val)
        assert rendered == exp_str
        assert "000" not in rendered or rendered == "100.00%", f"Double scaling anomaly in {rendered}"


# ============================================================================
# STRESS TEST 6: SQA-018 Analytics PnL Extreme Values & Parity Oracle
# ============================================================================
def test_analytics_extreme_pnl_and_formatting(test_db):
    """Stress test: Extreme PnL percentages (-99.95%, +500.00%, +0.01%, 0.00%, -0.25% fee loss).

    Verifies that all trades render accurately via formatPctValue without 100x multiplication.
    """
    session = ReplaySession(
        symbol="FPT",
        timeframe="1D",
        adjustment_type="unadjusted",
        start_date=datetime(2024, 1, 2, tzinfo=timezone.utc),
        end_date=datetime(2024, 2, 28, tzinfo=timezone.utc),
        initial_cash=100_000_000.0,
        current_cash=100_000_000.0,
        current_index=10,
        status="active",
    )
    test_db.add(session)
    test_db.commit()

    trades_data = [
        # (id, pnl_percent, net_pnl, expected_render)
        (1, -99.95, -9_995_000.0, "-99.95%"),
        (2, 500.00, 50_000_000.0, "500.00%"),
        (3, 0.01, 1_000.0, "0.01%"),
        (4, 0.00, 0.0, "0.00%"),
        (5, -0.25, -25_000.0, "-0.25%"),
    ]

    for tid, pct, net, _ in trades_data:
        test_db.add(Trade(
            session_id=session.id,
            symbol="FPT",
            entry_date=datetime(2024, 1, 2, tzinfo=timezone.utc),
            entry_price=100_000.0,
            quantity=100.0,
            exit_date=datetime(2024, 1, 10, tzinfo=timezone.utc),
            exit_price=100_000.0,
            gross_pnl=net,
            net_pnl=net,
            pnl_percent=pct,
            status="closed",
            result="win" if pct > 0 else "loss" if pct < 0 else "breakeven",
        ))
    test_db.commit()

    analytics = AnalyticsService.get_analytics(test_db, session.id)
    assert analytics is not None
    assert len(analytics.trade_distribution) == 5

    def format_pct_value(val):
        return f"{val:.2f}%" if val is not None else "—"

    td_by_id = {item.trade_id: item for item in analytics.trade_distribution}
    for tid, pct, net, expected_render in trades_data:
        actual_pct = td_by_id[tid].pnl_percent
        assert actual_pct == pytest.approx(pct, abs=1e-2)
        rendered = format_pct_value(actual_pct)
        assert rendered == expected_render, f"Expected {expected_render}, got {rendered}"
        # Assert no 100x double-scaling occurred (e.g. -9995% instead of -99.95%)
        assert "9995" not in rendered
        assert "50000" not in rendered


# ============================================================================
# STRESS TEST 7: Invalid Quantities and Negative/Zero Prices
# ============================================================================
def test_invalid_quantity_and_price_rejection(test_db):
    """Stress test: Negative/zero quantities and prices must be rejected fail-closed at submission time."""
    _seed_candles(test_db, "FPT", count=5, base_price=90_000.0)

    session = ReplayService.create_session(test_db, ReplaySessionCreate(
        symbol="FPT", timeframe="1D", start_date=date(2024, 1, 2), end_date=date(2024, 1, 20), initial_cash=100_000_000.0
    ))

    # 1. Quantity = 0
    with pytest.raises(HTTPException) as exc1:
        TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
            action=DecisionAction.BUY, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=0.0, price=90_000.0
        ))
    assert exc1.value.status_code == 400

    # 2. Negative quantity
    with pytest.raises(HTTPException) as exc2:
        TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
            action=DecisionAction.BUY, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=-100.0, price=90_000.0
        ))
    assert exc2.value.status_code == 400

    # 3. Non-100 lot increment (e.g. 50 shares) -> caught fail-closed by Pydantic schema validator
    with pytest.raises((HTTPException, ValueError)):
        DecisionCreate(
            action=DecisionAction.BUY, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=50.0, price=90_000.0
        )

    # 4. Zero price
    with pytest.raises(HTTPException) as exc4:
        TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
            action=DecisionAction.BUY, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=100.0, price=0.0
        ))
    assert exc4.value.status_code == 400

    # 5. Negative price
    with pytest.raises(HTTPException) as exc5:
        TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
            action=DecisionAction.BUY, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=100.0, price=-50_000.0
        ))
    assert exc5.value.status_code == 400

    # Verify session is still completely healthy and active
    test_db.refresh(session)
    assert session.status == SessionStatus.ACTIVE.value
    assert session.current_index == 0
    s_next = ReplayService.next_candle(test_db, session.id, steps=1)
    assert s_next.current_index == 1
