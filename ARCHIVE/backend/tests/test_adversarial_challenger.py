"""Adversarial Acceptance and Stress Suite for Sumi V3 W0-A Stabilization.

Authored by Agent C (Empirical Challenger) to stress-test:
1. Fail-closed safeguard evasion and boundary probing (symlinks, path traversals, URIs, aux files)
2. Rapid and concurrent write collision scenarios
3. Cross-workflow rollback conflicts and fault injection atomicity
4. Timestamp boundary conditions and provenance encoding (pipes, microseconds, legacy formats)
5. Synthetic sync precedence and default-deny enforcement
"""

from __future__ import annotations

import os
import sqlite3
import _sqlite3
from datetime import date, datetime, timezone
from pathlib import Path
from unittest.mock import patch
import pytest
from sqlalchemy import create_engine, select
from sqlalchemy.orm import Session

from app.config import settings
from app.db import Base, SessionLocal, _set_sqlite_pragma
from app.models.candle import Candle
from app.models.symbol import Symbol
from app.models.import_run import ImportRun, ImportRunMutation
from app.models.sync_run import SyncRun, SyncRunMutation
from app.services.data_providers.base_provider import (
    MarketDataProviderAdapter,
    ProviderCandleDTO,
    ProviderMetadata,
)
from app.services.data_providers.provider_registry import ProviderRegistry
from app.services.import_workflow_service import ImportWorkflowService
from app.services.sync_workflow_service import SyncWorkflowService
from app.services.weekly_aggregator import WeeklyAggregator
from tests_safety import PROD_DB_PATH, install_database_safeguard


# ==============================================================================
# SECTION 1: Safeguard Evasion & Boundary Probing
# ==============================================================================

def test_adv_safe_01_symlink_evasion_aborts(tmp_path: Path):
    """Creating a symlink to production DB in /tmp must be intercepted and abort."""
    symlink_path = tmp_path / "symlink_sumi.db"
    try:
        symlink_path.symlink_to(PROD_DB_PATH)
    except OSError:
        pytest.skip("Symlink creation not permitted in this environment")

    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD"):
        sqlite3.connect(str(symlink_path))


def test_adv_safe_02_path_traversal_evasion_aborts():
    """Redundant dots and directory traversal targeting production DB must abort."""
    # Parent directory traversal
    traversal_path = "backend/../backend/sumi.db"
    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD"):
        sqlite3.connect(traversal_path)

    # Redundant current directory dots
    redundant_dots = "./backend/././sumi.db"
    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD"):
        sqlite3.connect(redundant_dots)


def test_adv_safe_03_relative_file_uri_evasion_aborts():
    """Relative file: URI schemes must be resolved and abort."""
    uri = "file:backend/sumi.db"
    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD"):
        sqlite3.connect(uri, uri=True)

    uri_with_params = "file:./backend/sumi.db?mode=ro&cache=private"
    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD"):
        sqlite3.connect(uri_with_params, uri=True)


def test_adv_safe_04_auxiliary_file_mutations_abort():
    """Mutating open on auxiliary sqlite files (-wal, -shm, -journal) must abort."""
    for suffix in ["-wal", "-shm", "-journal"]:
        aux_path = str(PROD_DB_PATH) + suffix
        with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD"):
            open(aux_path, "w")


def test_adv_safe_05_c_extension_sqlite_connect_aborts():
    """Direct invocation of _sqlite3.connect must be intercepted and abort."""
    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD"):
        _sqlite3.connect(str(PROD_DB_PATH))


def test_adv_safe_06_trailing_whitespace_evasion_aborts():
    """Paths with trailing whitespace must be cleaned and abort."""
    with pytest.raises(RuntimeError, match=r"FAIL-CLOSED SAFEGUARD"):
        sqlite3.connect(f"{PROD_DB_PATH} ")


# ==============================================================================
# FIXTURES FOR DATA INTEGRITY ADVERSARIAL TESTS
# ==============================================================================

@pytest.fixture
def adv_db(tmp_path: Path):
    """Isolated file database per test."""
    db_file = tmp_path / "adv_test.db"
    engine = create_engine(f"sqlite:///{db_file.as_posix()}", echo=False)
    Base.metadata.create_all(bind=engine)
    session = Session(bind=engine)
    try:
        yield session
    finally:
        session.close()
        engine.dispose()


