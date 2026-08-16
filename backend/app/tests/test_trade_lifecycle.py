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


@pytest.mark.parametrize(
    ("price", "quantity", "expected_detail"),
    [
        (-100.0, 100.0, "price"),
        (100.0, -100.0, "quantity"),
        (100.0, 0.0, "quantity"),
    ],
)
def test_buy_rejects_non_positive_execution_inputs(
    db_session, price, quantity, expected_detail
):
    candle = Candle(
        symbol="INVALID",
        timeframe="1D",
        timestamp=date(2024, 1, 2),
        open=100,
        high=101,
        low=99,
        close=100,
        volume=1000,
    )
    db_session.add(candle)
    db_session.commit()
    session = ReplayService.create_session(
        db_session,
        ReplaySessionCreate(
            symbol="INVALID",
            start_date=date(2024, 1, 2),
            end_date=date(2024, 1, 2),
            initial_cash=100_000,
        ),
    )

    with pytest.raises(HTTPException) as exc_info:
        TradeLifecycleService.process_decision(
            db_session,
            session.id,
            DecisionCreate(
                action=DecisionAction.BUY,
                price=price,
                quantity=quantity,
            ),
        )

    assert exc_info.value.status_code == 400
    assert expected_detail in exc_info.value.detail.lower()
    db_session.refresh(session)
    assert session.current_cash == pytest.approx(100_000)
    assert db_session.query(Order).filter_by(session_id=session.id).count() == 0
    assert db_session.query(Execution).filter_by(session_id=session.id).count() == 0

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


def test_buy_rejects_when_cash_is_insufficient(db_session):
    symbol = "CASH_LIMIT_TEST"
    initial_cash = 10_000.0
    db_session.add(Candle(
        symbol=symbol,
        timeframe="1D",
        timestamp=date(2024, 1, 1),
        open=100.0,
        high=101.0,
        low=99.0,
        close=100.0,
        volume=1000000,
        adjustment_type="unadjusted",
    ))
    db_session.commit()

    session = ReplayService.create_session(
        db_session,
        ReplaySessionCreate(
            symbol=symbol,
            start_date=date(2024, 1, 1),
            end_date=date(2024, 1, 1),
            initial_cash=initial_cash,
        ),
    )

    with pytest.raises(HTTPException) as excinfo:
        TradeLifecycleService.process_decision(
            db_session,
            session.id,
            DecisionCreate(action=DecisionAction.BUY, quantity=200),
        )

    assert excinfo.value.status_code == 400
    assert "insufficient cash" in excinfo.value.detail.lower()
    assert db_session.query(Order).filter_by(session_id=session.id).count() == 0
    assert db_session.query(Position).filter_by(session_id=session.id).count() == 0
    db_session.refresh(session)
    assert session.current_cash == initial_cash


def test_close_without_open_position_is_rejected(db_session):
    symbol = "EMPTY_CLOSE_TEST"
    db_session.add(Candle(
        symbol=symbol,
        timeframe="1D",
        timestamp=date(2024, 1, 1),
        open=100.0,
        high=101.0,
        low=99.0,
        close=100.0,
        volume=1000000,
        adjustment_type="unadjusted",
    ))
    db_session.commit()

    session = ReplayService.create_session(
        db_session,
        ReplaySessionCreate(
            symbol=symbol,
            start_date=date(2024, 1, 1),
            end_date=date(2024, 1, 1),
        ),
    )

    with pytest.raises(HTTPException) as excinfo:
        TradeLifecycleService.process_decision(
            db_session,
            session.id,
            DecisionCreate(action=DecisionAction.CLOSE),
        )

    assert excinfo.value.status_code == 400
    assert "no open position" in excinfo.value.detail.lower()
    assert db_session.query(Order).filter_by(session_id=session.id).count() == 0
    assert db_session.query(Trade).filter_by(session_id=session.id).count() == 0


