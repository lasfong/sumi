# Sumi Session Handoff Protocol

This file defines the durable boundary between DEV (including Antigravity) and the Independent Reviewer (Codex).

## Single source of truth

Every session must read, in order:

1. `AGENTS.md`
2. `docs/MACHINE_TRANSFER_HANDOFF_2026-08-10.md` when the workspace has moved machines
3. `docs/AUTONOMOUS_EXECUTION_STATE.md`
4. The active or prepared-next batch ExecPlan under `docs/exec-plans/`
5. The active DEV or Reviewer prompt named by the state file
6. The newest review record under `docs/reviews/`

The workspace, not chat transcripts, is the handoff channel.

## DEV contract

DEV works on one batch only. Before coding, record scope, acceptance IDs, rollback, and commands in the ExecPlan. After coding, run the required gates and write exact results, artifact paths, hashes, and cleanup evidence. DEV may claim only `IMPLEMENTED — REVIEW PENDING`. DEV must not approve itself, start the next batch, commit without authorization, push, or weaken assertions.

## Reviewer contract

Reviewer independently inspects the diff and evidence and reruns the relevant gates. Reviewer writes a dated record under `docs/reviews/` with one verdict: `APPROVE` or `REWORK`. `APPROVE` requires every Definition-of-Done item to be green. On `REWORK`, list only reproducible blockers and stop.

## User interaction contract

The user should start or resume a session with one sentence: `Read the repository handoff files and continue the active batch exactly from the recorded control point.` Routine progress reports are written to the ExecPlan/state file. The user only relays milestone completion, blocker, or authorization for commit/push/release/destructive/scope-changing actions.

When no batch is active, use the exact next-session prompt recorded by `docs/AUTONOMOUS_EXECUTION_STATE.md` or the machine-transfer handoff. A prepared plan is not active until the user invokes its standalone DEV prompt.

For a machine move, source code, untracked files, ignored local data, and ignored reviewer evidence are separate transfer layers. A fresh clone is insufficient whenever the source workspace is dirty. Verify the recorded commit, dirty inventory, database hash, and reviewer-result hash before continuing.

## Batch boundary

No session crosses from DEV to Reviewer or from Reviewer approval to the next PRO batch implicitly. A new session is safe when the prior session has written its handoff record and left the working tree/evidence inspectable.
