import pytest
import io
from datetime import datetime, date
from app.models.candle import Candle
from app.models.symbol import Symbol
from app.models.import_run import ImportRun, ImportRunMutation
from app.services.import_workflow_service import ImportWorkflowService

def test_preview_does_not_mutate_accepted_candles(db_session):
    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nPREVIEW_SYM,20260105,25.0,26.0,24.5,25.5,500000\nPREVIEW_SYM,20260106,25.5,26.5,25.0,26.0,600000\n"
    initial_count = db_session.query(Candle).filter(Candle.symbol == "PREVIEW_SYM").count()
    
    res = ImportWorkflowService.preview_import(
        db=db_session,
        file_content=csv_content,
        filename="PREVIEW_SYM_2026.csv",
        adjustment_type="unadjusted"
    )

    assert res.can_accept is True
    assert res.parsed_count == 2
    assert res.status == "previewed"

    # Verify Candles table is untouched for PREVIEW_SYM
    candles_count = db_session.query(Candle).filter(Candle.symbol == "PREVIEW_SYM").count()
    assert candles_count == initial_count == 0


def test_explicit_accept_and_weekly_derivation(db_session):
    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260105,25.0,26.0,24.5,25.5,500000\nHPG,20260106,25.5,26.5,25.0,26.0,600000\n"
    preview = ImportWorkflowService.preview_import(
        db=db_session,
        file_content=csv_content,
        filename="HPG_2026.csv",
        adjustment_type="unadjusted"
    )

    accept_res = ImportWorkflowService.accept_import(
        db=db_session,
        run_id=preview.run_id,
        content_sha256=preview.content_sha256
    )

    assert accept_res.status == "accepted"
    assert accept_res.accepted_count == 2

    # Verify 1D candles
    daily_candles = db_session.query(Candle).filter(Candle.symbol == "HPG", Candle.timeframe == "1D").all()
    assert len(daily_candles) == 2

    # Verify 1W weekly candle auto-derived
    weekly_candles = db_session.query(Candle).filter(Candle.symbol == "HPG", Candle.timeframe == "1W").all()
    assert len(weekly_candles) == 1
    assert weekly_candles[0].open == 25.0
    assert weekly_candles[0].close == 26.0

def test_idempotent_reaccept(db_session):
    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260105,25.0,26.0,24.5,25.5,500000\n"
    preview = ImportWorkflowService.preview_import(db=db_session, file_content=csv_content, filename="HPG.csv")
    accept1 = ImportWorkflowService.accept_import(db=db_session, run_id=preview.run_id, content_sha256=preview.content_sha256)
    assert accept1.status == "accepted"

    # Re-accepting same run
    accept2 = ImportWorkflowService.accept_import(db=db_session, run_id=preview.run_id, content_sha256=preview.content_sha256)
    assert accept2.status == "noop"
    assert accept2.accepted_count == 0

def test_rollback_restores_previous_state(db_session):
    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260105,25.0,26.0,24.5,25.5,500000\n"
    preview = ImportWorkflowService.preview_import(db=db_session, file_content=csv_content, filename="HPG.csv")
    ImportWorkflowService.accept_import(db=db_session, run_id=preview.run_id, content_sha256=preview.content_sha256)

    assert db_session.query(Candle).filter(Candle.symbol == "HPG").count() > 0

    rollback_res = ImportWorkflowService.rollback_import(db=db_session, run_id=preview.run_id)
    assert rollback_res.status == "rolled_back"

    # Verify candles deleted
    assert db_session.query(Candle).filter(Candle.symbol == "HPG").count() == 0

def test_catalog_query(db_session):
    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nFPT,20260105,90.0,92.0,89.5,91.0,300000\n"
    preview = ImportWorkflowService.preview_import(db=db_session, file_content=csv_content, filename="FPT.csv")
    ImportWorkflowService.accept_import(db=db_session, run_id=preview.run_id, content_sha256=preview.content_sha256)

    catalog = ImportWorkflowService.get_catalog(db_session)
    assert len(catalog) >= 1
    fpt_items = [c for c in catalog if c.symbol == "FPT" and c.timeframe == "1D"]
    assert len(fpt_items) == 1
    assert fpt_items[0].provenance_state == "import_run"
    assert fpt_items[0].row_count == 1