def test_partial_reduce_keeps_trade_open_and_updates_cash(db_session):
    symbol = "PARTIAL_REDUCE_TEST"
    initial_cash = 100_000_000.0
    closes = [100.0, 101.0, 110.0, 111.0]

    for i, close in enumerate(closes):
        db_session.add(Candle(
            symbol=symbol,
            timeframe="1D",
            timestamp=date(2024, 3, 1) + timedelta(days=i),
            open=close,
            high=close + 1,
            low=close - 1,
            close=close,
            volume=1000000,
            adjustment_type="unadjusted",
        ))
    db_session.commit()

    session = ReplayService.create_session(
        db_session,
        ReplaySessionCreate(
            symbol=symbol,
            start_date=date(2024, 3, 1),
            end_date=date(2024, 3, 4),
            initial_cash=initial_cash,
        ),
    )

    TradeLifecycleService.process_decision(
        db_session,
        session.id,
        DecisionCreate(action=DecisionAction.BUY, quantity=100),
    )
    ReplayService.next_candle(db_session, session.id)
    ReplayService.next_candle(db_session, session.id)
    TradeLifecycleService.process_decision(
        db_session,
        session.id,
        DecisionCreate(action=DecisionAction.REDUCE, quantity=40),
    )

    position = db_session.query(Position).filter_by(session_id=session.id).first()
    assert position.status == "open"
    assert position.quantity == 60
    assert position.realized_pnl == pytest.approx((110.0 - 100.0) * 40)

    trade = db_session.query(Trade).filter_by(session_id=session.id).first()
    assert trade.status == "open"
    assert trade.exit_date is None
    assert trade.quantity == 100

    buy_execution = db_session.query(Execution).join(Order).filter(
        Execution.session_id == session.id,
        Order.side == "BUY",
    ).first()
    sell_execution = db_session.query(Execution).join(Order).filter(
        Execution.session_id == session.id,
        Order.side == "SELL",
    ).first()
    db_session.refresh(session)
    assert session.current_cash == pytest.approx(
        initial_cash - buy_execution.net_amount + sell_execution.net_amount
    )


def test_force_liquidation_closes_trade_with_fee_tax_net_pnl(db_session):
    symbol = "FORCE_LIQUIDATE_TEST"
    initial_cash = 100_000_000.0
    db_session.add(Candle(
        symbol=symbol,
        timeframe="1D",
        timestamp=date(2024, 4, 1),
        open=100.0,
        high=101.0,
        low=99.0,
        close=100.0,
        volume=1000000,
        adjustment_type="unadjusted",
    ))
    liquidation_candle = Candle(
        symbol=symbol,
        timeframe="1D",
        timestamp=date(2024, 4, 2),
        open=80.0,
        high=81.0,
        low=79.0,
        close=80.0,
        volume=1000000,
        adjustment_type="unadjusted",
    )
    db_session.add(liquidation_candle)
    db_session.commit()

    session = ReplayService.create_session(
        db_session,
        ReplaySessionCreate(
            symbol=symbol,
            start_date=date(2024, 4, 1),
            end_date=date(2024, 4, 2),
            initial_cash=initial_cash,
        ),
    )
    TradeLifecycleService.process_decision(
        db_session,
        session.id,
        DecisionCreate(action=DecisionAction.BUY, quantity=1000),
    )
    buy_execution = db_session.query(Execution).join(Order).filter(
        Execution.session_id == session.id,
        Order.side == "BUY",
    ).first()

    session.current_index = 1
    TradeLifecycleService.force_liquidate_all(db_session, session, liquidation_candle)

    sell_execution = db_session.query(Execution).join(Order).filter(
        Execution.session_id == session.id,
        Order.side == "SELL",
    ).first()
    trade = db_session.query(Trade).filter(Trade.session_id == session.id).first()
    position = db_session.query(Position).filter(Position.session_id == session.id).first()

    assert sell_execution.trade_id == trade.id
    assert trade.status == "closed"
    assert trade.result == "loss"
    assert trade.net_pnl == pytest.approx(sell_execution.net_amount - buy_execution.net_amount)
    assert trade.gross_pnl == pytest.approx((80.0 - 100.0) * 1000)
    assert position.status == "closed"
    assert position.quantity == 0


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


