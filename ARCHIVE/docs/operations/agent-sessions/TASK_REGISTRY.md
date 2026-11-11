# SUMI V3 — REPOSITORY TASK REGISTRY

- **Document ID**: `SUMI-V3-TASK-REGISTRY-V6`
- **Last Updated**: `2026-10-20` (Session `SUMI-REL001-BASELINE-CLOSE-001`)
- **Repository Root**: `/Users/mizuhara/workspace/sumi`

---

## 1. Canonical Task Status & Authorization Semantics

Every entry in this registry separates **Lifecycle Status**, **Authorization State**, **Execution State**, and **Commit Permission State** so that future sessions never confuse an unexecuted task or unexecuted Owner UAT with completed work.

### 1.1 Allowed Lifecycle Status Values (`7` Canonical States)

| Status Value | Exact Semantic Meaning | Execution State | When to Use |
| :--- | :--- | :--- | :--- |
| **`PROPOSED`** | Candidate task scoped for a future session; **not yet authorized** by the Product Owner. | `NOT_EXECUTED` | Recommended next tasks in `NEXT_TASK.md` before Owner dispatch. |
| **`AUTHORIZED`** | Explicitly approved by the Product Owner for execution in a bounded session, ready to start. | `NOT_STARTED` | Tasks explicitly dispatched by the Owner prior to execution start. |
| **`IN_PROGRESS`** | Currently being executed by the active Antigravity session. | `RUNNING` | Active task during Steps 1–6 of `SESSION_PROTOCOL.md`. |
| **`BLOCKED`** | Cannot proceed until an unexecuted prerequisite (such as manual Product Owner UAT walkthrough), escalation resolution, or policy decision completes. | `NOT_EXECUTED` or `HALTED` | Tasks waiting on manual Owner UAT execution (`OWNER-UAT-SIGNOFF-001`) or blocked by an escalation trigger. |
| **`AWAITING_OWNER_REVIEW`** | An agent session **has completed and verified all of its in-scope deliverables** on disk, and those completed deliverables now await Product Owner review or commit disposition. | `EXECUTED_AND_VERIFIED` | Completed un-committed agent sessions (e.g., `SUMI-RELEASE-GATE-001`). **Never use for unexecuted Owner UAT.** |
| **`COMPLETED`** | Bounded task work is finished, verified, and closed (committed on `master`, or for a bootstrap session task, succeeded/hardened by a follow-up session). | `EXECUTED_AND_VERIFIED` | Closed development/QA milestones (`d2d048a` and earlier) and closed predecessor context sessions (`CTX-BOOTSTRAP-001`, `CTX-HARDEN-002`, `OWNER-UAT-PREP-001`). |
| **`DEFERRED`** | Intentionally postponed to post-V3 backlog by governance decision. | `POSTPONED` | Non-blocking `LOW` / `P2` backlog items (`BACKLOG-POST-V3-LOW`). |

### 1.2 Non-Negotiable Task Governance Rules

1. **Unexecuted Owner UAT Rule**: An unexecuted Manual Product Owner UAT (`OWNER-UAT-SIGNOFF-001`) must **never** be marked `AWAITING_OWNER_REVIEW` or `COMPLETED`. Until the Product Owner actually performs manual UAT, `OWNER-UAT-SIGNOFF-001` remains `BLOCKED` (`Execution State: NOT_EXECUTED`, `OWNER_ACCEPTED: false`).
2. **Default Commit Permission is `DENIED`**: Every task defaults to `Commit Permission: DENIED` unless the Product Owner expressly grants commit permission for that specific task.
3. **No Automatic Chaining**: Completing a task does **not** authorize the next task in this registry.
4. **No Mission Expansion**: A session must **not** expand its scope merely because a `PROPOSED` task is visible in this registry or in [`NEXT_TASK.md`](NEXT_TASK.md).

---

## 2. Master Task Register

