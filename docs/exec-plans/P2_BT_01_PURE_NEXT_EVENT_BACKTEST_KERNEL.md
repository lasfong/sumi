# P2-BT-01 — Pure Next-Event Backtest Kernel

## Outcome
Eliminate look-ahead price bias in daily backtesting by enforcing pure next-event causal execution (signal on Close[T] executes at Open[T+1]), ensure deterministic simulation with zero intermediate database writes, separate signal/order/fill timestamps in the ledger, and maintain transactional database persistence for backward compatibility with downstream analytics and API clients.

## Context and problem
In the legacy backtest implementation (`backend/app/services/backtest_service.py`), daily strategy rules are evaluated at the close of bar $T$ using `Close[T]`, and the order is immediately executed at `Close[T]` on the same bar. This introduces forward-looking look-ahead bias (`BT-TIME-001`), as a trader cannot know the daily close price before session end to execute at that exact price. Furthermore, the legacy backtest calls `TradeLifecycleService`, writing directly to the SQLite database on each bar during the loop; any failure leaves partial dirty data in the database.

Addressed:
- Roadmap Task: `P2-BT-01`
- Canonical Specs: `SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md` §Phase 2, `BT-TIME-001`, `BT-TIME-002`, `TEST-BT-001`, `NFR-DET-001`

## In scope
- Pure in-memory domain execution models (`SignalEvent`, `BacktestOrder`, `BacktestExecution`, `BacktestTrade`, `BacktestLedger`, `BacktestKernelResult`).
- Pure causal `BacktestExecutionKernel` running 100% in-memory without database dependencies.
- Next-event execution timing: daily signal at Close[T] fills at Open[T+1].
- Final-bar signal cutoff handling (`UNFILLED_AT_CUTOFF`).
- Fee-aware lot sizing per Vietnam market standards (`100`-share lots, insufficient cash rejection).
- Simultaneous SL/TP exit determinism (Stop Loss precedence).
- Atomic `BacktestPersistenceAdapter` for transactional persistence of completed runs into SQLite.
- Backward compatibility for `/api/backtest/run` and `ExecutionAssumptions`.
- Dedicated unit and integration test suite `test_backtest_kernel.py`.

## Out of scope
- Multi-asset portfolio cash sharing (Phase 4).
- Complex Vietnam holiday calendars and corporate action adjustments (P2-MKT-02).
- Signal library expansions (Phase 3).
- UI redesign or new frontend routes (Phase 9).

## Invariants
- Never leak future candles: signals at bar $T$ must not access bar $T+1$, fills at bar $T+1$ must use bar $T+1$ Open.
- No database pollution: simulation loop must have zero database interactions.
- Do not mutate production `backend/sumi.db`.
- Preserve the 159 historically staged deletions and uncommitted baseline files intact.
- Retain authoritative indicator calculation in backend `IndicatorEngine`.

## Current architecture
- `backend/app/services/backtest_service.py`: runs simulation with direct SQLite writes per bar and same-bar Close fills.
- `backend/app/domain/engine/broker.py`: event broker prototype, unused by service.
- `backend/app/schemas/analytics_trust_schema.py`: default assumptions specify same-bar close execution.

## Target design
- `backend/app/domain/backtest/models.py`: domain dataclasses for pure in-memory execution.
- `backend/app/domain/backtest/execution.py`: pure next-event simulation kernel.
- `backend/app/domain/backtest/persistence_adapter.py`: transactional adapter saving completed session to SQLite in one commit.
- `backend/app/services/backtest_service.py`: orchestrator delegating simulation to `BacktestExecutionKernel` and persistence to `BacktestPersistenceAdapter`.
- `backend/app/schemas/analytics_trust_schema.py`: updated timing defaults.

## Milestones
1. Domain models & Pure Next-Event Execution Kernel implementation with unit test coverage.
2. Transactional persistence adapter & `BacktestService` refactoring with backward-compatibility test coverage.
3. Verification with fast technical gate (`verify-v2.ps1`), state update in `STATE.json`, and reviewer evidence authoring in `docs/reviews/P2_BT_01_REVIEW.md`.

## Acceptance mapping
| Acceptance ID | Implementation evidence | Test/UAT evidence |
| --- | --- | --- |
| BT-TIME-001 | `BacktestExecutionKernel` queues at Close[T], fills at Open[T+1] | `test_backtest_kernel.py::test_signal_close_fills_at_next_open` |
| BT-TIME-002 | `SignalEvent`, `BacktestOrder`, `BacktestExecution` separate timestamps | `test_backtest_kernel.py::test_ledger_timestamp_separation` |
| TEST-BT-001 | Final bar signal cutoff | `test_backtest_kernel.py::test_final_bar_signal_unfilled_cutoff` |
| NFR-DET-001 | Zero DB writes during simulation, single atomic commit on success | `test_backtest_kernel.py::test_zero_db_interaction_during_simulation` |
| UAT-COMPAT | Existing backtest API & trust schema backward compatibility | `test_backtest.py`, `scripts/verify-v2.ps1` |

## Verification commands
```powershell
pytest backend/app/tests/test_backtest_kernel.py -v
pytest backend/app/tests/test_backtest.py -v
powershell .\scripts\verify-v2.ps1
powershell .\scripts\run-comprehensive-uat.ps1
node scripts/verify-dev-program.mjs
```

## Rollback and compatibility
The changes maintain full schema and API compatibility. If rollback is necessary, git checkout of `backend/app/services/backtest_service.py` and `backend/app/schemas/analytics_trust_schema.py` reverts to legacy behavior, and `backend/app/domain/backtest/` can be removed.

## Risks and mitigations
- *Risk*: Discrepancy between legacy backtest numbers and next-event backtest numbers.
  *Mitigation*: Next-event execution is mathematically more conservative and realistic. Timing assumptions are explicitly exposed in `ExecutionAssumptions`.
- *Risk*: UAT assertion failures if assumptions schema strings change.
  *Mitigation*: Retain `price_basis: "OHLC close"` and compatible format expected by `scripts/product-uat.mjs`.

## Progress log
- 2026-09-15: Initial ExecPlan drafted, architecture surveyed, test invariants verified.
- 2026-09-16: Executed comprehensive browser UAT (`.\scripts\run-comprehensive-uat.ps1`). 31/31 tests passed.

## Decision log
- Decision: Use in-memory pure domain execution and an atomic persistence adapter.
  Rationale: Satisfies `NFR-DET-001` and zero-DB pollution while keeping full backward compatibility with `AnalyticsService` and `test_backtest.py`.

## Completion evidence
- Unit/Integration tests: 5/5 kernel tests passed (`test_backtest_kernel.py`), 10/10 backtest tests passed (`test_backtest.py`).
- Fast technical gate: `verify-v2.ps1` passed (227 backend tests, 210 frontend tests, lint, build).
- Comprehensive browser UAT: 31/31 tests passed (100%), report at `test-results/comprehensive-uat/report.json`, screenshots captured at 1440x1000.
- Database invariant: Zero DB mutation on `backend/sumi.db` verified via SHA256 match.
- State verified: `node scripts/verify-dev-program.mjs` exits with code 0.
