# PRO-03 — Data Catalog and Import Quality

## Outcome

The user can inspect the exact provenance and coverage of local Daily/Weekly market histories, preview every import consequence before accepted candles change, explicitly accept an unambiguous import, safely repeat it as a no-op, and roll it back to the exact prior accepted state.

## Context and problem

Authority: `AGENTS.md`, `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`, `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`, and `docs/program/PRO_03_DATA_CATALOG_AND_IMPORT_QUALITY.md`.

The existing `CafeFImporter.import_data` combines parsing, validation, symbol creation, candle upsert, and commit. It skips malformed rows and uses `ON CONFLICT DO UPDATE`, so a conflicting file can silently overwrite accepted values. The API exposes direct mutation and the UI labels any response as successful. There is no import-run manifest, catalog provenance, Weekly derivation contract, or exact rollback record.

Acceptance IDs: `PRO-DATA-01` through `PRO-DATA-07`.

## In scope

- Versioned import-run, staged-candidate, accepted-mutation/provenance, and Weekly-provenance domain records with an Alembic migration and compatibility tests.
- Read-only catalog and import-run history APIs.
- A two-step preview → explicit accept workflow for CSV/TXT/ZIP Daily histories.
- Deterministic classification of parsed, rejected, duplicate-identical, conflicting, missing-date, and out-of-order rows before acceptance.
- SHA-256, source filename, source type, parser version, symbol/timeframe/adjustment/timezone semantics, outcome, and timestamps in the manifest.
- Idempotent re-acceptance and fail-closed ambiguity/conflict behavior.
- Adjustment isolation and explicit conflict resolution; no silent overwrite.
- Deterministic Daily→Weekly aggregation and provenance using `VN_TRADING_WEEK_V1`.
- Exact, conflict-aware rollback of the newest eligible accepted import.
- Product UI for catalog, preview, confirmation, outcome, history, and rollback.
- Focused backend/frontend tests and additive product-UAT assertions/evidence.
- All supported import entrypoints, including scripts, must use the safe application service or be explicitly disabled; no bypass may silently mutate accepted candles.

## Out of scope

- Network provider selection or sync (PRO-10/PRO-11).
- Intraday data, corporate-action adjustment calculation, or guessing an adjustment type.
- Editing accepted candle values by hand.
- General database backup UI or release packaging.
- Indicator, replay, strategy, scanner, or journal feature expansion except regression compatibility.
- PRO-04 or later work.

## Invariants

- Never mutate `backend/sumi.db` during development, tests, or UAT. Record its SHA-256 before and after every full gate.
- Accepted candle data changes only after a preview checksum is explicitly accepted in one transaction.
- Ambiguous date, timezone, source, symbol, timeframe, or adjustment semantics block acceptance.
- Adjusted and unadjusted histories remain distinct in keys, catalog rows, preview counts, manifests, and rollback.
- Existing accepted values are never overwritten merely because a later file differs.
- Repeating the same semantic import is a deterministic no-op with a recorded outcome.
- Backend services own parsing, classification, acceptance, aggregation, and rollback; routes only validate transport and call services.
- Weekly derivation never fabricates Daily candles or uses external/network data.
- Preserve local-first behavior and all accepted PRO-00–PRO-02 contracts.
- Do not weaken, delete, rename, duplicate, or make non-blocking an accepted UAT assertion.

## Current architecture

- `backend/app/api/import_data.py`: direct `/cafef` upload mutation and destructive `/benchmark` replacement.
- `backend/app/services/cafef_importer.py`: parses CSV/TXT/ZIP, drops invalid rows, de-duplicates by keeping last, upserts candle conflicts, and commits.
- `backend/app/services/data_quality_service.py`: row-level OHLCV validation but no semantic ambiguity or accepted-data conflict classification.
- `backend/app/schemas/import_schema.py`: only aggregate counters/warnings.
- `backend/app/models/candle.py` and `symbol.py`: accepted candles and symbol metadata; no run/provenance records.
- `backend/alembic/versions/`: versioned schema migration mechanism.
- `frontend/src/pages/ImportPage.tsx` and `frontend/src/api/importApi.ts`: one-step import UI/API.
- `backend/scripts/import_batch.py` and `backend/seed_cafef.py`: direct importer callers that could bypass a safe workflow.
- `scripts/product-uat.mjs` and `scripts/fixtures/product-uat-v3-baseline.json`: fail-closed browser acceptance harness and manifest.

