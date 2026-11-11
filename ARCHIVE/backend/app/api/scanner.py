from typing import Any, Dict, List, Optional

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel
from sqlalchemy.orm import Session

from app.dependencies import get_db
from app.services.scanner_history_service import ScannerHistoryService
from app.services.scanner_service import ScannerService
from app.schemas.replay_schema import ReplayIntent, ReplaySessionResponse


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
    replay_intent: ReplayIntent = ReplayIntent.BLIND_PRACTICE


class ScannerReplaySessionResponse(BaseModel):
    session: ReplaySessionResponse
    window_start: str
    window_end: str


class ScannerRankingSchema(BaseModel):
    status: str = "not_applicable"
    eligible: bool = False
    metric: Optional[str] = None
    reason: Optional[str] = None


class ScannerResultItemSchema(BaseModel):
    symbol: str
    timestamp: str
    signal_type: str
    strategy: str
    price: float
    regime: str
    ranking: Optional[ScannerRankingSchema] = None


class ScannerResponseSchema(BaseModel):
    status: str
    run_id: Optional[int] = None
    total_results: Optional[int] = None
    truncated: Optional[bool] = None
    results: List[ScannerResultItemSchema] = []
    warnings: List[str] = []
    error_code: Optional[str] = None
    message: Optional[str] = None
    ranking_policy: Optional[str] = None


class ScannerHistoryRunSchema(BaseModel):
    id: int
    label: str
    status: str
    total_results: int
    request_config: Dict[str, Any]
    result_payload: ScannerResponseSchema
    created_at: str


@router.post("/run", response_model=ScannerResponseSchema)
def run_scanner(config: ScannerRequest, db: Session = Depends(get_db)):
    request_config = config.model_dump()
    result = scanner_service.run_scan(db, request_config)
    saved_run = ScannerHistoryService.create_run(db, request_config, result)
    return {**result, "run_id": saved_run["id"]}


@router.get("/runs", response_model=List[ScannerHistoryRunSchema])
def list_scanner_runs(limit: int = 30, db: Session = Depends(get_db)):
    return ScannerHistoryService.list_runs(db, limit)


@router.get("/runs/{run_id}", response_model=ScannerHistoryRunSchema)
def get_scanner_run(run_id: int, db: Session = Depends(get_db)):
    run = ScannerHistoryService.get_run(db, run_id)
    if not run:
        raise HTTPException(status_code=404, detail="Scanner run not found")
    return run


@router.post("/replay-session", response_model=ScannerReplaySessionResponse)
def create_replay_session_from_signal(config: ScannerReplaySessionRequest, db: Session = Depends(get_db)):
    return scanner_service.create_replay_session_from_signal(db, config.model_dump())
