# SUMI V3 — NEXT WAVE TEAMWORK PROMPT (PROPOSED)

- **Document ID**: `SUMI-V3-NEXT-WAVE-PROMPT-2026-10-14`
- **Status**: **PROPOSED** (Reconciled per External GPT-6 Architect Review — Awaiting W0-A Baseline Commit & Owner Approval)
- **Target Execution Mode**: Antigravity `/teamwork-preview` (Multi-Agent Wave with Strictly Sequential Implementation Writes: `B03` → `B10` → `B11`)
- **Target Wave**: **Wave `W0-B + W1` (`B03` → `B10` → `B11`) — Schema/Preview Baseline Closure & Replay Zero-Lookahead Lockdown**
- **Repository Root**: `/Users/mizuhara/workspace/sumi`
- **Companion Documents**:
  - [SUMI_V3_PRODUCT_COMPLETION_MASTER_PLAN_PROPOSED.md](./SUMI_V3_PRODUCT_COMPLETION_MASTER_PLAN_PROPOSED.md)
  - [SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md](./SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md)

---

## PART 1 — SMALLEST NECESSARY PRECONDITION GATE (`Gate G-W0A`)

Because `B00`, `B01`, and `B02` (`Wave W0-A`) currently reside as uncommitted modifications across 19 tracked files and 9 untracked source/test files in the working tree (`VERIFIED` on commit `1ff5f04ed7a26969130eaba75da50b20684e763e`), and `B03` touches `backend/alembic/env.py` and `backend/app/services/import_workflow_service.py`, **product implementation of Wave `W0-B + W1` must not start until the following minimal precondition gate (`Gate G-W0A`) is completed**:

### Gate `G-W0A` Checklist (Smallest Necessary Precondition)
1. **Architect & Product Owner Ratification of `W0-A` and the Three Proposed ADR Addenda (`DEC-01`)**:
   - External GPT-6 Technical Architect and Product Owner review and ratify `W0-A` (`B00`, `B01`, `B02`) and the three supporting ADR addenda indexed in [ADR_ADDENDA_W0A_PROPOSALS.md](../../architecture/ADR_ADDENDA_W0A_PROPOSALS.md):
     - [ADR-PROV-001-ADDENDUM-01-quarantine-promotion.md](../../architecture/decisions/ADR-PROV-001-ADDENDUM-01-quarantine-promotion.md): **Option A Quarantine Promotion & Precedence** — In-place promotion of a single unambiguous `quarantine:...` row preserving `Candle.id` and downstream foreign-key references (`WeeklyCandleProvenance`, `Execution`); atomic pre-validation rejection with `HTTP 400` + `db.rollback()` when `len(candidates) > 1` share the same calendar date; synthetic sync never overwrites canonical data (`daily_db_vietstock`, `cafef_csv`, `vnstock`, `legacy`).
     - [ADR-TX-001-ADDENDUM-01-mutation-provenance.md](../../architecture/decisions/ADR-TX-001-ADDENDUM-01-mutation-provenance.md): **Mutation Provenance Encoding & State Reconstruction** — `UPDATE:{prior_source}|{prior_timestamp_iso}` encoding in `ImportRunMutation.action` / `SyncRunMutation.action`, right-split parsing (`rsplit("|", 1)`), fail-closed `HTTP 400` validation on empty/malformed ISO timestamps or missing/ambiguous rollback target candles, and transactional `db.rollback()` atomicity.
     - [ADR-TX-001-ADDENDUM-02-cross-workflow-rollback.md](../../architecture/decisions/ADR-TX-001-ADDENDUM-02-cross-workflow-rollback.md): **Cross-Workflow Symmetrical Rollback Conflict Resolution** — `ImportWorkflowService.rollback_import` checks for subsequent accepted `SyncRun`s on affected symbols and aborts with `HTTP 400`; `SyncWorkflowService.rollback_sync` checks for subsequent accepted `ImportRun`s on the symbol and aborts with `HTTP 400`; non-accepted runs (`ROLLED_BACK`, `REJECTED`, `FAILED`, `DRY_RUN`) do not block rollback.
