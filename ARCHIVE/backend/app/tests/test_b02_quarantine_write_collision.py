import pytest
from datetime import date, datetime, timezone
from unittest.mock import patch
from fastapi import HTTPException
from sqlalchemy.orm import Session

from app.models.candle import Candle
from app.models.symbol import Symbol
from app.models.import_run import ImportRun, ImportRunMutation
from app.models.sync_run import SyncRun, SyncRunMutation
from app.services.import_workflow_service import ImportWorkflowService
from app.services.sync_workflow_service import SyncWorkflowService
from app.services.weekly_aggregator import WeeklyAggregator
from app.services.data_providers.provider_registry import ProviderRegistry
from app.services.data_providers.base_provider import (
    MarketDataProviderAdapter,
    ProviderCandleDTO,
    ProviderMetadata,
)
from app.schemas.sync_schema import SyncPreviewRequest
from app.config import settings


@pytest.fixture(autouse=True)
def clean_registry():
    yield
    ProviderRegistry.reset_instance()


class RealProviderStub(MarketDataProviderAdapter):
    def __init__(self, provider_id: str = "real_feed", candle_date: date = date(2026, 8, 3)):
        self._provider_id = provider_id
        self._candle_date = candle_date
        self._metadata = ProviderMetadata(
            provider_id=provider_id,
            display_name="Real Production Market Feed",
            is_official=True,
            requires_auth=False,
            supported_timeframes=["1D"],
            supported_adjustments=["unadjusted"],
            rate_limit_rps=10,
            description="Production Feed",
            auth_fields=[],
            is_synthetic=False,
        )

    def get_metadata(self) -> ProviderMetadata:
        return self._metadata

    def test_connection(self, credentials=None) -> bool:
        return True

    def fetch_daily_candles(self, symbol: str, start_date: date, end_date: date, adjustment_type="unadjusted", credentials=None):
        return [
            ProviderCandleDTO(
                symbol=symbol,
                timeframe="1D",
                timestamp=self._candle_date,
                open=28.0,
                high=29.0,
                low=27.5,
                close=28.5,
                volume=50000.0,
                adjustment_type=adjustment_type,
            )
        ]

    def fetch_benchmark_indices(self, benchmark: str, start_date: date, end_date: date, credentials=None):
        return []


# ==============================================================================
# Q-WRITE-01: Canonical Import supersedes quarantine
# ==============================================================================
def test_q_write_01_canonical_import_supersedes_quarantine(db_session: Session):
    # Seed symbol and quarantine candle
    db_session.add(Symbol(symbol="HPG", is_active=True))
    quar_candle = Candle(
        symbol="HPG",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0),
        open=10.0,
        high=10.0,
        low=10.0,
        close=10.0,
        volume=100.0,
        source="quarantine:sync:fake_sync_1",
        adjustment_type="unadjusted"
    )
    db_session.add(quar_candle)
    db_session.commit()
    quar_id = quar_candle.id

    # Canonical CafeF CSV for HPG at exact same key
    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260803,28.0,29.0,27.5,28.5,50000\n"
    preview = ImportWorkflowService.preview_import(
        db=db_session,
        file_content=csv_content,
        filename="HPG_CANONICAL.csv",
        adjustment_type="unadjusted"
    )

    assert preview.can_accept is True
    assert preview.parsed_count == 1
    assert preview.duplicate_count == 0
    assert preview.conflicting_count == 0

    # Accept import must succeed atomically without UNIQUE constraint violation
    accept_resp = ImportWorkflowService.accept_import(
        db=db_session,
        run_id=preview.run_id,
        content_sha256=preview.content_sha256
    )
    assert accept_resp.status == "accepted"
    assert accept_resp.accepted_count == 1

    # Canonical reader returns exactly one canonical candle for this date
    canonical_candles = (
        db_session.query(Candle)
        .filter(Candle.symbol == "HPG", Candle.timeframe == "1D")
        .all()
    )
    assert len(canonical_candles) == 1
    c = canonical_candles[0]
    assert c.open == 28000.0
    assert c.high == 29000.0
    assert c.low == 27500.0
    assert c.close == 28500.0
    assert c.volume == 50000.0
    assert c.source == preview.run_id
    assert not c.source.startswith("quarantine:")

    # Mutation audit check
    mutations = (
        db_session.query(ImportRunMutation)
        .filter(ImportRunMutation.run_id == preview.run_id)
        .all()
    )
    assert len(mutations) == 1
    m = mutations[0]
    assert m.action.startswith("UPDATE")
    assert m.before_close == 10.0
    assert m.after_close == 28500.0


# ==============================================================================
# Q-WRITE-02: Canonical Sync supersedes quarantine
# ==============================================================================
def test_q_write_02_canonical_sync_supersedes_quarantine(client, db_session: Session, monkeypatch):
    monkeypatch.setattr(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", False)

    db_session.add(Symbol(symbol="VNM", is_active=True))
    quar_candle = Candle(
        symbol="VNM",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0),
        open=999.0,
        high=999.0,
        low=999.0,
        close=999.0,
        volume=999.0,
        source="quarantine:sync:fake_sync_vnm",
        adjustment_type="unadjusted"
    )
    db_session.add(quar_candle)
    db_session.commit()

    registry = ProviderRegistry.get_instance()
    real_provider = RealProviderStub(provider_id="real_vnm_feed", candle_date=date(2026, 8, 3))
    registry.register_provider(real_provider)

    # 1. Preview canonical sync
    prev_resp = client.post("/api/sync/preview", json={
        "symbol": "VNM",
        "start_date": "2026-08-03",
        "end_date": "2026-08-03",
        "provider_id": "real_vnm_feed",
        "adjustment_type": "unadjusted"
    })
    assert prev_resp.status_code == 200
    prev_data = prev_resp.json()
    assert prev_data["can_accept"] is True
    assert prev_data["parsed_count"] == 1
    assert prev_data["duplicate_count"] == 0
    assert prev_data["conflicting_count"] == 0

    sync_id = prev_data["sync_id"]
    checksum = prev_data["content_sha256"]

    # 2. Execute canonical sync
    exec_resp = client.post("/api/sync/execute", json={
        "sync_id": sync_id,
        "content_sha256": checksum
    })
    assert exec_resp.status_code == 200
    exec_data = exec_resp.json()
    assert exec_data["status"] == "accepted"
    assert exec_data["accepted_count"] == 1

    # Canonical reader returns the real candle
    canonical_candles = (
        db_session.query(Candle)
        .filter(Candle.symbol == "VNM", Candle.timeframe == "1D")
        .all()
    )
    assert len(canonical_candles) == 1
    c = canonical_candles[0]
    assert c.close == 28.5
    assert c.source == f"sync:{sync_id}"

    # Mutation audit check
    mutations = (
        db_session.query(SyncRunMutation)
        .filter(SyncRunMutation.sync_id == sync_id)
        .all()
    )
    assert len(mutations) == 1
    m = mutations[0]
    assert m.action.startswith("UPDATE")
    assert m.before_close == 999.0
    assert m.after_close == 28.5


# ==============================================================================
# Q-WRITE-03: Quarantine must never overwrite canonical
# ==============================================================================
def test_q_write_03_quarantine_never_overwrites_canonical(client, db_session: Session, monkeypatch):
    monkeypatch.setattr(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", True)

    db_session.add(Symbol(symbol="FPT", is_active=True))
    canon_candle = Candle(
        symbol="FPT",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0),
        open=100.0,
        high=102.0,
        low=99.0,
        close=101.0,
        volume=10000.0,
        source="import:run_authoritative",
        adjustment_type="unadjusted"
    )
    db_session.add(canon_candle)
    db_session.commit()

    # Attempt synthetic sync for FPT on exact same date
    prev_resp = client.post("/api/sync/preview", json={
        "symbol": "FPT",
        "start_date": "2026-08-03",
        "end_date": "2026-08-03",
        "provider_id": "ssi",
        "adjustment_type": "unadjusted"
    })
    assert prev_resp.status_code == 200
    prev_data = prev_resp.json()

    # SSI stub returns values differing from canonical -> conflicting, can_accept=False
    assert prev_data["can_accept"] is False
    assert prev_data["conflicting_count"] == 1

    # Attempt to execute must fail
    exec_resp = client.post("/api/sync/execute", json={
        "sync_id": prev_data["sync_id"],
        "content_sha256": prev_data["content_sha256"]
    })
    assert exec_resp.status_code == 400

    # Canonical candle remains completely authoritative and unchanged
    c = db_session.query(Candle).filter(Candle.symbol == "FPT", Candle.timeframe == "1D").one()
    assert c.close == 101.0
    assert c.source == "import:run_authoritative"