| Task ID | Task Title | Lifecycle Status | Authorization State | Execution State | Commit Permission | Output / Handoff Path | Dependencies | Next Action |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **`HIST-P01-P10`** | Historical Quantitative Research & Signal Foundation (`P1–P10`) | `COMPLETED` | `AUTHORIZED` (`STATE.json`, `2026-09-14`) | `EXECUTED_AND_VERIFIED` (`COMMITTED_ON_MASTER`) | `HISTORICAL_COMMITTED` | [`docs/V3_FINAL_HANDOFF_REPORT_2026-09-12.md`](../../V3_FINAL_HANDOFF_REPORT_2026-09-12.md) | None | Retained as domain foundation; superseded as active completion plan by `B00–B51`. |
| **`TRAIN-01-W0A`** | Wave `W0-A` Stabilization & Architecture Baseline (`B00`, `B01`, `B02`) | `COMPLETED` | `AUTHORIZED` (`DEC-01`, `2026-10-14`) | `EXECUTED_AND_VERIFIED` (`e34516c`, `2151f34`) | `HISTORICAL_COMMITTED` | `artifacts/local/W0-A/`, `artifacts/local/development-train-01/` | `HIST-P01-P10` | Closed on `master`. |
| **`TRAIN-01-02-W0B-W2`** | Waves `W0-B`, `W1`, `W2` & Remediation `R1–R8` (`B03`, `B10`, `B11`, `B20`, `B21`, `B22`) | `COMPLETED` | `AUTHORIZED` (`DEC-02..06`, `DEC-REM-01..08`, `2026-10-16`) | `EXECUTED_AND_VERIFIED` (`c331dc3`, `0732ca0`) | `HISTORICAL_COMMITTED` | `artifacts/local/development-train-02/`, `artifacts/local/train_01_02_remediation/` | `TRAIN-01-W0A` | Closed on `master`. |
| **`TRAIN-03-W3`** | Wave `W3` Replay State, WebSocket & `ChartKernel` Integration (`B30`, `B31`, `B32`) | `COMPLETED` | `AUTHORIZED` (`DEC-06`, `DEC-07`, `2026-10-16`) | `EXECUTED_AND_VERIFIED` (`12e166a`) | `HISTORICAL_COMMITTED` | `artifacts/local/development-train-03/` | `TRAIN-01-02-W0B-W2` | Closed on `master`. |
| **`TRAIN-04-W4`** | Wave `W4` API Contracts, Money Flow BB, Trading Practice & Strategy Lab UX (`B40`, `B41`, `B50`, `B51`) | `COMPLETED` | `AUTHORIZED` (`DEC-07`, `DEC-08`, `2026-10-17`) | `EXECUTED_AND_VERIFIED` (`40f6563`) | `HISTORICAL_COMMITTED` | `artifacts/local/development-train-04/` | `TRAIN-03-W3` | Closed on `master`. |
| **`TRAIN-05-W5-UAT`** | Wave `W5` Full Browser Product UAT & Defect Remediation (`DEF-W5-01..05`) | `COMPLETED` | `AUTHORIZED` (`DEC-08`, `DEC-09`, `2026-10-17`) | `EXECUTED_AND_VERIFIED` (`bf0bbc6`; `101/101` checks) | `HISTORICAL_COMMITTED` | `artifacts/local/development-train-05/OWNER_UAT_GUIDE.md` | `TRAIN-04-W4` | Closed on `master`. |
| **`QA-REM-001`** | Independent QA Financial & Product Acceptance Remediation (`DEF-001..006, 008, 012`) | `COMPLETED` | `AUTHORIZED` (`DEC-10`, `2026-10-17`) | `EXECUTED_AND_VERIFIED` (`925a095`; `854` BE / `272` FE) | `HISTORICAL_COMMITTED` | `artifacts/local/independent-qa-final-retest/OWNER_UAT_GUIDE.md` | `TRAIN-05-W5-UAT` | Closed on `master`. |
| **`QA-BROWSER-GATE-001`** | Full-Browser Product QA Remediation & Independent Acceptance Gate (`SQA-001..005, 009..018`) | `COMPLETED` | `AUTHORIZED` (Owner Gate & Commit Prompt, `2026-10-19`) | `EXECUTED_AND_VERIFIED` (`d2d048a`; `19/19` browser checks, `950` BE / `338` FE, `44/44` golden cases) | `HISTORICAL_COMMITTED` (`d2d048a`) | [`artifacts/local/full-browser-qa-acceptance-gate/FINAL_BASELINE_HANDOFF.md`](../../../artifacts/local/full-browser-qa-acceptance-gate/FINAL_BASELINE_HANDOFF.md) | `QA-REM-001` | Code frozen at `d2d048a` for manual Owner UAT. |
| **`CTX-BOOTSTRAP-001`** | Repository-Native Agent Session Context Bootstrap | `COMPLETED` | `AUTHORIZED` (Owner Prompt `CTX-BOOTSTRAP-001`, `2026-10-19`) | `EXECUTED_AND_VERIFIED` (Succeeded & hardened in-place by `CTX-HARDEN-002`; working-tree files uncommitted) | `DENIED` | [`artifacts/local/session-ops/CTX-BOOTSTRAP-001/SESSION_HANDOFF.md`](../../../artifacts/local/session-ops/CTX-BOOTSTRAP-001/SESSION_HANDOFF.md) | `QA-BROWSER-GATE-001` | Superseded by hardened `CTX-HARDEN-002` deliverables. |
| **`CTX-HARDEN-002`** | Session Context Protocol Hardening & Fresh-Session Bootstrap Dry-Run | `COMPLETED` | `AUTHORIZED` (Owner Prompt `CTX-HARDEN-002`, `2026-10-19`) | `EXECUTED_AND_VERIFIED` (Hardened protocol validated in-place by `OWNER-UAT-PREP-001`; working-tree files uncommitted) | `DENIED` | [`artifacts/local/session-ops/CTX-HARDEN-002/SESSION_HANDOFF.md`](../../../artifacts/local/session-ops/CTX-HARDEN-002/SESSION_HANDOFF.md) | `CTX-BOOTSTRAP-001` | Validated by real execution of `OWNER-UAT-PREP-001`. |
| **`OWNER-UAT-PREP-001`** | Product Owner UAT Workspace-Local DB Setup, Runbook & Consolidated Interactive Scorecard | `COMPLETED` | `AUTHORIZED` (Owner Prompt `OWNER-UAT-PREP-001`, `2026-10-19`) | `EXECUTED_AND_VERIFIED` (`OWNER_UAT_RUNBOOK.md`, `OWNER_UAT_SCORECARD.md`, `test_db/sumi_owner_uat_isolated.db`, and startup/shutdown smoke verified; uncommitted) | `DENIED` | [`artifacts/local/session-ops/OWNER-UAT-PREP-001/SESSION_HANDOFF.md`](../../../artifacts/local/session-ops/OWNER-UAT-PREP-001/SESSION_HANDOFF.md) | `CTX-HARDEN-002` | Validated and preserved unchanged as source baseline for `SUMI-RELEASE-GATE-001` and manual Owner UAT. |
| **`SUMI-RELEASE-GATE-001`** | Final Real-World Product Release Readiness Assessment (Condition A + Condition B) | `COMPLETED` | `AUTHORIZED` (Owner Prompt `SUMI-RELEASE-GATE-001`, `2026-10-19`) | `EXECUTED_AND_VERIFIED` (`CONDITIONAL_GO` recommendation, `9.33/10`, `44` features, `5` journeys, `65` screenshots, `951` BE / `338` FE tests, `REL-001..006` documented; uncommitted) | `DENIED` | [`artifacts/local/session-ops/SUMI-RELEASE-GATE-001/SESSION_HANDOFF.md`](../../../artifacts/local/session-ops/SUMI-RELEASE-GATE-001/SESSION_HANDOFF.md) | `OWNER-UAT-PREP-001` | Succeeded by bounded `REL-001` remediation (`SUMI-REL001-REMEDIATION-001`). |
| **`SUMI-REL001-REMEDIATION-001`** | Bounded `REL-001` CafeF Raw Data Import Correctness Remediation | `COMPLETED` | `AUTHORIZED` (Owner Prompt `SUMI-REL001-REMEDIATION-001`, `2026-10-20`) | `EXECUTED_AND_VERIFIED` (`COMMITTED_ON_MASTER` @ `864e751`) | `HISTORICAL_COMMITTED` (`864e751`) | [`artifacts/local/session-ops/SUMI-REL001-REMEDIATION-001/SESSION_HANDOFF.md`](../../../artifacts/local/session-ops/SUMI-REL001-REMEDIATION-001/SESSION_HANDOFF.md) | `SUMI-RELEASE-GATE-001` | Validated by `SUMI-REL001-INDEPENDENT-RETEST-001` and closed in commit `864e751` (`SUMI-REL001-BASELINE-CLOSE-001`). |
| **`SUMI-REL001-INDEPENDENT-RETEST-001`** | Independent Data Integrity & Release Retest of `REL-001` | `COMPLETED` | `AUTHORIZED` (Owner Prompt `SUMI-REL001-INDEPENDENT-RETEST-001`, `2026-10-20`) | `EXECUTED_AND_VERIFIED` (`PASS — REL001 READY_FOR_CONTROLLED_COMMIT`: Tests A–F passed, `961 passed, 1 skipped` BE / `339 passed` FE / `6/6` Chromium browser checks; closed in `864e751`) | `DENIED` | [`artifacts/local/session-ops/SUMI-REL001-INDEPENDENT-RETEST-001/SESSION_HANDOFF.md`](../../../artifacts/local/session-ops/SUMI-REL001-INDEPENDENT-RETEST-001/SESSION_HANDOFF.md) | `SUMI-REL001-REMEDIATION-001` | Succeeded by controlled commit in `SUMI-REL001-BASELINE-CLOSE-001` (`864e751`). |
| **`SUMI-REL001-BASELINE-CLOSE-001`** | Controlled Commit & Baseline Closure of `REL-001` (`9` Files) | `AWAITING_OWNER_REVIEW` | `AUTHORIZED` (Owner Prompt `SUMI-REL001-BASELINE-CLOSE-001`, `2026-10-20`) | `EXECUTED_AND_VERIFIED` (`COMMITTED_ON_MASTER` @ `864e751ebaf979114b5bc7b5db6c12dc52b2e5ab`; `9` files; `961 passed, 1 skipped` BE / `339 passed` FE; session-context docs remain untracked/uncommitted) | `AUTHORIZED` (Strictly `9` `REL-001` files only) | [`artifacts/local/session-ops/SUMI-REL001-BASELINE-CLOSE-001/SESSION_HANDOFF.md`](../../../artifacts/local/session-ops/SUMI-REL001-BASELINE-CLOSE-001/SESSION_HANDOFF.md) | `SUMI-REL001-INDEPENDENT-RETEST-001` | Product Owner reviews closure handoff, executes manual UAT (`OWNER-UAT-SIGNOFF-001`), and decides V3 release & governance closure. |
| **`OWNER-UAT-SIGNOFF-001`** | Interactive Manual Product Owner UAT Execution & V3 Final Acceptance Decision | `BLOCKED` | `RESERVED_FOR_OWNER` (`PENDING OWNER DECISION`) | **`NOT_EXECUTED`** (`OWNER_ACCEPTED: false`, `ACCEPTED_V3_COMPLETE: false`) | `RESERVED_FOR_OWNER` | Owner UAT decision & post-UAT governance commit | `QA-BROWSER-GATE-001`, `OWNER-UAT-PREP-001`, `SUMI-RELEASE-GATE-001`, `SUMI-REL001-REMEDIATION-001`, `SUMI-REL001-INDEPENDENT-RETEST-001`, `SUMI-REL001-BASELINE-CLOSE-001` | **Blocked pending manual Owner UAT walkthrough & release decision.** No agent may execute or mark accepted. |
| **`BACKLOG-POST-V3-LOW`** | Deferred Non-Blocking `LOW` & `P2` Usability / Cosmetic Backlog (`SQA-006..008, 019`, `DEF-007, 009..011`, `REL-002..006`) | `DEFERRED` | `DEFERRED_BY_GOVERNANCE` (`DEC-10` & `FINAL_BASELINE_HANDOFF.md` §5) | `POSTPONED` | `DENIED` | Post-V3 backlog (`PROJECT_SNAPSHOT.md` §5.2 & `RELEASE_ISSUE_REGISTER.md`) | `OWNER-UAT-SIGNOFF-001` | Re-evaluate only after V3 Owner decision. |

