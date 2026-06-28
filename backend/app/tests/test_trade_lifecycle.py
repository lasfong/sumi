import pytest
from datetime import date
from app.models.candle import Candle
from app.services.replay_service import ReplayService
from app.services.trade_lifecycle_service import TradeLifecycleService
from app.schemas.replay_schema import ReplaySessionCreate
from app.schemas.decision_schema import DecisionCreate
from app.domain.enums import DecisionAction
from app.models.position import Position
from app.models.trade import Trade
from app.models.execution import Execution
from app.models.order import Order

from fastapi import HTTPException

def test_sell_t_plus_1_is_rejected(db_session):
    # Setup session
    for i in range(5):
        candle = Candle(symbol="TEST", timeframe="1D", timestamp=date(2023, 10, i+1), open=100+i, high=102+i, low=98+i, close=100+i, volume=1000)
        db_session.add(candle)
    db_session.commit()

    session_in = ReplaySessionCreate(symbol="TEST", start_date=date(2023, 10, 1), end_date=date(2023, 10, 5))
    session = ReplayService.create_session(db_session, session_in)

    # Buy at index 0
    dec_buy = DecisionCreate(action=DecisionAction.BUY, quantity=100)
    TradeLifecycleService.process_decision(db_session, session.id, dec_buy)

    # Advance to index 1 (T+1)
    ReplayService.next_candle(db_session, session.id)

    # Sell All -> Should fail with 400
    dec_close = DecisionCreate(action=DecisionAction.CLOSE)
    with pytest.raises(HTTPException) as excinfo:
        TradeLifecycleService.process_decision(db_session, session.id, dec_close)

    assert excinfo.value.status_code == 400
    assert "T+2 constraint" in excinfo.value.detail

def test_sell_t_plus_2_is_allowed(db_session):
    # Setup session
    for i in range(5):
        candle = Candle(symbol="TEST", timeframe="1D", timestamp=date(2023, 10, i+1), open=100+i, high=102+i, low=98+i, close=100+i, volume=1000)
        db_session.add(candle)
    db_session.commit()

    session_in = ReplaySessionCreate(symbol="TEST", start_date=date(2023, 10, 1), end_date=date(2023, 10, 5))
    session = ReplayService.create_session(db_session, session_in)

    # Buy at index 0
    dec_buy = DecisionCreate(action=DecisionAction.BUY, quantity=100)
    TradeLifecycleService.process_decision(db_session, session.id, dec_buy)

    # Advance to index 1 (T+1)
    ReplayService.next_candle(db_session, session.id)
    # Advance to index 2 (T+2)
    ReplayService.next_candle(db_session, session.id)

    # Sell All (Close) at T+2
    dec_close = DecisionCreate(action=DecisionAction.CLOSE)
    TradeLifecycleService.process_decision(db_session, session.id, dec_close)

    pos = db_session.query(Position).filter_by(session_id=session.id).first()
    assert pos.quantity == 0
    assert pos.status == "closed"
    # realized_pnl = (sell_price - avg_price) * qty = (102 - 100) * 100 = 200
    assert pos.realized_pnl == 200.0

    trade = db_session.query(Trade).filter_by(session_id=session.id).first()
    assert trade.exit_date is not None
    assert trade.gross_pnl == 200.0
    assert trade.status == "closed"
    assert trade.result == "win"
    assert trade.holding_days == 2

from datetime import timedelta