# ==============================================================================
# Q-WRITE-04: Import rollback
# ==============================================================================
def test_q_write_04_import_rollback_preserves_quarantine(db_session: Session):
    db_session.add(Symbol(symbol="HPG", is_active=True))
    quar_candle = Candle(
        symbol="HPG",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0),
        open=10.0,
        high=10.0,
        low=10.0,
        close=10.0,
        volume=100.0,
        source="quarantine:sync:fake_test_04",
        adjustment_type="unadjusted"
    )
    db_session.add(quar_candle)
    db_session.commit()
    quar_id = quar_candle.id

    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260803,28.0,29.0,27.5,28.5,50000\n"
    preview = ImportWorkflowService.preview_import(
        db=db_session,
        file_content=csv_content,
        filename="HPG_CANONICAL.csv",
        adjustment_type="unadjusted"
    )

    # Inject failure during WeeklyAggregator (after candle staging, before commit)
    with patch("app.services.weekly_aggregator.WeeklyAggregator.derive_weekly_candles", side_effect=RuntimeError("Simulated DB Crash")):
        with pytest.raises(RuntimeError):
            ImportWorkflowService.accept_import(
                db=db_session,
                run_id=preview.run_id,
                content_sha256=preview.content_sha256
            )

    # After rollback, quarantine candle must still exist with original values and provenance
    q_reloaded = (
        db_session.query(Candle)
        .execution_options(include_quarantine=True)
        .filter(Candle.symbol == "HPG", Candle.timeframe == "1D")
        .one()
    )
    assert q_reloaded.id == quar_id
    assert q_reloaded.close == 10.0
    assert q_reloaded.source == "quarantine:sync:fake_test_04"

    # Canonical reader sees 0
    assert len(db_session.query(Candle).filter(Candle.symbol == "HPG", Candle.timeframe == "1D").all()) == 0


# ==============================================================================
# Q-WRITE-05: Sync rollback
# ==============================================================================
def test_q_write_05_sync_rollback_preserves_quarantine(db_session: Session, monkeypatch):
    monkeypatch.setattr(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", False)

    db_session.add(Symbol(symbol="VNM", is_active=True))
    quar_candle = Candle(
        symbol="VNM",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0),
        open=999.0,
        high=999.0,
        low=999.0,
        close=999.0,
        volume=999.0,
        source="quarantine:sync:fake_test_05",
        adjustment_type="unadjusted"
    )
    db_session.add(quar_candle)
    db_session.commit()
    quar_id = quar_candle.id

    registry = ProviderRegistry.get_instance()
    real_provider = RealProviderStub(provider_id="real_vnm_feed_05", candle_date=date(2026, 8, 3))
    registry.register_provider(real_provider)

    preview = SyncWorkflowService.generate_sync_preview(
        db=db_session,
        request=SyncPreviewRequest(
            provider_id="real_vnm_feed_05",
            symbol="VNM",
            start_date="2026-08-03",
            end_date="2026-08-03",
            adjustment_type="unadjusted"
        )
    )

    # Inject failure before commit
    with patch("app.services.weekly_aggregator.WeeklyAggregator.derive_weekly_candles", side_effect=RuntimeError("Simulated Sync Crash")):
        with pytest.raises(RuntimeError):
            SyncWorkflowService.execute_sync(
                db=db_session,
                sync_id=preview.sync_id,
                content_sha256=preview.content_sha256
            )

    # Quarantine candle intact
    q_reloaded = (
        db_session.query(Candle)
        .execution_options(include_quarantine=True)
        .filter(Candle.symbol == "VNM", Candle.timeframe == "1D")
        .one()
    )
    assert q_reloaded.id == quar_id
    assert q_reloaded.close == 999.0
    assert q_reloaded.source == "quarantine:sync:fake_test_05"

    # Canonical reader sees 0
    assert len(db_session.query(Candle).filter(Candle.symbol == "VNM", Candle.timeframe == "1D").all()) == 0


# ==============================================================================
# Q-WRITE-06: Weekly derivation after canonical takeover
# ==============================================================================
def test_q_write_06_weekly_derivation_after_canonical_takeover(db_session: Session):
    db_session.add(Symbol(symbol="HPG", is_active=True))
    # Mon: quarantine candle
    db_session.add(Candle(
        symbol="HPG",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0),
        open=999.0,
        high=999.0,
        low=999.0,
        close=999.0,
        volume=999.0,
        source="quarantine:sync:fake_q6",
        adjustment_type="unadjusted"
    ))
    db_session.commit()

    # Pre-derive weekly: only quarantine exists, so quarantine 1W candle is derived
    WeeklyAggregator.derive_weekly_candles(db_session, {"HPG"}, {"unadjusted"})
    assert len(db_session.query(Candle).filter(Candle.symbol == "HPG", Candle.timeframe == "1W").all()) == 0
    q_1w = (
        db_session.query(Candle)
        .execution_options(include_quarantine=True)
        .filter(Candle.symbol == "HPG", Candle.timeframe == "1W")
        .all()
    )
    assert len(q_1w) == 1
    assert q_1w[0].high == 999.0
    assert q_1w[0].source == "quarantine:VN_TRADING_WEEK_V1"

    # Now canonical import supersedes the quarantine daily candle
    csv_content = (
        b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        b"HPG,20260803,28.0,29.0,27.5,28.5,50000\n"
        b"HPG,20260804,28.5,30.0,28.0,29.5,60000\n"
    )
    preview = ImportWorkflowService.preview_import(
        db=db_session,
        file_content=csv_content,
        filename="HPG_WEEKLY_TAKEOVER.csv",
        adjustment_type="unadjusted"
    )
    ImportWorkflowService.accept_import(
        db=db_session,
        run_id=preview.run_id,
        content_sha256=preview.content_sha256
    )

    # Canonical weekly reader must return exactly 1 canonical weekly bar
    w_candles = (
        db_session.query(Candle)
        .filter(Candle.symbol == "HPG", Candle.timeframe == "1W")
        .all()
    )
    assert len(w_candles) == 1
    w = w_candles[0]
    assert w.source == "VN_TRADING_WEEK_V1"
    assert w.open == 28000.0
    assert w.high == 30000.0
    assert w.low == 27500.0
    assert w.close == 29500.0
    assert w.volume == 110000.0


# ==============================================================================
# Adversarial Cases A, B, C, D, E (Section 8)
# ==============================================================================
def test_case_a_no_quarantine_exists(db_session: Session):
    """Case A: No quarantine exists. Normal import behaves exactly as before."""
    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nAAA,20260803,15.0,16.0,14.5,15.5,20000\n"
    preview = ImportWorkflowService.preview_import(
        db=db_session,
        file_content=csv_content,
        filename="AAA.csv",
        adjustment_type="unadjusted"
    )
    assert preview.can_accept is True
    assert preview.parsed_count == 1
    accept_resp = ImportWorkflowService.accept_import(
        db=db_session,
        run_id=preview.run_id,
        content_sha256=preview.content_sha256
    )
    assert accept_resp.status == "accepted"
    c = db_session.query(Candle).filter(Candle.symbol == "AAA", Candle.timeframe == "1D").one()
    assert c.close == 15500.0


