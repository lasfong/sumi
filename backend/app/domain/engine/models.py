from dataclasses import dataclass, field
from datetime import datetime
from typing import Dict, List, Optional
import uuid

@dataclass
class EngineOrder:
    symbol: str
    order_type: str # 'MKT', 'LMT', 'STP'
    direction: str # 'BUY', 'SELL'
    quantity: float
    price: Optional[float] = None
    stop_price: Optional[float] = None
    status: str = 'CREATED' # 'CREATED', 'SUBMITTED', 'FILLED', 'CANCELLED', 'REJECTED'
    id: str = field(default_factory=lambda: str(uuid.uuid4()))
    created_at: datetime = field(default_factory=datetime.now)
    filled_at: Optional[datetime] = None
    fill_price: Optional[float] = None
    commission: float = 0.0
    slippage: float = 0.0

@dataclass
class EnginePosition:
    symbol: str
    quantity: float = 0.0
    average_price: float = 0.0
    realized_pnl: float = 0.0
    unrealized_pnl: float = 0.0
    buys: List[Dict] = field(default_factory=list) # [{'quantity': 100, 'bar_index': 5}]
    
    def get_available_quantity(self, current_bar_index: int) -> float:
        """Calculate quantity available to sell (T+2)"""
        from app.domain.engine.market_constraints import MarketConstraints
        available = 0.0
        for buy in self.buys:
            if MarketConstraints.calculate_t_plus_2_ready(current_bar_index, buy['bar_index']):
                available += buy['quantity']
        return available
        
    def update(self, direction: str, fill_price: float, quantity: float, bar_index: int = 0):
        q_sign = 1.0 if direction == 'BUY' else -1.0
        trade_qty = q_sign * quantity
        
        is_opening = (self.quantity >= 0 and direction == 'BUY') or (self.quantity <= 0 and direction == 'SELL')
        
        if is_opening:
            total_cost = self.average_price * self.quantity + fill_price * trade_qty
            self.quantity += trade_qty
            self.average_price = total_cost / self.quantity if self.quantity != 0 else 0.0
            
            if direction == 'BUY':
                self.buys.append({'quantity': quantity, 'bar_index': bar_index})
        else:
            if abs(trade_qty) > abs(self.quantity):
                closed_qty = self.quantity
                remaining_qty = trade_qty + self.quantity
                
                self.realized_pnl += closed_qty * (fill_price - self.average_price)
                
                self.quantity = remaining_qty
                self.average_price = fill_price
                
                if direction == 'SELL':
                    # We just closed the whole long position and went short
                    self.buys.clear()
            else:
                self.quantity += trade_qty
                self.realized_pnl += (-trade_qty) * (fill_price - self.average_price)
                
                if self.quantity == 0:
                    self.average_price = 0.0
                    self.buys.clear()
                elif direction == 'SELL':
                    # Reduce buys FIFO
                    qty_to_reduce = quantity
                    for buy in self.buys:
                        if buy['quantity'] <= qty_to_reduce:
                            qty_to_reduce -= buy['quantity']
                            buy['quantity'] = 0
                        else:
                            buy['quantity'] -= qty_to_reduce
                            qty_to_reduce = 0
                            break
                    self.buys = [b for b in self.buys if b['quantity'] > 0]

    def update_unrealized_pnl(self, current_price: float):
        if self.quantity != 0:
            self.unrealized_pnl = self.quantity * (current_price - self.average_price)
        else:
            self.unrealized_pnl = 0.0

class Portfolio:
    def __init__(self, initial_capital: float = 100000.0):
        self.initial_capital = initial_capital
        self.cash = initial_capital
        self.positions: Dict[str, EnginePosition] = {}
        self.equity = initial_capital
        
    def get_position(self, symbol: str) -> EnginePosition:
        if symbol not in self.positions:
            self.positions[symbol] = EnginePosition(symbol=symbol)
        return self.positions[symbol]

    def update_from_fill(self, fill_event):
        position = self.get_position(fill_event.symbol)
        
        # Calculate cost
        cost = fill_event.fill_price * fill_event.quantity
        commission = fill_event.commission
        
        if fill_event.direction == 'BUY':
            self.cash -= (cost + commission)
        else:
            self.cash += (cost - commission)
            
        position.update(fill_event.direction, fill_event.fill_price, fill_event.quantity, getattr(fill_event, 'bar_index', 0))

    def update_market_price(self, symbol: str, current_price: float):
        if symbol in self.positions:
            self.positions[symbol].update_unrealized_pnl(current_price)
            
        self._recalculate_equity()
        
    def _recalculate_equity(self):
        total_unrealized = sum(p.unrealized_pnl for p in self.positions.values())
        position_value = sum(abs(p.quantity) * p.average_price for p in self.positions.values()) + total_unrealized
        self.equity = self.cash + position_value
