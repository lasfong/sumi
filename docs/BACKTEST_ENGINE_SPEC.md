# Automated Backtest Engine Spec

Status: V2 detailed spec.
Date: 2026-06-28.

## Goal

The automated engine validates technical-analysis methods across symbols, regimes and time periods. It must not be a toy PnL calculator.

## MVP Principle

Build a safe, deterministic, local, single-process engine first. Add code editors, subprocess Python and optimization only after the engine is correct.

## Strategy Model

MVP strategy should be declarative:

```yaml
name: "MACD RSI Confirmation"
universe:
  symbols: ["FPT", "SSI", "VCI"]
timeframe: "1D"
indicators:
  - id: macd
    params: {fast: 12, slow: 26, signal: 9}
  - id: rsi
    params: {length: 14}
entry:
  all:
    - gt: ["macd.histogram", 0]
    - cross_up: ["macd.line", "macd.signal"]
    - lt: ["rsi.value", 70]
exit:
  any:
    - cross_down: ["macd.line", "macd.signal"]
    - lt: ["rsi.value", 45]
risk:
  position_sizing: {method: fixed_quantity, quantity: 1000}
  stop_loss_pct: 8
  take_profit_pct: null
```

Do not evaluate arbitrary condition strings. Allowed operators should be parsed as data:

- `gt`, `gte`, `lt`, `lte`, `eq`.
- `and`, `or`, `not`.
- `cross_up`, `cross_down`.
- `rising`, `falling`.
- `between`.

## Security Requirements

P0:

- Remove Python `eval()` from API-controlled backtest expressions.
- Reject unknown identifiers.
- Reject `__import__`, `open`, `os`, `subprocess`, attribute access and function calls not in whitelist.
- Backtest errors must return failed status with message; no silent `traceback.print_exc()` only.

Future trusted-local Python:

- Run in subprocess.
- Use timeout.
- Use temp working directory.
- Capture stdout/stderr.
- Clearly label as trusted local code unless a real sandbox exists.

## Engine Flow

```text
Load strategy
-> validate strategy schema
-> load candles
-> compute indicators with warmup
-> compute optional regime labels
-> iterate bars
-> generate orders
-> broker validates cash, T+2, no short, price limits
-> write executions/trades/equity
-> compute analytics
-> compute result slices
```

## Shared Broker Requirement

The automated engine must call the same broker/accounting layer as manual replay. It can create a `mode="backtest"` session, but trade lifecycle rules must not be forked.

Required broker behavior:

- Cash ledger.
- Holdings ledger.
- Pending order queue.
- T+2 availability.
- Fees and tax.
- Price limit validation.
- Trade lifecycle mapping.
- Force liquidation / bankruptcy handling.

## Backtest Outputs

Top-level metrics:

- Initial cash.
- Final equity.
- Total return.
- CAGR if period is long enough.
- Total trades.
- Win rate.
- Average win/loss.
- Profit factor.
- Expectancy and expectancy R.
- Max drawdown amount/pct.
- Drawdown duration.
- Sharpe/Sortino where meaningful.
- Exposure time.
- Best/worst trade.
- Median trade.
- Top 5 winners contribution.
- Top 5 losers contribution.

Grouped slices:

- By symbol.
- By sector.
- By indicator setup.
- By pattern/signal.
- By setup type.
- By market regime.
- By time period.
- By holding period bucket.
- By risk/reward bucket.

The sample in `docs/BacktestSample/Sample.md` implies that Sumi must support period/regime comparisons like:

- 2021-03-20 to 2026-04-16 full market period.
- 2025-09-01 to 2026-04-16 sideways period.
- 2026-03-01 to 2026-04-16 difficult period.

## API Contracts

### Run Backtest

```http
POST /api/backtests/run
```

Request:

```json
{
  "strategy": {},
  "symbols": ["FPT", "SSI"],
  "start_date": "2021-01-01",
  "end_date": "2026-04-16",
  "initial_cash": 100000000,
  "benchmark_symbol": "VNINDEX",
  "slice_by": ["symbol", "regime", "setup_type"]
}
```

Response:

```json
{
  "run_id": 1,
  "status": "succeeded",
  "metrics": {},
  "equity_curve": [],
  "benchmark_curve": [],
  "slices": [],
  "warnings": []
}
```

### Failure Response

```json
{
  "run_id": 1,
  "status": "failed",
  "error_code": "INVALID_RULE",
  "message": "Unknown identifier: os",
  "bar_index": null
}
```

## MVP Backtest Acceptance

- MA crossover sample runs.
- MACD + RSI sample runs.
- Malicious expression is rejected.
- No-data input returns clear error.
- No-trade strategy returns valid zero-trade analytics.
- Backtest uses same PnL as manual trades.
- Multi-round-trip PnL is correct.
- Result table can group by symbol and period at minimum.

