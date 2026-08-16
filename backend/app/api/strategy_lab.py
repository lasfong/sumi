from typing import Any, Dict, List, Optional

from fastapi import APIRouter, Depends
from pydantic import BaseModel, Field
from sqlalchemy.orm import Session

from app.dependencies import get_db
from app.services.strategy_lab_history_service import StrategyLabHistoryService
from app.services.strategy_lab_service import StrategyLabService, SweepCancellationManager


router = APIRouter()
strategy_lab_service = StrategyLabService()


class SweepParameter(BaseModel):
    path: Optional[str] = None
    target_type: Optional[str] = None  # "indicator", "position_sizing", "risk_management"
    target_name: Optional[str] = None  # e.g. "sma_short"
    parameter: Optional[str] = None    # e.g. "length"
    values: List[Any]


class ParameterSweepRequest(BaseModel):
    sweep_id: Optional[str] = None
    symbol: Optional[str] = None
    symbols: Optional[List[str]] = None
    start_date: str
    end_date: str
    oos_start_date: Optional[str] = None
    oos_end_date: Optional[str] = None
    initial_cash: float = 100000000
    benchmark_symbol: Optional[str] = "VNINDEX"
    strategy: Dict[str, Any]
    sweep: List[SweepParameter]
    max_variants: int = 30


class SweepCancelRequest(BaseModel):
    sweep_id: str


class StrategyLabRunCreate(BaseModel):
    run_type: str
    label: str
    request_config: Dict[str, Any] = Field(default_factory=dict)
    result_payload: Dict[str, Any] = Field(default_factory=dict)
    metrics: Dict[str, Any] = Field(default_factory=dict)


class StrategyValidationRequest(BaseModel):
    strategy: Dict[str, Any]


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


@router.post("/sweep/cancel")
def cancel_parameter_sweep(payload: SweepCancelRequest):
    SweepCancellationManager.cancel_sweep(payload.sweep_id)
    return {
        "status": "succeeded",
        "message": f"Cancellation requested for sweep {payload.sweep_id}",
        "sweep_id": payload.sweep_id,
    }


@router.post("/validate")
def validate_strategy(payload: StrategyValidationRequest):
    return StrategyLabService.validate_strategy_definition(payload.strategy)


@router.post("/parameters")
def get_strategy_parameters(payload: StrategyValidationRequest):
    return {
        "status": "succeeded",
        "parameters": StrategyLabService.get_strategy_parameters(payload.strategy),
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
