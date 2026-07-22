from collections import defaultdict
from datetime import date, datetime

from fastapi import HTTPException
from sqlalchemy import and_
from sqlalchemy.orm import Session

from app.models.candle import Candle
from app.models.decision import Decision
from app.models.execution import Execution
from app.models.order import Order
from app.models.position import Position
from app.models.trade import Trade
from app.schemas.practice_workflow_schema import (
    PracticeDecision,
    PracticeExecution,
    PracticeOrder,
    PracticePosition,
    PracticeTrade,
    PracticeWorkflowSnapshot,
)
from app.services.replay_service import ReplayService
from app.utils.date_range import end_before, start_at


def _date_key(value: date | datetime) -> date:
    return value.date() if isinstance(value, datetime) else value


class PracticeWorkflowService:
    """Build a deterministic, read-only lifecycle projection at the replay cursor."""

    @staticmethod
    def _candles(db: Session, session) -> list[Candle]:
        return db.query(Candle).filter(and_(
            Candle.symbol == session.symbol,
            Candle.timeframe == session.timeframe,
            Candle.adjustment_type == session.adjustment_type,
            Candle.timestamp >= start_at(session.start_date),
            Candle.timestamp < end_before(session.end_date),
        )).order_by(Candle.timestamp.asc()).all()

    @staticmethod
    def _ledger(db: Session, session_id: int):
        decisions = db.query(Decision).filter(Decision.session_id == session_id).order_by(Decision.candle_index, Decision.id).all()
        orders = db.query(Order).filter(Order.session_id == session_id).order_by(Order.id).all()
        executions = db.query(Execution).filter(Execution.session_id == session_id).order_by(Execution.execution_date, Execution.id).all()
        trades = db.query(Trade).filter(Trade.session_id == session_id).order_by(Trade.id).all()
        return decisions, orders, executions, trades

    @staticmethod
    def latest_activity_index(db: Session, session_id: int) -> int:
        session = ReplayService.get_session(db, session_id)
        candles = PracticeWorkflowService._candles(db, session)
        if not candles:
            return 0
        index_by_date = {_date_key(c.timestamp): index for index, c in enumerate(candles)}
        decisions, _, executions, _ = PracticeWorkflowService._ledger(db, session_id)
        indices = [decision.candle_index for decision in decisions]
        for execution in executions:
            index = index_by_date.get(_date_key(execution.execution_date))
            if index is None:
                raise HTTPException(status_code=409, detail="Execution cannot be mapped to the replay candle timeline")
            indices.append(index)
        return max(indices, default=0)

    @staticmethod
    def assert_writable_tip(db: Session, session_id: int) -> None:
        session = ReplayService.get_session(db, session_id)
        latest = PracticeWorkflowService.latest_activity_index(db, session_id)
        if latest > session.current_index:
            raise HTTPException(
                status_code=409,
                detail=f"Historical replay view is read-only. Advance to activity bar #{latest + 1} before recording a new decision.",
            )

    @staticmethod
    def get_snapshot(db: Session, session_id: int) -> PracticeWorkflowSnapshot:
        session = ReplayService.get_session(db, session_id)
        candles = PracticeWorkflowService._candles(db, session)
        if not candles:
            raise HTTPException(status_code=400, detail="No candles available in session")
        current_index = min(max(session.current_index, 0), len(candles) - 1)
        current_candle = candles[current_index]
        index_by_date = {_date_key(c.timestamp): index for index, c in enumerate(candles)}
        decisions, orders, executions, trades = PracticeWorkflowService._ledger(db, session_id)
        decision_by_id = {decision.id: decision for decision in decisions}
        order_by_id = {order.id: order for order in orders}

        execution_indices: dict[int, int] = {}
        for execution in executions:
            index = index_by_date.get(_date_key(execution.execution_date))
            if index is None:
                raise HTTPException(status_code=409, detail="Execution cannot be mapped to the replay candle timeline")
            execution_indices[execution.id] = index

        latest_activity_index = max(
            [decision.candle_index for decision in decisions] + list(execution_indices.values()),
            default=0,
        )
        visible_decisions = [decision for decision in decisions if decision.candle_index <= current_index]
        visible_decision_ids = {decision.id for decision in visible_decisions}
        visible_executions = [execution for execution in executions if execution_indices[execution.id] <= current_index]
        visible_execution_by_order = {execution.order_id: execution for execution in visible_executions}

        projected_decisions = [PracticeDecision(
            id=decision.id,
            action=decision.action,
            candle_index=decision.candle_index,
            decision_date=decision.decision_date,
            price=decision.price,
            setup_type=decision.setup_type,
            confidence_score=decision.confidence_score,
            market_context=decision.market_context,
            reason=decision.reason,
            note=decision.note,
            mistake_tag=decision.mistake_tag,
        ) for decision in visible_decisions]

        projected_orders: list[PracticeOrder] = []
        for order in orders:
            decision = decision_by_id.get(order.decision_id)
            if not decision or decision.id not in visible_decision_ids:
                continue
            execution = visible_execution_by_order.get(order.id)
            if execution:
                status = "executed"
                explanation = f"Executed on bar #{execution_indices[execution.id] + 1}."
            elif order.status in {"rejected", "cancelled"}:
                status = order.status
                explanation = "Rejected by order validation." if status == "rejected" else "Order cancelled."
            elif order.order_type == "LIMIT":
                status = "pending"
                explanation = "Pending until a future visible candle range reaches the requested price."
            else:
                status = order.status
                explanation = "Order recorded at the current replay bar."
            projected_orders.append(PracticeOrder(
                id=order.id,
                decision_id=order.decision_id,
                side=order.side,
                order_type=order.order_type,
                requested_price=order.requested_price,
                quantity=order.quantity,
                status=status,
                decision_index=decision.candle_index,
                explanation=explanation,
            ))

        projected_executions: list[PracticeExecution] = []
        for execution in visible_executions:
            order = order_by_id[execution.order_id]
            projected_executions.append(PracticeExecution(
                id=execution.id,
                order_id=execution.order_id,
                decision_id=order.decision_id,
                side=order.side,
                execution_index=execution_indices[execution.id],
                execution_date=execution.execution_date,
                execution_price=execution.execution_price,
                quantity=execution.quantity,
                net_amount=execution.net_amount,
            ))

        quantity = 0.0
        total_cost = 0.0
        realized_pnl = 0.0
        current_cash = float(session.initial_cash)
        opened_at = None
        for execution in visible_executions:
            order = order_by_id[execution.order_id]
            if order.side == "BUY":
                if quantity <= 0:
                    total_cost = 0.0
                    opened_at = execution.execution_date
                quantity += execution.quantity
                total_cost += execution.gross_amount
                current_cash -= execution.net_amount
            else:
                average_price = total_cost / quantity if quantity else 0.0
                realized_pnl += (execution.execution_price - average_price) * execution.quantity
                quantity -= execution.quantity
                total_cost = max(0.0, average_price * quantity)
                current_cash += execution.net_amount

        settled_bought = sum(
            execution.quantity for execution in visible_executions
            if order_by_id[execution.order_id].side == "BUY"
            and decision_by_id[order_by_id[execution.order_id].decision_id].candle_index <= current_index - 2
        )
        visible_sold = sum(
            execution.quantity for execution in visible_executions
            if order_by_id[execution.order_id].side == "SELL"
        )
        available_quantity = max(0.0, settled_bought - visible_sold)
        average_price = total_cost / quantity if quantity > 0 else 0.0
        stored_position = db.query(Position).filter(Position.session_id == session_id).order_by(Position.id).first()
        positions = []
        if quantity > 0 and opened_at is not None:
            positions.append(PracticePosition(
                id=stored_position.id if stored_position else 0,
                symbol=session.symbol,
                quantity=quantity,
                average_price=average_price,
                total_cost=total_cost,
                current_price=current_candle.close,
                realized_pnl=realized_pnl,
                unrealized_pnl=(current_candle.close - average_price) * quantity,
                available_quantity=min(quantity, available_quantity),
                opened_at=opened_at,
            ))

        executions_by_trade: dict[int, list[Execution]] = defaultdict(list)
        for execution in visible_executions:
            if execution.trade_id is not None:
                executions_by_trade[execution.trade_id].append(execution)
        projected_trades: list[PracticeTrade] = []
        for trade in trades:
            trade_executions = executions_by_trade.get(trade.id, [])
            buys = [execution for execution in trade_executions if order_by_id[execution.order_id].side == "BUY"]
            sells = [execution for execution in trade_executions if order_by_id[execution.order_id].side == "SELL"]
            if not buys:
                continue
            bought_qty = sum(execution.quantity for execution in buys)
            sold_qty = sum(execution.quantity for execution in sells)
            closed = sold_qty >= bought_qty and bool(sells)
            net_pnl = sum(execution.net_amount for execution in sells) - sum(execution.net_amount for execution in buys) if closed else None
            buy_cash = sum(execution.net_amount for execution in buys)
            projected_trades.append(PracticeTrade(
                id=trade.id,
                symbol=trade.symbol,
                entry_date=buys[0].execution_date,
                entry_price=trade.entry_price,
                quantity=bought_qty,
                exit_date=sells[-1].execution_date if closed else None,
                exit_price=sells[-1].execution_price if closed else None,
                net_pnl=net_pnl,
                pnl_percent=(net_pnl / buy_cash * 100) if closed and buy_cash else None,
                status="closed" if closed else "open",
                result=("win" if net_pnl and net_pnl > 0 else "loss" if net_pnl and net_pnl < 0 else "breakeven") if closed else "open",
            ))

        historical = latest_activity_index > current_index
        return PracticeWorkflowSnapshot(
            session_id=session.id,
            symbol=session.symbol,
            current_index=current_index,
            visible_bar=current_index + 1,
            total_bars=len(candles),
            current_date=current_candle.timestamp,
            current_price=current_candle.close,
            current_volume=current_candle.volume,
            initial_cash=session.initial_cash,
            current_cash=current_cash,
            available_quantity=min(max(quantity, 0.0), available_quantity),
            latest_activity_index=latest_activity_index,
            historical=historical,
            can_trade=not historical,
            trade_block_reason=(
                f"Historical view: advance to bar #{latest_activity_index + 1} to record a new decision."
                if historical else None
            ),
            decisions=projected_decisions,
            orders=projected_orders,
            executions=projected_executions,
            positions=positions,
            trades=projected_trades,
        )
