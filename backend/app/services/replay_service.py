from sqlalchemy.orm import Session
from sqlalchemy import select, and_
from fastapi import HTTPException
from app.models.replay_session import ReplaySession
from app.models.candle import Candle
from app.schemas.replay_schema import (
    ReplayIntent,
    ReplaySessionCreate,
    ReplaySessionResponse,
    ReplaySourceContext,
    ReplaySourceSignal,
    ScannerSourcePayload,
)
from app.domain.enums import SessionMode, SessionStatus
from app.services.event_logging_service import EventLoggingService
from app.utils.date_range import end_before, start_at
from typing import List

class ReplayService:
    @staticmethod
    def _session_candle_query(db: Session, session: ReplaySession):
        return db.query(Candle).filter(
            and_(
                Candle.symbol == session.symbol,
                Candle.timeframe == session.timeframe,
                Candle.adjustment_type == session.adjustment_type,
                Candle.timestamp >= start_at(session.start_date),
                Candle.timestamp < end_before(session.end_date)
            )
        ).order_by(Candle.timestamp.asc())

    @staticmethod
    def _scanner_source_context(db: Session, session: ReplaySession) -> ReplaySourceContext:
        fallback = ReplaySourceContext(
            source_type=session.source_type,
            replay_intent=ReplayIntent.BLIND_PRACTICE,
        )
        if not session.source_payload:
            return fallback
        try:
            stored = ScannerSourcePayload.model_validate_json(session.source_payload)
        except Exception:
            return fallback

        candles = ReplayService._session_candle_query(db, session).all()
        signal_timestamp = stored.signal_timestamp.replace(tzinfo=None)
        reveal_at_index = next(
            (
                index for index, candle in enumerate(candles)
                if candle.timestamp.replace(tzinfo=None) == signal_timestamp
            ),
            None,
        )
        if reveal_at_index is None:
            return ReplaySourceContext(
                source_type=session.source_type,
                replay_intent=stored.replay_intent,
            )

        revealed = session.current_index >= reveal_at_index
        return ReplaySourceContext(
            source_type=session.source_type,
            replay_intent=stored.replay_intent,
            reveal_at_index=reveal_at_index,
            revealed=revealed,
            signal=ReplaySourceSignal(
                timestamp=stored.signal_timestamp,
                type=stored.signal_type,
                strategy=stored.strategy,
                price=stored.price,
                regime=stored.regime,
            ) if revealed else None,
        )

    @staticmethod
    def source_context(db: Session, session: ReplaySession) -> ReplaySourceContext:
        if session.source_type == "scanner_signal":
            return ReplayService._scanner_source_context(db, session)
        return ReplaySourceContext(source_type=session.source_type)

    @staticmethod
    def serialize_session(db: Session, session: ReplaySession) -> ReplaySessionResponse:
        values = {
            column.name: getattr(session, column.name)
            for column in ReplaySession.__table__.columns
            if column.name in ReplaySessionResponse.model_fields
        }
        if session.source_type == "scanner_signal":
            values["source_payload"] = None
        values["source_context"] = ReplayService.source_context(db, session)
        return ReplaySessionResponse.model_validate(values)

    @staticmethod
    def create_session(db: Session, session_in: ReplaySessionCreate) -> ReplaySession:
        # Check if candles exist
        candles_exist = db.execute(
            select(Candle.id).where(
                and_(
                    Candle.symbol == session_in.symbol,
                    Candle.timeframe == session_in.timeframe,
                    Candle.adjustment_type == session_in.adjustment_type,
                    Candle.timestamp >= start_at(session_in.start_date),
                    Candle.timestamp < end_before(session_in.end_date)
                )
            ).limit(1)
        ).scalar()

        if not candles_exist:
            raise HTTPException(status_code=400, detail="No candles found for the specified parameters")

        new_session = ReplaySession(
            symbol=session_in.symbol,
            timeframe=session_in.timeframe,
            adjustment_type=session_in.adjustment_type,
            start_date=session_in.start_date,
            end_date=session_in.end_date,
            current_index=0,
            initial_cash=session_in.initial_cash,
            current_cash=session_in.initial_cash,
            status=SessionStatus.ACTIVE.value,
            mode=session_in.mode.value,
            hide_symbol=session_in.hide_symbol,
            hide_date=session_in.hide_date,
            source_type=session_in.source_type,
            source_payload=session_in.source_payload,
        )
        db.add(new_session)
        db.commit()
        db.refresh(new_session)

        EventLoggingService.log_event(
            db=db,
            event_type="SESSION_CREATED",
            message=f"Created session {new_session.id} for {new_session.symbol}",
            session_id=new_session.id,
            details={
                "symbol": new_session.symbol,
                "start_date": str(new_session.start_date),
                "end_date": str(new_session.end_date),
                "initial_cash": new_session.initial_cash
            }
        )

        return new_session

    @staticmethod
    def get_session(db: Session, session_id: int) -> ReplaySession:
        session = db.query(ReplaySession).filter(ReplaySession.id == session_id).first()
        if not session:
            raise HTTPException(status_code=404, detail="Session not found")
        return session

    @staticmethod
    def list_sessions(db: Session, limit: int = 20) -> List[ReplaySession]:
        clean_limit = max(1, min(int(limit), 100))
        replay_modes = [mode.value for mode in SessionMode if mode != SessionMode.BACKTEST]
        return db.query(ReplaySession)\
            .filter(ReplaySession.mode.in_(replay_modes))\
            .order_by(ReplaySession.updated_at.desc(), ReplaySession.id.desc())\
            .limit(clean_limit)\
            .all()

    @staticmethod
    def get_candles(db: Session, session_id: int, target_timeframe: str = None) -> List[Candle]:
        session = ReplayService.get_session(db, session_id)

        # 1. Get the current timestamp of the main session's timeframe
        # The session's primary candles up to current_index
        limit = session.current_index + 1
        primary_candles = ReplayService._session_candle_query(db, session).limit(limit).all()

        if not primary_candles:
            return []

        # If target_timeframe is not specified or same as session, return primary candles
        if not target_timeframe or target_timeframe == session.timeframe:
            return primary_candles

        # 2. Get the current timestamp (T)
        current_timestamp = primary_candles[-1].timestamp

        # 3. Query the target_timeframe candles up to T
        mtf_candles = db.query(Candle).filter(
            and_(
                Candle.symbol == session.symbol,
                Candle.timeframe == target_timeframe,
                Candle.adjustment_type == session.adjustment_type,
                Candle.timestamp >= start_at(session.start_date),
                Candle.timestamp <= current_timestamp
            )
        ).order_by(Candle.timestamp.asc()).all()

        return mtf_candles

    @staticmethod
    def next_candle(db: Session, session_id: int, steps: int = 1) -> ReplaySession:
        session = ReplayService.get_session(db, session_id)
        if steps < 1:
            raise HTTPException(status_code=400, detail="Replay advance steps must be at least 1")

        # Find total possible candles
        total_candles = db.query(Candle).filter(
            and_(
                Candle.symbol == session.symbol,
                Candle.timeframe == session.timeframe,
                Candle.adjustment_type == session.adjustment_type,
                Candle.timestamp >= start_at(session.start_date),
                Candle.timestamp < end_before(session.end_date)
            )
        ).count()
        if total_candles <= 0:
            raise HTTPException(status_code=400, detail="No candles available in session")

        if session.status == SessionStatus.BANKRUPT.value:
            raise HTTPException(status_code=400, detail="Session is bankrupt. Cannot proceed.")

        final_index = total_candles - 1
        destination_index = min(session.current_index + steps, final_index)
        if destination_index == session.current_index:
            session.status = SessionStatus.COMPLETED.value
            db.commit()
            db.refresh(session)
            return session

        for next_index in range(session.current_index + 1, destination_index + 1):
            session.current_index = next_index
            session.status = SessionStatus.COMPLETED.value if next_index == final_index else SessionStatus.ACTIVE.value
            db.commit()
            db.refresh(session)
            ReplayService._process_current_candle_lifecycle(db, session)
            if session.status == SessionStatus.BANKRUPT.value:
                break

        return session

    @staticmethod
    def _process_current_candle_lifecycle(db: Session, session: ReplaySession) -> None:
        from app.models.position import Position
        from app.services.trade_lifecycle_service import TradeLifecycleService

        current_candles = ReplayService.get_candles(db, session.id)
        if not current_candles:
            raise HTTPException(status_code=400, detail="No candles available in session")
        current_candle = current_candles[-1]
        positions = db.query(Position).filter(Position.session_id == session.id, Position.status == "open").all()

        total_stock_value = 0.0
        for position in positions:
            position.unrealized_pnl = (current_candle.close - position.average_price) * position.quantity
            total_stock_value += position.quantity * current_candle.close

        total_equity = session.current_cash + total_stock_value
        if total_equity <= 0:
            session.status = SessionStatus.BANKRUPT.value
            db.commit()
            TradeLifecycleService.force_liquidate_all(db, session, current_candle)
            EventLoggingService.log_event(
                db=db,
                event_type="MARGIN_CALL",
                message=f"Session {session.id} bankrupted at equity {total_equity}",
                session_id=session.id,
                details={"total_equity": total_equity, "cash": session.current_cash},
            )
            return

        db.commit()
        ReplayService._match_pending_orders(db, session)

    @staticmethod
    def _match_pending_orders(db: Session, session: ReplaySession):
        from app.models.order import Order
        from app.domain.enums import OrderStatus
        from app.services.trade_lifecycle_service import TradeLifecycleService

        candles = ReplayService.get_candles(db, session.id)
        if not candles:
            return

        current_candle = candles[-1]

        pending_orders = db.query(Order).filter(
            Order.session_id == session.id,
            Order.status == OrderStatus.PENDING.value
        ).all()

        for order in pending_orders:
            if current_candle.low <= order.requested_price <= current_candle.high:
                exec_price = order.requested_price
                if order.side == "BUY":
                    if current_candle.open <= order.requested_price:
                        exec_price = current_candle.open
                else:
                    if current_candle.open >= order.requested_price:
                        exec_price = current_candle.open

                TradeLifecycleService.execute_pending_order(db, session, order, exec_price, current_candle)
                db.commit()

    @staticmethod
    def previous_candle(db: Session, session_id: int, steps: int = 1) -> ReplaySession:
        session = ReplayService.get_session(db, session_id)

        if session.current_index > 0:
            session.current_index = max(0, session.current_index - steps)
            if session.status == SessionStatus.COMPLETED.value:
                session.status = SessionStatus.ACTIVE.value
            db.commit()
            db.refresh(session)
        else:
            raise HTTPException(status_code=400, detail="Already at the beginning of the session")

        return session
