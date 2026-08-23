import pytest
from datetime import datetime, timedelta
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from fastapi import HTTPException
import sys
import os

from app.db import Base
from app.models.symbol import Symbol
from app.models.replay_session import ReplaySession
from app.models.decision import Decision
from app.models.order import Order
from app.models.execution import Execution
from app.models.position import Position
from app.models.trade import Trade
from app.services.trade_lifecycle_service import TradeLifecycleService
from app.schemas.decision_schema import DecisionCreate
from app.domain.enums import DecisionAction, OrderType, OrderSide

# Test models/mock objects
class MockCandle:
    def __init__(self, index, close=100.0, timestamp=None):
        self.index = index
        self.close = close
        self.timestamp = timestamp or datetime.now()

class MockSession:
    def __init__(self, id, symbol="FPT", current_cash=100000000, current_index=0):
        self.id = id
        self.symbol = symbol
        self.current_cash = current_cash
        self.current_index = current_index

@pytest.fixture
def db_session():
    engine = create_engine("sqlite:///:memory:")
    Base.metadata.create_all(engine)
    SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)
    db = SessionLocal()
    
    # Add a dummy symbol
    db.add(Symbol(symbol="FPT", exchange="HOSE"))
    db.commit()
    
    yield db
    db.close()

from app.models.candle import Candle

def create_mock_setup(db, session_id=1):
    start_d = datetime.now() - timedelta(days=30)
    end_d = datetime.now() + timedelta(days=30)
    # Just need the records in DB
    replay_session = ReplaySession(id=session_id, symbol="FPT", timeframe="1D", adjustment_type="unadjusted", start_date=start_d, end_date=end_d, initial_cash=100000000, current_cash=100000000, current_index=0)
    db.add(replay_session)
    
    # Add dummy candles
    for i in range(20):
        c = Candle(symbol="FPT", timeframe="1D", timestamp=start_d + timedelta(days=i), open=100, high=100, low=100, close=100, volume=1000, adjustment_type="unadjusted")
        db.add(c)
        
    db.commit()
    return replay_session

def test_market_buy_sell_rejected(db_session):
    # 1. Market Buy at bar 10, sell at bar 11 → rejected
    msession = create_mock_setup(db_session)
    msession.current_index = 10
    
    # Buy at bar 10
    decision_buy = DecisionCreate(action=DecisionAction.BUY, order_type=OrderType.MARKET_AT_CLOSE.value, quantity=100, price=50.0)
    
    # Mocking process_decision directly is complex because it calls ReplayService.get_session which we don't have.
    # Let's call _execute_buy directly.
    # We need a decision record first.
    dec_buy = Decision(session_id=msession.id, symbol="FPT", decision_date=datetime.now(), candle_index=10, action=DecisionAction.BUY.value)
    db_session.add(dec_buy)
    db_session.commit()
    
    TradeLifecycleService._execute_buy(db_session, msession, dec_buy.id, decision_buy, "FPT", datetime.now(), 50.0, 100)
    db_session.commit()
    
    # Sell at bar 11
    msession.current_index = 11
    dec_sell = Decision(session_id=msession.id, symbol="FPT", decision_date=datetime.now(), candle_index=11, action=DecisionAction.SELL.value)
    db_session.add(dec_sell)
    db_session.commit()
    
    decision_sell = DecisionCreate(action=DecisionAction.SELL, order_type=OrderType.MARKET_AT_CLOSE.value, quantity=100, price=52.0)
    
    with pytest.raises(HTTPException) as excinfo:
        TradeLifecycleService._execute_sell(db_session, msession, dec_sell.id, decision_sell, "FPT", datetime.now(), 52.0, 100)
    
    assert "Cannot sell: T+2 constraint" in str(excinfo.value.detail)


def test_market_buy_sell_accepted(db_session):
    # 2. Market Buy at bar 10, sell at bar 12 → accepted
    msession = create_mock_setup(db_session)
    msession.current_index = 10
    
    dec_buy = Decision(session_id=msession.id, symbol="FPT", decision_date=datetime.now(), candle_index=10, action=DecisionAction.BUY.value)
    db_session.add(dec_buy)
    db_session.commit()
    
    decision_buy = DecisionCreate(action=DecisionAction.BUY, order_type=OrderType.MARKET_AT_CLOSE.value, quantity=100, price=50.0)
    TradeLifecycleService._execute_buy(db_session, msession, dec_buy.id, decision_buy, "FPT", datetime.now(), 50.0, 100)
    db_session.commit()
    
    msession.current_index = 12
    dec_sell = Decision(session_id=msession.id, symbol="FPT", decision_date=datetime.now(), candle_index=12, action=DecisionAction.SELL.value)
    db_session.add(dec_sell)
    db_session.commit()
    
    decision_sell = DecisionCreate(action=DecisionAction.SELL, order_type=OrderType.MARKET_AT_CLOSE.value, quantity=100, price=52.0)
    
    # Should not raise
    TradeLifecycleService._execute_sell(db_session, msession, dec_sell.id, decision_sell, "FPT", datetime.now(), 52.0, 100)
    db_session.commit()
    
    pos = db_session.query(Position).first()
    assert pos.quantity == 0


