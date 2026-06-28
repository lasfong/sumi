# Sumi Specification V2

Status: Proposed canonical V2 planning spec.
Date: 2026-06-28.

## 1. Scope

Sumi has two distinct products that share data, indicators, broker/accounting and analytics.

### Product A: Manual Replay Trading Lab

Purpose:

- Help the user practice technical analysis and decision-making.
- Hide future data completely.
- Capture decisions and reasoning.
- Simulate realistic Vietnam stock trading.
- Convert practice into measurable learning feedback.

Primary page:

- Replay Page.

Primary output:

- Session journal, decision log, trade log and manual performance analytics.

### Product B: Automated Backtest Research Lab

Purpose:

- Validate technical methods systematically.
- Compare setups across symbols, regimes, periods and indicators.
- Detect whether results are robust or outlier-driven.
- Support scanner and strategy research later.

Primary page:

- Backtest / Strategy Lab Page.

Primary output:

- Backtest result, metrics, equity curve, trade list and grouped research tables.

## 2. Shared Architecture

```text
Market Data
-> Indicator Engine
-> Signal/Pattern Engine
-> Broker/Execution Engine
-> Trade Ledger
-> Analytics Engine
-> UI Reports
```

Manual replay and automated backtest must not duplicate broker/accounting logic. If BUY/SELL/fee/T+2 changes, both products must use the same implementation.

## 3. Data Model V2

### Existing Core Tables To Keep

- `symbols`
- `candles`
- `replay_sessions`
- `decisions`
- `orders`
- `executions`
- `positions`
- `trades`
- `journal_entries`
- `event_logs`
- `drawings`

### Required Fixes

- `executions` or `orders` must link to `trade_id`, or a separate ledger must map executions to trade lifecycle. Session+symbol grouping is insufficient.
- `symbols.exchange` must be populated when possible.
- Analytics equity points must use one contract:

```json
{
  "timestamp": "2024-01-02",
  "equity": 100000000,
  "cash": 90000000,
  "holdings_value": 10000000,
  "drawdown": 0,
  "drawdown_pct": 0
}
```

### Professional v1 Tables

- `indicator_definitions`
- `indicator_presets`
- `signal_definitions`
- `pattern_events`
- `market_regimes`
- `strategy_definitions`
- `backtest_runs`
- `backtest_result_slices`
- `backtest_trades`
- `scan_results`
- `sector_memberships`

## 4. Indicator Architecture

Each indicator must have:

- Stable `id`, display name and category.
- Parameter schema with defaults and valid ranges.
- Output series metadata: pane, line/histogram/band/cloud, color hint.
- Warmup requirement.
- No-future-leak mode for replay.
- Batch mode for backtest.

MVP indicators:

- SMA, EMA, RSI, Volume MA.

Professional v1 indicators:

- MACD, Bollinger Bands, ATR, ADX, Ichimoku, Stochastic, relative strength vs VNINDEX.

## 5. Signal And Pattern Taxonomy

Every strategy, scanner result and manual journal setup should use common tags:

- `indicator_signal`: RSI oversold, MACD cross, MA cross, ADX trend strength.
- `price_pattern`: breakout, pullback, base, retest, failed breakout.
- `candlestick_pattern`: engulfing, pin bar, doji, gap.
- `context_signal`: volume expansion, relative strength, sector strength.
- `direction`: bullish, bearish, neutral.
- `setup_type`: breakout, pullback, trend-following, reversal, accumulation, distribution.
- `quality_grade`: A, B, C, Avoid.

## 6. Market Regime Model

MVP can store a manual/context field. Professional v1 needs computed regimes:

- `bullish`
- `sideways`
- `bearish`
- `volatile`
- `accumulation`
- `distribution`

Suggested v1 classifier inputs:

- VNINDEX close relative to MA20/MA50/MA200.
- VNINDEX slope of MA20/MA50.
- ATR percentile or realized volatility.
- Market breadth if available.
- Sector strength if sector metadata exists.

Regime classifier output must be reproducible and stored per date.

## 7. API Surface

### Manual Replay

- `POST /api/replay/sessions`
- `GET /api/replay/sessions/{id}/candles`
- `POST /api/replay/sessions/{id}/next`
- `POST /api/replay/sessions/{id}/previous`
- `GET /api/replay/sessions/{id}/indicators`
- `POST /api/replay/sessions/{id}/decisions`
- `GET /api/replay/sessions/{id}/positions`
- `GET /api/replay/sessions/{id}/trades`
- `GET /api/replay/sessions/{id}/analytics`
- `GET/PUT /api/replay/sessions/{id}/drawings`

### Backtest / Strategy Lab

- `GET /api/strategies`
- `POST /api/strategies`
- `POST /api/backtests/run`
- `GET /api/backtests/{run_id}`
- `GET /api/backtests/{run_id}/trades`
- `GET /api/backtests/{run_id}/slices`
- `POST /api/backtests/compare`

MVP can keep `POST /api/backtest/run`, but response must be deterministic, safe and typed.

## 8. Testing Requirements

MVP release cannot pass unless:

- Frontend build passes.
- Frontend lint passes.
- Frontend tests pass.
- Backend tests pass.
- Alembic migration runs on a fresh DB.
- Replay candles no-future-leak test passes.
- Replay indicators no-future-leak test passes.
- T+1 sell rejected, T+2 sell allowed.
- Multi-round-trip PnL per trade is correct.
- Analytics frontend/backend contract is tested.
- Backtest malicious expression is rejected.
- Backtest errors do not get swallowed.

