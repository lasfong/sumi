from fastapi import APIRouter
from app.config import settings

router = APIRouter()

@router.get("/health")
def health_check():
    return {
        "status": "ok",
        "message": "Sumi API is running",
        "version": settings.VERSION,
    }
