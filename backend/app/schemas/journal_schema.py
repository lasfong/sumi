from pydantic import BaseModel, ConfigDict
from datetime import datetime
from typing import Optional

class JournalEntryBase(BaseModel):
    note_type: str
    content: str
    tags: Optional[str] = None
    decision_id: Optional[int] = None
    trade_id: Optional[int] = None

class JournalEntryCreate(JournalEntryBase):
    pass

class JournalEntryResponse(JournalEntryBase):
    id: int
    session_id: int
    created_at: datetime
    updated_at: datetime
    model_config = ConfigDict(from_attributes=True)
