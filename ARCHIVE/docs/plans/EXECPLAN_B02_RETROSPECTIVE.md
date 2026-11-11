# B02 — Retrospective ExecPlan: Synthetic Provider Default-Deny & Quarantine Data Remediation

## Outcome
Synthetic stub providers (SSI, Vnstock) are blocked by default (HTTP 403) from writing simulated market data into the database; when explicitly enabled via configuration (`SUMI_ALLOW_SYNTHETIC_PROVIDERS=1`), synthetic candles are isolated in a quarantined dataset (`source LIKE 'quarantine:%'`) invisible to canonical trading workflows (Replay, Scanner, Signal, Backtest, Practice). Furthermore, real canonical data (CafeF Import, real provider Sync) supersedes quarantine rows atomically via Option A in-place promotion without unique constraint collisions, maintaining full mutation audit provenance (`UPDATE:{source}|{ts}`) and symmetrical cross-workflow rollback safety.

## Context and problem
- **Review Finding**: GAP-DB-04 (Stub SSI/Vnstock providers write synthetic data into canonical tables without quarantine), B02 Remediation Round 2 Blocker (Quarantine and canonical unique key collision `(symbol, timeframe, timestamp, adjustment_type)` causing INSERT failures and sync classification blocks).
- **Architecture Decision**: ADR-PROV-001 (Provider & Synthetic Data Policy: quarantine by default-deny).
- **Acceptance IDs**: GAP-DB-04, Q-READ-01..06, Q-WRITE-01..27, SYN-01..08, ADV-SYN-01..07, R1–R5 Data Integrity.
- **Problem Statement**:
  Prior to B02, stub providers (`SSIProviderAdapter`, `VnstockProviderAdapter`) generated algorithmic candles (`md5 + sin` functions) without real broker HTTP connections, yet `execute_sync` saved them directly into `Candle` with `source=f"sync:{sync_id}"`. Canonical workflows treated these synthetic bars as genuine market data.
  When initial read isolation was added (`source LIKE 'quarantine:%'`), a critical write collision emerged: because `Candle` enforces a unique constraint on `(symbol, timeframe, timestamp, adjustment_type)`, canonical Import (which does not see quarantine rows in Preview) attempted an `INSERT` upon Accept, crashing with `sqlite3.IntegrityError: UNIQUE constraint failed`. In Sync, inspecting quarantine rows caused real provider data to be misclassified as duplicate or conflicting.

## In scope
- Centralized configuration settings in `backend/app/config.py`: `SUMI_ALLOW_SYNTHETIC_PROVIDERS: bool = False` (default-deny) and provider credential declarations.
- Provider classification in `backend/app/services/data_providers/`: `is_synthetic: bool` on `ProviderMetadata`, flagging SSI and Vnstock stubs as synthetic.
- Centralized read isolation boundary in `backend/app/db.py`: SQLAlchemy ORM session listener `_filter_quarantine_candles` excluding `source LIKE 'quarantine:%'` unless opted in via `.execution_options(include_quarantine=True)`.
- Contract alignment in `backend/app/schemas/sync_schema.py`: Exposing `is_synthetic` and warning banners across metadata, preview, and execution responses.
- Option A in-place promotion in `backend/app/services/import_workflow_service.py` and `backend/app/services/sync_workflow_service.py`: Canonical writes atomically take over existing quarantine rows without UNIQUE constraint collisions.
- Mutation provenance logging: Recording `UPDATE:{source}|{prior_ts_iso}` in `ImportRunMutation` and `SyncRunMutation` to preserve complete pre-operation quarantine state.
- Robust rollback state reconstruction: Parsing provenance via `rsplit("|", 1)`, restoring original quarantine source and timestamp with date normalization fallbacks.
- Symmetrical cross-workflow rollback conflict resolution: Blocking rollback if an overlapping subsequent accepted Import or Sync run exists (fail-closed HTTP 400).
- Weekly Aggregator daily member partitioning in `backend/app/services/weekly_aggregator.py`: Deriving canonical 1W bars strictly from canonical daily bars, and quarantined 1W bars (`quarantine:VN_TRADING_WEEK_V1`) when only quarantined daily bars exist.
- Frontend transparency in `DataSyncPanel.tsx` and `syncApi.ts`: Yellow warning banners and badges for synthetic providers and preview payloads.
- Test suites: `test_b02_synthetic_deny.py` (13 tests), `test_b02_quarantine_isolation.py` (12 tests), `test_b02_quarantine_write_collision.py` (31 tests).

## Out of scope
- Database schema migrations or creating a separate `quarantine_candles` table (avoided to prevent schema drift and keep B03 blocked).
- Real broker OAuth / network integration for SSI or Vnstock.
- Modifying backtesting execution kernels, order sizing, or fee models.

