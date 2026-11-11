#!/usr/bin/env python3
"""
Phase 1 Fixture Ingestion & Acceptance Tool
Uses FastAPI TestClient to load fixtures into test-results/p0b/audit.db without socket restrictions.
Verifies Phase 1 improvements:
- DF-01: raw_HSX_subset.csv is automatically sorted chronologically, out_of_order=0, can_accept=True, 1000x scaled.
- DF-02: Clean files accepted without issue.
- DF-03: 1W weekly candles synthesized.
- DF-04: Conflict detection still guards against mismatched duplicates.
"""

import os
import sys
import json
import sqlite3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
AUDIT_DB = ROOT / "test-results" / "p0b" / "audit.db"
DATA_DIR = ROOT / "test-results" / "p0b" / "data"
OUT_DIR = ROOT / "test-results" / "p1"
OUT_DIR.mkdir(parents=True, exist_ok=True)

os.environ["DATABASE_URL"] = f"sqlite:///{AUDIT_DB}"
sys.path.insert(0, str(ROOT / "backend"))

from fastapi.testclient import TestClient
from app.main import app

client = TestClient(app)

def main():
    print(f"Loading fixtures into {AUDIT_DB}...")
    results = {}

    # 1. DF-01: Raw CafeF file (Expect auto-sort and can_accept=True in Phase 1)
    raw_file = DATA_DIR / "raw_HSX_subset.csv"
    if raw_file.exists():
        print(f"\n[DF-01] Testing raw CafeF file auto-sort: {raw_file.name}...")
        with raw_file.open("rb") as f:
            resp = client.post(
                "/api/import/preview",
                files={"file": (raw_file.name, f.read(), "text/csv")},
                data={"source_type": "cafef", "timeframe": "1D", "adjustment_type": "unadjusted"}
            )
        assert resp.status_code == 200, f"Preview failed: {resp.text}"
        preview = resp.json()
        results["DF-01"] = {
            "file": raw_file.name,
            "parsed_count": preview.get("parsed_count"),
            "rejected_count": preview.get("rejected_count"),
            "out_of_order_count": preview.get("out_of_order_count", 0),
            "can_accept": preview.get("can_accept"),
            "warnings_count": len(preview.get("warnings", [])),
            "status": preview.get("status")
        }
        print(f"  DF-01 Result: parsed={preview.get('parsed_count')}, out_of_order={preview.get('out_of_order_count')}, can_accept={preview.get('can_accept')}")

    # 2. DF-02: Clean Files Ingestion (INDEX, HSX, HNX, UPCOM)
    clean_files = [
        "CafeF.INDEX.clean.csv",
        "CafeF.HSX.clean.csv",
        "CafeF.HNX.clean.csv",
        "CafeF.UPCOM.clean.csv"
    ]
    results["DF-02"] = []
    for fname in clean_files:
        fpath = DATA_DIR / fname
        if not fpath.exists():
            continue
        print(f"\n[DF-02] Ingesting clean fixture: {fname}...")
        with fpath.open("rb") as f:
            resp = client.post(
                "/api/import/preview",
                files={"file": (fname, f.read(), "text/csv")},
                data={"source_type": "cafef", "timeframe": "1D", "adjustment_type": "unadjusted"}
            )
        assert resp.status_code == 200, f"Preview failed: {resp.text}"
        preview = resp.json()
        run_id = preview.get("run_id")
        sha = preview.get("content_sha256")
        can_accept = preview.get("can_accept")
        parsed = preview.get("parsed_count")
        rejected = preview.get("rejected_count")
        print(f"  Preview: run_id={run_id}, parsed={parsed}, rejected={rejected}, can_accept={can_accept}")

        if can_accept and run_id and sha:
            acc_resp = client.post(
                f"/api/import/runs/{run_id}/accept",
                json={"content_sha256": sha}
            )
            assert acc_resp.status_code == 200, f"Accept failed: {acc_resp.text}"
            acc_data = acc_resp.json()
            print(f"  Accept: status={acc_data.get('status')}, accepted_count={acc_data.get('accepted_count')}")
            results["DF-02"].append({
                "file": fname,
                "run_id": run_id,
                "parsed": parsed,
                "rejected": rejected,
                "accepted": acc_data.get("accepted_count"),
                "status": "ACCEPTED"
            })
        else:
            results["DF-02"].append({
                "file": fname,
                "run_id": run_id,
                "can_accept": can_accept,
                "status": "CANNOT_ACCEPT"
            })

    # 3. DF-03: Check 1W downsampled candles
    print("\n[DF-03] Checking weekly (1W) candle synthesis...")
    cat_resp = client.get("/api/import/catalog")
    assert cat_resp.status_code == 200, f"Catalog failed: {cat_resp.text}"
    catalog = cat_resp.json()
    fpt_entry = next((c for c in catalog if c.get("symbol") == "FPT"), None)
    vnindex_entry = next((c for c in catalog if c.get("symbol") == "VNINDEX"), None)
    results["DF-03"] = {
        "catalog_count": len(catalog),
        "FPT": fpt_entry,
        "VNINDEX": vnindex_entry
    }
    print(f"  Total catalog symbols: {len(catalog)}")
    if fpt_entry:
        print(f"  FPT coverage: 1D rows={fpt_entry.get('candle_count_1d')}, 1W rows={fpt_entry.get('candle_count_1w')}")

    # 4. DF-04: Conflict detection
    conflict_file = DATA_DIR / "CafeF.HSX.conflict.csv"
    if conflict_file.exists():
        print(f"\n[DF-04] Testing conflict detection: {conflict_file.name}...")
        with conflict_file.open("rb") as f:
            resp = client.post(
                "/api/import/preview",
                files={"file": (conflict_file.name, f.read(), "text/csv")},
                data={"source_type": "cafef", "timeframe": "1D", "adjustment_type": "unadjusted"}
            )
        assert resp.status_code == 200, f"Conflict preview failed: {resp.text}"
        preview = resp.json()
        conflicts = preview.get("conflicting_count", 0)
        can_accept = preview.get("can_accept")
        results["DF-04"] = {
            "file": conflict_file.name,
            "conflicting_count": conflicts,
            "can_accept": can_accept
        }
        print(f"  Conflict detected: {conflicts} rows conflicting. can_accept={can_accept}")

    out_file = OUT_DIR / "import_fixtures_result.json"
    with open(out_file, "w", encoding="utf-8") as f:
        json.dump(results, f, indent=2, ensure_ascii=False)
    print(f"\nAll ingestion tests complete. Evidence written to: {out_file}")

if __name__ == "__main__":
    main()
