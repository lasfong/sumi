from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session
from typing import List
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
from app.services.replay_service import ReplayService

router = APIRouter()

@router.post("/sessions/{session_id}/decisions", response_model=DecisionResponse)
def submit_decision(session_id: int, decision_in: DecisionCreate, db: Session = Depends(get_db)):
    return TradeLifecycleService.process_decision(db, session_id, decision_in)

@router.get("/sessions/{session_id}/decisions", response_model=List[DecisionResponse])
def get_decisions(session_id: int, db: Session = Depends(get_db)):
    return db.query(Decision).filter(Decision.session_id == session_id).all()

@router.get("/sessions/{session_id}/position", response_model=List[PositionResponse])
def get_positions(session_id: int, db: Session = Depends(get_db)):
    positions = db.query(Position).filter(Position.session_id == session_id).all()
    
    # Calculate unrealized PnL dynamically based on the current candle
    if positions:
        candles = ReplayService.get_candles(db, session_id)
        if candles:
            current_price = candles[-1].close
            for p in positions:
                if p.quantity > 0:
                    p.unrealized_pnl = (current_price - p.average_price) * p.quantity
                elif p.quantity < 0:
                    p.unrealized_pnl = (p.average_price - current_price) * abs(p.quantity)
    
    return positions

@router.get("/sessions/{session_id}/orders", response_model=List[OrderResponse])
def get_orders(session_id: int, db: Session = Depends(get_db)):
    return db.query(Order).filter(Order.session_id == session_id).order_by(Order.created_at.desc()).all()

@router.get("/sessions/{session_id}/trades", response_model=List[TradeResponse])
def get_trades(session_id: int, db: Session = Depends(get_db)):
    return db.query(Trade).filter(Trade.session_id == session_id).all()
