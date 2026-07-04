# Sumi Product Completion Plan

Date: 2026-07-04  
Status: Active product roadmap  
Target: A trustworthy local-first trading replay and research product

## 1. Executive Direction

Sumi should not be rewritten as a whole. The current codebase has a working
product skeleton, a usable Lightweight Charts v5 workspace, backend-driven
indicators, replay/backtest/scanner flows and repeatable automated gates. The
correct strategy is to preserve these assets and harden the business core in a
controlled sequence.

The next development cycle is not a feature expansion cycle. Its purpose is to
turn the current release-candidate baseline into a release that can be trusted
for daily-candle training and research. Accounting and analytics correctness
take priority over new indicators, drawing tools or scanner features.

## 2. Current Product Baseline

### Done and verified

- Lightweight Charts v5.2 with the official pane API.
- Price overlays: MA, EMA, Ichimoku, Bollinger Bands and PSAR.
- Separate Volume, RSI, MACD and CCI panes.
- Backend indicator results remain the source of truth.
- Indicator and drawing workspace persistence.
- Replay, Backtest, Strategy Lab, Scanner to Replay and Analytics browser smoke.
- Deterministic demo data for `FPT`, `SSI`, `VCI` and `VNINDEX`.
- Shared fee/tax constants and canonical execution amount calculations.
- Insufficient-cash, invalid-close, T+2, partial-close and force-liquidation
  regression coverage.
- Inclusive date-only ranges across replay, backtest, scanner and benchmark
  queries.
- Latest recorded gates: backend `55 passed, 1 skipped`; frontend lint, 18
  tests and production build passed; product browser smoke passed.

### Working but not release-proven

- Analytics renders, but exact equity, drawdown, benchmark and aggregate
  metrics still need reconciliation against a hand-calculated ledger fixture.
- `TradeLifecycleService` is the product broker path, while
  `BrokerSimulation` remains a second engine abstraction. Shared rules reduce
  drift, but the ownership boundary is not yet final.
- Browser smoke is runnable, but is not yet an enforced cross-platform release
  gate in CI.
- SQLite is suitable for the local-first scope, but concurrent write workloads
  must remain serialized or explicitly coordinated.
- The current implementation is still an uncommitted working tree and therefore
  is not yet a reproducible release artifact.

### Explicitly outside the current release

- Multi-user authentication and authorization.
- Cloud/SaaS deployment and tenant isolation.
- Realtime or intraday market data.
- Broker integration or use with real money.
- Guaranteed multi-symbol portfolio accounting in one replay session.
- Production dependency on community drawing/indicator repositories.

## 3. Definition of Product Complete

Sumi V2 local-first is product complete only when all conditions below are met:

1. A clean checkout can be installed, migrated, seeded and started from the
   documented commands.
2. Backend tests, frontend lint/tests/build and browser smoke all pass from that
   checkout.
3. Replay supports a complete buy-to-close lifecycle with correct cash, fees,
   tax, T+2 settlement, position quantity and persisted drawings/indicators.
4. Backtest and Strategy Lab produce deterministic results for bundled
   strategies without SQLite lock failures.
5. Scanner results can open a replay at the expected symbol/date without future
   data leakage.
6. Analytics totals reconcile exactly with a known trade ledger and benchmark
   fixture.
7. All P0/P1 defects are closed; accepted P2 limitations are documented.
8. The release is represented by a reviewed commit/tag and a completed release
   evidence record.

## 4. Development Roadmap

### Milestone 0 - Freeze the Release Candidate Baseline

Purpose: establish one reproducible truth before further changes.

Work:

- Review the current diff and separate source changes from local artifacts.
- Run the full backend/frontend/browser gate once more.
- Update handoff, roadmap, release checklist and setup instructions.
- Commit the reviewed baseline on the chart-v5 spike branch and open a review
  pull request; do not merge directly to `master`.

Acceptance:

- Clean worktree after commit.
- Reviewable diff with no research repositories or generated artifacts.
- Gate evidence records command, date and result.

Estimate: 0.5-1 engineering day.  
Priority: P0.  
Status: documentation updated; commit/review artifact pending.

### Milestone 1 - Accounting and Trade Ledger Hardening

Purpose: make every product path agree on cash, position and realized PnL.

Work:

- Keep `app/domain/accounting.py` as the canonical pure calculation layer.
- Define explicit contracts for order validation, execution amounts, settlement
  policy, position mutation and trade closure.
- Add table-driven tests for market/limit orders, partial fills or explicitly
  reject unsupported partial fills, multiple round trips, rejected orders,
  force liquidation and fee/tax rounding.
- Assert accounting invariants after every execution:
  - cash never becomes negative in the supported long-only model;
  - position quantity never becomes negative;
  - closed-trade quantity equals linked sell executions;
  - realized PnL equals proceeds minus cost basis and all charges.
- Document `TradeLifecycleService` as the canonical product path. Keep
  `BrokerSimulation` experimental until it implements the same contract; do not
  attempt a large merge without tests proving the need.

Acceptance:

- A hand-calculated ledger fixture matches cash, position, fees, tax and PnL at
  every step.
- Replay and backtest use the same accounting functions.
- No duplicate fee/tax formula remains in a user-facing path.

Estimate: 3-5 engineering days.  
Priority: P0.  
Status: core calculations and first regression set done; invariant matrix and
known-ledger fixture pending.

### Milestone 2 - Analytics Reconciliation

Purpose: prove that reports describe the ledger accurately, not merely that the
page renders.

Work:

- Build deterministic fixtures for one trade, multiple round trips, loss,
  partial close, no-trade session and force liquidation.
