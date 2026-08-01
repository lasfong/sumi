# PRO-00 verification continuation — standalone prompt

You are the dedicated verification continuation task for Sumi PRO-00 only. Begin from the current `origin/master` checkout containing implementation commit `55ec5f9` or a documented later descendant. You have no authority to begin PRO-01.

## Required reading

Read completely before acting:

1. `AGENTS.md`
2. every canonical source required by `AGENTS.md`
3. `PLANS.md`
4. `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`
5. `docs/dev-prompts/PRO_00_INTEGRITY_AND_EVIDENCE_CLOSURE_PROMPT.md`
6. `docs/exec-plans/PRO_00_INTEGRITY_AND_EVIDENCE_CLOSURE.md`
7. `docs/PROFESSIONALIZATION_HANDOFF_2026-08-01.md`

Treat `docs/tester/` only as research. It cannot override canonical acceptance or serve as release evidence.

## Authority and scope

PRO-00 implementation is frozen. This task may:

- audit checkout provenance and the complete PRO-00 diff;
- run focused and canonical verification with bounded execution time;
- retain machine-readable results, logs, screenshots, hashes, and failure diagnostics;
- update only the PRO-00 ExecPlan and evidence documentation;
- make a product-code correction only when a reproduced verification failure proves an in-scope PRO-00 defect.

This task must not:

- start or plan implementation of PRO-01;
- broaden Scanner, Replay, WebSocket, persistence, analytics, or backtest scope;
- weaken, remove, rename, duplicate, or downgrade an accepted assertion;
- mutate `backend/sumi.db` or use it for automated verification;
- reset, clean, restore, overwrite, stage, commit, or push without explicit user authority;
- make release, Professional-complete, product-complete, or “TradingView-like” claims.

## Initial audit

Record in the ExecPlan:

- branch, HEAD, `origin/master`, and `v2.0.0-rc2` identities;
- `git status --short --branch`, staged/unstaged/untracked inventory, and `git diff --check`;
- production database SHA-256 before verification;
- available retained PRO-00 result bundles and screenshots;
- exact runtime/tool versions needed to explain machine-specific behavior.

Do not discard unexpected local state. Stop and report if it overlaps the verification scope and cannot be safely preserved.

## Verification sequence

Use bounded commands and report progress between long gates. Do not let a command remain active indefinitely.

1. Run the focused backend, frontend, and manifest tests recorded in the PRO-00 ExecPlan.
2. Run the full backend suite, frontend suite, lint, and build.
3. Run `scripts/verify-v2.sh`.
4. Run the standalone product UAT through the platform-appropriate checked-in wrapper.
5. Run `scripts/verify-product.sh` only after the standalone runner has proved clean startup and cleanup on this machine.
6. Recompute the production database SHA-256 and require an exact before/after match.
7. Inspect listeners/processes and prove the UAT left no owned backend, frontend, or browser processes running.

If a runner fails or stalls:

- retain its partial result and logs;
- identify whether the cause is product, harness, environment, or platform compatibility;
- stop only the exact process tree created by the run;
- do not repeat the same long command without a concrete corrective hypothesis;
- update the ExecPlan and report the blocker when continuation would be speculative.

## Evidence requirements

Require and record:

- manifest path, SHA-256, declared count, actual count, and exact ID reconciliation;
- passed, failed, blocking-failed, missing, unexpected, duplicate, and blocking-mismatch counts;
- API/request/WebSocket/runtime error outcomes;
- temporary database identity and production database hashes;
- screenshots for blind-practice boundary at 1440×1000 and signal-review at 1280×800;
- visual inspection results, not merely screenshot existence;
- exact commands, exit codes, durations when available, and artifact paths.

## Completion and stop boundary

Update the PRO-00 ExecPlan completion evidence and deviations. Review the diff against PRO-00 acceptance IDs and explicitly state whether any accepted test was weakened.

Then stop for independent Reviewer inspection. Do not self-approve and do not begin PRO-01.

The final status line, only if every required DEV verification gate is green and no required work remains, is:

```text
PRO-00 DEV COMPLETE — STOPPED AT REVIEWER GATE
```

If any gate remains pending or failed, use an accurate pending/blocked status instead and do not emit the completion line.