---

## 3. Detailed 12-Attribute Task Boundary Cards (Current & Upcoming Tasks)

### 3.1 `CTX-BOOTSTRAP-001` — Repository-Native Agent Session Context Bootstrap
1. **Unique Task ID**: `CTX-BOOTSTRAP-001`
2. **Clear Mission**: Create initial repository-native session context files under `docs/operations/agent-sessions/` and handoff artifacts under `artifacts/local/session-ops/CTX-BOOTSTRAP-001/`.
3. **Authorization State**: `AUTHORIZED` (Explicit Product Owner prompt `CTX-BOOTSTRAP-001`, `2026-10-19`); **Lifecycle Status**: `COMPLETED` (succeeded and hardened in-place by `CTX-HARDEN-002`).
4. **Explicit In-Scope & Out-of-Scope Work**:
   - *In-Scope*: Create the 6 `docs/operations/agent-sessions/*.md` files and `artifacts/local/session-ops/CTX-BOOTSTRAP-001/*`.
   - *Out-of-Scope*: Application code, test runs, ADRs, governance file edits, Git commits.
5. **Protected Data & Files**: `backend/sumi.db` (`234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86`), `DECISION_LOG.md`, `EXECUTION_STATE.md`, `SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md`.
6. **Allowed Source Changes**: `docs/operations/agent-sessions/{SESSION_PROTOCOL,CONTEXT_INDEX,PROJECT_SNAPSHOT,TASK_REGISTRY,NEXT_TASK,HANDOFF_TEMPLATE}.md`, `artifacts/local/session-ops/CTX-BOOTSTRAP-001/*`.
7. **Expected Tests**: Path existence verification, `git diff --check`, and `backend/sumi.db` SHA-256 verification.
8. **Acceptance Criteria**: All 6 files created, all referenced paths verified on disk, zero DB mutation, zero governance file mutation.
9. **Required Evidence**: `artifacts/local/session-ops/CTX-BOOTSTRAP-001/logs/{git_state_pre,referenced_paths_verification,governance_files_diff_stat,git_diff_check,git_status_post,db_sha256_post}.log`.
10. **Handoff Path**: `artifacts/local/session-ops/CTX-BOOTSTRAP-001/{SESSION_HANDOFF.md,SESSION_RESULT.json}`.
11. **Commit Permission State**: `DENIED`.
12. **Stop Conditions**: Stop immediately after writing handoff without starting `OWNER-UAT-PREP-001`.

