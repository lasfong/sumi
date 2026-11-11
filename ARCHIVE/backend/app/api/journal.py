from fastapi import APIRouter, Depends, Query
from fastapi.responses import JSONResponse, PlainTextResponse
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

@router.get("/sessions/{session_id}/journal/export")
def export_journal(
    session_id: int,
    format: str = Query("json", pattern="^(json|csv)$"),
    db: Session = Depends(get_db),
):
    if format == "csv":
        csv_data = JournalService.export_session_journal_csv(db, session_id)
        return PlainTextResponse(
            content=csv_data,
            media_type="text/csv",
            headers={"Content-Disposition": f'attachment; filename="session_{session_id}_journal.csv"'},
        )
    json_data = JournalService.export_session_journal_json(db, session_id)
    return JSONResponse(content=json_data)
