from typing import Dict, List, Optional
from datetime import datetime
from app.domain.engine.events import EventDispatcher, EventType, OrderEvent, FillEvent, MarketEvent
from app.domain.engine.models import EngineOrder, Portfolio
from app.domain.engine.market_constraints import MarketConstraints

class SlippageModel:
    def __init__(self, pct: float = 0.0):
        self.pct = pct
        
    def apply(self, price: float, direction: str) -> float:
        if direction == 'BUY':
            return price * (1 + self.pct)
        else:
            return price * (1 - self.pct)

class CommissionModel:
    def __init__(self, pct: float = 0.0, fixed: float = 0.0):
        self.pct = pct
        self.fixed = fixed
        
    def calculate(self, price: float, quantity: float) -> float:
        return (price * quantity * self.pct) + self.fixed

class BrokerSimulation:
    def __init__(self, dispatcher: EventDispatcher, initial_capital: float = 100000.0, exchange: str = 'HOSE'):
        self.dispatcher = dispatcher
        self.portfolio = Portfolio(initial_capital)
        self.active_orders: List[EngineOrder] = []
        self.slippage_model = SlippageModel(pct=0.001) 
        # Cập nhật phí theo chuẩn VN: Mua 0.15%, Bán 0.15% + Thuế 0.1% = 0.25%
        self.buy_commission = CommissionModel(pct=0.0015) 
        self.sell_commission = CommissionModel(pct=0.0025) 
        self.exchange = exchange
        
        # Subscribe to events
        self.dispatcher.subscribe(EventType.ORDER, self.on_order)
        self.dispatcher.subscribe(EventType.MARKET, self.on_market)
        
        # Keep track of market data
        self.latest_market_data: Dict[str, Dict] = {}
        self.previous_market_data: Dict[str, Dict] = {}
        self.symbol_bar_index: Dict[str, int] = {} # Dùng đếm số phiên để tính T+2

    def on_order(self, event: OrderEvent):
        """Handle incoming order from strategy"""
        order = EngineOrder(
            symbol=event.symbol,
            order_type=event.order_type,
            direction=event.direction,
            quantity=event.quantity,
            price=event.price,
            stop_price=event.stop_price,
            status='SUBMITTED'
        )
        
        # Kiểm tra trước (Pre-trade checks)
        # 1. Kiểm tra T+2 cho lệnh bán
        if order.direction == 'SELL':
            current_idx = self.symbol_bar_index.get(order.symbol, 0)
            position = self.portfolio.get_position(order.symbol)
            available_qty = position.get_available_quantity(current_idx)
            if order.quantity > available_qty:
                order.status = 'REJECTED'
                print(f"Order Rejected: Insufficient T+2 quantity for {order.symbol}. Available: {available_qty}")
                return
                
        # 2. Kiểm tra biên độ trần sàn đối với Limit/Stop orders
        if order.order_type in ['LMT', 'STP'] and order.price is not None:
            prev_data = self.previous_market_data.get(order.symbol)
            if prev_data:
                if not MarketConstraints.is_price_within_limits(order.price, self.exchange, prev_data['close']):
                    order.status = 'REJECTED'
                    print(f"Order Rejected: Limit price {order.price} is outside limits for {order.symbol}")
                    return

        self.active_orders.append(order)
        
        # Nếu có dữ liệu thị trường, cố gắng khớp luôn
        if order.symbol in self.latest_market_data:
            self._try_match_order(order, self.latest_market_data[order.symbol])

    def on_market(self, event: MarketEvent):
        """Update market state and try to fill active orders"""
        if event.symbol in self.latest_market_data:
            self.previous_market_data[event.symbol] = self.latest_market_data[event.symbol]
            
        self.latest_market_data[event.symbol] = event.data
        self.symbol_bar_index[event.symbol] = self.symbol_bar_index.get(event.symbol, -1) + 1
        
        self.portfolio.update_market_price(event.symbol, event.data['close'])
        
        orders_to_process = [o for o in self.active_orders if o.symbol == event.symbol and o.status == 'SUBMITTED']
        for order in orders_to_process:
            self._try_match_order(order, event.data)

    def _try_match_order(self, order: EngineOrder, bar: Dict):
        """Match order against a price bar"""
        fill_price = None
        
        if order.order_type == 'MKT':
            # Fills at the current open
            fill_price = bar['open']
            
        elif order.order_type == 'LMT':
            if order.direction == 'BUY':
                if bar['low'] <= order.price:
                    fill_price = bar['open'] if bar['open'] < order.price else order.price
            else:
                if bar['high'] >= order.price:
                    fill_price = bar['open'] if bar['open'] > order.price else order.price
                    
        elif order.order_type == 'STP':
            if order.direction == 'BUY':
                if bar['high'] >= order.stop_price:
                    fill_price = bar['open'] if bar['open'] > order.stop_price else order.stop_price
            else:
                if bar['low'] <= order.stop_price:
                    fill_price = bar['open'] if bar['open'] < order.stop_price else order.stop_price

        if fill_price is not None:
            self._execute_fill(order, fill_price, bar['timestamp'])

    def _execute_fill(self, order: EngineOrder, raw_price: float, timestamp: datetime):
        # Apply slippage
        executed_price = self.slippage_model.apply(raw_price, order.direction)
        
        # Calculate commission
        commission_model = self.buy_commission if order.direction == 'BUY' else self.sell_commission
        commission = commission_model.calculate(executed_price, order.quantity)
        
        # Verify purchasing power
        cost = (executed_price * order.quantity) + commission
        if order.direction == 'BUY' and cost > self.portfolio.cash:
            order.status = 'REJECTED'
            print(f"Order Rejected: Insufficient funds for {order.symbol}")
            return
            
        # Update order status
        order.status = 'FILLED'
        order.filled_at = timestamp
        order.fill_price = executed_price
        order.commission = commission
        order.slippage = abs(executed_price - raw_price)
        
        # Remove from active orders
        self.active_orders.remove(order)
        
        # Create and dispatch fill event
        fill_event = FillEvent(
            symbol=order.symbol,
            exchange="SIMULATION",
            quantity=order.quantity,
            direction=order.direction,
            fill_price=executed_price,
            commission=commission,
            slippage=order.slippage,
            bar_index=self.symbol_bar_index.get(order.symbol, 0)
        )
        
        # Update portfolio
        self.portfolio.update_from_fill(fill_event)
        
        self.dispatcher.dispatch(fill_event)
