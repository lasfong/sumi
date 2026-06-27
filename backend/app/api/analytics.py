from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.orm import Session
from app.dependencies import get_db
from app.schemas.analytics_schema import AnalyticsResponse
from app.services.analytics_service import AnalyticsService

router = APIRouter()

@router.get("/sessions/{session_id}/analytics", response_model=AnalyticsResponse)
def get_session_analytics(session_id: int, db: Session = Depends(get_db)):
    try:
        return AnalyticsService.get_analytics(db, session_id)
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))
