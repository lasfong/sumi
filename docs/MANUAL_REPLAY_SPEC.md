# Manual Replay Product Spec

Status: V2 detailed spec.
Date: 2026-06-28.

## Goal

Manual replay is Sumi's core experience. It must feel like a serious TradingView-style practice lab, but with journal, trading simulation and analytics built into the flow.

## User Workflow

1. Import or select market data.
2. Choose symbol, timeframe, date range and initial cash.
3. Create replay session.
4. View only candles available up to `current_index`.
5. Add indicators and drawings.
6. Step/play forward bar by bar.
7. Make decisions: BUY, SELL, HOLD, SKIP, ADD, REDUCE, CLOSE, CUT_LOSS, TAKE_PROFIT.
8. Record setup, reason, stop, target, confidence, market context and mistake tag.
9. Review position, trade log and journal.
10. End session and inspect analytics.

## Non-Negotiable Rules

- Backend returns only visible candles.
- Replay indicators are computed from visible candles only.
- Frontend must never receive future replay data.
- Previous/rewind must not delete decisions silently.
- Trading logic must enforce no shorting, T+2, fee/tax and position limits.
- All financial metrics must be reproducible by tests.

## UI Requirements

### Replay Page

Top bar:

- Symbol, session id, current bar, OHLC, replay controls, speed, indicator selector.

Main chart:

- Candle series.
- Volume pane.
- Indicator overlay/panes.
- Trade markers.
- Drawings.

Left toolbar:

- Cursor.
- Trendline.
- Horizontal line.
- Fibonacci.
- Clear drawings.

Right panel:

- Order/decision ticket.
- Position summary.
- Decision journal.
- Open/pending orders.

Bottom or secondary panel:

- Trade log.
- Event log.
- Notes/review.

## Indicator Requirements

MVP:

- SMA.
- EMA.
- RSI.
- Volume MA.

Professional v1:

- MACD.
- Bollinger Bands.
- ATR.
- ADX.
- Ichimoku.
- Stochastic.
- Relative strength vs VNINDEX.

Indicator response must include:

```json
{
  "session_id": 1,
  "indicator": "macd",
  "timeframe": "1D",
  "data": [
    {
      "timestamp": "2024-01-02T00:00:00",
      "MACD_12_26_9": 1.2,
      "MACDs_12_26_9": 1.0,
      "MACDh_12_26_9": 0.2
    }
  ]
}
```

## Drawing Requirements

MVP:

- Persist drawings by session.
- Drawings use candle time and price coordinates.
- Drawing type cannot include `cursor`; cursor is a tool, not a persisted drawing.

Professional v1:

- Persist drawing templates by symbol.
- Support labels/notes attached to drawing.
- Support visibility by timeframe.

## Trading Simulation Requirements

Order lifecycle:

```text
created -> pending -> executed
created -> rejected
pending -> cancelled
pending -> expired
```

MVP order types:

- `MARKET_AT_CLOSE`.
- `LIMIT`.

Professional v1:

- `MARKET_NEXT_OPEN`.
- `STOP`.
- `STOP_LIMIT`.

Vietnam rules:

- No shorting for stock.
- T+2 sell availability.
- Buy fee default: 0.15%.
- Sell fee default: 0.15%.
- Sell tax default: 0.1%.
- Price limits: HOSE 7%, HNX 10%, UPCOM 15%.

## Journal Requirements

Every meaningful decision should capture:

- Setup type.
- Signal/pattern tags.
- Market regime/context.
- Stop loss.
- Target.
- Planned R/R.
- Confidence.
- Reason.
- Emotion state.
- Mistake tag.
- Post-trade review.
- Lesson learned.

## Manual Analytics

Required outputs:

- Total trades, closed trades, open trades.
- Win rate.
- Gross and net PnL.
- Average win/loss.
- Profit factor.
- Expectancy and expectancy R.
- R-multiple distribution.
- Max drawdown amount and percentage.
- Drawdown periods.
- Sharpe/Sortino when equity curve is sufficient.
- Performance by setup, mistake, symbol, period and regime.
- Outlier contribution.

## Current Gaps To Fix First

- Align `AnalyticsService._build_equity_curve()` with frontend `EquityChart` contract.
- Link executions to trade lifecycle so multiple trades in the same session/symbol calculate correctly.
- Make lint pass.
- Make CORS and debug config safe by default.
- Keep session-scoped indicator endpoint and add regression tests.