def test_case_b_quarantine_at_unrelated_dates(db_session: Session):
    """Case B: Quarantine exists at unrelated dates. Canonical operation unaffected."""
    db_session.add(Symbol(symbol="HPG", is_active=True))
    db_session.add(Candle(
        symbol="HPG",
        timeframe="1D",
        timestamp=datetime(2026, 8, 10, 0, 0),
        open=99000.0, high=99000.0, low=99000.0, close=99000.0, volume=99.0,
        source="quarantine:sync:future",
        adjustment_type="unadjusted"
    ))
    db_session.commit()

    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260803,28.0,29.0,27.5,28.5,50000\n"
    preview = ImportWorkflowService.preview_import(
        db=db_session,
        file_content=csv_content,
        filename="HPG_UNRELATED.csv",
        adjustment_type="unadjusted"
    )
    assert preview.can_accept is True
    ImportWorkflowService.accept_import(
        db=db_session,
        run_id=preview.run_id,
        content_sha256=preview.content_sha256
    )

    # Canonical reader sees the canonical 2026-08-03 candle, but NOT the quarantine 2026-08-10 candle
    candles = db_session.query(Candle).filter(Candle.symbol == "HPG", Candle.timeframe == "1D").all()
    assert len(candles) == 1
    assert candles[0].timestamp.date() == date(2026, 8, 3)


def test_case_d_canonical_already_exists_duplicate_and_conflict(db_session: Session):
    """Case D: Canonical already exists. Normal duplicate/conflict behavior remains unchanged."""
    db_session.add(Symbol(symbol="HPG", is_active=True))
    db_session.add(Candle(
        symbol="HPG",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0),
        open=28000.0, high=29000.0, low=27500.0, close=28500.0, volume=50000.0,
        source="import:run_prev",
        adjustment_type="unadjusted"
    ))
    db_session.commit()

    # Identical values -> duplicate (noop)
    csv_dup = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260803,28.0,29.0,27.5,28.5,50000\n"
    prev_dup = ImportWorkflowService.preview_import(
        db=db_session,
        file_content=csv_dup,
        filename="DUP.csv",
        adjustment_type="unadjusted"
    )
    assert prev_dup.duplicate_count == 1
    assert prev_dup.parsed_count == 0

    # Conflicting values -> conflict (blocked)
    csv_conf = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260803,30.0,31.0,29.5,30.5,50000\n"
    prev_conf = ImportWorkflowService.preview_import(
        db=db_session,
        file_content=csv_conf,
        filename="CONF.csv",
        adjustment_type="unadjusted"
    )
    assert prev_conf.conflicting_count == 1
    assert prev_conf.can_accept is False


def test_case_e_only_quarantine_exists_canonical_sees_empty(db_session: Session):
    """Case E: Only quarantine exists and no real data arrives. Canonical readers still see no canonical data."""
    db_session.add(Symbol(symbol="QUAR_ONLY", is_active=True))
    db_session.add(Candle(
        symbol="QUAR_ONLY",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0),
        open=50.0, high=55.0, low=49.0, close=52.0, volume=5000.0,
        source="quarantine:sync:only",
        adjustment_type="unadjusted"
    ))
    db_session.commit()

    # Canonical reader sees 0 candles
    assert len(db_session.query(Candle).filter(Candle.symbol == "QUAR_ONLY").all()) == 0

    # Authorized quarantine reader sees 1 candle
    q_candles = (
        db_session.query(Candle)
        .execution_options(include_quarantine=True)
        .filter(Candle.symbol == "QUAR_ONLY")
        .all()
    )
    assert len(q_candles) == 1


# ==============================================================================
# Post-Acceptance Workflow Rollback Tests (Audit & Restoration Verification)
# ==============================================================================
def test_q_write_07_post_acceptance_import_rollback_restores_quarantine(db_session: Session):
    """
    When an accepted canonical import that superseded a quarantine row is rolled back via
    rollback_import, the candle must be restored to its pre-operation quarantine state.
    """
    db_session.add(Symbol(symbol="HPG", is_active=True))
    quar_candle = Candle(
        symbol="HPG",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0),
        open=10.0, high=10.0, low=10.0, close=10.0, volume=100.0,
        source="quarantine:sync:prior_seed",
        adjustment_type="unadjusted"
    )
    db_session.add(quar_candle)
    db_session.commit()
    quar_id = quar_candle.id

    # Supersede via canonical import
    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260803,28.0,29.0,27.5,28.5,50000\n"
    preview = ImportWorkflowService.preview_import(
        db=db_session,
        file_content=csv_content,
        filename="HPG_CANONICAL.csv",
        adjustment_type="unadjusted"
    )
    ImportWorkflowService.accept_import(
        db=db_session,
        run_id=preview.run_id,
        content_sha256=preview.content_sha256
    )

    # Roll back the accepted import
    rb_resp = ImportWorkflowService.rollback_import(db=db_session, run_id=preview.run_id)
    assert rb_resp.status == "rolled_back"
    assert rb_resp.restored_mutations_count == 1

    # Canonical reader sees 0
    assert len(db_session.query(Candle).filter(Candle.symbol == "HPG", Candle.timeframe == "1D").all()) == 0

    # Quarantine reader sees the restored quarantine candle with original values and source
    c = (
        db_session.query(Candle)
        .execution_options(include_quarantine=True)
        .filter(Candle.symbol == "HPG", Candle.timeframe == "1D")
        .one()
    )
    assert c.id == quar_id
    assert c.close == 10.0
    assert c.source == "quarantine:sync:prior_seed"


def test_q_write_08_post_acceptance_sync_rollback_restores_quarantine(db_session: Session, monkeypatch):
    """
    When an accepted canonical sync that superseded a quarantine row is rolled back via
    rollback_sync, the candle must be restored to its pre-operation quarantine state.
    """
    monkeypatch.setattr(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", False)

    db_session.add(Symbol(symbol="VNM", is_active=True))
    quar_candle = Candle(
        symbol="VNM",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0),
        open=999.0, high=999.0, low=999.0, close=999.0, volume=999.0,
        source="quarantine:sync:prior_vnm_seed",
        adjustment_type="unadjusted"
    )
    db_session.add(quar_candle)
    db_session.commit()
    quar_id = quar_candle.id

    registry = ProviderRegistry.get_instance()
    real_provider = RealProviderStub(provider_id="real_feed_08", candle_date=date(2026, 8, 3))
    registry.register_provider(real_provider)

    preview = SyncWorkflowService.generate_sync_preview(
        db=db_session,
        request=SyncPreviewRequest(
            provider_id="real_feed_08",
            symbol="VNM",
            start_date="2026-08-03",
            end_date="2026-08-03",
            adjustment_type="unadjusted"
        )
    )
    exec_resp = SyncWorkflowService.execute_sync(
        db=db_session,
        sync_id=preview.sync_id,
        content_sha256=preview.content_sha256
    )
    assert exec_resp.status == "accepted"

    # Roll back the accepted sync
    rb_resp = SyncWorkflowService.rollback_sync(db=db_session, sync_id=preview.sync_id)
    assert rb_resp.status == "rolled_back"
    assert rb_resp.restored_mutations_count == 1

    # Canonical reader sees 0
    assert len(db_session.query(Candle).filter(Candle.symbol == "VNM", Candle.timeframe == "1D").all()) == 0

    # Quarantine reader sees the restored quarantine candle with original values and source
    c = (
        db_session.query(Candle)
        .execution_options(include_quarantine=True)
        .filter(Candle.symbol == "VNM", Candle.timeframe == "1D")
        .one()
    )
    assert c.id == quar_id
    assert c.close == 999.0
    assert c.source == "quarantine:sync:prior_vnm_seed"


