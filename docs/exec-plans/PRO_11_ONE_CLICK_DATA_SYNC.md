# PRO-11 — One-Click Local Data Synchronization

Status: `PREPARED — USER AUTHORIZED`

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

| ID | Requirement |
| --- | --- |
| PRO-DATA-08 | One-click data sync requires explicit user confirmation, shows a pre-commit diff/preview, and can be cancelled before applying. |
| PRO-DATA-09 | Sync operations produce an immutable audit manifest recording symbol, range, provider, duration, record counts, and status. |
| PRO-DATA-10 | Rollback restores the catalog and candle store to its pre-sync state without data corruption or index inconsistency. |
| PRO-PROV-06 | Provider Boundary Adapter Architecture enforces strict isolation and vendor independence. |

## Verification commands

```powershell
Get-FileHash -Algorithm SHA256 backend\sumi.db
Set-Location backend
& .\.venv\Scripts\python.exe -m pytest app/tests/ -v
Set-Location ..\frontend
npm.cmd test -- --run
Set-Location ..
.\scripts\verify-v2.ps1
.\scripts\run-product-uat.ps1
git diff --check
Get-FileHash -Algorithm SHA256 backend\sumi.db
```

## Progress log

- 2026-08-16: User authorized PRO-11. Reviewer prepared ExecPlan and standalone DEV prompt. Batch is ready for DEV implementation.