def test_limit_buy_rejected(db_session):
    # 3. Limit Buy placed at bar 10, fills at bar 13, sell at bar 14 → rejected
    msession = create_mock_setup(db_session)
    msession.current_index = 10
    
    dec_buy = Decision(session_id=msession.id, symbol="FPT", decision_date=datetime.now(), candle_index=10, action=DecisionAction.BUY.value)
    db_session.add(dec_buy)
    db_session.commit()
    
    order_buy = Order(session_id=msession.id, decision_id=dec_buy.id, symbol="FPT", side=OrderSide.BUY.value, order_type=OrderType.LIMIT.value, requested_price=48.0, quantity=100, status="PENDING")
    db_session.add(order_buy)
    db_session.commit()
    
    # Fills at bar 13
    msession.current_index = 13
    TradeLifecycleService.execute_pending_order(db_session, msession, order_buy, 48.0, MockCandle(13))
    db_session.commit()
    
    # Sell at bar 14
    msession.current_index = 14
    dec_sell = Decision(session_id=msession.id, symbol="FPT", decision_date=datetime.now(), candle_index=14, action=DecisionAction.SELL.value)
    db_session.add(dec_sell)
    db_session.commit()
    
    decision_sell = DecisionCreate(action=DecisionAction.SELL, order_type=OrderType.MARKET_AT_CLOSE.value, quantity=100, price=50.0)
    
    with pytest.raises(HTTPException) as excinfo:
        TradeLifecycleService._execute_sell(db_session, msession, dec_sell.id, decision_sell, "FPT", datetime.now(), 50.0, 100)
    assert "Cannot sell: T+2 constraint" in str(excinfo.value.detail)


def test_limit_buy_accepted(db_session):
    # 4. Limit Buy placed at bar 10, fills at bar 13, sell at bar 15 → accepted
    msession = create_mock_setup(db_session)
    msession.current_index = 10
    
    dec_buy = Decision(session_id=msession.id, symbol="FPT", decision_date=datetime.now(), candle_index=10, action=DecisionAction.BUY.value)
    db_session.add(dec_buy)
    db_session.commit()
    
    order_buy = Order(session_id=msession.id, decision_id=dec_buy.id, symbol="FPT", side=OrderSide.BUY.value, order_type=OrderType.LIMIT.value, requested_price=48.0, quantity=100, status="PENDING")
    db_session.add(order_buy)
    db_session.commit()
    
    # Fills at bar 13
    msession.current_index = 13
    TradeLifecycleService.execute_pending_order(db_session, msession, order_buy, 48.0, MockCandle(13))
    db_session.commit()
    
    # Sell at bar 15
    msession.current_index = 15
    dec_sell = Decision(session_id=msession.id, symbol="FPT", decision_date=datetime.now(), candle_index=15, action=DecisionAction.SELL.value)
    db_session.add(dec_sell)
    db_session.commit()
    
    decision_sell = DecisionCreate(action=DecisionAction.SELL, order_type=OrderType.MARKET_AT_CLOSE.value, quantity=100, price=50.0)
    
    # Should not raise
    TradeLifecycleService._execute_sell(db_session, msession, dec_sell.id, decision_sell, "FPT", datetime.now(), 50.0, 100)
    db_session.commit()
    
    pos = db_session.query(Position).first()
    assert pos.quantity == 0


def test_mixed_scenario(db_session):
    # 5. Mixed scenario: Market Buy at bar 5 + Limit Buy fills at bar 10, sell all at bar 12 → partial
    msession = create_mock_setup(db_session)
    
    # Market Buy at 5
    msession.current_index = 5
    dec_buy1 = Decision(session_id=msession.id, symbol="FPT", decision_date=datetime.now(), candle_index=5, action=DecisionAction.BUY.value)
    db_session.add(dec_buy1)
    db_session.commit()
    decision_buy1 = DecisionCreate(action=DecisionAction.BUY, order_type=OrderType.MARKET_AT_CLOSE.value, quantity=100, price=50.0)
    TradeLifecycleService._execute_buy(db_session, msession, dec_buy1.id, decision_buy1, "FPT", datetime.now(), 50.0, 100)
    db_session.commit()
    
    # Limit Buy placed at 5
    dec_buy2 = Decision(session_id=msession.id, symbol="FPT", decision_date=datetime.now(), candle_index=5, action=DecisionAction.BUY.value)
    db_session.add(dec_buy2)
    db_session.commit()
    order_buy2 = Order(session_id=msession.id, decision_id=dec_buy2.id, symbol="FPT", side=OrderSide.BUY.value, order_type=OrderType.LIMIT.value, requested_price=48.0, quantity=100, status="PENDING")
    db_session.add(order_buy2)
    db_session.commit()
    
    # Limit Buy fills at 10
    msession.current_index = 10
    TradeLifecycleService.execute_pending_order(db_session, msession, order_buy2, 48.0, MockCandle(10))
    db_session.commit()
    
    # Sell 200 at bar 11
    msession.current_index = 11
    dec_sell = Decision(session_id=msession.id, symbol="FPT", decision_date=datetime.now(), candle_index=11, action=DecisionAction.SELL.value)
    db_session.add(dec_sell)
    db_session.commit()
    
    decision_sell_all = DecisionCreate(action=DecisionAction.SELL, order_type=OrderType.MARKET_AT_CLOSE.value, quantity=200, price=52.0)
    
    with pytest.raises(HTTPException) as excinfo:
        TradeLifecycleService._execute_sell(db_session, msession, dec_sell.id, decision_sell_all, "FPT", datetime.now(), 52.0, 200)
    
    assert "Cannot sell: T+2 constraint" in str(excinfo.value.detail)
    assert "Available: 100" in str(excinfo.value.detail)
    assert "Blocked: 100" in str(excinfo.value.detail)