def test_trade_planning_and_rich_taxonomy_recorded(db_session):
    """
    Verify PRO-TRADE-01, PRO-TRADE-06, PRO-TRADE-07, PRO-TRADE-08:
    A plan records entry, stop, target, direction, account risk, planned quantity, fees, and expected R multiple.
    Taxonomy tags and checklist snapshots are persisted immutably and compared.
    """
    symbol = "PLAN_TEST"
    closes = [50.0, 52.0, 55.0]
    for i, close in enumerate(closes):
        db_session.add(Candle(
            symbol=symbol, timeframe="1D", timestamp=date(2024, 5, 1) + timedelta(days=i),
            open=close, high=close + 1, low=close - 1, close=close, volume=1000000,
            adjustment_type="unadjusted"
        ))
    db_session.commit()

    session = ReplayService.create_session(
        db_session,
        ReplaySessionCreate(symbol=symbol, start_date=date(2024, 5, 1), end_date=date(2024, 5, 3), initial_cash=100_000_000.0)
    )

    decision_in = DecisionCreate(
        action=DecisionAction.BUY,
        quantity=500.0,
        price=50.0,
        stop_loss=45.0,
        target_price=60.0,
        planned_quantity=500.0,
        setup_type="Breakout",
        market_regime="Bull Trend",
        confidence_score=4,
        emotion="Calm / Disciplined",
        mistake_tag="None",
        rule_violation="None",
        reason="Clean cup-and-handle breakout on high volume",
        note="Plan to trail stop after 1R",
        checklist_snapshot='{"checks": {"trendIdentified": true, "setupConfirmed": true}}',
    )
    decision = TradeLifecycleService.process_decision(db_session, session.id, decision_in)

    assert decision.stop_loss == 45.0
    assert decision.target_price == 60.0
    assert decision.planned_quantity == 500.0
    assert decision.planned_risk == (50.0 - 45.0) * 500.0 # 2,500
    assert decision.planned_r == (60.0 - 50.0) / (50.0 - 45.0) # 2.0
    assert decision.setup_type == "Breakout"
    assert decision.market_regime == "Bull Trend"
    assert decision.confidence_score == 4
    assert decision.emotion == "Calm / Disciplined"
    assert decision.checklist_snapshot is not None

    trade = db_session.query(Trade).filter_by(session_id=session.id).first()
    assert trade is not None
    assert trade.planned_entry_price == 50.0
    assert trade.planned_quantity == 500.0
    assert trade.initial_stop_loss == 45.0
    assert trade.target_price == 60.0
    assert trade.planned_r == 2.0
    assert trade.setup_type == "Breakout"
    assert trade.market_regime == "Bull Trend"
    assert trade.emotion == "Calm / Disciplined"

    # Advance to T+2 and close trade at 55.0
    ReplayService.next_candle(db_session, session.id)
    ReplayService.next_candle(db_session, session.id)

    close_decision = DecisionCreate(action=DecisionAction.CLOSE)
    TradeLifecycleService.process_decision(db_session, session.id, close_decision)

    db_session.refresh(trade)
    assert trade.status == "closed"
    assert trade.exit_price == 55.0
    # Gross profit = (55 - 50) * 500 = 2,500. Initial risk = (50 - 45) * 500 = 2,500
    assert trade.r_multiple is not None
    # Net PnL is close to 2,500 minus fees/taxes (~2,400) -> R is ~0.96
    assert trade.r_multiple == pytest.approx(trade.net_pnl / trade.initial_risk, abs=0.001)

    # Check practice workflow snapshot projection
    from app.services.practice_workflow_service import PracticeWorkflowService
    snapshot = PracticeWorkflowService.get_snapshot(db_session, session.id)
    assert len(snapshot.trades) == 1
    p_trade = snapshot.trades[0]
    assert p_trade.setup_type == "Breakout"
    assert p_trade.planned_r == 2.0
    assert p_trade.entry_drift == 0.0 # 50 - 50
    assert p_trade.size_variance == 0.0 # 500 - 500
    assert p_trade.r_multiple == pytest.approx(trade.r_multiple)


