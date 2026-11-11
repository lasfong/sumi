# Formal ADR Addenda Proposals — Sumi V3 W0-A Stabilization

**Document Type:** Formal Architectural Addenda Package for External Review  
**Target Reviewer:** Technical Architect (GPT-6) / Lead Orchestrator  
**Date:** 2026-10-12  
**Baseline ADRs:** `ADR-PROV-001` (Synthetic Policy), `ADR-TX-001` (Transaction Boundaries)  
**Authors:** Sumi V3 Stabilization Engineering Team (Agent B, Agent C, Agent D)  

---

## Executive Summary

During the W0-A Stabilization campaign (covering batches B00, B01, and B02), three subtle architectural challenges were uncovered and remediated without introducing schema migrations or mutating production databases:
1. **Unique Key Collision during Quarantine Takeover**: Resolving the database constraint conflict when canonical real data supersedes quarantine placeholders.
2. **Mutation Provenance Loss in Rollback**: Ensuring that rolled-back promoted rows fully recover their pre-operation quarantine state and source tag.
3. **Cross-Workflow Rollback Incoherence**: Preventing an Import rollback from corrupting subsequent Sync runs and vice versa.

In accordance with Invariant 6 ("Ratified ADRs are immutable sources of truth; proposed policy extensions must be drafted as formal addenda proposals for external review"), the original text of `ADR-PROV-001` and `ADR-TX-001` has been preserved without alteration. The three addenda below document the proposed rationale, mechanics, and verification oracles.

---

## Index of Proposed Addenda

1. [ADR-PROV-001-ADDENDUM-01: Option A Quarantine Promotion & Precedence](decisions/ADR-PROV-001-ADDENDUM-01-quarantine-promotion.md)
2. [ADR-TX-001-ADDENDUM-01: Mutation Provenance Encoding & State Reconstruction](decisions/ADR-TX-001-ADDENDUM-01-mutation-provenance.md)
3. [ADR-TX-001-ADDENDUM-02: Cross-Workflow Symmetrical Rollback Conflict Resolution](decisions/ADR-TX-001-ADDENDUM-02-cross-workflow-rollback.md)

---

## Addendum 1: ADR-PROV-001-ADDENDUM-01
### Option A Quarantine Promotion & Pre-Validation Rejection

- **Reference:** `docs/architecture/decisions/ADR-PROV-001-ADDENDUM-01-quarantine-promotion.md`
- **Status:** Ratified
- **Core Policy**:
  - **Absolute Precedence**: Canonical real market data always supersedes synthetic quarantine data. Quarantine data never permanently blocks a canonical candle key.
  - **Multi-Candidate Quarantine Grouping**: Incoming canonical data queries existing quarantine candles and groups them by date key `(symbol, timeframe, date, adjustment_type)` using `quarantine_groups = defaultdict(list)`.
  - **Ambiguous Candidate Pre-Validation Rejection (R2)**: Before any database writes or mutation logs occur, an atomic pre-validation pass checks all incoming items. If multiple quarantine candidates share the same date key (`len(candidates) > 1`), the transaction immediately calls `db.rollback()` and raises `HTTPException(400)`, preventing silent candidate picking or `IntegrityError: UNIQUE constraint failed` collisions.
  - **Option A In-Place Promotion**: When exactly one quarantine candle matches (`len(candidates) == 1`), in-place promotion updates OHLCV values and normalizes timestamps to midnight while capturing pre-promotion provenance `UPDATE:{prior_source}|{prior_iso_ts}`.
  - **Synthetic Precedence Protection**: Synthetic sync can NEVER overwrite or downgrade canonical data. Differing OHLCV triggers a conflict block (`can_accept=False`); identical OHLCV triggers an idempotent no-op (`status="noop"`).
- **Verification Evidence**:
  - `test_b02_quarantine_write_collision.py`: `test_q_write_01`, `test_q_write_02`, `test_q_write_03`, `test_q_write_27`.
  - `test_w0a_final_adversarial.py`: `test_08`, `test_09`, `test_10`, `test_11`, `test_14`.

---

## Addendum 2: ADR-TX-001-ADDENDUM-01
### Mutation Provenance Encoding & Fail-Closed State Restoration

