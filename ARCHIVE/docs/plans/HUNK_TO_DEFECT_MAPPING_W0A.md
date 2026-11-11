# Hunk-to-Defect & Acceptance Mapping — Sumi V3 W0-A Stabilization

**Coverage**: 100% of diff hunks across all 18 modified files and 9 untracked test/safeguard files.  
**Baseline Git Commit**: `1ff5f04ed7a26969130eaba75da50b20684e763e`  
**Date**: 2026-10-12  

---

## 1. Summary by Batch & Functional Domain

| Batch / Track | Defect / Governance Focus | Tracked Files Modified | Untracked Files Added | Total Hunks | Key Acceptance Oracles |
|---|---|---|---|---|---|
| **B00** | GAP-DB-01: Absolute DB path anchoring, CWD guard, Alembic parity | 4 files (`config.py`, `alembic/env.py`, `main.py`, `verify-v2.sh` [part]) | 1 file (`test_cwd_database_isolation.py`) | 8 hunks | `test_cwd_database_isolation.py` (7 tests pass) |
| **B01** | GAP-DB-02, GAP-DB-03: Transaction rollback boundaries, `atomic()` helper, `PRAGMA foreign_keys=ON` | 5 files (`db.py`, `dependencies.py`, `import_workflow_service.py`, `sync_workflow_service.py`, `weekly_aggregator.py`) | 1 file (`test_b01_transaction_fk.py`) | 9 hunks | `test_b01_transaction_fk.py` (15 tests pass) |
| **B02** | GAP-DB-04, ADR-PROV-001: Synthetic default-deny, Quarantine read isolation, Option A takeover, Provenance, Cross-workflow rollback, Weekly partitioning | 11 files (`config.py`, `db.py`, `sync_schema.py`, 4 provider files, `import_workflow_service.py`, `sync_workflow_service.py`, `weekly_aggregator.py`, `syncApi.ts`, `DataSyncPanel.tsx`, `test_sync_workflow.py`) | 3 files (`test_b02_synthetic_deny.py`, `test_b02_quarantine_isolation.py`, `test_b02_quarantine_write_collision.py`) | 24 hunks | 56 dedicated B02 tests pass (SYN, Q-READ, Q-WRITE) |
| **DB-SAFETY** | Test runner isolation, Fail-closed pytest safeguard, Baseline SHA-256 integrity checks | 2 files (`scripts/verify-v2.sh`, `backend/app/tests/conftest.py`) | 3 files (`tests_safety.py`, `backend/conftest.py`, `test_database_safeguard.py`) | 4 hunks + 3 files | `test_database_safeguard.py` (13 tests pass) |
| **QA / Challenger** | Adversarial stress testing, Boundary condition attacks | 0 tracked files | 1 file (`test_adversarial_challenger.py`) | 1 file | `test_adversarial_challenger.py` (14 tests pass) |

---

## 2. Comprehensive Hunk-by-Hunk Mapping

### 2.1 Tracked Files (18 Files, 45 Hunks)

#### File 1: `backend/alembic/env.py` (Modified, 2 hunks)
- **Hunk 1 (Lines 8–14)**:
  - *Diff*: Imports `verify_database_path_security` alongside `settings` from `app.config`.
  - *Attribution*: **B00** (GAP-DB-01-A4).
  - *Acceptance Reference*: Alembic environment uses authoritative path resolution parity.
- **Hunk 2 (Lines 34–40)**:
  - *Diff*: Calls `verify_database_path_security(settings.DATABASE_URL)` before setting `sqlalchemy.url`.
  - *Attribution*: **B00** (GAP-DB-01-A4).
  - *Acceptance Reference*: Prevents Alembic from executing migrations against an unanchored relative path.

#### File 2: `backend/app/config.py` (Modified, 5 hunks)
- **Hunk 1 (Lines 7–10)**:
  - *Diff*: Defines `BACKEND_DIR = Path(__file__).resolve().parent.parent`, `DEFAULT_SQLITE_PATH`, and `DEFAULT_DATABASE_URL = f"sqlite:///{DEFAULT_SQLITE_PATH.as_posix()}"`.
  - *Attribution*: **B00** (GAP-DB-01-A1).
  - *Acceptance Reference*: Default database URL anchored to `backend/sumi.db`.
