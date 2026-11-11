"""Transactional database persistence adapter for completed backtest runs.

Writes completed backtest results into SQLite in a single atomic transaction
to ensure backward compatibility with AnalyticsService, ReplaySession queries,
and existing API tests without polluting the database during simulation.
"""

import json
from typing import Optional

from sqlalchemy.orm import Session

from app.domain.backtest.models import BacktestKernelResult
from app.models.decision import Decision
from app.models.execution import Execution
from app.models.order import Order
from app.models.position import Position
from app.models.replay_session import ReplaySession
from app.models.trade import Trade


class BacktestPersistenceAdapter:
    """Persists a successful in-memory BacktestKernelResult into the database."""

    @staticmethod
    def persist_result(
        db: Session,
        result: BacktestKernelResult,
        benchmark_symbol: str = "VNINDEX",
    ) -> ReplaySession:
        """Atomically persist ReplaySession, Decisions, Orders, Executions, and Trades."""
        # 1. Create ReplaySession
        session = ReplaySession(
            symbol=result.symbol,
            timeframe=result.timeframe,
            start_date=result.start_date,
            end_date=result.end_date,
            current_index=result.last_analyzed_index,
            initial_cash=result.ledger.initial_cash,
            current_cash=result.ledger.current_cash,
            mode="backtest",
            status="active",
        )
        session.source_payload = json.dumps({"benchmark_symbol": benchmark_symbol})
        db.add(session)
        db.flush()

        # 2. Persist Trades
        order_to_trade_map = {}
        for trade in result.ledger.trades:
            db_trade = Trade(
                session_id=session.id,
                symbol=result.symbol,
                entry_date=trade.entry_time,
                entry_price=trade.entry_price,
                exit_date=trade.exit_time,
                exit_price=trade.exit_price,
                quantity=trade.quantity,
                gross_pnl=trade.gross_pnl,
                net_pnl=trade.net_pnl,
                pnl_percent=trade.pnl_percent,
                holding_candles=trade.holding_bars,
                status=trade.status,
                result=trade.result,
            )
            db.add(db_trade)
            db.flush()
            order_to_trade_map[trade.entry_order_id] = db_trade.id
            if trade.exit_order_id:
                order_to_trade_map[trade.exit_order_id] = db_trade.id

        # 3. Map Executions by Order ID
        exec_by_order = {ex.order_id: ex for ex in result.ledger.executions}

        # 4. Persist Decisions, Orders, and Executions
        for order in result.ledger.orders:
            db_decision = Decision(
                session_id=session.id,
                symbol=result.symbol,
                decision_date=order.signal_timestamp,
                candle_index=order.signal_bar_index,
                action=order.side,
                price=order.requested_price,
                reason=order.status,
            )
            db.add(db_decision)
            db.flush()

            status_str = "executed" if order.status == "FILLED" else order.status.lower()
            db_order = Order(
                session_id=session.id,
                decision_id=db_decision.id,
                symbol=result.symbol,
                side=order.side,
                order_type=order.order_type,
                requested_price=order.requested_price,
                quantity=order.quantity,
                status=status_str,
            )
            db.add(db_order)
            db.flush()

            if order.id in exec_by_order:
                ex = exec_by_order[order.id]
                trade_id = order_to_trade_map.get(order.id)
                db_exec = Execution(
                    order_id=db_order.id,
                    trade_id=trade_id,
                    session_id=session.id,
                    symbol=result.symbol,
                    execution_date=ex.fill_timestamp,
                    execution_price=ex.price,
                    quantity=ex.quantity,
                    fee=ex.fee,
                    tax=ex.tax,
                    execution_candle_index=ex.fill_bar_index,
                    gross_amount=ex.gross_amount,
                    net_amount=ex.net_amount,
                )
                db.add(db_exec)

        # 5. Atomic Commit
        db.commit()
        db.refresh(session)
        return session
