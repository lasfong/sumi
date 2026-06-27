from enum import Enum
from typing import Any, Callable, Dict, List
from datetime import datetime

class EventType(Enum):
    MARKET = "MARKET"       # New market data available
    SIGNAL = "SIGNAL"       # Strategy generated a signal
    ORDER = "ORDER"         # Order sent to broker
    FILL = "FILL"           # Order filled by broker

class Event:
    """Base Event class"""
    def __init__(self, event_type: EventType):
        self.type = event_type
        self.timestamp = datetime.now()

class MarketEvent(Event):
    """
    Handles the event of receiving a new market update (e.g. a new candle).
    """
    def __init__(self, symbol: str, data: Dict[str, Any]):
        super().__init__(EventType.MARKET)
        self.symbol = symbol
        self.data = data # Dictionary representing a candle: open, high, low, close, volume, timestamp

class SignalEvent(Event):
    """
    Handles the event of a Strategy sending a signal.
    """
    def __init__(self, symbol: str, strategy_id: str, signal_type: str, strength: float = 1.0):
        super().__init__(EventType.SIGNAL)
        self.symbol = symbol
        self.strategy_id = strategy_id
        self.signal_type = signal_type # 'LONG', 'SHORT', 'EXIT'
        self.strength = strength

class OrderEvent(Event):
    """
    Handles the event of an order being sent to the execution system.
    """
    def __init__(self, symbol: str, order_type: str, quantity: float, direction: str, price: float = None, stop_price: float = None):
        super().__init__(EventType.ORDER)
        self.symbol = symbol
        self.order_type = order_type # 'MKT', 'LMT', 'STP'
        self.quantity = quantity
        self.direction = direction # 'BUY', 'SELL'
        self.price = price
        self.stop_price = stop_price

class FillEvent(Event):
    """
    Encapsulates the notion of a Filled Order.
    """
    def __init__(self, symbol: str, exchange: str, quantity: float, direction: str, fill_price: float, commission: float = 0.0, slippage: float = 0.0, bar_index: int = 0):
        super().__init__(EventType.FILL)
        self.symbol = symbol
        self.exchange = exchange
        self.quantity = quantity
        self.direction = direction
        self.fill_price = fill_price
        self.commission = commission
        self.slippage = slippage
        self.bar_index = bar_index

class EventDispatcher:
    """
    Simple publish/subscribe event dispatcher.
    """
    def __init__(self):
        self.handlers: Dict[EventType, List[Callable[[Event], None]]] = {
            event_type: [] for event_type in EventType
        }
        
    def subscribe(self, event_type: EventType, handler: Callable[[Event], None]):
        if handler not in self.handlers[event_type]:
            self.handlers[event_type].append(handler)
            
    def unsubscribe(self, event_type: EventType, handler: Callable[[Event], None]):
        if handler in self.handlers[event_type]:
            self.handlers[event_type].remove(handler)
            
    def dispatch(self, event: Event):
        for handler in self.handlers[event.type]:
            handler(event)
