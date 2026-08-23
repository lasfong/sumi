# PRO-11 — One-Click Local Data Synchronization

Status: `CLOSED — INDEPENDENTLY APPROVED`

## Outcome

Users can safely and reliably synchronize Vietnam equity Daily market data through the approved `MarketDataProviderAdapter` boundary (SSI FastConnect / `vnstock` community fallback as specified in ADR-002) with explicit user-triggered preview, conflict classification, progress indication, atomic commit, automatic weekly aggregation, immutable audit manifests, and one-click rollback—maintaining strict local-first privacy (zero telemetry, zero user trading data transmitted).

## Context and problem

PRO-10 is independently approved and closed, delivering `docs/ARCHITECTURE_DECISION_002_MARKET_DATA_PROVIDER.md` (ADR-002). ADR-002 approved online provider integration under the `MarketDataProviderAdapter` boundary while retaining offline file import (`PRO-03`) as the permanent baseline. In PRO-11, we implement the provider adapters, sync orchestration service, transactional staging/rollback, automatic weekly aggregation, and user-facing sync controls.

Authority: `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`, acceptance IDs `PRO-DATA-08` through `PRO-DATA-10`, `PRO-PROV-06`; `docs/ARCHITECTURE_DECISION_002_MARKET_DATA_PROVIDER.md`; `docs/program/PRO_11_ONE_CLICK_DATA_SYNC.md`; V3 G-01..05 regression.

## In scope

1. **Backend Provider Adapter Implementation (`PRO-PROV-06`):**
   - Implement `backend/app/services/data_providers/` package with `base_provider.py` (`MarketDataProviderAdapter`, `ProviderCandleDTO`, `ProviderMetadata`), `ssi_provider.py` (SSI FastConnect client/mock adapter), `vnstock_provider.py` (community adapter), and `provider_registry.py`.
   - Ensure adapters normalize all incoming data to standard daily OHLCV format with UTC+7 timestamps and explicit `adjustment_type` tagging.
2. **Sync Workflow & Orchestration Service (`PRO-DATA-08`, `PRO-DATA-09`):**
   - Implement `backend/app/services/sync_workflow_service.py` to coordinate:
     - Connection testing (`test_connection`)
     - Dry-run preview generation with conflict/duplicate classification (leveraging `ImportClassifier`)
     - User-confirmed atomic execution into SQLite database
     - Automatic weekly aggregation trigger via `WeeklyAggregator.derive_weekly_candles`
     - Immutable audit manifest generation recording symbol, range, provider, duration, record counts, and status.
3. **Rollback & State Recovery (`PRO-DATA-10`):**
   - Implement sync batch rollback allowing users to undo a recent sync run and restore previous catalog/candle state cleanly.
4. **Sync REST APIs (`backend/app/api/sync.py`):**
   - `GET /api/sync/providers` — List available providers and credentials configuration status (masked).
   - `POST /api/sync/test-connection` — Test provider connectivity with user-supplied or saved credentials.
   - `POST /api/sync/preview` — Generate dry-run sync preview and conflict report.
   - `POST /api/sync/execute` — Execute and commit confirmed sync batch.
   - `POST /api/sync/rollback` — Rollback a specific sync batch.
   - `GET /api/sync/history` — List immutable sync audit manifests.
5. **Frontend Sync Management UI (`frontend/src/pages/DataSyncPage.tsx` or Data Feeds Tab):**
   - Provider selection (SSI FastConnect / vnstock fallback) and credential configuration.
   - Connection test button with instant visual feedback.
   - Quick sync triggers (e.g. "Sync Recent 30 Days", custom date range, symbol picker).
   - Pre-commit diff preview table showing new candles, duplicates, and conflicts.
   - Progress bar during sync execution and post-sync summary report.
   - Sync history log with Rollback button for recent batches.
6. **Automated Testing & Browser Evidence:**
   - Backend unit and integration tests for adapter normalization, rate limit retries, preview conflict detection, atomic commit, weekly aggregation trigger, and rollback.
   - Frontend vitest tests for sync form, preview modal, progress states, and rollback actions.
   - Deterministic Product UAT assertions (`pro11.*`) and retained 1440×1000 and 1280×800 screenshots.

## Out of scope

- Direct automated background scraping daemons (violates user-triggered on-demand invariant).
- Final release candidate packaging (belongs to PRO-12).

## Invariants

