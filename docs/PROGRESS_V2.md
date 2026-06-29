# Sumi V2 Progress

Status: active execution tracker.
Last updated: 2026-06-29.

## Completed Batches

1. V2 product/spec foundation and repository cleanup.
2. QA foundation: backend tests, frontend lint/test/build, safer backtest rule evaluation, analytics contract alignment.
3. Benchmark and market-regime slices.
4. Historical signal scanner.
5. Strategy Lab comparison.
6. Strategy parameter sweep.
7. Frontend route chunk splitting.
8. Strategy Lab run history UI.
9. Persisted Strategy Lab runs with Alembic migration.
10. Scanner signal to Replay workflow.
11. Replay session signal provenance and Replay signal context.
12. Replay session resume UX: recent sessions, persisted active session, session status, and new-session escape hatch.
13. Scanner V1 persistence: saved scan runs, scan history UI, and reloadable historical signals.
14. Analytics acceptance UI: benchmark summary, drawdown periods, trade distribution, and typed analytics contract.
15. Backtest strategy polish: MACD+RSI sample strategy, RSI/MACD backtest indicators, clean MA sample, and selected-strategy context UI.
16. Release hardening: verification script, release checklist, README refresh, and final automated gate pass.

## Phase Progress

| Roadmap phase | Status | Notes |
| --- | --- | --- |
| Phase 0 - Stabilize QA blockers | Complete | Final gate passes: backend pytest, Alembic upgrade, frontend lint/test/build. |
| Phase 1 - Manual Replay MVP Lock | Complete | Core replay, no-future-leak tests, order lifecycle, drawings, scanner-launched sessions and resume UX exist. |
| Phase 2 - Analytics Professional Baseline | Complete | Core metrics, benchmark, outlier impact, drawdown periods, symbol/mistake/setup views and trade distribution are visible. |
| Phase 3 - Indicator And Signal Registry | Complete for V2 | Registry includes SMA/EMA/MACD/RSI/BB/ATR/ADX/Ichimoku/Stochastic and additional indicators. Advanced signal taxonomy extensions remain future scope. |
| Phase 4 - Backtest MVP Safe Runner | Complete | Safe declarative rules, multi-symbol runs, MA and MACD+RSI samples, and symbol/period/regime slices exist. |
| Phase 5 - Regime Classifier And Research Slices | Complete for V2 | Regime classifier and slices exist. Sector strength remains future unless sector metadata is complete. |
| Phase 6 - Scanner V1 | Complete | Historical scanner, replay links, saved scan runs and scan history UI exist. Advanced filters remain future scope. |
| Phase 7 - Strategy Lab Professional V1 | Complete for V2 | Comparison, sweep, persisted run history, and reusable sample strategies exist. Export can remain future scope. |
| Phase 8 - Optional Infrastructure | Deferred | Defer unless sync SQLite workflow becomes too slow. |

## Remaining Large Batches

Estimated remaining after the current batch: 0 large batches.

Release hardening is complete. Future work should be tracked as post-V2 enhancements.

## Final Verification

Last automated release gate: 2026-06-29.

```text
scripts/verify-v2.ps1
Backend pytest: 44 passed, 1 warning.
Alembic upgrade head: passed.
Frontend lint: passed.
Frontend tests: 5 files / 7 tests passed.
Frontend build: passed.
```

## Current Gate

Every batch should end with:

- Backend pytest.
- Alembic upgrade head when schema changes.
- Frontend lint.
- Frontend tests.
- Frontend build.
- Commit and push with clean workspace.
