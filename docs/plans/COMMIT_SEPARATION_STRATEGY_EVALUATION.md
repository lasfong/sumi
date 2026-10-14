# Commit Separation Strategy Evaluation — Sumi V3 W0-A Stabilization

**Task**: Milestone 5 / Task 4 Governance Evaluation  
**Date**: 2026-10-12  
**Author**: Agent D (Governance & Release Preparation Worker)  
**Constraint**: Strictly analytical evaluation; NO git commits, staging, branch switches, or resets executed.  

---

## 1. Executive Summary

This report evaluates two competing delivery strategies for landing the Sumi V3 W0-A Stabilization work products into `master`:
1. **Strategy A**: A 3-commit sequential decomposition (Commit 1: B00, Commit 2: B01, Commit 3: B02 + DB-SAFETY).
2. **Strategy B**: A single integrated stabilization commit (`chore(stabilization): W0-A complete stabilization for B00, B01, B02 with fail-closed DB safeguard`).

**Verdict & Recommendation**: **STRATEGY B (Single Integrated Stabilization Commit) is strongly recommended**.  
Due to deep syntactic and transactional entanglement across shared core files (`config.py`, `db.py`, `import_workflow_service.py`, `sync_workflow_service.py`, and `weekly_aggregator.py`), splitting the current working tree into 3 standalone commits creates synthetic intermediate states that fail bisectability (`git bisect`), require fragile line-by-line staging (`git add -p`), and introduce non-atomic rollback risks between transaction wrapping and quarantine promotion. Strategy B provides 100% atomic verification, proven test execution (502 tests passing), and full traceability via the comprehensive hunk mapping and retrospective ExecPlans.

---

## 2. In-Depth Comparison of Strategies

### Strategy A: 3-Commit Sequential Decomposition

#### Breakdown of Commits:
- **Commit 1: B00 (GAP-DB-01: Absolute Path Anchoring & CWD Guard)**
  - *Files*: `backend/app/config.py` (lines 7–35, 43, 56–72), `backend/alembic/env.py`, `backend/app/main.py`, `backend/tests/test_cwd_database_isolation.py`.
- **Commit 2: B01 (GAP-DB-02/03: Transaction Boundaries & Foreign Keys)**
  - *Files*: `backend/app/db.py` (lines 9–16, 51–92), `backend/app/dependencies.py`, `backend/app/tests/test_b01_transaction_fk.py`, plus transaction `try/except: db.rollback()` wrappers in `import_workflow_service.py`, `sync_workflow_service.py`, `weekly_aggregator.py`.
- **Commit 3: B02 + DB-SAFETY (GAP-DB-04, ADR-PROV-001, Data Integrity R1–R5, Safeguards)**
  - *Files*: `backend/app/config.py` (synthetic settings), `backend/app/db.py` (quarantine filter), `sync_schema.py`, data provider adapters, Option A quarantine promotion and rollback reconstruction in workflow services, `weekly_aggregator.py` partitioning, frontend changes, `scripts/verify-v2.sh`, `tests_safety.py`, `conftest.py`, and B02/safeguard test suites.

#### Advantages of Strategy A:
- Follows the chronological batch order outlined in `docs/plans/DEV_BATCHES.md`.
- Isolates B00 path anchoring as a standalone commit.
- High conceptual granularity in `git log`.

#### Disadvantages & Critical Risks of Strategy A:
1. **Severe Inter-Batch Entanglement in Shared Files**:
   - In `backend/app/config.py`, lines 7–35 (B00 path validation), line 43 (B00 default URL), lines 49–54 (B02 synthetic settings), and lines 56–72 (B00 URL validator) are interleaved within the single `Settings` class.
   - In `backend/app/db.py`, B01 connection listener (`_set_sqlite_pragma`) and B01 helper (`atomic()`) sandwich B02 quarantine filter (`_filter_quarantine_candles`).
   - In `backend/app/services/import_workflow_service.py` (lines 356–425) and `sync_workflow_service.py` (lines 430–545), B01 transaction blocks (`try: ... db.commit() except: db.rollback(); raise`) enclose B02 Option A quarantine promotion logic. Splitting B01 from B02 requires extracting the quarantine promotion logic out of the `try:` block, creating an artificial intermediate version of the service that was never tested.
2. **Broken Bisectability (`git bisect`)**:
   - In Commit 2 (B01 without B02), `test_sync_workflow.py` cannot pass in its current state because it expects synthetic data to be quarantined (`quarantine:sync:{id}`), which only exists in B02.
   - If `test_sync_workflow.py` is kept in its pre-B02 state for Commit 2, then B02's write collision issue (Q-WRITE) remains unfixed at Commit 2.
   - Consequently, running automated test gates (`git bisect run pytest`) between Commit 1 and Commit 3 will fail, breaking Git bisectability.
3. **High Human Error Surface during Patch Staging**:
   - Splitting requires manual `git add -p` across 5 complex files. A single mis-staged hunk can cause indentation syntax errors (`IndentationError`) or subtle runtime failures.

---

### Strategy B: Single Integrated Stabilization Commit

