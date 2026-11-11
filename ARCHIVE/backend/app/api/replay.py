from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session
from typing import Any, List, Optional
from app.dependencies import get_db
from app.schemas.replay_schema import ReplaySessionCreate, ReplaySessionResponse
from app.schemas.candle_schema import CandleResponse
from app.services.replay_service import ReplayService
from app.services.practice_workflow_service import PracticeWorkflowService

router = APIRouter()

@router.post("/sessions", response_model=ReplaySessionResponse)
def create_session(session_in: ReplaySessionCreate, db: Session = Depends(get_db)):
    return ReplayService.serialize_session(db, ReplayService.create_session(db, session_in))

@router.get("/sessions", response_model=List[ReplaySessionResponse])
def list_sessions(limit: int = 20, db: Session = Depends(get_db)):
    return [ReplayService.serialize_session(db, session) for session in ReplayService.list_sessions(db, limit)]

@router.get("/sessions/{session_id}", response_model=ReplaySessionResponse)
def get_session(session_id: int, db: Session = Depends(get_db)):
    return ReplayService.serialize_session(db, ReplayService.get_session(db, session_id))

@router.get("/sessions/{session_id}/candles", response_model=List[CandleResponse])
def get_session_candles(session_id: int, target_timeframe: str = None, db: Session = Depends(get_db)):
    return ReplayService.get_candles(db, session_id, target_timeframe)

@router.post("/sessions/{session_id}/next", response_model=ReplaySessionResponse)
def next_candle(session_id: int, steps: int = 1, db: Session = Depends(get_db)):
    return ReplayService.serialize_session(db, ReplayService.next_candle(db, session_id, steps))

@router.post("/sessions/{session_id}/previous", response_model=ReplaySessionResponse)
def previous_candle(session_id: int, steps: int = 1, db: Session = Depends(get_db)):
    return ReplayService.serialize_session(db, ReplayService.previous_candle(db, session_id, steps))

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
import numpy as np
import pandas as pd
from app.models.candle import Candle
from app.domain.engine.indicator_engine import IndicatorEngine


def get_indicator_warmup_bars(indicator_id: str, params: dict[str, Any]) -> int:
    try:
        norm_id = IndicatorEngine.normalize_indicator_id(indicator_id)
    except Exception:
        norm_id = indicator_id.lower().strip()

    warmup_table = {
        "sma": lambda p: int(p.get("length", 20)),
        "ema": lambda p: int(p.get("length", 20)),
        "wma": lambda p: int(p.get("length", 20)),
        "rsi": lambda p: int(p.get("length", 14)),
        "macd": lambda p: int(p.get("slow", 26)) + int(p.get("signal", 9)),
        "bbands": lambda p: int(p.get("length", 20)),
        "stoch": lambda p: int(p.get("k", 14)) + int(p.get("smooth_k", 3)),
        "adx": lambda p: 2 * int(p.get("length", 14)),
        "atr": lambda p: int(p.get("length", 14)),
        "cci": lambda p: int(p.get("length", 20)),
        "ichimoku": lambda p: int(p.get("senkou", 52)),
        "supertrend": lambda p: int(p.get("length", 7)),
        "volume_sma": lambda p: int(p.get("length", 20)),
        "relative_strength": lambda p: int(p.get("length", 20)),
        "kc": lambda p: int(p.get("length", 20)),
        "mfi": lambda p: int(p.get("length", 14)),
        "psar": lambda p: 1,
    }
    calc = warmup_table.get(norm_id)
    if calc:
        try:
            return max(1, calc(params))
        except Exception:
            return 1
    if "length" in params:
        try:
            return max(1, int(params["length"]))
        except Exception:
            pass
    return 1


