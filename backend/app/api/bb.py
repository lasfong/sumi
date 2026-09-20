"""FastAPI router for Money Flow Blackbox (BB) endpoints.

Exposes endpoints for calculating and retrieving symbol-level Technical Flow BB
under the audited OHLCV_PROXY method.
"""

from typing import List, Optional
from fastapi import APIRouter, Depends, HTTPException, Query
from sqlalchemy.orm import Session

from app.dependencies import get_db
from app.domain.bb.contracts import BBHorizon, BBSymbolRequest
from app.domain.bb.chart_adapter import BBChartAdapter
from app.domain.universe.models import UniverseMode
from app.schemas.bb_schema import (
    BBCalculateRequest,
    BBSymbolSeriesResponse,
    MarketBBCalculateRequest,
    MarketBBSeriesResponse,
)
from app.services.bb_service import MoneyFlowBlackboxService

router = APIRouter()


@router.get("/horizons")
def get_bb_horizons():
    """List standard BB evaluation horizons and lookback bars."""
    return {
        "horizons": [
            {"id": h.value, "lookback_bars": h.lookback_bars}
            for h in BBHorizon
        ],
        "default_horizons": ["T03", "T05", "T20", "T50", "T200"],
        "methodology": "OHLCV_PROXY",
    }


@router.get("/symbol/{symbol}", response_model=None)
def get_symbol_bb(
    symbol: str,
    as_of: Optional[str] = Query(None, description="Historical as-of date cutoff (YYYY-MM-DD)"),
    horizons: Optional[str] = Query(None, description="Comma-separated horizons e.g. T03,T05,T20"),
    limit: int = Query(500, ge=1, le=2000, description="Max lookback bars to load"),
    format: str = Query("series", description="Output format: 'series' (canonical DTO) or 'chart' (chart series)"),
    db: Session = Depends(get_db),
):
    """Retrieve Money Flow Blackbox metrics for a given symbol."""
    symbol = symbol.upper().strip()

    # Parse horizons if supplied
    parsed_horizons: Optional[List[BBHorizon]] = None
    if horizons:
        try:
            parsed_horizons = [BBHorizon(h.strip().upper()) for h in horizons.split(",") if h.strip()]
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=f"Invalid horizon specified: {exc}")

    try:
        req = BBSymbolRequest(
            symbol=symbol,
            as_of=as_of,
            horizons=parsed_horizons or [
                BBHorizon.T03, BBHorizon.T05, BBHorizon.T10, BBHorizon.T20, BBHorizon.T50, BBHorizon.T200
            ],
        )
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc))

    service = MoneyFlowBlackboxService()
    try:
        series_res = service.calculate_symbol_bb(req, db=db, limit=limit)
    except Exception as exc:
        raise HTTPException(status_code=500, detail=f"Error calculating BB for {symbol}: {exc}")

    if format == "chart":
        return BBChartAdapter.to_chart_series(series_res)

    return series_res.to_dict()


@router.post("/calculate", response_model=BBSymbolSeriesResponse)
def calculate_symbol_bb(
    payload: BBCalculateRequest,
    db: Session = Depends(get_db),
):
    """Calculate Money Flow Blackbox on-demand for a symbol."""
    symbol = payload.symbol.upper().strip()

    parsed_horizons: Optional[List[BBHorizon]] = None
    if payload.horizons:
        try:
            parsed_horizons = [BBHorizon(h.strip().upper()) for h in payload.horizons if h.strip()]
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=f"Invalid horizon specified: {exc}")

    try:
        req = BBSymbolRequest(
            symbol=symbol,
            as_of=payload.as_of,
            horizons=parsed_horizons or [
                BBHorizon.T03, BBHorizon.T05, BBHorizon.T10, BBHorizon.T20, BBHorizon.T50, BBHorizon.T200
            ],
        )
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc))

    service = MoneyFlowBlackboxService()
    series_res = service.calculate_symbol_bb(req, db=db, limit=payload.limit or 500)
    return series_res.to_dict()


@router.get("/market/{universe_id}", response_model=None)
def get_market_bb(
    universe_id: str,
    as_of: Optional[str] = Query(None, description="Cutoff date (YYYY-MM-DD)"),
    mode: Optional[str] = Query(None, description="Universe mode: 'POINT_IN_TIME' or 'RETROSPECTIVE_FIXED'"),
    horizons: Optional[str] = Query(None, description="Comma-separated horizons e.g. T03,T05,T20"),
    buffer: float = Query(0.0, ge=0.0, le=10.0, description="Neutral zone buffer for breadth"),
    coverage_threshold: float = Query(0.80, ge=0.0, le=1.0, description="Minimum coverage threshold"),
    limit: int = Query(500, ge=1, le=2000, description="Max lookback bars per constituent"),
    db: Session = Depends(get_db),
):
    """Retrieve whole-market Blackbox and flow breadth metrics for a target universe."""
    universe_id = universe_id.upper().strip()

    parsed_mode: Optional[UniverseMode] = None
    if mode:
        try:
            parsed_mode = UniverseMode(mode.upper().strip())
        except ValueError:
            raise HTTPException(status_code=400, detail=f"Invalid universe mode '{mode}'.")

    parsed_horizons: Optional[List[BBHorizon]] = None
    if horizons:
        try:
            parsed_horizons = [BBHorizon(h.strip().upper()) for h in horizons.split(",") if h.strip()]
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=f"Invalid horizon specified: {exc}")

    service = MoneyFlowBlackboxService()
    try:
        res = service.calculate_market_bb(
            universe_id=universe_id,
            universe_mode=parsed_mode,
            horizons=parsed_horizons,
            as_of=as_of,
            breadth_buffer=buffer,
            coverage_threshold=coverage_threshold,
            db=db,
            limit=limit,
        )
    except ValueError as exc:
        raise HTTPException(status_code=404, detail=str(exc))
    except Exception as exc:
        raise HTTPException(status_code=500, detail=f"Error calculating Market BB: {exc}")

    return res.to_dict()


@router.post("/market/calculate", response_model=MarketBBSeriesResponse)
def calculate_market_bb(
    payload: MarketBBCalculateRequest,
    db: Session = Depends(get_db),
):
    """Calculate whole-market Blackbox and flow breadth on-demand."""
    universe_id = payload.universe_id.upper().strip()

    parsed_mode: Optional[UniverseMode] = None
    if payload.universe_mode:
        try:
            parsed_mode = UniverseMode(payload.universe_mode.upper().strip())
        except ValueError:
            raise HTTPException(status_code=400, detail=f"Invalid universe mode '{payload.universe_mode}'.")

    parsed_horizons: Optional[List[BBHorizon]] = None
    if payload.horizons:
        try:
            parsed_horizons = [BBHorizon(h.strip().upper()) for h in payload.horizons if h.strip()]
        except ValueError as exc:
            raise HTTPException(status_code=400, detail=f"Invalid horizon specified: {exc}")

    service = MoneyFlowBlackboxService()
    try:
        res = service.calculate_market_bb(
            universe_id=universe_id,
            universe_mode=parsed_mode,
            horizons=parsed_horizons,
            as_of=payload.as_of,
            breadth_buffer=payload.breadth_buffer or 0.0,
            db=db,
            limit=payload.limit or 500,
        )
    except ValueError as exc:
        raise HTTPException(status_code=404, detail=str(exc))
    except Exception as exc:
        raise HTTPException(status_code=500, detail=f"Error calculating Market BB: {exc}")

    return res.to_dict()

