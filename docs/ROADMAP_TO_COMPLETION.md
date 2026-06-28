# Roadmap To Completion

Status: Developer execution plan.
Date: 2026-06-28.

This roadmap is intentionally explicit for junior developers. Do not skip phases.

## Phase 0 - Stabilize Current QA Blockers

Goal:

- Make the existing project acceptable for further product development.

Scope:

- Security, PnL correctness, analytics contract, lint, Alembic, config.

Files/modules:

- `backend/app/services/backtest_service.py`
- `backend/app/services/trade_lifecycle_service.py`
- `backend/app/services/analytics_service.py`
- `backend/app/schemas/analytics_schema.py`
- `backend/app/models/execution.py`
- `backend/app/models/order.py`
- `backend/app/models/trade.py`
- `backend/app/main.py`
- `backend/app/config.py`
- `backend/requirements.txt`
- `frontend/src/components/analytics/EquityChart.tsx`
- `frontend/src/types/analytics.ts`

Tasks:

1. Replace `eval()` with safe rule evaluator.
2. Add malicious rule tests.
3. Add `trade_id` linkage or execution-to-trade mapping.
4. Fix multi-round-trip PnL.
5. Standardize equity point schema.
6. Fix frontend `EquityChart` to use the same schema.
7. Make lint pass without disabling rules globally.
8. Add Alembic dependency and verify migration commands.
9. Tighten CORS and config defaults.

Tests:

- Backend pytest full suite.
- Frontend lint/build/test.
- Alembic upgrade on fresh DB.

Acceptance:

- No P0/P1 QA blocker remains.

Risks:

- PnL fix may require schema migration.
- Frontend lint may expose hidden contract issues.

## Phase 1 - Manual Replay MVP Lock

Goal:

- Manual replay is trustworthy for learning.

Scope:

- No-future-leak, indicators, drawing persistence, order lifecycle, journal.

Files/modules:

- `backend/app/services/replay_service.py`
- `backend/app/api/replay.py`
- `backend/app/api/decisions.py`
- `frontend/src/pages/ReplayPage.tsx`
- `frontend/src/components/chart/*`
- `frontend/src/components/replay/*`

Tasks:

1. Add regression tests for candle and indicator no-future-leak.
2. Add pending order panel/status.
3. Add journal fields for setup taxonomy and mistake taxonomy.
4. Ensure previous/rewind behavior is explicit and tested.
5. Add visible session status and error states in UI.

Tests:

- Replay API tests.
- Replay page component tests for basic interaction.

Acceptance:

- A user can complete one full replay session and review accurate results.

## Phase 2 - Analytics Professional Baseline

Goal:

- Analytics answer learning and method-quality questions.

Scope:

- Metrics, slices, benchmark, outlier analysis.

Files/modules:

- `backend/app/services/analytics_service.py`
- `backend/app/schemas/analytics_schema.py`
- `frontend/src/pages/AnalyticsPage.tsx`
- `frontend/src/components/analytics/*`

Tasks:

1. Add performance by setup, symbol, mistake and period.
2. Add R distribution.
3. Add trade distribution histogram.
4. Add top-N outlier contribution.
5. Add VNINDEX benchmark.
6. Add drawdown periods table.

Tests:

- Deterministic metric tests with hand-calculated examples.
- Frontend chart contract tests.

Acceptance:

- User can see where they win/lose and whether performance depends on outliers.

## Phase 3 - Indicator And Signal Registry

Goal:

- Build a scalable technical-analysis foundation.

Scope:

- Indicator registry and signal taxonomy.

Files/modules:

- `backend/app/domain/engine/indicator_engine.py`
- `backend/app/domain/signals/`
- `frontend/src/components/chart/IndicatorSelector.tsx`

Tasks:

1. Define indicator metadata schema.
2. Add MACD, Bollinger Bands, ATR, ADX, Ichimoku, Stochastic.
3. Add signal definitions: cross, threshold, slope, breakout, volume expansion.
4. Use same indicator outputs in replay and backtest.

Tests:

- Indicator computation tests.
- Warmup/no-leak tests.

Acceptance:

- New indicator can be added by registry entry plus compute function, not by scattered UI edits.

## Phase 4 - Backtest MVP Safe Runner

Goal:

- Automated backtest becomes safe and useful.

Scope:

- Declarative strategies, safe rule evaluator, single-symbol/multi-symbol run.

Files/modules:

- `backend/app/domain/strategy/*`
- `backend/app/services/backtest_service.py`
- `backend/app/api/backtest.py`
- `frontend/src/pages/BacktestPage.tsx`

Tasks:

1. Define rule AST/DSL.
2. Validate strategy schema.
3. Run MA crossover and MACD+RSI strategies.
4. Share broker/accounting with manual replay.
5. Persist run summary and trades.
6. Add result slices by symbol and period.

Tests:

- Sample strategy tests.
- Security tests.
- No-data/no-trade/error tests.

Acceptance:

- Backtest can answer "MACD + RSI win rate by symbol and period" safely.

## Phase 5 - Regime Classifier And Research Slices

Goal:

- Compare setups by market condition.

Scope:

- VNINDEX regime labels, sector strength, period buckets.

Files/modules:

- `backend/app/domain/regime/`
- `backend/app/services/backtest_service.py`
- `backend/app/services/analytics_service.py`

Tasks:

1. Compute VNINDEX regime per date.
2. Store regime labels.
3. Add result slicing by regime.
4. Add sector strength if sector metadata exists.

Tests:

- Known VNINDEX scenarios produce expected regime labels.
- Backtest result groups by regime.

Acceptance:

- User can compare a strategy in bullish, sideways, bearish and volatile periods.

## Phase 6 - Scanner V1

Goal:

- Find current or historical setups using the same signal taxonomy.

Scope:

- Historical scanner first, live/current scanner later.

Tasks:

1. Run signal definitions over symbol universe.
2. Save scan results.
3. Filter by signal, setup, sector, regime.
4. Link scan result to replay session start.

Acceptance:

- User can find symbols that match a setup and then practice/replay them.

## Phase 7 - Strategy Lab Professional V1

Goal:

- Compare strategies and parameters without over-engineering.

Scope:

- Saved strategies, comparisons, parameter grid.

Tasks:

1. Save strategy versions.
2. Compare runs.
3. Add parameter sweep for selected numeric params.
4. Add heatmap/table outputs.

Acceptance:

- User can compare variants and see robustness, not just best result.

## Phase 8 - Optional Infrastructure

Goal:

- Scale only after correctness.

Scope:

- Background worker, PostgreSQL option, larger datasets.

Tasks:

1. Add background job queue only if sync runner is too slow.
2. Add PostgreSQL/TimescaleDB profile only if SQLite is insufficient.
3. Keep SQLite path documented and tested.

Acceptance:

- Infrastructure does not make the local-first product harder to run.