---

### 3.2 `CTX-HARDEN-002` — Session Context Protocol Hardening & Fresh-Session Bootstrap Dry-Run
1. **Unique Task ID**: `CTX-HARDEN-002`
2. **Clear Mission**: Harden the repository-native Agent Session Context System created by `CTX-BOOTSTRAP-001` so fresh-session bootstrapping is progressive (Phase A / Phase B), task boundaries are 12-attribute unambiguous, status semantics distinguish unexecuted Owner UAT from completed agent work, test artifacts stay workspace-local (`artifacts/local/session-ops/<task-id>/test_db/`), document authority is reconciled, and a dry-run walkthrough of `OWNER-UAT-PREP-001` is verified.
3. **Authorization State**: `AUTHORIZED` (Explicit Product Owner prompt `CTX-HARDEN-002`, `2026-10-19`); **Lifecycle Status**: `COMPLETED` (validated in real execution by `OWNER-UAT-PREP-001`).
4. **Explicit In-Scope & Out-of-Scope Work**:
   - *In-Scope*: Edit the 6 `docs/operations/agent-sessions/*.md` files; create `artifacts/local/session-ops/CTX-HARDEN-002/{SESSION_HANDOFF.md,SESSION_RESULT.json,BOOTSTRAP_DRY_RUN_REPORT.md,logs/*}`.
   - *Out-of-Scope*: Owner UAT execution, executing `OWNER-UAT-PREP-001`, application code/test modifications, editing the 3 lagging governance files, editing `CTX-BOOTSTRAP-001` handoff artifacts, or creating Git commits.
