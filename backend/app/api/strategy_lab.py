from typing import Any, Dict, List, Optional

from fastapi import APIRouter, Depends
from pydantic import BaseModel
from sqlalchemy.orm import Session

from app.dependencies import get_db
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