## Target design

### Domain ownership

Create an application service (for example `ImportWorkflowService`) that owns preview, accept, history, and rollback. Keep file parsing in a parser adapter and classification in pure/testable domain functions. Routes must not open transactions or implement conflict rules.

Persist a versioned run manifest and enough staged/mutation data to prove and reverse each accepted change. Models may be split differently after repository inspection, but the durable contract must represent:

- immutable run identity, file SHA-256, semantic-content checksum, parser version, source filename/type;
- explicit `1D`/`1W`, `Asia/Ho_Chi_Minh`, and adjustment type;
- lifecycle (`previewed`, `blocked`, `accepted`, `noop`, `rolled_back`, `failed`) and counts;
- staged normalized rows tied to the checksum used for explicit acceptance;
- every inserted/deleted/restored candle before/after image and provenance;
- Weekly output membership/provenance back to accepted Daily candle keys and source runs.

### API/data flow

Provide transport-equivalent operations with stable typed schemas:

1. `POST /api/import/preview`: upload plus explicit source/timeframe/adjustment/timezone semantics; parse and classify without changing accepted Symbol/Candle data.
2. `POST /api/import/runs/{run_id}/accept`: require the preview/semantic checksum and explicit confirmation; reject blocked/stale/tampered previews; apply once atomically.
3. `GET /api/import/runs`: list manifests/outcomes.
4. `POST /api/import/runs/{run_id}/rollback`: restore exact before-images only when no later/current change makes rollback unsafe; otherwise fail closed with a reason.
5. `GET /api/data/catalog`: return source, symbol, timeframe, adjustment, first/last timestamp, row count, last accepted update, and provenance/derivation state.

Names may follow existing route conventions, but all operations and response fields above are required. Legacy `/api/import/cafef`, `/api/import/benchmark`, seed, and batch entrypoints must not remain an unreviewed mutation bypass. Preserve compatibility only if it can require the same explicit safe contract; otherwise fail with an actionable response. Network benchmark acquisition remains deferred to PRO-10/11.

### Classification contract

- `parsed`: syntactically normalized source rows.
- `rejected`: invalid OHLCV, invalid/non-trading date, missing required value, or unsupported semantic value.
- `duplicate`: identical normalized keys/values inside the file or against accepted data; conflicting duplicates are conflicts, not silently last-wins.
- `conflicting`: same `(symbol,timeframe,timestamp,adjustment)` with different accepted or intra-file values.
- `missing`: gaps between first/last eligible weekday dates; report them without inventing candles or guessing holidays.
- `out_of_order`: a valid row appearing before an earlier source row for the same symbol/history.

Any ambiguity or unresolved conflict sets `can_accept=false`. Rejected rows are visible; the default workflow must not quietly accept a partial file. If a bounded explicit partial-accept option is introduced, it requires Reviewer approval and separate manifest semantics; it is not assumed by this plan.

### Weekly rule

`VN_TRADING_WEEK_V1` converts timestamps to `Asia/Ho_Chi_Minh` calendar dates, accepts Monday–Friday dates only, groups by Monday-starting week, orders actual accepted Daily dates, uses first open/max high/min low/last close/summed volume, and timestamps the Weekly candle at the final included trading date. Missing weekdays are reported but not synthesized or guessed as holidays. Every Weekly candle stores the ordered Daily member keys/checksums and derivation-rule version. Re-derivation is deterministic and scoped to affected symbol/adjustment weeks.

### UI

