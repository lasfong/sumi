"""FastAPI router exposing read-only managed universe contracts."""

from __future__ import annotations

from datetime import date
from typing import List, Optional

from fastapi import APIRouter, HTTPException, Query

from app.schemas.universe_schema import (
    UniverseCoverageReportSchema,
    UniverseCoverageRequestSchema,
    UniverseDetailSchema,
    UniverseResolutionSchema,
    UniverseSummarySchema,
)
from app.services.universe_service import UniverseService

router = APIRouter(prefix="/api/universe", tags=["universe"])


@router.get("/list", response_model=List[UniverseSummarySchema])
def list_universes() -> List[UniverseSummarySchema]:
    """List all registered universe definitions and metadata."""
    try:
        return UniverseService.list_universes()
    except Exception as exc:
        raise HTTPException(status_code=500, detail=str(exc)) from exc


@router.get("/{universe_id}/definition", response_model=UniverseDetailSchema)
def get_universe_definition(
    universe_id: str,
    version: Optional[str] = Query(None, description="Specific universe version string"),
) -> UniverseDetailSchema:
    """Get full specification detail for a universe definition."""
    try:
        return UniverseService.get_definition(universe_id, version=version)
    except KeyError as exc:
        raise HTTPException(status_code=404, detail=str(exc)) from exc
    except Exception as exc:
        raise HTTPException(status_code=500, detail=str(exc)) from exc


@router.get("/{universe_id}/resolve", response_model=UniverseResolutionSchema)
def resolve_universe(
    universe_id: str,
    as_of: date = Query(..., description="Target evaluation date (YYYY-MM-DD)"),
    version: Optional[str] = Query(None, description="Specific universe version string"),
    mode: Optional[str] = Query(None, description="Resolution mode: POINT_IN_TIME or RETROSPECTIVE_FIXED"),
) -> UniverseResolutionSchema:
    """Resolve active constituents of a universe as of the specified date."""
    try:
        return UniverseService.resolve_universe(universe_id, as_of=as_of, version=version, mode=mode)
    except KeyError as exc:
        raise HTTPException(status_code=404, detail=str(exc)) from exc
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    except Exception as exc:
        raise HTTPException(status_code=500, detail=str(exc)) from exc


@router.post("/{universe_id}/coverage", response_model=UniverseCoverageReportSchema)
def check_universe_coverage(
    universe_id: str,
    payload: UniverseCoverageRequestSchema,
    as_of: date = Query(..., description="Target evaluation date (YYYY-MM-DD)"),
    version: Optional[str] = Query(None, description="Specific universe version string"),
    mode: Optional[str] = Query(None, description="Resolution mode: POINT_IN_TIME or RETROSPECTIVE_FIXED"),
) -> UniverseCoverageReportSchema:
    """Evaluate observed data coverage against target universe constituents on the specified date."""
    try:
        return UniverseService.check_coverage(
            universe_id,
            as_of=as_of,
            available_symbols=payload.symbols,
            version=version,
            mode=mode,
            threshold=payload.threshold,
        )
    except KeyError as exc:
        raise HTTPException(status_code=404, detail=str(exc)) from exc
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    except Exception as exc:
        raise HTTPException(status_code=500, detail=str(exc)) from exc
