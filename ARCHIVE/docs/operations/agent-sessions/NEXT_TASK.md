# SUMI V3 — NEXT RECOMMENDED BOUNDED TASK

- **Document ID**: `SUMI-V3-NEXT-TASK-V7`
- **Updated By Session**: `SUMI-REL001-BASELINE-CLOSE-001` (`2026-10-20`)
- **Classification**: `PENDING OWNER DECISION` (`RESERVED_FOR_OWNER` / `BLOCKED`)

> [!IMPORTANT]
> **Non-Execution & Non-Expansion Rule**:
> 1. `SUMI-REL001-BASELINE-CLOSE-001` has verified and closed the independently accepted `REL-001` remediation (`SUMI-REL001-REMEDIATION-001` + `SUMI-REL001-INDEPENDENT-RETEST-001`) as exactly one controlled commit on `master` at **`864e751ebaf979114b5bc7b5db6c12dc52b2e5ab`** (`9` files: `7` modified source files + `2` new test files) under `artifacts/local/session-ops/SUMI-REL001-BASELINE-CLOSE-001/`.
> 2. The next milestone is **`OWNER-UAT-SIGNOFF-001` (Interactive Manual Product Owner UAT Execution & V3 Final Acceptance / Release Decision)**, which is **`RESERVED_FOR_OWNER`** and **`BLOCKED`** pending the Product Owner's manual UAT execution and decision.
> 3. **No autonomous agent session may self-execute manual Owner UAT on behalf of the Product Owner, commit governance/session-context files without Owner authorization, change the release verdict to `GO` automatically, or mark `OWNER_ACCEPTED: true` / `ACCEPTED_V3_COMPLETE: true`.**

---

## 1. Strategic Context After `SUMI-REL001-BASELINE-CLOSE-001`

All 16 product-completion batches (`B00–B51`), Wave `W5` UAT remediations (`DEF-W5-01..05`), Independent QA remediations (`DEF-001..006, 008, 012`), Full-Browser QA acceptance remediations (`SQA-001..005, 009..018` @ `d2d048a`), and `REL-001` CafeF raw data import correctness (`SUMI-REL001-REMEDIATION-001` / `SUMI-REL001-INDEPENDENT-RETEST-001` @ `864e751`) are committed on `master` at `864e751ebaf979114b5bc7b5db6c12dc52b2e5ab`.

Session `SUMI-RELEASE-GATE-001` operated the live application across all 5 end-to-end user journeys on Condition A (deterministic fixtures) and Condition B (real-market CafeF CSVs), producing:
- Final Release Readiness Report: `artifacts/local/session-ops/SUMI-RELEASE-GATE-001/reports/FINAL_RELEASE_READINESS_REPORT.md`
- Feature Coverage Matrix (`44` features): `artifacts/local/session-ops/SUMI-RELEASE-GATE-001/reports/RELEASE_FEATURE_COVERAGE_MATRIX.md`
- Product Quality Scorecard (`9.33 / 10`): `artifacts/local/session-ops/SUMI-RELEASE-GATE-001/reports/PRODUCT_QUALITY_SCORECARD.md`
- Release Issue Register (`REL-001..REL-006`): `artifacts/local/session-ops/SUMI-RELEASE-GATE-001/reports/RELEASE_ISSUE_REGISTER.md`

**Manual Product Owner UAT (`OWNER-UAT-SIGNOFF-001`) remains `NOT_EXECUTED` (`Lifecycle Status: BLOCKED`, `OWNER_ACCEPTED: false`).**

---

## 2. Complete 12-Attribute Task Boundary Specification: `OWNER-UAT-SIGNOFF-001`