2. **Single Clean `W0-A` Baseline Commit (`Strategy B` from [COMMIT_SEPARATION_STRATEGY_EVALUATION.md](../COMMIT_SEPARATION_STRATEGY_EVALUATION.md))**:
   - Stage and commit the 28 `W0-A` implementation/test files (and, in the same or an immediately following commit, the planning/architecture documentation files listed in Section I.4 of [SUMI_V3_PRODUCT_COMPLETION_MASTER_PLAN_PROPOSED.md](./SUMI_V3_PRODUCT_COMPLETION_MASTER_PLAN_PROPOSED.md)), while excluding `artifacts/local/` and all `.zip` archives.
   - Verify `git status --porcelain` has zero uncommitted changes to tracked files before starting `B03`.
3. **Note on Local Database Re-Seeding (`W0-OP-DB` — Non-Blocking)**:
   - Re-seeding `backend/sumi.db` from local CafeF CSVs is an **optional Product Owner convenience operation** after `B03` is accepted, **not** a precondition gate for `W0-B + W1` or a release gate in `W5`. All automated tests in `W0-B + W1` must run exclusively against isolated `tmp_path` SQLite fixtures.

---

## PART 2 — CONDITIONAL NEXT-WAVE `/teamwork-preview` PROMPT (`W0-B + W1`: `B03 → B10 → B11`)

> **Instructions for Operator / Product Owner**: Once `Gate G-W0A` above is satisfied (`W0-A` committed and `git status` clean of tracked modifications), copy the prompt block below into Antigravity to execute **Wave `W0-B + W1` (`B03` first → `B10` second → `B11` third)**.

````markdown
# SUMI V3 — WAVE W0-B + W1 EXECUTION PROMPT (TEAMWORK-PREVIEW)
## Sequential Implementation Wave: `B03` (First) → `B10` (Second) → `B11` (Third)

### 1. WAVE GOAL & USER-VISIBLE OUTCOMES

You are executing **Wave `W0-B + W1`** (`B03` → `B10` → `B11`) of the Sumi V3 Product Completion Program inside `/Users/mizuhara/workspace/sumi`.

**Default Implementation & Acceptance Order is Strictly Sequential**:
1. **`B03` FIRST** (`Schema, Migration Compatibility & Preview Isolation`): Implement, verify, and accept `B03` before any `B10` implementation writes begin.
2. **`B10` SECOND** (`Replay Ledger, Horizon & Eligibility Zero-Lookahead Lock`): Implement, verify, and accept `B10` after `B03` passes before any `B11` implementation writes begin.
3. **`B11` THIRD** (`Indicator Forward Projection Clamping & Typed Metadata Contract`): Implement, verify, and accept `B11` after `B10` passes.

*(Reviewers, specification investigators, and test designers may work in parallel in read-only mode to prepare test matrices, inspect migration history, and design golden verification assertions, but they must not modify shared application files concurrently with active implementation writes.)*

**Primary User-Visible & Product Outcomes**:
1. **Full 20-Model Alembic Registry, Migration Compatibility & Zero-Side-Effect Preview (`B03`)**:
   - `backend/alembic/env.py` registers all 20 ORM models exported by `backend/app/models/__init__.py` (`Symbol`, `Candle`, `ReplaySession`, `Decision`, `Order`, `Execution`, `Position`, `Trade`, `JournalEntry`, `EventLog`, `DrawingState`, `StrategyLabRun`, `ScannerRun`, `ImportRun`, `ImportRunItem`, `ImportRunMutation`, `WeeklyCandleProvenance`, `SyncRun`, `SyncRunItem`, `SyncRunMutation`).
   - Migration-chain compatibility across the 9 existing revisions (`cdf80254e9dc` through `a2d639ecd5d5_add_execution_candle_index.py`) is assessed before changing any migration file; existing applied revisions are not automatically rewritten, and an additive corrective migration is preferred for `GAP-DB-06` composite indexes and schema parity.
   - Previewing a CSV import (`confirm=False` / dry-run) or running `seed_cafef(..., dry_run=True)` is 100% read-only across **all tables potentially affected by the workflow** (`candles`, `symbols`, `import_runs`, `import_run_items`, `import_run_mutations`, `weekly_candle_provenance`, `sync_runs`, `sync_run_items`, `sync_run_mutations`, `event_logs`).
