# Future Roadmap

This document summarizes long-term ideas from `docs/SPEC.md` without making them immediate MVP requirements.

The current implementation contract remains `docs/PRODUCT_SPEC.md`.

## Phase F1 - Analytics Deepening

Build after the manual replay/trade lifecycle is stable.

Scope:

- Equity curve from real ledger and mark-to-market holdings.
- VNINDEX benchmark overlay.
- Max drawdown amount and percentage.
- Drawdown duration.
- Trade distribution histogram.
- Sharpe ratio from equity returns.
- Profit factor, expectancy, average R, payoff ratio.

Exit criteria:

- Metrics have deterministic unit tests.
- No hard-coded initial capital.
- No division-by-zero failures.

## Phase F2 - Market Rules Expansion

Scope:

- Complete HOSE/HNX/UPCOM price-limit validation.
- Exchange metadata import for all CafeF symbols.
- Tick-size rounding.
- More order types:
  - `MARKET_NEXT_OPEN`
  - `LIMIT`
  - `STOP`
  - `STOP_LIMIT`
- Pending order lifecycle.

Exit criteria:

- TC-004 passes.
- Pending orders can be placed, matched, cancelled, and audited.

## Phase F3 - Algorithmic Backtest Runner

Start with a backend runner before a fancy editor.

Scope:

- `BaseStrategy` contract.
- Strategy upload or source-code submission.
- Backtest job model.
- Backtest result persistence.
- Subprocess execution with timeout.
- Shared broker/accounting engine.
- Reuse analytics calculations.

Not in the first runner:

- Celery/Redis.
- Multi-user sandboxing.
- Parameter optimization UI.
- Cloud execution.

Exit criteria:

- A sample SMA crossover strategy runs end-to-end.
- Result includes trades, executions, equity curve, and metrics.
- Errors are captured and shown clearly.

## Phase F4 - Background Worker

Only add this after the sync runner is correct.

Scope:

- Celery or a simpler local task queue.
- Job status polling.
- Cancel job.
- Persist logs.
- Long-running optimization jobs.

Exit criteria:

- Backtest jobs do not block the API server.
- Failed jobs preserve error details.

## Phase F5 - Strategy Lab UI

Scope:

- Strategy list.
- Code editor.
- Run configuration form.
- Backtest result dashboard.
- Equity/benchmark chart.
- Trade table.
- Parameter testing UI.

Exit criteria:

- User can run a known sample strategy without touching backend code.

## Phase F6 - PostgreSQL / TimescaleDB Option

This is optional and should not block the local-first MVP.

Scope:

- Postgres deployment profile.
- Alembic migrations compatible with SQLite and Postgres where practical.
- TimescaleDB hypertable for OHLCV if historical data size requires it.

Exit criteria:

- SQLite remains supported for local users.
- Postgres path is documented and tested separately.

