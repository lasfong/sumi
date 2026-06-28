from sqlalchemy.orm import Session
from sqlalchemy import select, and_
from fastapi import HTTPException
from app.models.replay_session import ReplaySession
from app.models.candle import Candle
from app.schemas.replay_schema import ReplaySessionCreate
from app.domain.enums import SessionStatus
from app.services.event_logging_service import EventLoggingService
from app.domain.enums import SessionStatus
from typing import List

class ReplayService:
    @staticmethod
    def create_session(db: Session, session_in: ReplaySessionCreate) -> ReplaySession:
        # Check if candles exist
        candles_exist = db.execute(
            select(Candle.id).where(
                and_(
                    Candle.symbol == session_in.symbol,
                    Candle.timeframe == session_in.timeframe,
                    Candle.adjustment_type == session_in.adjustment_type,
                    Candle.timestamp >= session_in.start_date,
                    Candle.timestamp <= session_in.end_date
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
            hide_date=session_in.hide_date
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
    def get_candles(db: Session, session_id: int, target_timeframe: str = None) -> List[Candle]:
        session = ReplayService.get_session(db, session_id)

        # 1. Get the current timestamp of the main session's timeframe
        # The session's primary candles up to current_index
        limit = session.current_index + 1
        primary_candles = db.query(Candle).filter(
            and_(
                Candle.symbol == session.symbol,
                Candle.timeframe == session.timeframe,
                Candle.adjustment_type == session.adjustment_type,
                Candle.timestamp >= session.start_date,
                Candle.timestamp <= session.end_date
            )
        ).order_by(Candle.timestamp.asc()).limit(limit).all()

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
                Candle.timestamp >= session.start_date,
                Candle.timestamp <= current_timestamp
            )
        ).order_by(Candle.timestamp.asc()).all()

        return mtf_candles

    @staticmethod
    def next_candle(db: Session, session_id: int, steps: int = 1) -> ReplaySession:
        from app.models.position import Position
        from app.services.trade_lifecycle_service import TradeLifecycleService

        session = ReplayService.get_session(db, session_id)

        # Find total possible candles
        total_candles = db.query(Candle).filter(
            and_(
                Candle.symbol == session.symbol,
                Candle.timeframe == session.timeframe,
                Candle.adjustment_type == session.adjustment_type,
                Candle.timestamp >= session.start_date,
                Candle.timestamp <= session.end_date
            )
        ).count()

        if session.status == SessionStatus.BANKRUPT.value:
            raise HTTPException(status_code=400, detail="Session is bankrupt. Cannot proceed.")

        if session.current_index + steps < total_candles:
            session.current_index += steps
            db.commit()
            db.refresh(session)
        else:
            # Mark session as completed and move to the end
            session.current_index = total_candles - 1
            session.status = SessionStatus.COMPLETED.value
            db.commit()
            db.refresh(session)

        # MTM & Bankrupt logic
        if session.status in [SessionStatus.ACTIVE.value, SessionStatus.COMPLETED.value]:
            current_candle = ReplayService.get_candles(db, session_id)[-1]
            positions = db.query(Position).filter(Position.session_id == session.id, Position.status == "open").all()

            total_stock_value = 0
            for pos in positions:
                # Mark-to-Market
                pos.unrealized_pnl = (current_candle.close - pos.average_price) * pos.quantity
                total_stock_value += pos.quantity * current_candle.close

            total_equity = session.current_cash + total_stock_value

            # Check Bankrupt TC-005
            if total_equity <= 0:
                session.status = SessionStatus.BANKRUPT.value
                db.commit() # Save status first

                # Force Liquidate All
                TradeLifecycleService.force_liquidate_all(db, session, current_candle)

                EventLoggingService.log_event(
                    db=db,
                    event_type="MARGIN_CALL",
                    message=f"Session {session.id} bankrupted at equity {total_equity}",
                    session_id=session.id,
                    details={"total_equity": total_equity, "cash": session.current_cash}
                )
            else:
                db.commit()
                # If not bankrupt, match pending limit orders
                ReplayService._match_pending_orders(db, session)

        return session

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
