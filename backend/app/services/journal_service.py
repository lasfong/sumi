import json
from datetime import date

from fastapi import HTTPException
from sqlalchemy.orm import Session

from app.models.decision import Decision
from app.models.journal_entry import JournalEntry
from app.models.trade import Trade
from app.schemas.journal_schema import JournalEntryCreate
from app.services.replay_service import ReplayService


CHECKLIST_FIELDS = {
    "trendIdentified",
    "setupConfirmed",
    "entryTriggerDefined",
    "riskDefined",
    "exitPlanDefined",
    "emotionChecked",
}


class JournalService:
    @staticmethod
    def _check_associations(db: Session, session_id: int, entry_in: JournalEntryCreate) -> None:
        if entry_in.decision_id is not None:
            decision = db.query(Decision).filter(Decision.id == entry_in.decision_id).first()
            if not decision or decision.session_id != session_id:
                raise HTTPException(status_code=400, detail="Journal decision does not belong to this replay session")
        if entry_in.trade_id is not None:
            trade = db.query(Trade).filter(Trade.id == entry_in.trade_id).first()
            if not trade or trade.session_id != session_id:
                raise HTTPException(status_code=400, detail="Journal trade does not belong to this replay session")

    @staticmethod
    def _validate_checklist(db: Session, session, content: str) -> dict:
        try:
            payload = json.loads(content)
        except json.JSONDecodeError as error:
            raise HTTPException(status_code=400, detail="Practice checklist content must be valid JSON") from error
        if not isinstance(payload, dict) or payload.get("schemaVersion") != 1:
            raise HTTPException(status_code=400, detail="Unsupported practice checklist schema")
        if set(payload) != {"schemaVersion", "context", "checks", "observation"}:
            raise HTTPException(status_code=400, detail="Practice checklist contains unknown fields")
        context = payload.get("context")
        checks = payload.get("checks")
        if not isinstance(context, dict) or set(context) != {"sessionId", "symbol", "candleIndex", "date"}:
            raise HTTPException(status_code=400, detail="Practice checklist context is invalid")
        if context.get("sessionId") != session.id or context.get("symbol") != session.symbol or context.get("candleIndex") != session.current_index:
            raise HTTPException(status_code=409, detail="Practice checklist context is stale or belongs to another workspace")
        context_date = context.get("date")
        if not isinstance(context_date, str) or not context_date:
            raise HTTPException(status_code=400, detail="Practice checklist date is required")
        try:
            parsed_date = date.fromisoformat(context_date)
        except ValueError as error:
            raise HTTPException(status_code=400, detail="Practice checklist date must be a valid YYYY-MM-DD date") from error
        if parsed_date.isoformat() != context_date:
            raise HTTPException(status_code=400, detail="Practice checklist date must use canonical YYYY-MM-DD format")
        candles = ReplayService.get_candles(db, session.id)
        if len(candles) != session.current_index + 1:
            raise HTTPException(status_code=409, detail="Authoritative replay candle could not be resolved")
        authoritative_date = candles[-1].timestamp.date().isoformat()
        if context_date != authoritative_date:
            raise HTTPException(status_code=409, detail="Practice checklist date does not match the current replay candle")
        if not isinstance(checks, dict) or set(checks) != CHECKLIST_FIELDS or any(type(value) is not bool for value in checks.values()):
            raise HTTPException(status_code=400, detail="Practice checklist fields are invalid")
        observation = payload.get("observation")
        if not isinstance(observation, str) or len(observation.strip()) > 4000:
            raise HTTPException(status_code=400, detail="Practice checklist observation is invalid")
        return payload

    @staticmethod
    def create(db: Session, session_id: int, entry_in: JournalEntryCreate) -> JournalEntry:
        session = ReplayService.get_session(db, session_id)
        content = entry_in.content.strip()
        if not content or len(content) > 10_000:
            raise HTTPException(status_code=400, detail="Journal content must contain 1 to 10000 characters")
        JournalService._check_associations(db, session_id, entry_in)
        if entry_in.note_type == "practice_checklist":
            JournalService._validate_checklist(db, session, content)
        entry = JournalEntry(
            session_id=session_id,
            decision_id=entry_in.decision_id,
            trade_id=entry_in.trade_id,
            note_type=entry_in.note_type,
            content=content,
            tags=entry_in.tags,
        )
        db.add(entry)
        db.commit()
        db.refresh(entry)
        return entry

    @staticmethod
    def list_visible(db: Session, session_id: int) -> list[JournalEntry]:
        session = ReplayService.get_session(db, session_id)
        decisions = {
            decision.id: decision for decision in db.query(Decision).filter(Decision.session_id == session_id).all()
        }
        entries = db.query(JournalEntry).filter(JournalEntry.session_id == session_id).order_by(JournalEntry.created_at.desc(), JournalEntry.id.desc()).all()
        visible = []
        for entry in entries:
            if entry.decision_id is not None:
                decision = decisions.get(entry.decision_id)
                if decision and decision.candle_index > session.current_index:
                    continue
            if entry.note_type == "practice_checklist":
                try:
                    payload = json.loads(entry.content)
                    index = payload["context"]["candleIndex"]
                except (json.JSONDecodeError, KeyError, TypeError):
                    continue
                if not isinstance(index, int) or index > session.current_index:
                    continue
            visible.append(entry)
        return visible