# ==============================================================================
# Additional Adversarial & Edge Case Tests (Review Round 1)
# ==============================================================================
def test_q_write_09_multi_symbol_import_quarantine_and_canonical_conflict(db_session: Session):
    """
    Multi-symbol bulk CSV import where subset of symbols has quarantine placeholder
    and another subset has pre-existing canonical conflict. Preview must block,
    accept must reject, and quarantine placeholder must remain untouched.
    """
    db_session.add(Symbol(symbol="SYM_A", is_active=True))
    db_session.add(Candle(
        symbol="SYM_A",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0),
        open=10.0, high=10.0, low=10.0, close=10.0, volume=100.0,
        source="quarantine:sync:sym_a",
        adjustment_type="unadjusted"
    ))

    db_session.add(Symbol(symbol="SYM_B", is_active=True))
    db_session.add(Candle(
        symbol="SYM_B",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0),
        open=50.0, high=55.0, low=49.0, close=52.0, volume=1000.0,
        source="cafef_prev_run",
        adjustment_type="unadjusted"
    ))
    db_session.commit()

    csv_content = (
        b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        b"SYM_A,20260803,28.0,29.0,27.5,28.5,50000\n"
        b"SYM_B,20260803,50.0,100.0,49.0,99.0,1000\n"
    )

    preview = ImportWorkflowService.preview_import(
        db=db_session,
        file_content=csv_content,
        filename="MULTI_CONFLICT.csv",
        adjustment_type="unadjusted"
    )

    assert preview.can_accept is False
    assert preview.parsed_count == 1
    assert preview.conflicting_count == 1

    with pytest.raises(HTTPException) as exc_info:
        ImportWorkflowService.accept_import(
            db=db_session,
            run_id=preview.run_id,
            content_sha256=preview.content_sha256
        )
    assert exc_info.value.status_code == 400

    # SYM_A quarantine candle remains completely untouched
    sym_a = (
        db_session.query(Candle)
        .execution_options(include_quarantine=True)
        .filter(Candle.symbol == "SYM_A")
        .one()
    )
    assert sym_a.source == "quarantine:sync:sym_a"
    assert sym_a.close == 10.0
    assert len(db_session.query(Candle).filter(Candle.symbol == "SYM_A").all()) == 0


def test_q_write_10_multi_symbol_import_fault_injection_during_acceptance(db_session: Session):
    """
    Multi-symbol bulk CSV import where subset of symbols has quarantine placeholder
    and subset is new. A fault injected during acceptance rolls back the transaction
    atomically without leaving the quarantine candle partially promoted.
    """
    db_session.add(Symbol(symbol="SYM_A", is_active=True))
    db_session.add(Candle(
        symbol="SYM_A",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0),
        open=10.0, high=10.0, low=10.0, close=10.0, volume=100.0,
        source="quarantine:sync:sym_a",
        adjustment_type="unadjusted"
    ))
    db_session.commit()

    csv_content = (
        b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        b"SYM_A,20260803,28.0,29.0,27.5,28.5,50000\n"
        b"SYM_B,20260803,50.0,55.0,49.0,52.0,1000\n"
    )

    preview = ImportWorkflowService.preview_import(
        db=db_session,
        file_content=csv_content,
        filename="MULTI_PASS.csv",
        adjustment_type="unadjusted"
    )
    assert preview.can_accept is True
    assert preview.parsed_count == 2

    with patch.object(WeeklyAggregator, "derive_weekly_candles", side_effect=RuntimeError("Simulated Weekly Crash")):
        with pytest.raises(RuntimeError, match="Simulated Weekly Crash"):
            ImportWorkflowService.accept_import(
                db=db_session,
                run_id=preview.run_id,
                content_sha256=preview.content_sha256
            )

    sym_a = (
        db_session.query(Candle)
        .execution_options(include_quarantine=True)
        .filter(Candle.symbol == "SYM_A")
        .one()
    )
    assert sym_a.source == "quarantine:sync:sym_a"
    assert sym_a.close == 10.0
    assert db_session.query(Candle).execution_options(include_quarantine=True).filter(Candle.symbol == "SYM_B").count() == 0
    assert db_session.query(ImportRunMutation).filter(ImportRunMutation.run_id == preview.run_id).count() == 0


def test_q_write_11_multi_symbol_post_acceptance_rollback(db_session: Session):
    """
    Multi-symbol bulk import accepted then rolled back: restores quarantine candle
    and its quarantine weekly bar for SYM_A, deletes SYM_B.
    """
    db_session.add(Symbol(symbol="SYM_A", is_active=True))
    db_session.add(Symbol(symbol="SYM_B", is_active=True))
    db_session.add(Candle(
        symbol="SYM_A",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0),
        open=10.0, high=10.0, low=10.0, close=10.0, volume=100.0,
        source="quarantine:sync:sym_a",
        adjustment_type="unadjusted"
    ))
    db_session.commit()

    csv_content = (
        b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        b"SYM_A,20260803,28.0,29.0,27.5,28.5,50000\n"
        b"SYM_B,20260803,50.0,55.0,49.0,52.0,1000\n"
    )

    preview = ImportWorkflowService.preview_import(
        db=db_session,
        file_content=csv_content,
        filename="MULTI_PASS.csv",
        adjustment_type="unadjusted"
    )
    accept_resp = ImportWorkflowService.accept_import(
        db=db_session,
        run_id=preview.run_id,
        content_sha256=preview.content_sha256
    )
    assert accept_resp.status == "accepted"

    assert len(db_session.query(Candle).filter(Candle.symbol == "SYM_A").all()) == 2
    assert len(db_session.query(Candle).filter(Candle.symbol == "SYM_B").all()) == 2

    rb_resp = ImportWorkflowService.rollback_import(db=db_session, run_id=preview.run_id)
    assert rb_resp.status == "rolled_back"
    assert rb_resp.restored_mutations_count == 2

    assert len(db_session.query(Candle).filter(Candle.symbol == "SYM_A").all()) == 0
    assert len(db_session.query(Candle).filter(Candle.symbol == "SYM_B").all()) == 0

    sym_a_1d = (
        db_session.query(Candle)
        .execution_options(include_quarantine=True)
        .filter(Candle.symbol == "SYM_A", Candle.timeframe == "1D")
        .one()
    )
    assert sym_a_1d.source == "quarantine:sync:sym_a"
    assert sym_a_1d.close == 10.0

    sym_a_1w = (
        db_session.query(Candle)
        .execution_options(include_quarantine=True)
        .filter(Candle.symbol == "SYM_A", Candle.timeframe == "1W")
        .one()
    )
    assert sym_a_1w.source == "quarantine:VN_TRADING_WEEK_V1"
    assert db_session.query(Candle).execution_options(include_quarantine=True).filter(Candle.symbol == "SYM_B").count() == 0


def test_q_write_12_import_takeover_timestamp_mismatch_and_rollback(db_session: Session):
    """
    Quarantine candle seeded with non-midnight timestamp. Canonical import takes over,
    normalizes timestamp to canonical midnight, and rollback restores the exact
    pre-operation quarantine timestamp and values.
    """
    db_session.add(Symbol(symbol="HPG", is_active=True))
    quar = Candle(
        symbol="HPG",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 10, 0),
        open=10.0, high=10.0, low=10.0, close=10.0, volume=100.0,
        source="quarantine:sync:fake_1",
        adjustment_type="unadjusted"
    )
    db_session.add(quar)
    db_session.commit()

    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260803,28.0,29.0,27.5,28.5,50000\n"
    preview = ImportWorkflowService.preview_import(
        db=db_session,
        file_content=csv_content,
        filename="HPG.csv",
        adjustment_type="unadjusted"
    )
    accept_resp = ImportWorkflowService.accept_import(
        db=db_session,
        run_id=preview.run_id,
        content_sha256=preview.content_sha256
    )
    assert accept_resp.status == "accepted"

    # Promoted candle has normalized timestamp
    c_promoted = db_session.query(Candle).filter(Candle.symbol == "HPG", Candle.timeframe == "1D").one()
    assert c_promoted.timestamp == datetime(2026, 8, 3, 0, 0)
    assert c_promoted.close == 28500.0

    # Rollback restores exact quarantine state including original timestamp
    rb_resp = ImportWorkflowService.rollback_import(db=db_session, run_id=preview.run_id)
    assert rb_resp.status == "rolled_back"

    restored = (
        db_session.query(Candle)
        .execution_options(include_quarantine=True)
        .filter(Candle.symbol == "HPG", Candle.timeframe == "1D")
        .first()
    )
    assert restored is not None
    assert restored.source == "quarantine:sync:fake_1"
    assert restored.close == 10.0
    assert restored.timestamp == datetime(2026, 8, 3, 10, 0)


