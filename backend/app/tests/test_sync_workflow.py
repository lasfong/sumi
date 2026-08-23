import pytest
from datetime import date, datetime, timezone
from app.models.candle import Candle
from app.models.symbol import Symbol
from app.models.sync_run import SyncRun, SyncRunItem, SyncRunMutation
from app.models.import_run import WeeklyCandleProvenance
from app.services.data_providers.provider_registry import ProviderRegistry
from app.services.data_providers.base_provider import (
    ProviderCandleDTO,
    ProviderAuthError,
    ProviderRateLimitError,
    ProviderNetworkError,
    ProviderDataError,
)
from app.services.sync_workflow_service import SyncWorkflowService
from app.schemas.sync_schema import SyncPreviewRequest

def test_provider_registry_and_metadata():
    registry = ProviderRegistry.get_instance()
    providers = registry.list_providers()
    assert len(providers) >= 2
    
    ssi = registry.get_provider("ssi")
    assert ssi is not None
    ssi_meta = ssi.get_metadata()
    assert ssi_meta.provider_id == "ssi"
    assert ssi_meta.is_official is True
    assert ssi_meta.requires_auth is True
    assert "1D" in ssi_meta.supported_timeframes
    assert "consumer_id" in ssi_meta.auth_fields

    vnstock = registry.get_provider("vnstock")
    assert vnstock is not None
    vnstock_meta = vnstock.get_metadata()
    assert vnstock_meta.provider_id == "vnstock"
    assert vnstock_meta.is_official is False
    assert vnstock_meta.requires_auth is False


def test_ssi_and_vnstock_adapters_normalization():
    registry = ProviderRegistry.get_instance()
    ssi = registry.get_provider("ssi")
    
    start_d = date(2026, 8, 3)   # Monday
    end_d = date(2026, 8, 7)     # Friday
    candles = ssi.fetch_daily_candles("VNM", start_d, end_d, "unadjusted")
    assert len(candles) == 5  # 5 trading days
    for c in candles:
        assert isinstance(c, ProviderCandleDTO)
        assert c.symbol == "VNM"
        assert c.timeframe == "1D"
        assert c.adjustment_type == "unadjusted"
        assert c.open > 0
        assert c.high >= max(c.open, c.close)
        assert c.low <= min(c.open, c.close)
        assert c.volume > 0

    # Benchmark fetch
    bench_candles = ssi.fetch_benchmark_indices("VNINDEX", start_d, end_d)
    assert len(bench_candles) == 5
    assert bench_candles[0].symbol == "VNINDEX"


def test_provider_error_handling_and_fail_closed():
    registry = ProviderRegistry.get_instance()
    ssi = registry.get_provider("ssi")
    
    # Invalid date range
    with pytest.raises(ProviderDataError):
        ssi.fetch_daily_candles("VNM", date(2026, 8, 10), date(2026, 8, 1))

    # Error simulation triggers
    with pytest.raises(ProviderRateLimitError):
        ssi.fetch_daily_candles("ERROR_RATE_LIMIT", date(2026, 8, 1), date(2026, 8, 5))

    with pytest.raises(ProviderAuthError):
        ssi.fetch_daily_candles("ERROR_AUTH", date(2026, 8, 1), date(2026, 8, 5))

    with pytest.raises(ProviderNetworkError):
        ssi.fetch_daily_candles("ERROR_NETWORK", date(2026, 8, 1), date(2026, 8, 5))


def test_test_connection_endpoint(client):
    # SSI test connection valid
    resp = client.post("/api/sync/test-connection", json={
        "provider_id": "ssi",
        "credentials": {"consumer_id": "valid_user", "consumer_secret": "secret_key"}
    })
    assert resp.status_code == 200
    data = resp.json()
    assert data["success"] is True
    assert "Kết nối thành công" in data["message"]
    assert data["latency_ms"] is not None

    # SSI test connection auth failure
    resp_fail = client.post("/api/sync/test-connection", json={
        "provider_id": "ssi",
        "credentials": {"consumer_id": "invalid", "consumer_secret": "invalid"}
    })
    assert resp_fail.status_code == 200
    data_fail = resp_fail.json()
    assert data_fail["success"] is False
    assert "Lỗi xác thực" in data_fail["message"]

    # Unknown provider
    resp_unknown = client.post("/api/sync/test-connection", json={
        "provider_id": "unknown_broker"
    })
    assert resp_unknown.status_code == 404


