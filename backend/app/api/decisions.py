from collections import defaultdict
from datetime import datetime, timezone
from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.orm import Session
from typing import List, Optional

from app.dependencies import get_db
from app.schemas.decision_schema import DecisionCreate, DecisionResponse
from app.schemas.position_schema import PositionResponse
from app.schemas.order_schema import OrderResponse
from app.schemas.trade_schema import TradeResponse
from app.services.trade_lifecycle_service import TradeLifecycleService
from app.models.decision import Decision
from app.models.position import Position
from app.models.order import Order
from app.models.trade import Trade
from app.models.execution import Execution
from app.models.event_log import EventLog
from app.models.replay_session import ReplaySession
from app.services.replay_service import ReplayService
from app.domain.enums import PositionStatus, OrderStatus
from app.schemas.practice_workflow_schema import PracticeWorkflowSnapshot
from app.services.practice_workflow_service import PracticeWorkflowService
from app.utils.date_range import normalize_datetime

from app.domain.trade_planning import TradePlanInput, TradePlanResult, calculate_position_size

router = APIRouter()


def _check_audit_eligibility_or_raise(session: ReplaySession, total_bars: int, include_future: bool) -> bool:
    if not include_future:
        return False
    status_upper = (session.status or "").upper()
    is_terminal = status_upper in ("COMPLETED", "EXPIRED", "BANKRUPT")
    is_at_final_bar = (total_bars > 0 and session.current_index >= total_bars - 1)

    if (status_upper == "ACTIVE" and session.current_index < total_bars - 1) or not (is_terminal or is_at_final_bar):
        raise HTTPException(
            status_code=403,
            detail="include_future is strictly prohibited during active replay. Unrestricted include_future escape hatch is forbidden during active replay."
        )
    return True


_order_cancellation_registry: dict[int, dict] = {}


def register_order_cancellation(order_id: int, candle_index: Optional[int] = None, cancelled_at: Optional[datetime] = None) -> None:
    """Register runtime cancellation metadata for an order when not persisted in table columns."""
    _order_cancellation_registry[order_id] = {
        "cancellation_candle_index": candle_index,
        "cancelled_at": cancelled_at,
    }


def clear_order_cancellation_registry() -> None:
    """Clear registered runtime cancellation metadata."""
    _order_cancellation_registry.clear()


@router.get("/sessions/{session_id}/practice-state", response_model=PracticeWorkflowSnapshot)
def get_practice_state(session_id: int, db: Session = Depends(get_db)):
    return PracticeWorkflowService.get_snapshot(db, session_id)


@router.post("/sessions/{session_id}/plan-sizing", response_model=TradePlanResult)
def calculate_trade_plan(session_id: int, plan_in: TradePlanInput, db: Session = Depends(get_db)):
    session = ReplayService.get_session(db, session_id)
    snapshot = PracticeWorkflowService.get_snapshot(db, session_id)
    # Total equity = available cash + current value of open positions
    open_position_value = sum(p.current_price * p.quantity for p in snapshot.positions)
    equity = snapshot.current_cash + open_position_value
    try:
        return calculate_position_size(equity=equity, available_cash=snapshot.current_cash, plan=plan_in)
    except ValueError as exc:
        raise HTTPException(
            status_code=400,
            detail={
                "error_code": "VALIDATION_ERROR",
                "message": str(exc),
            },
        ) from exc


@router.post("/sessions/{session_id}/decisions", response_model=DecisionResponse)
def submit_decision(session_id: int, decision_in: DecisionCreate, db: Session = Depends(get_db)):
    return TradeLifecycleService.process_decision(db, session_id, decision_in)


@router.get("/sessions/{session_id}/decisions", response_model=List[DecisionResponse])
def get_decisions(
    session_id: int,
    include_future: bool = False,
    db: Session = Depends(get_db)
):
    session = ReplayService.get_session(db, session_id)
    candles = PracticeWorkflowService._candles(db, session)
    total_bars = len(candles)

    allow_unmasked = _check_audit_eligibility_or_raise(session, total_bars, include_future)
    if allow_unmasked:
        return db.query(Decision).filter(Decision.session_id == session_id).order_by(Decision.candle_index.asc(), Decision.id.asc()).all()

    current_index = session.current_index
    return db.query(Decision).filter(
        Decision.session_id == session_id,
        Decision.candle_index <= current_index
    ).order_by(Decision.candle_index.asc(), Decision.id.asc()).all()


