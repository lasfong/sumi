# Sumi low-model autonomous execution protocol

## Purpose

This protocol lets a lower-cost implementation model execute one approved Sumi batch without relying on chat history or repeatedly asking the user what to do next. Repository files and retained evidence are the only durable source of truth.

It does not relax `AGENTS.md`, acceptance criteria, architecture decisions, independent Reviewer gates, or the prohibition on starting a later batch before the current batch is approved.

## Roles

### Implementation session

- Own exactly one batch and one ExecPlan.
- Read the complete authority set before editing.
- Implement continuously through focused tests, full technical gates, browser UAT, evidence retention, self-review, and ExecPlan completion.
- Make reasonable in-scope decisions from repository evidence without asking routine questions.
- Never self-approve, commit, push, release, add dependencies, perform migrations, or start the next batch.
- May report only `IMPLEMENTED — REVIEW PENDING` at the Reviewer gate. The words `DONE`, `COMPLETE`, `VERIFIED`, and `APPROVED` are reserved for an explicit Independent Reviewer verdict when they describe the batch as a whole.

### Independent Reviewer session

- Is read-only until it returns findings.
- Reconstructs context from the prompt, ExecPlan, current-state ledger, complete diff, tests, result JSON, screenshots, hashes, and cleanup evidence.
- Returns `APPROVE` or `REWORK` with prioritized findings.
- Does not accept “tests passed” as a substitute for contract or visual review.

## Single entrypoint rule

The user should need to give a new implementation session only one instruction:

```text
Execute docs/dev-prompts/PRO_02_DAILY_TRADER_WORKFLOW_LOW_MODEL_PROMPT.md exactly. Work continuously until a genuine blocker or the Independent Reviewer gate. Do not rely on chat history.
```

The implementation session must discover everything else from files named by that prompt. The user must not be asked to copy intermediate reports between sessions.

## Communication policy

Send at most these progress messages:

1. One start message after reading authority and confirming provenance.
2. One milestone message after focused implementation tests are green.
3. One message before the first long standalone/full product gate.
4. One final handoff at a genuine blocker or Reviewer gate.

Do not send per-file updates, narrate ordinary searches, ask for confirmation on reversible in-scope work, or repeatedly report unchanged long-running state. Long commands must be bounded. If a command fails, retain evidence, form a concrete corrective hypothesis, and do not repeat it blindly.

## Mandatory durable state

Every batch must keep these repository-controlled records current:

- standalone authority under `docs/dev-prompts/`;
- living ExecPlan under `docs/exec-plans/`;
- `docs/AUTONOMOUS_EXECUTION_STATE.md` with current milestone, last green commands, active blocker, and exact next action;
- machine-readable UAT result or partial result;
- required screenshots and their dimensions/SHA-256;
- production DB before/after SHA-256 and temporary DB identity;
- final diff inventory, decision/deviation log, cleanup evidence, and Reviewer checklist.

Update the state ledger after each milestone before proceeding. Do not place essential context only in chat.

## Stop conditions

Stop only when one of these is true:

- a destructive migration or data-loss risk appears;
- a dependency/provider/license/security decision is required;
- acceptance requirements conflict or would need weakening;
- the fix requires material work outside the active batch;
- `backend/sumi.db` changes;
- unexpected local changes overlap active files and cannot be preserved;
- the same blocker persists after one evidence-backed correction and a second bounded verification;
- implementation and all DEV gates are complete, so independent review is required.

When stopping, write the blocker and exact next action into `docs/AUTONOMOUS_EXECUTION_STATE.md`; give the user one concise action, not several ambiguous choices.

## Definition of done for an implementation batch

This heading describes eligibility to enter review, not product approval. Passing every DEV gate produces `IMPLEMENTED — REVIEW PENDING`; only the Independent Reviewer can produce `APPROVE`.

A batch reaches the Reviewer gate only when all are true:

- every assigned acceptance ID has implementation and evidence mapping;
- focused unit/integration tests pass, including required edge fixtures;
- full backend/frontend suites, lint, type/build, and `verify-v2` pass;
- standalone product UAT and `verify-product` pass when the batch affects user-visible behavior;
- manifest reconciliation has zero missing, unexpected, duplicate, blocking-mismatch, failed, and blocking-failed assertions;
- required screenshots have been visually reviewed at 1440×1000 and 1280×800;
- console/page/runtime/provider/request outcomes are classified and retained;
- production DB before/after hashes match exactly;
- temporary DB and owned listeners/processes are cleaned up;
- `git diff --check` passes and the diff contains no later-batch work;
- no accepted assertion was removed, renamed, weakened, duplicated, or made non-blocking;
- no runtime/page/console/provider/request error was hidden by a global ignore or suppression; expected failures are narrowly scoped and retained;
- ExecPlan progress, decisions, deviations, rollback, verification, completion evidence, and Reviewer checklist are complete.

## Rework loop

Reviewer findings return to the same implementation session or a fresh session reading the same durable files. DEV corrects only the findings, adds regression evidence, reruns proportionate gates, updates the ExecPlan/state ledger, and returns to Reviewer. The user is not required to relay technical context manually.
