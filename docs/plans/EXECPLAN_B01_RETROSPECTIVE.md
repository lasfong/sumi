# B01 — Retrospective ExecPlan: Transaction Boundaries & Relational Integrity

## Outcome
All backend write operations (Import, Sync, Weekly Aggregation, Decisions, Web Requests) execute inside transactional boundaries with automatic rollback on error, and all SQLite connections enforce relational integrity via foreign key constraints (`PRAGMA foreign_keys=ON`), preventing dangling/orphan records or half-committed states under fault injection.

## Context and problem
- **Review Finding**: GAP-DB-02 (Missing rollback handling across write paths), GAP-DB-03 (Missing SQLite foreign key PRAGMA).
- **Architecture Decision**: ADR-TX-001 (Transaction boundary everywhere + FK + atomic helper).
- **Acceptance IDs**: GAP-DB-02, GAP-DB-03, TX-01..TX-08.
- **Problem Statement**:
  Prior to B01, database writes across `import_workflow_service.py`, `sync_workflow_service.py`, and `weekly_aggregator.py` called bare `db.commit()` or `db.flush()` without enclosing `try/except: db.rollback(); raise` blocks. In FastAPI routes, `get_db()` yielded the session and only closed it in `finally`, leaving uncommitted or broken transactions active upon unhandled exceptions. Furthermore, SQLite disables foreign key enforcement by default, allowing orphan child records (such as order executions with nonexistent session IDs) to be inserted without error.

## In scope
- Register `@event.listens_for(Engine, "connect")` listener `_set_sqlite_pragma` in `backend/app/db.py` to enforce `PRAGMA foreign_keys=ON` across all DBAPI connections.
- Implement reusable `atomic(target)` transactional boundary helper (supporting both context manager `with atomic(db):` and decorator `@atomic`) in `backend/app/db.py`.
- Wrap session dependency `get_db()` in `backend/app/dependencies.py` with `try/except: db.rollback(); raise` before closing.
- Wrap all write and rollback methods in `backend/app/services/import_workflow_service.py`, `backend/app/services/sync_workflow_service.py`, and `backend/app/services/weekly_aggregator.py` with transactional rollback blocks.
- Author comprehensive transaction and foreign key test suite `backend/app/tests/test_b01_transaction_fk.py` (15 test functions).

## Out of scope
- Database schema changes, alterations to constraints, or Alembic migrations (retained for B03).
- Data quarantine logic or provider synthetic policy (handled in B02).
- Frontend application modifications.

## Invariants
- **All-or-Nothing Atomicity**: Any runtime exception during a multi-stage operation (e.g., daily candle persistence followed by weekly aggregation) must trigger complete session rollback with zero partial writes surviving.
- **Foreign Key Rigidity**: Every connection must enforce foreign key integrity; invalid foreign keys must raise `sqlite3.IntegrityError` / `sqlalchemy.exc.IntegrityError`.
- **Zero Production Database Mutation**: Tests must execute on isolated temporary databases (`tmp_path / "b01_test.db"`).

## Current architecture
- `backend/app/db.py`: Bare `create_engine` without connection event listeners or transaction helper utilities.
- `backend/app/dependencies.py`: `get_db()` with `try: yield db finally: db.close()`.
- Service classes: Unprotected `db.commit()` calls leaving open transactions upon failures during weekly aggregation.

## Target design
- **Connection Event Listener**:
  ```python
  @event.listens_for(Engine, "connect")
  def _set_sqlite_pragma(dbapi_connection, connection_record):
      if isinstance(dbapi_connection, sqlite3.Connection):
          cursor = dbapi_connection.cursor()
          cursor.execute("PRAGMA foreign_keys=ON")
          cursor.close()
  ```
- **Atomic Helper**:
  ```python
  @contextmanager
  def _atomic_context(session):
      try:
          yield session
          session.commit()
      except Exception:
          session.rollback()
          raise
  ```
- **Service Transaction Pattern**:
  ```python
  try:
      # Stage mutations and entities
      db.flush()
      WeeklyAggregator.derive_weekly_candles(db, ...)
      db.commit()
  except Exception:
      db.rollback()
      raise
  ```

