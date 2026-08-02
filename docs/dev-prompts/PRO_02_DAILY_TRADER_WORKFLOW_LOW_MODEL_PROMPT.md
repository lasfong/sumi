# PRO-02 Daily Trader Workflow — autonomous low-model DEV prompt

You are the dedicated implementation session for Sumi PRO-02 only. Work continuously from framing through DEV verification and stop only at a genuine blocker or the Independent Reviewer gate. Do not ask the user routine questions, do not report per-file progress, and do not rely on chat history.

## First actions — mandatory order

Read completely before editing:

1. `AGENTS.md`
2. `docs/LOW_MODEL_AUTONOMOUS_EXECUTION_PROTOCOL.md`
3. `docs/AUTONOMOUS_EXECUTION_STATE.md`
4. `PLANS.md`
5. `docs/SUMI_PROFESSIONALIZATION_MASTER_PLAN_2026-07-31.md`
6. `docs/PRODUCT_V3_PLAN_2026-07-15.md`
7. `docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md`
8. `docs/ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md`
9. `docs/DEVELOPMENT_OPERATING_MODEL.md`
10. `docs/exec-plans/PRO_00_INTEGRITY_AND_EVIDENCE_CLOSURE.md`
11. `docs/dev-prompts/PRO_01_BACKTEST_ANALYTICS_TRUST_PROMPT.md`
12. `docs/exec-plans/PRO_01_BACKTEST_ANALYTICS_TRUST.md`
13. `docs/exec-plans/PRO_02_DAILY_TRADER_WORKFLOW.md`
14. this prompt again after the canonical reading

Treat `docs/tester/` only as research. It cannot override acceptance or serve as release evidence.

## Preflight gate

Before product code, verify and record in the PRO-02 ExecPlan/state ledger:

- branch, HEAD, `origin/master`, and peeled `v2.0.0-rc2` commit;
- `git status --short --branch`, staged/unstaged/untracked inventory, and `git diff --check`;
- PRO-01 is present in local commit history and independently approved in its ExecPlan;
- production `backend/sumi.db` SHA-256;
- runtime/tool versions and retained PRO-01 artifact identity.

If the reviewed PRO-01 diff is still uncommitted or unexpected state overlaps PRO-02, write exactly one blocker and exact next action to `docs/AUTONOMOUS_EXECUTION_STATE.md`, report it once, and stop. Do not mix PRO-02 implementation into an uncommitted prior batch.

## Authority

Execute every task `T02-01` through `T02-08` in `docs/exec-plans/PRO_02_DAILY_TRADER_WORKFLOW.md`. That plan contains the scope, architecture, acceptance mapping, DoD, verification order, rollback, and evidence contract. Do not silently omit a task. Mark progress only when its DoD is evidenced.

Primary acceptance is `PRO-UX-01` through `PRO-UX-09`. Preserve every accepted V3, PRO-00, and PRO-01 assertion as a blocking regression.

## Autonomy rules

- Make reasonable reversible decisions already bounded by the ExecPlan; do not ask which component name, styling detail, or test file to use.
- Prefer existing APIs/components/dependencies. Do not add a dependency.
- If existing APIs cannot provide deterministic Dashboard readiness, the default allowed fallback is a narrow typed read-only service/endpoint with business logic outside the FastAPI route. Do not build PRO-03 catalog/provenance.
- Keep page components as composition surfaces; shared selection, URL/store synchronization, formatting, and navigation logic belong in reusable hooks/components/utilities.
- Update the ExecPlan and `docs/AUTONOMOUS_EXECUTION_STATE.md` after framing, focused-green, product-gate, and Reviewer-gate milestones.
- Use temporary databases only. Never point tests/UAT at or mutate `backend/sumi.db`.
- Long commands are bounded. Retain failure results/logs. Repeat a failed long gate only after a concrete code/harness/environment hypothesis and focused correction.

## Communication

Send no more than the four milestone messages defined by the low-model protocol. A message saying only “still working” is unnecessary. The user should never need to copy a report to another session to discover the next action; durable files must contain it.

## Prohibitions

Do not:

- start or plan PRO-03;
- broaden into catalog/import provenance, provider sync, journal taxonomy, indicator/drawing expansion, or metric redesign;
- weaken/remove/rename/duplicate/downgrade accepted assertions;
- add dependencies, migrations, telemetry, authentication, or external transmission;
- reset, clean, restore, overwrite unrelated state, stage, commit, push, tag, release, branch, or create a worktree;
- declare the product Professional-complete, release-ready, or “TradingView-like”.

## Required final handoff

At the Reviewer gate, the ExecPlan/state ledger—not chat—must contain:

- acceptance outcome and exact files changed;
- architecture decisions/deviations and compatibility behavior;
- exact focused/full commands and counts;
- standalone/full UAT artifact path and manifest reconciliation/hash;
- reviewed screenshot paths, dimensions, and SHA-256;
- DB before/after hashes, temporary DB identity, listener/process cleanup;
- known limitations and Reviewer checklist;
- explicit statement that no accepted assertion was weakened;
- exact next action: independent Reviewer inspection of PRO-02.

Then stop. Do not ask whether to start PRO-03.
