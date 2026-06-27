from abc import ABC, abstractmethod
from typing import Dict, Any, List
from app.domain.engine.events import EventDispatcher, OrderEvent
from app.domain.engine.models import Portfolio, EnginePosition

class BaseStrategy(ABC):
    """
    Base class for trading strategies.
    Strategies process market data and generate order events.
    """
    def __init__(self, dispatcher: EventDispatcher, portfolio: Portfolio):
        self.dispatcher = dispatcher
        self.portfolio = portfolio
        self.params: Dict[str, Any] = {}
        
    def set_params(self, **kwargs):
        """Set strategy parameters for optimization"""
        self.params.update(kwargs)

    @abstractmethod
    def next(self, symbol: str, bar: Dict[str, Any]):
        """
        Called on every new bar of data.
        Must be implemented by the user.
        """
        pass

    def get_position(self, symbol: str) -> EnginePosition:
        """Helper to get current position for a symbol"""
        return self.portfolio.get_position(symbol)

    def buy(self, symbol: str, quantity: float, order_type: str = 'MKT', price: float = None, stop_price: float = None):
        """Helper to generate a buy order event"""
        event = OrderEvent(
            symbol=symbol,
            order_type=order_type,
            quantity=quantity,
            direction='BUY',
            price=price,
            stop_price=stop_price
        )
        self.dispatcher.dispatch(event)

    def sell(self, symbol: str, quantity: float, order_type: str = 'MKT', price: float = None, stop_price: float = None):
        """Helper to generate a sell order event"""
        event = OrderEvent(
            symbol=symbol,
            order_type=order_type,
            quantity=quantity,
            direction='SELL',
            price=price,
            stop_price=stop_price
        )
        self.dispatcher.dispatch(event)
        
    def close(self, symbol: str):
        """Helper to close an existing position"""
        pos = self.get_position(symbol)
        if pos.quantity > 0:
            self.sell(symbol, pos.quantity)
        elif pos.quantity < 0:
            self.buy(symbol, abs(pos.quantity))

    @classmethod
    def optimize(cls, data_feed, param_grid: Dict[str, List[Any]]):
        """
        Basic grid search optimizer.
        In a real scenario, this would use itertools.product to test all combinations.
        """
        import itertools
        
        keys = param_grid.keys()
        values = param_grid.values()
        
        combinations = list(itertools.product(*values))
        results = []
        
        print(f"Starting optimization with {len(combinations)} combinations...")
        
        # This is just a stub for phase 3. Full backtest execution logic per combination
        # is required to compute SQN or PnL.
        
        for combo in combinations:
            params = dict(zip(keys, combo))
            # 1. Setup new broker and dispatcher
            # 2. Instantiate strategy with params
            # 3. Feed all historical data
            # 4. Calculate SQN
            # 5. Append to results
            
        return results