## Milestones
1. **Milestone 1 — Engine FK PRAGMA Listener & Atomic Helper**: Implement `_set_sqlite_pragma` and `atomic()` in `backend/app/db.py`. Exit criteria: Foreign key enforcement verified on newly opened connections.
2. **Milestone 2 — Web Route Transaction Recovery**: Update `get_db()` in `backend/app/dependencies.py` with exception rollback. Exit criteria: Web route exceptions trigger rollback.
3. **Milestone 3 — Service Write-Path Hardening**: Wrap `ImportWorkflowService`, `SyncWorkflowService`, and `WeeklyAggregator` write and rollback methods in `try/except: db.rollback(); raise`. Exit criteria: Fault injection leaves zero partial state.
4. **Milestone 4 — Test Suite Verification**: Author and execute `backend/app/tests/test_b01_transaction_fk.py`. Exit criteria: 15 tests pass.

## Acceptance mapping
| Acceptance ID | Implementation Evidence | Test / Verification Evidence |
|---|---|---|
| GAP-DB-02-A1 | `backend/app/services/import_workflow_service.py` (lines 354-440, 529-605), `sync_workflow_service.py` (lines 430-515, 649-715), `weekly_aggregator.py` (lines 22-26) | `test_b01_transaction_fk.py::test_tx04_fault_injection_during_import_rolls_back_daily_and_weekly`<br>`test_b01_transaction_fk.py::test_tx05_fault_injection_during_sync_rolls_back_daily_and_weekly` |
| GAP-DB-02-A2 | `backend/app/dependencies.py:7-10` | `test_b01_transaction_fk.py::test_tx08_get_db_dependency_rolls_back_on_exception` |
| GAP-DB-02-A3 | `backend/app/db.py:51-92` (`atomic` helper) | `test_b01_transaction_fk.py::test_tx07_atomic_helper_context_manager_and_decorator` |
| GAP-DB-03-A1 | `backend/app/db.py:9-15` (`_set_sqlite_pragma`) | `test_b01_transaction_fk.py::test_tx06_sqlite_pragma_foreign_keys_on_connect_event` |
| GAP-DB-03-A2 | SQLite engine connection listener | `test_b01_transaction_fk.py::test_tx01_foreign_key_orphan_insert_rejected` |
| GAP-DB-03-A3 | SQLite engine connection listener | `test_b01_transaction_fk.py::test_tx02_cascade_delete_enforced` |

## Verification commands
```bash
# 1. Run transaction and FK test suite on isolated temporary database
TMP_DB="$(mktemp /tmp/sumi-b01-XXXXXX.db)"
DATABASE_URL="sqlite:///$TMP_DB" .venv/bin/pytest backend/app/tests/test_b01_transaction_fk.py -v
rm -f "$TMP_DB"

# 2. Verify production database hash and size remain identical
shasum -a 256 backend/sumi.db
ls -l backend/sumi.db
```

## Rollback and compatibility
- **Rollback Strategy**: Revert changes in `backend/app/db.py`, `backend/app/dependencies.py`, and service files.
- **Compatibility**: Highly compatible. Valid operations succeed as before. Any legacy operation that previously left orphaned records will now fail fast, preserving database integrity.

## Risks and mitigations
- **Risk**: SQLite in-memory connections or separate thread engines might not receive PRAGMA if opened via raw sqlite3 driver.
  - *Mitigation*: The listener attaches to the SQLAlchemy `Engine` level, covering all sessions spawned by `sessionmaker` and `engine.connect()`. Agent A's safeguard further inspects raw `sqlite3.connect` calls in tests.
- **Risk**: Calling `db.rollback()` on a closed session.
  - *Mitigation*: `get_db()` places `db.rollback()` in the `except` block *before* `finally: db.close()`.

## Progress log
- **2026-10-10**: Implemented `_set_sqlite_pragma` and `atomic()` in `backend/app/db.py`.
- **2026-10-10**: Added rollback wrapper to `get_db()` in `backend/app/dependencies.py`.
- **2026-10-10**: Hardened `ImportWorkflowService`, `SyncWorkflowService`, and `WeeklyAggregator`.
- **2026-10-10**: Implemented `backend/app/tests/test_b01_transaction_fk.py` covering TX-01..TX-08.
- **2026-10-12**: Verified by Agent C R4 challenger with fault injection tests; 15/15 tests passing.

## Decision log
- **Decision 1**: Implement `atomic()` as both context manager and decorator to accommodate differing service coding styles without forcing mass refactors.
- **Decision 2**: Keep `PRAGMA foreign_keys=ON` in an engine connect listener rather than embedding it inside individual queries, ensuring complete transparency to ORM queries.

## Completion evidence
- Test Count: 15 passed in 0.42s.
- Production DB Checksum: `ef5c035c27d49de1de51a59e28e1cac5109d92240bcb600239f2180d49128f3e` (unchanged).
- Fault Injection: Zero orphaned or partial records survive simulated weekly derivation crashes.