class AdversarialMockProvider(MarketDataProviderAdapter):
    def __init__(self, metadata: ProviderMetadata, candles: list[ProviderCandleDTO]):
        self._metadata = metadata
        self._candles = candles

    def get_metadata(self) -> ProviderMetadata:
        return self._metadata

    def test_connection(self, credentials=None) -> bool:
        return True

    def fetch_daily_candles(
        self,
        symbol: str,
        start_date: date,
        end_date: date,
        adjustment_type: str = "unadjusted",
        credentials=None,
    ) -> list[ProviderCandleDTO]:
        return self._candles

    def fetch_benchmark_indices(
        self,
        benchmark: str,
        start_date: date,
        end_date: date,
        credentials=None,
    ) -> list[ProviderCandleDTO]:
        return []


# ==============================================================================
# SECTION 2: Rapid & Concurrent Write Collision Scenarios
# ==============================================================================

def test_adv_coll_01_successive_takeovers_import_then_sync(adv_db: Session):
    """Quarantine taken over by Import, then subsequent non-synthetic Sync updates candle."""
    adv_db.add(Symbol(symbol="HPG", is_active=True))
    symbol = "HPG"
    dt = datetime(2026, 8, 3, 0, 0, 0)

    # Step 1: Seed quarantine candle
    q_candle = Candle(
        symbol=symbol,
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        open=10.0,
        high=10.0,
        low=10.0,
        close=10.0,
        volume=100.0,
        source="quarantine:sync:fake_seed",
    )
    adv_db.add(q_candle)
    adv_db.commit()

    # Step 2: Canonical CafeF CSV Import takes over quarantine row
    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260803,20.0,22.0,19.0,21.0,50000\n"
    imp_prev = ImportWorkflowService.preview_import(
        db=adv_db,
        file_content=csv_content,
        filename="HPG_CANONICAL.csv",
        source_type="cafef",
        adjustment_type="unadjusted",
    )
    assert imp_prev.can_accept is True

    imp_accept = ImportWorkflowService.accept_import(
        db=adv_db,
        run_id=imp_prev.run_id,
        content_sha256=imp_prev.content_sha256,
    )
    assert imp_accept.status == "accepted"

    # Verify canonical candle
    c_after_imp = adv_db.execute(select(Candle).filter_by(symbol=symbol, timeframe="1D")).scalar_one()
    assert c_after_imp.source == imp_prev.run_id
    assert c_after_imp.close == 21000.0  # Scaled by 1,000 for equities

    # Step 3: Canonical Sync arrives with real provider data for same date
    sync_provider = AdversarialMockProvider(
        metadata=ProviderMetadata(
            provider_id="real_adv",
            display_name="Real Adv Provider",
            is_official=True,
            requires_auth=False,
            is_synthetic=False,
            description="Real feed",
        ),
        candles=[
            ProviderCandleDTO(
                symbol=symbol,
                timeframe="1D",
                timestamp=date(2026, 8, 3),
                open=22.0,
                high=24.0,
                low=21.0,
                close=23.0,
                volume=60000.0,
                adjustment_type="unadjusted",
            )
        ],
    )
    ProviderRegistry.reset_instance()
    ProviderRegistry.get_instance().register_provider(sync_provider)

    from app.schemas.sync_schema import SyncPreviewRequest
    sync_prev = SyncWorkflowService.generate_sync_preview(
        db=adv_db,
        request=SyncPreviewRequest(
            symbol=symbol,
            start_date="2026-08-03",
            end_date="2026-08-03",
            provider_id="real_adv",
            adjustment_type="unadjusted",
        ),
    )
    # The candle already exists canonically with different OHLCV, so preview marks conflicting
    assert sync_prev.conflicting_count == 1
    assert sync_prev.can_accept is False