- Hand-calculate and assert equity curve, realized/unrealized PnL, max drawdown,
  win rate, profit factor, expectancy, outlier contribution and grouped slices.
- Reconcile VNINDEX benchmark timestamps and base normalization against the
  inclusive session range.
- Define behavior for zero denominators, missing benchmark candles and sessions
  with no closed trades.
- Add frontend contract tests for populated and empty analytics states.

Acceptance:

- API output exactly matches fixture expectations.
- Analytics browser UAT uses a session with closed trades, not only an empty
  state.
- Any metric whose definition is ambiguous is documented in the product spec.

Estimate: 3-4 engineering days.  
Priority: P0.  
Status: pending.

### Milestone 3 - Data Integrity and Determinism

Purpose: ensure the same input always produces the same visible market state
and research result.

Work:

- Add no-future-leak contract tests across candles, indicators, scanner signals
  and replay-launched sessions.
- Validate candle uniqueness, ordering, timezone/date normalization and missing
  benchmark behavior during import.
- Make seed/import idempotency explicit and test repeated execution.
- Lock supported daily-candle assumptions in API validation and documentation.
- Add golden tests for bundled strategies on deterministic seed data.

Acceptance:

- Re-running seed and bundled backtests yields identical results.
- No response includes candles or indicator points after the replay cursor.
- Invalid or duplicate market data produces actionable errors.

Estimate: 2-4 engineering days.  
Priority: P1.  
Status: partial; deterministic seed and several date/no-leak protections exist.

### Milestone 4 - User Workflow and UX Completion

Purpose: make the product understandable and recoverable for a normal user.

Work:

- Test desktop and narrow viewport flows with the in-app browser.
- Standardize loading, empty, success and failure states on Replay, Backtest,
  Strategy Lab, Scanner, Analytics, Journal and Import.
- Show actionable validation for insufficient cash, T+2 rejection, missing
  data, failed strategy and import errors.
- Verify keyboard/mouse chart interactions, pane resizing, drawing restore and
  long-label layout.
- Add confirmation or clear state transitions for destructive/session-ending
  actions.
- Produce a concise operator/user guide based on actual UI labels.

Acceptance:

- A new user can seed data and complete replay, backtest, scanner and analytics
  workflows without developer intervention.
- No console errors, clipped controls or silent actions in supported viewports.
- UAT evidence includes screenshots and issue disposition.

Estimate: 3-5 engineering days.  
Priority: P1.  
Status: happy-path browser smoke passed; systematic UX/error-state review pending.

### Milestone 5 - Release Engineering and Operations

Purpose: make quality repeatable outside the current developer machine.

Work:

- Provide a Unix verification script equivalent to `verify-v2.ps1` or use a
  portable task runner.
- Make browser smoke an explicit release gate and run it in CI where supported.
- Verify a fresh SQLite migration, seed and startup sequence.
- Pin and audit runtime dependencies, especially the beta `pandas-ta` package.
- Document backup/restore, database location, log collection and known limits.
- Add version/build information visible to support and handoff reviewers.

Acceptance:

- One documented command sequence validates a clean checkout.
- Release checklist is signed with exact results and known issues.
- A tagged artifact can be reproduced without relying on untracked local state.

Estimate: 2-3 engineering days.  
Priority: P1.  
Status: partial; individual commands and Windows verification exist.

### Milestone 6 - Release Review and Go/No-Go

Purpose: decide from evidence whether V2 can be handed to users.

Work:

- Run full automated and manual UAT on the release candidate.
- Triage defects by P0/P1/P2 and document accepted limitations.
- Review architecture, security boundary and data recovery instructions.
- Produce final release notes, rollback notes and handoff evidence.

Go criteria:

- All Definition of Product Complete items pass.
- No open P0/P1 defect.
- Product owner accepts the local-first/daily-candle boundary.

No-go criteria:

- Ledger or analytics reconciliation differs from expected values.
- Future data is exposed in replay/research results.
- A clean installation cannot reproduce the tested result.

Estimate: 1-2 engineering days.  
Priority: P0 release gate.  
Status: pending.

## 5. Recommended Sequence and Capacity

Execute milestones in order: `M0 -> M1 -> M2 -> M3 -> M4 -> M5 -> M6`.
M1 and M2 must not be bypassed. Small parts of M3 and M5 may run in parallel
after the baseline is frozen. With one experienced full-stack engineer, the
remaining release-hardening effort is approximately 14-24 engineering days,
excluding defects discovered during reconciliation and product-owner feedback.

No new broad feature should enter this cycle. A requested feature must either
fix a release acceptance failure or be deferred to post-V2 backlog.

## 6. Post-V2 Roadmap

Only after V2 passes the release gate:

1. Decide whether to unify `BrokerSimulation` behind the canonical broker
   contract or remove it.
2. Add richer journal taxonomy and learning analytics.
3. Add strategy versioning, robust parameter comparison and export/reporting.
4. Evaluate community drawing libraries through `SumiDrawingAdapter` on a
   separate spike branch.
5. Design PostgreSQL, job execution and auth only if multi-user deployment is a
   confirmed product requirement.

## 7. Rewrite Decision Rule

Do not rewrite the whole project. Replace a subsystem only when all are true:

- its behavior is protected by acceptance tests;
- the current implementation repeatedly fails those tests or blocks change;
- replacement scope and migration path are bounded;
- retained APIs/data are documented;
- replacement cost is lower than incremental repair based on measured evidence.

The likely candidates for bounded replacement, if evidence requires it, are the
broker/ledger internals or the backtest execution orchestration. Chart/replay UI
is no longer a rewrite candidate after the Lightweight Charts v5 migration.