## Invariants
- **Real Canonical Precedence**: Canonical real data always takes precedence over synthetic quarantine data. Quarantine data must never permanently reserve or block a canonical candle key.
- **Read Boundary Isolation**: Canonical ORM queries must never return quarantine candles unless explicitly requested via `include_quarantine=True`.
- **Synthetic Never Overwrites Canonical**: Synthetic sync must never overwrite or downgrade existing canonical candles.
- **Rollback Consistency**: Reversing an operation that took over quarantine candles must completely restore the original quarantine state, values, and provenance.
- **Cross-Workflow Safety**: An earlier import or sync cannot be rolled back if a newer accepted operation has modified the same symbols.

## Current architecture
- Previously, synthetic stub providers committed directly to `Candle` with `source=sync:id`.
- During Round 1 remediation, read isolation caused `UniqueConstraint` crashes upon canonical write.
- Weekly aggregator mixed all daily members into a single 1W bar regardless of provenance.

## Target design
- **Option A Architectural Pattern**:
  ```python
  # During Preview: Canonical read excludes quarantine -> Row classified as new (parsed)
  # During Accept:
  existing_quar = db.query(Candle).execution_options(include_quarantine=True)...
  if existing_quar:
      mutation = ImportRunMutation(
          action=f"UPDATE:{existing_quar.source}|{existing_quar.timestamp.isoformat()}",
          before_open=existing_quar.open, ...
      )
      existing_quar.open = item.open
      existing_quar.source = run_id  # Promoted to canonical
  else:
      db.add(Candle(...))  # Normal insert
  ```
- **Provenance & Rollback Reconstruction**:
  ```python
  if m.action.startswith("UPDATE"):
      if ":" in m.action:
          parts = m.action.split(":", 1)[1]
          if "|" in parts:
              src, prior_ts_str = parts.rsplit("|", 1)
              candle.source = src
              candle.timestamp = datetime.fromisoformat(prior_ts_str)
  ```
- **Weekly Daily Member Partitioning**:
  ```python
  canonical_members = [c for c in members_sorted if not (c.source and c.source.startswith("quarantine:"))]
  quarantine_members = [c for c in members_sorted if c.source and c.source.startswith("quarantine:")]
  if canonical_members:
      target_members = canonical_members
      w_source = "VN_TRADING_WEEK_V1"
  elif quarantine_members:
      target_members = quarantine_members
      w_source = "quarantine:VN_TRADING_WEEK_V1"
  ```

## Milestones
1. **Milestone 1 — Synthetic Provider Default-Deny & Schema Contracts**: Implement `SUMI_ALLOW_SYNTHETIC_PROVIDERS`, 403 guard, DTO flags, and UI banners. Exit criteria: `test_b02_synthetic_deny.py` passes.
2. **Milestone 2 — Centralized Read Isolation Boundary**: Implement `_filter_quarantine_candles` in `app.db`. Exit criteria: `test_b02_quarantine_isolation.py` passes (Replay, Scanner, Signal see zero quarantine bars).
3. **Milestone 3 — Option A In-Place Promotion**: Implement atomic takeover in Import and Sync workflows. Exit criteria: Q-WRITE-01 and Q-WRITE-02 pass without `IntegrityError`.
4. **Milestone 4 — Mutation Provenance & Symmetrical Rollback**: Implement `UPDATE:{source}|{ts}` capture, `rsplit("|", 1)` parser, and cross-workflow conflict checks. Exit criteria: Q-WRITE-04, 05, 17, 18, 20 pass.
5. **Milestone 5 — Weekly Aggregator Daily Partitioning**: Implement member filtering and quarantine weekly tagging. Exit criteria: Q-WRITE-06, 10, 14 pass.