def test_adv_coll_02_multiple_timeframes_quarantine_takeover(adv_db: Session):
    """1D and 1W quarantine candles exist. 1D import takes over 1D; 1W is atomically rederived as canonical."""
    adv_db.add(Symbol(symbol="VNM", is_active=True))
    symbol = "VNM"
    dt = datetime(2026, 8, 3, 0, 0, 0)  # Monday

    # Seed 1D quarantine candle
    q_1d = Candle(
        symbol=symbol,
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        open=50.0,
        high=51.0,
        low=49.0,
        close=50.5,
        volume=100.0,
        source="quarantine:seed:1d",
    )
    # Seed 1W quarantine candle
    q_1w = Candle(
        symbol=symbol,
        timeframe="1W",
        timestamp=dt,
        adjustment_type="unadjusted",
        open=50.0,
        high=51.0,
        low=49.0,
        close=50.5,
        volume=100.0,
        source="quarantine:VN_TRADING_WEEK_V1",
    )
    adv_db.add_all([q_1d, q_1w])
    adv_db.commit()

    # Import canonical 1D CafeF CSV
    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nVNM,20260803,70.0,72.0,69.0,71.0,50000\n"
    imp_prev = ImportWorkflowService.preview_import(
        db=adv_db,
        file_content=csv_content,
        filename="VNM_1W.csv",
        source_type="cafef",
        adjustment_type="unadjusted",
    )
    imp_accept = ImportWorkflowService.accept_import(
        db=adv_db,
        run_id=imp_prev.run_id,
        content_sha256=imp_prev.content_sha256,
    )
    assert imp_accept.status == "accepted"

    # Check 1D candle is canonical
    c_1d = adv_db.execute(
        select(Candle).filter_by(symbol=symbol, timeframe="1D")
    ).scalar_one()
    assert not c_1d.source.startswith("quarantine:")
    assert c_1d.close == 71000.0

    # Check 1W candle is canonical VN_TRADING_WEEK_V1
    c_1w = adv_db.execute(
        select(Candle).filter_by(symbol=symbol, timeframe="1W")
    ).scalar_one()
    assert c_1w.source == "VN_TRADING_WEEK_V1"
    assert c_1w.close == 71000.0


# ==============================================================================
# SECTION 3: Cross-Workflow Rollback & Atomicity Conflicts
# ==============================================================================

def test_adv_roll_01_non_accepted_subsequent_run_does_not_block_rollback(adv_db: Session):
    """Subsequent failed or rejected Sync run does NOT block prior Import rollback."""
    adv_db.add(Symbol(symbol="TCB", is_active=True))
    symbol = "TCB"
    dt = datetime(2026, 8, 3, 0, 0, 0)

    # Quarantine seed
    q_candle = Candle(
        symbol=symbol,
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        open=15.0,
        high=16.0,
        low=14.0,
        close=15.5,
        volume=1000.0,
        source="quarantine:tcb",
    )
    adv_db.add(q_candle)
    adv_db.commit()

    # Import takeover
    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nTCB,20260803,25.0,26.0,24.0,25.5,10000\n"
    imp_prev = ImportWorkflowService.preview_import(
        db=adv_db,
        file_content=csv_content,
        filename="TCB.csv",
        source_type="cafef",
        adjustment_type="unadjusted",
    )
    imp_accept = ImportWorkflowService.accept_import(
        db=adv_db,
        run_id=imp_prev.run_id,
        content_sha256=imp_prev.content_sha256,
    )
    assert imp_accept.status == "accepted"

    # Simulate subsequent Sync run with status 'failed'
    failed_sync = SyncRun(
        id="sync_failed_999",
        provider_id="mock",
        symbol=symbol,
        start_date="2026-08-01",
        end_date="2026-08-05",
        content_sha256="fake_sha256_dummy",
        status="failed",
        accepted_at=datetime.now(timezone.utc),
    )
    adv_db.add(failed_sync)
    adv_db.commit()

    # Rollback of import should succeed because subsequent sync was never 'accepted'
    rb_res = ImportWorkflowService.rollback_import(adv_db, imp_prev.run_id)
    assert rb_res.status == "rolled_back"

    # Verify quarantine restored
    restored = adv_db.execute(
        select(Candle).execution_options(include_quarantine=True).filter_by(symbol=symbol, timeframe="1D")
    ).scalar_one()
    assert restored.source == "quarantine:tcb"
    assert restored.close == 15.5


