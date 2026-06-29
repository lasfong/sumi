from typing import Any, Dict, List, Optional

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel
from sqlalchemy.orm import Session

from app.dependencies import get_db
from app.services.scanner_history_service import ScannerHistoryService
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
    signal_type: str | None = None
    strategy: str | None = None
    price: float | None = None
    regime: str | None = None
    timeframe: str = "1D"
    adjustment_type: str = "unadjusted"
    lookback_days: int = 120
    forward_days: int = 90
    initial_cash: float = 100000000


@router.post("/run")
def run_scanner(config: ScannerRequest, db: Session = Depends(get_db)):
    request_config = config.model_dump()
    result = scanner_service.run_scan(db, request_config)
    saved_run = ScannerHistoryService.create_run(db, request_config, result)
    return {**result, "run_id": saved_run["id"]}


@router.get("/runs")
def list_scanner_runs(limit: int = 30, db: Session = Depends(get_db)):
    return ScannerHistoryService.list_runs(db, limit)


@router.get("/runs/{run_id}")
def get_scanner_run(run_id: int, db: Session = Depends(get_db)):
    run = ScannerHistoryService.get_run(db, run_id)
    if not run:
        raise HTTPException(status_code=404, detail="Scanner run not found")
    return run


@router.post("/replay-session")
def create_replay_session_from_signal(config: ScannerReplaySessionRequest, db: Session = Depends(get_db)):
    return scanner_service.create_replay_session_from_signal(db, config.model_dump())
