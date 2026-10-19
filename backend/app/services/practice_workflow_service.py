from collections import defaultdict
from datetime import date, datetime, timezone

from fastapi import HTTPException
from sqlalchemy import and_
from sqlalchemy.orm import Session

from app.models.candle import Candle
from app.models.decision import Decision
from app.models.event_log import EventLog
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
from app.domain.market.rules import DEFAULT_VN_MARKET_PROFILE
from app.domain.accounting import (
    compute_round_trip_pnl,
    calculate_net_pnl,
    calculate_pnl_percent,
)
from pydantic.fields import FieldInfo
from app.services.replay_service import ReplayService
from app.utils.date_range import end_before, start_at, normalize_datetime

if "settlement_label" not in PracticeWorkflowSnapshot.model_fields:
    PracticeWorkflowSnapshot.model_fields["settlement_label"] = FieldInfo.from_annotation(str)
    PracticeWorkflowSnapshot.model_fields["settlement_label"].default = DEFAULT_VN_MARKET_PROFILE.settlement_label
    PracticeWorkflowSnapshot.model_rebuild(force=True)



def _date_key(value: date | datetime | str) -> date:
    if isinstance(value, str):
        value = datetime.fromisoformat(value)
    if isinstance(value, datetime):
        if value.tzinfo is not None:
            value = value.astimezone(timezone.utc)
        return value.date()
    return value


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
    def extract_terminal_order_metadata(
        db: Session,
        session_id: int,
        decisions: list[Decision] | None = None,
    ) -> tuple[dict[int, dict], dict[int, dict]]:
        """Extract causal cancellation and rejection metadata (candle_index / timestamp) for session orders."""
        import json

        session_cancellation_metadata: dict[int, dict] = {}
        session_rejection_metadata: dict[int, dict] = {}
        event_logs = db.query(EventLog).filter(EventLog.session_id == session_id).all()
        for ev in event_logs:
            ev_type = (ev.event_type or "").lower()
            if ev_type in ("order_cancelled", "order_cancel", "cancel_order", "cancelled", "order_status"):
                target_order_id = None
                ev_candle_idx = None
                ev_ts = normalize_datetime(ev.created_at)
                if ev.details:
                    try:
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
                    for word in ev.message.lstrip("#").split():
                        clean = word.lstrip("#")
                        if clean.isdigit():
                            target_order_id = int(clean)
                            break
                if target_order_id is not None:
                    session_cancellation_metadata[target_order_id] = {
                        "cancel_bar": ev_candle_idx,
                        "cancel_ts": ev_ts,
                    }
            elif ev_type in ("order_rejected", "order_reject", "reject_order", "rejected"):
                target_order_id = None
                ev_candle_idx = None
                ev_ts = normalize_datetime(ev.created_at)
                if ev.details:
                    try:
                        d = json.loads(ev.details)
                        if isinstance(d, dict):
                            if "order_id" in d and d["order_id"] is not None:
                                target_order_id = int(d["order_id"])
                            if "candle_index" in d and d["candle_index"] is not None:
                                ev_candle_idx = int(d["candle_index"])
                            elif "rejection_candle_index" in d and d["rejection_candle_index"] is not None:
                                ev_candle_idx = int(d["rejection_candle_index"])
                    except Exception:
                        pass
                if target_order_id is None and ev.message:
                    for word in ev.message.lstrip("#").split():
                        clean = word.lstrip("#")
                        if clean.isdigit():
                            target_order_id = int(clean)
                            break
                if target_order_id is not None:
                    session_rejection_metadata[target_order_id] = {
                        "reject_bar": ev_candle_idx,
                        "reject_ts": ev_ts,
                    }

        if decisions is None:
            decisions = db.query(Decision).filter(Decision.session_id == session_id).all()
        for d in decisions:
            if (d.action or "").upper() in ("CANCEL", "CANCEL_ORDER"):
                order_id = getattr(d, "order_id", None)
                if order_id is None and d.note:
                    for word in d.note.split():
                        clean = word.lstrip("#")
                        if clean.isdigit():
                            order_id = int(clean)
                            break
                if order_id is not None:
                    session_cancellation_metadata[order_id] = {
                        "cancel_bar": d.candle_index,
                        "cancel_ts": normalize_datetime(d.decision_date),
                    }

        return session_cancellation_metadata, session_rejection_metadata

    @staticmethod
    def resolve_terminal_order_bar(
        order: Order,
        terminal_kind: str,
        candle_date_to_index: dict[date, int],
        event_metadata: dict[int, dict],
        runtime_registry: dict[int, dict] | None = None,
    ) -> tuple[int | None, datetime | None]:
        """Resolve the candle index and timestamp at which an order became cancelled or rejected."""
        event_bar = None
        if terminal_kind == "cancelled":
            for attr in ("cancellation_candle_index", "cancelled_candle_index", "cancel_bar", "cancellation_bar"):
                val = getattr(order, attr, None)
                if val is not None:
                    event_bar = int(val)
                    break
            ts_raw = getattr(order, "cancelled_at", None) or getattr(order, "cancellation_date", None)
            event_ts = normalize_datetime(ts_raw) if ts_raw else None
            if runtime_registry and order.id in runtime_registry:
                reg = runtime_registry[order.id]
                if event_bar is None and reg.get("cancellation_candle_index") is not None:
                    event_bar = int(reg["cancellation_candle_index"])
                if event_ts is None and reg.get("cancelled_at") is not None:
                    event_ts = normalize_datetime(reg["cancelled_at"])
            if event_bar is None and event_ts is None and order.id in event_metadata:
                meta = event_metadata[order.id]
                if meta.get("cancel_bar") is not None:
                    event_bar = int(meta["cancel_bar"])
                if meta.get("cancel_ts") is not None:
                    event_ts = meta["cancel_ts"]
        else:
            for attr in ("rejection_candle_index", "rejected_candle_index", "reject_bar", "rejection_bar"):
                val = getattr(order, attr, None)
                if val is not None:
                    event_bar = int(val)
                    break
            ts_raw = getattr(order, "rejected_at", None) or getattr(order, "rejection_date", None)
            event_ts = normalize_datetime(ts_raw) if ts_raw else None
            if runtime_registry and order.id in runtime_registry:
                reg = runtime_registry[order.id]
                if event_bar is None and reg.get("rejection_candle_index") is not None:
                    event_bar = int(reg["rejection_candle_index"])
                if event_ts is None and reg.get("rejected_at") is not None:
                    event_ts = normalize_datetime(reg["rejected_at"])
            if event_bar is None and event_ts is None and order.id in event_metadata:
                meta = event_metadata[order.id]
                if meta.get("reject_bar") is not None:
                    event_bar = int(meta["reject_bar"])
                if meta.get("reject_ts") is not None:
                    event_ts = meta["reject_ts"]

        if event_bar is None and event_ts is not None:
            event_bar = candle_date_to_index.get(event_ts.date(), None)

        return event_bar, event_ts

    @staticmethod
    def project_order_status_at_cursor(
        order: Order,
        current_index: int,
        current_candle_ts: datetime | None,
        candle_date_to_index: dict[date, int],
        execution_bar: int | None,
        cancellation_metadata: dict[int, dict],
        rejection_metadata: dict[int, dict],
        cancellation_registry: dict[int, dict] | None = None,
        rejection_registry: dict[int, dict] | None = None,
        execution_ts: datetime | None = None,
    ) -> tuple[str, str, bool]:
        """Project causal (status, explanation, is_masked_pending) for an order at cursor K."""
        pending_explanation = (
            "Pending until a future visible candle range reaches the requested price."
            if order.order_type == "LIMIT"
            else "Pending until next candle open."
        )

        if execution_bar is not None:
            if execution_bar <= current_index:
                return "executed", f"Executed on bar #{execution_bar + 1}.", False
            return "pending", pending_explanation, True
        elif execution_ts is not None and current_candle_ts is not None and execution_ts > current_candle_ts:
            return "pending", pending_explanation, True

        raw_status = (order.status or "").lower()
        if raw_status == "cancelled":
            cancel_bar, cancel_ts = PracticeWorkflowService.resolve_terminal_order_bar(
                order,
                "cancelled",
                candle_date_to_index,
                cancellation_metadata,
                cancellation_registry,
            )
            is_future_cancel = (cancel_bar is not None and cancel_bar > current_index) or (
                cancel_bar is None and current_candle_ts is not None and cancel_ts is not None and cancel_ts > current_candle_ts
            )
            if is_future_cancel:
                return "pending", pending_explanation, True
            return "cancelled", "Order cancelled.", False

        if raw_status == "rejected":
            reject_bar, reject_ts = PracticeWorkflowService.resolve_terminal_order_bar(
                order,
                "rejected",
                candle_date_to_index,
                rejection_metadata,
                rejection_registry,
            )
            is_future_reject = (reject_bar is not None and reject_bar > current_index) or (
                reject_bar is None and current_candle_ts is not None and reject_ts is not None and reject_ts > current_candle_ts
            )
            if is_future_reject:
                return "pending", pending_explanation, True
            return "rejected", "Rejected by order validation.", False

        if order.order_type == "LIMIT":
            return "pending", "Pending until a future visible candle range reaches the requested price.", False

        return order.status, "Order recorded at the current replay bar.", False

    @staticmethod
    def latest_activity_index(db: Session, session_id: int) -> int:
        session = ReplayService.get_session(db, session_id)
        candles = PracticeWorkflowService._candles(db, session)
        if not candles:
            return 0
        index_by_date = {_date_key(c.timestamp): index for index, c in enumerate(candles)}
        decisions, orders, executions, _ = PracticeWorkflowService._ledger(db, session_id)
        indices = [decision.candle_index for decision in decisions]
        for execution in executions:
            if execution.execution_candle_index is not None and 0 <= execution.execution_candle_index < len(candles):
                index = execution.execution_candle_index
            else:
                index = index_by_date.get(_date_key(execution.execution_date))
            if index is None:
                raise HTTPException(status_code=409, detail="Execution cannot be mapped to the replay candle timeline")
            indices.append(index)
        cancellation_metadata, rejection_metadata = PracticeWorkflowService.extract_terminal_order_metadata(
            db, session_id, decisions
        )
        for order in orders:
            raw_status = (order.status or "").lower()
            if raw_status in ("cancelled", "rejected"):
                bar, _ = PracticeWorkflowService.resolve_terminal_order_bar(
                    order,
                    raw_status,
                    index_by_date,
                    cancellation_metadata if raw_status == "cancelled" else rejection_metadata,
                )
                if bar is not None and 0 <= bar < len(candles):
                    indices.append(bar)
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
        current_candle_ts = normalize_datetime(current_candle.timestamp)
        index_by_date = {_date_key(c.timestamp): index for index, c in enumerate(candles)}
        decisions, orders, executions, trades = PracticeWorkflowService._ledger(db, session_id)
        decision_by_id = {decision.id: decision for decision in decisions}
        order_by_id = {order.id: order for order in orders}

        execution_indices: dict[int, int] = {}
        execution_by_order: dict[int, Execution] = {}
        for execution in executions:
            if execution.execution_candle_index is not None and 0 <= execution.execution_candle_index < len(candles):
                index = execution.execution_candle_index
            else:
                index = index_by_date.get(_date_key(execution.execution_date))
            if index is None:
                raise HTTPException(status_code=409, detail="Execution cannot be mapped to the replay candle timeline")
            execution_indices[execution.id] = index
            execution_by_order[execution.order_id] = execution

        cancellation_metadata, rejection_metadata = PracticeWorkflowService.extract_terminal_order_metadata(
            db, session_id, decisions
        )
        try:
            from app.api.decisions import _order_cancellation_registry, _order_rejection_registry
        except Exception:
            _order_cancellation_registry = None
            _order_rejection_registry = None

        terminal_order_indices: list[int] = []
        for order in orders:
            raw_status = (order.status or "").lower()
            if raw_status in ("cancelled", "rejected"):
                bar, _ = PracticeWorkflowService.resolve_terminal_order_bar(
                    order,
                    raw_status,
                    index_by_date,
                    cancellation_metadata if raw_status == "cancelled" else rejection_metadata,
                    _order_cancellation_registry if raw_status == "cancelled" else _order_rejection_registry,
                )
                if bar is not None and 0 <= bar < len(candles):
                    terminal_order_indices.append(bar)

        latest_activity_index = max(
            [decision.candle_index for decision in decisions]
            + list(execution_indices.values())
            + terminal_order_indices,
            default=0,
        )
        visible_decisions = [decision for decision in decisions if decision.candle_index <= current_index]
        visible_decision_ids = {decision.id for decision in visible_decisions}
        visible_executions = [execution for execution in executions if execution_indices[execution.id] <= current_index]

        projected_decisions = [PracticeDecision(
            id=decision.id,
            action=decision.action,
            candle_index=decision.candle_index,
            decision_date=decision.decision_date,
            price=decision.price,
            quantity=decision.planned_quantity,
            stop_loss=decision.stop_loss,
            target_price=decision.target_price,
            planned_quantity=decision.planned_quantity,
            planned_risk=decision.planned_risk,
            planned_r=decision.planned_r,
            setup_type=decision.setup_type,
            confidence_score=decision.confidence_score,
            market_context=decision.market_context,
            market_regime=decision.market_regime,
            emotion=decision.emotion,
            reason=decision.reason,
            note=decision.note,
            mistake_tag=decision.mistake_tag,
            rule_violation=decision.rule_violation,
            checklist_snapshot=decision.checklist_snapshot,
        ) for decision in visible_decisions]

        projected_orders: list[PracticeOrder] = []
        for order in orders:
            decision = decision_by_id.get(order.decision_id)
            if not decision or decision.id not in visible_decision_ids:
                continue
            order_exec = execution_by_order.get(order.id)
            exec_bar = execution_indices.get(order_exec.id) if order_exec is not None else None
            exec_ts = normalize_datetime(order_exec.execution_date) if order_exec is not None else None
            status, explanation, _ = PracticeWorkflowService.project_order_status_at_cursor(
                order=order,
                current_index=current_index,
                current_candle_ts=current_candle_ts,
                candle_date_to_index=index_by_date,
                execution_bar=exec_bar,
                cancellation_metadata=cancellation_metadata,
                rejection_metadata=rejection_metadata,
                cancellation_registry=_order_cancellation_registry,
                rejection_registry=_order_rejection_registry,
                execution_ts=exec_ts,
            )
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
                rt_pnl = compute_round_trip_pnl(
                    entry_price=average_price,
                    exit_price=execution.execution_price,
                    quantity=execution.quantity,
                    side="LONG",
                    profile=DEFAULT_VN_MARKET_PROFILE,
                )
                realized_pnl += rt_pnl.net_pnl
                quantity -= execution.quantity
                total_cost = max(0.0, average_price * quantity)
                current_cash += execution.net_amount

        def get_exc_bar_idx(execution):
            if execution.execution_candle_index is not None:
                return execution.execution_candle_index
            return decision_by_id[order_by_id[execution.order_id].decision_id].candle_index

        profile = DEFAULT_VN_MARKET_PROFILE
        engine = profile.settlement_engine
        cur_candle = current_candle if current_candle else (candles[current_index] if current_index < len(candles) else None)
        cur_date = cur_candle.timestamp.date() if cur_candle and hasattr(cur_candle.timestamp, 'date') else (cur_candle.timestamp if cur_candle else None)

        settled_bought = 0.0
        for execution in visible_executions:
            order = order_by_id[execution.order_id]
            if order.side == "BUY":
                exc_bar = get_exc_bar_idx(execution)
                exc_candle = candles[exc_bar] if exc_bar is not None and exc_bar < len(candles) else None
                exc_date = exc_candle.timestamp.date() if exc_candle and hasattr(exc_candle.timestamp, 'date') else (execution.execution_date.date() if hasattr(execution.execution_date, 'date') else execution.execution_date)

                is_settled = False
                if exc_date and cur_date:
                    is_settled = engine.is_sellable(exc_date, cur_date, current_session_phase="close")

                if is_settled:
                    settled_bought += execution.quantity

        visible_sold = sum(
            execution.quantity for execution in visible_executions
            if order_by_id[execution.order_id].side == "SELL"
        )
        available_quantity = max(0.0, settled_bought - visible_sold)
        blocked_quantity = max(0.0, quantity - available_quantity)

        earliest_release_date = None
        if blocked_quantity > 0:
            unsettled_buys = []
            for execution in visible_executions:
                order = order_by_id[execution.order_id]
                if order.side == "BUY":
                    exc_bar = get_exc_bar_idx(execution)
                    exc_candle = candles[exc_bar] if exc_bar is not None and exc_bar < len(candles) else None
                    exc_date = exc_candle.timestamp.date() if exc_candle and hasattr(exc_candle.timestamp, 'date') else (execution.execution_date.date() if hasattr(execution.execution_date, 'date') else execution.execution_date)

                    is_settled = False
                    if exc_date and cur_date:
                        is_settled = engine.is_sellable(exc_date, cur_date, current_session_phase="close")

                    if not is_settled:
                        unsettled_buys.append((exc_bar, exc_date))

            if unsettled_buys:
                earliest_buy_idx, earliest_buy_date = min(unsettled_buys, key=lambda x: x[0] if x[0] is not None else 999999)
                if not earliest_buy_date and earliest_buy_idx is not None and candles and earliest_buy_idx < len(candles):
                    c_buy = candles[earliest_buy_idx]
                    earliest_buy_date = c_buy.timestamp.date() if hasattr(c_buy.timestamp, 'date') else c_buy.timestamp

                if earliest_buy_date:
                    rel_d = engine.calculate_sellable_date(earliest_buy_date)
                    max_candle_date = candles[-1].timestamp.date() if (candles and hasattr(candles[-1].timestamp, 'date')) else (candles[-1].timestamp if candles else None)
                    session_end_date = _date_key(session.end_date) if session.end_date else None
                    if (max_candle_date and rel_d > max_candle_date) or (session_end_date and rel_d > session_end_date):
                        earliest_release_date = None
                    else:
                        earliest_release_date = datetime.combine(rel_d, datetime.min.time())


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
                blocked_quantity=blocked_quantity,
                earliest_release_date=earliest_release_date,
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
            buy_cash = sum(execution.net_amount for execution in buys)
            sell_cash = sum(execution.net_amount for execution in sells)
            net_pnl = calculate_net_pnl(buy_cash, sell_cash) if closed else None
            pnl_percent = calculate_pnl_percent(net_pnl, buy_cash) if (closed and net_pnl is not None and buy_cash > 0) else None
            r_multiple = (net_pnl / trade.initial_risk) if (closed and net_pnl is not None and trade.initial_risk and trade.initial_risk > 0) else None
            entry_drift = (trade.entry_price - trade.planned_entry_price) if (trade.planned_entry_price is not None) else None
            size_variance = (bought_qty - trade.planned_quantity) if (trade.planned_quantity is not None) else None
            r_variance = (r_multiple - trade.planned_r) if (r_multiple is not None and trade.planned_r is not None) else None

            projected_trades.append(PracticeTrade(
                id=trade.id,
                symbol=trade.symbol,
                entry_date=buys[0].execution_date,
                entry_price=trade.entry_price,
                quantity=bought_qty,
                exit_date=sells[-1].execution_date if closed else None,
                exit_price=sells[-1].execution_price if closed else None,
                net_pnl=net_pnl,
                pnl_percent=pnl_percent,
                initial_stop_loss=trade.initial_stop_loss,
                target_price=trade.target_price,
                initial_risk=trade.initial_risk,
                r_multiple=r_multiple,
                planned_entry_price=trade.planned_entry_price,
                planned_quantity=trade.planned_quantity,
                planned_r=trade.planned_r,
                setup_type=trade.setup_type,
                market_regime=trade.market_regime,
                emotion=trade.emotion,
                mistake_tag=trade.mistake_tag,
                rule_violation=trade.rule_violation,
                entry_drift=entry_drift,
                size_variance=size_variance,
                r_variance=r_variance,
                notes=trade.notes,
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
            blocked_quantity=blocked_quantity,
            earliest_release_date=earliest_release_date,
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
            settlement_label=DEFAULT_VN_MARKET_PROFILE.settlement_label,
        )

    @staticmethod
    def reset_practice(db: Session, session_id: int) -> None:
        session = ReplayService.get_session(db, session_id)
        db.query(Execution).filter(Execution.session_id == session_id).delete()
        db.query(Order).filter(Order.session_id == session_id).delete()
        db.query(Decision).filter(Decision.session_id == session_id).delete()
        db.query(Position).filter(Position.session_id == session_id).delete()
        db.query(Trade).filter(Trade.session_id == session_id).delete()
        session.current_cash = session.initial_cash
        db.commit()