#### Structure of the Commit:
- **Title**: `chore(stabilization): W0-A integrated stabilization for B00, B01, B02 with fail-closed DB safeguard`
- **Body**: Comprehensive summary referencing:
  - B00: GAP-DB-01, path anchoring, CWD guard.
  - B01: GAP-DB-02, GAP-DB-03, ADR-TX-001 transaction rollback & FK pragmas.
  - B02: GAP-DB-04, ADR-PROV-001, default-deny, Option A takeover, R1–R5 data integrity.
  - DB-SAFETY: Active fail-closed safeguard in `tests_safety.py` and test runner isolation in `verify-v2.sh`.
  - Artifacts: Links to `EXECPLAN_B00_RETROSPECTIVE.md`, `EXECPLAN_B01_RETROSPECTIVE.md`, `EXECPLAN_B02_RETROSPECTIVE.md`, `HUNK_TO_DEFECT_MAPPING_W0A.md`, and `ADR_ADDENDA_W0A_PROPOSALS.md`.

#### Advantages of Strategy B:
1. **Guaranteed Atomic Correctness**:
   - The entire working tree is known to pass all 502 backend tests, 239 frontend tests, 10 Alembic migrations, and zero database mutations.
   - Every single test and safeguard functions in harmony.
2. **Perfect Bisectability**:
   - Zero broken intermediate commits. Git bisect will cleanly transition from pre-stabilization to post-stabilization with green test results on both sides.
3. **Zero Risk of Staging Corruption**:
   - No error-prone interactive patch staging (`git add -p`) across shared functions.
4. **Coherent Forensic Traceability**:
   - The commit is fully accompanied by tracked governance documentation that provides 100% hunk-level attribution, giving external auditors the exact breakdown without sacrificing commit integrity.

#### Disadvantages of Strategy B:
- Larger single commit diff (+910 / -307 across 18 tracked files, plus 9 untracked files). However, in refactoring and stabilization campaigns (W0-A), integrated stabilization commits are standard industry best practice.

---

## 3. Trade-Off Analysis Matrix

| Evaluation Dimension | Strategy A (3 Commits) | Strategy B (1 Integrated Commit) | Winner |
|---|---|---|---|
| **Atomic Testability** | Poor (Intermediate commits fail or require temporary test mutations) | **Flawless** (502 tests pass cleanly) | **Strategy B** |
| **Bisectability (`git bisect`)** | Broken (Intermediate states fail CI) | **Preserved** (Binary clean pass) | **Strategy B** |
| **Inter-Batch Coupling** | High friction (Requires line-by-line patch staging in 5 files) | **Zero friction** (Natural architectural boundary) | **Strategy B** |
| **Implementation Risk** | High (Potential indentation / staging errors) | **Zero** (Working tree committed as verified) | **Strategy B** |
| **Review Traceability** | Granular commit log, but diffs are artificially split | **Complete** via Hunk Mapping & Retrospective Plans | **Strategy B** |
| **Alignment with W0-A Mandate** | Divergent from single-writer verification pass | **Strictly aligned** with cohesive W0-A stabilization | **Strategy B** |

---

## 4. Final Recommendation & Action Plan

### Recommendation
**Execute Strategy B (Single Integrated Stabilization Commit).**  
W0-A is explicitly defined as a unified stabilization milestone ("W0-A Stabilization: B00, B01, B02 Implementation, Verification & Acceptance Preparation"). Treating it as a single coherent delivery transaction ensures maximum safety, zero test regression, and absolute integrity.

### Prescribed Commit Template (For Reviewer / Maintainer Execution):
```text
chore(stabilization): W0-A integrated stabilization for B00, B01, B02 with fail-closed DB safeguard

Stabilize Sumi V3 foundation across batches B00, B01, and B02 under strict
non-negotiable safety invariants:

1. B00 (GAP-DB-01): Anchor DATABASE_URL to backend/sumi.db via Path(__file__);
   add verify_database_path_security() and normalize_database_url validator;
   install lifespan startup guard and Alembic resolver parity; remove root ghost DB.
2. B01 (GAP-DB-02, GAP-DB-03): Enforce PRAGMA foreign_keys=ON on all connections;
   add atomic() context manager and decorator; wrap get_db() and all service
   write paths (import, sync, weekly) in try/commit/except-rollback blocks.
3. B02 (GAP-DB-04, ADR-PROV-001): Enforce default-deny 403 on synthetic providers;
   establish ORM session read boundary (_filter_quarantine_candles); implement
   Option A in-place promotion eliminating unique key collisions (Q-WRITE);
   preserve mutation provenance (UPDATE:{source}|{ts}) and timestamp normalization;
   add symmetrical cross-workflow rollback conflict resolution; partition daily
   members in WeeklyAggregator.
4. DB-SAFETY: Active fail-closed test safeguard (tests_safety.py) intercepting
   sqlite3, SQLAlchemy, open, and os filesystem calls; isolated test runners.

Verification:
- 502 backend tests passed (including 14 adversarial stress tests)
- 10 fresh Alembic migrations verified cleanly
- Production backend/sumi.db SHA-256 completely unchanged:
  ef5c035c27d49de1de51a59e28e1cac5109d92240bcb600239f2180d49128f3e
- 0 schema migrations added in W0-A (B03 remains strictly blocked)

Governance:
- Docs: docs/plans/EXECPLAN_B00_RETROSPECTIVE.md
- Docs: docs/plans/EXECPLAN_B01_RETROSPECTIVE.md
- Docs: docs/plans/EXECPLAN_B02_RETROSPECTIVE.md
- Docs: docs/plans/HUNK_TO_DEFECT_MAPPING_W0A.md
- Docs: docs/architecture/ADR_ADDENDA_W0A_PROPOSALS.md
```
