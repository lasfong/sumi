# Acceptance Criteria V2

Status: QA gate for Sumi completion.
Date: 2026-06-28.

## Release Gate

Sumi cannot be accepted while any P0/P1 item below is open.

## P0 Blockers

1. Backtest engine does not execute API-controlled strings with `eval()`.
2. Trade PnL is correct for multiple round-trip trades in the same session and symbol.
3. Backend/frontend analytics contract is aligned and tested.
4. Replay candles and indicators do not leak future data.
5. Automated backtest errors are not swallowed.

## P1 Blockers

1. `npm.cmd run lint` passes.
2. `npm.cmd run build` passes.
3. `npm.cmd test` passes.
4. Backend pytest suite passes.
5. Alembic is installed and `alembic upgrade head` works on a fresh DB.
6. Default config is safe: `DEBUG=False`, `AUTO_CREATE_TABLES=False` outside explicit local dev.
7. CORS is configurable and defaults to localhost frontend origins.

## Command Gate

Run from repo root or listed folders:

```bash
cd backend
.\.venv\Scripts\python.exe -m pytest -q app\tests
.\.venv\Scripts\alembic.exe upgrade head

cd ..\frontend
npm.cmd run lint
npm.cmd run build
npm.cmd test
```

## Manual Replay UAT

1. Import CafeF sample.
2. List symbols.
3. Create replay session.
4. Confirm chart shows only visible candles.
5. Add EMA/RSI and confirm response has no timestamp after visible candle.
6. BUY market at current bar.
7. SELL at T+1 is rejected.
8. SELL at T+2 succeeds.
9. LIMIT above HOSE/HNX/UPCOM ceiling is rejected.
10. Valid LIMIT remains pending and fills when a revealed candle touches price.
11. Journal captures setup/reason/confidence/mistake.
12. Analytics renders equity, drawdown and trades.

## Backtest UAT

1. Run MA crossover sample.
2. Run MACD + RSI sample.
3. Run malicious rule sample and confirm rejection.
4. Run no-data sample and confirm clear error.
5. Run no-trade sample and confirm zero-trade analytics.
6. Compare result by symbol.
7. Compare result by period/regime.
8. Confirm top outlier contribution is visible.

## Analytics Acceptance

Equity point contract:

```json
{
  "timestamp": "2024-01-02",
  "equity": 101000000,
  "cash": 90000000,
  "holdings_value": 11000000,
  "drawdown": 0,
  "drawdown_pct": 0
}
```

Required metric tests:

- Win rate.
- Profit factor with no losses.
- Profit factor with no wins.
- Expectancy.
- Average R.
- Max drawdown amount.
- Max drawdown pct.
- Sharpe with zero volatility.
- VNINDEX benchmark if data exists.
- Trade distribution.
- Outlier contribution.

## Definition Of Done

A phase is done only when:

- Implementation exists.
- Tests cover critical logic.
- Commands pass.
- Docs reflect behavior.
- Known limitations are documented.
- No user data is sent outside local machine.