def test_full_trade_lifecycle_e2e(db_session):
    """
    E2E test for the complete trade lifecycle including fees, taxes, and PnL correctly.
    """
    symbol = "E2E_TEST"
    initial_cash = 100_000_000.0

    # 1. Setup Session & Candles
    for i in range(10):
        candle = Candle(
            symbol=symbol, timeframe="1D", timestamp=date(2024, 1, 1) + timedelta(days=i),
            open=100.0+i, high=101.0+i, low=99.0+i, close=100.0+i, volume=1000000
        )
        db_session.add(candle)
    db_session.commit()

    session_in = ReplaySessionCreate(symbol=symbol, start_date=date(2024, 1, 1), end_date=date(2024, 1, 10), initial_cash=initial_cash)
    session = ReplayService.create_session(db_session, session_in)

    # 2. BUY at index 0
    buy_price = 100.0
    buy_qty = 1000.0
    dec_buy = DecisionCreate(action=DecisionAction.BUY, quantity=buy_qty)
    TradeLifecycleService.process_decision(db_session, session.id, dec_buy)

    # Verify Buy Execution
    buy_execution = db_session.query(Execution).join(Order).filter(
        Execution.session_id == session.id, Order.side == "BUY"
    ).first()

    buy_gross = buy_price * buy_qty
    buy_fee = buy_gross * 0.0015
    assert buy_execution.gross_amount == buy_gross
    assert buy_execution.fee == buy_fee
    assert buy_execution.tax == 0.0
    assert buy_execution.net_amount == buy_gross + buy_fee

    # 3. Advance to T+2
    ReplayService.next_candle(db_session, session.id)
    ReplayService.next_candle(db_session, session.id)

    # 4. SELL at T+2
    sell_price = 102.0
    dec_close = DecisionCreate(action=DecisionAction.CLOSE)
    TradeLifecycleService.process_decision(db_session, session.id, dec_close)

    # Verify Sell Execution
    sell_execution = db_session.query(Execution).join(Order).filter(
        Execution.session_id == session.id, Order.side == "SELL"
    ).first()

    sell_gross = sell_price * buy_qty
    sell_fee = sell_gross * 0.0015
    sell_tax = sell_gross * 0.001
    assert sell_execution.gross_amount == sell_gross
    assert sell_execution.fee == sell_fee
    assert sell_execution.tax == sell_tax
    assert sell_execution.net_amount == sell_gross - sell_fee - sell_tax

    # Verify Trade net PnL
    trade = db_session.query(Trade).filter(Trade.session_id == session.id).first()
    assert trade.status == "closed"
    assert trade.result == "win"

    expected_net_pnl = sell_execution.net_amount - buy_execution.net_amount
    assert trade.net_pnl == expected_net_pnl
    assert trade.pnl_percent == (expected_net_pnl / buy_execution.net_amount) * 100

    # Verify Final Cash
    session = ReplayService.get_session(db_session, session.id)
    expected_cash = initial_cash - buy_execution.net_amount + sell_execution.net_amount
    assert session.current_cash == expected_cash


def test_multiple_round_trips_same_session_symbol_have_independent_net_pnl(db_session):
    symbol = "ROUNDTRIP_TEST"
    initial_cash = 100_000_000.0
    closes = [100.0, 101.0, 102.0, 101.0, 100.0, 99.0, 98.0]

    for i, close in enumerate(closes):
        db_session.add(Candle(
            symbol=symbol,
            timeframe="1D",
            timestamp=date(2024, 2, 1) + timedelta(days=i),
            open=close,
            high=close + 1,
            low=close - 1,
            close=close,
            volume=1000000,
        ))
    db_session.commit()

    session_in = ReplaySessionCreate(
        symbol=symbol,
        start_date=date(2024, 2, 1),
        end_date=date(2024, 2, 7),
        initial_cash=initial_cash,
    )
    session = ReplayService.create_session(db_session, session_in)

    qty = 100.0

    TradeLifecycleService.process_decision(
        db_session,
        session.id,
        DecisionCreate(action=DecisionAction.BUY, quantity=qty),
    )
    ReplayService.next_candle(db_session, session.id)
    ReplayService.next_candle(db_session, session.id)
    TradeLifecycleService.process_decision(
        db_session,
        session.id,
        DecisionCreate(action=DecisionAction.CLOSE),
    )

    TradeLifecycleService.process_decision(
        db_session,
        session.id,
        DecisionCreate(action=DecisionAction.BUY, quantity=qty),
    )
    ReplayService.next_candle(db_session, session.id)
    ReplayService.next_candle(db_session, session.id)
    TradeLifecycleService.process_decision(
        db_session,
        session.id,
        DecisionCreate(action=DecisionAction.CLOSE),
    )

    trades = db_session.query(Trade).filter_by(session_id=session.id).order_by(Trade.id).all()
    assert len(trades) == 2
    assert all(t.status == "closed" for t in trades)

    first_expected = (102.0 * qty * (1 - 0.0015 - 0.001)) - (100.0 * qty * (1 + 0.0015))
    second_expected = (100.0 * qty * (1 - 0.0015 - 0.001)) - (102.0 * qty * (1 + 0.0015))

    assert trades[0].net_pnl == pytest.approx(first_expected)
    assert trades[1].net_pnl == pytest.approx(second_expected)
    assert trades[0].result == "win"
    assert trades[1].result == "loss"