2. **Strict Replay Ledger & Horizon Zero-Lookahead (`B10`)**:
   - During an active Replay session, `/api/sessions/{id}/decisions`, `/orders`, and `/trades` enforce server-side zero-lookahead at `<= session.current_index` with **no unrestricted `include_future=true` escape hatch during active Replay**.
   - Persisted orders and trades created at `<= session.current_index` whose fill, status transition, or exit occurred at a bar `> session.current_index` have their future state fully masked (returned as pending/open at `current_index` with future `filled_price`, `filled_at`, `exit_price`, `exit_bar_index`, `closed_at`, and `realized_pnl` masked).
   - Post-session audit access (`include_future=true` or full-session ledger inspection) is governed by explicit server-side eligibility rules (permitted only when `session.status in ("COMPLETED", "EXPIRED", "BANKRUPT")` or `session.current_index >= total_bars - 1`; rejected or ignored during active mid-replay).
   - `total_bars` and `earliest_release_date` in `backend/app/services/practice_workflow_service.py` are clamped to `session.end_date` and `session.current_index`, and naive vs UTC-aware `datetime` comparisons are normalized.
3. **Server-Side Replay Indicator Timestamp Clamping & Typed `MappingResult` Contract (`B11`)**:
   - `backend/app/domain/engine/indicator_engine.py` and `backend/app/api/replay.py` clamp all Replay indicator series to `timestamp <= max_visible_ts` (`candles[current_index].timestamp`) on the server so forward-projected series (`Ichimoku Senkou Span A/B +26`) never leak future timestamps during Replay.
   - Non-Replay chart behavior where forward projection (`Ichimoku +26` cloud ahead of the latest market bar) is intentionally supported is preserved.
   - Typed warming/error metadata compatibility (`MappingResult`: `OK | EMPTY_WARMING | UNMAPPED_COLUMN | INVALID_SPEC`) is maintained so the client can distinguish warming indicators (`current_index < warmup_bars`) from backend mapping/calculation errors.

---

### 2. EXACT BATCH SEQUENCE, PROMPT REFERENCES & GAP COVERAGE

| Execution Order | Batch ID | Canonical Specification Reference | Gap IDs Closed | Dependency Rule |
|---|---|---|---|---|
| **1st (Sequential)** | **`B03`** | `docs/plans/DEV_BATCHES.md` (`B03`) & `docs/plans/SUMI_V3_MASTER_REFACTOR_PLAN.md` | `GAP-DB-05`, `GAP-DB-06`, `GAP-DB-10`, `GAP-DB-14` | Requires `Gate G-W0A` (`W0-A` committed). Must complete and pass verification before `B10` writes begin. |
| **2nd (Sequential)** | **`B10`** | `docs/plans/prompts/B10_replay-ledger.md` & `docs/plans/DEV_BATCHES.md` (`B10`) | `GAP-REPLAY-01`, `GAP-EXEC-08`, `GAP-DB-09` | Starts implementation writes only after `B03` is accepted. Must complete and pass verification before `B11` writes begin. |
| **3rd (Sequential)** | **`B11`** | `docs/plans/prompts/B11_projected.md` & `docs/plans/DEV_BATCHES.md` (`B11`) | `GAP-REPLAY-02` | Starts implementation writes only after `B10` is accepted. |

Read the canonical specifications (`docs/plans/DEV_BATCHES.md`, `docs/plans/prompts/B10_replay-ledger.md`, `docs/plans/prompts/B11_projected.md`), `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`, `ADR-001`, `ADR-TX-001`, `ADR-EXEC-001`, and `ADR-CHART-001` before modifying any file.

---

### 3. PRECONDITIONS & WORKSPACE SAFETY (VERIFY BEFORE CODING)

The **Orchestrator** must verify all preconditions before authorizing `B03` implementation writes. **STOP immediately** if any check fails:
1. `git status --porcelain` shows zero uncommitted changes to tracked application files (`W0-A` is committed cleanly).
2. Do **not** access, read, modify, migrate, chmod, or delete `backend/sumi.db`. Rely on `backend/conftest.py` and `backend/tests_safety.py` (`SUMI_PROD_DB_SHA256` session lock) to enforce database invariance automatically.
3. Confirm all pytest runs execute strictly against isolated `tmp_path` SQLite databases.
4. Keep all verification logs and reports inside `/Users/mizuhara/workspace/sumi/artifacts/local/` — never write files to `/tmp`, parent directories, or Antigravity brain.

---

### 4. LOGICAL AGENT ROLES & PARALLEL VS. SEQUENTIAL COORDINATION