Replace the one-click success surface with catalog and import workflow states: selecting semantics/file, preview table/counts, blocked reasons, explicit confirmation, accepted/no-op outcome, run history, and guarded rollback. Use Vietnamese user-facing copy and existing formatting/error conventions. The UI must never imply success for a blocked or partially parsed file.

## Milestones

1. **Contract and migration:** add schemas/models/migration and pure parser/classifier fixtures. Exit: migration upgrade on a temporary copy and focused tests cover every classification and legacy reads.
2. **Safe application service:** implement preview, checksum, atomic accept/no-op, manifest, adjustment isolation, and rollback. Exit: conflicting/ambiguous fixtures leave accepted-data hashes/counts unchanged; rollback restores exact pre-import state.
3. **Catalog and Weekly provenance:** implement catalog query and `VN_TRADING_WEEK_V1`. Exit: hand-calculated multi-week/gap/adjustment fixtures prove deterministic OHLCV and provenance.
4. **API/UI integration:** expose typed operations and replace direct-import UX. Exit: frontend tests cover preview/block/accept/no-op/history/rollback; all mutation entrypoints are accounted for.
5. **Product evidence and handoff:** add manifest-protected UAT journey, run full gates, inspect both screenshots, reconcile hashes/process cleanup, update this plan and ledger, then stop at Reviewer gate.

## Acceptance mapping

| Acceptance ID | Required implementation evidence | Required test/UAT evidence |
| --- | --- | --- |
| PRO-DATA-01 | Catalog query/schema with provenance and coverage | API fixtures and browser catalog assertions |
| PRO-DATA-02 | Pure preview classifier and typed counts/details | Malformed, duplicate, conflict, gap, and out-of-order fixtures before mutation |
| PRO-DATA-03 | Explicit semantic fields and fail-closed guards | Ambiguous date/timezone/symbol/timeframe/adjustment rejection with unchanged DB |
| PRO-DATA-04 | Immutable run manifest, checksums, parser version, no-op outcome | Same file and semantically identical file repeated deterministically |
| PRO-DATA-05 | `VN_TRADING_WEEK_V1` and member provenance | Hand-calculated gap/week-boundary/adjustment fixtures |
| PRO-DATA-06 | Adjustment-key isolation and conflict quarantine | Adjusted/unadjusted coexist; conflicting accepted values remain unchanged |
| PRO-DATA-07 | Before/after mutation journal and guarded rollback | Exact pre/post dataset digest comparison and unsafe rollback rejection |

## Verification commands

Run focused tests first, adding exact file/test selectors to the progress log. Then run from repository root as applicable:

```powershell
git diff --check
Push-Location backend; pytest; Pop-Location
Push-Location frontend; npm test -- --run; npm run lint; npm run build; Pop-Location
./scripts/verify-v2.ps1
./scripts/run-product-uat.ps1
./scripts/verify-product.ps1
Get-FileHash -Algorithm SHA256 backend/sumi.db
```

If only `.sh` wrappers exist for a gate, use the repository-supported shell equivalent and record the exact command. Product UAT must retain machine-readable results, runtime/API errors, catalog/import screenshots at 1440×1000 and 1280×800, and manifest reconciliation. Run the final standalone product UAT twice consecutively if import cleanup or process timing is changed.

## Rollback and compatibility

- Add schema only through an Alembic migration; prove upgrade against a temporary copied database and preserve existing Candle/Symbol reads.
- Existing accepted candles initially appear as legacy/local catalog data with honest unknown provenance; do not invent manifests.
- Product rollback restores exact recorded before-images in one transaction and refuses unsafe/out-of-order rollback.
- Code rollback must not strand a partially migrated production DB; document downgrade/read compatibility before Reviewer gate.
- No test or UAT may use the production database.

## Risks and mitigations

- Silent conflict overwrite: eliminate last-wins behavior and test unchanged database digests.
- Huge staged imports: use bounded/chunked persistence and payload summaries; test representative volume without embedding files in API results.
- SQLite transaction/locking: keep mutation atomic and bounded; test interruption/failure rollback.
- Weekly holiday ambiguity: report missing weekdays and derive only from present accepted Daily rows under the versioned rule.
- Legacy scripts bypass service: inventory all callers and route or disable each one.
- Stale preview/tampering: bind acceptance to immutable run id and semantic checksum.
- Rollback after later changes: fail closed when current after-images or dependency order do not match.

