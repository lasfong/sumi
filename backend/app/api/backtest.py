from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session
from app.dependencies import get_db
from app.services.backtest_service import BacktestService
from pydantic import BaseModel
from typing import Dict, Any, Optional, List

router = APIRouter()
backtest_service = BacktestService()

class BacktestRequest(BaseModel):
    symbol: Optional[str] = None
    symbols: Optional[List[str]] = None
    start_date: str
    end_date: str
    initial_cash: float = 100000000
    benchmark_symbol: Optional[str] = "VNINDEX"
    strategy: Dict[str, Any]

@router.post("/run")
async def run_backtest(config: BacktestRequest, db: Session = Depends(get_db)):
    """
    Chạy backtest cho strategy.
    """
    result = await backtest_service.run_backtest(db, config.dict())
    return result

@router.get("/strategies")
async def list_strategies():
    """Liệt kê strategies có sẵn."""
    from app.domain.strategy.strategy_loader import list_available_strategies
    return list_available_strategies()