- **Reference:** `docs/architecture/decisions/ADR-TX-001-ADDENDUM-01-mutation-provenance.md`
- **Status:** Ratified
- **Core Policy**:
  - **Provenance Format**: When a quarantine candle is promoted, the mutation log records `action = f"UPDATE:{quar_candle.source}|{quar_ts_str}"` along with `before_*` numeric values.
  - **Right-Split Parsing**: In rollback, `parts.rsplit("|", 1)` splits the action string from the right, preserving source identifiers containing delimiter pipes.
  - **Fail-Closed Validation (R1)**: Strict parsing prohibits silent exception swallowing (`except Exception: pass`). Empty provenance timestamps and malformed ISO strings immediately raise `HTTPException(400)`.
  - **Target Validation (R1)**: Missing rollback target candles raise `HTTPException(400)`. In date fallback matching (`1D`/`D`), multiple candles matching the same calendar date trigger an ambiguous rollback target error (`HTTPException(400)`).
  - **Transactional Atomicity**: Any error during rollback executes `db.rollback()`, ensuring zero partial state mutation and keeping `run.status` as `"accepted"`.
  - **Backward Compatibility**: Supports legacy `UPDATE` and `UPDATE:source` actions cleanly.
- **Verification Evidence**:
  - `test_b02_quarantine_write_collision.py`: `test_q_write_12`, `test_q_write_13`, `test_q_write_18`, `test_q_write_19`.
  - `test_adversarial_challenger.py`: `test_adv_time_01`, `test_adv_time_02`.
  - `test_w0a_final_adversarial.py`: `test_01`, `test_02`, `test_03`, `test_04`, `test_05`, `test_06`, `test_07`.

---

## Addendum 3: ADR-TX-001-ADDENDUM-02
### Cross-Workflow Symmetrical Rollback Conflict Resolution

- **Reference:** `docs/architecture/decisions/ADR-TX-001-ADDENDUM-02-cross-workflow-rollback.md`
- **Status:** Ratified
- **Core Policy**:
  - **Symmetrical Dependency Checks**:
    - `ImportWorkflowService.rollback_import`: Queries for subsequent accepted `SyncRun`s touching affected symbols. If detected, aborts with HTTP 400.
    - `SyncWorkflowService.rollback_sync`: Queries for subsequent accepted `ImportRun`s touching the symbol. If detected, aborts with HTTP 400.
  - **Status Filtering**: Only runs with `status == "accepted"` block rollback; failed, rejected, or rolled-back runs do not prevent rollback.
  - **All-or-Nothing Atomicity**: Any conflict or error executes `db.rollback()`, ensuring zero partial state mutation.
- **Verification Evidence**:
  - `test_b02_quarantine_write_collision.py`: `test_q_write_17`, `test_q_write_20`.
  - `test_adversarial_challenger.py`: `test_adv_roll_01`, `test_adv_roll_02`, `test_adv_roll_03`.

---

## Summary Matrix

| Addendum | Parent ADR | Defect Addressed | Core Solution | Migration Impact | Verification Suite |
|---|---|---|---|---|---|
| **ADDENDUM-01** | ADR-PROV-001 | Quarantine Unique Key Collision & Ambiguity (R2) | Multi-candidate grouping + pre-validation rejection (HTTP 400); Option A in-place promotion | 0 migrations (retains single `Candle` table) | `test_b02_quarantine_write_collision.py` (31 tests), `test_w0a_final_adversarial.py` (tests 8-14) |
| **ADDENDUM-01** | ADR-TX-001 | Mutation Provenance Loss & Fail-Closed Rollback (R1) | `UPDATE:{source}\|{ts}` encoding + fail-closed validation (empty/malformed/ambiguous HTTP 400) | 0 migrations (encodes in existing `action` column) | `test_q_write_18`, `test_adv_time_01`, `test_w0a_final_adversarial.py` (tests 1-7) |
| **ADDENDUM-02** | ADR-TX-001 | Asymmetrical Rollback Conflict across Import/Sync | Symmetrical cross-workflow dependency checks (HTTP 400) | 0 migrations (queries existing run tables) | `test_q_write_17`, `test_q_write_20`, `test_adv_roll_01` |