1. **Orchestrator (`Lead Architect & Sequential Gatekeeper`)**:
   - Enforces the strict sequential write order **`B03` → `B10` → `B11`**.
   - Dispatches read-only specification investigators / reviewers in parallel while ensuring **only one implementation writer is active at a time**.
   - Verifies each batch's acceptance criteria and diff before unlocking the next batch.
2. **Backend Implementer (`Single Active Writer — Sequential B03 → B10 → B11`)**:
   - Executes implementation writes sequentially: first `B03`, then (after `B03` acceptance) `B10`, then (after `B10` acceptance) `B11`.
3. **Frontend Type Adapter Support (`Sequential Tail of B11 — If Needed`)**:
   - After `B11` backend changes are complete, adds optional additive TypeScript types for `indicator_meta` / `MappingResult` in `frontend/src/types/` or `frontend/src/services/` only if required for type compatibility prior to `B32` / `B40`.
4. **Migration & Schema Investigator (`Parallel Read-Only Support for B03`)**:
   - Performs read-only inspection of `backend/alembic/env.py`, `backend/alembic/versions/` (`cdf80254e9dc` through `a2d639ecd5d5_add_execution_candle_index.py`), and `backend/app/models/__init__.py` (20 models) to deliver the migration-chain compatibility assessment before `B03` edits begin.
5. **Quant/Financial Reviewer (`Parallel Specification & Post-Batch Zero-Lookahead Auditor for B10 & B11`)**:
   - Designs the `B10` and `B11` zero-lookahead test cases in parallel (read-only) while `B03` is being implemented, then audits the `B10` and `B11` diffs against `ADR-001` and Golden Case `G-10` (`docs/testing/FINANCIAL_GOLDEN_CASES.md`).
6. **QA/Integration Reviewer (`Batch Gate Verifier`)**:
   - Runs the isolated pytest gate after `B03`, after `B10`, and after `B11`, followed by full backend pytest and frontend Vitest + `npm run build` at wave closeout.

---

### 5. SEQUENTIAL FILE OWNERSHIP PER BATCH PHASE

Only one batch phase may perform application writes at any given time:

| Sequential Phase | Authorized Write Scope | Parallel Read-Only Activities Permitted |
|---|---|---|
| **Phase 1: `B03` Implementation & Acceptance** | `backend/alembic/env.py`<br/>`backend/alembic/versions/<new_b03_corrective_migration>.py`<br/>*(Existing revisions in `backend/alembic/versions/` may only be touched if the compatibility assessment proves an idempotency guard is strictly required)*<br/>`backend/app/services/import_workflow_service.py`<br/>`backend/app/seed_cafef.py`<br/>`backend/tests/test_b03_alembic_and_preview_isolation.py` | Quant Reviewer & Test Designer prepare `B10` and `B11` zero-lookahead test matrices in read-only mode. |
| **Phase 2: `B10` Implementation & Acceptance** *(Starts only after Phase 1 passes)* | `backend/app/api/decisions.py`<br/>`backend/app/services/practice_workflow_service.py`<br/>`backend/app/schemas/` *(if needed for masked order/trade responses)*<br/>`backend/tests/test_b10_replay_lookahead_lock.py` | Quant Reviewer audits `B03` closeout & prepares `B11` Ichimoku/+26 & `MappingResult` verification cases in read-only mode. |
| **Phase 3: `B11` Implementation & Acceptance** *(Starts only after Phase 2 passes)* | `backend/app/domain/engine/indicator_engine.py`<br/>`backend/app/api/replay.py`<br/>`backend/app/schemas/replay_schema.py` *(or `backend/app/schemas/replay.py`)*<br/>`frontend/src/types/` *(additive optional `indicator_meta` types only, if needed)*<br/>`backend/tests/test_b11_indicator_projection_lock.py` | Quant Reviewer & QA Reviewer audit `B11` diffs and run full wave regression verification. |

---

### 6. AUTHORIZED CHANGES — DETAILED SPECIFICATION BY SEQUENTIAL BATCH

#### 6.1 Phase 1 — Batch `B03` (`GAP-DB-05`, `GAP-DB-06`, `GAP-DB-10`, `GAP-DB-14`)

