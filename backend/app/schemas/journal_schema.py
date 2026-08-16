from pydantic import BaseModel, ConfigDict
from datetime import datetime
from typing import Optional, Union

class JournalEntryBase(BaseModel):
    note_type: str
    content: str
    tags: Optional[str] = None
    decision_id: Optional[int] = None
    trade_id: Optional[int] = None
    setup_type: Optional[str] = None
    market_regime: Optional[str] = None
    confidence_score: Optional[int] = None
    emotion: Optional[str] = None
    mistake_tag: Optional[str] = None
    rule_violation: Optional[str] = None
    checklist_snapshot: Optional[Union[dict, str]] = None

class JournalEntryCreate(JournalEntryBase):
    pass

class JournalEntryResponse(JournalEntryBase):
    id: int
    session_id: int
    created_at: datetime
    updated_at: datetime
    model_config = ConfigDict(from_attributes=True)
