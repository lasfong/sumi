from typing import Any, Dict, List, Optional

from fastapi import APIRouter, Depends
from pydantic import BaseModel, Field
from sqlalchemy.orm import Session

from app.dependencies import get_db
from app.services.strategy_lab_history_service import StrategyLabHistoryService
from app.services.strategy_lab_service import StrategyLabService


router = APIRouter()
strategy_lab_service = StrategyLabService()


class SweepParameter(BaseModel):
    path: str
    values: List[Any]


class ParameterSweepRequest(BaseModel):
    symbol: Optional[str] = None
    symbols: Optional[List[str]] = None
    start_date: str
    end_date: str
    initial_cash: float = 100000000
    benchmark_symbol: Optional[str] = "VNINDEX"
    strategy: Dict[str, Any]
    sweep: List[SweepParameter]
    max_variants: int = 30


class StrategyLabRunCreate(BaseModel):
    run_type: str
    label: str
    request_config: Dict[str, Any] = Field(default_factory=dict)
    result_payload: Dict[str, Any] = Field(default_factory=dict)
    metrics: Dict[str, Any] = Field(default_factory=dict)


@router.post("/sweep")
async def run_parameter_sweep(config: ParameterSweepRequest, db: Session = Depends(get_db)):
    try:
        return await strategy_lab_service.run_parameter_sweep(db, config.model_dump())
    except ValueError as exc:
        return {
            "status": "failed",
            "error_code": "INVALID_SWEEP",
            "message": str(exc),
            "variants": [],
        }


@router.post("/runs")
def create_strategy_lab_run(payload: StrategyLabRunCreate, db: Session = Depends(get_db)):
    return StrategyLabHistoryService.create_run(db, payload.model_dump())


@router.get("/runs")
def list_strategy_lab_runs(limit: int = 50, db: Session = Depends(get_db)):
    return StrategyLabHistoryService.list_runs(db, limit=limit)


@router.get("/runs/{run_id}")
def get_strategy_lab_run(run_id: int, db: Session = Depends(get_db)):
    run = StrategyLabHistoryService.get_run(db, run_id)
    if not run:
        return {"status": "failed", "error_code": "NOT_FOUND", "message": "Strategy Lab run not found."}
    return run


@router.delete("/runs/{run_id}")
def delete_strategy_lab_run(run_id: int, db: Session = Depends(get_db)):
    deleted = StrategyLabHistoryService.delete_run(db, run_id)
    return {"status": "succeeded" if deleted else "failed", "deleted": deleted}