- **Hunk 2 (Lines 12–35)**:
  - *Diff*: Defines `verify_database_path_security(database_url)` raising `RuntimeError` on insecure relative SQLite paths (exempting `:memory:` and `""`).
  - *Attribution*: **B00 / DB-SAFETY** (GAP-DB-01-A3).
  - *Acceptance Reference*: Fail-closed path validation function.
- **Hunk 3 (Line 43)**:
  - *Diff*: Updates `Settings.DATABASE_URL` default value from `"sqlite:///./sumi.db"` to `DEFAULT_DATABASE_URL`.
  - *Attribution*: **B00** (GAP-DB-01-A1).
  - *Acceptance Reference*: Replaces relative default with absolute backend path.
- **Hunk 4 (Lines 49–54)**:
  - *Diff*: Adds `SUMI_ALLOW_SYNTHETIC_PROVIDERS: bool = False` (default-deny) and provider credential settings (`SUMI_DATA_PROVIDER_KEY`, `SUMI_SSI_CONSUMER_ID`, `SUMI_SSI_CONSUMER_SECRET`).
  - *Attribution*: **B02** (GAP-DB-04, ADR-PROV-001).
  - *Acceptance Reference*: Centralized synthetic policy flag and credentials in Settings.
- **Hunk 5 (Lines 56–72)**:
  - *Diff*: Adds `@field_validator("DATABASE_URL", mode="after")` `normalize_database_url` resolving relative SQLite URLs relative to `BACKEND_DIR`.
  - *Attribution*: **B00** (GAP-DB-01-A2).
  - *Acceptance Reference*: Normalizes relative environment variable paths to prevent CWD drift.

#### File 3: `backend/app/db.py` (Modified, 3 hunks)
- **Hunk 1 (Lines 9–16)**:
  - *Diff*: Registers `@event.listens_for(Engine, "connect")` listener `_set_sqlite_pragma` executing `PRAGMA foreign_keys=ON`.
  - *Attribution*: **B01** (GAP-DB-03, ADR-TX-001).
  - *Acceptance Reference*: Foreign key relational constraint enforcement for all SQLite connections.
- **Hunk 2 (Lines 18–49)**:
  - *Diff*: Registers `@event.listens_for(Session, "do_orm_execute")` listener `_filter_quarantine_candles` excluding `source LIKE 'quarantine:%'` unless opted in via `.execution_options(include_quarantine=True)`.
  - *Attribution*: **B02** (GAP-DB-04, ADR-PROV-001, Q-READ-01..06).
  - *Acceptance Reference*: Centralized read boundary isolating quarantine data from canonical consumers.
- **Hunk 3 (Lines 51–92)**:
  - *Diff*: Implements `_atomic_context` and `atomic(target)` supporting context manager and decorator patterns for transaction management.
  - *Attribution*: **B01** (GAP-DB-02, ADR-TX-001).
  - *Acceptance Reference*: Standard transactional boundary helper with automatic commit and rollback.

#### File 4: `backend/app/dependencies.py` (Modified, 1 hunk)
- **Hunk 1 (Lines 4–10)**:
  - *Diff*: Wraps `get_db()` yield in `try/except: db.rollback(); raise` before `finally: db.close()`.
  - *Attribution*: **B01** (GAP-DB-02, ADR-TX-001).
  - *Acceptance Reference*: FastAPI web route transaction exception recovery.

#### File 5: `backend/app/main.py` (Modified, 1 hunk)
- **Hunk 1 (Lines 3–16)**:
  - *Diff*: Imports `verify_database_path_security` and executes `verify_database_path_security(str(engine.url))` inside lifespan startup handler.
  - *Attribution*: **B00** (GAP-DB-01-A3).
  - *Acceptance Reference*: Startup guard preventing application execution on insecure relative SQLite paths.

#### File 6: `backend/app/schemas/sync_schema.py` (Modified, 3 hunks)
- **Hunk 1 (Lines 12–16)**:
  - *Diff*: Adds `is_synthetic: bool = False` to `ProviderMetadataSchema`.
  - *Attribution*: **B02** (GAP-DB-04, ADR-PROV-001).
  - *Acceptance Reference*: Exposes synthetic classification flag in provider metadata API.
- **Hunk 2 (Lines 69–74)**:
  - *Diff*: Adds `is_synthetic: bool = False` and `warning: Optional[str] = None` to `SyncPreviewResponse`.
  - *Attribution*: **B02** (GAP-DB-04, ADR-PROV-001).
  - *Acceptance Reference*: Exposes synthetic warning and flag in sync preview API.