def test_sync_preview_creation(client, db_session):
    # Generate preview for VNM 2026-08-03 to 2026-08-07
    req_payload = {
        "symbol": "VNM",
        "start_date": "2026-08-03",
        "end_date": "2026-08-07",
        "provider_id": "ssi",
        "adjustment_type": "unadjusted"
    }
    resp = client.post("/api/sync/preview", json=req_payload)
    assert resp.status_code == 200
    data = resp.json()
    assert data["symbol"] == "VNM"
    assert data["status"] == "previewed"
    assert data["can_accept"] is True
    assert data["parsed_count"] == 5
    assert data["duplicate_count"] == 0
    assert data["conflicting_count"] == 0
    assert len(data["items"]) == 5
    assert len(data["content_sha256"]) == 64

    # Ensure no candles were committed to the Candle table during preview
    candles_count = db_session.query(Candle).filter(Candle.symbol == "VNM").count()
    assert candles_count == 0


def test_sync_execute_atomic_and_weekly_derivation(client, db_session):
    # 1. Preview
    req_payload = {
        "symbol": "FPT",
        "start_date": "2026-08-03",
        "end_date": "2026-08-07",
        "provider_id": "ssi",
        "adjustment_type": "unadjusted"
    }
    prev_resp = client.post("/api/sync/preview", json=req_payload)
    prev_data = prev_resp.json()
    sync_id = prev_data["sync_id"]
    content_sha256 = prev_data["content_sha256"]

    # 2. Execute
    exec_resp = client.post("/api/sync/execute", json={
        "sync_id": sync_id,
        "content_sha256": content_sha256
    })
    assert exec_resp.status_code == 200
    exec_data = exec_resp.json()
    assert exec_data["status"] == "accepted"
    assert exec_data["accepted_count"] == 5
    assert exec_data["symbol"] == "FPT"
    assert exec_data["duration_ms"] >= 0

    # Verify audit manifest in response
    manifest = exec_data["manifest"]
    assert manifest is not None
    assert manifest["audit_version"] == "SUMI_SYNC_MANIFEST_V1"
    assert manifest["symbol"] == "FPT"
    assert manifest["counts"]["accepted"] == 5

    # Verify 1D candles committed
    daily_candles = db_session.query(Candle).filter(Candle.symbol == "FPT", Candle.timeframe == "1D").all()
    assert len(daily_candles) == 5
    for c in daily_candles:
        assert c.source == f"sync:{sync_id}"

    # Verify 1W candles derived automatically by WeeklyAggregator
    weekly_candles = db_session.query(Candle).filter(Candle.symbol == "FPT", Candle.timeframe == "1W").all()
    assert len(weekly_candles) == 1
    assert weekly_candles[0].symbol == "FPT"

    # Verify WeeklyCandleProvenance record created
    prov = db_session.query(WeeklyCandleProvenance).filter(WeeklyCandleProvenance.symbol == "FPT").first()
    assert prov is not None
    assert prov.rule_version == "VN_TRADING_WEEK_V1"


def test_sync_execute_idempotent(client, db_session):
    # Preview and execute
    req_payload = {
        "symbol": "HPG",
        "start_date": "2026-08-03",
        "end_date": "2026-08-07",
        "provider_id": "ssi",
        "adjustment_type": "unadjusted"
    }
    prev_resp = client.post("/api/sync/preview", json=req_payload)
    sync_id = prev_resp.json()["sync_id"]
    checksum = prev_resp.json()["content_sha256"]

    # First execute -> accepted
    exec_resp1 = client.post("/api/sync/execute", json={"sync_id": sync_id, "content_sha256": checksum})
    assert exec_resp1.status_code == 200
    assert exec_resp1.json()["status"] == "accepted"

    # Repeat execute with same checksum -> idempotent noop
    exec_resp2 = client.post("/api/sync/execute", json={"sync_id": sync_id, "content_sha256": checksum})
    assert exec_resp2.status_code == 200
    assert exec_resp2.json()["status"] == "noop"

    # Repeat execute with tampered checksum -> 400 error
    exec_resp3 = client.post("/api/sync/execute", json={"sync_id": sync_id, "content_sha256": "tampered_checksum"})
    assert exec_resp3.status_code == 400


def test_sync_execute_stale_preview_fail_closed(client, db_session):
    # Preview
    req_payload = {
        "symbol": "SSI",
        "start_date": "2026-08-03",
        "end_date": "2026-08-07",
        "provider_id": "ssi",
        "adjustment_type": "unadjusted"
    }
    prev_resp = client.post("/api/sync/preview", json=req_payload)
    sync_id = prev_resp.json()["sync_id"]
    checksum = prev_resp.json()["content_sha256"]

    # In between preview and execute, simulate an external insert of a candle on 2026-08-05
    stale_candle = Candle(
        symbol="SSI",
        timeframe="1D",
        timestamp=datetime(2026, 8, 5, 0, 0, tzinfo=timezone.utc),
        open=34000.0,
        high=35000.0,
        low=33500.0,
        close=34500.0,
        volume=1000000.0,
        source="manual",
        adjustment_type="unadjusted"
    )
    db_session.add(stale_candle)
    db_session.commit()

    # Execute must fail closed due to stale preview
    exec_resp = client.post("/api/sync/execute", json={"sync_id": sync_id, "content_sha256": checksum})
    assert exec_resp.status_code == 400
    assert "Bản xem trước đã hết hạn" in exec_resp.json()["detail"]

    # Verify sync_run status updated to blocked
    run = db_session.query(SyncRun).filter(SyncRun.id == sync_id).first()
    assert run.status == "blocked"
    assert run.can_accept is False


