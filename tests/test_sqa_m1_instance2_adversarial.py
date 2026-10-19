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


def _seed_candles(db, symbol: str = "FPT", count: int = 20, base_price: float = 90_000.0):
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
# CHALLENGE 1: EventLog-Driven Rejection Persistence Across Process Restarts
# ============================================================================
def test_eventlog_driven_rejection_persistence_across_process_restarts(test_db):
    """Stress test: Multi-bar orders rejected across different bars, with the in-memory
    registry explicitly purged between operations (simulating process restarts / multi-worker).

    Verifies:
    - session_rejection_metadata parsed from EventLogs correctly reconstructs rejection bars.
    - Cursor rewind masks future rejections as PENDING even with an empty in-memory registry.
    - Prior candle rewind excludes orders not yet created.
    """
    _seed_candles(test_db, "FPT", count=15, base_price=90_000.0)

    session = ReplayService.create_session(test_db, ReplaySessionCreate(
        symbol="FPT", timeframe="1D", start_date=date(2024, 1, 2), end_date=date(2024, 2, 15), initial_cash=50_000_000.0
    ))
    assert session.current_index == 0

    # Candle 0: Place 1st unaffordable order (Order 1, requires ~90M > 50M)
    d1 = TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
        action=DecisionAction.BUY, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=1_000.0, price=90_000.0
    ))

    # Advance to Candle 1: Order 1 is matched and REJECTED
    ReplayService.next_candle(test_db, session.id, steps=1)
    test_db.refresh(session)
    assert session.current_index == 1

    # Candle 1: Place 2nd unaffordable order (Order 2, requires ~180M > 50M)
    d2 = TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
        action=DecisionAction.BUY, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=2_000.0, price=90_000.0
    ))

    # Advance to Candle 2: Order 2 is matched and REJECTED
    ReplayService.next_candle(test_db, session.id, steps=1)
    test_db.refresh(session)
    assert session.current_index == 2

    # Advance to Candle 3:
    ReplayService.next_candle(test_db, session.id, steps=1)
    test_db.refresh(session)
    assert session.current_index == 3

    # NOW SIMULATE SERVER RESTART / PROCESS PURGE:
    clear_order_rejection_registry()

    # Query orders at Candle 3: Both Order 1 and Order 2 must be REJECTED
    orders_at_3 = {o.id: o for o in get_orders(session.id, db=test_db)}
    assert len(orders_at_3) == 2
    assert orders_at_3[1].status == OrderStatus.REJECTED
    assert orders_at_3[2].status == OrderStatus.REJECTED

    # Rewind to Candle 2:
    ReplayService.previous_candle(test_db, session.id, steps=1)
    test_db.refresh(session)
    assert session.current_index == 2

    clear_order_rejection_registry()
    orders_at_2 = {o.id: o for o in get_orders(session.id, db=test_db)}
    assert len(orders_at_2) == 2
    assert orders_at_2[1].status == OrderStatus.REJECTED  # rejected at candle 1 <= 2
    assert orders_at_2[2].status == OrderStatus.REJECTED  # rejected at candle 2 <= 2

    # Rewind to Candle 1:
    ReplayService.previous_candle(test_db, session.id, steps=1)
    test_db.refresh(session)
    assert session.current_index == 1

    clear_order_rejection_registry()
    orders_at_1 = {o.id: o for o in get_orders(session.id, db=test_db)}
    assert len(orders_at_1) == 2
    assert orders_at_1[1].status == OrderStatus.REJECTED  # rejected at candle 1 <= 1
    assert orders_at_1[2].status == OrderStatus.PENDING   # rejected at candle 2 > 1 -> MASKED AS PENDING!

    # Rewind to Candle 0:
    ReplayService.previous_candle(test_db, session.id, steps=1)
    test_db.refresh(session)
    assert session.current_index == 0

    clear_order_rejection_registry()
    orders_at_0 = {o.id: o for o in get_orders(session.id, db=test_db)}
    assert len(orders_at_0) == 1
    assert orders_at_0[1].status == OrderStatus.PENDING  # created at 0, rejected at 1 > 0 -> MASKED AS PENDING!
    assert 2 not in orders_at_0                          # created at candle 1 > 0 -> DOES NOT EXIST!


