# SUMI V3 — REPOSITORY-NATIVE AGENT SESSION PROTOCOL

- **Document ID**: `SUMI-V3-SESSION-PROTOCOL-V2`
- **Classification**: `VERIFIED CURRENT STATE` (Created in `CTX-BOOTSTRAP-001`; hardened in `CTX-HARDEN-002`)
- **Last Updated**: `2026-10-19` (Session `CTX-HARDEN-002`)
- **Repository Root**: `/Users/mizuhara/workspace/sumi`
- **Core Operating Principle**:
  > **ONE BOUNDED TASK = ONE NEW SESSION = PROGRESSIVE CONTEXT LOADING = ONE DURABLE HANDOFF.**

---

## 1. Purpose, Zero-Memory Rule & Authority Hierarchy

Every Sumi task must be executable in a **completely new Antigravity session** with **zero reliance on prior chat history, conversational memory, or previous session identities**.

1. **Repository-Native Recovery**: All required context—governance rules, architectural invariants, financial formulas, current execution state, historical QA evidence, and task boundaries—must be recovered from repository files and live Git/database checks.
2. **Reconciliation Over Assumption**: If actual Git state, working-tree files, or database checksums differ from a recorded snapshot, the active session must reconcile the difference using `git log`, `git status`, `git diff`, and authoritative repository documents without destructive Git operations.
3. **Evidence Is Not Owner Approval**: A passing test suite or completed session handoff is technical or operational evidence (`HISTORICAL EVIDENCE` / `AWAITING_OWNER_REVIEW`), never automatic Product Owner acceptance (`OWNER_ACCEPTED` / `ACCEPTED_V3_COMPLETE`).

### 1.1 Explicit Document Authority & Precedence Hierarchy

When documents disagree or lag behind recent commits, sessions must resolve authority in the following strict order (highest to lowest):

| Rank | Authority Tier | Canonical Sources | Precedence & Reconciliation Rule |
| :---: | :--- | :--- | :--- |
| **1** | **Live Workspace & Cryptographic Ground Truth** (`VERIFIED CURRENT STATE`) | Live outputs of `git rev-parse HEAD`, `git status --short --untracked-files=all`, `git diff`, and `shasum -a 256 backend/sumi.db` in the active session | Overrides any stale text in snapshots or handoffs. Must be verified in Phase A before any task work. |
| **2** | **Top-Level Operating Charter, Ratified ADRs & Release Criteria** (`RATIFIED DECISION`) | [`AGENTS.md`](../../../AGENTS.md), [`docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`](../../PRODUCT_ACCEPTANCE_CRITERIA_V3.md), [`docs/testing/FINANCIAL_GOLDEN_CASES.md`](../../testing/FINANCIAL_GOLDEN_CASES.md), [`ADR-001`](../../ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md), [`ADR-002`](../../ARCHITECTURE_DECISION_002_MARKET_DATA_PROVIDER.md), [`docs/architecture/decisions/ADR-*.md`](../../architecture/decisions/) | Non-negotiable product, architectural, and financial invariants. Cannot be weakened without explicit Owner/Architect decision. |
| **3** | **Ratified Governance Decision Log** (`RATIFIED DECISION` — *Lagging Working-Tree Note*) | [`docs/plans/product-completion/DECISION_LOG.md`](../../plans/product-completion/DECISION_LOG.md) | Authoritative for ratified decisions `DEC-01..DEC-10`, `DEC-TRAIN01..04`, and `DEC-REM-01..08`. **Note**: Contains preserved uncommitted changes from `2026-10-17` (commit `925a095`) and lags behind `HEAD` (`d2d048a`, `2026-10-19`). Do not edit, reset, stage, or commit without Owner authorization. |
| **4** | **Active Session-Context Operational Records** (`VERIFIED CURRENT STATE`) | [`SESSION_PROTOCOL.md`](SESSION_PROTOCOL.md), [`PROJECT_SNAPSHOT.md`](PROJECT_SNAPSHOT.md), [`TASK_REGISTRY.md`](TASK_REGISTRY.md), [`CONTEXT_INDEX.md`](CONTEXT_INDEX.md), [`HANDOFF_TEMPLATE.md`](HANDOFF_TEMPLATE.md) | Authoritative operational orientation reconciled to current Git `HEAD` (`d2d048a`). Always cross-check against Rank 1 and Rank 2. |
| **5** | **Historical Baseline Handoffs & Lagging Execution Trackers** (`HISTORICAL EVIDENCE`) | [`artifacts/local/full-browser-qa-acceptance-gate/FINAL_BASELINE_HANDOFF.md`](../../../artifacts/local/full-browser-qa-acceptance-gate/FINAL_BASELINE_HANDOFF.md), [`FINAL_ACCEPTANCE_GATE_REPORT.md`](../../../artifacts/local/full-browser-qa-acceptance-gate/FINAL_ACCEPTANCE_GATE_REPORT.md), [`artifacts/local/session-ops/`](../../../artifacts/local/session-ops/), and lagging working-tree files [`EXECUTION_STATE.md`](../../plans/product-completion/EXECUTION_STATE.md) & [`SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md`](../../plans/product-completion/SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md) | Historical verification records on disk. `FINAL_BASELINE_HANDOFF.md` documents commit `d2d048a`; `EXECUTION_STATE.md` and `SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md` stop at `925a095` (`2026-10-17`). Note: workspace-local path rules in Rank 4 (`artifacts/local/session-ops/<task-id>/test_db/`) supersede legacy `/tmp` examples in historical reports. |
| **6** | **Candidate Task Proposals & Unratified Roadmap Drafts** (`PROPOSED / UNRATIFIED`) | [`NEXT_TASK.md`](NEXT_TASK.md), `PROPOSED` entries in [`TASK_REGISTRY.md`](TASK_REGISTRY.md), unratified sections of [`SUMI_V3_PRODUCT_COMPLETION_MASTER_PLAN_PROPOSED.md`](../../plans/product-completion/SUMI_V3_PRODUCT_COMPLETION_MASTER_PLAN_PROPOSED.md) | Non-binding proposals. Never execute a `PROPOSED` task or promote a proposal into policy without explicit Product Owner authorization. |