## Progress log

- 2026-08-09: Independent Reviewer framed PRO-03 after PRO-02 approval, audited the current import path, and created the stable dossier, detailed ExecPlan, and low-model DEV prompt. Product implementation has not started.
- 2026-08-09: Milestone 1 complete. Added `ImportRun`, `ImportRunItem`, `ImportRunMutation`, and `WeeklyCandleProvenance` domain models, Pydantic schemas, Alembic migration `20260810_0001_import_catalog_quality`, pure `ImportClassifier`, and 7 focused unit tests (`app/tests/test_import_classifier.py` - 7 passed). Tested migration upgrade on temporary database.
- 2026-08-09: Milestones 2-5 complete. Implemented `ImportWorkflowService`, `WeeklyAggregator` (`VN_TRADING_WEEK_V1`), FastAPI endpoints in `app/api/import_data.py` and `app/api/symbols.py`, frontend API client in `importApi.ts` and Vietnamese UI surface in `ImportPage.tsx`. Unit tests (`137 passed, 1 skipped`), frontend tests (`155 passed`), lint (`0 errors`), build (`tsc -b && vite build` passed) and Product UAT (`305/305 passed, 0 failed, 0 blocking failed`) all green.
- 2026-08-10: PRO-03 REWORK-01..04 complete (partially approved by Reviewer on 2026-08-10 with REWORK-03 and REWORK-05 open).
- 2026-08-10: PRO-03 REWORK-03 & REWORK-05 complete:
  - **REWORK-03**: Enforced explicit confirmation parameter (`confirm_accept=True`, matching `run_id`, matching `content_sha256`) in `CafeFImporter.import_data()`. Any direct invocation without confirmation generates preview only and raises `RuntimeError` without mutating any `Symbol` or `Candle` data (0 bytes mutated). Updated `seed_cafef.py` and `import_batch.py` to require `--confirm` CLI flag for explicit acceptance. Added regression test `test_import_data_without_confirmation_raises_error_and_does_not_mutate_data`.
  - **REWORK-05**: Fixed test DB isolation in `conftest.py` by converting `client` fixture from module-scope to function-scope (`@pytest.fixture(scope="function")`) and dropping/recreating database tables before and after every test function. Focused test suite `pytest app/tests/test_import_classifier.py app/tests/test_import_api.py app/tests/test_cafef_importer.py app/tests/test_import_workflow.py app/tests/test_weekly_aggregator.py -q` passes 27/27 green in forward and reverse collection order.
  - All gates green: full pytest 140 passed, frontend vitest 155 passed, lint 0 errors, build succeeded, `git diff --check` passed (exit code 0), and Product UAT 305/305 passed.
- 2026-08-10: Independent Reviewer R2 approved and closed PRO-03. Focused tests passed 27/27 in forward and reverse order; `scripts/verify-v2.ps1` passed with the production DB hash unchanged; independent Product UAT passed 305/305 with no runtime errors in `test-results/product-uat/2026-08-10T13-19-58-163Z`. Verdict record: `docs/reviews/PRO_03_REVIEW_2026-08-10_R2.md`. PRO-04 was not started.

## Decision log

- Use just-in-time detailed planning for the active batch; future PRO dossiers remain stable framing to avoid stale implementation prescriptions.
- Require preview and explicit acceptance as separate operations; direct last-wins upsert is incompatible with PRO-DATA-02/06.
- Use mutation before/after evidence rather than claiming that a database backup alone proves logical rollback.
- Define `VN_TRADING_WEEK_V1` now so a lower-model session does not invent calendar semantics.
- Disable legacy `/api/import/cafef` route with explicit HTTP 400 error rather than allowing unconfirmed direct auto-acceptance.
- Require explicit confirmation input (`confirm_accept=True`, matching `run_id`, matching `content_sha256`) on `CafeFImporter.import_data()`, `seed_cafef.py` (`--confirm`), and `import_batch.py` (`--confirm`) to prevent accidental unconfirmed candle mutations.
- Isolate test fixtures cleanly per test function (`@pytest.fixture(scope="function")` for `client` and `db_session`) with explicit table teardown to eliminate collection-order test dependencies.

