from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session
from typing import List
from app.dependencies import get_db
from app.schemas.replay_schema import ReplaySessionCreate, ReplaySessionResponse
from app.schemas.candle_schema import CandleResponse
from app.services.replay_service import ReplayService

router = APIRouter()

@router.post("/sessions", response_model=ReplaySessionResponse)
def create_session(session_in: ReplaySessionCreate, db: Session = Depends(get_db)):
    return ReplayService.create_session(db, session_in)

@router.get("/sessions/{session_id}", response_model=ReplaySessionResponse)
def get_session(session_id: int, db: Session = Depends(get_db)):
    return ReplayService.get_session(db, session_id)

@router.get("/sessions/{session_id}/candles", response_model=List[CandleResponse])
def get_session_candles(session_id: int, target_timeframe: str = None, db: Session = Depends(get_db)):
    return ReplayService.get_candles(db, session_id, target_timeframe)

@router.post("/sessions/{session_id}/next", response_model=ReplaySessionResponse)
def next_candle(session_id: int, steps: int = 1, db: Session = Depends(get_db)):
    return ReplayService.next_candle(db, session_id, steps)

@router.post("/sessions/{session_id}/previous", response_model=ReplaySessionResponse)
def previous_candle(session_id: int, steps: int = 1, db: Session = Depends(get_db)):
    return ReplayService.previous_candle(db, session_id, steps)

from app.schemas.drawing_schema import DrawingStateResponse, DrawingStateUpdate
from app.models.drawing import DrawingState
from fastapi import HTTPException

@router.get("/sessions/{session_id}/drawings", response_model=DrawingStateResponse)
def get_drawings(session_id: int, db: Session = Depends(get_db)):
    state = db.query(DrawingState).filter(DrawingState.session_id == session_id).first()
    if not state:
        # Create empty state
        from app.models.replay_session import ReplaySession
        session = db.query(ReplaySession).filter(ReplaySession.id == session_id).first()
        if not session:
            raise HTTPException(status_code=404, detail="Session not found")
        state = DrawingState(session_id=session_id, symbol=session.symbol, state_data="[]")
        db.add(state)
        db.commit()
        db.refresh(state)
    return state

@router.put("/sessions/{session_id}/drawings", response_model=DrawingStateResponse)
def update_drawings(session_id: int, update_in: DrawingStateUpdate, db: Session = Depends(get_db)):
    state = db.query(DrawingState).filter(DrawingState.session_id == session_id).first()
    if not state:
        from app.models.replay_session import ReplaySession
        session = db.query(ReplaySession).filter(ReplaySession.id == session_id).first()
        if not session:
            raise HTTPException(status_code=404, detail="Session not found")
        state = DrawingState(session_id=session_id, symbol=session.symbol, state_data=update_in.state_data)
        db.add(state)
    else:
        state.state_data = update_in.state_data
    db.commit()
    db.refresh(state)
    return state

from fastapi import Query, Request
import pandas as pd
from app.domain.engine.indicator_engine import IndicatorEngine

@router.get("/sessions/{session_id}/indicators")
def get_session_indicators(
    request: Request,
    session_id: int,
    indicator: str = Query(..., description="Name of the indicator (e.g., rsi, macd, ema)"),
    timeframe: str = Query("1D", description="Timeframe (e.g., 1D, 1H)"),
    db: Session = Depends(get_db)
):
    """
    Calculate an indicator dynamically for a replay session.
    Crucially, this uses ReplayService.get_candles to ensure no future data is leaked.
    """
    # Fetch visible candles ONLY
    candles = ReplayService.get_candles(db, session_id, timeframe)
    
    if not candles:
        raise HTTPException(status_code=404, detail=f"No visible candles found for session {session_id}")
        
    # Convert to DataFrame
    data = []
    for c in candles:
        data.append({
            "timestamp": c.timestamp,
            "open": float(c.open),
            "high": float(c.high),
            "low": float(c.low),
            "close": float(c.close),
            "volume": float(c.volume)
        })
    df = pd.DataFrame(data)
    df.set_index("timestamp", inplace=True)
    
    # Calculate Indicator
    try:
        kwargs = {}
        for key, value in request.query_params.items():
            if key not in ['indicator', 'timeframe']:
                try:
                    kwargs[key] = int(value)
                except ValueError:
                    try:
                        kwargs[key] = float(value)
                    except ValueError:
                        if value.lower() == 'true':
                            kwargs[key] = True
                        elif value.lower() == 'false':
                            kwargs[key] = False
                        else:
                            kwargs[key] = value

        result_df = IndicatorEngine.compute(df, indicator, **kwargs)
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))
        
    # Find the new columns added by pandas-ta
    original_cols = set(df.columns)
    new_cols = set(result_df.columns) - original_cols
    
    if not new_cols:
        raise HTTPException(status_code=400, detail=f"Indicator '{indicator}' did not generate any data.")
        
    # Prepare response
    response_data = []
    result_df.reset_index(inplace=True)
    for _, row in result_df.iterrows():
        record = {"timestamp": row["timestamp"].isoformat()}
        for col in new_cols:
            val = row[col]
            record[col] = None if pd.isna(val) else val
        response_data.append(record)
        
    return {
        "session_id": session_id,
        "timeframe": timeframe,
        "indicator": indicator,
        "data": response_data
    }
