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

## Phase Progress

| Roadmap phase | Status | Notes |
| --- | --- | --- |
| Phase 0 - Stabilize QA blockers | Mostly complete | Current gates pass: backend pytest, frontend lint/test/build, Alembic upgrade. |
| Phase 1 - Manual Replay MVP Lock | Mostly complete | Core replay, no-future-leak tests, order lifecycle, drawings, scanner-launched sessions and resume UX exist. Needs final smoke/UAT review. |
| Phase 2 - Analytics Professional Baseline | Partial | Core metrics, benchmark/outlier concepts exist. Needs UI polish and deterministic acceptance review. |
| Phase 3 - Indicator And Signal Registry | Mostly complete | Registry includes SMA/EMA/MACD/RSI/BB/ATR/ADX/Ichimoku/Stochastic and additional indicators. Signal taxonomy can still be formalized. |
| Phase 4 - Backtest MVP Safe Runner | Mostly complete | Safe declarative rules and multi-symbol runs exist. Needs persisted backtest runs/export if required for product polish. |
| Phase 5 - Regime Classifier And Research Slices | Partial | Regime classifier and slices exist. Sector strength remains future unless sector data is complete. |
| Phase 6 - Scanner V1 | In progress | Historical scanner and replay link exist. Needs saved scan result history and richer filters. |
| Phase 7 - Strategy Lab Professional V1 | Mostly complete | Comparison, sweep, and persisted run history exist. Needs export/UX polish. |
| Phase 8 - Optional Infrastructure | Not started | Defer unless sync SQLite workflow becomes too slow. |

## Remaining Large Batches

Estimated remaining after the current batch: 4 large batches.

1. Scanner V1 completion: saved scan results, scan history, richer filters by signal/regime/setup, replay links from saved rows.
2. Analytics acceptance polish: setup/symbol/regime views, outlier contribution UI, drawdown period table, benchmark review.
3. Backtest/Strategy Lab product polish: saved backtest runs if separate from Strategy Lab, export/comparison ergonomics, sample MACD+RSI strategy pack.
4. Release hardening: final manual replay smoke, sample data flow, end-to-end smoke script, docs handoff, final UAT checklist and clean git release point.

## Current Gate

Every batch should end with:

- Backend pytest.
- Alembic upgrade head when schema changes.
- Frontend lint.
- Frontend tests.
- Frontend build.
- Commit and push with clean workspace.
