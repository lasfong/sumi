import uuid
from datetime import date, datetime, timezone
from unittest.mock import patch

import pytest
from fastapi import HTTPException
from sqlalchemy import create_engine, text
from sqlalchemy.exc import IntegrityError
from sqlalchemy.orm import sessionmaker

from app.db import Base, _set_sqlite_pragma
from app.dependencies import get_db
import app.models
from app.models.candle import Candle
from app.models.drawing import DrawingState
from app.models.import_run import ImportRun, ImportRunItem, ImportRunMutation, WeeklyCandleProvenance
from app.models.replay_session import ReplaySession
from app.models.sync_run import SyncRun, SyncRunItem, SyncRunMutation
from app.services.import_workflow_service import ImportWorkflowService
from app.services.sync_workflow_service import SyncWorkflowService
from app.services.weekly_aggregator import WeeklyAggregator
from app.config import settings


@pytest.fixture
def isolated_db(tmp_path):
    """Provides an isolated SQLite file database for transaction and FK verification."""
    db_file = tmp_path / "b01_test.db"
    db_url = f"sqlite:///{db_file.as_posix()}"
    test_engine = create_engine(db_url)
    Base.metadata.create_all(bind=test_engine)
    TestingSession = sessionmaker(autocommit=False, autoflush=False, bind=test_engine)
    session = TestingSession()
    try:
        yield session, test_engine
    finally:
        session.close()
        test_engine.dispose()


def test_tx06_new_db_connection_reports_pragma_foreign_keys_on(tmp_path):
    """TX-06: A newly-created DB connection reports: PRAGMA foreign_keys = 1."""
    from sqlalchemy import event, Engine

    # 1. Verify that the application FK listener is registered on Engine
    assert event.contains(Engine, "connect", _set_sqlite_pragma), (
        "Application FK listener _set_sqlite_pragma must be registered on Engine"
    )

    # 2. Verify independent temporary file-based SQLite engine receives PRAGMA foreign_keys = 1 via listener
    temp_db_file = tmp_path / "tx06_fk_test.db"
    file_engine = create_engine(f"sqlite:///{temp_db_file.as_posix()}")
    try:
        with file_engine.connect() as conn:
            fk_file = conn.execute(text("PRAGMA foreign_keys")).scalar()
            assert fk_file == 1, f"Expected PRAGMA foreign_keys = 1 on independent file engine, got {fk_file}"
    finally:
        file_engine.dispose()

    # 3. Verify newly-created in-memory SQLite engine also receives PRAGMA foreign_keys = 1 via listener
    mem_engine = create_engine("sqlite:///:memory:")
    try:
        with mem_engine.connect() as conn:
            fk_mem = conn.execute(text("PRAGMA foreign_keys")).scalar()
            assert fk_mem == 1, f"Expected PRAGMA foreign_keys = 1 on newly created engine, got {fk_mem}"
    finally:
        mem_engine.dispose()


def test_tx04_invalid_foreign_key_rejected(isolated_db):
    """TX-04: Invalid foreign-key child is rejected by the application's normal SQLAlchemy connection."""
    session, _ = isolated_db
    # Attempt to insert DrawingState pointing to non-existent replay session (id=999999)
    orphan_drawing = DrawingState(
        session_id=999999,
        symbol="VCB",
        state_data="[]"
    )
    session.add(orphan_drawing)
    with pytest.raises(IntegrityError):
        session.commit()
    session.rollback()

    # Attempt to insert ImportRunItem pointing to non-existent import run
    orphan_item = ImportRunItem(
        run_id=str(uuid.uuid4()),
        row_index=1,
        symbol="VCB",
        timeframe="1D",
        timestamp=datetime(2024, 1, 2),
        adjustment_type="unadjusted",
        open=80.0,
        high=82.0,
        low=79.0,
        close=81.0,
        volume=100000,
        classification="parsed"
    )
    session.add(orphan_item)
    with pytest.raises(IntegrityError):
        session.commit()
    session.rollback()