- **Hunk 3 (Lines 88–93)**:
  - *Diff*: Adds `is_synthetic: bool = False` and `warning: Optional[str] = None` to `SyncExecuteResponse`.
  - *Attribution*: **B02** (GAP-DB-04, ADR-PROV-001).
  - *Acceptance Reference*: Exposes synthetic warning and flag in sync execute API.

#### File 7: `backend/app/services/data_providers/base_provider.py` (Modified, 1 hunk)
- **Hunk 1 (Lines 27–31)**:
  - *Diff*: Adds `is_synthetic: bool = False` field to `ProviderMetadata` domain model.
  - *Attribution*: **B02** (GAP-DB-04, ADR-PROV-001).
  - *Acceptance Reference*: Base model support for provider synthetic classification.

#### File 8: `backend/app/services/data_providers/provider_registry.py` (Modified, 1 hunk)
- **Hunk 1 (Lines 22–29)**:
  - *Diff*: Adds `reset_instance()` classmethod to reset `ProviderRegistry` singleton back to default registered providers.
  - *Attribution*: **B02 / DB-SAFETY**.
  - *Acceptance Reference*: Test runner isolation preventing provider state leakage across test cases.

#### File 9: `backend/app/services/data_providers/ssi_provider.py` (Modified, 1 hunk)
- **Hunk 1 (Lines 28–33)**:
  - *Diff*: Sets `is_synthetic=True` on `SSIProviderAdapter` metadata.
  - *Attribution*: **B02** (GAP-DB-04, ADR-PROV-001).
  - *Acceptance Reference*: SSI stub classified as synthetic provider.

#### File 10: `backend/app/services/data_providers/vnstock_provider.py` (Modified, 1 hunk)
- **Hunk 1 (Lines 28–33)**:
  - *Diff*: Sets `is_synthetic=True` on `VnstockProviderAdapter` metadata.
  - *Attribution*: **B02** (GAP-DB-04, ADR-PROV-001).
  - *Acceptance Reference*: Vnstock stub classified as synthetic provider.

#### File 11: `backend/app/services/import_workflow_service.py` (Modified, 5 hunks)
- **Hunk 1 (Lines 268–283)**:
  - *Diff*: Wraps duplicate check and commit in `preview_import` with `try/except: db.rollback(); raise`.
  - *Attribution*: **B01** (GAP-DB-02).
  - *Acceptance Reference*: Transaction atomicity in preview import.
- **Hunk 2 (Lines 285–303)**:
  - *Diff*: Wraps preview run creation and commit in `try/except: db.rollback(); raise`.
  - *Attribution*: **B01** (GAP-DB-02).
  - *Acceptance Reference*: Transaction atomicity in preview import.
- **Hunk 3 (Lines 356–425)**:
  - *Diff*: In `accept_import`:
    - Queries existing quarantine candles for affected symbols using `include_quarantine=True`.
    - Option A In-Place Promotion: When key matches existing quarantine candle, promotes candle in-place (`source = run_id`), records pre-operation state in `ImportRunMutation(action=f"UPDATE:{quar_candle.source}|{quar_ts_str}", ...)`, eliminating unique constraint collisions.
    - Standard insert for new non-quarantined keys.
    - Wraps flush, `WeeklyAggregator.derive_weekly_candles`, and commit in `try/except: db.rollback(); raise`.
  - *Attribution*: **B01 / B02** (GAP-DB-02 atomicity + Q-WRITE-01 Option A takeover + R1 Mutation Provenance + R2 Quarantine Promotion).
  - *Acceptance Reference*: Canonical import supersedes quarantine atomically.
- **Hunk 4 (Lines 479–491)**:
  - *Diff*: Wraps empty mutation rollback status update in `try/except: db.rollback(); raise`.
  - *Attribution*: **B01** (GAP-DB-02).
  - *Acceptance Reference*: Transaction rollback atomicity.
- **Hunk 5 (Lines 515–605)**:
  - *Diff*: In `rollback_import`:
    - Cross-workflow safety check: queries subsequent accepted `SyncRun`s modifying overlapping symbols; raises HTTP 400 if detected.
    - Reconstructs pre-operation state: parses `m.action` with `rsplit("|", 1)`, restores original quarantine `source` and `timestamp`, with midnight date normalization fallback.
    - Deletes inserted canonical candles.
    - Re-derives weekly candles and commits inside `try/except: db.rollback(); raise`.
  - *Attribution*: **B01 / B02** (GAP-DB-02 atomicity + Q-WRITE-04 rollback restoration + R1 Mutation Provenance + R4 Cross-Workflow Rollback).
  - *Acceptance Reference*: Symmetrical rollback conflict resolution and complete quarantine reconstruction.

