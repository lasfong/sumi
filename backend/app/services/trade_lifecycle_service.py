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
from app.domain.accounting import (
    calculate_buy_amounts,
    calculate_net_pnl,
    calculate_pnl_percent,
    calculate_sell_amounts,
)
from app.services.event_logging_service import EventLoggingService

class TradeLifecycleService:
    @staticmethod
    def process_decision(db: Session, session_id: int, decision_in: DecisionCreate) -> Decision:
        session = ReplayService.get_session(db, session_id)
        from app.services.practice_workflow_service import PracticeWorkflowService
        PracticeWorkflowService.assert_writable_tip(db, session_id)
        candles = ReplayService.get_candles(db, session_id)
        if not candles:
            raise HTTPException(status_code=400, detail="No candles available in session")

        current_candle = candles[-1] # The latest visible candle

        # 1. Create Decision
        exec_price = decision_in.price if decision_in.price is not None else current_candle.close
        planned_qty = decision_in.planned_quantity if decision_in.planned_quantity is not None else decision_in.quantity
        planned_risk = None
        planned_r = None
        if decision_in.stop_loss and decision_in.stop_loss > 0 and exec_price > decision_in.stop_loss:
            risk_unit = exec_price - decision_in.stop_loss
            if planned_qty:
                planned_risk = risk_unit * planned_qty
            if decision_in.target_price and decision_in.target_price > exec_price:
                planned_r = (decision_in.target_price - exec_price) / risk_unit

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
            mistake_tag=decision_in.mistake_tag,
            stop_loss=decision_in.stop_loss,
            target_price=decision_in.target_price,
            planned_quantity=decision_in.planned_quantity,
            planned_risk=planned_risk,
            planned_r=planned_r,
            market_regime=decision_in.market_regime,
            emotion=decision_in.emotion,
            rule_violation=decision_in.rule_violation,
            checklist_snapshot=decision_in.checklist_snapshot,
        )
        db.add(decision)
        db.flush() # get decision.id

        # HOLD/SKIP — just record decision, no order
        if decision_in.action in [DecisionAction.HOLD, DecisionAction.SKIP]:
            db.commit()
            db.refresh(decision)
            return decision

        # Execute logic if it's a trading action
        qty = decision_in.quantity if decision_in.quantity is not None else 100.0

        if exec_price <= 0:
            db.rollback()
            raise HTTPException(status_code=400, detail="Execution price must be greater than zero")
        if qty <= 0:
            db.rollback()
            raise HTTPException(status_code=400, detail="Execution quantity must be greater than zero")

        try:
            if decision_in.order_type == OrderType.LIMIT.value:
                TradeLifecycleService._create_limit_order(db, session, decision.id, decision_in, session.symbol, exec_price, qty, candles)
            else:
                if decision_in.action in [DecisionAction.BUY, DecisionAction.ADD]:
                    TradeLifecycleService._execute_buy(db, session, decision.id, decision_in, session.symbol, current_candle.timestamp, exec_price, qty)

                elif decision_in.action in [DecisionAction.SELL, DecisionAction.REDUCE]:
                    TradeLifecycleService._execute_sell(db, session, decision.id, decision_in, session.symbol, current_candle.timestamp, exec_price, qty)

                elif decision_in.action in [DecisionAction.CLOSE, DecisionAction.CUT_LOSS, DecisionAction.TAKE_PROFIT]:
                    position = db.query(Position).filter(Position.session_id == session_id, Position.status == PositionStatus.OPEN.value).first()
                    if not position or position.quantity <= 0:
                        raise HTTPException(status_code=400, detail="Cannot close: no open position")
                    qty = position.quantity
                    TradeLifecycleService._execute_sell(db, session, decision.id, decision_in, session.symbol, current_candle.timestamp, exec_price, qty)
        except HTTPException:
            db.rollback()
            raise

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
    def _execute_buy(db: Session, session, decision_id: int, decision_in: DecisionCreate, symbol: str, date: datetime, price: float, qty: float, pending_order=None):
        amounts = calculate_buy_amounts(price, qty)

        if amounts.net_amount > session.current_cash:
            if pending_order:
                pending_order.status = OrderStatus.REJECTED.value
                db.add(pending_order)
            raise HTTPException(status_code=400, detail=f"Cannot buy: insufficient cash. Required: {amounts.net_amount}, Available: {session.current_cash}")

        if pending_order:
            order = pending_order
            order.status = OrderStatus.EXECUTED.value
            db.add(order)
        else:
            order = Order(session_id=session.id, decision_id=decision_id, symbol=symbol, side=OrderSide.BUY.value, order_type=OrderType.MARKET_AT_CLOSE.value, requested_price=price, quantity=qty, status=OrderStatus.EXECUTED.value)
            db.add(order)
            db.flush()

        position = db.query(Position).filter(Position.session_id == session.id, Position.status == PositionStatus.OPEN.value).first()
        if not position:
            position = Position(session_id=session.id, symbol=symbol, quantity=qty, average_price=price, total_cost=amounts.gross_amount, opened_at=date)
            db.add(position)

            # Calculate initial risk if stop_loss provided
            initial_risk = None
            planned_r = None
            if decision_in.stop_loss and decision_in.stop_loss > 0 and price > decision_in.stop_loss:
                initial_risk = (price - decision_in.stop_loss) * qty
                if decision_in.target_price and decision_in.target_price > price:
                    planned_r = (decision_in.target_price - price) / (price - decision_in.stop_loss)

            trade = Trade(
                session_id=session.id, symbol=symbol,
                entry_date=date, entry_price=price, quantity=qty,
                initial_stop_loss=decision_in.stop_loss,
                target_price=decision_in.target_price,
                initial_risk=initial_risk,
                planned_entry_price=decision_in.price if decision_in.price is not None else price,
                planned_quantity=decision_in.planned_quantity if decision_in.planned_quantity is not None else qty,
                planned_r=planned_r,
                status='open', result='open',
                setup_type=decision_in.setup_type,
                market_regime=decision_in.market_regime,
                emotion=decision_in.emotion,
                mistake_tag=decision_in.mistake_tag,
                rule_violation=decision_in.rule_violation,
                notes=decision_in.note or decision_in.reason,
            )
            db.add(trade)
            db.flush()
        else:
            new_cost = position.total_cost + amounts.gross_amount
            position.quantity += qty
            position.average_price = new_cost / position.quantity
            position.total_cost = new_cost

            trade = db.query(Trade).filter(Trade.session_id == session.id, Trade.exit_date == None).first()
            if trade:
                trade.quantity += qty
                trade.entry_price = position.average_price

        execution = Execution(order_id=order.id, trade_id=trade.id if trade else None, session_id=session.id, symbol=symbol, execution_date=date, execution_price=price, quantity=qty, gross_amount=amounts.gross_amount, net_amount=amounts.net_amount, fee=amounts.fee, tax=amounts.tax)
        db.add(execution)

        # Update cash on session
        session.current_cash -= amounts.net_amount

    @staticmethod
    def _execute_sell(db: Session, session, decision_id: int, decision_in: DecisionCreate, symbol: str, date: datetime, price: float, qty: float, pending_order=None):
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

        available_qty = max(0.0, settled_bought - total_sold)

        if available_qty < qty:
            blocked_qty = position.quantity - available_qty
            # Find earliest release date from unsettled buy decisions
            unsettled_decisions = db.query(Decision.candle_index, Decision.decision_date) \
                .join(Order, Decision.id == Order.decision_id) \
                .join(Execution, Order.id == Execution.order_id) \
                .filter(
                    Execution.session_id == session.id,
                    Execution.symbol == symbol,
                    Order.side == OrderSide.BUY.value,
                    Decision.candle_index > session.current_index - 2
                ).order_by(Decision.candle_index.asc()).all()

            release_date_str = "T+2"
            if unsettled_decisions:
                earliest_candle_index = unsettled_decisions[0][0]
                release_bar_index = earliest_candle_index + 2
                from app.services.practice_workflow_service import PracticeWorkflowService
                all_candles = PracticeWorkflowService._candles(db, session)
                if all_candles and release_bar_index < len(all_candles):
                    release_date_str = all_candles[release_bar_index].timestamp.date().isoformat()
                else:
                    release_date_str = f"bar #{release_bar_index + 1}"

            raise HTTPException(
                status_code=400,
                detail=f"Cannot sell: T+2 constraint. Available: {available_qty:g}, Blocked: {blocked_qty:g}, Earliest release date: {release_date_str}"
            )


        amounts = calculate_sell_amounts(price, qty)

        trade = db.query(Trade).filter(Trade.session_id == session.id, Trade.exit_date == None).first()

        if pending_order:
            order = pending_order
            order.status = OrderStatus.EXECUTED.value
            db.add(order)
        else:
            order = Order(session_id=session.id, decision_id=decision_id, symbol=symbol, side=OrderSide.SELL.value, order_type=OrderType.MARKET_AT_CLOSE.value, requested_price=price, quantity=qty, status=OrderStatus.EXECUTED.value)
            db.add(order)
            db.flush()

        execution = Execution(order_id=order.id, trade_id=trade.id if trade else None, session_id=session.id, symbol=symbol, execution_date=date, execution_price=price, quantity=qty, gross_amount=amounts.gross_amount, net_amount=amounts.net_amount, fee=amounts.fee, tax=amounts.tax)
        db.add(execution)
        db.flush()

        # Update cash on session
        session.current_cash += amounts.net_amount

        # Calculate PnL for this chunk
        realized_pnl_chunk = (price - position.average_price) * qty
        position.realized_pnl += realized_pnl_chunk
        position.quantity -= qty
        position.total_cost = position.average_price * position.quantity

        if position.quantity <= 0:
            position.status = PositionStatus.CLOSED.value
            position.closed_at = date
            if trade:
                trade.exit_date = date
                trade.exit_price = price
                trade.gross_pnl = position.realized_pnl

                # Calculate net PnL accounting for fees/taxes
                buy_executions = db.query(Execution).join(Order).filter(
                    Execution.session_id == session.id,
                    Execution.symbol == symbol,
                    Execution.trade_id == trade.id,
                    Order.side == OrderSide.BUY.value
                ).all()
                sell_executions = db.query(Execution).join(Order).filter(
                    Execution.session_id == session.id,
                    Execution.symbol == symbol,
                    Execution.trade_id == trade.id,
                    Order.side == OrderSide.SELL.value
                ).all()

                buy_cash_out = sum(e.net_amount for e in buy_executions)
                sell_cash_in = sum(e.net_amount for e in sell_executions)

                trade.net_pnl = calculate_net_pnl(buy_cash_out, sell_cash_in)
                trade.pnl_percent = calculate_pnl_percent(trade.net_pnl, buy_cash_out)

                # Determine result based on net_pnl
                if trade.net_pnl > 0:
                    trade.result = "win"
                elif trade.net_pnl < 0:
                    trade.result = "loss"
                else:
                    trade.result = "breakeven"

                # Calculate holding days
                if trade.entry_date and trade.exit_date:
                    try:
                        entry_d = trade.entry_date.date() if hasattr(trade.entry_date, 'hour') else trade.entry_date
                        exit_d = trade.exit_date.date() if hasattr(trade.exit_date, 'hour') else trade.exit_date
                        trade.holding_days = (exit_d - entry_d).days
                    except (TypeError, AttributeError):
                        pass

                # Calculate R-multiple
                if trade.initial_risk and trade.initial_risk > 0:
                    trade.r_multiple = trade.net_pnl / trade.initial_risk

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

        amounts = calculate_sell_amounts(price, qty)

        order = Order(session_id=session.id, decision_id=decision.id, symbol=session.symbol, side=OrderSide.SELL.value, order_type=OrderType.MARKET_AT_CLOSE.value, requested_price=price, quantity=qty, status=OrderStatus.EXECUTED.value)
        db.add(order)
        db.flush()

        trade = db.query(Trade).filter(Trade.session_id == session.id, Trade.exit_date == None).first()

        execution = Execution(order_id=order.id, trade_id=trade.id if trade else None, session_id=session.id, symbol=session.symbol, execution_date=current_candle.timestamp, execution_price=price, quantity=qty, gross_amount=amounts.gross_amount, net_amount=amounts.net_amount, fee=amounts.fee, tax=amounts.tax)
        db.add(execution)

        # Update session
        session.current_cash += amounts.net_amount

        # Close position
        realized_pnl = (price - position.average_price) * qty
        position.realized_pnl += realized_pnl
        position.quantity = 0
        position.total_cost = 0
        position.status = PositionStatus.CLOSED.value
        position.closed_at = current_candle.timestamp

        # Close trade
        if trade:
            trade.exit_date = current_candle.timestamp
            trade.exit_price = price
            trade.gross_pnl = position.realized_pnl
            buy_executions = db.query(Execution).join(Order).filter(
                Execution.session_id == session.id,
                Execution.symbol == session.symbol,
                Execution.trade_id == trade.id,
                Order.side == OrderSide.BUY.value
            ).all()
            buy_cash_out = sum(e.net_amount for e in buy_executions)
            trade.net_pnl = calculate_net_pnl(buy_cash_out, amounts.net_amount)
            trade.pnl_percent = calculate_pnl_percent(trade.net_pnl, buy_cash_out)
            if trade.net_pnl > 0:
                trade.result = 'win'
            elif trade.net_pnl < 0:
                trade.result = 'loss'
            else:
                trade.result = 'breakeven'
            trade.status = 'closed'

        db.commit()

    @staticmethod
    def _create_limit_order(db: Session, session, decision_id: int, decision_in: DecisionCreate, symbol: str, price: float, qty: float, candles: list):
        from app.models.symbol import Symbol
        from app.domain.engine.market_constraints import MarketConstraints

        # 1. Get exchange
        symbol_record = db.query(Symbol).filter(Symbol.symbol == symbol).first()
        exchange = symbol_record.exchange if symbol_record and symbol_record.exchange else "HOSE"

        # 2. Get reference price (previous candle close)
        if session.current_index > 0:
            reference_price = candles[session.current_index - 1].close
        else:
            reference_price = candles[0].close

        # 3. Validate price limits
        if not MarketConstraints.is_price_within_limits(price, exchange, reference_price):
            floor, ceiling = MarketConstraints.get_price_limits(reference_price, exchange)

            side = OrderSide.BUY.value if decision_in.action in [DecisionAction.BUY, DecisionAction.ADD] else OrderSide.SELL.value
            order = Order(session_id=session.id, decision_id=decision_id, symbol=symbol, side=side, order_type=OrderType.LIMIT.value, requested_price=price, quantity=qty, status=OrderStatus.REJECTED.value)
            db.add(order)
            db.commit()

            raise HTTPException(
                status_code=400,
                detail=f"Giá {price} nằm ngoài biên độ {exchange}. Trần: {ceiling:.2f}, Sàn: {floor:.2f}"
            )

        # 4. Create pending order
        side = OrderSide.BUY.value if decision_in.action in [DecisionAction.BUY, DecisionAction.ADD] else OrderSide.SELL.value
        order = Order(session_id=session.id, decision_id=decision_id, symbol=symbol, side=side, order_type=OrderType.LIMIT.value, requested_price=price, quantity=qty, status=OrderStatus.PENDING.value)
        db.add(order)
        # Note: Do not flush yet, let process_decision commit it.

    @staticmethod
    def execute_pending_order(db: Session, session, order: Order, exec_price: float, current_candle) -> None:
        """Executes a pending LIMIT order"""
        dummy_decision = DecisionCreate(
            action=DecisionAction.BUY if order.side == "BUY" else DecisionAction.SELL,
            price=exec_price,
            quantity=order.quantity
        )

        if order.side == "BUY":
            TradeLifecycleService._execute_buy(db, session, order.decision_id, dummy_decision, session.symbol, current_candle.timestamp, exec_price, order.quantity, pending_order=order)
        else:
            TradeLifecycleService._execute_sell(db, session, order.decision_id, dummy_decision, session.symbol, current_candle.timestamp, exec_price, order.quantity, pending_order=order)