def test_tx05_valid_parent_child_persistence_and_cascade(isolated_db):
    """TX-05: Valid parent/child persistence succeeds and cascade deletion functions."""
    session, _ = isolated_db
    # Create parent ReplaySession
    replay_session = ReplaySession(
        symbol="VCB",
        start_date=date(2024, 1, 1),
        end_date=date(2024, 1, 31),
        initial_cash=100000000.0,
        current_cash=100000000.0,
        status="created"
    )
    session.add(replay_session)
    session.commit()
    session.refresh(replay_session)

    # Add child DrawingState
    drawing = DrawingState(
        session_id=replay_session.id,
        symbol="VCB",
        state_data='[{"type": "trendline"}]'
    )
    session.add(drawing)
    session.commit()

    assert session.query(DrawingState).filter(DrawingState.session_id == replay_session.id).count() == 1

    # Cascade delete verification: deleting parent deletes child
    session.delete(replay_session)
    session.commit()
    assert session.query(DrawingState).filter(DrawingState.session_id == replay_session.id).count() == 0


def test_tx01_tx02_tx03_import_weekly_failure_causes_complete_rollback(isolated_db):
    """TX-01, TX-02, TX-03: Failure during weekly/derived-data stage rolls back the parent workflow atomically."""
    session, _ = isolated_db
    run_id = str(uuid.uuid4())
    run = ImportRun(
        id=run_id,
        file_name="daily_data.csv",
        file_sha256="hash123",
        content_sha256="hash123",
        parser_version="cafef_v1",
        source_type="cafef",
        timeframe="1D",
        adjustment_type="unadjusted",
        timezone="Asia/Ho_Chi_Minh",
        status="previewed",
        can_accept=True
    )
    session.add(run)
    item = ImportRunItem(
        run_id=run_id,
        row_index=1,
        symbol="VCB",
        timeframe="1D",
        timestamp=datetime(2024, 1, 8),
        adjustment_type="unadjusted",
        open=80.0,
        high=82.0,
        low=79.0,
        close=81.0,
        volume=100000,
        classification="parsed"
    )
    session.add(item)
    session.commit()

    # Inject failure during WeeklyAggregator.derive_weekly_candles
    with patch.object(WeeklyAggregator, "derive_weekly_candles", side_effect=RuntimeError("Derived weekly failure")):
        with pytest.raises(RuntimeError, match="Derived weekly failure"):
            ImportWorkflowService.accept_import(session, run_id, "hash123")

    # Verify atomic rollback: zero partial state remains
    # 1. No candles inserted
    assert session.query(Candle).filter(Candle.symbol == "VCB").count() == 0
    # 2. No mutations inserted
    assert session.query(ImportRunMutation).filter(ImportRunMutation.run_id == run_id).count() == 0
    # 3. No weekly provenance
    assert session.query(WeeklyCandleProvenance).filter(WeeklyCandleProvenance.symbol == "VCB").count() == 0
    # 4. Run status was NOT committed as accepted; remains previewed
    run_in_db = session.query(ImportRun).filter(ImportRun.id == run_id).first()
    assert run_in_db.status == "previewed"
    assert run_in_db.accepted_at is None


