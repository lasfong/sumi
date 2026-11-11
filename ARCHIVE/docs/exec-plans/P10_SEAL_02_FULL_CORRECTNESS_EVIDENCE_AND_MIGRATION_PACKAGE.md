# P10-SEAL-02 — Full Correctness, Evidence Closure, and Migration Package

## Outcome
Comprehensive program completion sealing the Sumi DEV Program. Closes acceptance traceability across all batches (P1 through P10), publishes the final Acceptance Matrix & Limitations, provides the Subsystem Extraction Map and API Schema Snapshots for Doraemon/Mizuhara adoption, verifies repository security and data immutability, and issues the final program review seal.

## Context and problem
- **References**: `SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` lines 537–549; `SUMI_MASTER_FUNCTIONAL_TECHNICAL_SPEC_FINAL.md` lines 238–245 (`NFR-OBS-001`, `TEST-REPRO-001`, Repository Definition of Done).
- **Scope**:
  1. Acceptance matrix and explicit limitations documentation.
  2. Subsystem extraction map and versioned API schemas for downstream consumer adoption (Doraemon/Mizuhara).
  3. Final verification across all technical gates: fast technical gate (`verify-v2.ps1`), comprehensive browser UAT (`run-comprehensive-uat.ps1`), performance benchmarks, secret scan, and database immutability verification (`sumi.db` SHA-256 baseline).
  4. Final independent review seal and state transition.

## In scope
1. **Acceptance Matrix & Explicit Limitations (`docs/release/ACCEPTANCE_MATRIX_AND_LIMITATIONS.md`)**:
   - Complete traceability of all functional, non-functional, and testing acceptance IDs from P1 to P10.
   - Explicit disclaimers and status boundaries for research capabilities (e.g. `OHLCV_PROXY` classification, synthetic fixture status vs. live market data).
2. **Subsystem Extraction Map (`docs/migration/DORAEMON_MIZUHARA_EXTRACTION_MAP.md`)**:
   - Extraction interfaces and modular boundaries for:
     - Pure Next-Event Backtest Kernel (`app.domain.backtest`)
     - Vietnam Market Rule Engine (`app.domain.market`)
     - Signal Registry & 72 Causal Triggers (`app.domain.signals`)
     - Technical Health & Money Flow BB Engine (`app.domain.bb`)
     - Universe & Index Framework (`app.domain.universe`)
     - Bounded Invalidation-Safe Cache Engine (`app.domain.engine.cache`)
3. **API Schema Snapshots (`docs/migration/API_SCHEMA_SNAPSHOTS.md`)**:
   - Frozen OpenAPI / JSON schemas for Backtest, Replay, Signals, BB, and Universe endpoints.
4. **Security & Secrets Verification**:
   - Scan codebase for exposed API keys, credentials, private tokens, or leaked secrets.
5. **Full Technical Gate Execution**:
   - Fast technical gate: `verify-v2.ps1` (backend pytest, linters, frontend tests, vite build).
   - Comprehensive browser E2E UAT: `run-comprehensive-uat.ps1` (31/31 passing, zero console errors).
   - Performance benchmark: `benchmark_research_hotspots.py`.
   - Immutable DB SHA-256 confirmation: `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`.
   - Preserved git status: 159 historically staged deletions untouched.
6. **Independent Review Seal (`docs/reviews/P10_SEAL_02_REVIEW.md`)**:
   - Final review seal certifying program completion.

## Out of scope
- Mutating `backend/sumi.db` or running persistent database migrations.
- Modifying previously validated trading logic or mathematical algorithms.
- Connecting unapproved or paid external data scrapers.

## Invariants
- Zero future candle leak across all replay and backtest interfaces.
- Deterministic reproducibility (`TEST-REPRO-001`).
- Database immutability (`sumi.db` SHA-256 must match `92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`).
- Retention of 159 staged deletions untouched.
- Clean zero-telemetry local-first operation.

## Milestones
1. **Milestone 1**: Compile and publish Acceptance Matrix & Limitations (`docs/release/V3_ACCEPTANCE_MATRIX_AND_LIMITATIONS.md`).
2. **Milestone 2**: Compile and publish Subsystem Extraction Map & API Schema Snapshots (`docs/migration/`).
3. **Milestone 3**: Run security/credential audit across workspace.
4. **Milestone 4**: Execute full technical gates (`verify-v2.ps1`, `run-comprehensive-uat.ps1`, `benchmark_research_hotspots.py`).
5. **Milestone 5**: Issue final review seal `docs/reviews/P10_SEAL_02_REVIEW.md`, update `STATE.json`, and run `verify-dev-program.mjs`.

## Progress log
- 2026-09-19: ExecPlan drafted and batch `P10-SEAL-02` transitioned to running.
- 2026-09-19: Published Acceptance Matrix & Limitations in `docs/release/V3_ACCEPTANCE_MATRIX_AND_LIMITATIONS.md`.
- 2026-09-19: Published Doraemon/Mizuhara Subsystem Extraction Map in `docs/migration/DORAEMON_MIZUHARA_EXTRACTION_MAP.md`.
- 2026-09-19: Published Frozen API Schema Snapshots in `docs/migration/API_SCHEMA_SNAPSHOTS.md`.
- 2026-09-19: Completed security and secrets audit (zero exposed credentials/keys).
- 2026-09-19: Executed performance benchmarks (`benchmark_research_hotspots.py`), fast technical gate (`verify-v2.ps1`), and comprehensive browser E2E UAT (`run-comprehensive-uat.ps1`: 31/31 passed).
- 2026-09-19: Verified `sumi.db` SHA-256 hash matches baseline (`92A7F65AB8B7BB69F9D7DCD6E5B6A4DF38A14964B84F9255E0574CBA9E399A64`) and 159 staged deletions preserved untouched.
- 2026-09-19: Final independent review seal issued in `docs/reviews/P10_SEAL_02_REVIEW.md` (ACCEPTED).

## Decision log
- Decision: Provide standalone migration package in `docs/migration/` so Doraemon and Mizuhara teams can consume Sumi modules as standalone Python packages or microservice endpoints without code tangling.

