from typing import List, Optional
from fastapi import APIRouter, Depends, Query
from sqlalchemy import case, func
from sqlalchemy.orm import Session

from app.dependencies import get_db
from app.models.symbol import Symbol
from app.schemas.import_schema import CatalogItemSchema
from app.schemas.readiness_schema import DataReadinessResponse
from app.schemas.symbol_schema import SymbolResponse
from app.services.import_workflow_service import ImportWorkflowService
from app.services.readiness_service import ReadinessService

router = APIRouter()


@router.get("/data/catalog", response_model=List[CatalogItemSchema])
def get_data_catalog_endpoint(db: Session = Depends(get_db)):
    return ImportWorkflowService.get_catalog(db)


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
        s = search.strip().upper()
        is_exact = case((func.upper(Symbol.symbol) == s, 0), else_=1)
        is_short_stock = case((func.length(Symbol.symbol) <= 5, 0), else_=1)
        is_prefix = case((func.upper(Symbol.symbol).like(f"{s}%"), 0), else_=1)
        query = query.filter(Symbol.symbol.ilike(f"%{s}%"))
        return query.order_by(is_exact, is_short_stock, is_prefix, Symbol.symbol).limit(50).all()
    return query.order_by(Symbol.symbol).all()
