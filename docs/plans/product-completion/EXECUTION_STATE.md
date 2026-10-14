# SUMI V3 — DEVELOPMENT TRAIN 01 EXECUTION STATE

- **Train**: Development Train 01 (`W0-A Closure → B03 → B10 → B11`)
- **Current Phase**: **Phase A Complete / Baseline Committed (DEC-01 Approved; Ready for Phase B `B03`)**
- **Baseline Git HEAD**: `1ff5f04ed7a26969130eaba75da50b20684e763e` (`master`)
- **Production DB Invariant (`backend/sumi.db`)**: `234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86` (`188,416` bytes — `0` bytes delta verified)

---

## Batch Status Summary

### 1. Phase A — `W0-A` (`B00`, `B01`, `B02` + `DB-SAFETY`)
- **Scope**:
  - `B00` (`GAP-DB-01`): Anchor default `DATABASE_URL` to `backend/sumi.db` via `Path(__file__)`, enforce `verify_database_path_security()` and `normalize_database_url` in `config.py`, FastAPI lifespan startup guard (`main.py`), and `alembic/env.py`.
  - `B01` (`GAP-DB-02`, `GAP-DB-03`): Enforce `PRAGMA foreign_keys=ON` on SQLite connections, add `atomic()` helper in `db.py`, wrap `get_db()` in `dependencies.py` and write/rollback paths in `import_workflow_service.py`, `sync_workflow_service.py`, and `weekly_aggregator.py` with `try/commit/except-rollback`.
  - `B02` (`GAP-DB-04`, `ADR-PROV-001`): Default-deny (`HTTP 403`) on synthetic providers (`SSI`, `Vnstock` stubs) unless `SUMI_ALLOW_SYNTHETIC_PROVIDERS=1`; quarantine read filter (`_filter_quarantine_candles`); Option A in-place quarantine promotion and fail-closed `HTTP 400` pre-validation on multi-candidate same-date collisions (`ADR-PROV-001-ADDENDUM-01`); structured `UPDATE:{source}|{ts}` rollback provenance with fail-closed `HTTP 400` validation (`ADR-TX-001-ADDENDUM-01`); symmetrical cross-workflow rollback checks (`ADR-TX-001-ADDENDUM-02`); daily canonical/quarantine partitioning in `WeeklyAggregator`.
  - `DB-SAFETY`: Multi-layer test safeguard (`backend/tests_safety.py`, `backend/conftest.py`) and isolated test runner (`scripts/verify-v2.sh`).
- **Implementation Status**: `COMPLETE_COMMITTED` (Baseline Commit 1 `e34516c` and Commit 2 established).
- **Test Evidence**:
  - Live isolated Phase A verification run (`artifacts/local/development-train-01/phase_a_w0a_verification.log`): `140 passed` in `3.25s` across all 8 W0-A/safeguard/adversarial suites (`test_cwd_database_isolation.py`, `test_database_safeguard.py`, `test_b01_transaction_fk.py`, `test_b02_synthetic_deny.py`, `test_b02_quarantine_isolation.py`, `test_b02_quarantine_write_collision.py`, `test_adversarial_challenger.py`, `test_w0a_final_adversarial.py`), `0` bytes delta on `backend/sumi.db`.
  - Preserved full-suite W0-A logs (`artifacts/local/W0-A/final-remediation/logs/`): `537 passed, 1 skipped` (backend pytest), `239 passed` (frontend Vitest), `0` errors (`npm run build`).
- **Internal QA Result**: `PASS — BASELINE COMMITTED` (Zero technical or migration blockers).
- **Open Issues**: None in `W0-A` scope (`B03` 20-model Alembic registration and preview isolation queued for Phase B).
- **Dependencies**: DEC-01 approved (`HEAD` `1ff5f04` -> Commit 1 `e34516c`).
- **Next Action**: DEC-01 approved by Product Owner; W0-A baseline committed. Phase A Complete. Ready for Phase B (`B03`).

---

### 2. Phase B — `B03` (`Schema, Migration Compatibility & Preview Isolation`)
- **Scope**: Register all 20 ORM models in `backend/alembic/env.py` (`GAP-DB-05`), assess 9-revision Alembic chain and add inspector-guarded additive corrective migration for composite indexes/idempotency (`GAP-DB-06`, `GAP-DB-10`), and isolate CSV preview (`confirm=False`) and `seed_cafef(..., dry_run=True)` as 100% read-only across all 10 workflow tables (`GAP-DB-14`).
- **Implementation Status**: `NOT_STARTED` (Blocked on Phase A commit gate).
- **Test Evidence**: Pending (`backend/tests/test_b03_alembic_and_preview_isolation.py`).
- **Internal QA Result**: `PENDING`
- **Open Issues**: None.
- **Dependencies**: Authorized `W0-A` baseline commit (`Gate G-W0A`).
- **Next Action**: Execute immediately after `W0-A` baseline commit is completed and verified.

---

### 3. Phase C — `B10` (`Replay Ledger, Horizon & Eligibility Zero-Lookahead Lock`)
- **Scope**: Enforce `<= session.current_index` filtering on `/api/sessions/{id}/decisions`, `/orders`, and `/trades`; mask future fills, status transitions, exits, and realized PnL for records created at `<= current_index` but filled/closed at `> current_index`; reject/ignore `include_future=true` during active Replay while permitting post-session audit on completed/expired/bankrupt/end-of-bar sessions (`GAP-REPLAY-01`); clamp `total_bars` and `earliest_release_date` and normalize naive vs UTC datetimes in `practice_workflow_service.py` (`GAP-EXEC-08`, `GAP-DB-09`).
- **Implementation Status**: `NOT_STARTED` (Sequential after `B03`).
- **Test Evidence**: Pending (`backend/tests/test_b10_replay_lookahead_lock.py`).
- **Internal QA Result**: `PENDING`
- **Open Issues**: None.
- **Dependencies**: `B03` internal acceptance.
- **Next Action**: Execute automatically after `B03` passes internal QA.

---

### 4. Phase D — `B11` (`Indicator Forward Projection Clamping & Typed Metadata Contract`)
- **Scope**: Clamp Replay indicator series server-side to `timestamp <= max_visible_ts` (`candles[current_index].timestamp`) in `indicator_engine.py` and `api/replay.py` so forward-projected series (`Ichimoku Senkou Span A/B +26`) never leak future timestamps in Replay (`GAP-REPLAY-02`); preserve intentional forward projection outside Replay; finalize typed `MappingResult` metadata (`OK | EMPTY_WARMING | UNMAPPED_COLUMN | INVALID_SPEC`, `warmup_bars`, `available_bars`) with backward-compatible frontend contract types.
- **Implementation Status**: `NOT_STARTED` (Sequential after `B10`).
- **Test Evidence**: Pending (`backend/tests/test_b11_indicator_projection_lock.py`).
- **Internal QA Result**: `PENDING`
- **Open Issues**: None.
- **Dependencies**: `B10` internal acceptance.
- **Next Action**: Execute automatically after `B10` passes internal QA.
