# Independent Review Seal: Batch P7-MKT-02 — Market BB/Breadth Framework and Publication Gate

**Date**: 2026-09-19  
**Batch ID**: `P7-MKT-02`  
**Review Mechanism**: Independent Context (`critic_auditor`)  
**Status**: **ACCEPTED**

---

## 1. Executive Summary

Batch `P7-MKT-02` delivers the pure Market Aggregator and Breadth calculation engine implementing value-weighted aggregate-before-ratio money flow, independent flow breadth (count and value distributions), and publication guardrails (`Master G.4 / Section 26`, `BB v3 08`, `BBI-SIG-001/003`, `SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` lines 447–460, 563, 629, 655, 711–736).

### Key Architectural Accomplishments
1. **Aggregate-Before-Ratio Identity (`BB-MKT-001` / `AT09`)**:
   - Implemented value decomposition for each bar: $B_i = \frac{1+P_i}{2} V_i$, $S_i = \frac{1-P_i}{2} V_i$.
   - Aggregated constituent active buy and sell values across eligible universe members before computing rolling horizon ratio $\text{Market BB} = 100 \times \frac{\sum B_i}{\sum B_i + \sum S_i}$.
2. **Anti-Averaging Principle (`BB-MKT-002` / `AT10`)**:
   - Proved mathematically and programmatically that unweighted score averaging `mean(Symbol BB)` severely distorts whole-market money flow.
   - Tested concrete counterexample ($V_A=100M, BB_A=90$; $V_B=10M, BB_B=10$) where arithmetic mean is $50.0$ while true value-weighted Market BB is $82.73$.
3. **Universe Point-in-Time Boundary Enforcement (`BB-MKT-003` / `AT11`)**:
   - Constituents contribute only during active eligibility intervals (`effective_from <= session_date <= effective_to`). Delisted symbols stop contributing; newly listed symbols start only on listing date.
4. **Independent Flow Breadth (`BB-MKT-004` / `AT17`)**:
   - Measures member participation distributions across both count and value partitions (positive, neutral, negative) independently of the aggregate Market BB score.
   - Count ratios and value ratios strictly sum to $1.0$ within floating tolerance.
5. **Publication Guardrails (`BB-MKT-005`)**:
   - Blocks `CANONICAL_PUBLISHED` on candidate universes or retrospective fixed mode, setting `RESEARCH_RETROSPECTIVE` with explicit survivorship caveats.
   - Blocks calculation and publication with `UNAVAILABLE_DEGRADED` if constituent coverage falls below 50%.
6. **Zero Database Mutation & Preserved Deletions**:
   - Pure domain computation in memory with zero SQLite table modifications in `sumi.db`.
   - All 159 historically staged deletions preserved intact.

---

## 2. Invariant & Acceptance Verification

| Acceptance Invariant | Verification Target | Result | Evidence |
|---|---|---|---|
| `BB-MKT-001` / `AT09` (Aggregate-before-ratio identity) | `backend/app/tests/test_market_bb.py` | **PASS** | `test_at09_aggregate_identity` validates multi-constituent sum ratios match proxy formula. |
| `BB-MKT-002` / `AT10` (No score averaging) | `backend/app/tests/test_market_bb.py` | **PASS** | `test_at10_no_score_averaging_counterexample` proves arithmetic mean (50.0) != true Market BB (82.73). |
| `BB-MKT-003` / `AT11` (Universe PIT boundary) | `backend/app/tests/test_market_bb.py` | **PASS** | `test_at11_universe_pit_boundary` tests addition and delisting transitions on exact calendar boundaries. |
| `BB-MKT-004` / `AT17` (Flow Breadth consistency) | `backend/app/tests/test_market_bb.py` | **PASS** | `test_at17_breadth_consistency_and_sums` validates count and value ratio partitions sum to 1.0. |
| `BB-MKT-005` (Publication Gate enforcement) | `backend/app/tests/test_market_bb.py` | **PASS** | `test_publication_gate_rules` verifies CANDIDATE, RETROSPECTIVE, and degraded coverage behavior. |
| `AT12` (Missing data drops coverage) | `backend/app/tests/test_market_bb.py` | **PASS** | `test_at12_missing_data_drops_coverage` ensures missing vendor bars do not inject zero values. |
| `AT05` / `AT08` (Zero activity & warmup) | `backend/app/tests/test_market_bb.py` | **PASS** | `test_at05_at08_zero_activity_and_warmup` confirms null scores during warmup and zero activity. |
| Read-Only API Contract | `/api/bb/market` routes | **PASS** | `test_market_bb_api_endpoints` validates GET `/api/bb/market/{universe_id}` and POST `/api/bb/market/calculate`. |
| Zero Database Mutation | SHA256 of `backend/sumi.db` | **PASS** | Hash strictly matches baseline: `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`. |
| Preserved Staged Deletions | `git status --short` | **PASS** | Exactly 159 historically staged deletions preserved untouched. |

---

## 3. Automated Test Evidence

### Backend Focused Suite
```powershell
python -m pytest backend/app/tests/test_market_bb.py -v
```
- **Result**: 8 passed in 0.24s.

### Fast Technical Gate (`.\scripts\verify-v2.ps1`)
- **Backend Tests**: 338 passed (0 failed).
- **Alembic Migrations**: Upgraded successfully on temporary database.
- **Frontend Linter**: Clean (`eslint .` passed).
- **Frontend Tests**: 32 test files, 210 passed (0 failed).
- **Frontend Production Build**: `tsc -b && vite build` built successfully in 662ms.

### Comprehensive Browser E2E UAT (`.\scripts\run-comprehensive-uat.ps1`)
- **Total Scenarios**: 31/31 passed (100.0%).
- **Runtime Console Errors**: 0 errors (`TC-SYS-04: PASS`).
- **Database Immutability**: Verified (`TC-SYS-05: PASS`).

---

## 4. Reviewer Seal & Decision

Batch `P7-MKT-02` fulfills all functional and technical acceptance requirements specified in `Master G.4`, `Section 26`, and `SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md`.

**DECISION: ACCEPTED AND SEALED**

**RECOMMENDED NEXT TASK**: `P3-VSA-02` (Volume Spread Analysis and causal support/resistance) per dependency readiness in `STATE.json`.