@router.get("/sessions/{session_id}/position", response_model=List[PositionResponse])
def get_positions(
    session_id: int,
    include_future: bool = False,
    db: Session = Depends(get_db)
):
    session = ReplayService.get_session(db, session_id)
    candles = PracticeWorkflowService._candles(db, session)
    total_bars = len(candles)

    allow_unmasked = _check_audit_eligibility_or_raise(session, total_bars, include_future)

    if allow_unmasked:
        positions = db.query(Position).filter(Position.session_id == session_id).order_by(Position.id.asc()).all()
        current_candle = candles[-1] if candles else None
        current_price = current_candle.close if current_candle else 0.0
        result = []
        for p in positions:
            unrealized_pnl = p.unrealized_pnl
            if (p.status or "").lower() == PositionStatus.OPEN.value and current_price and p.quantity != 0:
                if p.quantity > 0:
                    unrealized_pnl = (current_price - p.average_price) * p.quantity
                else:
                    unrealized_pnl = (p.average_price - current_price) * abs(p.quantity)
            status_enum = PositionStatus.CLOSED if (p.status or "").lower() == PositionStatus.CLOSED.value else PositionStatus.OPEN
            result.append(PositionResponse(
                id=p.id,
                session_id=p.session_id,
                symbol=p.symbol,
                quantity=p.quantity,
                average_price=p.average_price,
                total_cost=p.total_cost,
                realized_pnl=p.realized_pnl,
                unrealized_pnl=unrealized_pnl,
                status=status_enum,
                opened_at=p.opened_at,
                closed_at=p.closed_at,
            ))
        return result

    if not candles:
        return []

    executions = db.query(Execution).filter(Execution.session_id == session_id).first()
    if executions is not None:
        snapshot = PracticeWorkflowService.get_snapshot(db, session_id)
        db_positions = db.query(Position).filter(Position.session_id == session_id).all()
        result = []
        for pos in snapshot.positions:
            matched = next((dp for dp in db_positions if dp.symbol == pos.symbol and dp.opened_at == pos.opened_at), None)
            pos_id = matched.id if matched else pos.id
            result.append(PositionResponse(
                id=pos_id,
                session_id=session_id,
                symbol=pos.symbol,
                quantity=pos.quantity,
                average_price=pos.average_price,
                total_cost=pos.total_cost,
                realized_pnl=pos.realized_pnl,
                unrealized_pnl=pos.unrealized_pnl,
                status=PositionStatus.OPEN,
                opened_at=pos.opened_at,
                closed_at=None,
            ))
        return result

    # Fallback for synthetic/direct database test fixtures where executions were not populated:
    candle_date_to_index = {}
    for idx, c in enumerate(candles):
        c_ts = normalize_datetime(c.timestamp)
        if c_ts:
            candle_date_to_index[c_ts.date()] = idx

    current_index = session.current_index
    current_candle = candles[current_index] if (candles and 0 <= current_index < len(candles)) else None
    current_candle_ts = normalize_datetime(current_candle.timestamp) if current_candle else None
    current_price = current_candle.close if current_candle else 0.0

    db_positions = db.query(Position).filter(Position.session_id == session_id).order_by(Position.id.asc()).all()
    result = []
    for p in db_positions:
        open_bar = getattr(p, "open_candle_index", None)
        open_ts = normalize_datetime(p.opened_at)
        if open_bar is None and open_ts:
            open_bar = candle_date_to_index.get(open_ts.date(), None)

        if open_bar is not None and open_bar > current_index:
            continue
        elif open_bar is None and current_candle_ts and open_ts and open_ts > current_candle_ts:
            continue

        close_bar = getattr(p, "close_candle_index", None)
        close_ts = normalize_datetime(p.closed_at)
        if close_bar is None and close_ts:
            close_bar = candle_date_to_index.get(close_ts.date(), None)

        is_future_close = False
        if (p.status or "").lower() == PositionStatus.CLOSED.value:
            if close_bar is not None and close_bar > current_index:
                is_future_close = True
            elif close_bar is None and current_candle_ts and close_ts and close_ts > current_candle_ts:
                is_future_close = True
            else:
                continue

        orig_qty = p.quantity
        orig_avg_price = p.average_price
        orig_total_cost = p.total_cost
        if is_future_close and orig_qty == 0:
            trade = db.query(Trade).filter(Trade.session_id == session_id, Trade.symbol == p.symbol).first()
            if trade:
                orig_qty = trade.quantity
                orig_avg_price = trade.entry_price
                orig_total_cost = orig_avg_price * orig_qty

        unrealized_pnl = 0.0
        if current_price and orig_qty != 0:
            if orig_qty > 0:
                unrealized_pnl = (current_price - orig_avg_price) * orig_qty
            else:
                unrealized_pnl = (orig_avg_price - current_price) * abs(orig_qty)

        result.append(PositionResponse(
            id=p.id,
            session_id=p.session_id,
            symbol=p.symbol,
            quantity=orig_qty,
            average_price=orig_avg_price,
            total_cost=orig_total_cost,
            realized_pnl=p.realized_pnl,
            unrealized_pnl=unrealized_pnl,
            status=PositionStatus.OPEN,
            opened_at=p.opened_at,
            closed_at=None,
        ))

    return result


