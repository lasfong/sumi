from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session
from typing import List
from app.dependencies import get_db
from app.schemas.journal_schema import JournalEntryCreate, JournalEntryResponse
from app.models.journal_entry import JournalEntry

router = APIRouter()

@router.post("/sessions/{session_id}/journal", response_model=JournalEntryResponse)
def create_journal_entry(session_id: int, entry_in: JournalEntryCreate, db: Session = Depends(get_db)):
    entry = JournalEntry(
        session_id=session_id,
        decision_id=entry_in.decision_id,
        trade_id=entry_in.trade_id,
        note_type=entry_in.note_type,
        content=entry_in.content,
        tags=entry_in.tags
    )
    db.add(entry)
    db.commit()
    db.refresh(entry)
    return entry

@router.get("/sessions/{session_id}/journal", response_model=List[JournalEntryResponse])
def get_journal_entries(session_id: int, db: Session = Depends(get_db)):
    return db.query(JournalEntry).filter(JournalEntry.session_id == session_id).order_by(JournalEntry.created_at.desc()).all()
