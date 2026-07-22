from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session
from typing import List
from app.dependencies import get_db
from app.schemas.journal_schema import JournalEntryCreate, JournalEntryResponse
from app.services.journal_service import JournalService

router = APIRouter()

@router.post("/sessions/{session_id}/journal", response_model=JournalEntryResponse)
def create_journal_entry(session_id: int, entry_in: JournalEntryCreate, db: Session = Depends(get_db)):
    return JournalService.create(db, session_id, entry_in)

@router.get("/sessions/{session_id}/journal", response_model=List[JournalEntryResponse])
def get_journal_entries(session_id: int, db: Session = Depends(get_db)):
    return JournalService.list_visible(db, session_id)
