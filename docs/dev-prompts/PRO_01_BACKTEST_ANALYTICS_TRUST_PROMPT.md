# PRO-01 — Backtest and Analytics Trust

You are the dedicated DEV task for Sumi PRO-01 only. PRO-00 was independently approved and committed locally as `24468dd`; do not modify or reopen PRO-00 unless a regression is reproduced. Do not start PRO-02 or any later batch.

## Outcome

Make quantitative output communicate what the available data supports and refuse false precision across Backtest, Analytics, Strategy Lab, and Scanner ranking.

## Required acceptance IDs

- `PRO-BT-01` through `PRO-BT-10`.
- Preserve all released V3 and PRO-00 regression IDs, especially `PRO-INT-01` through `PRO-INT-10`.

## Required contract

- Introduce typed coverage, execution-assumption, metric-validity, and reproducibility/run-manifest structures where the existing architecture requires them.
- Metrics must be nullable and carry validity (`valid`, `insufficient_data`, `not_applicable`) plus a human-readable reason where unavailable.
- SQN requires at least 30 closed trades; Sharpe/Sortino require at least 30 periodic returns, with Sortino also requiring at least two downside observations.
- Win rate and profit factor always show sample size and cannot present insufficient samples as positive evidence.
- Propagate validity consistently through Backtest, Analytics, Strategy Lab, and Scanner ranking; invalid metrics cannot win ranking or recommendation surfaces.
- Preserve backend business logic outside FastAPI routes, declarative strategy evaluation, local-first behavior, and temporary-DB-only tests/UAT.

## Operating constraints

- Read `AGENTS.md`, all canonical V3 sources, `PLANS.md`, the professionalization master plan, this prompt, and inspect the PRO-00 ExecPlan before coding.
- Create/update `docs/exec-plans/PRO_01_BACKTEST_ANALYTICS_TRUST.md` before product code.
- Do not change acceptance criteria, weaken assertions, add dependencies, mutate `backend/sumi.db`, commit, push, or start PRO-02.
- Use temporary databases for tests/UAT and retain exact evidence, screenshots, hashes, and cleanup diagnostics.
- Stop and return to Reviewer if a contract migration, dependency, external provider, destructive data operation, or out-of-scope rewrite is required.

## Verification and handoff

Run focused hand-calculated metric fixtures first, then full backend/frontend/lint/build/`verify-v2`, deterministic product UAT, and `verify-product` as applicable. Review the diff against every acceptance ID, explicitly state whether any accepted assertion was weakened, and stop at the independent Reviewer gate. Do not self-approve and do not declare Professional-complete or release-ready.