---

## 2. Epistemic Classification Standard

Every session must classify facts, claims, and recommendations using these six explicit labels:

| Classification Label | Definition & Usage Rule |
| :--- | :--- |
| **`VERIFIED CURRENT STATE`** | Directly confirmed in the **current session** by inspecting repository files, running `git status`/`git log`/`git diff`, verifying file existence/checksums (`shasum -a 256`), or executing verification commands in the current session. |
| **`RATIFIED DECISION`** | Architectural, financial, or governance rule formally approved in [`AGENTS.md`](../../../AGENTS.md), [`docs/architecture/decisions/`](../../architecture/decisions/), or [`DECISION_LOG.md`](../../plans/product-completion/DECISION_LOG.md). |
| **`HISTORICAL EVIDENCE`** | Test results, QA verdicts, screenshots, or handoff claims recorded by a **prior session** whose artifact files exist on disk (`artifacts/local/...`), but whose runtime/test commands were not re-executed in the current session. Never promote historical claims into `VERIFIED CURRENT STATE` without re-verification. |
| **`PROPOSED / UNRATIFIED`** | Recommended next tasks, proposed architecture/roadmap adjustments, or unratified plan drafts awaiting Technical Architect or Product Owner approval. |
| **`PENDING OWNER DECISION`** | Actions or state transitions strictly reserved for the Product Owner (e.g., `OWNER_ACCEPTED` / `ACCEPTED_V3_COMPLETE` sign-off, task dispatch authorization, Git commit authorization). |
| **`UNKNOWN / REQUIRES VERIFICATION`** | Any runtime behavior, external data state, or uninspected subsystem that cannot be confirmed from static inspection or current-session checks. |

---

## 3. Two-Phase Progressive Context Loading (Mandatory Startup Discipline)

Future sessions **MUST NOT** indiscriminately ingest all repository documentation. Context loading is strictly divided into **Phase A (Minimal Bootstrap)** and **Phase B (Task-Specific Context)**.

### Phase A — Minimal Bootstrap (Execute First in Every Session)

A new session must perform **only** these 6 steps during initial bootstrap:

1. **Read [`AGENTS.md`](../../../AGENTS.md)**: Recover non-negotiable product invariants, architecture boundaries, escalation rules, and workspace safety rules.
2. **Read [`SESSION_PROTOCOL.md`](SESSION_PROTOCOL.md)**: Recover this two-phase startup sequence, authority hierarchy, task boundary contract, and artifact/DB isolation rules.
3. **Read [`PROJECT_SNAPSHOT.md`](PROJECT_SNAPSHOT.md)**: Recover the verified Git `HEAD` (`d2d048a507370a787a750984e3c1a89690078613`), the 3 preserved lagging uncommitted governance files, the `backend/sumi.db` checksum, and current product readiness state (`READY_FOR_OWNER_UAT`, `OWNER_ACCEPTED: false`).
4. **Locate Authorized Task ID in [`TASK_REGISTRY.md`](TASK_REGISTRY.md) (or Task Brief)**:
   - Find the assigned `Task ID` in `TASK_REGISTRY.md` (and read `NEXT_TASK.md` **only** if the assigned `Task ID` matches the candidate task in `NEXT_TASK.md` or if the task updates `NEXT_TASK.md`).
   - Verify the task's **Authorization State** (`AUTHORIZED` vs `PROPOSED` / `BLOCKED`) and all 12 task boundary attributes (§4).
   - If the task is `PROPOSED` in `TASK_REGISTRY.md` and the current session was **not** launched by an explicit Product Owner prompt authorizing that `Task ID`, **STOP immediately**.