def test_t2_settlement_detailed_rejection_feedback(db_session):
    """
    Verify PRO-TRADE-05: T+2 availability and rejection feedback state the blocked quantity,
    available quantity, and release date.
    """
    symbol = "T2_FEEDBACK_TEST"
    for i in range(5):
        db_session.add(Candle(
            symbol=symbol, timeframe="1D", timestamp=date(2024, 6, 1) + timedelta(days=i),
            open=100.0, high=101.0, low=99.0, close=100.0, volume=1000000,
            adjustment_type="unadjusted"
        ))
    db_session.commit()

    session = ReplayService.create_session(
        db_session,
        ReplaySessionCreate(symbol=symbol, start_date=date(2024, 6, 1), end_date=date(2024, 6, 5), initial_cash=100_000_000.0)
    )

    # Buy 200 at Bar 1 (T0)
    TradeLifecycleService.process_decision(db_session, session.id, DecisionCreate(action=DecisionAction.BUY, quantity=200))
    # Step to Bar 2 (T1)
    ReplayService.next_candle(db_session, session.id)

    # Attempt to sell at T1 -> Rejected with detailed message
    with pytest.raises(HTTPException) as excinfo:
        TradeLifecycleService.process_decision(db_session, session.id, DecisionCreate(action=DecisionAction.SELL, quantity=100))

    assert excinfo.value.status_code == 400
    detail = excinfo.value.detail
    assert "T+2 constraint" in detail
    assert "Available: 0" in detail
    assert "Blocked: 200" in detail
    assert "Earliest release date: 2024-06-03" in detail


def test_journal_json_and_csv_export(db_session):
    """
    Verify PRO-TRADE-10: Journal export and backup preserve local privacy and stable identifiers.
    """
    symbol = "EXPORT_TEST"
    for i in range(4):
        db_session.add(Candle(
            symbol=symbol, timeframe="1D", timestamp=date(2024, 7, 1) + timedelta(days=i),
            open=100.0, high=101.0, low=99.0, close=100.0, volume=1000000,
            adjustment_type="unadjusted"
        ))
    db_session.commit()

    session = ReplayService.create_session(
        db_session,
        ReplaySessionCreate(symbol=symbol, start_date=date(2024, 7, 1), end_date=date(2024, 7, 4), initial_cash=100_000_000.0)
    )

    # Add a decision and trade
    TradeLifecycleService.process_decision(
        db_session, session.id,
        DecisionCreate(
            action=DecisionAction.BUY, quantity=300, price=100.0,
            stop_loss=90.0, target_price=120.0, setup_type="Pullback", market_regime="Bull Trend",
            emotion="Disciplined", reason="Support bounce"
        )
    )

    # Add a journal entry
    from app.services.journal_service import JournalService
    from app.schemas.journal_schema import JournalEntryCreate
    JournalService.create(
        db_session, session.id,
        JournalEntryCreate(
            note_type="session_review",
            content="Great execution following daily trading plan.",
            tags="discipline,plan",
            setup_type="Pullback",
            market_regime="Bull Trend",
            confidence_score=5,
            emotion="Disciplined",
        )
    )

    # Export JSON
    json_export = JournalService.export_session_journal_json(db_session, session.id)
    assert json_export["schema_version"] == 1
    assert json_export["session"]["symbol"] == symbol
    assert len(json_export["decisions"]) == 1
    assert len(json_export["trades"]) == 1
    assert len(json_export["journal_entries"]) == 1
    assert json_export["decisions"][0]["setup_type"] == "Pullback"

    # Export CSV
    csv_export = JournalService.export_session_journal_csv(db_session, session.id)
    assert "# TRADES" in csv_export
    assert "# JOURNAL ENTRIES" in csv_export
    assert "Pullback" in csv_export
    assert "Disciplined" in csv_export
