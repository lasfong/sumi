# Sumi V2 Release Checklist

Status: release hardening checklist.
Date: 2026-06-29.

## Product Scope

Sumi V2 is considered complete when the local-first product supports:

- Manual replay with no-future-leak candles and indicators.
- Drawing persistence, decision journal, trade lifecycle, pending orders and T+2 constraints.
- Replay session resume and scanner-launched replay sessions.
- Analytics for equity, drawdown, benchmark, outlier impact, setup/symbol/mistake grouping and trade distribution.
- Safe automated backtest with declarative rules, no `eval()`, MA and MACD+RSI sample strategies.
- Historical scanner with saved scan runs and replay links.
- Strategy Lab comparison, parameter sweep and persisted run history.

## Automated Gate

Run from repository root:

```powershell
.\scripts\verify-v2.ps1
```

This runs:

- Backend pytest suite.
- Alembic upgrade head.
- Frontend lint.
- Frontend tests.
- Frontend production build.

## Browser Regression Gate

Run this after starting backend and frontend locally:

```powershell
cd frontend
npm.cmd run smoke:browser
```

Or from repository root after both local services are already running:

```powershell
.\scripts\verify-v2.ps1 -BrowserSmoke
```

This browser smoke covers:

- Replay session creation.
- EMA/RSI/MACD indicator warm-up.
- BUY, T+1 SELL rejection and T+2 SELL success.
- Backtest MACD RSI Momentum.
- Strategy Lab comparison and sweep.
- Scanner run and scanner-to-replay launch.
- Analytics render for the smoke replay session.

## Manual UAT Checklist

1. Start backend and frontend locally.
2. Import CafeF data or use an existing local SQLite dataset.
3. Create or resume a replay session.
4. Confirm Replay shows only revealed candles.
5. Add EMA/RSI/MACD indicators and confirm no future timestamps appear in session-scoped indicator output.
6. Submit BUY, HOLD and SELL decisions.
7. Confirm T+1 sell rejection and T+2 sell success.
8. Place invalid and valid limit orders, then advance replay to verify pending/fill behavior.
9. Add drawings, reload, and confirm drawings persist.
10. Open Analytics and verify equity, drawdown, benchmark, outlier and trade distribution panels render.
11. Run MA Crossover backtest.
12. Run MACD RSI Momentum backtest.
13. Run Strategy Lab comparison and parameter sweep.
14. Run Scanner, save scan history, load a saved scan and open Replay from a signal.

## Known Product Limits

- SQLite remains the default and supported V2 database.
- Daily candles are the supported happy path.
- Sector strength and advanced scanner filters are future scope unless sector metadata is complete.
- Export/reporting is useful future scope but not required for V2 completion.
- No multi-user auth; this is intentionally local-first.

## Release Definition

The release is ready when:

- `scripts/verify-v2.ps1` passes.
- Git workspace is clean.
- Latest commit is pushed to `origin/master`.
- Any remaining issue is documented as future scope rather than hidden as a broken feature.