5. **Protected Data & Files**: `backend/sumi.db`, `docs/plans/product-completion/{DECISION_LOG,EXECUTION_STATE,SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED}.md`, `docs/architecture/decisions/*`, `artifacts/local/session-ops/CTX-BOOTSTRAP-001/*`, `artifacts/local/full-browser-qa-acceptance-gate/*`.
6. **Allowed Source Changes**: Only `docs/operations/agent-sessions/{SESSION_PROTOCOL,CONTEXT_INDEX,PROJECT_SNAPSHOT,TASK_REGISTRY,NEXT_TASK,HANDOFF_TEMPLATE}.md` and new files under `artifacts/local/session-ops/CTX-HARDEN-002/`.
7. **Expected Tests**: Pre/post Git & DB SHA-256 verification, path existence check, `/tmp`-default elimination check, protected governance diff check, `git diff --check`, and `OWNER-UAT-PREP-001` dry-run verification. Do not run unrelated application tests.
8. **Acceptance Criteria**: All 6 context docs consistent; Phase A/B loading defined; 12-attribute boundaries enforced; `OWNER-UAT-SIGNOFF-001` not misrepresented as completed testing; no `/tmp` default prescribed; 3 governance files untouched; `git diff --check` passes; `backend/sumi.db` SHA-256 unchanged.
9. **Required Evidence**: `artifacts/local/session-ops/CTX-HARDEN-002/{SESSION_HANDOFF.md,SESSION_RESULT.json,BOOTSTRAP_DRY_RUN_REPORT.md}` and `artifacts/local/session-ops/CTX-HARDEN-002/logs/*.log`.
10. **Handoff Path**: `artifacts/local/session-ops/CTX-HARDEN-002/`.
11. **Commit Permission State**: `DENIED`.
12. **Stop Conditions**: Stop immediately after writing `CTX-HARDEN-002` handoff and verification logs; do not execute `OWNER-UAT-PREP-001`.