def test_tx03_sync_weekly_failure_causes_complete_rollback(isolated_db, monkeypatch):
    """TX-03: Failure during weekly/derived-data stage in sync rolls back atomically."""
    monkeypatch.setattr(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", True)
    session, _ = isolated_db
    sync_id = str(uuid.uuid4())
    sync_run = SyncRun(
        id=sync_id,
        provider_id="ssi",
        symbol="FPT",
        start_date=date(2024, 1, 1),
        end_date=date(2024, 1, 15),
        timeframe="1D",
        adjustment_type="unadjusted",
        status="previewed",
        content_sha256="synchash456",
        parsed_count=1,
        can_accept=True
    )
    session.add(sync_run)
    item = SyncRunItem(
        sync_id=sync_id,
        row_index=1,
        symbol="FPT",
        timeframe="1D",
        timestamp=datetime(2024, 1, 8),
        adjustment_type="unadjusted",
        open=95.0,
        high=98.0,
        low=94.0,
        close=97.0,
        volume=50000,
        classification="parsed"
    )
    session.add(item)
    session.commit()

    with patch.object(WeeklyAggregator, "derive_weekly_candles", side_effect=RuntimeError("Sync weekly failure")):
        with pytest.raises(RuntimeError, match="Sync weekly failure"):
            SyncWorkflowService.execute_sync(session, sync_id, "synchash456")

    # Verify zero partial state in DB
    assert session.query(Candle).filter(Candle.symbol == "FPT").count() == 0
    assert session.query(SyncRunMutation).filter(SyncRunMutation.sync_id == sync_id).count() == 0
    run_in_db = session.query(SyncRun).filter(SyncRun.id == sync_id).first()
    assert run_in_db.status == "previewed"
    assert run_in_db.accepted_at is None


def test_tx07_failure_followed_by_valid_transaction_succeeds(isolated_db):
    """TX-07: Failure followed by another valid transaction succeeds, proving session recovery."""
    session, _ = isolated_db

    # 1. Trigger an integrity failure in session
    session.add(DrawingState(session_id=123456, symbol="FPT", state_data="[]"))
    with pytest.raises(IntegrityError):
        session.commit()

    # Session must be rolled back to recover
    session.rollback()

    # 2. Subsequent valid write in the same session must succeed without PendingRollbackError
    valid_session = ReplaySession(
        symbol="FPT",
        start_date=date(2024, 1, 1),
        end_date=date(2024, 1, 10),
        initial_cash=50000000.0,
        current_cash=50000000.0
    )
    session.add(valid_session)
    session.commit()
    assert valid_session.id is not None
    assert session.query(ReplaySession).filter(ReplaySession.id == valid_session.id).count() == 1


def test_tx07_get_db_dependency_rolls_back_on_exception(isolated_db):
    """TX-07: get_db dependency automatically rolls back when an exception occurs inside route."""
    session, _ = isolated_db
    with patch("app.dependencies.SessionLocal", return_value=session):
        db_gen = get_db()
        gen_session = next(db_gen)

        # Stage dirty uncommitted state on isolated db
        dirty_session = ReplaySession(
            symbol="TEST_ROLLBACK",
            start_date=date(2024, 1, 1),
            end_date=date(2024, 1, 10)
        )
        gen_session.add(dirty_session)
        gen_session.flush()

        # Simulate route exception propagated into generator
        with patch.object(gen_session, "rollback", wraps=gen_session.rollback) as mock_rollback:
            with pytest.raises(RuntimeError, match="Simulated route crash"):
                try:
                    raise RuntimeError("Simulated route crash")
                except Exception as e:
                    db_gen.throw(e)

            mock_rollback.assert_called_once()

    # Query DB directly to confirm uncommitted row was never persisted
    _, engine = isolated_db
    with engine.connect() as conn:
        count = conn.execute(text("SELECT count(*) FROM replay_sessions WHERE symbol = 'TEST_ROLLBACK'")).scalar()
        assert count == 0


def test_tx02_commit_failure_causes_complete_rollback(isolated_db):
    """TX-02: Failure at commit time triggers complete rollback leaving zero partial state."""
    session, _ = isolated_db
    run_id = str(uuid.uuid4())
    run = ImportRun(
        id=run_id,
        file_name="commit_fail.csv",
        file_sha256="hash_cf",
        content_sha256="hash_cf",
        parser_version="cafef_v1",
        source_type="cafef",
        timeframe="1D",
        adjustment_type="unadjusted",
        timezone="Asia/Ho_Chi_Minh",
        status="previewed",
        can_accept=True
    )
    session.add(run)
    item = ImportRunItem(
        run_id=run_id,
        row_index=1,
        symbol="HPG",
        timeframe="1D",
        timestamp=datetime(2024, 1, 8),
        adjustment_type="unadjusted",
        open=28.0,
        high=29.0,
        low=27.5,
        close=28.5,
        volume=80000,
        classification="parsed"
    )
    session.add(item)
    session.commit()

    with patch.object(session, "commit", side_effect=RuntimeError("Simulated commit disk I/O failure")):
        with pytest.raises(RuntimeError, match="Simulated commit disk I/O failure"):
            ImportWorkflowService.accept_import(session, run_id, "hash_cf")

    # Verify zero partial state in database
    assert session.query(Candle).filter(Candle.symbol == "HPG").count() == 0
    assert session.query(ImportRunMutation).filter(ImportRunMutation.run_id == run_id).count() == 0
    run_in_db = session.query(ImportRun).filter(ImportRun.id == run_id).first()
    assert run_in_db.status == "previewed"


def test_tx_rollback_import_failure_rolls_back_atomically(isolated_db):
    """Failure during import rollback (e.g. weekly recalculation) rolls back atomically."""
    session, _ = isolated_db
    run_id = str(uuid.uuid4())
    run = ImportRun(
        id=run_id,
        file_name="rollback_data.csv",
        file_sha256="hash_rb",
        content_sha256="hash_rb",
        parser_version="cafef_v1",
        source_type="cafef",
        timeframe="1D",
        adjustment_type="unadjusted",
        timezone="Asia/Ho_Chi_Minh",
        status="previewed",
        can_accept=True
    )
    session.add(run)
    item = ImportRunItem(
        run_id=run_id,
        row_index=1,
        symbol="TCB",
        timeframe="1D",
        timestamp=datetime(2024, 1, 8),
        adjustment_type="unadjusted",
        open=25.0,
        high=26.0,
        low=24.5,
        close=25.5,
        volume=50000,
        classification="parsed"
    )
    session.add(item)
    session.commit()

    ImportWorkflowService.accept_import(session, run_id, "hash_rb")
    assert session.query(Candle).filter(Candle.symbol == "TCB").count() >= 1

    # Inject failure during WeeklyAggregator.derive_weekly_candles inside rollback_import
    with patch.object(WeeklyAggregator, "derive_weekly_candles", side_effect=RuntimeError("Rollback derivation failed")):
        with pytest.raises(RuntimeError, match="Rollback derivation failed"):
            ImportWorkflowService.rollback_import(session, run_id)

    # Status remains accepted, candles NOT deleted
    run_after = session.query(ImportRun).filter(ImportRun.id == run_id).first()
    assert run_after.status == "accepted"
    assert session.query(Candle).filter(Candle.symbol == "TCB").count() >= 1


def test_tx_rollback_sync_failure_rolls_back_atomically(isolated_db, monkeypatch):
    """Failure during sync rollback (e.g. weekly recalculation) rolls back atomically."""
    monkeypatch.setattr(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", True)
    session, _ = isolated_db
    sync_id = str(uuid.uuid4())
    sync_run = SyncRun(
        id=sync_id,
        provider_id="ssi",
        symbol="MBB",
        start_date=date(2024, 1, 1),
        end_date=date(2024, 1, 15),
        timeframe="1D",
        adjustment_type="unadjusted",
        status="previewed",
        content_sha256="hash_sync_rb",
        parsed_count=1,
        can_accept=True
    )
    session.add(sync_run)
    item = SyncRunItem(
        sync_id=sync_id,
        row_index=1,
        symbol="MBB",
        timeframe="1D",
        timestamp=datetime(2024, 1, 8),
        adjustment_type="unadjusted",
        open=20.0,
        high=21.0,
        low=19.5,
        close=20.5,
        volume=30000,
        classification="parsed"
    )
    session.add(item)
    session.commit()

    SyncWorkflowService.execute_sync(session, sync_id, "hash_sync_rb")
    assert session.query(Candle).execution_options(include_quarantine=True).filter(Candle.symbol == "MBB").count() >= 1

    with patch.object(WeeklyAggregator, "derive_weekly_candles", side_effect=RuntimeError("Sync rollback failed")):
        with pytest.raises(RuntimeError, match="Sync rollback failed"):
            SyncWorkflowService.rollback_sync(session, sync_id)

    run_after = session.query(SyncRun).filter(SyncRun.id == sync_id).first()
    assert run_after.status == "accepted"
    assert session.query(Candle).execution_options(include_quarantine=True).filter(Candle.symbol == "MBB").count() >= 1


def test_tx_weekly_aggregator_internal_failure_rolls_back_session(isolated_db):
    """WeeklyAggregator's internal try/except rollback handler triggers on derivation error."""
    session, _ = isolated_db

    staged_candle = Candle(
        symbol="DIRTY_CANDLE",
        timeframe="1D",
        timestamp=datetime(2024, 1, 8),
        open=10.0,
        high=11.0,
        low=9.0,
        close=10.0,
        volume=1000,
        source="test",
        adjustment_type="unadjusted"
    )
    session.add(staged_candle)
    session.flush()

    with patch.object(WeeklyAggregator, "_derive_weekly_candles_impl", side_effect=RuntimeError("Internal math error")):
        with pytest.raises(RuntimeError, match="Internal math error"):
            WeeklyAggregator.derive_weekly_candles(session, {"DIRTY_CANDLE"}, {"unadjusted"})

    # Verify session was rolled back by WeeklyAggregator
    assert session.query(Candle).filter(Candle.symbol == "DIRTY_CANDLE").count() == 0


def test_tx08_normal_successful_write_path_commits_state(isolated_db):
    """TX-08: Normal successful write path commits the complete expected state."""
    session, _ = isolated_db
    run_id = str(uuid.uuid4())
    run = ImportRun(
        id=run_id,
        file_name="clean_data.csv",
        file_sha256="hash789",
        content_sha256="hash789",
        parser_version="cafef_v1",
        source_type="cafef",
        timeframe="1D",
        adjustment_type="unadjusted",
        timezone="Asia/Ho_Chi_Minh",
        status="previewed",
        can_accept=True
    )
    session.add(run)
    item = ImportRunItem(
        run_id=run_id,
        row_index=1,
        symbol="MWG",
        timeframe="1D",
        timestamp=datetime(2024, 1, 8),
        adjustment_type="unadjusted",
        open=45.0,
        high=46.0,
        low=44.5,
        close=45.5,
        volume=200000,
        classification="parsed"
    )
    session.add(item)
    session.commit()

    # Execute accept_import normally
    resp = ImportWorkflowService.accept_import(session, run_id, "hash789")
    assert resp.status == "accepted"

    # Verify committed state
    candles = session.query(Candle).filter(Candle.symbol == "MWG").all()
    assert len(candles) >= 1  # 1D candle + derived 1W candle
    run_in_db = session.query(ImportRun).filter(ImportRun.id == run_id).first()
    assert run_in_db.status == "accepted"
    assert run_in_db.accepted_count == 1


def test_tx_adv_partial_iteration_crash_rolls_back_earlier_items(isolated_db):
    """Adversarial: Failure midway through a multi-item batch rolls back earlier items."""
    session, _ = isolated_db
    run_id = str(uuid.uuid4())
    run = ImportRun(
        id=run_id,
        file_name="multi_item.csv",
        file_sha256="hash_multi",
        content_sha256="hash_multi",
        parser_version="cafef_v1",
        source_type="cafef",
        timeframe="1D",
        adjustment_type="unadjusted",
        timezone="Asia/Ho_Chi_Minh",
        status="previewed",
        can_accept=True
    )
    session.add(run)

    for i, sym in enumerate(["SYM_A", "SYM_B", "SYM_C"]):
        item = ImportRunItem(
            run_id=run_id,
            row_index=i + 1,
            symbol=sym,
            timeframe="1D",
            timestamp=datetime(2024, 1, 8),
            adjustment_type="unadjusted",
            open=10.0,
            high=11.0,
            low=9.0,
            close=10.5,
            volume=1000,
            classification="parsed"
        )
        session.add(item)
    session.commit()

    # Fault injection: inject error during the 3rd item's candle addition
    original_add = session.add
    call_count = 0

    def fail_on_third_candle(entity):
        nonlocal call_count
        if isinstance(entity, Candle) and entity.symbol == "SYM_C":
            raise RuntimeError("Injected failure during 3rd candle insertion")
        return original_add(entity)

    with patch.object(session, "add", side_effect=fail_on_third_candle):
        with pytest.raises(RuntimeError, match="Injected failure during 3rd candle"):
            ImportWorkflowService.accept_import(session, run_id, "hash_multi")

    # Verify atomic rollback: SYM_A and SYM_B must NOT have persisted candles or mutations
    assert session.query(Candle).filter(Candle.symbol.in_(["SYM_A", "SYM_B", "SYM_C"])).count() == 0
    assert session.query(ImportRunMutation).filter(ImportRunMutation.run_id == run_id).count() == 0
    run_in_db = session.query(ImportRun).filter(ImportRun.id == run_id).first()
    assert run_in_db.status == "previewed"


def test_tx_adv_foreign_key_multilevel_enforcement(isolated_db):
    """Adversarial: Multi-level FK constraints: Order -> Execution and ReplaySession protection."""
    from app.models.decision import Decision
    from app.models.order import Order
    from app.models.execution import Execution

    session, _ = isolated_db

    # 1. Attempting to insert Execution with non-existent order_id must fail
    orphan_exec = Execution(
        session_id=1,
        order_id=99999,
        symbol="VCB",
        quantity=100.0,
        execution_price=80.0,
        gross_amount=8000.0,
        net_amount=8100.0,
        execution_date=datetime.now(timezone.utc)
    )
    session.add(orphan_exec)
    with pytest.raises(IntegrityError):
        session.commit()
    session.rollback()

    # 2. Create valid hierarchy: ReplaySession -> Decision -> Order -> Execution
    replay_sess = ReplaySession(
        symbol="VCB",
        start_date=date(2024, 1, 1),
        end_date=date(2024, 1, 10),
        initial_cash=100000000.0,
        current_cash=100000000.0
    )
    session.add(replay_sess)
    session.commit()

    decision = Decision(
        session_id=replay_sess.id,
        symbol="VCB",
        action="BUY",
        candle_index=0,
        decision_date=datetime.now(timezone.utc)
    )
    session.add(decision)
    session.commit()

    order = Order(
        session_id=replay_sess.id,
        decision_id=decision.id,
        symbol="VCB",
        order_type="MARKET",
        side="BUY",
        quantity=100.0,
        requested_price=80.0,
        status="executed"
    )
    session.add(order)
    session.commit()

    execution = Execution(
        session_id=replay_sess.id,
        order_id=order.id,
        symbol="VCB",
        quantity=100.0,
        execution_price=80.0,
        gross_amount=8000.0,
        net_amount=8100.0,
        execution_date=datetime.now(timezone.utc)
    )
    session.add(execution)
    session.commit()

    # 3. Deleting ReplaySession when Order/Decision exist without cascade on Order must raise IntegrityError
    session.delete(replay_sess)
    with pytest.raises(IntegrityError):
        session.commit()
    session.rollback()

    # Verify session and order remain intact after rollback
    assert session.query(Order).filter(Order.id == order.id).count() == 1


def test_tx_adv_atomic_helper_context_and_decorator(isolated_db):
    """Adversarial: Verify atomic() helper from app.db works as both context manager and decorator."""
    from app.db import atomic
    session, _ = isolated_db

    # Context manager success commits
    with atomic(session):
        s1 = ReplaySession(
            symbol="ATOM_1",
            start_date=date(2024, 1, 1),
            end_date=date(2024, 1, 10)
        )
        session.add(s1)

    assert session.query(ReplaySession).filter(ReplaySession.symbol == "ATOM_1").count() == 1

    # Context manager failure rolls back
    with pytest.raises(ValueError, match="Intentional atomic failure"):
        with atomic(session):
            s2 = ReplaySession(
                symbol="ATOM_2",
                start_date=date(2024, 1, 1),
                end_date=date(2024, 1, 10)
            )
            session.add(s2)
            raise ValueError("Intentional atomic failure")

    assert session.query(ReplaySession).filter(ReplaySession.symbol == "ATOM_2").count() == 0

    # Decorator usage success
    @atomic
    def successful_op(db):
        s3 = ReplaySession(
            symbol="ATOM_3",
            start_date=date(2024, 1, 1),
            end_date=date(2024, 1, 10)
        )
        db.add(s3)
        return s3

    successful_op(db=session)
    assert session.query(ReplaySession).filter(ReplaySession.symbol == "ATOM_3").count() == 1

    # Decorator usage failure rolls back
    @atomic
    def failed_op(db):
        s4 = ReplaySession(
            symbol="ATOM_4",
            start_date=date(2024, 1, 1),
            end_date=date(2024, 1, 10)
        )
        db.add(s4)
        raise RuntimeError("Decorator crash")

    with pytest.raises(RuntimeError, match="Decorator crash"):
        failed_op(db=session)

    assert session.query(ReplaySession).filter(ReplaySession.symbol == "ATOM_4").count() == 0

