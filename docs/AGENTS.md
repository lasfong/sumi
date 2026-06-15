# AGENTS.md — RULES CHO CODEX / ANTIGRAVITY

## Project

This project is `vn-technical-replay-lab`, a local-first personal webapp for Vietnamese stock technical analysis replay and trading journal.

## Main goal

Build a TradingView-like bar replay tool for Vietnamese equities using imported CafeF historical data.

The app should let the user:

* Import CafeF historical stock data.
* View candlestick charts.
* Replay historical candles one by one.
* Make manual Buy/Sell/Hold/Skip decisions.
* Record trading reasons, setup types, confidence scores, stop-loss and target.
* Review trades and calculate performance statistics.

## Non-negotiable rule

Never leak future candles in replay mode.

Backend must only return visible candles up to `current_index`.

Do not send all candles to frontend and slice there.

## Tech stack

Backend:

* Python
* FastAPI
* SQLAlchemy
* SQLite
* Pandas
* Pydantic
* pytest

Frontend:

* React
* TypeScript
* Vite
* TradingView Lightweight Charts
* TanStack Query

## Code style

* Use clear names.
* Use type hints.
* Keep services separated from API routes.
* Do not put business logic directly inside route handlers.
* Write small functions.
* Add comments for non-obvious logic.
* Prefer explicit error messages.
* Avoid over-engineering.

## Safety rules

* Do not run destructive shell commands.
* Do not delete files outside the project.
* Do not access secrets.
* Do not add telemetry.
* Do not send user data to external APIs.
* Do not scrape websites aggressively.
* Import data from user-uploaded files first.

## Backend conventions

Use these folders:

```text
app/models
app/schemas
app/api
app/services
app/tests
```

API routes should be grouped by domain:

```text
import_data.py
symbols.py
replay.py
decisions.py
reports.py
```

Business services:

```text
cafef_importer.py
data_normalizer.py
indicator_service.py
replay_engine.py
trade_engine.py
stats_engine.py
```

## Frontend conventions

Use these folders:

```text
src/api
src/components
src/pages
src/types
src/store
```

Main pages:

```text
ImportPage
ReplayPage
JournalPage
ReportPage
```

## Testing priorities

Minimum tests:

1. Importer can parse sample CafeF-like CSV.
2. Replay session returns only visible candles.
3. Next candle increases current index by 1.
4. Previous candle decreases current index by 1.
5. Buy/Sell decisions are saved.
6. Closed trade calculates PnL correctly.

## MVP acceptance

The MVP is complete when:

* User can import a CafeF file.
* User can choose a symbol.
* User can create a replay session.
* Chart displays visible candles only.
* Next Candle shows one more candle.
* User can submit Buy/Sell decisions.
* Decisions are saved.
* Buy/Sell markers appear on chart.
* Simple report shows PnL and win rate.
