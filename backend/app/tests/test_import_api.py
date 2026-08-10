import pytest
from fastapi.testclient import TestClient

def test_import_preview_api(client: TestClient):
    csv_bytes = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nVNM,20260105,65.0,66.0,64.5,65.5,400000\nVNM,20260106,65.5,67.0,65.0,66.5,450000\n"
    files = {"file": ("VNM.csv", csv_bytes, "text/csv")}
    response = client.post("/api/import/preview", files=files, data={"adjustment_type": "unadjusted"})
    
    assert response.status_code == 200
    data = response.json()
    assert data["can_accept"] is True
    assert data["parsed_count"] == 2
    assert "run_id" in data
    assert "content_sha256" in data

def test_import_accept_api(client: TestClient):
    csv_bytes = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nVNM,20260105,65.0,66.0,64.5,65.5,400000\n"
    files = {"file": ("VNM.csv", csv_bytes, "text/csv")}
    preview_res = client.post("/api/import/preview", files=files, data={"adjustment_type": "unadjusted"}).json()
    
    run_id = preview_res["run_id"]
    checksum = preview_res["content_sha256"]
    
    accept_res = client.post(f"/api/import/runs/{run_id}/accept", json={"content_sha256": checksum})
    assert accept_res.status_code == 200
    accept_data = accept_res.json()
    assert accept_data["status"] == "accepted"
    assert accept_data["accepted_count"] == 1

def test_data_catalog_api(client: TestClient):
    res = client.get("/api/data/catalog")
    assert res.status_code == 200
    catalog = res.json()
    assert isinstance(catalog, list)

def test_import_history_api(client: TestClient):
    res = client.get("/api/import/runs")
    assert res.status_code == 200
    runs = res.json()
    assert isinstance(runs, list)

def test_import_rollback_api(client: TestClient):
    csv_bytes = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nTCB,20260105,22.0,23.0,21.5,22.5,800000\n"
    files = {"file": ("TCB.csv", csv_bytes, "text/csv")}
    preview_res = client.post("/api/import/preview", files=files).json()
    run_id = preview_res["run_id"]
    checksum = preview_res["content_sha256"]
    client.post(f"/api/import/runs/{run_id}/accept", json={"content_sha256": checksum})

    rollback_res = client.post(f"/api/import/runs/{run_id}/rollback")
    assert rollback_res.status_code == 200
    data = rollback_res.json()
    assert data["status"] == "rolled_back"

def test_import_preview_metadata_fail_closed(client: TestClient):
    csv_bytes = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nVNM,20260105,65.0,66.0,64.5,65.5,400000\n"
    files = {"file": ("VNM.csv", csv_bytes, "text/csv")}
    
    # 1. Invalid source_type
    res1 = client.post("/api/import/preview", files=files, data={"source_type": "unknown-provider"})
    assert res1.status_code == 200
    d1 = res1.json()
    assert d1["can_accept"] is False
    assert d1["status"] == "blocked"
    assert "Nguồn dữ liệu" in d1["block_reason"]

    # Attempting to accept blocked run fails with HTTP 400
    accept_attempt = client.post(f"/api/import/runs/{d1['run_id']}/accept", json={"content_sha256": d1["content_sha256"]})
    assert accept_attempt.status_code == 400

    # 2. Invalid timeframe
    res2 = client.post("/api/import/preview", files=files, data={"timeframe": "banana"})
    assert res2.json()["can_accept"] is False
    assert res2.json()["status"] == "blocked"

    # 3. Invalid adjustment_type
    res3 = client.post("/api/import/preview", files=files, data={"adjustment_type": "mystery"})
    assert res3.json()["can_accept"] is False
    assert res3.json()["status"] == "blocked"

    # 4. Invalid timezone_str
    res4 = client.post("/api/import/preview", files=files, data={"timezone_str": "Mars/Olympus_Mons"})
    assert res4.json()["can_accept"] is False
    assert res4.json()["status"] == "blocked"

def test_legacy_cafef_route_disabled(client: TestClient):
    csv_bytes = b"<Ticker>,<DTYYYYMMDD>,<Open>,<High>,<Low>,<Close>,<Volume>\nVNM,20260105,65.0,66.0,64.5,65.5,400000\n"
    files = {"file": ("VNM.csv", csv_bytes, "text/csv")}
    res = client.post("/api/import/cafef", files=files)
    assert res.status_code == 400
    assert "vô hiệu hóa" in res.json()["detail"]

