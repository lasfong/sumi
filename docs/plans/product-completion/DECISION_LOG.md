# SUMI V3 — DEVELOPMENT TRAIN 01 DECISION LOG

Concise record of architectural, governance, and commit-gate decisions for Development Train 01 (`W0-A Closure → B03 → B10 → B11`).

---

## Ratified & Approved Governance Decisions

### `DEC-01` — W0-A Baseline Acceptance, ADR Addenda Ratification & Git Commit Authorization (`APPROVED`)
- **Date**: 2026-10-14
- **Phase**: Phase A (`B00`, `B01`, `B02`, `DB-SAFETY`)
- **Status**: **`APPROVED / RATIFIED`** (Authorized by Product Owner, 2026-10-14)
- **Context**:
  - `W0-A` implementation and safeguard files (19 tracked modified files + 10 untracked test/safeguard files) are complete and verified (`140/140` focused W0-A tests passed in `artifacts/local/development-train-01/phase_a_w0a_verification.log`; `537/538` full backend pytest and `239/239` frontend Vitest passed in preserved logs; `0` bytes delta on `backend/sumi.db`).
  - Three ADR Addenda (`ADR-PROV-001-ADDENDUM-01`, `ADR-TX-001-ADDENDUM-01`, `ADR-TX-001-ADDENDUM-02`) and proposal index ratified in `docs/architecture/`.
- **Approved Actions**:
  1. **Ratification of the 3 `W0-A` ADR Addenda**:
     - `docs/architecture/decisions/ADR-PROV-001-ADDENDUM-01-quarantine-promotion.md` (Option A in-place quarantine promotion + fail-closed `HTTP 400` pre-validation on multi-candidate same-date collisions) — **RATIFIED**
     - `docs/architecture/decisions/ADR-TX-001-ADDENDUM-01-mutation-provenance.md` (`UPDATE:{prior_source}|{prior_timestamp_iso}` encoding + `rsplit("|", 1)` + fail-closed `HTTP 400` rollback validation) — **RATIFIED**
     - `docs/architecture/decisions/ADR-TX-001-ADDENDUM-02-cross-workflow-rollback.md` (Symmetrical `ImportRun` ↔ `SyncRun` rollback conflict checks) — **RATIFIED**
  2. **Two-Commit Baseline Authorization (Option 1A)**:
     - Commit 1: 38 explicit files (`W0-A` implementation, test, safeguard, retrospective, and ADR addenda).
     - Commit 2: 46 explicit files (architecture baseline, test strategy, batch prompts, and execution governance records).
  3. **Strict Exclusions Enforced**:
     - `backend/sumi.db`, `test.db`, `artifacts/local/`, `.DS_Store`, `.venv/`, `node_modules/`, caches excluded from staging.
- **Outcome**: Phase A closed; baseline established cleanly; Phase B (`B03`) unblocked.

---

## Recorded Technical & Operational Decisions (In-Scope)

### `DEC-TRAIN01-01` — Sequential Single-Writer Order (`B03 → B10 → B11`)
- **Date**: 2026-10-14
- **Status**: `ACTIVE`
- **Decision**: Enforce strictly sequential code writes across `B03` (first) → `B10` (second) → `B11` (third). Read-only code reviews, migration-chain inspection, and test design may run concurrently, but no two agents may edit the same source file simultaneously.
- **Rationale**: Prevents working-tree entanglement and ensures each batch passes its isolated verification gate before the next batch starts.

### `DEC-TRAIN01-02` — Non-Destructive Additive Migration Strategy for `B03`
- **Date**: 2026-10-14
- **Status**: `ACTIVE`
- **Decision**: Preserve the 9 existing Alembic migration files (`cdf80254e9dc` through `a2d639ecd5d5`) unless an idempotency guard (`_has_column` / `_has_index`) is strictly necessary for clean empty-DB vs `create_all()` compatibility, and add an additive head migration for `GAP-DB-06` composite indexes and 20-model parity. Never run migrations against `backend/sumi.db` during automated verification.