5. **Verify Live Git `HEAD`, Dirty State, and Safety Invariants**:
   Run this read-only verification command and save the output to `artifacts/local/session-ops/<task-id>/logs/git_state_pre.log`:
   ```bash
   git rev-parse --abbrev-ref HEAD
   git rev-parse HEAD
   git log -n 5 --oneline
   git status --short --untracked-files=all
   git diff --stat docs/plans/product-completion/DECISION_LOG.md docs/plans/product-completion/EXECUTION_STATE.md docs/plans/product-completion/SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md
   shasum -a 256 backend/sumi.db
   wc -c backend/sumi.db
   ```
   - Confirm `HEAD` matches `d2d048a507370a787a750984e3c1a89690078613` (or reconcile via `git log` without destructive Git commands if a new commit was authorized).
   - Confirm `backend/sumi.db` equals `234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86` (`188,416` bytes).
   - Confirm the 3 modified governance files (`DECISION_LOG.md`, `EXECUTION_STATE.md`, `SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md`) are identified as protected working-tree files.
6. **Identify Only Task-Specific Documents Required**:
   Consult the task's `Required Context Pack` in `TASK_REGISTRY.md` (and [`CONTEXT_INDEX.md`](CONTEXT_INDEX.md) as a lookup directory) to list **only** the specific files needed for Phase B.

> [!NOTE]
> **Reconciling `AGENTS.md` Context Recovery with Progressive Loading**: `AGENTS.md` references the 4 V3 completion governance documents (`SUMI_V3_PRODUCT_COMPLETION_MASTER_PLAN_PROPOSED.md`, `SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md`, `EXECUTION_STATE.md`, `DECISION_LOG.md`). `PROJECT_SNAPSHOT.md` already distills their verified state and documents that the 3 uncommitted files stop at commit `925a095` (`2026-10-17`), one commit behind `HEAD` (`d2d048a`, `2026-10-19`). Sessions should read those 4 full documents in Phase B **only** when their bounded task directly involves batch status, governance reconciliation, or decision history.

### Phase B — Task-Specific Context (Read Selectively)

After completing Phase A, read **only** the documents applicable to the active task:

- **Ratified ADRs**: Read only the `ADR-*` files governing the subsystem in scope (e.g., `ADR-EXEC-001` and `ADR-SETTLE-001` for trading/settlement; `ADR-FE-001`, `ADR-WS-001`, and `ADR-CHART-001` for Replay UI).
- **Source Modules**: Read only the `backend/` or `frontend/` files permitted or inspected by the task.
- **Product Requirements**: Read the relevant sections of [`docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`](../../PRODUCT_ACCEPTANCE_CRITERIA_V3.md) or [`docs/testing/WEBSOCKET_TEST_MATRIX.md`](../../testing/WEBSOCKET_TEST_MATRIX.md).
- **Test Specifications**: Read [`docs/testing/FINANCIAL_GOLDEN_CASES.md`](../../testing/FINANCIAL_GOLDEN_CASES.md) or specific test files relevant to the task.
- **Previous Handoffs & Evidence**: Read only the directly relevant prior handoff (e.g., [`FINAL_BASELINE_HANDOFF.md`](../../../artifacts/local/full-browser-qa-acceptance-gate/FINAL_BASELINE_HANDOFF.md) or the immediate predecessor session handoff).
- **Related Decisions**: Read only the relevant entries in [`DECISION_LOG.md`](../../plans/product-completion/DECISION_LOG.md).

#### Strict Phase B Prohibitions
1. **Do NOT read unrelated development history**: Do not read `SUMI_GAP_REGISTER.md`, `DEV_BATCHES.md`, `V3_FINAL_HANDOFF_REPORT_2026-09-12.md`, or historical `development-train-01..04` folders unless explicitly required by the task.
2. **Do NOT assume a document is authoritative merely because it appears in `CONTEXT_INDEX.md`**: Always check its `Authority Level` column in `CONTEXT_INDEX.md` and apply §1.1 precedence rules.
3. **Do NOT silently promote a proposal, historical claim, or unratified decision into project policy**: Keep `PROPOSED / UNRATIFIED` and `HISTORICAL EVIDENCE` strictly labeled as such.