## Acceptance mapping
| Acceptance ID | Implementation Evidence | Test / Verification Evidence |
|---|---|---|
| GAP-DB-04-A1 | `backend/app/services/sync_workflow_service.py:346-355` | `test_b02_synthetic_deny.py::test_syn01_synthetic_provider_rejected_by_default` |
| GAP-DB-04-A2 | `backend/app/services/sync_workflow_service.py:465-475` | `test_b02_synthetic_deny.py::test_syn03_explicit_opt_in_executes_quarantined_dataset` |
| GAP-DB-04-A3 | `backend/app/db.py:29-49` (`_filter_quarantine_candles`) | `test_b02_quarantine_isolation.py::test_q_read_01_replay_service_excludes_quarantine` |
| Q-WRITE-01 | `backend/app/services/import_workflow_service.py:382-420` | `test_b02_quarantine_write_collision.py::test_q_write_01_canonical_import_supersedes_quarantine` |
| Q-WRITE-02 | `backend/app/services/sync_workflow_service.py:460-501` | `test_b02_quarantine_write_collision.py::test_q_write_02_canonical_sync_supersedes_quarantine` |
| Q-WRITE-03 | `backend/app/services/sync_workflow_service.py:177-189, 362` | `test_b02_quarantine_write_collision.py::test_q_write_03_quarantine_must_never_overwrite_canonical` |
| Q-WRITE-04 | `backend/app/services/import_workflow_service.py:536-590` | `test_b02_quarantine_write_collision.py::test_q_write_04_import_rollback_restores_quarantine` |
| Q-WRITE-05 | `backend/app/services/sync_workflow_service.py:650-703` | `test_b02_quarantine_write_collision.py::test_q_write_05_sync_rollback_restores_quarantine` |
| Q-WRITE-06 | `backend/app/services/weekly_aggregator.py:97-120` | `test_b02_quarantine_write_collision.py::test_q_write_06_weekly_derivation_after_canonical_takeover` |
| R1-PROV | `import_workflow_service.py:575-585`, `sync_workflow_service.py:688-698` | `test_b02_quarantine_write_collision.py::test_q_write_18_pipe_heavy_source_name_provenance_roundtrip`<br>`test_b02_quarantine_write_collision.py::test_q_write_12_import_rollback_restores_quarantine_with_normalized_midnight_timestamp` |
| R4-CROSS | `import_workflow_service.py:518-534`, `sync_workflow_service.py:631-647` | `test_b02_quarantine_write_collision.py::test_q_write_17_cross_workflow_conflict_sync_blocks_import_rollback`<br>`test_b02_quarantine_write_collision.py::test_q_write_20_cross_workflow_conflict_import_blocks_sync_rollback` |

## Verification commands
```bash
# 1. Run B02 test suites on isolated temporary database
TMP_DB="$(mktemp /tmp/sumi-b02-XXXXXX.db)"
DATABASE_URL="sqlite:///$TMP_DB" .venv/bin/pytest \
  backend/app/tests/test_b02_synthetic_deny.py \
  backend/app/tests/test_b02_quarantine_isolation.py \
  backend/app/tests/test_b02_quarantine_write_collision.py \
  backend/app/tests/test_sync_workflow.py \
  backend/app/tests/test_import_workflow.py \
  backend/app/tests/test_weekly_aggregator.py -v
rm -f "$TMP_DB"

# 2. Verify production database hash remains identical
shasum -a 256 backend/sumi.db
```

## Rollback and compatibility
- **Rollback Strategy**: Revert changes in `config.py`, `db.py`, schemas, providers, and workflow services.
- **Compatibility**: Retains full compatibility with legacy mutation logs (`action="UPDATE"` without source/timestamp) via fallback branches. Existing canonical databases require zero schema migrations.

## Risks and mitigations
- **Risk**: Deleting and inserting rows (Option B) could create temporary key collisions or fail foreign keys.
  - *Mitigation*: Option A promotes existing rows in-place, preserving candle IDs and eliminating constraint violations.
- **Risk**: Quarantine sources containing delimiter characters (e.g. `quarantine:sync:sub|part|id`) could corrupt string parsing.
  - *Mitigation*: Uses `rsplit("|", 1)` splitting from the right, ensuring source integrity regardless of internal pipes.

## Progress log
- **2026-10-10**: Implemented synthetic default-deny and quarantine tagging.
- **2026-10-11**: Identified Round 2 write collision blocker. Designed Option A in-place promotion.
- **2026-10-11**: Implemented Option A takeover in `ImportWorkflowService` and `SyncWorkflowService`. Added `test_b02_quarantine_write_collision.py`.
- **2026-10-12**: Remediated 5 core integrity focus areas (provenance encoding, Option A promotion, synthetic precedence, cross-workflow rollback conflict resolution, weekly daily member partitioning). Verified 100% pass rate across 56 B02 tests.

## Decision log
- **Decision 1**: Adopt Option A (in-place promotion) instead of Option B (delete + insert).
  - *Rationale*: Avoids ID churn, simplifies transactional locks, and guarantees atomic replacement under existing SQLite unique index without requiring schema modifications.
- **Decision 2**: Retain single `Candle` table with `source LIKE 'quarantine:%'` discrimination rather than introducing a separate table.
  - *Rationale*: Eliminates the need for an Alembic schema migration (keeping B03 strictly blocked) while maintaining absolute read isolation via SQLAlchemy ORM session criteria filter.

## Completion evidence
- Test Counts:
  - `test_b02_synthetic_deny.py`: 13 passed
  - `test_b02_quarantine_isolation.py`: 12 passed
  - `test_b02_quarantine_write_collision.py`: 31 passed
  - Related workflow tests: 19 passed
  - Total B02 verification: 75 passed with 0 failures
- Production DB Checksum: `ef5c035c27d49de1de51a59e28e1cac5109d92240bcb600239f2180d49128f3e` (unchanged).
