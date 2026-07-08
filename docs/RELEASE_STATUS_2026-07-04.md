# Sumi Release Status - Post-RC2 Hardening

Date: 2026-07-05
Branch: `codex/post-rc2-hardening`
Scope: local-first SQLite, daily-candle replay/backtest/scanner workflow.

## Current Status

`v2.0.0-rc2` is a verified technical release for the local-first SQLite daily-candle scope. Its provenance has already been verified and the existing `v2.0.0-rc2` tag must not be moved, retagged, or rewritten.

This branch adds post-RC2 hardening only. It does not declare a stable release and does not add new product features.

## Fixed Risks

- Backtest-created `ReplaySession(mode="backtest")` records now have explicit retention cleanup through `BacktestCleanupService` and `POST /api/backtest/cleanup-sessions`.
- Cleanup deletes dependent decisions, orders, executions, positions, trades, journal entries, drawing states, and event logs for selected backtest sessions.
- Cleanup never selects manual replay sessions because it filters by `ReplaySession.mode == "backtest"`.
- Scanner rule evaluation is decoupled from BacktestService private methods. Backtest and Scanner now use shared `StrategyRuleEvaluator`.
- E2E indicator parity coverage compares Replay indicator API output against the StrategyIndicatorAdapter path for RSI, MACD, and CCI by timestamp after warmup.
- Browser smoke keeps normal success runs fast and stores screenshot, trace, and video artifacts only on failure.

## Accepted Limitations

- Scope remains local-first SQLite and daily candles only.
- Cleanup is manual via API; it is intentionally not run automatically on app start.
- Browser smoke failure videos may be large, so artifacts are retained only for failed runs under `test-results/browser-smoke/`.
- Community chart/drawing libraries remain adapter-scoped; RC2 does not vendor or clone third-party repositories into the source tree.

## Open Risks

- No stable-release declaration yet. This branch is post-RC2 hardening and still requires full verification before any later release candidate.
- Browser smoke depends on local backend/frontend services and an installed Playwright-compatible browser channel.
- SaaS/multi-tenant operations, realtime data, broker integration, and real-money trading are out of scope.

## Explicit Non-Goals

- SaaS deployment.
- Realtime/intraday market data.
- Real-money trading.
- Broker integration.
- Moving, retagging, or rewriting `v2.0.0-rc2`.

## Verification Commands

```bash
git status --porcelain
git rev-list -n 1 v2.0.0-rc2
cd backend && ../.venv/bin/python -m pytest app/tests -q
cd frontend && npm run lint
cd frontend && npm test
cd frontend && npm run build
cd backend && DATABASE_URL=sqlite:////tmp/sumi-hardening-verify.db ../.venv/bin/python -m alembic upgrade head
cd backend && DATABASE_URL=sqlite:////tmp/sumi-hardening-verify.db ../.venv/bin/python scripts/seed_demo.py
cd backend && DATABASE_URL=sqlite:////tmp/sumi-hardening-verify.db ../.venv/bin/python -m uvicorn app.main:app --host 127.0.0.1 --port 8000
cd frontend && npm run dev -- --host 127.0.0.1 --port 5173
cd frontend && SUMI_BROWSER_CHANNEL=chrome npm run smoke:browser
```