def test_sync_rollback_success(client, db_session):
    # 1. Sync MWG
    req_payload = {
        "symbol": "MWG",
        "start_date": "2026-08-03",
        "end_date": "2026-08-07",
        "provider_id": "ssi",
        "adjustment_type": "unadjusted"
    }
    prev_resp = client.post("/api/sync/preview", json=req_payload)
    sync_id = prev_resp.json()["sync_id"]
    checksum = prev_resp.json()["content_sha256"]
    client.post("/api/sync/execute", json={"sync_id": sync_id, "content_sha256": checksum})

    # Verify candles exist
    assert db_session.query(Candle).filter(Candle.symbol == "MWG", Candle.timeframe == "1D").count() == 5
    assert db_session.query(Candle).filter(Candle.symbol == "MWG", Candle.timeframe == "1W").count() == 1

    # 2. Rollback
    rb_resp = client.post("/api/sync/rollback", json={"sync_id": sync_id})
    assert rb_resp.status_code == 200
    rb_data = rb_resp.json()
    assert rb_data["status"] == "rolled_back"
    assert rb_data["restored_mutations_count"] == 5

    # Verify candles removed and weekly series cleared
    assert db_session.query(Candle).filter(Candle.symbol == "MWG", Candle.timeframe == "1D").count() == 0
    assert db_session.query(Candle).filter(Candle.symbol == "MWG", Candle.timeframe == "1W").count() == 0

    # Verify sync_run record updated
    run = db_session.query(SyncRun).filter(SyncRun.id == sync_id).first()
    assert run.status == "rolled_back"
    assert run.rolled_back_at is not None


def test_sync_rollback_overlap_safety(client, db_session):
    # 1. First sync VCB 2026-08-03 to 2026-08-05
    p1 = client.post("/api/sync/preview", json={
        "symbol": "VCB",
        "start_date": "2026-08-03",
        "end_date": "2026-08-05",
        "provider_id": "ssi",
        "adjustment_type": "unadjusted"
    }).json()
    client.post("/api/sync/execute", json={"sync_id": p1["sync_id"], "content_sha256": p1["content_sha256"]})

    # 2. Second sync VCB 2026-08-06 to 2026-08-07
    p2 = client.post("/api/sync/preview", json={
        "symbol": "VCB",
        "start_date": "2026-08-06",
        "end_date": "2026-08-07",
        "provider_id": "ssi",
        "adjustment_type": "unadjusted"
    }).json()
    client.post("/api/sync/execute", json={"sync_id": p2["sync_id"], "content_sha256": p2["content_sha256"]})

    # 3. Attempting to rollback the earlier sync (p1) must be rejected because a newer sync (p2) touched VCB
    rb_fail = client.post("/api/sync/rollback", json={"sync_id": p1["sync_id"]})
    assert rb_fail.status_code == 400
    assert "Hoàn tác bị từ chối" in rb_fail.json()["detail"]


def test_sync_history_and_manifest_endpoints(client, db_session):
    # Sync TCB
    p = client.post("/api/sync/preview", json={
        "symbol": "TCB",
        "start_date": "2026-08-03",
        "end_date": "2026-08-07",
        "provider_id": "ssi",
        "adjustment_type": "unadjusted"
    }).json()
    client.post("/api/sync/execute", json={"sync_id": p["sync_id"], "content_sha256": p["content_sha256"]})

    # List history
    hist_resp = client.get("/api/sync/history")
    assert hist_resp.status_code == 200
    hist = hist_resp.json()
    assert len(hist) >= 1
    tcb_item = next((item for item in hist if item["symbol"] == "TCB"), None)
    assert tcb_item is not None
    assert tcb_item["status"] == "accepted"
    assert tcb_item["accepted_count"] == 5

    # Query single manifest endpoint
    man_resp = client.get(f"/api/sync/{p['sync_id']}/manifest")
    assert man_resp.status_code == 200
    man = man_resp.json()
    assert man["sync_id"] == p["sync_id"]
    assert man["manifest"]["audit_version"] == "SUMI_SYNC_MANIFEST_V1"