def test_tc004_hose_limit_above_ceiling_rejected(db_session):
    from app.models.symbol import Symbol
    db_session.add(Symbol(symbol="HOSE_TEST", exchange="HOSE"))
    db_session.add(Candle(symbol="HOSE_TEST", timeframe="1D", timestamp=date(2024, 1, 1), open=100.0, high=100.0, low=100.0, close=100.0, volume=1000))
    db_session.add(Candle(symbol="HOSE_TEST", timeframe="1D", timestamp=date(2024, 1, 2), open=100.0, high=100.0, low=100.0, close=100.0, volume=1000))
    db_session.commit()

    session_in = ReplaySessionCreate(symbol="HOSE_TEST", start_date=date(2024, 1, 1), end_date=date(2024, 1, 10))
    session = ReplayService.create_session(db_session, session_in)
    ReplayService.next_candle(db_session, session.id) # current_index = 1, ref_price = 100.0 (index 0)

    # 7% of 100 = 7. Ceiling = 107
    dec_buy = DecisionCreate(action=DecisionAction.BUY, quantity=1000, order_type="LIMIT", price=111.0)

    with pytest.raises(HTTPException) as excinfo:
        TradeLifecycleService.process_decision(db_session, session.id, dec_buy)

    assert excinfo.value.status_code == 400
    assert "nằm ngoài biên độ HOSE" in excinfo.value.detail

def test_tc004_hose_limit_within_band_pending_and_fills(db_session):
    from app.models.symbol import Symbol
    db_session.add(Symbol(symbol="HOSE_TEST2", exchange="HOSE"))
    # Index 0: ref close = 100
    db_session.add(Candle(symbol="HOSE_TEST2", timeframe="1D", timestamp=date(2024, 1, 1), open=100.0, high=100.0, low=100.0, close=100.0, volume=1000))
    # Index 1: current candle, but doesn't reach limit price
    db_session.add(Candle(symbol="HOSE_TEST2", timeframe="1D", timestamp=date(2024, 1, 2), open=101.0, high=102.0, low=100.0, close=101.0, volume=1000))
    # Index 2: candle drops to 105, which will fill our 105 limit order
    db_session.add(Candle(symbol="HOSE_TEST2", timeframe="1D", timestamp=date(2024, 1, 3), open=108.0, high=108.0, low=104.0, close=106.0, volume=1000))
    db_session.commit()

    session_in = ReplaySessionCreate(symbol="HOSE_TEST2", start_date=date(2024, 1, 1), end_date=date(2024, 1, 10))
    session = ReplayService.create_session(db_session, session_in)
    ReplayService.next_candle(db_session, session.id) # now at index 1

    # Limit BUY at 105. < 107 ceiling, so should be accepted as PENDING
    dec_buy = DecisionCreate(action=DecisionAction.BUY, quantity=1000, order_type="LIMIT", price=105.0)
    TradeLifecycleService.process_decision(db_session, session.id, dec_buy)

    # Check pending order
    order = db_session.query(Order).filter_by(session_id=session.id).first()
    assert order.status == "pending"
    assert order.requested_price == 105.0

    # Position should still be empty
    pos = db_session.query(Position).filter_by(session_id=session.id).first()
    assert pos is None

    # Next candle (index 2) low is 104, so 105 is hit!
    ReplayService.next_candle(db_session, session.id)

    # Check order is executed
    db_session.refresh(order)
    assert order.status == "executed"

    # Check position exists
    pos = db_session.query(Position).filter_by(session_id=session.id).first()
    assert pos is not None
    assert pos.status == "open"
    assert pos.quantity == 1000