1. **Migration-Chain Compatibility Assessment (Mandatory First Step)**:
   - Inspect all 9 existing revisions in `backend/alembic/versions/` (`cdf80254e9dc` through `a2d639ecd5d5_add_execution_candle_index.py`) and compare against the 20 ORM models in `backend/app/models/__init__.py`.
   - **Do NOT automatically rewrite existing applied Alembic revisions.** Document any gap between the existing 9-revision chain and `Base.metadata`, and prefer an **additive corrective migration** at the head of the chain where appropriate.
2. **Exact 20-Model Registry Verification (`backend/alembic/env.py` — `GAP-DB-05`)**:
   - Update `backend/alembic/env.py` so it imports the complete canonical model registry from `app.models` (`backend/app/models/__init__.py`).
   - Verify the **exact expected table/model registry** (all 20 models: `Symbol`, `Candle`, `ReplaySession`, `Decision`, `Order`, `Execution`, `Position`, `Trade`, `JournalEntry`, `EventLog`, `DrawingState`, `StrategyLabRun`, `ScannerRun`, `ImportRun`, `ImportRunItem`, `ImportRunMutation`, `WeeklyCandleProvenance`, `SyncRun`, `SyncRunItem`, `SyncRunMutation`), **not merely `len(metadata.tables) >= 20`**.
3. **Additive Corrective Migration (`GAP-DB-06`, `GAP-DB-14`)**:
   - Create a new additive Alembic revision chained after `a2d639ecd5d5_add_execution_candle_index.py` that adds missing composite indexes and reconciles any remaining schema/index drift using inspector-guarded idempotent helpers (`_has_table`, `_has_column`, `_has_index`) so `alembic upgrade head` succeeds cleanly on both (a) a brand-new empty SQLite database and (b) a database pre-initialized via `Base.metadata.create_all()`.
4. **Preview Read-Only Isolation Across All Workflow Tables (`GAP-DB-10`)**:
   - In `backend/app/services/import_workflow_service.py` (when `confirm=False` / preview mode) and `backend/app/seed_cafef.py` (when `dry_run=True`), ensure preview/dry-run execution never persists mutations.
   - Verify read-only behavior across **all tables potentially affected by the workflow** (`candles`, `symbols`, `import_runs`, `import_run_items`, `import_run_mutations`, `weekly_candle_provenance`, `sync_runs`, `sync_run_items`, `sync_run_mutations`, `event_logs`), asserting before/after row counts and state are identical.

#### 6.2 Phase 2 — Batch `B10` (`GAP-REPLAY-01`, `GAP-EXEC-08`, `GAP-DB-09`)

1. **No Unrestricted `include_future=true` Escape Hatch During Active Replay (`backend/app/api/decisions.py` — `GAP-REPLAY-01`)**:
   - On `GET /api/sessions/{session_id}/decisions`, `GET /api/sessions/{session_id}/orders`, and `GET /api/sessions/{session_id}/trades`:
     - By default (`include_future=False`), strictly clamp all returned records to `<= session.current_index`.
     - **Do NOT provide an unrestricted `include_future=true` escape hatch during active Replay.** If a caller passes `include_future=true` while the session is still active (`session.status == "ACTIVE"` and `session.current_index < total_bars - 1`), the server must reject the request with `HTTP 403`/`HTTP 400` (or strictly enforce `include_future=False`) so a user or frontend bug cannot bypass zero-lookahead mid-session.
     - **Explicit Post-Session Audit Eligibility Rules**: `include_future=true` (unmasked full-session audit access) is permitted **only** when `session.status in ("COMPLETED", "EXPIRED", "BANKRUPT")` or `session.current_index >= total_bars - 1`.
2. **Full Future-State Masking of Persisted Orders & Trades (`GAP-REPLAY-01`)**:
   - Filtering only by creation/submission bar index is insufficient when a user steps backward in Replay or submits an order that fills/exits on a later bar:
     - **Decisions**: Return only decisions with `candle_index <= session.current_index`.
     - **Orders**: Return only orders submitted at `<= session.current_index`. If an order was filled, cancelled, or modified at a bar `> session.current_index` (or timestamp `> current_candle.timestamp`), mask its state at `current_index` as open/pending (`status="PENDING"`, `filled_price=None`, `filled_at=None`, `filled_bar_index=None`) so future execution outcomes never leak.
     - **Trades**: Return only trades whose entry occurred at `<= session.current_index`. If a trade exited/closed at a bar `> session.current_index` (or timestamp `> current_candle.timestamp`), mask its future exit state at `current_index` (`status="OPEN"`, `exit_price=None`, `exit_bar_index=None`, `closed_at=None`, `realized_pnl=None`, `holding_bars` clamped to `current_index - entry_bar_index`).
