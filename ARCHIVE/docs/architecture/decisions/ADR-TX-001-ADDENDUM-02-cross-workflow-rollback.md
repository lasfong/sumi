# ADR-TX-001-ADDENDUM-02 — Cross-Workflow Symmetrical Rollback Conflict Resolution

**Status:** Ratified  
**Date:** 2026-10-12  
**Target Reference:** `docs/architecture/decisions/ADR-TX-001-transaction-boundary.md`  
**Authors:** Sumi V3 Stabilization Team (Agent B & Agent D)  

---

## 1. Context & Architectural Problem

Sumi supports two distinct data ingestion entry points:
1. **Import Workflow (`ImportWorkflowService`)**: User uploads historical CSV files (e.g. CafeF).
2. **Sync Workflow (`SyncWorkflowService`)**: User synchronizes data via provider adapters.

In a local-first application, workflows can be interleaved across identical symbols. For example:
- Step 1: User imports 2026-03 daily candles for symbol `HPG` via Import Workflow at $T_1$.
- Step 2: User synchronizes 2026-03 daily candles for symbol `HPG` via Sync Workflow at $T_2 > T_1$.

Previously, `ImportWorkflowService.rollback_import` only checked whether a subsequent *import* run existed (`ImportRun.accepted_at > run.accepted_at`), and `SyncWorkflowService.rollback_sync` only checked whether a subsequent *sync* run existed. Neither service checked for operations executed by the *other* workflow!

If a user rolled back Step 1 (Import), the import rollback would delete or mutate candles that were actively depended upon or updated by Step 2 (Sync), corrupting the dataset and producing orphaned weekly candles.

---

## 2. Decision: Symmetrical Cross-Workflow Dependency Resolution

1. **Import-then-Sync Resolution**:
   In `ImportWorkflowService.rollback_import`, before initiating candle deletion or reversal:
   ```python
   subsequent_syncs = (
       db.query(SyncRun)
       .filter(
           SyncRun.status == "accepted",
           SyncRun.symbol.in_(list(affected_symbols)),
           SyncRun.accepted_at > run.accepted_at
       )
       .all()
   )
   if subsequent_syncs:
       overlap = affected_symbols.intersection(set(s.symbol for s in subsequent_syncs))
       raise HTTPException(
           status_code=400,
           detail=f"Hoàn tác bị từ chối: Đã có lượt đồng bộ mới hơn ({subsequent_syncs[0].id}) thay đổi các mã {', '.join(overlap)}"
       )
   ```
2. **Sync-then-Import Resolution**:
   In `SyncWorkflowService.rollback_sync`, before initiating candle deletion or reversal:
   ```python
   subsequent_imports = (
       db.query(ImportRun)
       .join(ImportRunMutation, ImportRunMutation.run_id == ImportRun.id)
       .filter(
           ImportRun.status == "accepted",
           ImportRunMutation.symbol == run.symbol,
           ImportRun.accepted_at > run.accepted_at
       )
       .all()
   )
   if subsequent_imports:
       raise HTTPException(
           status_code=400,
           detail=f"Hoàn tác bị từ chối: Đã có lượt nhập tập tin mới hơn ({subsequent_imports[0].file_name}) thay đổi mã {run.symbol}"
       )
   ```
3. **Filter on Accepted Status**:
   Only runs with `status == "accepted"` block rollback. Runs that were `failed`, `rejected`, or already `rolled_back` do NOT block rollback.
4. **All-or-Nothing Atomicity**:
   If an exception or conflict occurs during rollback evaluation, `db.rollback()` executes immediately. The operation is strictly fail-closed, leaving zero partial state.

---

## 3. Alternatives Evaluated

| Alternative | Mechanics | Evaluation & Disposition |
|---|---|---|
| **Fail-Closed Block (Adopted)** | Detect cross-workflow subsequent accepted runs and return HTTP 400 with descriptive error. | **Accepted**: Safe, deterministic, prevents silent data corruption, informs user of exact dependency. |
| **Cascading Rollback** | Automatically roll back the newer run before rolling back the requested run. | **Rejected**: Dangerous and unexpected for users; rolling back a sync could discard significant downloaded data without explicit confirmation. |
| **Blind Unchecked Rollback** | Allow rollback to proceed regardless of subsequent runs in the other workflow. | **Rejected**: Produces corrupted states, phantom deletions, and incorrect weekly bars. |

---

## 4. Consequences & Invariants

- **Positive**: Complete cross-workflow dataset consistency and integrity.
- **Positive**: Explanatory error messages clarify which newer operation prevents rollback.
- **Invariant**: Any rollback operation must verify both internal and external workflow dependencies prior to executing mutations.

---

## 5. Verification & Test Oracles

- `backend/app/tests/test_b02_quarantine_write_collision.py`:
  - `test_q_write_17_cross_workflow_conflict_sync_blocks_import_rollback`
  - `test_q_write_20_cross_workflow_conflict_import_blocks_sync_rollback`
- `backend/tests/test_adversarial_challenger.py`:
  - `test_adv_roll_01_cross_workflow_rollback_permits_rollback_when_subsequent_run_failed`
  - `test_adv_roll_02_fault_injection_during_import_rollback_preserves_atomic_state`
  - `test_adv_roll_03_multi_cycle_takeover_and_rollback_preserves_quarantine_integrity`
