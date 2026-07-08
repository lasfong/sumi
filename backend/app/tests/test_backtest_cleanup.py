from datetime import date, datetime

from fastapi.testclient import TestClient

from app.dependencies import get_db
from app.main import app
from app.models.decision import Decision
from app.models.drawing import DrawingState
from app.models.event_log import EventLog
from app.models.execution import Execution
from app.models.journal_entry import JournalEntry
from app.models.order import Order
from app.models.position import Position
from app.models.replay_session import ReplaySession
from app.models.trade import Trade
from app.services.backtest_cleanup_service import BacktestCleanupService


def _create_session(db_session, mode: str, symbol: str) -> ReplaySession:
    session = ReplaySession(
        symbol=symbol,
        timeframe="1D",
        adjustment_type="unadjusted",
        start_date=date(2024, 1, 1),
        end_date=date(2024, 1, 10),
        current_index=3,
        initial_cash=100_000_000,
        current_cash=99_000_000,
        status="completed",
        mode=mode,
    )
    db_session.add(session)
    db_session.flush()
    return session


def _add_related_records(db_session, session: ReplaySession) -> None:
    decision = Decision(
        session_id=session.id,
        symbol=session.symbol,
        decision_date=datetime(2024, 1, 2),
        candle_index=1,
        action="BUY",
        price=100.0,
    )
    db_session.add(decision)
    db_session.flush()

    order = Order(
        session_id=session.id,
        decision_id=decision.id,
        symbol=session.symbol,
        side="BUY",
        order_type="MARKET_AT_CLOSE",
        quantity=100,
        status="executed",
    )
    trade = Trade(
        session_id=session.id,
        symbol=session.symbol,
        entry_date=datetime(2024, 1, 2),
        entry_price=100.0,
        quantity=100,
        status="open",
    )
    position = Position(
        session_id=session.id,
        symbol=session.symbol,
        quantity=100,
        average_price=100.0,
        total_cost=10_000.0,
        opened_at=datetime(2024, 1, 2),
    )
    drawing = DrawingState(session_id=session.id, symbol=session.symbol, state_data="[]")
    event = EventLog(session_id=session.id, event_type="TEST", message="test")
    db_session.add_all([order, trade, position, drawing, event])
    db_session.flush()

    execution = Execution(
        session_id=session.id,
        order_id=order.id,
        trade_id=trade.id,
        symbol=session.symbol,
        execution_date=datetime(2024, 1, 2),
        execution_price=100.0,
        quantity=100,
        gross_amount=10_000.0,
        net_amount=10_015.0,
    )
    journal = JournalEntry(
        session_id=session.id,
        decision_id=decision.id,
        trade_id=trade.id,
        note_type="session_review",
        content="test",
    )
    db_session.add_all([execution, journal])


def test_cleanup_deletes_only_backtest_sessions_and_related_records(db_session):
    backtest_session = _create_session(db_session, "backtest", "BT_CLEAN")
    manual_session = _create_session(db_session, "normal", "MANUAL_KEEP")
    _add_related_records(db_session, backtest_session)
    _add_related_records(db_session, manual_session)
    db_session.commit()
    backtest_session_id = backtest_session.id
    manual_session_id = manual_session.id

    result = BacktestCleanupService.cleanup_backtest_sessions(db_session)

    assert result["status"] == "succeeded"
    assert result["deleted_session_ids"] == [backtest_session_id]
    assert result["deleted_counts"]["decisions"] == 1
    assert result["deleted_counts"]["orders"] == 1
    assert result["deleted_counts"]["executions"] == 1
    assert result["deleted_counts"]["positions"] == 1
    assert result["deleted_counts"]["trades"] == 1
    assert result["deleted_counts"]["journal_entries"] == 1
    assert result["deleted_counts"]["drawing_states"] == 1
    assert result["deleted_counts"]["event_logs"] == 1

    assert db_session.query(ReplaySession).filter_by(id=backtest_session_id).first() is None
    assert db_session.query(ReplaySession).filter_by(id=manual_session_id).first() is not None
    for model in (Decision, Order, Execution, Position, Trade, JournalEntry, DrawingState, EventLog):
        assert db_session.query(model).filter_by(session_id=backtest_session_id).count() == 0
        assert db_session.query(model).filter_by(session_id=manual_session_id).count() == 1


def test_cleanup_endpoint_supports_dry_run_without_deleting_manual_sessions(db_session):
    backtest_session = _create_session(db_session, "backtest", "BT_DRY")
    manual_session = _create_session(db_session, "normal", "MANUAL_DRY")
    _add_related_records(db_session, backtest_session)
    _add_related_records(db_session, manual_session)
    db_session.commit()

    def override_get_db():
        try:
            yield db_session
        finally:
            pass

    app.dependency_overrides[get_db] = override_get_db
    try:
        with TestClient(app) as client:
            response = client.post(
                "/api/backtest/cleanup-sessions",
                json={"session_ids": [backtest_session.id, manual_session.id], "dry_run": True},
            )
    finally:
        app.dependency_overrides.clear()

    assert response.status_code == 200, response.text
    payload = response.json()
    assert payload["status"] == "dry_run"
    assert payload["deleted_session_ids"] == [backtest_session.id]
    assert db_session.query(ReplaySession).filter_by(id=backtest_session.id).first() is not None
    assert db_session.query(ReplaySession).filter_by(id=manual_session.id).first() is not None