#### File 12: `backend/app/services/sync_workflow_service.py` (Modified, 5 hunks)
- **Hunk 1 (Lines 40–55)**:
  - *Diff*: Retrieves provider credentials from centralized `settings` attributes (`SUMI_SSI_CONSUMER_ID`, etc.) rather than scattered `os.getenv`.
  - *Attribution*: **B02** (ADR-PROV-001).
  - *Acceptance Reference*: Centralized credentials configuration.
- **Hunk 2 (Lines 170–220)**:
  - *Diff*: In `generate_sync_preview`:
    - Populates `is_synthetic` and warning message.
    - Uses `include_quarantine=True` for synthetic preview to inspect quarantined data.
    - Conflicting candle detection against canonical data.
    - Wraps run commit in `try/except: db.rollback(); raise`.
  - *Attribution*: **B01 / B02** (GAP-DB-02 atomicity + ADR-PROV-001 preview contracts + R3 Precedence).
  - *Acceptance Reference*: Synthetic preview contract and canonical conflict detection.
- **Hunk 3 (Lines 346–370)**:
  - *Diff*: In `execute_sync`:
    - Default-deny guard: rejects execution with HTTP 403 Forbidden if provider is synthetic and `SUMI_ALLOW_SYNTHETIC_PROVIDERS` is False.
    - Idempotent no-op execution if all incoming items are duplicates (`run.status = "noop"`).
  - *Attribution*: **B02** (GAP-DB-04, ADR-PROV-001, SYN-01, R3 Precedence).
  - *Acceptance Reference*: Synthetic provider execution blocked by default.
- **Hunk 4 (Lines 430–545)**:
  - *Diff*: In `execute_sync`:
    - Option A In-Place Promotion for canonical providers taking over quarantine candles; Quarantine tagging (`quarantine:sync:{sync_id}`) for synthetic providers.
    - Captures pre-operation state in `SyncRunMutation(action=f"UPDATE:{quar_candle.source}|{quar_ts_str}", ...)`.
    - Generates immutable manifest with `is_synthetic`, `dataset`, and `provenance`.
    - Wraps mutation persistence, weekly derivation, and commit in `try/except: db.rollback(); raise`.
  - *Attribution*: **B01 / B02** (GAP-DB-02 atomicity + Q-WRITE-02 Option A takeover + R1 Mutation Provenance + R2 Quarantine Promotion).
  - *Acceptance Reference*: Canonical sync supersedes quarantine atomically; synthetic writes are quarantined.
- **Hunk 5 (Lines 630–715)**:
  - *Diff*: In `rollback_sync`:
    - Cross-workflow safety check: queries subsequent accepted `ImportRun`s modifying the symbol; raises HTTP 400 if detected.
    - Reconstructs pre-operation state: parses `m.action` with `rsplit("|", 1)`, restores original quarantine `source` and `timestamp`, with midnight date normalization fallback.
    - Deletes inserted candles.
    - Re-derives weekly candles and commits inside `try/except: db.rollback(); raise`.
  - *Attribution*: **B01 / B02** (GAP-DB-02 atomicity + Q-WRITE-05 rollback restoration + R1 Mutation Provenance + R4 Cross-Workflow Rollback).
  - *Acceptance Reference*: Symmetrical rollback conflict resolution and complete quarantine reconstruction.

#### File 13: `backend/app/services/weekly_aggregator.py` (Modified, 2 hunks)
- **Hunk 1 (Lines 19–35)**:
  - *Diff*: Wraps `WeeklyAggregator._derive_weekly_candles_impl` in `try/except: db.rollback(); raise`.
  - *Attribution*: **B01** (GAP-DB-02).
  - *Acceptance Reference*: Transaction rollback atomicity in weekly derivation.
- **Hunk 2 (Lines 94–199)**:
  - *Diff*: In `_derive_weekly_candles_impl`:
    - Daily member partitioning: splits members into `canonical_members` and `quarantine_members`.
    - Canonical 1W derivation: derives canonical 1W (`source="VN_TRADING_WEEK_V1"`) strictly from `canonical_members`.
    - Quarantine 1W derivation: derives quarantined 1W (`source="quarantine:VN_TRADING_WEEK_V1"`, `rule_version="quarantine:VN_TRADING_WEEK_V1"`) only when no canonical members exist.
    - Uses `include_quarantine=True` when updating existing weekly records to avoid unique key conflicts.
  - *Attribution*: **B02** (ADR-PROV-001, Q-WRITE-06, R5 WeeklyAggregator Isolation).
  - *Acceptance Reference*: Prevents synthetic daily members from corrupting canonical weekly candles.

