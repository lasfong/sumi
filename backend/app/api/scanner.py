from typing import Any, Dict, List, Optional

from fastapi import APIRouter, Depends
from pydantic import BaseModel
from sqlalchemy.orm import Session

from app.dependencies import get_db
from app.services.scanner_service import ScannerService


router = APIRouter()
scanner_service = ScannerService()


class ScannerRequest(BaseModel):
    symbols: List[str]
    start_date: str
    end_date: str
    strategy: Dict[str, Any]
    benchmark_symbol: Optional[str] = "VNINDEX"
    max_results: int = 500


class ScannerReplaySessionRequest(BaseModel):
    symbol: str
    signal_timestamp: str
    timeframe: str = "1D"
    adjustment_type: str = "unadjusted"
    lookback_days: int = 120
    forward_days: int = 90
    initial_cash: float = 100000000


@router.post("/run")
def run_scanner(config: ScannerRequest, db: Session = Depends(get_db)):
    return scanner_service.run_scan(db, config.model_dump())


@router.post("/replay-session")
def create_replay_session_from_signal(config: ScannerReplaySessionRequest, db: Session = Depends(get_db)):
    return scanner_service.create_replay_session_from_signal(db, config.model_dump())