| # | Attribute | Specification |
| :---: | :--- | :--- |
| **1** | **Unique Task ID** | `OWNER-UAT-SIGNOFF-001` |
| **2** | **Clear Mission** | Interactive manual execution of the 5 core user journeys by the Product Owner against `artifacts/local/session-ops/OWNER-UAT-PREP-001/test_db/sumi_owner_uat_isolated.db`, followed by the Product Owner's decision on `OWNER_ACCEPTED` (`ACCEPTED_V3_COMPLETE`) and final release commit authorization. |
| **3** | **Authorization State** | **`RESERVED_FOR_OWNER`** (`Lifecycle Status: BLOCKED`, `Execution State: NOT_EXECUTED`, `PENDING OWNER DECISION`). Only after the Product Owner manually completes UAT and issues an explicit post-UAT prompt may an agent session be dispatched to record the Owner's verdict and reconcile/commit governance files. |
| **4** | **Explicit In-Scope & Out-of-Scope Work** | **In-Scope (Product Owner Manual Walkthrough)**:<br>1. Start isolated backend (`:8000`) and frontend (`:5173`) per `artifacts/local/session-ops/OWNER-UAT-PREP-001/OWNER_UAT_RUNBOOK.md`.<br>2. Walk through Journeys 1–5 in `artifacts/local/session-ops/OWNER-UAT-PREP-001/OWNER_UAT_SCORECARD.md`.<br>3. Record `PASS` / `FAIL` / `BLOCKED` verdicts and decide whether Sumi V3 is accepted (`OWNER_ACCEPTED: true`) or requires follow-up fixes.<br><br>**Out-of-Scope for Any Autonomous Agent**:<br>- Self-executing manual Product Owner UAT on behalf of the Owner.<br>- Setting `OWNER_ACCEPTED: true` or `ACCEPTED_V3_COMPLETE: true` without explicit post-UAT Owner instruction.<br>- Modifying `backend/sumi.db` or the 3 protected governance files (`DECISION_LOG.md`, `EXECUTION_STATE.md`, `SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md`) without explicit Owner authorization. |
| **5** | **Protected Data & Files** | - `backend/sumi.db` (`188,416` bytes, SHA-256 `234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86`)<br>- `artifacts/local/session-ops/OWNER-UAT-PREP-001/test_db/sumi_owner_uat_baseline_snapshot.db`<br>- `artifacts/local/full-browser-qa-acceptance-gate/test_db/sumi_acceptance_gate_isolated.db`<br>- `docs/plans/product-completion/{DECISION_LOG,EXECUTION_STATE,SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED}.md` (until explicitly unlocked by Owner after UAT) |
| **6** | **Allowed Source Changes** | **`NONE`** (until the Product Owner explicitly issues post-UAT governance closure or defect remediation instructions). |
| **7** | **Expected Tests** | Manual Product Owner browser walkthrough of Journeys 1–5 on `http://127.0.0.1:5173` backed by `sumi_owner_uat_isolated.db`. |
| **8** | **Acceptance Criteria** | Explicit Product Owner sign-off confirming all 5 journeys pass manual inspection (or explicit Owner defect list if changes are requested). |
| **9** | **Required Evidence** | Completed `OWNER_UAT_SCORECARD.md` and explicit Product Owner decision prompt. |
| **10** | **Handoff Path** | `artifacts/local/session-ops/OWNER-UAT-SIGNOFF-001/` *(only if an agent session is later dispatched by the Owner to record post-UAT sign-off)* |
| **11** | **Commit Permission State** | **`DENIED`** (until explicitly granted by the Product Owner after manual UAT). |
| **12** | **Stop Conditions** | Any agent session invoked before the Product Owner completes manual UAT must halt immediately if asked to mark Sumi V3 `OWNER_ACCEPTED` or `ACCEPTED_V3_COMPLETE`. |

---

## 3. Progressive Context Pack for Any Post-UAT Follow-Up Session

If the Product Owner launches a post-UAT session after completing manual UAT:
- **Phase A (Minimal Bootstrap)**:
  1. `AGENTS.md`
  2. `docs/operations/agent-sessions/SESSION_PROTOCOL.md`
  3. `docs/operations/agent-sessions/PROJECT_SNAPSHOT.md`
  4. `docs/operations/agent-sessions/TASK_REGISTRY.md` and `docs/operations/agent-sessions/NEXT_TASK.md`
  5. Live `git rev-parse HEAD`, `git status --short --untracked-files=all`, and `shasum -a 256 backend/sumi.db`
- **Phase B (Task-Specific Context)**:
  1. `artifacts/local/session-ops/OWNER-UAT-PREP-001/SESSION_HANDOFF.md`
  2. `artifacts/local/session-ops/OWNER-UAT-PREP-001/OWNER_UAT_SCORECARD.md`
  3. `docs/operations/agent-sessions/HANDOFF_TEMPLATE.md`
  4. *(Only if explicitly unlocked by the Owner for post-UAT closure)* `docs/plans/product-completion/{DECISION_LOG,EXECUTION_STATE,SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED}.md`
