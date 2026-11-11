import pytest
from datetime import date, datetime
from unittest.mock import patch, MagicMock
from fastapi import HTTPException
from sqlalchemy.orm import Session

from app.models.candle import Candle
from app.models.symbol import Symbol
from app.models.sync_run import SyncRun, SyncRunItem, SyncRunMutation
from app.services.data_providers.provider_registry import ProviderRegistry
from app.services.data_providers.base_provider import (
    MarketDataProviderAdapter,
    ProviderCandleDTO,
    ProviderMetadata,
)
from app.services.sync_workflow_service import SyncWorkflowService
from app.schemas.sync_schema import SyncPreviewRequest, SyncExecuteRequest
from app.config import settings, Settings

@pytest.fixture(autouse=True)
def clean_registry():
    """Ensure registry singleton is cleanly reset after each test."""
    yield
    ProviderRegistry.reset_instance()


# Test double for a real (non-synthetic) provider
class FakeRealProviderAdapter(MarketDataProviderAdapter):
    def __init__(self, provider_id: str = "real_broker"):
        self._metadata = ProviderMetadata(
            provider_id=provider_id,
            display_name="Real Broker Production API",
            is_official=True,
            requires_auth=False,
            supported_timeframes=["1D"],
            supported_adjustments=["unadjusted"],
            rate_limit_rps=10,
            description="Verified live production market data provider",
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
                timestamp=date(2026, 8, 3),
                open=100.0,
                high=105.0,
                low=99.0,
                close=104.0,
                volume=100000.0,
                adjustment_type=adjustment_type,
            )
        ]

    def fetch_benchmark_indices(self, benchmark: str, start_date: date, end_date: date, credentials=None):
        return []


# Test double for a custom synthetic stub with a non-standard name
class FakeCustomStubAdapter(MarketDataProviderAdapter):
    def __init__(self, provider_id: str = "custom_stub"):
        self._metadata = ProviderMetadata(
            provider_id=provider_id,
            display_name="Custom In-Memory Stub",
            is_official=False,
            requires_auth=False,
            supported_timeframes=["1D"],
            supported_adjustments=["unadjusted"],
            rate_limit_rps=10,
            description="Arbitrary synthetic stub",
            auth_fields=[],
            is_synthetic=True,
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
                timestamp=date(2026, 8, 3),
                open=50.0,
                high=55.0,
                low=49.0,
                close=52.0,
                volume=50000.0,
                adjustment_type=adjustment_type,
            )
        ]

    def fetch_benchmark_indices(self, benchmark: str, start_date: date, end_date: date, credentials=None):
        return []


# ==============================================================================
# SYN-01: Synthetic Provider Identified
# ==============================================================================
def test_syn01_synthetic_provider_identified():
    """SYN-01: ssi and vnstock providers must expose machine-readable is_synthetic == True."""
    registry = ProviderRegistry.get_instance()
    
    ssi = registry.get_provider("ssi")
    assert ssi is not None
    ssi_meta = ssi.get_metadata()
    assert hasattr(ssi_meta, "is_synthetic"), "ProviderMetadata must have is_synthetic attribute"
    assert ssi_meta.is_synthetic is True, "SSI stub adapter must declare is_synthetic=True"

    vnstock = registry.get_provider("vnstock")
    assert vnstock is not None
    vnstock_meta = vnstock.get_metadata()
    assert hasattr(vnstock_meta, "is_synthetic"), "ProviderMetadata must have is_synthetic attribute"
    assert vnstock_meta.is_synthetic is True, "vnstock stub adapter must declare is_synthetic=True"


