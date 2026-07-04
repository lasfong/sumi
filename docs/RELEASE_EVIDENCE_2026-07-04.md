# Sumi V2 RC2 Release Evidence

Date: 2026-07-04  
Branch: `codex/lightweight-charts-v5-spike`  
Version: `2.0.0-rc2`  
Scope: local-first, SQLite, daily-candle replay and research

## Final Gate Evidence

Command:

```bash
SUMI_BROWSER_SMOKE=1 SUMI_BROWSER_CHANNEL=chrome ./scripts/verify-v2.sh
```

Results:

- Backend: `71 passed, 1 skipped, 1 dependency warning`.
- Fresh SQLite migration: passed through Alembic head `20260629_0004`.
- Frontend lint: passed.
- Frontend tests: `9 files, 18 tests` passed.
- Frontend production build: passed.
- Browser smoke: passed.
- Deterministic seed: 2,080 candles across FPT, SSI, VCI and VNINDEX.

The skipped backend test is the optional environment-dependent browser/e2e
helper. The warning is Starlette's `httpx` compatibility deprecation and does
not represent a failed product behavior.

## Business Reconciliation Evidence

- Non-positive execution price/quantity is rejected before order creation.
- Known ledger reconciles buy/sell fee, sell tax, cash and net PnL.
- Closed-trade analytics fixture reconciles cash, holdings, equity, drawdown,
  win rate, expectancy and normalized VNINDEX benchmark values.
- No-trade analytics returns deterministic flat equity and benchmark output.
- Import is idempotent and updates an existing candle without duplication.
- Scanner-created replay starts at the lookback cursor and reveals one candle;
  future signal/forward candles are not exposed initially.

## Browser UAT Evidence

Validated workflows:

- Replay session creation, EMA/RSI/MACD, BUY, T+1 rejection and T+2 close.
- Backtest with result metrics and equity output.
- Strategy comparison and parameter sweep.
- Scanner run and scanner-to-replay transition.
- Analytics on a session containing a completed trade.
- Mobile navigation and all primary routes at 390 x 844.

Visual review found and fixed two mobile defects:

- Fixed sidebar consumed most of the mobile viewport.
- Replay retained a three-column workstation layout and compressed the chart.

The mobile layout now uses a compact horizontal navigation strip, horizontal
drawing toolbar, full-width chart and stacked trading details.

## Go/No-Go Decision

**GO** for an internal/expert-user release candidate within the documented V2
scope: local-first, SQLite and daily candles.

**NO-GO** for production SaaS, real-money trading, broker connectivity,
intraday/realtime use or guaranteed multi-symbol portfolio accounting. Those
capabilities were not part of this release and require separate architecture,
security and operational work.

No open P0/P1 defect was found in the supported acceptance matrix. Remaining
known technical items are the Starlette/httpx deprecation warning, Node test
localStorage warnings, and the intentional serialized Strategy Lab execution
required by SQLite.