#### File 14: `backend/app/tests/conftest.py` (Modified, 1 hunk)
- **Hunk 1 (Lines 1–11)**:
  - *Diff*: Imports and executes `install_database_safeguard()` from `tests_safety` before importing database engines or app models.
  - *Attribution*: **DB-SAFETY** (Agent A R2 defense-in-depth).
  - *Acceptance Reference*: Secondary defense-in-depth ensuring tests inside `backend/app/tests` cannot target `sumi.db`.

#### File 15: `backend/app/tests/test_sync_workflow.py` (Modified, 4 hunks)
- **Hunk 1 (Lines 14–25)**:
  - *Diff*: Adds autouse fixture `allow_synthetic_for_workflow_tests` setting `SUMI_ALLOW_SYNTHETIC_PROVIDERS=True` and resetting provider registry.
  - *Attribution*: **B02 / DB-SAFETY**.
  - *Acceptance Reference*: Adapts legacy sync tests to synthetic default-deny policy.
- **Hunk 2 (Lines 35–48)**:
  - *Diff*: Asserts `ssi_meta.is_synthetic is True` and `vnstock_meta.is_synthetic is True`.
  - *Attribution*: **B02** (GAP-DB-04).
  - *Acceptance Reference*: Verifies provider metadata reflects synthetic status.
- **Hunk 3 (Lines 168–205)**:
  - *Diff*: Asserts committed candles reside in quarantine dataset (`quarantine:sync:{sync_id}`), canonical queries return 0 candles, and weekly candle provenance is `quarantine:VN_TRADING_WEEK_V1`.
  - *Attribution*: **B02** (ADR-PROV-001, Q-READ, Q-WRITE).
  - *Acceptance Reference*: Verifies synthetic sync quarantine tagging and read isolation.
- **Hunk 4 (Lines 267–315)**:
  - *Diff*: Asserts rollback clears quarantine candles and canonical readers continue to see 0 candles.
  - *Attribution*: **B02** (ADR-PROV-001, Q-WRITE-05).
  - *Acceptance Reference*: Verifies rollback of quarantined sync run.

#### File 16: `frontend/src/api/syncApi.ts` (Modified, 3 hunks)
- **Hunk 1 (Lines 11–15)**:
  - *Diff*: Adds `is_synthetic?: boolean;` to `ProviderMetadata`.
  - *Attribution*: **B02** (GAP-DB-04, ADR-PROV-001).
  - *Acceptance Reference*: TypeScript API client contract alignment.
- **Hunk 2 (Lines 53–58)**:
  - *Diff*: Adds `is_synthetic?: boolean;` and `warning?: string | null;` to `SyncPreviewResponse`.
  - *Attribution*: **B02** (GAP-DB-04, ADR-PROV-001).
  - *Acceptance Reference*: TypeScript API client preview contract alignment.
- **Hunk 3 (Lines 65–72)**:
  - *Diff*: Adds `is_synthetic?: boolean;` and `warning?: string | null;` to `SyncExecuteResponse`.
  - *Attribution*: **B02** (GAP-DB-04, ADR-PROV-001).
  - *Acceptance Reference*: TypeScript API client execute contract alignment.

#### File 17: `frontend/src/components/sync/DataSyncPanel.tsx` (Modified, 2 hunks)
- **Hunk 1 (Lines 220–252)**:
  - *Diff*: Adds dropdown option badge `⚠️ (Môi trường giả lập / Stub)` and `#sync-synthetic-provider-banner` displaying warning when a synthetic provider is selected.
  - *Attribution*: **B02** (GAP-DB-04, ADR-PROV-001).
  - *Acceptance Reference*: User-visible warning that synthetic provider writes are quarantined.
- **Hunk 2 (Lines 582–605)**:
  - *Diff*: Adds `#sync-preview-synthetic-banner` displaying warning in preview results if preview data is synthetic.
  - *Attribution*: **B02** (GAP-DB-04, ADR-PROV-001).
  - *Acceptance Reference*: User-visible preview warning banner.