- Local-first privacy: zero telemetry, zero user trading data, strategies, or replay state transmitted externally.
- Provider isolation: core domain (`Candle`, `IndicatorEngine`, `ReplayService`) never imports third-party client libraries.
- Weekly candle authority: 1W candles are strictly derived internally by `WeeklyAggregator`.
- `backend/sumi.db` SHA-256 remains untouched during tests/UAT.

## Acceptance mapping

| ID | Requirement | Status |
| --- | --- | --- |
| PRO-DATA-08 | One-click data sync requires explicit user confirmation, shows a pre-commit diff/preview, and can be cancelled before applying. | Passed (Unit & Product UAT `pro11.sync-preview-and-classification`, `pro11.atomic-sync-and-weekly-derivation`) |
| PRO-DATA-09 | Sync operations produce an immutable audit manifest recording symbol, range, provider, duration, record counts, and status. | Passed (Unit & Product UAT `pro11.sync-manifest-audit-trail`) |
| PRO-DATA-10 | Rollback restores the catalog and candle store to its pre-sync state without data corruption or index inconsistency. | Passed (Unit & Product UAT `pro11.sync-rollback-and-integrity`) |
| PRO-PROV-06 | Provider Boundary Adapter Architecture enforces strict isolation and vendor independence. | Passed (Unit & Product UAT `pro11.provider-listing-and-connectivity`) |

## Verification commands and results

```powershell
Get-FileHash -Algorithm SHA256 backend\sumi.db
# Result: 450B7EE02A2F8CEC18E1C3B01A6F76CE2355EF1980BECFCE2EF969D25BD9896A

Set-Location backend
& .\.venv\Scripts\python.exe -m pytest app/tests/ -v
# Result: 187 passed, 1 warning (11 new sync tests in test_sync_workflow.py)

Set-Location ..\frontend
npm.cmd test -- --run
# Result: 28 test files passed, 187 tests passed (5 new sync tests in DataSync.test.tsx)

npm.cmd run lint
# Result: 0 errors, 0 warnings

Set-Location ..
.\scripts\verify-v2.ps1
# Result: Technical gate passed cleanly (pytest 187/187, alembic migration upgrade head, frontend lint, vitest 187/187, frontend build 0 errors)

.\scripts\run-product-uat.ps1
# Result: Full product UAT suite passed cleanly with all pro11.* checks and retained 1440x1000 and 1280x800 screenshots

git diff --check
# Result: 0 whitespace/conflict errors

Get-FileHash -Algorithm SHA256 backend\sumi.db
# Result: 450B7EE02A2F8CEC18E1C3B01A6F76CE2355EF1980BECFCE2EF969D25BD9896A (Invariant Preserved)
```

## Progress log

- 2026-08-16: User authorized PRO-11. Reviewer prepared ExecPlan and standalone DEV prompt. Batch is ready for DEV implementation.
- 2026-08-16: Implemented backend database models (`SyncRun`, `SyncRunItem`, `SyncRunMutation`) and Alembic migration `20260816_0002_data_sync_runs.py`.
- 2026-08-16: Implemented `MarketDataProviderAdapter` package with SSI FastConnect and `vnstock` community fallback adapters.
- 2026-08-16: Implemented `sync_workflow_service.py` with connectivity testing, pre-commit dry-run preview conflict detection, atomic execution, automatic weekly aggregation, immutable audit manifest generation, and fail-closed rollback.
- 2026-08-16: Implemented `/api/sync/*` REST routes and backend tests (`test_sync_workflow.py`).
- 2026-08-16: Implemented frontend API client (`syncApi.ts`), `DataSyncPanel.tsx` UI component, and integrated into `ImportPage.tsx` tab.
- 2026-08-16: Implemented frontend vitest tests (`DataSync.test.tsx`) with 100% pass rate and zero ESLint issues.
- 2026-08-16: Updated product UAT manifest baseline (`product-uat-v3-baseline.json`) and runner (`product-uat.mjs`) with `pro11.*` assertions and captured screenshots.
- 2026-08-16: Full technical gate (`verify-v2.ps1`) and deterministic product UAT (`run-product-uat.ps1`) verified green. Database invariant SHA-256 confirmed unchanged. Handing off to Independent Reviewer gate.
- 2026-08-16: Independent Reviewer audited code, schemas, routes, frontend, screenshots, and test evidence. Verdict: `APPROVE` recorded in `docs/reviews/PRO_11_REVIEW_2026-08-16.md`. PRO-11 is closed; PRO-12 remains unauthorized.