3. **Practice Horizon Clamping & Datetime Normalization (`backend/app/services/practice_workflow_service.py` — `GAP-EXEC-08`, `GAP-DB-09`)**:
   - Clamp `total_bars` calculation to only count candles within `[session.start_date, session.end_date]` (`GAP-EXEC-08`).
   - Clamp `earliest_release_date` and settlement queries so they never inspect candles beyond `session.end_date` or `session.current_index`.
   - Normalize naive vs UTC-aware `datetime` comparisons (`GAP-DB-09`) using a deterministic helper across Replay and Practice workflows.

#### 6.3 Phase 3 — Batch `B11` (`GAP-REPLAY-02`)

1. **Server-Side Replay Indicator Timestamp Clamping (`backend/app/domain/engine/indicator_engine.py` & `backend/app/api/replay.py`)**:
   - In Replay mode, determine `max_visible_ts = visible_candles[-1].timestamp` (`candles[current_index].timestamp`).
   - Clamp every Replay indicator output series on the server so no point with `timestamp > max_visible_ts` is ever serialized or transmitted to the client (specifically clamping forward-shifted `Ichimoku Senkou Span A/B +26` bars).
2. **Preserve Non-Replay Forward Projection Behavior**:
   - Where non-Replay full-market chart endpoints intentionally support forward projection (`Ichimoku +26` cloud ahead of the latest market bar), preserve that non-Replay behavior (e.g., via an explicit `clamp_to_last_candle: bool = True` parameter in Replay calls vs `False` in non-Replay full-chart calls).
3. **Maintain Typed Warming/Error Metadata Compatibility (`MappingResult` — `ADR-CHART-001`)**:
   - Ensure Replay indicator responses include typed metadata (`OK | EMPTY_WARMING | UNMAPPED_COLUMN | INVALID_SPEC`, `warmup_bars`, `available_bars`) while preserving backward-compatible series payloads so existing chart consumers continue to work seamlessly before `B32` and `B40`.

---

### 7. EXPLICIT NON-GOALS (DO NOT TOUCH IN THIS WAVE)

- **Do NOT** implement `B20–B22` (`BacktestExecutionKernel`, `strategy_lab_service.py`, `backtest_service.py`, `settlement.py`, `metrics.py`).
- **Do NOT** implement `B30–B32`, `B40–B41`, or `B50–B51` (note: `B51` will later require integrating a real Single Backtest workspace/tab before redirecting `/backtest` to `/strategy-lab?tab=single`).
- **Do NOT** touch, read, migrate, chmod, or re-seed `backend/sumi.db` during this wave.
- **Do NOT** weaken server-side zero-lookahead or disable any safeguard in `backend/conftest.py` or `backend/tests_safety.py`.

---

### 8. REQUIRED UNIT, INTEGRATION & GOLDEN TESTS

All tests must execute against isolated `tmp_path` SQLite fixtures and pass with `0` failures:

1. **`backend/tests/test_b03_alembic_and_preview_isolation.py` (Gate 1 — Must pass before `B10` begins)**:
   - `test_alembic_env_registers_exact_20_expected_models`: Verifies that `Base.metadata.tables` in `alembic/env.py` matches the exact set of 20 table names from `backend/app/models/__init__.py`.
   - `test_alembic_upgrade_head_on_empty_and_precreated_sqlite_db`: Runs `alembic upgrade head` on (a) an empty `tmp_path` SQLite DB and (b) a `tmp_path` SQLite DB initialized via `Base.metadata.create_all()`; asserts zero `OperationalError` and verifies expected composite indexes exist.
   - `test_import_preview_confirm_false_is_read_only_across_all_workflow_tables`: Snapshots row counts across all 10 workflow tables (`candles`, `symbols`, `import_runs`, `import_run_items`, `import_run_mutations`, `weekly_candle_provenance`, `sync_runs`, `sync_run_items`, `sync_run_mutations`, `event_logs`), runs CSV preview (`confirm=False`), and asserts `0` row delta across every table.
   - `test_seed_cafef_dry_run_is_read_only_across_all_workflow_tables`: Runs `seed_cafef(..., dry_run=True)` on a temp DB and asserts `0` row delta across all workflow tables.