def test_q_write_13_sync_takeover_timestamp_mismatch_and_rollback(db_session: Session, monkeypatch):
    """
    Quarantine candle seeded with non-midnight timestamp. Real provider sync takes over,
    normalizes timestamp, and rollback restores exact pre-operation quarantine state.
    """
    monkeypatch.setattr(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", False)

    db_session.add(Symbol(symbol="VNM", is_active=True))
    quar = Candle(
        symbol="VNM",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 14, 30),
        open=10.0, high=10.0, low=10.0, close=10.0, volume=100.0,
        source="quarantine:sync:fake_sync_vnm",
        adjustment_type="unadjusted"
    )
    db_session.add(quar)
    db_session.commit()

    registry = ProviderRegistry.get_instance()
    provider = RealProviderStub(provider_id="real_vnm_feed_probe", candle_date=date(2026, 8, 3))
    registry.register_provider(provider)

    preview = SyncWorkflowService.generate_sync_preview(
        db=db_session,
        request=SyncPreviewRequest(
            provider_id="real_vnm_feed_probe",
            symbol="VNM",
            start_date="2026-08-03",
            end_date="2026-08-03",
            adjustment_type="unadjusted"
        )
    )
    assert preview.can_accept is True

    exec_resp = SyncWorkflowService.execute_sync(
        db=db_session,
        sync_id=preview.sync_id,
        content_sha256=preview.content_sha256
    )
    assert exec_resp.status == "accepted"

    # Promoted candle has normalized timestamp
    c_promoted = db_session.query(Candle).filter(Candle.symbol == "VNM", Candle.timeframe == "1D").one()
    assert c_promoted.timestamp == datetime(2026, 8, 3, 0, 0)

    rb_resp = SyncWorkflowService.rollback_sync(db=db_session, sync_id=preview.sync_id)
    assert rb_resp.status == "rolled_back"

    restored = (
        db_session.query(Candle)
        .execution_options(include_quarantine=True)
        .filter(Candle.symbol == "VNM", Candle.timeframe == "1D")
        .first()
    )
    assert restored is not None
    assert restored.source == "quarantine:sync:fake_sync_vnm"
    assert restored.close == 10.0
    assert restored.timestamp == datetime(2026, 8, 3, 14, 30)


def test_q_write_14_weekly_derivation_excludes_coexisting_quarantine_daily(db_session: Session):
    """
    Weekly aggregation deriving from mixed daily members in the same week:
    canonical 1W candle must derive strictly from canonical daily members and
    must not be contaminated by coexisting quarantine daily members.
    """
    db_session.add(Symbol(symbol="HPG", is_active=True))
    db_session.add(Candle(
        symbol="HPG",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0),
        open=28.0, high=29.0, low=27.5, close=28.5, volume=50000.0,
        source="cafef_import_mon",
        adjustment_type="unadjusted"
    ))
    db_session.add(Candle(
        symbol="HPG",
        timeframe="1D",
        timestamp=datetime(2026, 8, 4, 0, 0),
        open=100.0, high=100.0, low=100.0, close=100.0, volume=999999.0,
        source="quarantine:sync:fake_tue",
        adjustment_type="unadjusted"
    ))
    db_session.commit()

    WeeklyAggregator.derive_weekly_candles(db_session, {"HPG"}, {"unadjusted"})

    w_candles = (
        db_session.query(Candle)
        .filter(Candle.symbol == "HPG", Candle.timeframe == "1W")
        .all()
    )
    assert len(w_candles) == 1
    w = w_candles[0]
    assert w.source == "VN_TRADING_WEEK_V1"
    assert w.open == 28.0
    assert w.high == 29.0
    assert w.low == 27.5
    assert w.close == 28.5
    assert w.volume == 50000.0



# ==============================================================================
# Additional Adversarial & Integrity Tests (Review Round 2)
# ==============================================================================
def test_q_write_15_retakeover_after_rollback_import(db_session: Session):
    """
    Test that after an import takeover is rolled back, a second import can take over
    the restored quarantine candle cleanly without unique key collision or stale errors.
    """
    db_session.add(Symbol(symbol="HPG", is_active=True))
    quar = Candle(
        symbol="HPG",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0),
        open=10.0, high=10.0, low=10.0, close=10.0, volume=100.0,
        source="quarantine:sync:seed_1",
        adjustment_type="unadjusted"
    )
    db_session.add(quar)
    db_session.commit()
    quar_id = quar.id

    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260803,28.0,29.0,27.5,28.5,50000\n"

    # 1st import
    prev1 = ImportWorkflowService.preview_import(db_session, csv_content, "HPG_1.csv")
    assert prev1.can_accept is True
    acc1 = ImportWorkflowService.accept_import(db_session, prev1.run_id, prev1.content_sha256)
    assert acc1.status == "accepted"

    # 1st rollback
    rb1 = ImportWorkflowService.rollback_import(db_session, prev1.run_id)
    assert rb1.status == "rolled_back"

    # Verify quarantine restored
    restored = db_session.query(Candle).execution_options(include_quarantine=True).filter(Candle.symbol == "HPG", Candle.timeframe == "1D").one()
    assert restored.id == quar_id
    assert restored.source == "quarantine:sync:seed_1"
    assert restored.close == 10.0

    # 2nd import taking over the restored quarantine candle
    csv_content_2 = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260803,29.0,30.0,28.5,29.5,60000\n"
    prev2 = ImportWorkflowService.preview_import(db_session, csv_content_2, "HPG_2.csv")
    assert prev2.can_accept is True
    assert prev2.parsed_count == 1
    acc2 = ImportWorkflowService.accept_import(db_session, prev2.run_id, prev2.content_sha256)
    assert acc2.status == "accepted"

    # Verify canonical candle has 2nd import values
    c2 = db_session.query(Candle).filter(Candle.symbol == "HPG", Candle.timeframe == "1D").one()
    assert c2.id == quar_id
    assert c2.close == 29500.0
    assert c2.source == prev2.run_id


def test_q_write_16_retakeover_after_rollback_sync(db_session: Session, monkeypatch):
    """
    Test that after a sync takeover is rolled back, a second sync can take over
    the restored quarantine candle cleanly without errors.
    """
    monkeypatch.setattr(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", False)
    db_session.add(Symbol(symbol="VNM", is_active=True))
    quar = Candle(
        symbol="VNM",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0),
        open=10.0, high=10.0, low=10.0, close=10.0, volume=100.0,
        source="quarantine:sync:seed_vnm",
        adjustment_type="unadjusted"
    )
    db_session.add(quar)
    db_session.commit()
    quar_id = quar.id

    reg = ProviderRegistry.get_instance()
    p1 = RealProviderStub(provider_id="feed_16_a", candle_date=date(2026, 8, 3))
    reg.register_provider(p1)

    prev1 = SyncWorkflowService.generate_sync_preview(
        db_session,
        SyncPreviewRequest(provider_id="feed_16_a", symbol="VNM", start_date="2026-08-03", end_date="2026-08-03", adjustment_type="unadjusted")
    )
    assert prev1.can_accept is True
    exec1 = SyncWorkflowService.execute_sync(db_session, prev1.sync_id, prev1.content_sha256)
    assert exec1.status == "accepted"

    rb1 = SyncWorkflowService.rollback_sync(db_session, prev1.sync_id)
    assert rb1.status == "rolled_back"

    # Restored
    restored = db_session.query(Candle).execution_options(include_quarantine=True).filter(Candle.symbol == "VNM", Candle.timeframe == "1D").one()
    assert restored.id == quar_id
    assert restored.source == "quarantine:sync:seed_vnm"

    # 2nd sync taking over
    p2 = RealProviderStub(provider_id="feed_16_b", candle_date=date(2026, 8, 3))
    reg.register_provider(p2)
    prev2 = SyncWorkflowService.generate_sync_preview(
        db_session,
        SyncPreviewRequest(provider_id="feed_16_b", symbol="VNM", start_date="2026-08-03", end_date="2026-08-03", adjustment_type="unadjusted")
    )
    assert prev2.can_accept is True
    assert prev2.parsed_count == 1
    exec2 = SyncWorkflowService.execute_sync(db_session, prev2.sync_id, prev2.content_sha256)
    assert exec2.status == "accepted"

    c2 = db_session.query(Candle).filter(Candle.symbol == "VNM", Candle.timeframe == "1D").one()
    assert c2.id == quar_id
    assert c2.close == 28.5
    assert c2.source == f"sync:{prev2.sync_id}"