def compute_indicator_for_replay(
    db: Session,
    session_id: int,
    indicator: str,
    timeframe: str = "1D",
    params: dict[str, Any] | None = None,
    candles: list[Candle] | None = None,
) -> dict[str, Any]:
    raw_params = dict(params or {})
    kwargs: dict[str, Any] = {}
    for key, value in raw_params.items():
        if key not in ['indicator', 'timeframe']:
            if isinstance(value, str):
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
            else:
                kwargs[key] = value

    if candles is None:
        candles = ReplayService.get_candles(db, session_id, timeframe)

    if not candles:
        raise HTTPException(status_code=404, detail=f"No visible candles found for session {session_id}")

    available_bars = len(candles)
    max_visible_ts = candles[-1].timestamp
    max_visible_dt = pd.to_datetime(max_visible_ts)

    try:
        norm_indicator = IndicatorEngine.normalize_indicator_id(indicator)
    except ValueError:
        return {
            "session_id": session_id,
            "timeframe": timeframe,
            "indicator": indicator,
            "data": [],
            "meta": {
                "status": "INVALID_SPEC",
                "warmup_bars": 0,
                "available_bars": available_bars,
                "is_projected_clamped": False,
            }
        }

    warmup_bars = get_indicator_warmup_bars(norm_indicator, kwargs)

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

    if norm_indicator in ("relative_strength", "rs", "rs_vnindex"):
        benchmark_symbol = kwargs.get("benchmark", "VNINDEX")
        if db is not None:
            bench_candles = db.query(Candle).filter(
                Candle.symbol == benchmark_symbol,
                Candle.timeframe == timeframe,
                Candle.timestamp <= max_visible_ts
            ).order_by(Candle.timestamp.asc()).all()
            if bench_candles:
                b_data = [{"timestamp": c.timestamp, "close": float(c.close)} for c in bench_candles]
                b_df = pd.DataFrame(b_data).set_index("timestamp")
                kwargs["benchmark_df"] = b_df

    try:
        result_df = IndicatorEngine.compute(df, norm_indicator, clamp_to_last_candle=True, **kwargs)
    except ValueError:
        return {
            "session_id": session_id,
            "timeframe": timeframe,
            "indicator": indicator,
            "data": [],
            "meta": {
                "status": "INVALID_SPEC",
                "warmup_bars": warmup_bars,
                "available_bars": available_bars,
                "is_projected_clamped": False,
            }
        }
    except Exception as exc:
        raise HTTPException(status_code=400, detail=str(exc))

    original_cols = set(df.columns)
    new_cols = [c for c in result_df.columns if c not in original_cols]

    response_data = []
    result_df.index.name = "timestamp"
    result_df.reset_index(inplace=True)

    is_projected_clamped = False
    if norm_indicator == "ichimoku":
        is_projected_clamped = True

    for _, row in result_df.iterrows():
        row_ts = row["timestamp"]
        try:
            if pd.to_datetime(row_ts) > max_visible_dt:
                is_projected_clamped = True
                continue  # Drop projected future rows completely from Replay
        except Exception:
            pass

        record = {
            "timestamp": row_ts.isoformat() if hasattr(row_ts, "isoformat") else str(row_ts),
        }
        for col in new_cols:
            val = row[col]
            record[col] = None if pd.isna(val) else val
        response_data.append(record)

    # Determine status
    if available_bars < warmup_bars:
        status = "EMPTY_WARMING"
    elif not new_cols:
        status = "UNMAPPED_COLUMN"
    else:
        has_finite_value = False
        for record in response_data:
            for col in new_cols:
                val = record.get(col)
                if val is not None and not (isinstance(val, float) and (np.isnan(val) or np.isinf(val))):
                    has_finite_value = True
                    break
            if has_finite_value:
                break
        status = "OK" if has_finite_value else "EMPTY_WARMING"

    return {
        "session_id": session_id,
        "timeframe": timeframe,
        "indicator": indicator,
        "data": response_data,
        "meta": {
            "status": status,
            "warmup_bars": warmup_bars,
            "available_bars": available_bars,
            "is_projected_clamped": is_projected_clamped,
        }
    }


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
    Crucially, this uses ReplayService.get_candles to ensure no future data is leaked,
    and server-side clamps all projected future points.
    """
    return compute_indicator_for_replay(
        db=db,
        session_id=session_id,
        indicator=indicator,
        timeframe=timeframe,
        params=dict(request.query_params),
    )


@router.post("/sessions/{session_id}/reset-practice")
def reset_practice(session_id: int, db: Session = Depends(get_db)):
    PracticeWorkflowService.reset_practice(db, session_id)
    return {"status": "succeeded", "message": f"Practice trades reset for session {session_id}"}