---

## 4. Mandatory 12-Attribute Task Boundary Contract

Every task registered in [`TASK_REGISTRY.md`](TASK_REGISTRY.md), proposed in [`NEXT_TASK.md`](NEXT_TASK.md), or executed in a session must explicitly specify all **12 boundary attributes**:

| # | Boundary Attribute | Mandatory Requirement |
| :---: | :--- | :--- |
| **1** | **Unique Task ID** | Immutable identifier (e.g., `CTX-HARDEN-002`, `OWNER-UAT-PREP-001`). |
| **2** | **Clear Mission** | Concise statement of the single objective for the session. |
| **3** | **Authorization State** | Explicit authorization status (`AUTHORIZED`, `PROPOSED — NOT AUTHORIZED`, or `RESERVED_FOR_OWNER`) + source of authorization. |
| **4** | **Explicit In-Scope & Out-of-Scope Work** | Exact work items permitted and exact work items prohibited. |
| **5** | **Protected Data & Files** | Files that must remain untouched (`backend/sumi.db`, the 3 uncommitted governance files `DECISION_LOG.md`, `EXECUTION_STATE.md`, `SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md`, ratified ADRs, prior session handoffs). |
| **6** | **Allowed Source Changes** | Exact allowlist of repository paths the session may create or edit (or `NONE` for read-only/prep tasks). |
| **7** | **Expected Tests** | Exact verification commands to execute in the session (or explicit instruction not to run unrelated application test suites). |
| **8** | **Acceptance Criteria** | Verifiable conditions required for the task to be considered complete. |
| **9** | **Required Evidence** | Exact logs, checksums, screenshots, or reports that must be retained on disk. |
| **10** | **Handoff Path** | `artifacts/local/session-ops/<task-id>/` (`SESSION_HANDOFF.md`, `SESSION_RESULT.json`, `logs/`). |
| **11** | **Commit Permission State** | **Default: `DENIED`**. May only be `AUTHORIZED` if the Product Owner's prompt for that specific task expressly grants commit permission. |
| **12** | **Stop Conditions** | Explicit conditions under which the session must halt immediately (including task completion, missing authorization, DB mutation risk, or escalation triggers). |

### Non-Negotiable Task Scope Rules
- **Default Commit Permission is `DENIED`**: Never stage or commit changes unless expressly authorized by the Product Owner for the active task.
- **No Automatic Task Chaining**: A task's completion does **not** automatically authorize the next task.
- **No Mission Expansion**: A session must **not** expand its mission merely because a candidate next task is visible in `TASK_REGISTRY.md` or `NEXT_TASK.md`.

---

## 5. Workspace-Local Artifact & Isolated Test Database Standard

1. **Workspace-Local Persistence Rule**: All persistent session outputs, logs, handoffs, and isolated test databases must remain inside `/Users/mizuhara/workspace/sumi` (under git-ignored `artifacts/local/`).
2. **Preferred Session Artifact Directory**:
   ```text
   artifacts/local/session-ops/<task-id>/
   ```
3. **Preferred Isolated Test / UAT Database Directory**:
   ```text
   artifacts/local/session-ops/<task-id>/test_db/
   ```
   Example isolated backend launch command:
   ```bash
   mkdir -p artifacts/local/session-ops/<task-id>/test_db
   cp artifacts/local/full-browser-qa-acceptance-gate/test_db/sumi_acceptance_gate_isolated.db \
     artifacts/local/session-ops/<task-id>/test_db/sumi_<task-id>_isolated.db

   PYTHONPATH=backend DATABASE_URL="sqlite:////Users/mizuhara/workspace/sumi/artifacts/local/session-ops/<task-id>/test_db/sumi_<task-id>_isolated.db" \
     ./.venv/bin/uvicorn app.main:app --host 127.0.0.1 --port 8000
   ```
4. **No `/tmp` Default**: Do **not** prescribe `/tmp` as the default UAT, test database, or fixture location. (Note: Where historical reports such as `FINAL_BASELINE_HANDOFF.md` §8 showed `/tmp/sumi-owner-uat/` examples, this protocol supersedes them with `artifacts/local/session-ops/<task-id>/test_db/`.)
5. **Narrow External Temporary File Exception**: If a narrowly scoped temporary external file outside the workspace is strictly required by an OS or tool constraint (e.g., UNIX socket path length limits), the session must document in `SESSION_HANDOFF.md`:
   - The exact technical reason why `artifacts/local/session-ops/<task-id>/` could not be used;
   - The file's exact path and lifetime;
   - The cleanup obligation (removing only the exact file created by this session—**never** modifying existing external files or cleaning unrelated directories);
   - Verification that cleanup succeeded.