def test_q_write_17_cross_workflow_rollback_import_with_subsequent_sync(db_session: Session, monkeypatch):
    """
    If ImportRun 1 accepts HPG, and subsequently SyncRun 2 accepts HPG,
    attempting to roll back ImportRun 1 must be rejected with 400 to preserve cross-workflow dataset integrity.
    """
    monkeypatch.setattr(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", False)
    db_session.add(Symbol(symbol="HPG", is_active=True))
    db_session.commit()

    # 1. ImportRun 1 accepts HPG on Day 1
    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260803,28.0,29.0,27.5,28.5,50000\n"
    prev1 = ImportWorkflowService.preview_import(db_session, csv_content, "HPG_DAY1.csv")
    acc1 = ImportWorkflowService.accept_import(db_session, prev1.run_id, prev1.content_sha256)
    assert acc1.status == "accepted"

    # 2. SyncRun 2 accepts HPG on Day 2
    reg = ProviderRegistry.get_instance()
    p = RealProviderStub(provider_id="feed_17_sync", candle_date=date(2026, 8, 4))
    reg.register_provider(p)
    prev2 = SyncWorkflowService.generate_sync_preview(
        db_session,
        SyncPreviewRequest(provider_id="feed_17_sync", symbol="HPG", start_date="2026-08-04", end_date="2026-08-04", adjustment_type="unadjusted")
    )
    exec2 = SyncWorkflowService.execute_sync(db_session, prev2.sync_id, prev2.content_sha256)
    assert exec2.status == "accepted"

    # 3. Attempt to rollback ImportRun 1 must be rejected because SyncRun 2 was accepted after ImportRun 1
    with pytest.raises(HTTPException) as exc_info:
        ImportWorkflowService.rollback_import(db_session, prev1.run_id)
    assert exc_info.value.status_code == 400
    assert "đồng bộ" in exc_info.value.detail


def test_q_write_18_import_quarantine_source_containing_pipe(db_session: Session):
    """
    If quarantine source contains a pipe character (e.g. 'quarantine:custom|tag:123'),
    import takeover and subsequent rollback must restore the full source string without truncation.
    """
    db_session.add(Symbol(symbol="HPG", is_active=True))
    quar = Candle(
        symbol="HPG",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 10, 0),
        open=10.0, high=10.0, low=10.0, close=10.0, volume=100.0,
        source="quarantine:custom|tag:123",
        adjustment_type="unadjusted"
    )
    db_session.add(quar)
    db_session.commit()
    quar_id = quar.id

    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260803,28.0,29.0,27.5,28.5,50000\n"
    prev = ImportWorkflowService.preview_import(db_session, csv_content, "HPG_PIPE.csv")
    acc = ImportWorkflowService.accept_import(db_session, prev.run_id, prev.content_sha256)
    assert acc.status == "accepted"

    rb = ImportWorkflowService.rollback_import(db_session, prev.run_id)
    assert rb.status == "rolled_back"

    restored = db_session.query(Candle).execution_options(include_quarantine=True).filter(Candle.symbol == "HPG", Candle.timeframe == "1D").one()
    assert restored.source == "quarantine:custom|tag:123"
    assert restored.timestamp == datetime(2026, 8, 3, 10, 0)


def test_q_write_19_sync_quarantine_source_containing_pipe(db_session: Session, monkeypatch):
    """
    If quarantine source contains a pipe character, sync takeover and subsequent rollback
    must restore the full source string without truncation.
    """
    monkeypatch.setattr(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", False)
    db_session.add(Symbol(symbol="VNM", is_active=True))
    quar = Candle(
        symbol="VNM",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 10, 0),
        open=10.0, high=10.0, low=10.0, close=10.0, volume=100.0,
        source="quarantine:custom|tag:123",
        adjustment_type="unadjusted"
    )
    db_session.add(quar)
    db_session.commit()
    quar_id = quar.id

    reg = ProviderRegistry.get_instance()
    p = RealProviderStub(provider_id="feed_19_pipe", candle_date=date(2026, 8, 3))
    reg.register_provider(p)

    prev = SyncWorkflowService.generate_sync_preview(
        db_session,
        SyncPreviewRequest(provider_id="feed_19_pipe", symbol="VNM", start_date="2026-08-03", end_date="2026-08-03", adjustment_type="unadjusted")
    )
    exec_resp = SyncWorkflowService.execute_sync(db_session, prev.sync_id, prev.content_sha256)
    assert exec_resp.status == "accepted"

    rb = SyncWorkflowService.rollback_sync(db_session, prev.sync_id)
    assert rb.status == "rolled_back"

    restored = db_session.query(Candle).execution_options(include_quarantine=True).filter(Candle.symbol == "VNM", Candle.timeframe == "1D").one()
    assert restored.source == "quarantine:custom|tag:123"
    assert restored.timestamp == datetime(2026, 8, 3, 10, 0)


# ==============================================================================
# Additional Adversarial & Integrity Tests (Review Round 3)
# ==============================================================================
def test_q_write_20_cross_workflow_rollback_sync_with_subsequent_import(db_session: Session):
    """
    Symmetric cross-workflow rollback integrity:
    Real provider sync accepts HPG. Subsequently, a canonical import accepts HPG.
    Attempting to rollback the earlier sync run must be rejected with HTTP 400.
    """
    db_session.add(Symbol(symbol="HPG", is_active=True))
    db_session.commit()

    registry = ProviderRegistry.get_instance()
    provider = RealProviderStub(provider_id="sync_hpg_r3_prov", candle_date=date(2026, 8, 3))
    registry.register_provider(provider)

    # 1. Execute Sync for HPG
    p_sync = SyncWorkflowService.generate_sync_preview(
        db=db_session,
        request=SyncPreviewRequest(
            provider_id="sync_hpg_r3_prov",
            symbol="HPG",
            start_date="2026-08-03",
            end_date="2026-08-03",
            adjustment_type="unadjusted"
        )
    )
    exec_resp = SyncWorkflowService.execute_sync(db=db_session, sync_id=p_sync.sync_id, content_sha256=p_sync.content_sha256)
    assert exec_resp.status == "accepted"

    # 2. Execute Import for HPG
    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260804,29.0,30.0,28.5,29.5,60000\n"
    p_import = ImportWorkflowService.preview_import(
        db=db_session,
        file_content=csv_content,
        filename="HPG_NEXT.csv",
        adjustment_type="unadjusted"
    )
    accept_resp = ImportWorkflowService.accept_import(db=db_session, run_id=p_import.run_id, content_sha256=p_import.content_sha256)
    assert accept_resp.status == "accepted"

    # 3. Attempt rollback of the earlier Sync -> MUST be rejected with HTTP 400
    with pytest.raises(HTTPException) as exc:
        SyncWorkflowService.rollback_sync(db=db_session, sync_id=p_sync.sync_id)
    assert exc.value.status_code == 400
    assert "Đã có lượt nhập tập tin mới hơn" in exc.value.detail
    assert "HPG" in exc.value.detail


