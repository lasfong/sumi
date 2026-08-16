# PRO-11 DEV Prompt — One-Click Local Data Synchronization

You are the dedicated DEV session for **PRO-11 — One-Click Local Data Synchronization**. Implement this batch from the current workspace checkout; do not rely on chat history. Stop at the Independent Reviewer Gate when implementation and verification are complete. Do not approve your own work, commit, push, or start PRO-12.

## Read order

1. `AGENTS.md`
2. `docs/INDEX.md`
3. `docs/ANTIGRAVITY_TWO_SESSION_OPERATING_MODEL.md`
4. `docs/AUTONOMOUS_EXECUTION_STATE.md`
5. `docs/ARCHITECTURE_DECISION_002_MARKET_DATA_PROVIDER.md` (ADR-002: Provider Boundary Architecture)
6. `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md` (PRO-11, PRO-DATA-08..10, PRO-PROV-06)
7. `docs/program/PRO_11_ONE_CLICK_DATA_SYNC.md`
8. `docs/exec-plans/PRO_11_ONE_CLICK_DATA_SYNC.md`

## Outcome

Users can safely synchronize Vietnam equity Daily data through the approved `MarketDataProviderAdapter` boundary (SSI FastConnect / `vnstock` fallback) with explicit user-triggered preview, conflict classification, progress indication, atomic commit, automatic weekly aggregation, immutable audit manifests, and one-click rollback—maintaining strict local-first privacy.

## Implementation tasks

1. **Backend Provider Adapters Package (`backend/app/services/data_providers/`):**
   - Create `base_provider.py`: Define `MarketDataProviderAdapter`, `ProviderCandleDTO`, `ProviderMetadata`, and provider exceptions (`ProviderAuthError`, `ProviderRateLimitError`, `ProviderNetworkError`).
   - Create `ssi_provider.py`: Implement SSI FastConnect adapter with request signing/Bearer auth and daily candle normalization.
   - Create `vnstock_provider.py`: Implement community fallback adapter.
   - Create `provider_registry.py`: Factory resolving active adapter instances.
   - Ensure adapters tag candles with `adjustment_type` ('unadjusted' by default).

2. **Backend Sync Workflow Service (`backend/app/services/sync_workflow_service.py`):**
   - Implement `test_connection(provider_id, credentials)`.
   - Implement `generate_sync_preview(symbol, start_date, end_date, provider_id)` leveraging `ImportClassifier` to classify new vs duplicate vs conflict candles.
   - Implement `execute_sync(preview_id)`: Atomically insert new candles, update catalog metadata, trigger `WeeklyAggregator.derive_weekly_candles`, and persist an immutable sync manifest.
   - Implement `rollback_sync(sync_run_id)`: Revert added candles and restore catalog state.

3. **Backend Sync API Routes (`backend/app/api/sync.py`):**
   - Provide REST endpoints for provider listing, connection testing, sync preview, execution, history, and rollback.
   - Register route in `backend/app/main.py`.
   - Add unit/integration tests in `backend/app/tests/test_sync_workflow.py`.

4. **Frontend Sync UI (`frontend/src/pages/` or Data Feeds):**
   - Build sync controls: provider selector, credentials configuration, test connection button, symbol & date range selector.
   - Pre-commit diff preview table showing counts and sample candles before applying.
   - Progress bar during execution and detailed result summary.
   - Sync history log with one-click Rollback button.
   - Add frontend vitest unit tests.

5. **Product UAT & Screenshot Evidence:**
   - Add deterministic UAT checks in `scripts/product-uat.mjs` and `scripts/fixtures/product-uat-v3-baseline.json` for provider listing, test connection, sync preview, atomic execution, weekly derivation verification, and rollback.
   - Retain `pro11-data-sync-1440x1000.png` and `pro11-data-sync-1280x800.png`.

6. **Technical Gates & Hand-off:**
   - Run pytest and vitest suites.
   - Run `.\scripts\verify-v2.ps1`.
   - Run `.\scripts\run-product-uat.ps1`.
   - Check `backend/sumi.db` SHA-256 before/after.
   - Check `git diff --check`.
   - Update `docs/exec-plans/PRO_11_ONE_CLICK_DATA_SYNC.md` and `docs/AUTONOMOUS_EXECUTION_STATE.md`.

## Stop rule

Stop at the Independent Reviewer Gate. Report completion with exact commands, test counts, artifact hashes, and screenshot evidence. Do not commit, push, or start PRO-12.