---

### 3.3 `OWNER-UAT-PREP-001` — Product Owner UAT Workspace-Local DB Setup, Runbook & Consolidated Interactive Scorecard
1. **Unique Task ID**: `OWNER-UAT-PREP-001`
2. **Clear Mission**: Prepare a workspace-local isolated UAT database copy (`artifacts/local/session-ops/OWNER-UAT-PREP-001/test_db/sumi_owner_uat_isolated.db`), clean reset snapshot, CSV fixtures, validated `OWNER_UAT_RUNBOOK.md`, and a single consolidated 5-journey interactive `OWNER_UAT_SCORECARD.md` aligned to commit `d2d048a507370a787a750984e3c1a89690078613`.
3. **Authorization State**: `AUTHORIZED` (Explicit Product Owner prompt `OWNER-UAT-PREP-001`, `2026-10-19`); **Lifecycle Status**: `AWAITING_OWNER_REVIEW` (`Execution State: EXECUTED_AND_VERIFIED`).
4. **Explicit In-Scope & Out-of-Scope Work**:
   - *In-Scope*: Copy `artifacts/local/full-browser-qa-acceptance-gate/test_db/sumi_acceptance_gate_isolated.db` to `artifacts/local/session-ops/OWNER-UAT-PREP-001/test_db/sumi_owner_uat_isolated.db` (plus `sumi_owner_uat_baseline_snapshot.db` and `fixtures/*.csv`); verify expected seeded symbols (`FPT`, `SSI`, `VCI`, `VNINDEX`, `ZERO_PNL_SYM`, `WARM_SYM`, `EMPTY_SYM`) via read-only SQLite inspection; verify local startup, `lsof` DB connection identity, and safe shutdown; write `OWNER_UAT_RUNBOOK.md`, `OWNER_UAT_SCORECARD.md`, `SESSION_HANDOFF.md`, and `SESSION_RESULT.json`; update `PROJECT_SNAPSHOT.md`, `TASK_REGISTRY.md`, and `NEXT_TASK.md`.
   - *Out-of-Scope*: Modifying `backend/` or `frontend/` source/test files; modifying `backend/sumi.db`; using `/tmp` for UAT database storage; modifying `DECISION_LOG.md`, `EXECUTION_STATE.md`, or `SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md`; marking `OWNER_ACCEPTED: true` or `ACCEPTED_V3_COMPLETE: true`; creating Git commits.
