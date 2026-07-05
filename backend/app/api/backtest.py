from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session
from app.dependencies import get_db
from app.services.backtest_service import BacktestService
from app.services.backtest_cleanup_service import BacktestCleanupService
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

class BacktestCleanupRequest(BaseModel):
    session_ids: Optional[List[int]] = None
    older_than_days: Optional[int] = None
    dry_run: bool = False

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

@router.post("/cleanup-sessions")
async def cleanup_backtest_sessions(config: BacktestCleanupRequest, db: Session = Depends(get_db)):
    """
    Delete only ReplaySession records created by backtests and their dependent records.
    Manual replay sessions are never selected by this endpoint.
    """
    return BacktestCleanupService.cleanup_backtest_sessions(
        db,
        session_ids=config.session_ids,
        older_than_days=config.older_than_days,
        dry_run=config.dry_run,
    )
