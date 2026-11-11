# SUMI V3 — STANDARD SESSION HANDOFF TEMPLATE

- **Document ID**: `SUMI-V3-HANDOFF-TEMPLATE-V2`
- **Classification**: `VERIFIED CURRENT STATE` (Created in `CTX-BOOTSTRAP-001`; hardened in `CTX-HARDEN-002`)
- **Last Updated**: `2026-10-19` (Session `CTX-HARDEN-002`)
- **Target Output Location**: `artifacts/local/session-ops/<TASK-ID>/SESSION_HANDOFF.md` (with companion `SESSION_RESULT.json`)

Every Antigravity session must conclude by writing a complete `SESSION_HANDOFF.md` and `SESSION_RESULT.json` under `artifacts/local/session-ops/<TASK-ID>/` following the exact structure below. All persistent artifacts and any isolated test databases (`artifacts/local/session-ops/<TASK-ID>/test_db/`) must remain inside `/Users/mizuhara/workspace/sumi`.

---

## 1. Markdown Handoff Template (`SESSION_HANDOFF.md`)

```markdown
# SUMI V3 — SESSION HANDOFF: `<TASK-ID>`

- **Task ID**: `<TASK-ID>`
- **Task Title**: `<Concise Task Title>`
- **Session Timestamp (Start → End)**: `<ISO-8601 Start>` → `<ISO-8601 End>`
- **Repository**: `/Users/mizuhara/workspace/sumi`
- **Task Lifecycle Status**: `<COMPLETED | AWAITING_OWNER_REVIEW | BLOCKED | IN_PROGRESS>`
- **Authorization State**: `<AUTHORIZED | PROPOSED | RESERVED_FOR_OWNER>` (`<Source of authorization>`)
- **Commit Permission State**: `DENIED` *(default unless expressly authorized by Product Owner)*
- **Owner UAT Execution & Acceptance Status**: `NOT_EXECUTED` / `PENDING OWNER DECISION` (`OWNER_ACCEPTED: false`)

---

## 1. Epistemic Classification Used in This Handoff

- **`VERIFIED CURRENT STATE`**: Directly confirmed by commands, file reads, or checksums executed in **this session**.
- **`RATIFIED DECISION`**: Approved rule in `AGENTS.md`, `docs/architecture/decisions/`, or `DECISION_LOG.md`.
- **`HISTORICAL EVIDENCE`**: Prior session test/QA results or reports verified on disk (`artifacts/local/...`) but not re-executed in this session (`REPORTED`).
- **`PROPOSED / UNRATIFIED`**: Recommendations or next-task proposals awaiting Product Owner / Architect approval (`PROPOSED`).
- **`PENDING OWNER DECISION`**: Sign-offs, task authorizations, or commit decisions strictly reserved for the Product Owner.
- **`UNKNOWN / REQUIRES VERIFICATION`**: Unverified runtime or external state.

---

## 2. Mandatory 12-Attribute Task Boundary Contract

| # | Boundary Attribute | Verified Value for `<TASK-ID>` |
| :---: | :--- | :--- |
| **1** | **Unique Task ID** | `<TASK-ID>` |
| **2** | **Clear Mission** | `<Single bounded objective>` |
| **3** | **Authorization State** | `<AUTHORIZED / PROPOSED>` (`<Prompt or Decision reference>`) |
| **4** | **Explicit In-Scope & Out-of-Scope Work** | **In-Scope**: `<list>`<br>**Out-of-Scope**: `<list>` |
| **5** | **Protected Data & Files** | `backend/sumi.db` (`234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86`), `DECISION_LOG.md`, `EXECUTION_STATE.md`, `SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md`, `<other protected paths>` |
| **6** | **Allowed Source Changes** | `<Exact file allowlist or NONE>` |
| **7** | **Expected Tests** | `<Exact verification commands or checks>` |
| **8** | **Acceptance Criteria** | `<Verifiable completion criteria>` |
| **9** | **Required Evidence** | `artifacts/local/session-ops/<TASK-ID>/{SESSION_HANDOFF.md,SESSION_RESULT.json,logs/*}` |
| **10** | **Handoff Path** | `artifacts/local/session-ops/<TASK-ID>/` (Isolated DB dir if applicable: `artifacts/local/session-ops/<TASK-ID>/test_db/`) |
| **11** | **Commit Permission State** | `<DENIED (default) or AUTHORIZED>` |
| **12** | **Stop Conditions** | `<Conditions requiring immediate halt>` |

---

## 3. Starting Git & Database State (`VERIFIED CURRENT STATE`)

- **Branch**: `<branch>`
- **Starting `HEAD`**: `<40-char SHA>` (`<short subject>`)
- **Starting `git status --short --untracked-files=all`**:
  ```text
  <exact pre-session git status output>
  ```
- **Starting `backend/sumi.db` Integrity**:
  - Size: `<bytes>` bytes
  - SHA-256: `<sha256>`

---

## 4. Progressive Context Loaded (Phase A vs. Phase B)

### 4.1 Phase A — Minimal Bootstrap Documents Read
| Document Path | Authority Level | Purpose |
| :--- | :--- | :--- |
| `AGENTS.md` | `RATIFIED DECISION` | Operating rules & invariants |
| `docs/operations/agent-sessions/SESSION_PROTOCOL.md` | `VERIFIED CURRENT STATE` | Two-phase startup & guardrails |
| `docs/operations/agent-sessions/PROJECT_SNAPSHOT.md` | `VERIFIED CURRENT STATE` | Current baseline & dirty working-tree reconciliation |
| `docs/operations/agent-sessions/TASK_REGISTRY.md` | `VERIFIED CURRENT STATE` | Task authorization & 12-attribute boundary lookup |

### 4.2 Phase B — Task-Specific Documents Read
| Document Path | Authority Level | Relevance to `<TASK-ID>` |
| :--- | :--- | :--- |
| `<path>` | `<RATIFIED DECISION / HISTORICAL EVIDENCE / VERIFIED CURRENT STATE>` | `<why consulted>` |

---

## 5. Work Actually Performed (`VERIFIED CURRENT STATE`)

1. `<Step-by-step summary of actual actions taken in this session>`

---

## 6. Files Created or Changed (`VERIFIED CURRENT STATE`)

| File Path | Action (`CREATED` / `MODIFIED` / `UNCHANGED_PROTECTED`) | Purpose |
| :--- | :--- | :--- |
| `<path>` | `<action>` | `<summary>` |

---

## 7. Tests & Verification Commands Executed

### 7.1 Commands Executed in This Session (`VERIFIED CURRENT STATE`)
| Check | Exact Command | Exit Code & Observed Result | Log Path |
| :--- | :--- | :--- | :--- |
| `<check>` | `<command>` | `<result>` | `artifacts/local/session-ops/<TASK-ID>/logs/<file>.log` |

### 7.2 Prior Baseline Evidence Referenced (`HISTORICAL EVIDENCE` / `REPORTED`)
- `<Any historical test/QA metrics referenced without re-execution in this session>`

---

## 8. Retained Evidence Paths (Workspace-Local)

- `artifacts/local/session-ops/<TASK-ID>/SESSION_HANDOFF.md`
- `artifacts/local/session-ops/<TASK-ID>/SESSION_RESULT.json`
- `artifacts/local/session-ops/<TASK-ID>/logs/...`
- *(If applicable)* `artifacts/local/session-ops/<TASK-ID>/test_db/...`

---

## 9. Production Database Integrity (`VERIFIED CURRENT STATE`)

| Check Phase | File Path | Size (Bytes) | SHA-256 | Delta |
| :--- | :--- | ---: | :--- | :--- |
| Pre-Session | `backend/sumi.db` | `188,416` | `234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86` | Baseline |
| Post-Session | `backend/sumi.db` | `188,416` | `234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86` | **`0` bytes (`UNCHANGED`)** |

---

## 10. Problems, Limitations & Contradictions Observed

- `<Document any defects, blockers, or documentation/governance contradictions with exact paths and epistemic labels>`

---

## 11. Pending Owner Decisions (`PENDING OWNER DECISION`)

1. `<Decision item 1>`
2. `<Decision item 2>`

---

## 12. Final Git State (`VERIFIED CURRENT STATE`)

- **Final Branch & `HEAD`**: `<branch>` @ `<40-char SHA>` (`<Commits created: 0 or authorized SHA>`)
- **`git diff --check`**: `<0 errors>`
- **Final `git status --short --untracked-files=all`**:
  ```text
  <exact post-session git status output>
  ```

---

## 13. Task Completion Status & Suggested Next Task (`PROPOSED / UNRATIFIED`)

- **Current Task (`<TASK-ID>`) Status**: `<COMPLETED / AWAITING_OWNER_REVIEW / BLOCKED>`
- **Suggested Next Task**: `<Next Task ID and title from NEXT_TASK.md>` (`PROPOSED — NOT_AUTHORIZED`; requires new session & explicit Product Owner dispatch; not executed in this session).
```

---

## 2. Companion Machine-Readable Result Schema (`SESSION_RESULT.json`)

Every session must also write `artifacts/local/session-ops/<TASK-ID>/SESSION_RESULT.json` with the following fields:

```json
{
  "task_id": "<TASK-ID>",
  "task_title": "<Task Title>",
  "session_started_at": "<ISO-8601 timestamp>",
  "session_completed_at": "<ISO-8601 timestamp>",
  "repository": "/Users/mizuhara/workspace/sumi",
  "status": "AWAITING_OWNER_REVIEW",
  "authorization_state": "AUTHORIZED",
  "commit_permission_state": "DENIED",
  "owner_uat_execution_state": "NOT_EXECUTED",
  "owner_accepted": false,
  "artifact_paths": {
    "session_dir": "artifacts/local/session-ops/<TASK-ID>/",
    "isolated_test_db_dir": "artifacts/local/session-ops/<TASK-ID>/test_db/"
  },
  "git_state": {
    "branch": "master",
    "starting_head": "<40-char SHA>",
    "ending_head": "<40-char SHA>",
    "commits_created": 0,
    "git_diff_check_passed": true,
    "pre_existing_modified_files_preserved": [
      "docs/plans/product-completion/DECISION_LOG.md",
      "docs/plans/product-completion/EXECUTION_STATE.md",
      "docs/plans/product-completion/SUMI_V3_BATCH_DEPENDENCY_AND_STATUS_PROPOSED.md"
    ]
  },
  "database_integrity": {
    "path": "backend/sumi.db",
    "size_bytes": 188416,
    "pre_sha256": "234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86",
    "post_sha256": "234206fc75259f091337d4feb5f41575d0fe5ac02d6c10ba0de8fad68cbcee86",
    "unchanged": true
  },
  "files_created": [],
  "files_modified": [],
  "verification_executed_in_session": [],
  "historical_evidence_referenced": [],
  "pending_owner_decisions": [],
  "recommended_next_task": {
    "task_id": "<NEXT-TASK-ID>",
    "lifecycle_status": "PROPOSED",
    "authorization_state": "PROPOSED — NOT_AUTHORIZED",
    "requires_owner_authorization": true
  }
}
```
