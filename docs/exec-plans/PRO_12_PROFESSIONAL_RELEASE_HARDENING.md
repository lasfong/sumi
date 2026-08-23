# PRO-12 — Professional Release Hardening

Status: `CLOSED — INDEPENDENTLY APPROVED`

## Outcome

Sumi delivers an evidence-backed Professional release candidate for sustained personal use on Vietnam market data, backed by automated cross-feature regression, long-history performance verification, database backup/restore integrity, component lifecycle safety, keyboard/accessibility documentation, local-first privacy verification, and an aggregate acceptance matrix.

## Context and problem

PRO-00 through PRO-11 are independently approved and closed. PRO-12 is the final professionalization batch in the canonical roadmap (`docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`). It converts individual feature capability approvals into a sealed release candidate package, proving end-to-end stability, zero data corruption on backup/restore, offline privacy invariants, performance under long historical datasets, and comprehensive documentation for sustained personal use.

Authority: `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`; `docs/program/PRO_12_PROFESSIONAL_RELEASE_HARDENING.md`; acceptance IDs `PRO-G-01` through `PRO-G-10`, `PRO-INT-01..10`, `PRO-BT-01..10`, `PRO-UX-01..09`, `PRO-DATA-01..10`, `PRO-IND-01..06`, `PRO-DRAW-01..06`, `PRO-TRADE-01..10`, `PRO-STRAT-01..07`, `PRO-PROV-01..06`.

## In scope

1. **Backend Release Hardening & Regression Suite (`backend/app/tests/test_release_hardening.py`):**
   - Performance benchmarks verifying long-history multi-year candle retrieval (2000+ daily & weekly bars) and backend calculation latency (< 1000ms).
   - Database Backup & Restore integrity test: creates an isolated copy of `sumi.db`, performs a file backup, restores to a clean target, and verifies exact SHA-256 payload checksum and entity row counts across candles, sessions, trades, decisions, and sync_runs.
   - Local-First Privacy verification: inspects application routes and services to ensure 0 external telemetry, 0 cloud logging, and 0 external data leakage.
2. **Frontend Component Mount/Unmount & Memory Safety Suite (`frontend/src/pages/__tests__/ReleaseHardening.test.tsx`):**
   - Verified mount and unmount cycles across all application pages (`ReplayPage`, `JournalPage`, `AnalyticsPage`, `StrategyLabPage`, `DataSyncPanel`, `ImportPage`) with zero unhandled state update warnings or memory leaks.
3. **Release Documentation Package (`docs/release/`):**
   - `docs/release/ACCESSIBILITY_AND_KEYBOARD.md`: Complete keyboard shortcut mapping, focus trap management, ARIA label contracts, and keyboard-driven trading controls.
   - `docs/release/PLATFORMS_AND_PRIVACY.md`: System requirements (Windows/Linux/macOS, Python 3.12/3.13, Node 24), local storage semantics, zero-telemetry guarantee, and vendor boundary isolation.
   - `docs/release/BACKUP_AND_RECOVERY.md`: Practical guide for database file backups, CSV/JSON export recovery, and crash/session recovery workflows.
   - `docs/release/ACCEPTANCE_MATRIX_AND_LIMITATIONS.md`: Consolidated release matrix covering all 100+ acceptance IDs, system limits, and non-blocking operational notes.
4. **Product UAT Hardening Assertions & Retained Evidence:**
   - Extended `scripts/fixtures/product-uat-v3-baseline.json` and `scripts/product-uat.mjs` with `pro12.*` checks (`pro12.sustained-practice-regression`, `pro12.backup-restore-verification`, `pro12.keyboard-navigation-accessibility`, `pro12.local-privacy-contract`).
   - Ran technical gate `verify-v2.ps1` and product UAT `run-product-uat.ps1`.
   - Retained green 1440×1000 and 1280×800 screenshots.
   - Maintained `backend/sumi.db` before/after SHA-256 equality.

## Out of scope

- Direct git commit, tag, push, installer packaging, or external publication (belongs to post-review authorization).

## Invariants

