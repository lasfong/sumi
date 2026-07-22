# Sumi V3 development operating model

## Roles

### Reviewer/orchestrator task

- Owns product plan, architecture decisions, acceptance criteria, batch boundaries, and final quality judgement.
- Reviews DEV diffs and evidence; does not concurrently edit the same implementation files.
- Approves dependency/provider decisions and changes to acceptance criteria.
- Keeps the canonical roadmap and issue priorities current.

### DEV task

- Works in a dedicated Codex task but, by default, uses the current checkout and branch.
- Must not create/switch branches, create worktrees, commit, push, or merge unless the user/reviewer explicitly requests it.
- Executes one approved ExecPlan batch at a time.
- May refactor within batch scope but must not broaden product scope silently.
- Produces code, tests, UAT artifacts, updated progress/decision logs, and a concise handoff.

## Batch lifecycle

1. **Frame:** reviewer assigns outcome, acceptance IDs, constraints, and branch base.
2. **Plan:** DEV inspects code and writes/updates the ExecPlan before feature coding.
3. **Implement:** DEV delivers a vertical slice; no unrelated cleanup.
4. **Verify:** fast gate plus deterministic product UAT; retain artifacts.
5. **Self-review:** DEV reviews its own diff against acceptance IDs and records known limits.
6. **Reviewer gate:** orchestrator inspects code, browser evidence, architecture, and regressions.
7. **Accept/rework:** only an accepted batch unlocks the next batch.

## Checkout and task strategy

- Keep this current task as the pinned reviewer/orchestrator context.
- Create a separate DEV task that opens `/Users/mizuhara/workspace/sumi` directly in the current local checkout.
- A separate task isolates context and responsibility; it does not require a separate branch or worktree.
- Only one task may write the checkout at a time. While DEV is implementing a batch, the reviewer task inspects/status-checks but does not edit overlapping files.
- Use a branch/worktree only when the user explicitly requests parallel isolated development or when two independent writers must run concurrently.
- Preserve all existing uncommitted user/review/harness files; do not reset or clean the working tree.

## Required DEV handoff

Every batch handoff must include:

- Outcome and acceptance IDs completed.
- Exact files changed.
- Architectural decisions/deviations.
- Test commands and counts.
- Product UAT artifact path and screenshots.
- Known failures and risks.
- Recommended reviewer checks.
- Explicit statement that no acceptance test was weakened.

## Escalation rules

DEV must stop and return to reviewer when:

- A provider fails a mandatory spike criterion.
- Persistence migration could lose existing sessions/drawings.
- A new dependency changes license/security posture.
- Fixing the batch requires changing a backend contract outside scope.
- Product acceptance criteria are internally inconsistent or infeasible.

## Cadence

Use large, coherent batches—not one prompt per component and not a single uncontrolled rewrite. Each batch should leave the product in a testable state and should normally fit 3–10 working days.
