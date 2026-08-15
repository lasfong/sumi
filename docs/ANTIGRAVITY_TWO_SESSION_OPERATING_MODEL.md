# Sumi two-session Antigravity operating model

## Purpose

This is the durable operating model for running Sumi with two short-context Antigravity sessions: one **DEV** session and one **Independent REVIEW** session.  It is designed so that a fast, lower-reasoning model can complete the full PRO roadmap without depending on a prior chat, a human copying logs, or a model declaring its own work correct.

This document supplements, and does not relax, `AGENTS.md`, `docs/LOW_MODEL_AUTONOMOUS_EXECUTION_PROTOCOL.md`, the product acceptance criteria, architecture decisions, and `PLANS.md`.  Where they conflict, the stricter rule applies.

## Non-negotiable operating rules

1. The workspace is the sole handoff channel.  Chat messages are notifications, never authority.
2. One bounded PRO batch/rework is active at a time.  Only the batch named **Active batch** in `docs/AUTONOMOUS_EXECUTION_STATE.md` may be changed.
3. DEV owns implementation.  REVIEW owns the independent verdict.  A DEV session can never approve its own work.
4. Both sessions may exist at once, but they must not write concurrently to the shared checkout.  REVIEW starts only after DEV reports the Reviewer Gate; DEV starts again only after a recorded `REWORK` verdict.
5. `APPROVE` closes only the active batch.  It never authorizes the next PRO, commit, push, release, dependency, migration, or external action.
6. Any user-visible Replay/Chart/Indicator/Drawing change requires green isolated browser UAT, reviewed 1440×1000 and 1280×800 screenshots, and retained machine-readable evidence.
7. `backend/sumi.db` is production/user data: automated verification must use a temporary database and its before/after hash must match exactly.

## Durable control package

Every session reconstructs context in this order.  It must not continue if these records disagree; it records the conflict in the ledger and asks the user only when the conflict cannot be resolved from repository evidence.

1. `AGENTS.md`
2. `docs/INDEX.md`
3. this operating model
4. `docs/AUTONOMOUS_EXECUTION_STATE.md` — current authority and exact next action
5. `docs/MACHINE_TRANSFER_HANDOFF_2026-08-10.md` — bootstrap/machine continuity
6. `docs/LOW_MODEL_AUTONOMOUS_EXECUTION_PROTOCOL.md` and `docs/SESSION_HANDOFF_PROTOCOL.md`
7. `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`
8. the active dossier, ExecPlan, DEV prompt, and latest reviewer record named by the ledger

The control files have distinct jobs:

| File | Owner / change authority | Required content |
| --- | --- | --- |
| `AUTONOMOUS_EXECUTION_STATE.md` | DEV updates milestones/evidence; REVIEW updates verdict/next action | one active batch, state, exact prompt, latest review, evidence paths/hashes, blocker |
| `docs/program/PRO_*.md` | reviewer/orchestrator preparation only | stable product outcome, acceptance map, scope boundaries |
| `docs/exec-plans/PRO_*.md` | DEV progress; REVIEW verifies completeness | scope, modules, rollback, decisions, commands, evidence, deviations, checklist |
| `docs/dev-prompts/PRO_*.md` | REVIEW creates a rework prompt; prepared prompt is immutable during DEV except to correct a documented ambiguity | exact bounded DEV authority |
| `docs/reviews/PRO_*_R*.md` | REVIEW only | independent evidence, verdict, severity, disposition |
| `test-results/product-uat/<run>/` | DEV produces; REVIEW audits | result JSON, manifest, screenshots, logs, cleanup evidence |

## State machine and allowed writers

```text
PREPARED / USER-AUTHORIZED
  -> DEV IN PROGRESS
  -> IMPLEMENTED — REVIEW PENDING
  -> REVIEW: APPROVE -> CLOSED (next PRO still unauthorized)
  -> REVIEW: REWORK -> DEV REWORK-N IN PROGRESS
```

| State | DEV may edit code? | REVIEW may edit code? | Required transition evidence |
| --- | ---: | ---: | --- |
| `PREPARED` | only after user starts DEV | no | active dossier, ExecPlan, DEV prompt, acceptance/scope/rollback map |
| `IN PROGRESS` | yes, active batch only | no | focused tests and ledger milestones |
| `IMPLEMENTED — REVIEW PENDING` | no | no | DEV DoD complete, retained evidence, exact handoff report |
| `REVIEW` | no | no implementation edits | diff/evidence audit and an `APPROVE` or `REWORK` record |
| `REWORK-N` | yes, named finding only | no | regression test and all required re-verification |
| `CLOSED` | no | no | review record says `APPROVE`; next PRO remains prepared/inactive |

REVIEW may write its reviewer record, the next narrowly scoped DEV rework prompt, the relevant ExecPlan review disposition, and the state ledger.  It does not alter implementation, tests, acceptance criteria, or evidence to obtain a passing result.

## Efficient short-context workflow

The process deliberately exchanges broad model judgment for explicit, cheap checks:

- Each prompt names one batch and a short ordered reading set.  Do not ask the model to rediscover the roadmap from a large chat transcript.
- Split work by vertical capability, not by arbitrary files.  Every rework must name the defect, failure mode, exact tests, expected artifacts, and stop state.
- Use repository search and targeted file reads first; read entire documents only when they are authority documents.  Avoid repeating scans with unchanged inputs.
- Prefer fail-closed contracts, exact fixture names, deterministic UAT assertions, fixed command blocks, hashes, and screenshots over subjective “looks fine” claims.
- Limit DEV progress reports to start, focused-test milestone, before long gate, and final handoff.  Limit REVIEW reports to verdict and a concise evidence summary.
- A failed command produces one evidence-backed hypothesis and one bounded correction.  It may not be retried blindly or bypassed by weakening tests.
- Every model must write decisions/evidence into the control package before ending its session.  No essential information stays only in a session transcript.