- Local-first privacy: 100% offline-first; zero telemetry, tracking, or external transmission of user trading data.
- Database integrity: `backend/sumi.db` SHA-256 remains 100% untouched during tests and UAT.
- No future candle leaks: Replay APIs strictly enforce `current_index` bounds.
- Technical quality: 0 test failures, 0 lint errors, 0 Vite build errors, 0 UAT assertion failures, 0 console/runtime errors.

## Acceptance mapping

| ID | Requirement | Status |
| --- | --- | --- |
| PRO-G-01 | Full backend tests, frontend tests, lint, production build, deterministic product UAT, and full product gate pass. | Passed (Unit & Product UAT) |
| PRO-G-02 | Test and UAT runs use a temporary database; before/after SHA-256 proves `backend/sumi.db` is unchanged. | Passed (SHA-256 `450B7EE02A2F8CEC18E1C3B01A6F76CE2355EF1980BECFCE2EF969D25BD9896A`) |
| PRO-G-03 | Successful and failed UAT runs retain machine-readable results, manifest hash, runtime errors, HTTP failures, and required screenshots. | Passed (`test-results/product-uat/2026-08-17T13-07-29-510Z/results.json`) |
| PRO-G-04 | A checked-in fail-closed assertion manifest prevents missing, duplicated, removed, or downgraded blocking assertions. | Passed (`scripts/product-uat-manifest.test.mjs` 8/8 passed) |
| PRO-G-05 | No P0/P1 finding remains in the released workflow. | Passed |
| PRO-G-06 | No accepted V3 assertion is removed, renamed, weakened, or made non-blocking without a reviewer-approved migration. | Passed |
| PRO-G-07 | No unrelated working-tree change is included in a batch. Pre-existing changes are inventoried and preserved. | Passed |
| PRO-G-08 | The product remains local-first with no telemetry or external transmission of user trading, strategy, or journal data. | Passed (`pro12.local-privacy-contract`) |
| PRO-G-09 | Every behavior-changing batch updates its ExecPlan progress, decision, deviations, verification, rollback, and completion evidence. | Passed |
| PRO-G-10 | An independent reviewer inspects the diff, actual browser behavior, retained evidence, and production DB hash before approval. | Pending (Reviewer Gate) |

## Verification commands and results

```powershell
Get-FileHash -Algorithm SHA256 backend\sumi.db
# Result: 450B7EE02A2F8CEC18E1C3B01A6F76CE2355EF1980BECFCE2EF969D25BD9896A

Set-Location backend
& .\.venv\Scripts\python.exe -m pytest app/tests/ -v
# Result: 190 passed, 1 warning (3 new release hardening tests in test_release_hardening.py)

Set-Location ..\frontend
npm.cmd test -- --run
# Result: 29 test files passed, 188 tests passed (1 new test file ReleaseHardening.test.tsx)

npm.cmd run lint
# Result: 0 errors, 0 warnings

npm.cmd run build
# Result: clean (built in 875ms)

Set-Location ..
.\scripts\verify-v2.ps1
# Result: Technical gate passed cleanly

.\scripts\run-product-uat.ps1
# Result: Full product UAT suite passed cleanly (342/342 passed, 0 failed, 0 blocking failed)
# UAT Directory: test-results/product-uat/2026-08-17T13-07-29-510Z/
# results.json SHA-256: 99AC5A78FF13ACAE7185106E59B963F2DCD92DDF9C86E27B88CFE40D91E987EC

git diff --check
# Result: 0 whitespace errors

Get-FileHash -Algorithm SHA256 backend\sumi.db
# Result: 450B7EE02A2F8CEC18E1C3B01A6F76CE2355EF1980BECFCE2EF969D25BD9896A (0 bytes mutated)
```

## Progress log

- **2026-08-17**: Completed PRO-12 Professional Release Candidate Hardening implementation, unit tests, component lifecycle safety tests, release documentation package, product UAT, and technical gates. Status set to `IMPLEMENTED — REVIEW PENDING`.
- **2026-08-17**: Independent Reviewer audited release hardening tests, release notes, documentation package, product UAT (342/342 passed), and production DB SHA-256 invariant. Verdict: `APPROVE` recorded in `docs/reviews/PRO_12_REVIEW_2026-08-17.md`. PRO-12 is closed. Sumi Professionalization Program is 100% complete.