# ============================================================================
# CHALLENGE 2: Pending SELL Settlement Lock Rejection & Masking
# ============================================================================
def test_pending_sell_settlement_lock_rejection_and_masking(test_db):
    """Stress test: Pending SELL order submitted when shares are locked under T+2 settlement.

    Verifies:
    - Pending SELL is rejected cleanly when available_qty < qty
    - EventLog records ORDER_REJECTED with reason 'settlement_locked'
    - Replay advances cleanly without wedging
    - Cursor rewind isolates the rejection, masking as PENDING at bar of creation
    - Position remains intact
    """
    _seed_candles(test_db, "FPT", count=15, base_price=90_000.0)

    session = ReplayService.create_session(test_db, ReplaySessionCreate(
        symbol="FPT", timeframe="1D", start_date=date(2024, 1, 2), end_date=date(2024, 2, 15), initial_cash=100_000_000.0
    ))

    # Candle 0 (Tuesday Jan 2): Buy 500 shares at market close
    TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
        action=DecisionAction.BUY, order_type=OrderType.MARKET_AT_CLOSE.value, quantity=500.0, price=90_000.0
    ))
    test_db.refresh(session)

    # Position is open with 500 shares, but unsettled (T+2 settlement)
    pos = test_db.query(Position).filter(Position.session_id == session.id, Position.status == "open").first()
    assert pos is not None
    assert pos.quantity == 500.0

    # Advance to Candle 1 (Wednesday Jan 3)
    ReplayService.next_candle(test_db, session.id, steps=1)
    test_db.refresh(session)
    assert session.current_index == 1

    # At Candle 1: Place a pending MARKET_NEXT_OPEN SELL order for 500 shares
    # On next day (Thursday Jan 4 = T+2 morning open phase), shares are still not yet settled!
    d_sell = TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
        action=DecisionAction.SELL, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=500.0, price=90_000.0
    ))

    pending_sell = test_db.query(Order).filter(Order.session_id == session.id, Order.order_type == OrderType.MARKET_NEXT_OPEN.value).first()
    assert pending_sell is not None
    assert pending_sell.status == OrderStatus.PENDING.value

    # Advance to Candle 2 (Thursday Jan 4): Order matches at open, where T+2 shares are not yet available!
    ReplayService.next_candle(test_db, session.id, steps=1)
    test_db.refresh(session)
    assert session.current_index == 2

    # Verify pending SELL order was REJECTED cleanly!
    test_db.refresh(pending_sell)
    assert pending_sell.status == OrderStatus.REJECTED.value

    # Verify EventLog
    sell_rej_logs = test_db.query(EventLog).filter(
        EventLog.session_id == session.id,
        EventLog.event_type == "ORDER_REJECTED",
    ).all()
    assert len(sell_rej_logs) >= 1
    assert any("T+2 constraint" in l.message or "settled" in l.message for l in sell_rej_logs)

    # Verify position is still open and untouched!
    pos_after = test_db.query(Position).filter(Position.session_id == session.id, Position.status == "open").first()
    assert pos_after.quantity == 500.0

    # Advance further: stepping continues without any wedging!
    s3 = ReplayService.next_candle(test_db, session.id, steps=1)
    assert s3.current_index == 3

    # Rewind to Candle 1 (order created at 1, rejected at 2):
    ReplayService.previous_candle(test_db, session.id, steps=2)
    test_db.refresh(session)
    assert session.current_index == 1

    orders_at_1 = get_orders(session.id, db=test_db)
    sell_orders_at_1 = [o for o in orders_at_1 if o.side == "SELL"]
    assert len(sell_orders_at_1) == 1
    assert sell_orders_at_1[0].status == OrderStatus.PENDING, "Future rejection must be masked as PENDING at creation bar"


