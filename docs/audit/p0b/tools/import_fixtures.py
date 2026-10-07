#!/usr/bin/env python3
"""
Phase 0b — Fixture Ingestion & Acceptance Tool
Uploads fixtures to isolated audit backend (port 18200) via /api/import/preview and /api/import/runs/{run_id}/accept.
Captures evidence for DF-01, DF-02, DF-03, DF-04.
"""

import os
import sys
import json
import urllib.request
import urllib.parse
import mimetypes
from pathlib import Path

os.environ["no_proxy"] = "*"
os.environ["NO_PROXY"] = "*"

BACKEND_URL = "http://127.0.0.1:18200"
ROOT = Path(__file__).resolve().parents[4]
DATA_DIR = ROOT / "test-results" / "p0b" / "data"
OUT_DIR = ROOT / "test-results" / "p0b"

def multipart_post(url: str, file_path: Path, fields: dict) -> dict:
    boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW"
    body = bytearray()
    
    # Add form fields
    for k, v in fields.items():
        body.extend(f"--{boundary}\r\n".encode())
        body.extend(f'Content-Disposition: form-data; name="{k}"\r\n\r\n'.encode())
        body.extend(f"{v}\r\n".encode())
        
    # Add file
    content = file_path.read_bytes()
    body.extend(f"--{boundary}\r\n".encode())
    body.extend(f'Content-Disposition: form-data; name="file"; filename="{file_path.name}"\r\n'.encode())
    body.extend(b"Content-Type: text/csv\r\n\r\n")
    body.extend(content)
    body.extend(b"\r\n")
    body.extend(f"--{boundary}--\r\n".encode())
    
    req = urllib.request.Request(url, data=body, method="POST")
    req.add_header("Content-Type", f"multipart/form-data; boundary={boundary}")
    
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.loads(resp.read().decode())

def json_post(url: str, payload: dict) -> dict:
    data = json.dumps(payload).encode()
    req = urllib.request.Request(url, data=data, method="POST")
    req.add_header("Content-Type", "application/json")
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.loads(resp.read().decode())

def json_get(url: str) -> dict:
    req = urllib.request.Request(url, method="GET")
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.loads(resp.read().decode())

def main():
    print(f"Connecting to audit backend at {BACKEND_URL}...")
    try:
        health = json_get(f"{BACKEND_URL}/api/health")
        print(f"Backend status: {health.get('status')}")
    except Exception as e:
        print(f"ERROR: Cannot connect to audit backend: {e}")
        print("Please start environment first: bash docs/audit/p0b/tools/start_env.sh --fresh")
        sys.exit(1)

    results = {}

    # 1. DF-01: Raw CafeF file (Expect out_of_order and cannot accept)
    raw_file = DATA_DIR / "raw_HSX_subset.csv"
    if raw_file.exists():
        print(f"\n[DF-01] Testing raw CafeF file: {raw_file.name}...")
        try:
            preview = multipart_post(
                f"{BACKEND_URL}/api/import/preview",
                raw_file,
                {"source_type": "cafef", "timeframe": "1D", "adjustment_type": "unadjusted"}
            )
            results["DF-01"] = {
                "file": raw_file.name,
                "parsed_count": preview.get("parsed_count"),
                "rejected_count": preview.get("rejected_count"),
                "can_accept": preview.get("can_accept"),
                "warnings": preview.get("warnings", [])[:5]
            }
            print(f"  DF-01 Result: parsed={preview.get('parsed_count')}, rejected={preview.get('rejected_count')}, can_accept={preview.get('can_accept')}")
        except Exception as e:
            results["DF-01"] = {"error": str(e)}
            print(f"  DF-01 Exception: {e}")

    # 2. DF-02: Clean Files Ingestion (INDEX, HSX, HNX)
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
        try:
            # Preview
            preview = multipart_post(
                f"{BACKEND_URL}/api/import/preview",
                fpath,
                {"source_type": "cafef", "timeframe": "1D", "adjustment_type": "unadjusted"}
            )
            run_id = preview.get("run_id")
            sha = preview.get("content_sha256")
            can_accept = preview.get("can_accept")
            parsed = preview.get("parsed_count")
            rejected = preview.get("rejected_count")
            print(f"  Preview: run_id={run_id}, parsed={parsed}, rejected={rejected}, can_accept={can_accept}")

            # Accept
            if can_accept and run_id and sha:
                accept_res = json_post(
                    f"{BACKEND_URL}/api/import/runs/{run_id}/accept",
                    {"content_sha256": sha}
                )
                accepted_count = accept_res.get("accepted_count")
                print(f"  Accept: status={accept_res.get('status')}, accepted_count={accepted_count}")
                results["DF-02"].append({
                    "file": fname,
                    "run_id": run_id,
                    "parsed": parsed,
                    "rejected": rejected,
                    "accepted": accepted_count,
                    "status": "ACCEPTED"
                })
            else:
                results["DF-02"].append({
                    "file": fname,
                    "run_id": run_id,
                    "can_accept": can_accept,
                    "status": "CANNOT_ACCEPT"
                })
        except Exception as e:
            print(f"  Error on {fname}: {e}")
            results["DF-02"].append({"file": fname, "error": str(e)})

    # 3. DF-03: Check 1W downsampled candles
    print("\n[DF-03] Checking weekly (1W) candle synthesis...")
    try:
        catalog = json_get(f"{BACKEND_URL}/api/import/catalog")
        fpt_entry = next((c for c in catalog if c.get("symbol") == "FPT"), None)
        vnindex_entry = next((c for c in catalog if c.get("symbol") == "VNINDEX"), None)
        results["DF-03"] = {
            "catalog_count": len(catalog),
            "FPT": fpt_entry,
            "VNINDEX": vnindex_entry
        }
        print(f"  Total catalog symbols: {len(catalog)}")
        if fpt_entry:
            print(f"  FPT coverage: 1D rows={fpt_entry.get('candle_count_1d')}, 1W rows={fpt_entry.get('candle_count_1w')}, range={fpt_entry.get('first_candle_date')} -> {fpt_entry.get('last_candle_date')}")
    except Exception as e:
        results["DF-03"] = {"error": str(e)}
        print(f"  DF-03 Error: {e}")

    # 4. DF-04: Conflict detection
    conflict_file = DATA_DIR / "CafeF.HSX.conflict.csv"
    if conflict_file.exists():
        print(f"\n[DF-04] Testing conflict detection: {conflict_file.name}...")
        try:
            preview = multipart_post(
                f"{BACKEND_URL}/api/import/preview",
                conflict_file,
                {"source_type": "cafef", "timeframe": "1D", "adjustment_type": "unadjusted"}
            )
            conflicts = preview.get("conflicting_count", 0)
            can_accept = preview.get("can_accept")
            results["DF-04"] = {
                "file": conflict_file.name,
                "conflicting_count": conflicts,
                "can_accept": can_accept
            }
            print(f"  Conflict detected: {conflicts} rows conflicting. can_accept={can_accept}")
        except Exception as e:
            results["DF-04"] = {"error": str(e)}
            print(f"  DF-04 Error: {e}")

    # Save output
    out_file = OUT_DIR / "import_fixtures_result.json"
    with open(out_file, "w", encoding="utf-8") as f:
        json.dump(results, f, indent=2, ensure_ascii=False)
    print(f"\nAll ingestion tests complete. Evidence written to: {out_file}")

if __name__ == "__main__":
    main()