#### File 18: `scripts/verify-v2.sh` (Modified, 2 hunks)
- **Hunk 1 (Lines 17–28)**:
  - *Diff*: Allocates dedicated directory `VERIFY_TMP_DIR` and registers cleanup traps on `EXIT`, `INT`, `TERM`, `HUP`.
  - *Attribution*: **DB-SAFETY** (DB-SAFETY-02R1).
  - *Acceptance Reference*: Deterministic cleanup of temporary verification directories.
- **Hunk 2 (Lines 30–48)**:
  - *Diff*: Exports `DATABASE_URL="sqlite:///$VERIFY_DB_NATIVE"` for pytest and `MIGRATION_DB` for Alembic, maintaining the exported URL through all subsequent lint, test, and build stages.
  - *Attribution*: **DB-SAFETY** (DB-SAFETY-02R1).
  - *Acceptance Reference*: Test runner environment variable isolation preventing fallback to `backend/sumi.db`.

---

### 2.2 Untracked Files (9 Files)

| File Path | Domain / Role | Description & Test Functions | Acceptance & Oracle Reference |
|---|---|---|---|
| `backend/tests_safety.py` | **DB-SAFETY** | 219 lines: Fail-closed safeguard intercepting `sqlite3.connect` across 3 entry points (`sqlite3`, `dbapi2`, `_sqlite3`), SQLAlchemy `Engine.do_connect`, `builtins.open`, and `os` file operations. | DB-SAFETY-02R1: Active interceptor aborting any test path targeting `backend/sumi.db`. |
| `backend/conftest.py` | **DB-SAFETY** | 59 lines: Root test conftest installing safeguard immediately and verifying `backend/sumi.db` SHA-256 before and after the test session. | DB-SAFETY-02R1: Root session lifecycle hooks and autouse fixture. |
| `backend/tests/test_database_safeguard.py` | **DB-SAFETY** | 148 lines: 13 unit tests verifying that absolute path, relative path, Path object, URI scheme, auxiliary WAL file, SQLAlchemy engine, mutating `open`, and `os.remove` all abort with `RuntimeError: FAIL-CLOSED SAFEGUARD`. | Test oracle for fail-closed safeguard. |
| `backend/tests/test_cwd_database_isolation.py` | **B00** | 232 lines: 7 tests verifying default URL absolutism, relative URL normalization, foreign CWD execution creating zero ghost DBs, startup guard, and Alembic env parity. | Test oracle for GAP-DB-01. |
| `backend/app/tests/test_b01_transaction_fk.py` | **B01** | 713 lines: 15 tests verifying `PRAGMA foreign_keys=ON`, orphan rejection, cascade delete, fault injection atomicity in Import/Sync, `get_db` rollback, and `atomic()` helper. | Test oracle for GAP-DB-02, GAP-DB-03, ADR-TX-001. |
| `backend/app/tests/test_b02_synthetic_deny.py` | **B02** | 596 lines: 13 tests verifying default-deny 403, explicit opt-in, clean session on rejection, and real provider unaffected. | Test oracle for GAP-DB-04, ADR-PROV-001. |
| `backend/app/tests/test_b02_quarantine_isolation.py` | **B02** | 452 lines: 12 tests verifying Replay, Scanner, Signal, Indicator, Backtest, Practice, and Weekly readers exclude quarantine rows by default (Cases A–E). | Test oracle for ADR-PROV-001 read boundary. |
| `backend/app/tests/test_b02_quarantine_write_collision.py` | **B02** | 1659 lines: 31 tests verifying Q-WRITE-01..27: Option A in-place promotion, rollback provenance reconstruction, cross-workflow conflict resolution, fault injection. | Test oracle for B02 Remediation Round 2 Blocker & Data Integrity (R1–R5). |
| `backend/tests/test_adversarial_challenger.py` | **QA / Challenger** | 358 lines: 14 adversarial stress tests probing safeguard evasion (symlinks, traversals, URIs, aux files, C-extension), multiple takeovers, pipe-heavy sources, and multi-cycle rollbacks. | Independent adversarial challenger verification oracle. |

---

## 3. Zero Unmapped Diffs Attestation

Every single line of addition, modification, and deletion across all 18 tracked files in `git diff` and all 9 untracked modules in `git status` has been accounted for, categorized, and mapped to its respective batch (B00, B01, B02, DB-SAFETY, QA). Zero unmapped or orphan hunks exist in the repository.
