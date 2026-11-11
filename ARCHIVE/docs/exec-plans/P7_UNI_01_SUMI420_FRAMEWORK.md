# P7-UNI-01 — Versioned SUMI-420 framework and candidate v1

## Outcome
Establish a portable managed universe architecture in Sumi that prevents retrospective membership from masquerading as point-in-time (PIT) truth (`Master G.4 / Section 26`). Deliver the immutable `SUMI420_v1_candidate` membership artifact (~420 securities derived from Doraemon's audited active list as of 2026-09-11), pure resolver with explicit survivor bias caveats for retrospective dates, coverage reporting, and read-only API contracts.

## Context and problem
- **Canonical References**: `Master G.4`, `Section 26`, `SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` lines 433–445, 563, 610, 764.
- **Problem**: In quantitative backtesting and whole-market/breadth indicators, using current surviving constituents across historical dates causes severe survivorship and lookahead bias. Sumi needs an authoritative, portable, versioned universe contract that enforces point-in-time boundaries and flags retrospective evaluation explicitly.
- **Acceptance IDs**: `UNI-PIT-001` (point-in-time vs retrospective mode enforcement), `UNI-COV-002` (coverage calculation and status thresholds), `UNI-420-003` (candidate v1 artifact validation and immutability hash).

## In scope
- Pure domain models in `backend/app/domain/universe/models.py`: `UniverseMode`, `UniverseStatus`, `MemberStatus`, `UniverseMember`, `UniverseDefinition`, `UniverseResolution`, `UniverseCoverageReport`.
- Pure resolver in `backend/app/domain/universe/resolver.py` supporting `POINT_IN_TIME` and `RETROSPECTIVE_FIXED` modes with strict interval filtering and survivor bias caveats.
- Universe registry in `backend/app/domain/universe/registry.py` loading immutable JSON artifacts.
- Immutable definitions:
  - `backend/app/domain/universe/definitions/sumi420_v1_candidate.json`: 420 active liquid VN equity constituents + 50 reserve constituents derived from Doraemon audited active list as of 2026-09-11.
  - `backend/app/domain/universe/definitions/fixtures/pit_test_universe.json`: deterministic test fixture with active additions and delistings across 2024–2026.
- Read-only schemas (`backend/app/schemas/universe_schema.py`) and service (`backend/app/services/universe_service.py`).
- API router (`backend/app/api/universe.py`) mounted at `/api/universe`.
- Comprehensive unit and integration test suite (`backend/app/tests/test_universe.py`).

## Out of scope
- Mutating or adding tables to `backend/sumi.db` (Doraemon is the canonical store for universe episodes).
- Market BB and Breadth calculation formulas (reserved for `P7-MKT-02`).
- Promotion of `SUMI420_v1_candidate` to canonical `SUMI420_v1` without owner approval and historical constituent episodes.

## Invariants
- `backend/sumi.db` SHA-256 hash `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` must remain untouched.
- Single production writer on branch `master`.
- No historical staged deletions touched.
- Point-in-time queries prior to `effective_from` must refuse backfilling or clearly report out-of-interval.
- Retrospective mode must carry an unmistakable survivor bias caveat in API outputs.

## Current architecture
- Sumi currently has a simple `symbols` table in SQLite with 3,734 historical raw tickers, but no universe versioning, no effective intervals, and no point-in-time awareness.
- Doraemon has 470 audited active equities in `watch_list.csv` as of 2026-09-11.

## Target design
- Pure domain models under `backend/app/domain/universe/`.
- File-based versioned definitions stored as JSON in `backend/app/domain/universe/definitions/`.
- Deterministic content hashing: SHA-256 computed across sorted canonical member records.
- Resolver enforces `POINT_IN_TIME` vs `RETROSPECTIVE_FIXED`.
- Read-only FastAPI routes under `/api/universe`.

## Milestones
1. **Domain Models & Resolver**: Implement `models.py`, `resolver.py`, `registry.py` with invariant hashing and date-filtering.
2. **Artifacts Creation**: Build `sumi420_v1_candidate.json` (420 core + 50 reserve) and `pit_test_universe.json`.
3. **Schemas, Service & API**: Expose `/api/universe/list`, `/api/universe/{id}/definition`, `/api/universe/{id}/resolve`, `/api/universe/{id}/coverage`.
4. **Test Suite & Invariant Verification**: Author `test_universe.py` covering all edge cases, hash stability, PIT vs retrospective, coverage calculation, and candidate integrity.
5. **Technical Gates & Seal**: Pass `verify-v2.ps1`, `run-comprehensive-uat.ps1`, verify sumi.db hash, issue review seal, and advance `STATE.json`.

## Acceptance mapping
| Acceptance ID | Implementation evidence | Test/UAT evidence |
| --- | --- | --- |
| `UNI-PIT-001` | `UniverseResolver.resolve` with `POINT_IN_TIME` vs `RETROSPECTIVE_FIXED` | `test_universe_point_in_time_filtering`, `test_universe_retrospective_caveat` |
| `UNI-COV-002` | `UniverseResolver.check_coverage` with ratio and status | `test_universe_coverage_calculation` |
| `UNI-420-003` | `sumi420_v1_candidate.json` artifact with 420 active members, hash, candidate status | `test_sumi420_candidate_integrity`, `test_content_hash_determinism` |

## Verification commands
```powershell
python -m pytest backend/app/tests/test_universe.py -v
.\scripts\verify-v2.ps1
.\scripts\run-comprehensive-uat.ps1
Get-FileHash "backend\sumi.db" -Algorithm SHA256
node scripts/verify-dev-program.mjs
```

## Rollback and compatibility
- Delete new files under `backend/app/domain/universe/`, `backend/app/api/universe.py`, and remove router mount in `backend/app/main.py`.
- No database migrations exist, so rollback leaves database completely untouched.

## Risks and mitigations
- **Survivorship bias masquerading as truth**: Mitigated by strict `POINT_IN_TIME` vs `RETROSPECTIVE_FIXED` separation and mandatory warning strings.
- **Silent promotion**: Mitigated by explicit `CANDIDATE` status requiring formal sign-off.

## Progress log
- 2026-09-18: ExecPlan created, architecture specified, implementation completed.
  - Pure domain models (`models.py`), resolver (`resolver.py`), and registry (`registry.py`) created.
  - Candidate v1 definition (`sumi420_v1_candidate.json`) generated from Doraemon audited 470 symbols (420 core active + 50 reserve candidates) with candidate status and explicit survivor bias caveat on historical backfill.
  - Synthetic test fixture (`pit_test_universe.json`) created with additions, delistings, and suspensions across 2024-2026.
  - Pydantic schemas (`universe_schema.py`), service (`universe_service.py`), and API router (`universe.py`) mounted under `/api/universe`.
  - Comprehensive unit test suite (`test_universe.py`) passing 7/7 tests in 0.16s.
  - Fast technical gate (`verify-v2.ps1`) passing 330/330 backend tests, temp DB migrations, ESLint, 210/210 frontend tests, and production build.
  - Comprehensive browser UAT (`run-comprehensive-uat.ps1`) passing 31/31 scenarios with zero console errors and zero DB mutation.
  - Database SHA-256 hash verified untouched (`92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`).

## Decision log
- **Decision**: Manage universe definitions as immutable versioned domain artifacts rather than adding database tables in Sumi.
  - **Rationale**: Direct compliance with `SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` line 438. Doraemon is the future canonical store for universe episodes.

## Completion evidence
- **Backend Unit Tests**: `python -m pytest backend/app/tests/test_universe.py -v` (7 passed in 0.16s).
- **Fast Technical Gate**: `.\scripts\verify-v2.ps1` (330 backend tests, temp migrations, ESLint, 210 vitest tests, build PASS).
- **Comprehensive Browser UAT**: `.\scripts\run-comprehensive-uat.ps1` (31/31 passed, 100%, 0 console errors, report written to `test-results/comprehensive-uat/report.json`).
- **Database Hash**: `Get-FileHash "backend\sumi.db" -Algorithm SHA256` matches `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64` (0 bytes mutated).

