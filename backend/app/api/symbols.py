from fastapi import APIRouter, Depends, Query
from sqlalchemy.orm import Session
from typing import List, Optional
from app.dependencies import get_db
from app.schemas.symbol_schema import SymbolResponse
from app.schemas.readiness_schema import DataReadinessResponse
from app.models.symbol import Symbol
from app.services.readiness_service import ReadinessService

router = APIRouter()


@router.get("/symbols/readiness", response_model=DataReadinessResponse)
def get_data_readiness(db: Session = Depends(get_db)):
    return ReadinessService.get_readiness(db)


@router.get("/symbols", response_model=List[SymbolResponse])
def list_symbols(
    asset_type: Optional[str] = None,
    exchange: Optional[str] = None,
    search: Optional[str] = Query(None, description="Search by symbol name"),
    db: Session = Depends(get_db)
):
    query = db.query(Symbol)
    if asset_type:
        query = query.filter(Symbol.asset_type == asset_type)
    if exchange:
        query = query.filter(Symbol.exchange == exchange)
    if search:
        query = query.filter(Symbol.symbol.ilike(f"%{search}%"))
    return query.order_by(Symbol.symbol).all()
