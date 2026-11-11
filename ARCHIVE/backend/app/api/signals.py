"""FastAPI router endpoints for Signal Registry and Replay Calculations."""

from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session

from app.dependencies import get_db
from app.schemas.signal_schema import (
    SignalCalculationRequest,
    SignalCalculationResponse,
    SignalRegistryResponse,
)
from app.services.signal_service import SignalService

router = APIRouter()


@router.get("/registry", response_model=SignalRegistryResponse)
def get_signal_registry():
    """Retrieve all active registered signal definitions."""
    return SignalService.get_registry()


@router.post("/replay/{session_id}/calculate", response_model=SignalCalculationResponse)
def calculate_replay_signals(
    session_id: int,
    request: SignalCalculationRequest,
    db: Session = Depends(get_db),
):
    """Calculate signals over the authoritative replay session prefix."""
    return SignalService.calculate_replay_signals(db, session_id, request)