# ============================================================================
# CHALLENGE 3: Interleaved Cancellation vs Execution vs Rejection
# ============================================================================
def test_interleaved_cancellation_execution_and_rejection_batch(test_db):
    """Stress test: Complex batch with active cancellation before candle matching.

    Orders placed at candle 0:
    - O1: BUY MARKET_NEXT_OPEN 100 shares @ 90,000 (~9.01M) -> Affordable
    - O2: BUY MARKET_NEXT_OPEN 100,000 shares @ 90,000 (~9B) -> Unaffordable
    - O3: BUY LIMIT 100 shares @ 89,500 (~8.96M) -> To be cancelled
    - O4: BUY MARKET_NEXT_OPEN 100 shares @ 90,000 (~9.01M) -> Affordable

    User cancels O3 at candle 0 before advancing.
    Step to candle 1:
    - O1 executes (FILLED)
    - O2 rejects (REJECTED)
    - O3 was cancelled (CANCELLED)
    - O4 executes (FILLED)
    """
    _seed_candles(test_db, "FPT", count=10, base_price=90_000.0)

    session = ReplayService.create_session(test_db, ReplaySessionCreate(
        symbol="FPT", timeframe="1D", start_date=date(2024, 1, 2), end_date=date(2024, 1, 30), initial_cash=100_000_000.0
    ))

    # O1: BUY 100 shares MKT
    d1 = TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
        action=DecisionAction.BUY, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=100.0, price=90_000.0
    ))
    # O2: BUY 100,000 shares MKT
    d2 = TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
        action=DecisionAction.BUY, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=100_000.0, price=90_000.0
    ))
    # O3: BUY 100 shares LIMIT
    d3 = TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
        action=DecisionAction.BUY, order_type=OrderType.LIMIT.value, quantity=100.0, price=89_500.0
    ))
    # O4: BUY 100 shares MKT
    d4 = TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
        action=DecisionAction.BUY, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=100.0, price=90_000.0
    ))

    o3 = test_db.query(Order).filter(Order.decision_id == d3.id).first()
    assert o3 is not None
    assert o3.status == OrderStatus.PENDING.value

    # Cancel O3 at candle 0
    cancelled_o3 = TradeLifecycleService.cancel_pending_order(test_db, session.id, o3.id)
    assert cancelled_o3.status == OrderStatus.CANCELLED.value

    # Advance to Candle 1
    ReplayService.next_candle(test_db, session.id, steps=1)
    test_db.refresh(session)
    assert session.current_index == 1

    orders_by_dec = {o.decision_id: o for o in test_db.query(Order).filter(Order.session_id == session.id).all()}
    assert orders_by_dec[d1.id].status == OrderStatus.EXECUTED.value
    assert orders_by_dec[d2.id].status == OrderStatus.REJECTED.value
    assert orders_by_dec[d3.id].status == OrderStatus.CANCELLED.value
    assert orders_by_dec[d4.id].status == OrderStatus.EXECUTED.value

    # Position is exactly 200 shares
    pos = test_db.query(Position).filter(Position.session_id == session.id, Position.status == "open").first()
    assert pos.quantity == 200.0

    # Attempt to cancel O2 (which is already REJECTED): must raise HTTPException(400)
    with pytest.raises(HTTPException) as exc:
        TradeLifecycleService.cancel_pending_order(test_db, session.id, orders_by_dec[d2.id].id)
    assert exc.value.status_code == 400
    assert "Only pending orders can be cancelled" in exc.value.detail


# ============================================================================
# CHALLENGE 4: Advancement to Final Dataset Candle (Session Completion)
# ============================================================================
def test_unaffordable_order_at_dataset_boundary_and_completion(test_db):
    """Stress test: Unaffordable order placed when advancing to the final candle of the dataset.

    Verifies:
    - Session cleanly transitions to COMPLETED status
    - Pending order transitions to REJECTED cleanly
    - Rewinding previous_candle reactivates session to ACTIVE
    """
    total_candles = 5
    _seed_candles(test_db, "FPT", count=total_candles, base_price=90_000.0)

    session = ReplayService.create_session(test_db, ReplaySessionCreate(
        symbol="FPT", timeframe="1D", start_date=date(2024, 1, 2), end_date=date(2024, 1, 20), initial_cash=10_000_000.0
    ))
    assert session.current_index == 0

    # Step to candle 3 (penultimate candle before final candle 4)
    ReplayService.next_candle(test_db, session.id, steps=3)
    test_db.refresh(session)
    assert session.current_index == 3
    assert session.status == SessionStatus.ACTIVE.value

    # At candle 3: Place unaffordable order
    TradeLifecycleService.process_decision(test_db, session.id, DecisionCreate(
        action=DecisionAction.BUY, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=10_000.0, price=90_000.0
    ))

    # Step to final candle 4
    ReplayService.next_candle(test_db, session.id, steps=1)
    test_db.refresh(session)
    assert session.current_index == 4
    assert session.status == SessionStatus.COMPLETED.value

    order = test_db.query(Order).filter(Order.session_id == session.id).first()
    assert order.status == OrderStatus.REJECTED.value

    # Rewind to candle 3
    ReplayService.previous_candle(test_db, session.id, steps=1)
    test_db.refresh(session)
    assert session.current_index == 3
    assert session.status == SessionStatus.ACTIVE.value

    orders_at_3 = get_orders(session.id, db=test_db)
    assert len(orders_at_3) == 1
    assert orders_at_3[0].status == OrderStatus.PENDING