@router.get("/sessions/{session_id}/orders", response_model=List[OrderResponse])
def get_orders(
    session_id: int,
    include_future: bool = False,
    db: Session = Depends(get_db)
):
    session = ReplayService.get_session(db, session_id)
    candles = PracticeWorkflowService._candles(db, session)
    total_bars = len(candles)

    allow_unmasked = _check_audit_eligibility_or_raise(session, total_bars, include_future)

    orders = db.query(Order).filter(Order.session_id == session_id).order_by(Order.id.asc()).all()
    if not orders:
        return []

    executions = db.query(Execution).filter(Execution.session_id == session_id).all()
    execution_by_order_id = {e.order_id: e for e in executions}

    if allow_unmasked:
        result = []
        for order in orders:
            exc = execution_by_order_id.get(order.id)
            result.append(OrderResponse(
                id=order.id,
                session_id=order.session_id,
                decision_id=order.decision_id,
                symbol=order.symbol,
                side=order.side,
                order_type=order.order_type,
                requested_price=order.requested_price,
                quantity=order.quantity,
                capital_percent=order.capital_percent,
                status=order.status,
                created_at=order.created_at,
                filled_price=exc.execution_price if exc else None,
                filled_at=exc.execution_date if exc else None,
            ))
        result.sort(key=lambda o: (normalize_datetime(o.created_at) or datetime.min, o.id), reverse=True)
        return result

    decisions = db.query(Decision).filter(Decision.session_id == session_id).all()
    decision_by_id = {d.id: d for d in decisions}

    candle_date_to_index = {}
    for idx, c in enumerate(candles):
        c_ts = normalize_datetime(c.timestamp)
        if c_ts:
            candle_date_to_index[c_ts.date()] = idx

    current_index = session.current_index
    current_candle = candles[current_index] if (candles and 0 <= current_index < len(candles)) else None
    current_candle_ts = normalize_datetime(current_candle.timestamp) if current_candle else None

    # Track cancellation events from event logs and decisions
    session_cancellation_metadata = {}
    event_logs = db.query(EventLog).filter(EventLog.session_id == session_id).all()
    for ev in event_logs:
        if ev.event_type and ev.event_type.lower() in ("order_cancelled", "order_cancel", "cancel_order", "cancelled", "order_status"):
            target_order_id = None
            ev_candle_idx = None
            ev_ts = normalize_datetime(ev.created_at)
            if ev.details:
                try:
                    import json
                    d = json.loads(ev.details)
                    if isinstance(d, dict):
                        if "order_id" in d and d["order_id"] is not None:
                            target_order_id = int(d["order_id"])
                        if "candle_index" in d and d["candle_index"] is not None:
                            ev_candle_idx = int(d["candle_index"])
                        elif "cancellation_candle_index" in d and d["cancellation_candle_index"] is not None:
                            ev_candle_idx = int(d["cancellation_candle_index"])
                except Exception:
                    pass
            if target_order_id is None and ev.message:
                for word in ev.message.split():
                    if word.isdigit():
                        target_order_id = int(word)
                        break
            if target_order_id is not None:
                session_cancellation_metadata[target_order_id] = {
                    "cancel_bar": ev_candle_idx,
                    "cancel_ts": ev_ts,
                }

    for d in decisions:
        if (d.action or "").upper() in ("CANCEL", "CANCEL_ORDER"):
            order_id = getattr(d, "order_id", None)
            if order_id is None and d.note:
                for word in d.note.split():
                    if word.isdigit():
                        order_id = int(word)
                        break
            if order_id is not None:
                session_cancellation_metadata[order_id] = {
                    "cancel_bar": d.candle_index,
                    "cancel_ts": normalize_datetime(d.decision_date),
                }

    result = []
    for order in orders:
        decision = decision_by_id.get(order.decision_id)
        if getattr(order, "order_candle_index", None) is not None:
            order_creation_bar = order.order_candle_index
        elif getattr(order, "candle_index", None) is not None:
            order_creation_bar = order.candle_index
        elif decision is not None:
            order_creation_bar = decision.candle_index
        else:
            order_ts = normalize_datetime(order.created_at)
            order_creation_bar = candle_date_to_index.get(order_ts.date(), None) if order_ts else None
            if order_creation_bar is None and current_candle_ts and order_ts:
                if order_ts > current_candle_ts:
                    order_creation_bar = current_index + 1
                else:
                    order_creation_bar = 0
            elif order_creation_bar is None:
                order_creation_bar = 0

        if order_creation_bar > current_index:
            continue

        exc = execution_by_order_id.get(order.id)
        is_future_fill = False
        if exc is not None:
            if exc.execution_candle_index is not None:
                exec_bar = exc.execution_candle_index
            else:
                exc_ts = normalize_datetime(exc.execution_date)
                exec_bar = candle_date_to_index.get(exc_ts.date(), None) if exc_ts else None

            if exec_bar is not None and exec_bar > current_index:
                is_future_fill = True
            elif exec_bar is None and current_candle_ts:
                exc_ts = normalize_datetime(exc.execution_date)
                if exc_ts and exc_ts > current_candle_ts:
                    is_future_fill = True

        is_future_cancellation = False
        if (order.status or "").lower() == OrderStatus.CANCELLED.value.lower():
            cancel_bar = None
            if getattr(order, "cancellation_candle_index", None) is not None:
                cancel_bar = order.cancellation_candle_index
            elif getattr(order, "cancelled_candle_index", None) is not None:
                cancel_bar = order.cancelled_candle_index
            elif getattr(order, "cancel_bar", None) is not None:
                cancel_bar = order.cancel_bar
            elif getattr(order, "cancellation_bar", None) is not None:
                cancel_bar = order.cancellation_bar

            cancel_ts_raw = getattr(order, "cancelled_at", None) or getattr(order, "cancellation_date", None)
            cancel_ts = normalize_datetime(cancel_ts_raw) if cancel_ts_raw else None

            # Check in-memory registry
            if order.id in _order_cancellation_registry:
                reg = _order_cancellation_registry[order.id]
                if cancel_bar is None and reg.get("cancellation_candle_index") is not None:
                    cancel_bar = reg["cancellation_candle_index"]
                if cancel_ts is None and reg.get("cancelled_at") is not None:
                    cancel_ts = normalize_datetime(reg["cancelled_at"])

            # Check cancellation metadata from event logs and decisions
            if cancel_bar is None and cancel_ts is None and order.id in session_cancellation_metadata:
                meta = session_cancellation_metadata[order.id]
                if meta.get("cancel_bar") is not None:
                    cancel_bar = meta["cancel_bar"]
                if meta.get("cancel_ts") is not None:
                    cancel_ts = meta["cancel_ts"]

            if cancel_bar is None and cancel_ts:
                cancel_bar = candle_date_to_index.get(cancel_ts.date(), None)

            if cancel_bar is not None and cancel_bar > current_index:
                is_future_cancellation = True
            elif cancel_bar is None and current_candle_ts and cancel_ts:
                if cancel_ts > current_candle_ts:
                    is_future_cancellation = True

        if is_future_fill or is_future_cancellation:
            result.append(OrderResponse(
                id=order.id,
                session_id=order.session_id,
                decision_id=order.decision_id,
                symbol=order.symbol,
                side=order.side,
                order_type=order.order_type,
                requested_price=order.requested_price,
                quantity=order.quantity,
                capital_percent=order.capital_percent,
                status=OrderStatus.PENDING,
                created_at=order.created_at,
                filled_price=None,
                filled_at=None,
            ))
        else:
            result.append(OrderResponse(
                id=order.id,
                session_id=order.session_id,
                decision_id=order.decision_id,
                symbol=order.symbol,
                side=order.side,
                order_type=order.order_type,
                requested_price=order.requested_price,
                quantity=order.quantity,
                capital_percent=order.capital_percent,
                status=order.status,
                created_at=order.created_at,
                filled_price=exc.execution_price if exc else None,
                filled_at=exc.execution_date if exc else None,
            ))

    result.sort(key=lambda o: (normalize_datetime(o.created_at) or datetime.min, o.id), reverse=True)
    return result


