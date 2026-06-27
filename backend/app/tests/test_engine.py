import pytest
from datetime import datetime
from app.domain.engine.events import EventDispatcher, OrderEvent, MarketEvent, EventType
from app.domain.engine.broker import BrokerSimulation

def test_market_order_fill():
    dispatcher = EventDispatcher()
    broker = BrokerSimulation(dispatcher, initial_capital=10000.0)
    
    # Send a Market Buy Order
    order_event = OrderEvent(symbol="AAPL", order_type="MKT", quantity=10, direction="BUY")
    dispatcher.dispatch(order_event)
    
    assert len(broker.active_orders) == 1
    
    # Send a Market Event (New Candle)
    candle = {
        "timestamp": datetime.now(),
        "open": 150.0,
        "high": 155.0,
        "low": 149.0,
        "close": 154.0,
        "volume": 1000
    }
    market_event = MarketEvent(symbol="AAPL", data=candle)
    dispatcher.dispatch(market_event)
    
    # Order should be filled
    assert len(broker.active_orders) == 0
    position = broker.portfolio.get_position("AAPL")
    assert position.quantity == 10
    
    # Check slippage and commission applied
    # Slippage is 0.1% of open price (150.0 * 0.001 = 0.15) -> Exec price = 150.15
    # Cost = 150.15 * 10 = 1501.5
    # Comm = 1501.5 * 0.0015 = 2.25225
    # Total deduction = 1501.5 + 2.25225 = 1503.75225
    # Expected cash = 10000.0 - 1503.75225 = 8496.24775
    assert position.average_price == pytest.approx(150.15)
    assert abs(broker.portfolio.cash - 8496.24775) < 0.01

def test_limit_order_gap():
    dispatcher = EventDispatcher()
    broker = BrokerSimulation(dispatcher, initial_capital=10000.0)
    
    # Send a Buy Limit Order at 100.0
    order_event = OrderEvent(symbol="TSLA", order_type="LMT", quantity=5, direction="BUY", price=100.0)
    dispatcher.dispatch(order_event)
    
    # Gap down open at 90.0 (below limit of 100.0)
    candle = {
        "timestamp": datetime.now(),
        "open": 90.0,
        "high": 95.0,
        "low": 85.0,
        "close": 92.0,
        "volume": 1000
    }
    market_event = MarketEvent(symbol="TSLA", data=candle)
    dispatcher.dispatch(market_event)
    
    # Order should be filled at 90.0 (gap price) not 100.0
    position = broker.portfolio.get_position("TSLA")
    assert position.quantity == 5
    # Exec price = 90.0 * 1.001 (slippage) = 90.09
    assert position.average_price == pytest.approx(90.09)
    
def test_long_position_unrealized_pnl():
    dispatcher = EventDispatcher()
    broker = BrokerSimulation(dispatcher, initial_capital=10000.0)
    
    # Buy 10 shares at Market (assume open is 150.0)
    order_event = OrderEvent(symbol="MSFT", order_type="MKT", quantity=10, direction="BUY")
    dispatcher.dispatch(order_event)
    
    candle1 = {
        "timestamp": datetime.now(),
        "open": 150.0,
        "high": 155.0,
        "low": 145.0,
        "close": 150.0,
        "volume": 1000
    }
    dispatcher.dispatch(MarketEvent(symbol="MSFT", data=candle1))
    
    position = broker.portfolio.get_position("MSFT")
    assert position.quantity == 10
    # Exec price = 150.0 * 1.001 (slippage for buy) = 150.15
    assert position.average_price == pytest.approx(150.15)
    
    # Price rises to 200.0 (profit)
    candle2 = {
        "timestamp": datetime.now(),
        "open": 200.0,
        "high": 200.0,
        "low": 200.0,
        "close": 200.0,
        "volume": 1000
    }
    dispatcher.dispatch(MarketEvent(symbol="MSFT", data=candle2))
    
    # Check unrealized PnL: quantity is 10. Current price 200.0. Average price 150.15.
    # PnL = 10 * (200.0 - 150.15) = 10 * 49.85 = +498.5
    assert abs(position.unrealized_pnl - 498.5) < 0.01