def test_q_write_21_multi_cycle_takeover_and_rollback(db_session: Session):
    """
    Repeated cycles of takeover and rollback alternating between Import and Sync:
    Quarantine -> Import -> Rollback -> Sync -> Rollback -> Import.
    Proves exact quarantine provenance is restored at each rollback step.
    """
    db_session.add(Symbol(symbol="HPG", is_active=True))
    quar = Candle(
        symbol="HPG",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0),
        open=10.0, high=10.0, low=10.0, close=10.0, volume=100.0,
        source="quarantine:cycle_init",
        adjustment_type="unadjusted"
    )
    db_session.add(quar)
    db_session.commit()

    registry = ProviderRegistry.get_instance()
    provider = RealProviderStub(provider_id="real_cycle_r3_prov", candle_date=date(2026, 8, 3))
    registry.register_provider(provider)

    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260803,28.0,29.0,27.5,28.5,50000\n"

    # Cycle 1: Import takeover and rollback
    p1 = ImportWorkflowService.preview_import(db=db_session, file_content=csv_content, filename="cycle1.csv", adjustment_type="unadjusted")
    ImportWorkflowService.accept_import(db=db_session, run_id=p1.run_id, content_sha256=p1.content_sha256)
    c1 = db_session.query(Candle).filter(Candle.symbol == "HPG", Candle.timeframe == "1D").one()
    assert c1.close == 28500.0
    ImportWorkflowService.rollback_import(db=db_session, run_id=p1.run_id)
    r1 = db_session.query(Candle).execution_options(include_quarantine=True).filter(Candle.symbol == "HPG", Candle.timeframe == "1D").one()
    assert r1.source == "quarantine:cycle_init"
    assert r1.close == 10.0

    # Cycle 2: Sync takeover and rollback
    p2 = SyncWorkflowService.generate_sync_preview(
        db=db_session,
        request=SyncPreviewRequest(
            provider_id="real_cycle_r3_prov",
            symbol="HPG",
            start_date="2026-08-03",
            end_date="2026-08-03",
            adjustment_type="unadjusted"
        )
    )
    SyncWorkflowService.execute_sync(db=db_session, sync_id=p2.sync_id, content_sha256=p2.content_sha256)
    c2 = db_session.query(Candle).filter(Candle.symbol == "HPG", Candle.timeframe == "1D").one()
    assert c2.close == 28.5
    SyncWorkflowService.rollback_sync(db=db_session, sync_id=p2.sync_id)
    r2 = db_session.query(Candle).execution_options(include_quarantine=True).filter(Candle.symbol == "HPG", Candle.timeframe == "1D").one()
    assert r2.source == "quarantine:cycle_init"
    assert r2.close == 10.0

    # Cycle 3: Import takeover again
    p3 = ImportWorkflowService.preview_import(db=db_session, file_content=csv_content, filename="cycle3.csv", adjustment_type="unadjusted")
    ImportWorkflowService.accept_import(db=db_session, run_id=p3.run_id, content_sha256=p3.content_sha256)
    c3 = db_session.query(Candle).filter(Candle.symbol == "HPG", Candle.timeframe == "1D").one()
    assert c3.close == 28500.0


def test_q_write_22_stale_preview_detection_after_canonical_creation(db_session: Session):
    """
    Preview generated when quarantine row exists.
    If a canonical candle is committed before accept_import / execute_sync,
    the operation must detect stale preview and fail-closed with HTTP 400.
    """
    db_session.add(Symbol(symbol="HPG", is_active=True))
    quar = Candle(
        symbol="HPG", timeframe="1D", timestamp=datetime(2026, 8, 3, 0, 0),
        open=10.0, high=10.0, low=10.0, close=10.0, volume=100.0,
        source="quarantine:probe_stale", adjustment_type="unadjusted"
    )
    db_session.add(quar)
    db_session.commit()

    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260803,28.0,29.0,27.5,28.5,50000\n"
    preview = ImportWorkflowService.preview_import(
        db=db_session, file_content=csv_content, filename="stale.csv", adjustment_type="unadjusted"
    )

    # Simulate canonical candle inserted concurrently
    db_session.delete(quar)
    db_session.flush()
    db_session.add(Candle(
        symbol="HPG", timeframe="1D", timestamp=datetime(2026, 8, 3, 0, 0),
        open=25.0, high=26.0, low=24.5, close=25.5, volume=30000.0,
        source="concurrent_write", adjustment_type="unadjusted"
    ))
    db_session.commit()

    with pytest.raises(HTTPException) as exc:
        ImportWorkflowService.accept_import(db=db_session, run_id=preview.run_id, content_sha256=preview.content_sha256)
    assert exc.value.status_code == 400
    assert "Bản xem trước đã hết hạn" in exc.value.detail


def test_q_write_23_different_adjustment_type_coexists_safely(db_session: Session):
    """
    Quarantine row exists with adjustment_type='adjusted'.
    Canonical import arrives with adjustment_type='unadjusted'.
    Both candles coexist in DB at distinct unique keys; canonical reader sees only 'unadjusted'.
    """
    db_session.add(Symbol(symbol="HPG", is_active=True))
    quar = Candle(
        symbol="HPG", timeframe="1D", timestamp=datetime(2026, 8, 3, 0, 0),
        open=10.0, high=10.0, low=10.0, close=10.0, volume=100.0,
        source="quarantine:fake_adjusted", adjustment_type="adjusted"
    )
    db_session.add(quar)
    db_session.commit()

    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260803,28.0,29.0,27.5,28.5,50000\n"
    preview = ImportWorkflowService.preview_import(
        db=db_session, file_content=csv_content, filename="unadj.csv", adjustment_type="unadjusted"
    )
    accept_resp = ImportWorkflowService.accept_import(db=db_session, run_id=preview.run_id, content_sha256=preview.content_sha256)
    assert accept_resp.status == "accepted"

    all_1d = (
        db_session.query(Candle)
        .execution_options(include_quarantine=True)
        .filter(Candle.symbol == "HPG", Candle.timeframe == "1D")
        .all()
    )
    assert len(all_1d) == 2
    by_adj = {c.adjustment_type: c for c in all_1d}
    assert by_adj["adjusted"].source == "quarantine:fake_adjusted"
    assert by_adj["unadjusted"].close == 28500.0

    canonical = db_session.query(Candle).filter(Candle.symbol == "HPG", Candle.timeframe == "1D").all()
    assert len(canonical) == 1
    assert canonical[0].adjustment_type == "unadjusted"


def test_q_write_24_multi_symbol_mixed_batch_takeover_and_rollback(db_session: Session):
    """
    Multi-symbol mixed batch:
    SYM_Q has quarantine row (superseded).
    SYM_D has existing canonical row matching CSV values (duplicate).
    SYM_N has no existing row (inserted).
    Proves takeover, duplicate skipping, insert, and atomic rollback on mixed multi-symbol files.
    """
    db_session.add(Symbol(symbol="SYM_Q", is_active=True))
    db_session.add(Symbol(symbol="SYM_D", is_active=True))
    db_session.add(Symbol(symbol="SYM_N", is_active=True))

    db_session.add(Candle(
        symbol="SYM_Q", timeframe="1D", timestamp=datetime(2026, 8, 3, 0, 0),
        open=1.0, high=1.0, low=1.0, close=1.0, volume=10.0,
        source="quarantine:q_source", adjustment_type="unadjusted"
    ))
    db_session.add(Candle(
        symbol="SYM_D", timeframe="1D", timestamp=datetime(2026, 8, 3, 0, 0),
        open=20000.0, high=21000.0, low=19000.0, close=20500.0, volume=50000.0,
        source="existing_run", adjustment_type="unadjusted"
    ))
    db_session.commit()

    csv_content = (
        b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\n"
        b"SYM_Q,20260803,10.0,11.0,9.0,10.5,5000\n"
        b"SYM_D,20260803,20.0,21.0,19.0,20.5,50000\n"
        b"SYM_N,20260803,30.0,31.0,29.0,30.5,8000\n"
    )

    preview = ImportWorkflowService.preview_import(
        db=db_session, file_content=csv_content, filename="mixed.csv", adjustment_type="unadjusted"
    )
    assert preview.can_accept is True
    assert preview.parsed_count == 2
    assert preview.duplicate_count == 1

    accept_resp = ImportWorkflowService.accept_import(
        db=db_session, run_id=preview.run_id, content_sha256=preview.content_sha256
    )
    assert accept_resp.status == "accepted"
    assert accept_resp.accepted_count == 2

    q_c = db_session.query(Candle).filter(Candle.symbol == "SYM_Q", Candle.timeframe == "1D").one()
    assert q_c.close == 10500.0
    d_c = db_session.query(Candle).filter(Candle.symbol == "SYM_D", Candle.timeframe == "1D").one()
    assert d_c.source == "existing_run"
    n_c = db_session.query(Candle).filter(Candle.symbol == "SYM_N", Candle.timeframe == "1D").one()
    assert n_c.close == 30500.0

    ImportWorkflowService.rollback_import(db=db_session, run_id=preview.run_id)

    assert db_session.query(Candle).filter(Candle.symbol == "SYM_Q", Candle.timeframe == "1D").count() == 0
    restored_q = db_session.query(Candle).execution_options(include_quarantine=True).filter(Candle.symbol == "SYM_Q", Candle.timeframe == "1D").one()
    assert restored_q.source == "quarantine:q_source"
    assert restored_q.close == 1.0

    assert db_session.query(Candle).filter(Candle.symbol == "SYM_N", Candle.timeframe == "1D").count() == 0
    d_after = db_session.query(Candle).filter(Candle.symbol == "SYM_D", Candle.timeframe == "1D").one()
    assert d_after.source == "existing_run"


