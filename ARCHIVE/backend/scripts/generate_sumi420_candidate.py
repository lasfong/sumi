"""Generate SUMI-420 candidate v1 universe definition artifact from Doraemon audited active list."""

import csv
import json
from pathlib import Path


def generate_candidate():
    watch_list_path = Path("E:/Workspace/Doraemon/src/vn_fin_dw/seeds/watch_list.csv")
    if not watch_list_path.exists():
        raise FileNotFoundError(f"Watch list not found at {watch_list_path}")

    with open(watch_list_path, "r", encoding="utf-8-sig") as f:
        rows = list(csv.DictReader(f))

    members = []
    for i, r in enumerate(rows):
        sym = r["symbol"].strip().upper()
        is_active = i < 420
        sector_group = r.get("sector_group", "")
        members.append({
            "symbol": sym,
            "exchange": "VN_EQUITY",
            "company_name": None,
            "sector": r.get("sector"),
            "effective_from": "2026-09-11",
            "effective_to": None,
            "status": "ACTIVE" if is_active else "RESERVE",
            "inclusion_reason": f"Core candidate ({sector_group})" if is_active else f"Reserve candidate buffer ({sector_group})",
            "weight": 1.0,
            "metadata": {
                "sector_group": sector_group,
                "cashflow_group": r.get("cashflow_group"),
                "source": r.get("source"),
                "priority_rank": i + 1,
            }
        })

    payload = {
        "universe_id": "SUMI-420",
        "version": "v1_candidate",
        "name": "SUMI-420 Curated Liquid Vietnam Equity Universe (Candidate v1)",
        "status": "CANDIDATE",
        "default_mode": "RETROSPECTIVE_FIXED",
        "effective_from": "2026-09-11",
        "effective_to": None,
        "selection_purpose": "Curated ~420 liquid Vietnam equity universe for technical analysis, strategy backtesting, and Blackbox Money Flow aggregation.",
        "selection_rules": [
            "Audited active equity in Doraemon data warehouse as of 2026-09-11 (watch_list.csv)",
            "Valid 3-letter uppercase ticker format",
            "Tier 1 & Tier 2 priority allocation: 420 core active members, 50 reserve candidate members",
            "Candidate status: evaluation prior to 2026-09-11 is RETROSPECTIVE_FIXED with explicit survivorship bias warning",
            "Canonical promotion to SUMI420_v1 requires owner sign-off and episode-backed historical constituent intervals"
        ],
        "source_evidence": "Doraemon audited active equity list 2026-09-11 (watch_list.csv, 470 total symbols)",
        "metadata": {
            "total_audited_count": 470,
            "active_candidate_count": 420,
            "reserve_candidate_count": 50,
            "as_of_audit_date": "2026-09-11"
        },
        "members": members
    }

    out_path = Path("backend/app/domain/universe/definitions/sumi420_v1_candidate.json")
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(payload, f, indent=2, ensure_ascii=False)

    print(f"Generated {out_path} with {len(members)} total members ({sum(1 for m in members if m['status'] == 'ACTIVE')} active).")


if __name__ == "__main__":
    generate_candidate()
