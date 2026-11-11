#!/usr/bin/env python3
"""Phase 0b — build audit fixture CSVs from CafeF "Upto" files.

Does NOT touch product code or backend/sumi.db. Output goes to test-results/p0b/data/.

Outputs
  raw_HSX_subset.csv        Original CafeF order (newest first), selected HOSE symbols, all dates.
                            Used by DF-01 to reproduce the real-file import behaviour.
  CafeF.HSX.clean.csv       Selected symbols, >= START_DATE, sorted ascending, OHLC normalised
  CafeF.HNX.clean.csv       (high = max(o,h,l,c), low = min(o,h,l,c)). Used to load the audit DB.
  CafeF.UPCOM.clean.csv
  CafeF.INDEX.clean.csv
  CafeF.HSX.conflict.csv    Same as HSX clean but FPT close on CONFLICT_DATE changed (+1.0). DF-04.
  fixture_report.json       Per-symbol row counts, date ranges, number of OHLC rows normalised.

Usage
  .venv/bin/python docs/audit/p0b/tools/prepare_fixture.py
"""
import csv
import json
import sys
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
SRC = ROOT / "docs" / "CafeF.SolieuGD.Upto05102026"
OUT = ROOT / "test-results" / "p0b" / "data"
START_DATE = "20180101"
CONFLICT_DATE = None  # picked automatically: first FPT date in 2024

SYMBOLS = {
    "HSX": ["FPT", "SSI", "HPG", "VCB", "MWG", "VNM", "TCB", "MBB", "ACB", "VCI"],
    "HNX": ["SHS", "PVS"],
    "UPCOM": ["ACV", "VEA"],
    "INDEX": ["VNINDEX", "HNX-INDEX"],
}
FILES = {
    "HSX": "CafeF.HSX.Upto05.10.2026.csv",
    "HNX": "CafeF.HNX.Upto05.10.2026.csv",
    "UPCOM": "CafeF.UPCOM.Upto05.10.2026.csv",
    "INDEX": "CafeF.INDEX.Upto05.10.2026.csv",
}
HEADER = ["<Ticker>", "<DTYYYYMMDD>", "<Open>", "<High>", "<Low>", "<Close>", "<Volume>"]


def read_rows(path: Path, wanted: set):
    rows = []
    with path.open(encoding="utf-8-sig", newline="") as f:
        reader = csv.reader(f)
        next(reader, None)
        for r in reader:
            if len(r) < 7:
                continue
            if r[0].strip() in wanted:
                rows.append([c.strip() for c in r[:7]])
    return rows


def write(path: Path, rows):
    with path.open("w", encoding="utf-8", newline="") as f:
        w = csv.writer(f)
        w.writerow(HEADER)
        w.writerows(rows)


def main():
    if not SRC.exists():
        sys.exit(f"Source folder not found: {SRC}")
    OUT.mkdir(parents=True, exist_ok=True)
    report = {"source": str(SRC), "start_date": START_DATE, "files": {}, "missing_symbols": []}

    for key, fname in FILES.items():
        wanted = set(SYMBOLS[key])
        rows = read_rows(SRC / fname, wanted)
        found = {r[0] for r in rows}
        report["missing_symbols"] += sorted(wanted - found)

        if key == "HSX":
            write(OUT / "raw_HSX_subset.csv", rows)  # original order, all dates

        clean, fixed, per_symbol = [], 0, {}
        for r in rows:
            if r[1] < START_DATE:
                continue
            try:
                # Check weekend
                dt = datetime.strptime(r[1], "%Y%m%d")
                if dt.weekday() >= 5:
                    continue
                o, h, l, c = map(float, r[2:6])
                vol = float(r[6])
                if o <= 0 or h <= 0 or l <= 0 or c <= 0 or vol < 0:
                    continue
            except Exception:
                continue
            nh, nl = max(o, h, l, c), min(o, h, l, c)
            if nh != h or nl != l:
                fixed += 1
            clean.append([r[0], r[1], r[2], repr(nh), repr(nl), r[5], r[6]])
        clean.sort(key=lambda x: (x[0], x[1]))
        for r in clean:
            s = per_symbol.setdefault(r[0], {"rows": 0, "first": r[1], "last": r[1]})
            s["rows"] += 1
            s["last"] = r[1]
        write(OUT / f"CafeF.{key}.clean.csv", clean)
        report["files"][key] = {"rows": len(clean), "ohlc_normalised_rows": fixed, "symbols": per_symbol}

        if key == "HSX":
            conflict = [list(r) for r in clean]
            fpt_2024 = [r for r in conflict if r[0] == "FPT" and r[1].startswith("2024")]
            if fpt_2024:
                target = fpt_2024[0]
                target[5] = repr(round(float(target[5]) + 1.0, 4))
                target[3] = repr(max(float(target[3]), float(target[5])))
                report["conflict_row"] = {"symbol": "FPT", "date": target[1], "new_close": target[5]}
            write(OUT / "CafeF.HSX.conflict.csv", conflict)

    (OUT / "fixture_report.json").write_text(json.dumps(report, indent=1))
    print(json.dumps({k: v for k, v in report.items() if k != "files"}, indent=1))
    for k, v in report["files"].items():
        print(f"{k}: rows={v['rows']} normalised={v['ohlc_normalised_rows']} symbols={len(v['symbols'])}")
    print(f"Output: {OUT}")


if __name__ == "__main__":
    main()