def test_q_write_25_fault_injection_during_rollback_preserves_state(db_session: Session):
    """
    Fault injection during rollback execution must abort the transaction cleanly,
    leaving the canonical candles intact and run status as 'accepted'.
    """
    db_session.add(Symbol(symbol="HPG", is_active=True))
    quar = Candle(
        symbol="HPG", timeframe="1D", timestamp=datetime(2026, 8, 3, 0, 0),
        open=10.0, high=10.0, low=10.0, close=10.0, volume=100.0,
        source="quarantine:probe25", adjustment_type="unadjusted"
    )
    db_session.add(quar)
    db_session.commit()

    csv_content = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260803,28.0,29.0,27.5,28.5,50000\n"
    preview = ImportWorkflowService.preview_import(db=db_session, file_content=csv_content, filename="hpg.csv", adjustment_type="unadjusted")
    ImportWorkflowService.accept_import(db=db_session, run_id=preview.run_id, content_sha256=preview.content_sha256)

    with patch("app.services.weekly_aggregator.WeeklyAggregator.derive_weekly_candles", side_effect=RuntimeError("Rollback Crash")):
        with pytest.raises(RuntimeError):
            ImportWorkflowService.rollback_import(db=db_session, run_id=preview.run_id)

    run = db_session.query(ImportRun).filter(ImportRun.id == preview.run_id).one()
    assert run.status == "accepted"
    c = db_session.query(Candle).filter(Candle.symbol == "HPG", Candle.timeframe == "1D").one()
    assert c.close == 28500.0


def test_q_write_26_concurrent_separate_preview_race_detection(db_session: Session):
    """
    Two independent import previews P1 and P2 generated against quarantine placeholder row.
    P1 is accepted first, superseding quarantine with canonical data.
    P2 is then accepted: stale candidate check must detect that canonical data was committed
    after P2 was previewed, and fail-closed with HTTP 400 without corrupting or double-writing.
    """
    db_session.add(Symbol(symbol="HPG", is_active=True))
    quar = Candle(
        symbol="HPG", timeframe="1D", timestamp=datetime(2026, 8, 3, 0, 0),
        open=10.0, high=10.0, low=10.0, close=10.0, volume=100.0,
        source="quarantine:race_test", adjustment_type="unadjusted"
    )
    db_session.add(quar)
    db_session.commit()

    csv_p1 = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260803,28.0,29.0,27.5,28.5,50000\n"
    csv_p2 = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nHPG,20260803,28.2,29.1,27.6,28.7,55000\n"

    # Both generate preview while quarantine exists
    prev1 = ImportWorkflowService.preview_import(db_session, csv_p1, "run1.csv", adjustment_type="unadjusted")
    prev2 = ImportWorkflowService.preview_import(db_session, csv_p2, "run2.csv", adjustment_type="unadjusted")
    assert prev1.can_accept is True
    assert prev2.can_accept is True

    # P1 accepted first -> supersedes quarantine
    resp1 = ImportWorkflowService.accept_import(db_session, prev1.run_id, prev1.content_sha256)
    assert resp1.status == "accepted"

    # P2 attempted second -> MUST fail-closed because canonical data now exists
    with pytest.raises(HTTPException) as exc:
        ImportWorkflowService.accept_import(db_session, prev2.run_id, prev2.content_sha256)
    assert exc.value.status_code == 400
    assert "Bản xem trước đã hết hạn" in exc.value.detail

    # Verify P1 data remains authoritative
    c = db_session.query(Candle).filter(Candle.symbol == "HPG", Candle.timeframe == "1D").one()
    assert c.source == prev1.run_id
    assert c.close == 28500.0


def test_q_write_27_synthetic_sync_identical_ohlcv_noop_preserves_canonical(db_session: Session, monkeypatch):
    """
    R3 Synthetic/Canonical Precedence: Explicit identical OHLCV handling.
    When a canonical candle exists, and a synthetic provider returns data with IDENTICAL OHLCV values:
    1. Default deny (SUMI_ALLOW_SYNTHETIC_PROVIDERS=False): execute_sync raises HTTP 403 Forbidden.
    2. With opt-in (SUMI_ALLOW_SYNTHETIC_PROVIDERS=True):
       - Preview classifies the record as 'duplicate' (parsed_count=0, duplicate_count=1, conflicting_count=0).
       - execute_sync resolves as idempotent noop (status='noop', accepted_count=0).
       - Canonical candle remains completely authoritative (original source preserved, no quarantine tag).
       - Zero mutations recorded in SyncRunMutation.
    """
    db_session.add(Symbol(symbol="TCB", is_active=True))
    canon_candle = Candle(
        symbol="TCB",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0),
        open=25000.0,
        high=26000.0,
        low=24500.0,
        close=25500.0,
        volume=120000.0,
        source="import:canonical_tcb_run",
        adjustment_type="unadjusted"
    )
    db_session.add(canon_candle)
    db_session.commit()

    # Synthetic provider returning EXACT same OHLCV values
    class SyntheticIdenticalProvider(MarketDataProviderAdapter):
        def get_metadata(self) -> ProviderMetadata:
            return ProviderMetadata(
                provider_id="synth_ident_prov",
                display_name="Synthetic Identical Feed",
                is_official=False,
                requires_auth=False,
                supported_timeframes=["1D"],
                supported_adjustments=["unadjusted"],
                rate_limit_rps=10,
                description="Synthetic Test",
                auth_fields=[],
                is_synthetic=True,
            )

        def test_connection(self, credentials=None) -> bool:
            return True

        def fetch_daily_candles(self, symbol: str, start_date: date, end_date: date, adjustment_type="unadjusted", credentials=None):
            return [
                ProviderCandleDTO(
                    symbol=symbol,
                    timeframe="1D",
                    timestamp=date(2026, 8, 3),
                    open=25000.0,
                    high=26000.0,
                    low=24500.0,
                    close=25500.0,
                    volume=120000.0,
                    adjustment_type=adjustment_type,
                )
            ]

        def fetch_benchmark_indices(self, benchmark: str, start_date: date, end_date: date, credentials=None):
            return []

    reg = ProviderRegistry.get_instance()
    reg.register_provider(SyntheticIdenticalProvider())

    req = SyncPreviewRequest(
        provider_id="synth_ident_prov",
        symbol="TCB",
        start_date="2026-08-03",
        end_date="2026-08-03",
        adjustment_type="unadjusted"
    )

    # 1. Generate preview
    prev = SyncWorkflowService.generate_sync_preview(db_session, req)
    assert prev.can_accept is True
    assert prev.duplicate_count == 1
    assert prev.parsed_count == 0
    assert prev.conflicting_count == 0

    # 2. Under default-deny (SUMI_ALLOW_SYNTHETIC_PROVIDERS=False), execute_sync must fail with 403
    monkeypatch.setattr(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", False)
    with pytest.raises(HTTPException) as exc_info:
        SyncWorkflowService.execute_sync(db_session, prev.sync_id, prev.content_sha256)
    assert exc_info.value.status_code == 403

    # 3. Under opt-in (SUMI_ALLOW_SYNTHETIC_PROVIDERS=True), execute_sync must succeed as noop
    monkeypatch.setattr(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", True)
    exec_resp = SyncWorkflowService.execute_sync(db_session, prev.sync_id, prev.content_sha256)
    assert exec_resp.status == "noop"
    assert exec_resp.accepted_count == 0

    # 4. Verify canonical candle remains unchanged and authoritative
    c = db_session.query(Candle).filter(Candle.symbol == "TCB", Candle.timeframe == "1D").one()
    assert c.source == "import:canonical_tcb_run"
    assert c.close == 25500.0
    assert db_session.query(SyncRunMutation).filter(SyncRunMutation.sync_id == prev.sync_id).count() == 0