# ==============================================================================
# SYN-02 & SYN-03: Execute Denied by Default & DB Remains Clean
# ==============================================================================
def test_syn02_and_syn03_execute_denied_by_default_and_db_clean(client, db_session, monkeypatch):
    """
    SYN-02 & SYN-03: Preview -> Execute with synthetic provider (ssi/vnstock) without
    explicit opt-in must be denied (HTTP 403), writing 0 candles, 0 weekly candles, 0 mutations.
    """
    monkeypatch.setattr(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", False)

    # 1. Preview is allowed
    prev_resp = client.post("/api/sync/preview", json={
        "symbol": "VNM",
        "start_date": "2026-08-03",
        "end_date": "2026-08-07",
        "provider_id": "ssi",
        "adjustment_type": "unadjusted"
    })
    assert prev_resp.status_code == 200
    prev_data = prev_resp.json()
    sync_id = prev_data["sync_id"]
    checksum = prev_data["content_sha256"]

    # 2. Execute without opt-in must fail with 403 Forbidden
    exec_resp = client.post("/api/sync/execute", json={
        "sync_id": sync_id,
        "content_sha256": checksum
    })
    assert exec_resp.status_code == 403, f"Expected 403 Forbidden, got {exec_resp.status_code}: {exec_resp.text}"
    detail = exec_resp.json().get("detail", "")
    assert "giả lập" in detail or "synthetic" in detail.lower() or "quarantine" in detail.lower()

    # SYN-03: DB remains clean
    daily_count = db_session.query(Candle).filter(Candle.symbol == "VNM", Candle.timeframe == "1D").count()
    assert daily_count == 0, "No daily candles must be committed on denied synthetic execution"

    weekly_count = db_session.query(Candle).filter(Candle.symbol == "VNM", Candle.timeframe == "1W").count()
    assert weekly_count == 0, "No weekly candles must be committed on denied synthetic execution"

    mutations_count = db_session.query(SyncRunMutation).filter(SyncRunMutation.sync_id == sync_id).count()
    assert mutations_count == 0, "No mutations must be committed on denied synthetic execution"

    # SyncRun status must NOT be 'accepted'
    run = db_session.query(SyncRun).filter(SyncRun.id == sync_id).first()
    assert run is not None
    assert run.status != "accepted"


# ==============================================================================
# SYN-04 & SYN-05: Explicit Opt-in & Provenance
# ==============================================================================
def test_syn04_and_syn05_explicit_optin_and_quarantine_provenance(client, db_session, monkeypatch):
    """
    SYN-04 & SYN-05: When SUMI_ALLOW_SYNTHETIC_PROVIDERS=True, execution succeeds,
    writing to quarantine dataset where Candle.source is explicitly prefixed with quarantine:sync:
    and audit manifest flags is_synthetic=True.
    """
    monkeypatch.setattr(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", True)

    # 1. Preview
    prev_resp = client.post("/api/sync/preview", json={
        "symbol": "FPT",
        "start_date": "2026-08-03",
        "end_date": "2026-08-07",
        "provider_id": "ssi",
        "adjustment_type": "unadjusted"
    })
    assert prev_resp.status_code == 200
    prev_data = prev_resp.json()
    sync_id = prev_data["sync_id"]
    checksum = prev_data["content_sha256"]

    # 2. Execute with opt-in
    exec_resp = client.post("/api/sync/execute", json={
        "sync_id": sync_id,
        "content_sha256": checksum
    })
    assert exec_resp.status_code == 200, f"Expected 200 OK with opt-in, got {exec_resp.status_code}: {exec_resp.text}"
    exec_data = exec_resp.json()
    assert exec_data["status"] == "accepted"

    # SYN-05: Provenance must be unmistakable quarantine
    # Canonical reader sees 0
    assert len(db_session.query(Candle).filter(Candle.symbol == "FPT", Candle.timeframe == "1D").all()) == 0

    # Authorized quarantine reader sees the quarantined candles
    candles = db_session.query(Candle).execution_options(include_quarantine=True).filter(Candle.symbol == "FPT", Candle.timeframe == "1D").all()
    assert len(candles) == 5
    for c in candles:
        assert c.source.startswith("quarantine:sync:"), (
            f"Expected quarantine source prefix, got {c.source}"
        )
        assert sync_id in c.source

    # Manifest check
    manifest = exec_data.get("manifest")
    assert manifest is not None
    assert manifest.get("is_synthetic") is True
    assert manifest.get("dataset") == "quarantine"


# ==============================================================================
# SYN-06: Real / Non-Synthetic Provider Unaffected
# ==============================================================================
def test_syn06_real_provider_unaffected_by_deny_policy(client, db_session, monkeypatch):
    """
    SYN-06: A real provider (is_synthetic=False) succeeds normally even when
    SUMI_ALLOW_SYNTHETIC_PROVIDERS=False, writing canonical source='sync:{id}'.
    """
    monkeypatch.setattr(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", False)

    registry = ProviderRegistry.get_instance()
    real_adapter = FakeRealProviderAdapter(provider_id="real_test_broker")
    registry.register_provider(real_adapter)

    # 1. Preview
    prev_resp = client.post("/api/sync/preview", json={
        "symbol": "HPG",
        "start_date": "2026-08-03",
        "end_date": "2026-08-03",
        "provider_id": "real_test_broker",
        "adjustment_type": "unadjusted"
    })
    assert prev_resp.status_code == 200
    prev_data = prev_resp.json()
    sync_id = prev_data["sync_id"]
    checksum = prev_data["content_sha256"]

    # 2. Execute must succeed without needing SUMI_ALLOW_SYNTHETIC_PROVIDERS
    exec_resp = client.post("/api/sync/execute", json={
        "sync_id": sync_id,
        "content_sha256": checksum
    })
    assert exec_resp.status_code == 200
    exec_data = exec_resp.json()
    assert exec_data["status"] == "accepted"

    # Canonical source
    c = db_session.query(Candle).filter(Candle.symbol == "HPG", Candle.timeframe == "1D").first()
    assert c is not None
    assert c.source == f"sync:{sync_id}", f"Real provider must write canonical source, got {c.source}"

    # Manifest
    manifest = exec_data.get("manifest")
    assert manifest is not None
    assert manifest.get("is_synthetic") is False
    assert manifest.get("dataset") == "canonical"


# ==============================================================================
# SYN-07: Preview Policy
# ==============================================================================
def test_syn07_preview_policy_marks_synthetic_data(client):
    """
    SYN-07: Preview of synthetic provider is allowed, but explicitly marks
    is_synthetic=True and returns a visible warning in the response.
    """
    resp = client.post("/api/sync/preview", json={
        "symbol": "VNM",
        "start_date": "2026-08-03",
        "end_date": "2026-08-07",
        "provider_id": "ssi",
        "adjustment_type": "unadjusted"
    })
    assert resp.status_code == 200
    data = resp.json()
    assert "is_synthetic" in data, "SyncPreviewResponse must include is_synthetic flag"
    assert data["is_synthetic"] is True
    assert "warning" in data, "SyncPreviewResponse must include warning field for synthetic data"
    assert data["warning"] is not None
    assert "giả lập" in data["warning"] or "synthetic" in data["warning"].lower()


# ==============================================================================
# SYN-08: Rollback / Transaction Integration
# ==============================================================================
def test_syn08_denied_execution_leaves_session_clean(client, db_session, monkeypatch):
    """
    SYN-08: Denied synthetic execution leaves session unpoisoned;
    subsequent operation succeeds cleanly.
    """
    monkeypatch.setattr(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", False)

    # Denied synthetic preview -> execute
    prev_resp = client.post("/api/sync/preview", json={
        "symbol": "VNM",
        "start_date": "2026-08-03",
        "end_date": "2026-08-07",
        "provider_id": "ssi",
        "adjustment_type": "unadjusted"
    })
    sync_id = prev_resp.json()["sync_id"]
    checksum = prev_resp.json()["content_sha256"]

    exec_resp = client.post("/api/sync/execute", json={
        "sync_id": sync_id,
        "content_sha256": checksum
    })
    assert exec_resp.status_code == 403

    # Subsequent valid DB operation in same session/engine must succeed without PendingRollbackError
    sym = Symbol(symbol="TEST_RECOVERY", asset_type="stock", is_active=True)
    db_session.add(sym)
    db_session.commit()

    queried = db_session.query(Symbol).filter(Symbol.symbol == "TEST_RECOVERY").first()
    assert queried is not None


# ==============================================================================
# ADV-SYN-01: Provider named 'ssi' but is_synthetic=False
# ==============================================================================
def test_adv_syn01_provider_named_ssi_but_is_synthetic_false(client, db_session, monkeypatch):
    """
    ADV-SYN-01: Policy must depend on authoritative is_synthetic metadata, not string matching on 'ssi'.
    If an adapter named 'ssi' has is_synthetic=False, execution must be allowed by default.
    """
    monkeypatch.setattr(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", False)

    registry = ProviderRegistry.get_instance()
    real_ssi = FakeRealProviderAdapter(provider_id="ssi")
    registry.register_provider(real_ssi)

    prev_resp = client.post("/api/sync/preview", json={
        "symbol": "SSI",
        "start_date": "2026-08-03",
        "end_date": "2026-08-03",
        "provider_id": "ssi",
        "adjustment_type": "unadjusted"
    })
    sync_id = prev_resp.json()["sync_id"]
    checksum = prev_resp.json()["content_sha256"]

    exec_resp = client.post("/api/sync/execute", json={
        "sync_id": sync_id,
        "content_sha256": checksum
    })
    assert exec_resp.status_code == 200, "Should NOT deny based on string 'ssi' when is_synthetic=False"


# ==============================================================================
# ADV-SYN-02: Provider with is_synthetic=True and arbitrary name
# ==============================================================================
def test_adv_syn02_custom_named_synthetic_provider_denied_by_default(client, monkeypatch):
    """
    ADV-SYN-02: Any provider with is_synthetic=True must be denied by default,
    regardless of its provider_id string.
    """
    monkeypatch.setattr(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", False)

    registry = ProviderRegistry.get_instance()
    stub = FakeCustomStubAdapter(provider_id="arbitrary_custom_feed")
    registry.register_provider(stub)

    prev_resp = client.post("/api/sync/preview", json={
        "symbol": "CUSTOM",
        "start_date": "2026-08-03",
        "end_date": "2026-08-03",
        "provider_id": "arbitrary_custom_feed",
        "adjustment_type": "unadjusted"
    })
    sync_id = prev_resp.json()["sync_id"]
    checksum = prev_resp.json()["content_sha256"]

    exec_resp = client.post("/api/sync/execute", json={
        "sync_id": sync_id,
        "content_sha256": checksum
    })
    assert exec_resp.status_code == 403, "Any provider with is_synthetic=True must be denied"


# ==============================================================================
# ADV-SYN-03: Configuration default is fail-closed
# ==============================================================================
def test_adv_syn03_config_default_is_fail_closed():
    """
    ADV-SYN-03: Settings.SUMI_ALLOW_SYNTHETIC_PROVIDERS must default to False.
    """
    fresh_settings = Settings()
    assert fresh_settings.SUMI_ALLOW_SYNTHETIC_PROVIDERS is False, (
        "Settings.SUMI_ALLOW_SYNTHETIC_PROVIDERS must default to False"
    )


# ==============================================================================
# ADV-SYN-04 & ADV-SYN-07: Valid-looking OHLCV still denied & Weekly Aggregator not run
# ==============================================================================
def test_adv_syn04_and_syn07_valid_ohlcv_denied_and_weekly_aggregator_not_run(client, db_session, monkeypatch):
    """
    ADV-SYN-04 & ADV-SYN-07: Synthetic provider returning valid OHLCV is denied,
    and WeeklyAggregator.derive_weekly_candles is never executed on denied write.
    """
    monkeypatch.setattr(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", False)

    prev_resp = client.post("/api/sync/preview", json={
        "symbol": "VCB",
        "start_date": "2026-08-03",
        "end_date": "2026-08-07",
        "provider_id": "vnstock",
        "adjustment_type": "unadjusted"
    })
    sync_id = prev_resp.json()["sync_id"]
    checksum = prev_resp.json()["content_sha256"]

    with patch("app.services.weekly_aggregator.WeeklyAggregator.derive_weekly_candles") as spy_weekly:
        exec_resp = client.post("/api/sync/execute", json={
            "sync_id": sync_id,
            "content_sha256": checksum
        })
        assert exec_resp.status_code == 403
        spy_weekly.assert_not_called()


# ==============================================================================
# ADV-SYN-05: Denied operation followed by valid real-provider operation
# ==============================================================================
def test_adv_syn05_denied_operation_followed_by_valid_real_provider_operation(client, db_session, monkeypatch):
    """
    ADV-SYN-05: A denied synthetic execution is followed immediately by a valid
    real provider sync in the same application instance.
    Session remains completely usable; real provider persists canonical candles.
    """
    monkeypatch.setattr(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", False)

    # 1. Denied synthetic attempt
    prev_stub = client.post("/api/sync/preview", json={
        "symbol": "DENIED_SYM",
        "start_date": "2026-08-03",
        "end_date": "2026-08-07",
        "provider_id": "ssi",
        "adjustment_type": "unadjusted"
    }).json()

    exec_denied = client.post("/api/sync/execute", json={
        "sync_id": prev_stub["sync_id"],
        "content_sha256": prev_stub["content_sha256"]
    })
    assert exec_denied.status_code == 403
    assert db_session.query(Candle).filter(Candle.symbol == "DENIED_SYM").count() == 0

    # 2. Subsequent real provider operation
    registry = ProviderRegistry.get_instance()
    registry.register_provider(FakeRealProviderAdapter(provider_id="real_live_feed"))

    prev_real = client.post("/api/sync/preview", json={
        "symbol": "VALID_SYM",
        "start_date": "2026-08-03",
        "end_date": "2026-08-03",
        "provider_id": "real_live_feed",
        "adjustment_type": "unadjusted"
    }).json()

    exec_real = client.post("/api/sync/execute", json={
        "sync_id": prev_real["sync_id"],
        "content_sha256": prev_real["content_sha256"]
    })
    assert exec_real.status_code == 200
    assert exec_real.json()["status"] == "accepted"

    # Real candles committed with canonical provenance
    real_c = db_session.query(Candle).filter(Candle.symbol == "VALID_SYM").first()
    assert real_c is not None
    assert real_c.source == f"sync:{prev_real['sync_id']}"


# ==============================================================================
# ADV-SYN-06: Synthetic quarantine execution and rollback isolation
# ==============================================================================
def test_adv_syn06_synthetic_quarantine_execution_and_rollback_isolation(client, db_session, monkeypatch):
    """
    ADV-SYN-06: When synthetic execution is permitted, provenance is strictly quarantined:
    - 1D candles carry 'quarantine:sync:{id}'
    - 1W candles carry 'quarantine:VN_TRADING_WEEK_V1' (ADR-PROV-001)
    - Rollback cleanly purges quarantine candles without disturbing canonical candles.
    """
    monkeypatch.setattr(settings, "SUMI_ALLOW_SYNTHETIC_PROVIDERS", True)

    # Insert an existing canonical candle for another symbol/date
    canonical_c = Candle(
        symbol="CANONICAL_SYM",
        timeframe="1D",
        timestamp=datetime(2026, 8, 3, 0, 0),
        open=100.0, high=105.0, low=99.0, close=104.0, volume=50000.0,
        source="sync:real_prior_run",
        adjustment_type="unadjusted"
    )
    db_session.add(canonical_c)
    db_session.commit()

    # Execute synthetic sync
    prev_resp = client.post("/api/sync/preview", json={
        "symbol": "QUAR_SYM",
        "start_date": "2026-08-03",
        "end_date": "2026-08-07",
        "provider_id": "ssi",
        "adjustment_type": "unadjusted"
    }).json()
    sync_id = prev_resp["sync_id"]
    checksum = prev_resp["content_sha256"]

    exec_resp = client.post("/api/sync/execute", json={
        "sync_id": sync_id,
        "content_sha256": checksum
    })
    assert exec_resp.status_code == 200

    # Verify 1D quarantine provenance
    # Canonical reader sees 0
    assert len(db_session.query(Candle).filter(Candle.symbol == "QUAR_SYM", Candle.timeframe == "1D").all()) == 0

    # Authorized quarantine reader sees 5
    q_1d = db_session.query(Candle).execution_options(include_quarantine=True).filter(Candle.symbol == "QUAR_SYM", Candle.timeframe == "1D").all()
    assert len(q_1d) == 5
    for c in q_1d:
        assert c.source == f"quarantine:sync:{sync_id}"

    # Verify 1W quarantine provenance (ADR-PROV-001)
    # Canonical 1W reader sees 0
    assert len(db_session.query(Candle).filter(Candle.symbol == "QUAR_SYM", Candle.timeframe == "1W").all()) == 0

    # Authorized quarantine reader sees 1
    q_1w = db_session.query(Candle).execution_options(include_quarantine=True).filter(Candle.symbol == "QUAR_SYM", Candle.timeframe == "1W").all()
    assert len(q_1w) == 1
    assert q_1w[0].source == "quarantine:VN_TRADING_WEEK_V1"

    # Rollback the quarantine sync
    rb_resp = client.post("/api/sync/rollback", json={"sync_id": sync_id})
    assert rb_resp.status_code == 200
    assert rb_resp.json()["status"] == "rolled_back"

    # Verify quarantine candles are purged
    assert db_session.query(Candle).execution_options(include_quarantine=True).filter(Candle.symbol == "QUAR_SYM").count() == 0

    # Verify canonical candle was NOT disturbed
    assert db_session.query(Candle).filter(Candle.symbol == "CANONICAL_SYM").count() == 1


# ==============================================================================
# GAP-SEC-01: Unregistered provider in execute_sync must fail closed
# ==============================================================================
def test_unregistered_provider_in_execute_sync_fails_closed(client, db_session):
    """
    GAP-SEC-01: If a SyncRun has an unregistered or missing provider_id at execute time,
    it must fail closed (HTTP 400), not default to canonical production write.
    """
    # Create a SyncRun directly with an unregistered provider ID
    sync_run = SyncRun(
        id="ghost_sync_123",
        provider_id="non_existent_rogue_provider",
        symbol="GHOST",
        start_date="2026-08-03",
        end_date="2026-08-07",
        timeframe="1D",
        adjustment_type="unadjusted",
        status="previewed",
        content_sha256="ghosthash123",
        can_accept=True
    )
    db_session.add(sync_run)
    db_session.commit()

    exec_resp = client.post("/api/sync/execute", json={
        "sync_id": "ghost_sync_123",
        "content_sha256": "ghosthash123"
    })
    assert exec_resp.status_code == 400
    assert "không tồn tại" in exec_resp.json()["detail"]