# ============================================================================
# CHALLENGE 5: SQA-013 Strategy Lab Win Rate Precision & Nearby Metrics
# ============================================================================
def test_strategy_lab_win_rate_and_metrics_oracle():
    """Oracle test: Exhaustively tests edge trade distributions for Strategy Lab metrics:
    - 0 closed trades -> metric status 'not_applicable', value None
    - All breakeven trades (net_pnl == 0) -> win_rate 0.00
    - All winning trades -> win_rate 1.00 (100.00%)
    - Expectancy sample size threshold (< 30 trades -> 'insufficient_sample')
    - Profit factor when no losses (gross_loss == 0) -> None / 'not_applicable'
    """
    svc = StrategyLabService()
    strategy_obj = load_strategy_from_dict({
        "name": "StressTestStrategy",
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

    cash_100m = 100_000_000.0
    # Case A: 0 closed trades
    resp_empty = svc._format_kernel_response(SimpleNamespace(trades=[]), strategy_obj, "FPT", df, candles, cash_100m, "VNINDEX")
    m_empty = resp_empty["analytics"]["metrics"]
    assert m_empty["win_rate"]["value"] is None
    assert m_empty["win_rate"]["status"] == "not_applicable"
    assert m_empty["expectancy"]["status"] == "insufficient_sample"

    # Case B: All breakeven trades (net_pnl == 0.0)
    be_trades = [SimpleNamespace(net_pnl=0.0, status="closed") for _ in range(5)]
    resp_be = svc._format_kernel_response(SimpleNamespace(trades=be_trades), strategy_obj, "FPT", df, candles, cash_100m, "VNINDEX")
    m_be = resp_be["analytics"]["metrics"]
    assert m_be["win_rate"]["value"] == 0.0
    assert frontend_format_percent(m_be["win_rate"]["value"]) == "0.00%"

    # Case C: All winning trades, no losses (gross_loss = 0)
    win_trades = [SimpleNamespace(net_pnl=500_000.0, status="closed") for _ in range(10)]
    resp_win = svc._format_kernel_response(SimpleNamespace(trades=win_trades), strategy_obj, "FPT", df, candles, cash_100m, "VNINDEX")
    m_win = resp_win["analytics"]["metrics"]
    assert m_win["win_rate"]["value"] == 1.0
    assert frontend_format_percent(m_win["win_rate"]["value"]) == "100.00%"
    assert m_win["profit_factor"]["status"] == "not_applicable"  # No loss to divide by

    # Case D: Exactly 30 trades (sample size threshold for expectancy)
    t30 = [SimpleNamespace(net_pnl=100_000.0, status="closed") for _ in range(20)] + [SimpleNamespace(net_pnl=-50_000.0, status="closed") for _ in range(10)]
    resp_30 = svc._format_kernel_response(SimpleNamespace(trades=t30), strategy_obj, "FPT", df, candles, cash_100m, "VNINDEX")
    m_30 = resp_30["analytics"]["metrics"]
    assert m_30["expectancy"]["status"] == "valid"  # >= 30 trades


# ============================================================================
# CHALLENGE 6: SQA-018 Analytics Trade Distribution & Parity Across PnL Distributions
# ============================================================================
def test_analytics_pnl_percent_cross_surface_and_boundary_values(test_db):
    """Stress test: Multi-trade portfolio covering positive, negative, zero, and boundary percentages.

    Verifies:
    - Exact fidelity in TradeDistribution schema (no conversion / loss of precision)
    - Consistency with JournalPage representation (f"{pnl_percent:.2f}%")
    - Zero double-scaling on any trade row
    """
    session = ReplaySession(
        symbol="VNM",
        timeframe="1D",
        adjustment_type="unadjusted",
        start_date=datetime(2024, 1, 2, tzinfo=timezone.utc),
        end_date=datetime(2024, 2, 28, tzinfo=timezone.utc),
        initial_cash=100_000_000.0,
        current_cash=100_000_000.0,
        current_index=15,
        status="active",
    )
    test_db.add(session)
    test_db.commit()

    test_trades = [
        # (id, symbol, pnl_pct, net_pnl, expected_pct_str)
        (1, "VNM", 15.253, 1_525_300.0, "15.25%"),
        (2, "VNM", -8.749, -874_900.0, "-8.75%"),
        (3, "VNM", 0.000, 0.0, "0.00%"),
        (4, "VNM", 1250.00, 125_000_000.0, "1250.00%"),
        (5, "VNM", -100.00, -10_000_000.0, "-100.00%"),
        (6, "VNM", 0.051, 5_100.0, "0.05%"),
        (7, "VNM", -0.024, -2_400.0, "-0.02%"),
    ]

    for tid, sym, pct, net, _ in test_trades:
        test_db.add(Trade(
            id=tid,
            session_id=session.id,
            symbol=sym,
            entry_date=datetime(2024, 1, 2, tzinfo=timezone.utc),
            entry_price=70_000.0,
            quantity=100.0,
            exit_date=datetime(2024, 1, 10, tzinfo=timezone.utc),
            exit_price=75_000.0,
            net_pnl=net,
            pnl_percent=pct,
            status="closed",
            result="win" if pct > 0 else "loss" if pct < 0 else "breakeven",
        ))
    test_db.commit()

    analytics = AnalyticsService.get_analytics(test_db, session.id)
    assert analytics is not None
    assert len(analytics.trade_distribution) == len(test_trades)

    def format_pct_value(val):
        return f"{val:.2f}%" if val is not None else "—"

    td_map = {item.trade_id: item for item in analytics.trade_distribution}
    for tid, sym, pct, net, exp_str in test_trades:
        item = td_map[tid]
        assert item.pnl_percent == pytest.approx(pct, abs=1e-3)
        rendered = format_pct_value(item.pnl_percent)
        assert rendered == exp_str
        # Check against double-scaling: e.g. 15.25% should NEVER render as 1525.3%
        if pct != 0.0:
            wrong_double_scaled = f"{pct * 100:.2f}%"
            if abs(pct) < 100:
                assert rendered != wrong_double_scaled, f"Double-scaling detected: got {rendered}"


# ============================================================================
# CHALLENGE 7: Exact Cash Boundary & Rounding
# ============================================================================
def test_exact_cash_boundary_execution(test_db):
    """Stress test: Order with exact available cash must execute (not reject).
    Order with exact cash + 1 VND must reject.
    """
    _seed_candles(test_db, "FPT", count=5, base_price=90_000.0)

    # 100 shares @ 90,000 VND. Gross = 9,000,000. Fee (0.15%) = 13,500. Total = 9,013,500 VND.
    exact_cost = calculate_buy_amounts(90_000.0, 100.0).net_amount
    assert exact_cost == 9_013_500.0

    # Session A: Exactly 9,013,500 VND cash
    s_exact = ReplayService.create_session(test_db, ReplaySessionCreate(
        symbol="FPT", timeframe="1D", start_date=date(2024, 1, 2), end_date=date(2024, 1, 20), initial_cash=exact_cost
    ))
    TradeLifecycleService.process_decision(test_db, s_exact.id, DecisionCreate(
        action=DecisionAction.BUY, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=100.0, price=90_000.0
    ))
    # Must execute cleanly!
    s_exact_step = ReplayService.next_candle(test_db, s_exact.id, steps=1)
    assert s_exact_step.current_index == 1
    assert s_exact_step.current_cash == pytest.approx(0.0, abs=1e-2)
    o_exact = test_db.query(Order).filter(Order.session_id == s_exact.id).first()
    assert o_exact.status == OrderStatus.EXECUTED.value

    # Session B: 9,013,499.0 VND cash (1 VND short)
    s_short = ReplayService.create_session(test_db, ReplaySessionCreate(
        symbol="FPT", timeframe="1D", start_date=date(2024, 1, 2), end_date=date(2024, 1, 20), initial_cash=exact_cost - 1.0
    ))
    TradeLifecycleService.process_decision(test_db, s_short.id, DecisionCreate(
        action=DecisionAction.BUY, order_type=OrderType.MARKET_NEXT_OPEN.value, quantity=100.0, price=90_000.0
    ))
    # Must reject cleanly!
    s_short_step = ReplayService.next_candle(test_db, s_short.id, steps=1)
    assert s_short_step.current_index == 1
    assert s_short_step.current_cash == pytest.approx(exact_cost - 1.0, abs=1e-2)
    o_short = test_db.query(Order).filter(Order.session_id == s_short.id).first()
    assert o_short.status == OrderStatus.REJECTED.value