6. **Production Database Inviolability**: Never run state-changing tests, migrations, seeds, or manual UAT against `backend/sumi.db` (`SHA-256: 234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86`, `188,416` bytes).

---

## 6. Mandatory 7-Step Session Execution Lifecycle

1. **Step 1 — Execute Phase A Minimal Bootstrap (§3)**: Read `AGENTS.md`, `SESSION_PROTOCOL.md`, `PROJECT_SNAPSHOT.md`, and locate the assigned `Task ID` in `TASK_REGISTRY.md`. Verify live Git and `backend/sumi.db` state (`logs/git_state_pre.log`).
2. **Step 2 — Verify Task Authorization & 12 Boundary Attributes (§4)**: Confirm the task is explicitly authorized and record its allowed files, protected files, commit permission (`DENIED` by default), and stop conditions.
3. **Step 3 — Execute Phase B Selective Context Loading (§3)**: Read **only** the task-specific authoritative documents and files required by the task.
4. **Step 4 — Execute Bounded Task Within Architectural & Safety Guardrails**: Perform only the in-scope work while preserving all invariants (`ADR-*`, `G-01..G-12`, zero future leakage, `backend/sumi.db` untouched, 3 uncommitted governance files untouched).
5. **Step 5 — Run Task-Specific Verification & Retain Evidence (§5)**: Run only the verification checks appropriate to the task (isolated DB in `artifacts/local/session-ops/<task-id>/test_db/` if running tests/servers; path, diff, and checksum checks for documentation/governance tasks).
6. **Step 6 — Write Durable Handoff & Update Session Context**:
   - Create `artifacts/local/session-ops/<task-id>/SESSION_HANDOFF.md` and `SESSION_RESULT.json` per [`HANDOFF_TEMPLATE.md`](HANDOFF_TEMPLATE.md).
   - Update `PROJECT_SNAPSHOT.md`, `TASK_REGISTRY.md`, and `NEXT_TASK.md` if required by the task.
   - Run post-session `git diff --check`, `git status --short --untracked-files=all`, and `shasum -a 256 backend/sumi.db`.
7. **Step 7 — Stop Cleanly**: End the session immediately after writing and verifying the handoff. Never start the next task in the same session.

---

## 7. Strict Restrictions & Safety Guardrails

| Guardrail Area | Mandatory Restriction |
| :--- | :--- |
| **Git Commits & History** | Default commit permission is **`DENIED`**. Do **not** run `git add`, `git commit`, `git push`, `git tag`, `git stash`, `git rebase`, or `git merge` unless expressly authorized by the Product Owner for the active task. |
| **Destructive Git / File Operations** | Never run `git reset --hard`, `git checkout --`, `git restore`, `git clean`, or `rm` on tracked files, prior `artifacts/local/` directories, or pre-existing uncommitted changes. |
| **Production Database (`backend/sumi.db`)** | Strictly read-only (`SHA-256: 234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86`, `188,416` bytes). Verify SHA-256 before and after every session. |
| **Pre-Existing Uncommitted Governance Files** | Preserve uncommitted changes in `docs/plans/product-completion/DECISION_LOG.md`, `EXECUTION_STATE.md`, and `SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md` (`3 files changed, 56 insertions(+), 9 deletions(-)`) untouched until the Product Owner explicitly authorizes reconciling or committing them. |
| **Ratified ADRs & Financial Rules** | Do not edit files in `docs/architecture/decisions/` or alter ratified financial formulas (`FINANCIAL_GOLDEN_CASES.md`, `ADR-EXEC-001`, `ADR-SETTLE-001`) without explicit Owner/Architect authorization. |
| **Artifact & Test DB Location** | Keep all persistent artifacts under `artifacts/local/session-ops/<task-id>/` and isolated test DBs under `artifacts/local/session-ops/<task-id>/test_db/`. Do not use `/tmp` as a default location. |

---

## 8. Standard Minimal Bootstrap Prompt for Future Sessions

To launch any future session with zero prior chat history, provide only:

```markdown
Repository: /Users/mizuhara/workspace/sumi
Task ID: <TASK-ID>
Mission: <One-sentence bounded mission>

Follow `AGENTS.md` and `docs/operations/agent-sessions/SESSION_PROTOCOL.md`.
```
