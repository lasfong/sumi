from datetime import datetime, timedelta, timezone

from sqlalchemy.orm import Session

from app.models.decision import Decision
from app.models.drawing import DrawingState
from app.models.event_log import EventLog
from app.models.execution import Execution
from app.models.journal_entry import JournalEntry
from app.models.order import Order
from app.models.position import Position
from app.models.replay_session import ReplaySession
from app.models.trade import Trade


class BacktestCleanupService:
    """Deletes only backtest-created replay sessions and their dependent records."""

    RELATED_MODELS = (
        JournalEntry,
        DrawingState,
        EventLog,
        Execution,
        Order,
        Decision,
        Position,
        Trade,
    )

    @staticmethod
    def cleanup_backtest_sessions(
        db: Session,
        *,
        session_ids: list[int] | None = None,
        older_than_days: int | None = None,
        dry_run: bool = False,
    ) -> dict:
        query = db.query(ReplaySession.id).filter(ReplaySession.mode == "backtest")

        if session_ids:
            query = query.filter(ReplaySession.id.in_(session_ids))

        if older_than_days is not None:
            cutoff = datetime.now(timezone.utc) - timedelta(days=older_than_days)
            query = query.filter(ReplaySession.created_at < cutoff)

        ids = [row[0] for row in query.all()]
        counts = BacktestCleanupService._count_related(db, ids)
        counts["replay_sessions"] = len(ids)

        if dry_run or not ids:
            return {
                "status": "dry_run" if dry_run else "succeeded",
                "deleted_session_ids": ids,
                "deleted_counts": counts,
            }

        for model in BacktestCleanupService.RELATED_MODELS:
            db.query(model).filter(model.session_id.in_(ids)).delete(synchronize_session=False)
        db.query(ReplaySession).filter(ReplaySession.id.in_(ids)).delete(synchronize_session=False)
        db.commit()

        return {
            "status": "succeeded",
            "deleted_session_ids": ids,
            "deleted_counts": counts,
        }

    @staticmethod
    def _count_related(db: Session, session_ids: list[int]) -> dict:
        names = {
            JournalEntry: "journal_entries",
            DrawingState: "drawing_states",
            EventLog: "event_logs",
            Execution: "executions",
            Order: "orders",
            Decision: "decisions",
            Position: "positions",
            Trade: "trades",
        }
        counts = {}
        for model, name in names.items():
            counts[name] = (
                db.query(model)
                .filter(model.session_id.in_(session_ids))
                .count()
                if session_ids
                else 0
            )
        return counts