2. **`backend/tests/test_b10_replay_lookahead_lock.py` (Gate 2 — Must pass before `B11` begins)**:
   - `test_g10_active_replay_filters_future_records_and_masks_future_fills_and_exits`: Creates a session with `100` bars; records an order/trade entered at bar `15` that filled/exited at bar `30`, plus a decision/order/trade at bar `45`; steps backward to `current_index = 20`; asserts:
     - `/decisions`, `/orders`, `/trades` completely exclude bar `45` items.
     - The bar `15` order/trade has its bar `30` fill/exit state masked (`status="OPEN"`, `exit_price=None`, `realized_pnl=None`) at `current_index = 20`.
   - `test_g10_active_replay_rejects_unrestricted_include_future_escape_hatch`: Calls `/decisions?include_future=true`, `/orders?include_future=true`, and `/trades?include_future=true` while `session.status == "ACTIVE"` and `current_index == 20 < total_bars - 1`; asserts the server rejects or ignores `include_future=true` and leaks zero future data.
   - `test_g10_post_session_audit_eligibility_allows_completed_session`: Marks session `COMPLETED` (or advances to `current_index == total_bars - 1`) and verifies `include_future=true` returns the full unmasked audit ledger.
   - `test_practice_horizon_clamped_to_end_date_and_utc_normalized`: Seeds `200` bars, creates a session whose `end_date` covers `120` bars, and asserts `total_bars == 120` and `earliest_release_date` never queries beyond `current_index` / `end_date`.
3. **`backend/tests/test_b11_indicator_projection_lock.py` (Gate 3 — Must pass before Wave Closeout)**:
   - `test_g10_replay_ichimoku_future_cloud_clamped_at_current_index`: Requests Replay state + `ICHIMOKU(9,26,52)` at `current_index = 60`; asserts every returned point across `TENKAN`, `KIJUN`, `SENKOU_A`, `SENKOU_B`, and `CHIKOU` has `timestamp <= candles[60].timestamp`.
   - `test_non_replay_chart_preserves_intentional_forward_projection`: Verifies that non-Replay full-chart indicator calculation preserves forward-shifted `SENKOU_A` / `SENKOU_B` points when Replay clamping is not enabled.
   - `test_indicator_meta_warming_and_error_contract_compatibility`: Requests `SMA(50)` at `current_index = 10`; asserts typed metadata reports `EMPTY_WARMING` (`warmup_bars == 50`, `available_bars == 11`) without breaking series serialization.

---

### 9. EVIDENCE RETENTION (`artifacts/local/`) & STOP CONDITIONS

Retain all verification outputs inside `/Users/mizuhara/workspace/sumi/artifacts/local/`:
- `artifacts/local/w1_b03_migration_compatibility_assessment.md`
- `artifacts/local/w1_pytest_b03.log`, `artifacts/local/w1_pytest_b10.log`, `artifacts/local/w1_pytest_b11.log`
- `artifacts/local/w1_pytest_full_regression.log`
- `artifacts/local/w1_frontend_vitest_and_build.log`
- `artifacts/local/w1_quant_zero_lookahead_audit.md`

**STOP immediately and escalate to the Orchestrator if**:
1. The `B03` migration-chain compatibility assessment finds an irreconcilable conflict in existing applied revisions that cannot be solved via `env.py` + an additive corrective migration + idempotent guards.
2. Any existing `W0-A` test (`test_b01_*`, `test_b02_*`, `test_w0a_final_adversarial.py`) fails.
3. Fixing `B03`, `B10`, or `B11` requires modifying files outside the authorized scope in Section 5.
4. `backend/tests_safety.py` detects any change to `backend/sumi.db`.

---

### 10. FINAL WAVE REPORTING FORMAT

Upon completing `B03 → B10 → B11`, the Orchestrator must output a concise wave completion report containing:
1. Confirmation of sequential execution (`B03` accepted → `B10` accepted → `B11` accepted) and database safety invariance.
2. Summary of `B03` migration-chain compatibility findings and exact 20-model registry / 10-table preview isolation proof.
3. Summary of `B10` active-session zero-lookahead, future-state masking, and post-session audit eligibility verification.
4. Summary of `B11` Replay indicator timestamp clamping, non-Replay forward-projection preservation, and typed `MappingResult` compatibility.
5. `git status --short` and `git diff --stat` summary ready for Architect review.
````
