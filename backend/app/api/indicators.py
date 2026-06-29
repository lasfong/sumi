from fastapi import APIRouter, Depends, HTTPException, Query, Request
from sqlalchemy.orm import Session
from typing import List, Dict, Any
import pandas as pd

from app.dependencies import get_db
from app.models.candle import Candle
from app.domain.engine.indicator_engine import IndicatorEngine

router = APIRouter()

@router.get("/registry")
def list_indicator_registry():
    return {"indicators": IndicatorEngine.list_definitions()}

@router.get("/{symbol}")
def calculate_indicator(
    request: Request,
    symbol: str,
    indicator: str = Query(..., description="Name of the indicator (e.g., rsi, macd, ema)"),
    timeframe: str = Query("1D", description="Timeframe (e.g., 1D, 1H)"),
    limit: int = Query(500, description="Number of historical candles to fetch"),
    db: Session = Depends(get_db)
):
    """
    Calculate an indicator on-the-fly for a given symbol and timeframe.
    Additional kwargs can be passed via query string.
    
    WARNING: This endpoint fetches all historical candles up to 'limit'. 
    DO NOT use this for Replay Sessions as it will leak future data!
    Use the session-scoped endpoint instead: GET /api/replay/sessions/{session_id}/indicators
    """
    # Fetch candles from database
    candles = db.query(Candle).filter(
        Candle.symbol == symbol,
        Candle.timeframe == timeframe
    ).order_by(Candle.timestamp.asc()).limit(limit).all()
    
    if not candles:
        raise HTTPException(status_code=404, detail=f"No candles found for {symbol} on {timeframe}")
        
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
            if key not in ['indicator', 'timeframe', 'limit']:
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
    # pandas-ta usually prepends the indicator name to the column
    original_cols = set(df.columns)
    new_cols = set(result_df.columns) - original_cols
    
    # Prepare response
    response_data = []
    # Reset index to include timestamp in response
    result_df.reset_index(inplace=True)
    
    for _, row in result_df.iterrows():
        # Only return timestamp and the indicator values to save bandwidth
        record = {"timestamp": row["timestamp"].isoformat()}
        for col in new_cols:
            val = row[col]
            record[col] = None if pd.isna(val) else val
        response_data.append(record)
        
    return {
        "symbol": symbol,
        "timeframe": timeframe,
        "indicator": indicator,
        "data": response_data
    }
