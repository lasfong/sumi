# Sumi V2 Browser UAT Review - 2026-06-29

## Scope

Browser UAT was executed against local FastAPI (`127.0.0.1:8000`) and Vite (`127.0.0.1:5173`) using the existing local SQLite CafeF dataset.

## Passed Flows

- Replay session creation from UI: created FPT session and confirmed future candles are hidden at session start.
- Replay resume: resumed existing sessions and scanner-created sessions.
- Indicator selector: EMA, RSI, and MACD no longer fail during warm-up bars.
- Trade lifecycle: BUY succeeds, T+1 SELL is rejected by T+2 rule, T+2 SELL succeeds.
- Open positions panel: closed zero-quantity positions are no longer shown as `SHORT 0`.
- Drawing tools: horizontal line can be drawn and persists through `/api/replay/sessions/{id}/drawings`.
- Backtest Engine: MACD RSI Momentum runs from UI and renders metrics, equity curve, symbol slices, period slices, and regime slices.
- Strategy Lab: comparison, parameter sweep, and persisted run history render from UI.
- Signal Scanner: scanner run returns 45 signals for the default symbol set, persists history, and opens Replay from a signal with provenance panel.
- Analytics: session analytics renders equity/drawdown, benchmark, outlier impact, drawdown periods, trade distribution, setup performance, and trade history.

## Bugs Found And Fixed

- Session-scoped indicator endpoint returned `400` during warm-up when an indicator had not produced output columns yet.
- Replay chart crashed when REST and WebSocket candles mixed date formats or duplicate timestamps.
- Existing local SQLite databases could miss `executions.trade_id`, causing trade execution `500` errors despite models expecting the column.
- Open positions API returned closed positions, causing the UI to show `SHORT 0`.
- Equity charts crashed on SQL datetime strings such as `2020-01-02 00:00:00`.
- Recent Replay Sessions could include automated backtest sessions with `mode=backtest`, causing response validation errors.

## Evidence Sessions

- Manual trade flow: replay session `#9`.
- Scanner-to-replay provenance: replay session `#27`.
- Backtest UI result: session `#11`.
- Strategy Lab persisted comparison and sweep runs created at `2026-06-29 20:11` local time.

## Remaining Notes

- Browser console history retained older errors from pre-fix tabs; backend logs and fresh page state were used to verify fixed requests.
- Drawing requires clicking on a chart time coordinate. Clicking whitespace does not create a drawing because `lightweight-charts` does not provide a `param.time` there.
- `npm.cmd run smoke:browser` was added as a repeatable browser regression gate. It requires local backend/frontend services and a Playwright-compatible browser.