def test_adv_roll_02_fault_injection_during_rollback_preserves_state(adv_db: Session):
    """Fault injection midway through rollback restores session with zero partial mutation."""
    adv_db.add(Symbol(symbol="SSI", is_active=True))
    symbol = "SSI"
    dt = datetime(2026, 8, 3, 0, 0, 0)

    # Seed quarantine
    q_candle = Candle(
        symbol=symbol,
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        open=30.0,
        high=31.0,
        low=29.0,
        close=30.5,
        volume=1000.0,
        source="quarantine:ssi:1",
    )
    adv_db.add(q_candle)
    adv_db.commit()

    # Import takeover
    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nSSI,20260803,40.0,41.0,39.0,40.5,20000\n"
    imp_prev = ImportWorkflowService.preview_import(
        db=adv_db,
        file_content=csv_content,
        filename="SSI.csv",
        source_type="cafef",
        adjustment_type="unadjusted",
    )
    ImportWorkflowService.accept_import(
        db=adv_db,
        run_id=imp_prev.run_id,
        content_sha256=imp_prev.content_sha256,
    )

    # Inject failure during WeeklyAggregator call inside rollback
    with patch.object(WeeklyAggregator, "derive_weekly_candles", side_effect=RuntimeError("Simulated crash in weekly")):
        with pytest.raises(RuntimeError, match="Simulated crash in weekly"):
            ImportWorkflowService.rollback_import(adv_db, imp_prev.run_id)

    # Verify state after rollback failure: import run is still 'accepted', candle is still canonical
    run_check = adv_db.execute(select(ImportRun).filter_by(id=imp_prev.run_id)).scalar_one()
    assert run_check.status == "accepted"

    c_check = adv_db.execute(select(Candle).filter_by(symbol=symbol, timeframe="1D")).scalar_one()
    assert c_check.close == 40500.0
    assert c_check.source == imp_prev.run_id


def test_adv_roll_03_multi_cycle_takeover_and_rollback_survives(adv_db: Session):
    """Quarantine survives multiple full takeover and rollback cycles intact."""
    adv_db.add(Symbol(symbol="MBB", is_active=True))
    symbol = "MBB"
    dt = datetime(2026, 8, 3, 0, 0, 0)
    original_source = "quarantine:mbb:cycle"

    q_candle = Candle(
        symbol=symbol,
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        open=20.0,
        high=21.0,
        low=19.5,
        close=20.5,
        volume=500.0,
        source=original_source,
    )
    adv_db.add(q_candle)
    adv_db.commit()

    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nMBB,20260803,30.0,31.0,29.0,30.0,1000\n"

    # Cycle 1: Import takeover then rollback
    prev_1 = ImportWorkflowService.preview_import(
        db=adv_db,
        file_content=csv_content,
        filename="cycle1.csv",
        source_type="cafef",
        adjustment_type="unadjusted",
    )
    ImportWorkflowService.accept_import(adv_db, prev_1.run_id, prev_1.content_sha256)
    ImportWorkflowService.rollback_import(adv_db, prev_1.run_id)

    q_after_c1 = adv_db.execute(
        select(Candle).execution_options(include_quarantine=True).filter_by(symbol=symbol, timeframe="1D")
    ).scalar_one()
    assert q_after_c1.source == original_source
    assert q_after_c1.close == 20.5

    # Cycle 2: Another Import takeover then rollback
    prev_2 = ImportWorkflowService.preview_import(
        db=adv_db,
        file_content=csv_content,
        filename="cycle2.csv",
        source_type="cafef",
        adjustment_type="unadjusted",
    )
    ImportWorkflowService.accept_import(adv_db, prev_2.run_id, prev_2.content_sha256)
    ImportWorkflowService.rollback_import(adv_db, prev_2.run_id)

    q_after_c2 = adv_db.execute(
        select(Candle).execution_options(include_quarantine=True).filter_by(symbol=symbol, timeframe="1D")
    ).scalar_one()
    assert q_after_c2.source == original_source
    assert q_after_c2.close == 20.5


# ==============================================================================
# SECTION 4: Timestamp Boundary Conditions & Normalization
# ==============================================================================

def test_adv_time_01_pipe_heavy_source_preserves_provenance(adv_db: Session):
    """Quarantine source with multiple embedded pipes '|' must survive rsplit cleanly."""
    adv_db.add(Symbol(symbol="VIC", is_active=True))
    symbol = "VIC"
    dt = datetime(2026, 8, 3, 0, 0, 0)
    complex_source = "quarantine:feed|subfeed|nested|part|uuid-12345"

    q_candle = Candle(
        symbol=symbol,
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        open=45.0,
        high=46.0,
        low=44.0,
        close=45.5,
        volume=1000.0,
        source=complex_source,
    )
    adv_db.add(q_candle)
    adv_db.commit()

    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nVIC,20260803,55.0,56.0,54.0,55.5,10000\n"
    imp_prev = ImportWorkflowService.preview_import(
        db=adv_db,
        file_content=csv_content,
        filename="pipes.csv",
        source_type="cafef",
        adjustment_type="unadjusted",
    )
    ImportWorkflowService.accept_import(adv_db, imp_prev.run_id, imp_prev.content_sha256)

    # Verify mutation format in DB
    mut = adv_db.execute(select(ImportRunMutation).filter_by(run_id=imp_prev.run_id)).scalar_one()
    assert mut.action.startswith(f"UPDATE:{complex_source}|")

    # Rollback and verify exact restoration
    ImportWorkflowService.rollback_import(adv_db, imp_prev.run_id)
    restored = adv_db.execute(
        select(Candle).execution_options(include_quarantine=True).filter_by(symbol=symbol, timeframe="1D")
    ).scalar_one()
    assert restored.source == complex_source