## Completion evidence

- **Changed Files Inventory**:
  - Backend domain/models: `backend/app/models/import_run.py`, `backend/app/models/__init__.py`
  - Alembic migration: `backend/alembic/versions/20260810_0001_import_catalog_quality.py`
  - Backend schemas/services: `backend/app/schemas/import_schema.py`, `backend/app/services/import_classifier.py`, `backend/app/services/weekly_aggregator.py`, `backend/app/services/import_workflow_service.py`, `backend/app/services/cafef_importer.py`
  - Importer scripts: `backend/seed_cafef.py`, `backend/scripts/import_batch.py`
  - Backend API routes: `backend/app/api/import_data.py`, `backend/app/api/symbols.py`
  - Backend test suites: `backend/app/tests/test_import_classifier.py`, `backend/app/tests/test_weekly_aggregator.py`, `backend/app/tests/test_import_workflow.py`, `backend/app/tests/test_import_api.py`, `backend/app/tests/test_cafef_importer.py`, `backend/app/tests/conftest.py`
  - Verification script isolation: `scripts/verify-v2.ps1`
  - Frontend API & UI: `frontend/src/api/importApi.ts`, `frontend/src/pages/ImportPage.tsx`
  - Product UAT harness & manifest: `scripts/product-uat.mjs`, `scripts/fixtures/product-uat-v3-baseline.json`
  - State & Plan: `docs/exec-plans/PRO_03_DATA_CATALOG_AND_IMPORT_QUALITY.md`, `docs/AUTONOMOUS_EXECUTION_STATE.md`

- **Test Counts & Verification Gates**:
  - Focused pytest command (`app/tests/test_import_classifier.py app/tests/test_import_api.py app/tests/test_cafef_importer.py app/tests/test_import_workflow.py app/tests/test_weekly_aggregator.py`): **27 passed** (forward and reverse collection order).
  - Full `pytest`: **140 passed**.
  - `npm test -- --run`: **155 passed**.
  - `npm run lint`: 0 errors.
  - `npm run build`: `tsc -b && vite build` succeeded.
  - `git diff --check`: **0 errors** (exit code 0).
  - `./scripts/verify-v2.ps1`: PASSED.
  - `./scripts/run-product-uat.ps1`: **305/305 passed**, 0 failed, 0 blocking failed.

- **UAT Artifacts & Screenshot Retention**:
  - Results JSON: `test-results/product-uat/2026-08-10T13-12-23-223Z/results.json`
  - Catalog Screenshot (1440×1000): `test-results/product-uat/2026-08-10T13-12-23-223Z/pro03-catalog-1440x1000.png` (224,068 bytes, SHA256 `6e5e8e3d08fb01ea187b415a77f98c89bfdb8ed0bc0359873d6b0439c4d9cb5d`)
  - Import Preview Screenshot (1280×800): `test-results/product-uat/2026-08-10T13-12-23-223Z/pro03-import-preview-1280x800.png` (173,167 bytes, SHA256 `ebbe61eecfe5ff402c4dfdb80fcecfcfbce91ef78f0d01c64eb3074092bbf585`)

- **Database Integrity**:
  - Production DB (`backend/sumi.db`) SHA-256 Before: `F890F5BC16ECE557EA78E19A6095A362DE8641E341382DF66D6A9C997E84F080`
  - Production DB (`backend/sumi.db`) SHA-256 After: `F890F5BC16ECE557EA78E19A6095A362DE8641E341382DF66D6A9C997E84F080`
  - Result: Production database remains completely unchanged (`productionUnchanged: true`, 0 bytes mutated).