5. **Protected Data & Files**: `backend/sumi.db` (`234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86`), `artifacts/local/full-browser-qa-acceptance-gate/test_db/sumi_acceptance_gate_isolated.db` (source fixture — copy only, do not mutate in-place), `docs/plans/product-completion/{DECISION_LOG,EXECUTION_STATE,SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED}.md` (protected), `docs/architecture/decisions/*`, `backend/*`, `frontend/*`.
6. **Allowed Source Changes**: `artifacts/local/session-ops/OWNER-UAT-PREP-001/**` and `docs/operations/agent-sessions/{PROJECT_SNAPSHOT,TASK_REGISTRY,NEXT_TASK}.md`.
7. **Expected Tests**: Pre/post `backend/sumi.db` SHA-256 check; read-only SQLite table/symbol check on `artifacts/local/session-ops/OWNER-UAT-PREP-001/test_db/sumi_owner_uat_isolated.db`; live FastAPI/Vite startup, `lsof` DB connection identity verification, and safe shutdown; protected governance diff check (`3 files changed, 56 insertions(+), 9 deletions(-)`); `git diff --check`.
8. **Acceptance Criteria**: Isolated UAT DB exists inside `artifacts/local/session-ops/OWNER-UAT-PREP-001/test_db/` with verified symbols and `lsof`-confirmed runtime isolation; `OWNER_UAT_RUNBOOK.md` and `OWNER_UAT_SCORECARD.md` provide complete 5-journey checklist with `NOT_TESTED` defaults; `backend/sumi.db` SHA-256 unchanged; `OWNER_ACCEPTED` remains `false`.
9. **Required Evidence**: `artifacts/local/session-ops/OWNER-UAT-PREP-001/{OWNER_UAT_RUNBOOK.md,OWNER_UAT_SCORECARD.md,SESSION_HANDOFF.md,SESSION_RESULT.json}`, `test_db/{sumi_owner_uat_isolated.db,sumi_owner_uat_baseline_snapshot.db,FIXTURE_MANIFEST.json,fixtures/*.csv}`, and `logs/*.log`.
10. **Handoff Path**: `artifacts/local/session-ops/OWNER-UAT-PREP-001/`.
11. **Commit Permission State**: `DENIED`.
12. **Stop Conditions**: Stop immediately upon writing handoff without marking Owner UAT accepted or starting the next task.

---

### 3.4 `OWNER-UAT-SIGNOFF-001` — Interactive Manual Product Owner UAT Execution & V3 Final Acceptance Decision
1. **Unique Task ID**: `OWNER-UAT-SIGNOFF-001`
2. **Clear Mission**: Interactive manual execution of the 5 core user journeys by the Product Owner against a workspace-local isolated database copy, followed by the Product Owner's decision on `OWNER_ACCEPTED` (`ACCEPTED_V3_COMPLETE`) and final release commit authorization.
3. **Authorization State**: `RESERVED_FOR_OWNER` (`PENDING OWNER DECISION`); **Lifecycle Status**: `BLOCKED` (**Manual Product Owner UAT is UNEXECUTED**).
4. **Explicit In-Scope & Out-of-Scope Work**:
   - *In-Scope (Product Owner Only)*: Manual browser walkthrough of Journeys 1–5 and explicit Owner sign-off decision.
   - *Out-of-Scope for Any Autonomous Agent*: No agent session may self-execute manual Owner UAT as a substitute for the Product Owner or set `OWNER_ACCEPTED: true` / `ACCEPTED_V3_COMPLETE: true` without an explicit post-UAT instruction from the Product Owner.
5. **Protected Data & Files**: `backend/sumi.db` (`234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86`).
6. **Allowed Source Changes**: None until the Product Owner explicitly issues post-UAT governance update and commit instructions.
7. **Expected Tests**: Manual Product Owner verification of Journeys 1–5 on `http://127.0.0.1:5173` backed by `artifacts/local/session-ops/OWNER-UAT-PREP-001/test_db/sumi_owner_uat_isolated.db` (or another workspace-local copy under `artifacts/local/session-ops/`).
8. **Acceptance Criteria**: Explicit Product Owner sign-off confirming all 5 journeys pass manual inspection.
9. **Required Evidence**: Completed Owner UAT scorecard and explicit Owner sign-off prompt.
10. **Handoff Path**: `artifacts/local/session-ops/OWNER-UAT-SIGNOFF-001/` (only if an agent session is later dispatched by the Owner to record the Owner's completed decision).
11. **Commit Permission State**: `DENIED` (until explicitly granted by the Product Owner after manual UAT).
12. **Stop Conditions**: Any agent session asked to "complete V3 sign-off" without explicit confirmation that the Product Owner has finished manual UAT must stop and request Owner confirmation.