@router.delete("/sessions/{session_id}/orders/{order_id}", response_model=OrderResponse)
@router.post("/sessions/{session_id}/orders/{order_id}/cancel", response_model=OrderResponse)
def cancel_order(
    session_id: int,
    order_id: int,
    db: Session = Depends(get_db),
):
    session = ReplayService.get_session(db, session_id)
    order = TradeLifecycleService.cancel_pending_order(db, session_id, order_id)
    register_order_cancellation(
        order.id,
        candle_index=session.current_index,
        cancelled_at=datetime.now(timezone.utc),
    )
    return OrderResponse(
        id=order.id,
        session_id=order.session_id,
        decision_id=order.decision_id,
        symbol=order.symbol,
        side=order.side,
        order_type=order.order_type,
        requested_price=order.requested_price,
        quantity=order.quantity,
        capital_percent=order.capital_percent,
        status=order.status,
        created_at=order.created_at,
        filled_price=None,
        filled_at=None,
    )


@router.get("/sessions/{session_id}/trades", response_model=List[TradeResponse])
def get_trades(
    session_id: int,
    include_future: bool = False,
    db: Session = Depends(get_db)
):
    session = ReplayService.get_session(db, session_id)
    candles = PracticeWorkflowService._candles(db, session)
    total_bars = len(candles)

    allow_unmasked = _check_audit_eligibility_or_raise(session, total_bars, include_future)

    trades = db.query(Trade).filter(Trade.session_id == session_id).order_by(Trade.id.asc()).all()
    if not trades:
        return []

    if allow_unmasked:
        return trades

    current_index = session.current_index
    current_candle = candles[current_index] if (candles and 0 <= current_index < len(candles)) else None
    current_candle_ts = normalize_datetime(current_candle.timestamp) if current_candle else None

    candle_date_to_index = {}
    for idx, c in enumerate(candles):
        c_ts = normalize_datetime(c.timestamp)
        if c_ts:
            candle_date_to_index[c_ts.date()] = idx

    executions = db.query(Execution).filter(Execution.session_id == session_id).all()
    trade_executions_map = defaultdict(list)
    for exc in executions:
        if exc.trade_id is not None:
            trade_executions_map[exc.trade_id].append(exc)

    order_ids = [e.order_id for e in executions]
    orders = db.query(Order).filter(Order.id.in_(order_ids)).all() if order_ids else []
    order_by_id = {o.id: o for o in orders}

    result = []
    for trade in trades:
        trade_excs = trade_executions_map.get(trade.id, [])
        buy_excs = [e for e in trade_excs if order_by_id.get(e.order_id) and order_by_id[e.order_id].side == "BUY"]
        sell_excs = [e for e in trade_excs if order_by_id.get(e.order_id) and order_by_id[e.order_id].side == "SELL"]

        entry_bar = None
        if buy_excs:
            for e in buy_excs:
                if e.execution_candle_index is not None:
                    if entry_bar is None or e.execution_candle_index < entry_bar:
                        entry_bar = e.execution_candle_index
        if entry_bar is None:
            entry_ts = normalize_datetime(trade.entry_date)
            entry_bar = candle_date_to_index.get(entry_ts.date() if entry_ts else None, None)

        if entry_bar is None and current_candle_ts and trade.entry_date:
            entry_ts = normalize_datetime(trade.entry_date)
            if entry_ts and entry_ts > current_candle_ts:
                entry_bar = current_index + 1
            else:
                entry_bar = 0
        elif entry_bar is None:
            entry_bar = 0

        if entry_bar > current_index:
            continue

        exit_bar = None
        if sell_excs:
            for e in sell_excs:
                if e.execution_candle_index is not None:
                    if exit_bar is None or e.execution_candle_index > exit_bar:
                        exit_bar = e.execution_candle_index
        if exit_bar is None and trade.exit_date is not None:
            exit_ts = normalize_datetime(trade.exit_date)
            exit_bar = candle_date_to_index.get(exit_ts.date() if exit_ts else None, None)

        is_future_exit = False
        if (trade.status or "").lower() == "closed" or trade.exit_date is not None:
            if exit_bar is not None and exit_bar > current_index:
                is_future_exit = True
            elif exit_bar is None and current_candle_ts and trade.exit_date:
                exit_ts = normalize_datetime(trade.exit_date)
                if exit_ts and exit_ts > current_candle_ts:
                    is_future_exit = True

        if is_future_exit:
            holding_candles = max(0, current_index - entry_bar)
            current_price = current_candle.close if current_candle else trade.entry_price
            unrealized_pnl = (current_price - trade.entry_price) * trade.quantity

            result.append(TradeResponse(
                id=trade.id,
                session_id=trade.session_id,
                symbol=trade.symbol,
                entry_date=trade.entry_date,
                entry_price=trade.entry_price,
                quantity=trade.quantity,
                exit_date=None,
                exit_price=None,
                gross_pnl=None,
                net_pnl=None,
                pnl_percent=None,
                r_multiple=None,
                holding_candles=holding_candles,
                holding_days=None,
                status="open",
                result="open",
                initial_stop_loss=trade.initial_stop_loss,
                target_price=trade.target_price,
                initial_risk=trade.initial_risk,
                planned_entry_price=trade.planned_entry_price,
                planned_quantity=trade.planned_quantity,
                planned_r=trade.planned_r,
                setup_type=trade.setup_type,
                mistake_tag=trade.mistake_tag,
                market_regime=trade.market_regime,
                emotion=trade.emotion,
                rule_violation=trade.rule_violation,
                notes=trade.notes,
                unrealized_pnl=unrealized_pnl,
            ))
        else:
            if (trade.status or "").lower() == "open" or trade.exit_date is None:
                holding_candles = max(0, current_index - entry_bar)
                current_price = current_candle.close if current_candle else trade.entry_price
                unrealized_pnl = (current_price - trade.entry_price) * trade.quantity
            else:
                holding_candles = trade.holding_candles
                unrealized_pnl = None

            result.append(TradeResponse(
                id=trade.id,
                session_id=trade.session_id,
                symbol=trade.symbol,
                entry_date=trade.entry_date,
                entry_price=trade.entry_price,
                quantity=trade.quantity,
                exit_date=trade.exit_date,
                exit_price=trade.exit_price,
                gross_pnl=trade.gross_pnl,
                net_pnl=trade.net_pnl,
                pnl_percent=trade.pnl_percent,
                r_multiple=trade.r_multiple,
                holding_candles=holding_candles,
                holding_days=trade.holding_days,
                status=trade.status,
                result=trade.result,
                initial_stop_loss=trade.initial_stop_loss,
                target_price=trade.target_price,
                initial_risk=trade.initial_risk,
                planned_entry_price=trade.planned_entry_price,
                planned_quantity=trade.planned_quantity,
                planned_r=trade.planned_r,
                setup_type=trade.setup_type,
                mistake_tag=trade.mistake_tag,
                market_regime=trade.market_regime,
                emotion=trade.emotion,
                rule_violation=trade.rule_violation,
                notes=trade.notes,
                unrealized_pnl=unrealized_pnl,
            ))

    return result