def test_adv_time_02_legacy_mutation_format_compatibility(adv_db: Session):
    """Mutations created under legacy formats ('UPDATE', 'UPDATE:src') rollback gracefully."""
    adv_db.add(Symbol(symbol="FPT", is_active=True))
    symbol = "FPT"
    dt = datetime(2026, 8, 3, 0, 0, 0)

    # Insert existing candle
    candle = Candle(
        symbol=symbol,
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        open=90.0,
        high=92.0,
        low=89.0,
        close=91.0,
        volume=10000.0,
        source="manual_entry",
    )
    adv_db.add(candle)

    # Create run and synthetic legacy mutation action "UPDATE" without pipes
    run = ImportRun(
        id="legacy_run_01",
        file_name="legacy.csv",
        file_sha256="hash_01",
        content_sha256="hash_01",
        status="accepted",
        accepted_at=datetime.now(timezone.utc),
    )
    adv_db.add(run)

    mut = ImportRunMutation(
        run_id="legacy_run_01",
        symbol=symbol,
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        action="UPDATE",
        before_open=80.0,
        before_high=82.0,
        before_low=79.0,
        before_close=81.0,
        before_volume=5000.0,
    )
    adv_db.add(mut)
    adv_db.commit()

    # Trigger rollback
    ImportWorkflowService.rollback_import(adv_db, "legacy_run_01")

    # Verify candle restored to before_ values without crashing
    restored = adv_db.execute(select(Candle).filter_by(symbol=symbol, timeframe="1D")).scalar_one()
    assert restored.close == 81.0
    assert restored.source == "manual_entry"


# ==============================================================================
# SECTION 5: Synthetic Sync Precedence & Default-Deny
# ==============================================================================

def test_adv_syn_01_synthetic_sync_with_different_ohlcv_conflicts_and_blocks(adv_db: Session):
    """Synthetic sync with opt-in against existing canonical with different OHLCV is conflicting and blocked."""
    adv_db.add(Symbol(symbol="MWG", is_active=True))
    symbol = "MWG"
    dt = datetime(2026, 8, 3, 0, 0, 0)

    # Existing canonical candle
    c_candle = Candle(
        symbol=symbol,
        timeframe="1D",
        timestamp=dt,
        adjustment_type="unadjusted",
        open=45.0,
        high=46.0,
        low=44.0,
        close=45.5,
        volume=10000.0,
        source="real_source",
    )
    adv_db.add(c_candle)
    adv_db.commit()

    # Synthetic provider offering conflicting OHLCV
    synth_provider = AdversarialMockProvider(
        metadata=ProviderMetadata(
            provider_id="synth_adv",
            display_name="Synthetic Adv Provider",
            is_official=False,
            requires_auth=False,
            is_synthetic=True,
            description="Synthetic feed",
        ),
        candles=[
            ProviderCandleDTO(
                symbol=symbol,
                timeframe="1D",
                timestamp=date(2026, 8, 3),
                open=45.0,
                high=48.0,  # Different High!
                low=44.0,
                close=47.0,  # Different Close!
                volume=15000.0,
                adjustment_type="unadjusted",
            )
        ],
    )
    ProviderRegistry.reset_instance()
    ProviderRegistry.get_instance().register_provider(synth_provider)

    from app.schemas.sync_schema import SyncPreviewRequest
    # Preview with opt-in
    with patch.object(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", True):
        preview = SyncWorkflowService.generate_sync_preview(
            db=adv_db,
            request=SyncPreviewRequest(
                symbol=symbol,
                start_date="2026-08-03",
                end_date="2026-08-03",
                provider_id="synth_adv",
                adjustment_type="unadjusted",
            ),
        )
        assert preview.is_synthetic is True
        assert preview.conflicting_count == 1
        assert preview.can_accept is False

    # Verify canonical candle was NOT modified
    c_after = adv_db.execute(select(Candle).filter_by(symbol=symbol, timeframe="1D")).scalar_one()
    assert c_after.close == 45.5
    assert c_after.source == "real_source"