## DEV definition of done: eligibility for review

DEV may write `IMPLEMENTED — REVIEW PENDING` only when every item is true:

### Scope and design

- The active acceptance IDs, in/out-of-scope boundary, affected modules, rollback approach, and no-future-data/DB invariants are present in the ExecPlan.
- The implementation satisfies the acceptance map and does not include a later PRO or unrelated cleanup.
- Backend business logic remains outside FastAPI routes; chart provider calls remain behind adapters; released indicator state remains explicit and serializable.
- The implementation uses no unapproved dependency, provider, license, migration, telemetry, or external data transfer.

### Tests and technical verification

- Focused failing/regression tests prove the repaired behavior and relevant negative cases.  They do not merely assert page presence or finite values.
- Backend and frontend focused suites pass.  `scripts/verify-v2.ps1` passes with lint/type/build results retained.
- For user-visible work, deterministic `scripts/run-product-uat.ps1` passes and the Windows-compatible `scripts/verify-product.sh` result is retained.  If a command is unavailable, record the exact platform limitation and do not claim it passed.
- Product UAT has no missing, unexpected, duplicate, failed, blocking-mismatch, or blocking-failed assertions; runtime/page/console/provider/request errors are classified rather than globally suppressed.
- UAT specifically proves new semantic behavior against authoritative backend output where applicable, at replay boundaries and without future data.

### Evidence and cleanliness

- `results.json`, manifest result, screenshot paths/dimensions/SHA-256, command counts, artifact hashes, and process cleanup are recorded.
- Both required 1440×1000 and 1280×800 screenshots are visually inspected by DEV; values, pane/overlay alignment, labels, settings, and compact layout are usable.
- `backend/sumi.db` hashes match before/after; temporary DB identity is recorded; no production data was seeded/imported/migrated.
- `git diff --check` passes.  The final diff inventory and deviations explain every changed file.  No assertion was weakened, deleted, renamed, duplicated, or made non-blocking to make a gate green.
- ExecPlan and ledger show progress, decisions, rollback, deviations, verification, evidence, and Reviewer checklist completion.

## REVIEW definition of done

REVIEW issues an `APPROVE` only after independently completing all applicable checks.  Tests passing by themselves never satisfy this list.

### Reconstruction and code audit

- Re-read the authority package from disk; confirm the active PRO/rework, acceptance IDs, known prior findings, and no later PRO work.
- Inspect the full active diff and every affected contract boundary.  Trace a representative value from product settings through API, backend authority, returned data, semantic mapping, rendering, persistence, and UAT assertion.
- Read tests for meaningful counterexamples: wrong parameter, alias/alternate output, partial/missing multi-series data, warm-up/null/gap, replay boundary, stale/persisted state, and previously reported failures.
- Check that production labels/panes are not coupled to dataframe names and that a UI failure cannot silently show a different calculation.

### Independent evidence audit

- Rerun proportionate focused tests and at least the fast technical gate.  Rerun full product verification for user-visible risk or when retained evidence is incomplete/stale.  Record exact commands and results.
- Inspect `results.json`, manifest reconciliation, console/page/runtime outcomes, request scope, DB hashes, cleanup, `git diff --check`, and retained screenshots at both required sizes.
- Treat a UAT that only proves finite/non-blank values as insufficient when the contract requires semantic/backend parity.
- Verify DB before/after SHA-256 exact equality and no evidence was generated through `backend/sumi.db`.

### Verdict records

- `APPROVE`: write a dated review record listing independent evidence, remaining known non-blockers (if any), closure disposition, and explicitly state that later PROs remain unauthorized.
- `REWORK`: write a dated review record with severity (`P0` data/security/future leak; `P1` released behavior/acceptance failure; `P2` material quality/maintainability; `P3` minor), precise cause, exact repair constraints, mandatory regression/UAT evidence, and a new DEV rework prompt.  Update the ledger and ExecPlan status.
- REVIEW must never modify application/test code to make a verdict pass.  It leaves the workspace at the next session's clear control point.

## Standard handoff messages

The user starts sessions with the generic prompts below; those prompts discover the active PRO from the ledger.  The developer need not paste an implementation report into REVIEW.

**Start DEV:**

```text
Execute docs/dev-prompts/ANTIGRAVITY_DEV_SESSION_INIT_PROMPT.md exactly from the repository root. Work only on the active batch discovered from the workspace. Do not rely on chat history.
```

**Start REVIEW after DEV stops:**

```text
Execute docs/reviewer-prompts/ANTIGRAVITY_REVIEW_SESSION_INIT_PROMPT.md exactly from the repository root. Audit the active Reviewer Gate from the workspace. Do not rely on chat history.
```

**DEV final handoff:**

```text
Execution stops at the Independent Reviewer Gate. The codebase, documentation, and evidence artifacts are ready for R<N> Independent Reviewer audit. <NEXT-PRO> remains unauthorized.
```

**REVIEW final handoff:**

```text
Verdict recorded in <review-file>: APPROVE or REWORK. The ledger and the next exact session action have been updated; no later PRO is authorized.
```

## Recovery rules

- If the workspace is dirty, preserve unrelated files and compare the active diff against the ledger's expected inventory before editing.
- If DEV reaches a genuine authority, security, license, destructive-data, or cross-batch blocker, it records one concise blocker and exact next action, then stops.
- If a reviewer cannot reproduce DEV evidence, verdict is `REWORK` unless the record proves a harmless environment-only variance and all required independent checks remain valid.
- A fresh session always starts from the ledger and current files, never from assumptions about another session's final prose.
