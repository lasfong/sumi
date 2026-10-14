# ADR-TX-001-ADDENDUM-01 — Mutation Provenance Encoding & State Reconstruction

**Status:** Ratified  
**Date:** 2026-10-12  
**Target Reference:** `docs/architecture/decisions/ADR-TX-001-transaction-boundary.md`  
**Authors:** Sumi V3 Stabilization Team (Agent B & Agent D)  

---

## 1. Context & Architectural Problem

ADR-TX-001 established transactional boundaries and rollback requirements for all write operations. Sumi tracks audit history and rollback state using `ImportRunMutation` and `SyncRunMutation` tables.

In the legacy design:
- Mutations recorded only `action="INSERT"` or `action="UPDATE"`.
- When an `UPDATE` was executed to promote a quarantine row to canonical data, the mutation log recorded only the `before_*` numeric OHLCV fields.
- Upon rollback, the system updated the numbers back to `before_*`, but left `candle.source` set to the canonical `run_id` (or `sync:{id}`). Consequently, the quarantine candle lost its `quarantine:` prefix and became an illegitimate canonical candle!
- Furthermore, differences in ISO timestamp formatting (e.g. midnight timestamps vs microsecond strings) could cause candle lookups to fail during rollback.

---

## 2. Decision: Structured Provenance Action Encoding & Fail-Closed Restoration

1. **Structured Action Format**:
   When promoting an existing quarantine candle, the mutation `action` string is encoded as:
   ```text
   UPDATE:{prior_source}|{prior_timestamp_iso}
   ```
   *Example*: `UPDATE:quarantine:sync:sub_123|2026-03-02T00:00:00`
2. **Right-Split Parsing (`rsplit("|", 1)`)**:
   During rollback, the action string is parsed using `parts.rsplit("|", 1)`:
   - Splitting from the right guarantees that source strings containing internal pipes (e.g., `quarantine:provider|subfeed|uuid`) are preserved without corruption.
   - Extracts `src` and `prior_ts_str`.
3. **Fail-Closed Timestamp Validation (Finding R1)**:
   Silent exception swallowing (`except Exception: pass`) is strictly prohibited.
   - **Empty Timestamp**: If `prior_ts_str.strip()` is empty, rollback aborts immediately with `HTTPException(status_code=400, detail="Hoàn tác bị từ chối: Dữ liệu nguồn gốc thời gian bị trống cho mã {symbol} (Empty provenance timestamp)")`.
   - **Malformed Timestamp**: If `prior_ts_str` cannot be parsed by `datetime.fromisoformat()`, rollback aborts immediately with `HTTPException(status_code=400, detail="Hoàn tác bị từ chối: Dữ liệu nguồn gốc thời gian không hợp lệ '{prior_ts_str}' cho mã {symbol} (Malformed provenance timestamp)")`.
4. **Target Existence & Ambiguity Validation (Finding R1)**:
   - **Missing Target**: If no candle is found for the mutation record, rollback aborts with `HTTPException(status_code=400, detail="Hoàn tác bị từ chối: Không tìm thấy cây nến cần khôi phục cho mã {symbol} lúc {timestamp} (Missing rollback target)")`.
   - **Ambiguous Date Fallback**: If exact timestamp matching fails on daily timeframes (`1D`/`D`), candidate matching falls back to calendar date (`c.timestamp.date() == m.timestamp.date()`). If multiple candles match on that calendar date (`len(matching) > 1`), rollback aborts with `HTTPException(status_code=400, detail="Hoàn tác bị từ chối: Tìm thấy nhiều cây nến trùng ngày ({count}) cho mã {symbol} ngày {date} (Ambiguous rollback target)")`.
5. **State Restoration**:
   For valid mutations, rollback restores:
   - `candle.open = m.before_open`, ..., `candle.volume = m.before_volume`
   - `candle.source = src` (restores original quarantine provenance)
   - `candle.timestamp = prior_ts` (restores exact timestamp)
6. **Transactional Atomicity**:
   The entire rollback operation is wrapped in a `try ... except Exception: db.rollback(); raise` block. Any validation failure or runtime exception executes `db.rollback()`, ensuring complete zero-mutation reversion where `run.status` remains `"accepted"`.
7. **Backward Compatibility**:
   - `action == "UPDATE"`: Restores numeric OHLCV without altering source or timestamp.
   - `action == "UPDATE:source"` (without pipe): Restores source without altering timestamp.

---

## 3. Alternatives Evaluated

| Alternative | Mechanics | Evaluation & Disposition |
|---|---|---|
| **Action String Encoding + Fail-Closed Validation (Adopted)** | Encode metadata in existing `action` varchar column (`UPDATE:{source}\|{ts}`) with strict fail-closed validation and atomic transaction rollback. | **Accepted**: Zero database schema migration; completely backwards-compatible with existing schema and tables; transparent to ORM; robust against corrupted provenance. |
| **New Columns on Mutation Table** | Add `prior_source` and `prior_timestamp` columns to `import_run_mutations` and `sync_run_mutations`. | **Rejected**: Requires Alembic schema migrations across multiple tables, violating W0-A stabilization constraints where B03 is strictly blocked. |
| **JSON Payload in Action Column** | Store JSON string `{"source": "...", "ts": "..."}` in action. | **Rejected**: Action column length limits in existing schemas; unnecessary serialization overhead. |

---

## 4. Consequences & Invariants

- **Positive**: Complete, lossless round-trip reconstruction of quarantine state under rollback.
- **Positive**: Resilient against pipe characters in source identifiers and timestamp microsecond precision.
- **Positive**: Malformed or ambiguous provenance fails closed with HTTP 400 and full transaction rollback, preventing partial state corruption.
- **Invariant**: Any write operation promoting a quarantine row must record the prior source and ISO timestamp in the mutation log before modifying the candle.

---

## 5. Verification & Test Oracles

- `backend/app/tests/test_b02_quarantine_write_collision.py`:
  - `test_q_write_04_import_rollback_restores_quarantine`
  - `test_q_write_05_sync_rollback_restores_quarantine`
  - `test_q_write_12_import_rollback_restores_quarantine_with_normalized_midnight_timestamp`
  - `test_q_write_13_sync_rollback_restores_quarantine_with_normalized_midnight_timestamp`
  - `test_q_write_18_pipe_heavy_source_name_provenance_roundtrip`
  - `test_q_write_19_sync_pipe_heavy_source_name_provenance_roundtrip`
- `backend/tests/test_adversarial_challenger.py`:
  - `test_adv_time_01_pipe_heavy_source_preserves_provenance`
  - `test_adv_time_02_legacy_mutation_format_compatibility`
- `backend/tests/test_w0a_final_adversarial.py`:
  - `test_01_malformed_provenance_timestamp_fails_closed`
  - `test_02_missing_restoration_target_candle_rolls_back`
  - `test_03_ambiguous_restoration_target_candle_on_date_fallback`
  - `test_04_legacy_update_mutation_compatibility_without_colons_or_pipes`
  - `test_05_source_containing_pipes_and_colons_survives_or_safely_rejected`
  - `test_06_fault_injection_during_rollback_rolls_back_entire_db_transaction`
  - `test_07_successful_rollback_after_prior_failed_attempt`
