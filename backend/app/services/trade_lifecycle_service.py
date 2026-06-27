from sqlalchemy.orm import Session
from sqlalchemy import select
from fastapi import HTTPException
from datetime import datetime

from app.models.decision import Decision
from app.models.order import Order
from app.models.execution import Execution
from app.models.position import Position
from app.models.trade import Trade
from app.services.replay_service import ReplayService
from app.schemas.decision_schema import DecisionCreate
from app.domain.enums import DecisionAction, OrderSide, OrderType, OrderStatus, PositionStatus
from app.services.event_logging_service import EventLoggingService

# Vietnamese stock market fee/tax rates
BUY_FEE_RATE = 0.0015   # 0.15% broker fee on buy
SELL_FEE_RATE = 0.0015   # 0.15% broker fee on sell
SELL_TAX_RATE = 0.001    # 0.1% tax on sell

class TradeLifecycleService:
    @staticmethod
    def process_decision(db: Session, session_id: int, decision_in: DecisionCreate) -> Decision:
        session = ReplayService.get_session(db, session_id)
        candles = ReplayService.get_candles(db, session_id)
        if not candles:
            raise HTTPException(status_code=400, detail="No candles available in session")
            
        current_candle = candles[-1] # The latest visible candle
        
        # 1. Create Decision
        decision = Decision(
            session_id=session_id,
            symbol=session.symbol,
            decision_date=current_candle.timestamp,
            candle_index=session.current_index,
            action=decision_in.action.value,
            price=decision_in.price,
            confidence_score=decision_in.confidence_score,
            setup_type=decision_in.setup_type,
            market_context=decision_in.market_context,
            reason=decision_in.reason,
            note=decision_in.note,
            mistake_tag=decision_in.mistake_tag
        )
        db.add(decision)
        db.flush() # get decision.id
        
        # HOLD/SKIP — just record decision, no order
        if decision_in.action in [DecisionAction.HOLD, DecisionAction.SKIP]:
            db.commit()
            db.refresh(decision)
            return decision
        
        # Execute logic if it's a trading action
        exec_price = decision_in.price if decision_in.price else current_candle.close
        qty = decision_in.quantity if decision_in.quantity else 100.0
        
        if decision_in.action in [DecisionAction.BUY, DecisionAction.ADD]:
            TradeLifecycleService._execute_buy(db, session, decision.id, decision_in, session.symbol, current_candle.timestamp, exec_price, qty)
            
        elif decision_in.action in [DecisionAction.SELL, DecisionAction.REDUCE]:
            TradeLifecycleService._execute_sell(db, session, decision.id, decision_in, session.symbol, current_candle.timestamp, exec_price, qty)
            
        elif decision_in.action in [DecisionAction.CLOSE, DecisionAction.CUT_LOSS, DecisionAction.TAKE_PROFIT]:
            position = db.query(Position).filter(Position.session_id == session_id, Position.status == PositionStatus.OPEN.value).first()
            if position:
                qty = position.quantity
                TradeLifecycleService._execute_sell(db, session, decision.id, decision_in, session.symbol, current_candle.timestamp, exec_price, qty)

        EventLoggingService.log_event(
            db=db,
            event_type="TRADE_EXECUTED" if decision_in.action not in [DecisionAction.HOLD, DecisionAction.SKIP] else "DECISION_LOGGED",
            message=f"Action: {decision_in.action.value} {qty} shares",
            session_id=session_id,
            details={
                "action": decision_in.action.value,
                "quantity": qty,
                "price": exec_price,
                "setup": decision_in.setup_type
            }
        )

        db.commit()
        db.refresh(decision)
        return decision

    @staticmethod
    def _execute_buy(db: Session, session, decision_id: int, decision_in: DecisionCreate, symbol: str, date: datetime, price: float, qty: float):
        # Fee calculation
        gross_amount = price * qty
        fee = price * qty * BUY_FEE_RATE
        net_amount = gross_amount + fee
        
        order = Order(session_id=session.id, decision_id=decision_id, symbol=symbol, side=OrderSide.BUY.value, order_type=OrderType.MARKET_AT_CLOSE.value, requested_price=price, quantity=qty, status=OrderStatus.EXECUTED.value)
        db.add(order)
        db.flush()
        
        execution = Execution(order_id=order.id, session_id=session.id, symbol=symbol, execution_date=date, execution_price=price, quantity=qty, gross_amount=gross_amount, net_amount=net_amount)
        db.add(execution)
        
        # Update cash on session
        session.current_cash -= net_amount
        
        position = db.query(Position).filter(Position.session_id == session.id, Position.status == PositionStatus.OPEN.value).first()
        if not position:
            position = Position(session_id=session.id, symbol=symbol, quantity=qty, average_price=price, total_cost=gross_amount, opened_at=date)
            db.add(position)
            
            # Calculate initial risk if stop_loss provided
            initial_risk = None
            if decision_in.stop_loss:
                initial_risk = (price - decision_in.stop_loss) * qty
            
            trade = Trade(
                session_id=session.id, symbol=symbol,
                entry_date=date, entry_price=price, quantity=qty,
                initial_stop_loss=decision_in.stop_loss,
                target_price=decision_in.target_price,
                initial_risk=initial_risk,
                status='open', result='open',
                setup_type=decision_in.setup_type,
                mistake_tag=decision_in.mistake_tag
            )
            db.add(trade)
        else:
            new_cost = position.total_cost + gross_amount
            position.quantity += qty
            position.average_price = new_cost / position.quantity
            position.total_cost = new_cost
            
            trade = db.query(Trade).filter(Trade.session_id == session.id, Trade.exit_date == None).first()
            if trade:
                trade.quantity += qty
                trade.entry_price = position.average_price

    @staticmethod
    def _execute_sell(db: Session, session, decision_id: int, decision_in: DecisionCreate, symbol: str, date: datetime, price: float, qty: float):
        position = db.query(Position).filter(Position.session_id == session.id, Position.status == PositionStatus.OPEN.value).first()
        if not position or position.quantity < qty:
            raise HTTPException(status_code=400, detail="Cannot sell: insufficient position")
            
        # Calculate available quantity based on T+2 constraint
        from sqlalchemy import func
        from app.models.decision import Decision
        from app.models.order import Order
        from app.models.execution import Execution
        
        # 1. Calculate total quantity bought that has settled (T+2)
        settled_bought = db.query(func.sum(Execution.quantity)) \
            .join(Order, Execution.order_id == Order.id) \
            .join(Decision, Order.decision_id == Decision.id) \
            .filter(
                Execution.session_id == session.id,
                Execution.symbol == symbol,
                Order.side == OrderSide.BUY.value,
                Decision.candle_index <= session.current_index - 2
            ).scalar() or 0.0
            
        # 2. Calculate total quantity already sold (which naturally uses up the settled bought quantity first)
        total_sold = db.query(func.sum(Execution.quantity)) \
            .join(Order, Execution.order_id == Order.id) \
            .filter(
                Execution.session_id == session.id,
                Execution.symbol == symbol,
                Order.side == OrderSide.SELL.value
            ).scalar() or 0.0
            
        available_qty = settled_bought - total_sold
        
        if available_qty < qty:
            raise HTTPException(status_code=400, detail=f"Cannot sell: T+2 constraint. Available: {available_qty}, Requested: {qty}")

        
        # Fee/tax calculation
        gross_amount = price * qty
        fee = price * qty * SELL_FEE_RATE
        tax = price * qty * SELL_TAX_RATE
        net_amount = gross_amount - fee - tax
            
        order = Order(session_id=session.id, decision_id=decision_id, symbol=symbol, side=OrderSide.SELL.value, order_type=OrderType.MARKET_AT_CLOSE.value, requested_price=price, quantity=qty, status=OrderStatus.EXECUTED.value)
        db.add(order)
        db.flush()
        
        execution = Execution(order_id=order.id, session_id=session.id, symbol=symbol, execution_date=date, execution_price=price, quantity=qty, gross_amount=gross_amount, net_amount=net_amount)
        db.add(execution)
        
        # Update cash on session
        session.current_cash += net_amount
        
        # Calculate PnL for this chunk
        realized_pnl_chunk = (price - position.average_price) * qty
        position.realized_pnl += realized_pnl_chunk
        position.quantity -= qty
        position.total_cost = position.average_price * position.quantity
        
        trade = db.query(Trade).filter(Trade.session_id == session.id, Trade.exit_date == None).first()
        
        if position.quantity <= 0:
            position.status = PositionStatus.CLOSED.value
            position.closed_at = date
            if trade:
                trade.exit_date = date
                trade.exit_price = price
                trade.gross_pnl = position.realized_pnl
                
                # Calculate net PnL accounting for fees/taxes
                # Total buy cost fees were already tracked; here we use gross_pnl as approximation
                trade.net_pnl = position.realized_pnl  # simplified: fee impact is in cash tracking
                trade.pnl_percent = (trade.net_pnl / (trade.entry_price * trade.quantity)) if trade.quantity > 0 and trade.entry_price > 0 else 0
                
                # Calculate holding days
                if trade.entry_date and trade.exit_date:
                    # Normalize to date: if datetime has .date() method callable, use it
                    try:
                        entry_d = trade.entry_date.date() if hasattr(trade.entry_date, 'hour') else trade.entry_date
                        exit_d = trade.exit_date.date() if hasattr(trade.exit_date, 'hour') else trade.exit_date
                        trade.holding_days = (exit_d - entry_d).days
                    except (TypeError, AttributeError):
                        pass
                
                # Calculate R-multiple
                if trade.initial_risk and trade.initial_risk > 0:
                    trade.r_multiple = trade.net_pnl / trade.initial_risk
                
                # Determine result
                else:
                    trade.result = 'breakeven'
                trade.status = 'closed'

    @staticmethod
    def force_liquidate_all(db: Session, session, current_candle) -> None:
        """Kích hoạt ép bán toàn bộ cổ phiếu khi bị Margin Call, bỏ qua T+2"""
        position = db.query(Position).filter(Position.session_id == session.id, Position.status == PositionStatus.OPEN.value).first()
        if not position or position.quantity <= 0:
            return
            
        qty = position.quantity
        price = current_candle.close
        
        # Tạo decision hệ thống
        decision = Decision(
            session_id=session.id,
            symbol=session.symbol,
            decision_date=current_candle.timestamp,
            candle_index=session.current_index,
            action="MARGIN_CALL",
            price=price,
            confidence_score=100.0,
            setup_type="SYSTEM",
            market_context="BANKRUPT",
            reason="Force Liquidated due to negative equity",
            note="",
            mistake_tag="MARGIN_CALL"
        )
        db.add(decision)
        db.flush()
        
        # Fee/tax
        gross_amount = price * qty
        fee = price * qty * SELL_FEE_RATE
        tax = price * qty * SELL_TAX_RATE
        net_amount = gross_amount - fee - tax
            
        order = Order(session_id=session.id, decision_id=decision.id, symbol=session.symbol, side=OrderSide.SELL.value, order_type=OrderType.MARKET_AT_CLOSE.value, requested_price=price, quantity=qty, status=OrderStatus.EXECUTED.value)
        db.add(order)
        db.flush()
        
        execution = Execution(order_id=order.id, session_id=session.id, symbol=session.symbol, execution_date=current_candle.timestamp, execution_price=price, quantity=qty, gross_amount=gross_amount, net_amount=net_amount)
        db.add(execution)
        
        # Update session
        session.current_cash += net_amount
        
        # Close position
        realized_pnl = (price - position.average_price) * qty
        position.realized_pnl += realized_pnl
        position.quantity = 0
        position.total_cost = 0
        position.status = PositionStatus.CLOSED.value
        position.closed_at = current_candle.timestamp
        
        # Close trade
        trade = db.query(Trade).filter(Trade.session_id == session.id, Trade.exit_date == None).first()
        if trade:
            trade.exit_date = current_candle.timestamp
            trade.exit_price = price
            trade.gross_pnl = position.realized_pnl
            trade.net_pnl = position.realized_pnl
            trade.pnl_percent = (trade.net_pnl / (trade.entry_price * trade.quantity)) if trade.quantity > 0 and trade.entry_price > 0 else 0
            if trade.net_pnl > 0:
                trade.result = 'win'
            elif trade.net_pnl < 0:
                trade.result = 'loss'
            else:
                trade.result = 'breakeven'
            trade.status = 'closed'
            
        db.commit()
