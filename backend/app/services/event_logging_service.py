import json
from sqlalchemy.orm import Session
from app.models.event_log import EventLog
from typing import Optional, Dict, Any

class EventLoggingService:
    @staticmethod
    def log_event(
        db: Session, 
        event_type: str, 
        message: str, 
        session_id: Optional[int] = None, 
        details: Optional[Dict[str, Any]] = None
    ) -> EventLog:
        """
        Logs a system event.
        """
        log_entry = EventLog(
            session_id=session_id,
            event_type=event_type,
            message=message,
            details=json.dumps(details) if details else None
        )
        db.add(log_entry)
        db.commit()
        db.refresh(log_entry)
        return log_entry
