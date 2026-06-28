# Sumi Product Strategy V2

Status: Proposed product strategy.
Date: 2026-06-28.

## Product Thesis

Sumi is a local-first trading learning and research platform for Vietnam stock market users. Its strongest wedge is not "yet another backtester". Its wedge is:

```text
TradingView-like manual replay
+ disciplined trading journal
+ technical-analysis research engine
+ Vietnam market rules
+ local ownership of data
```

Manual replay is the center of the product. Automated backtest exists to validate the methods that the user practices manually.

## Target Questions Sumi Must Answer

1. With symbol A, how does MACD + RSI perform in bullish, sideways, bearish or volatile regimes?
2. Which symbols respond best to Ichimoku, and in which market phases?
3. Are bullish/bearish patterns actually predictive, or are they only visually convincing?
4. Which setup has positive expectancy after fees, tax, T+2 and realistic exits?
5. Which setup wins only because of one or two outlier trades?
6. Where does the user make manual mistakes: early entry, late exit, stop-loss violation, FOMO, holding through regime change?
7. Does the user's manual edge match the automated signal edge?

## Product Priority

### MVP Mandatory

- Local SQLite app runs reliably.
- CafeF CSV/TXT/ZIP import works with data-quality warnings.
- Manual replay page hides future data at backend level.
- Session-scoped indicators hide future indicator data.
- Chart shows candle, volume, overlays and trade markers.
- Decision, order, execution, position and trade lifecycle are separated.
- Vietnam rules: no shorting, T+2, buy/sell fee and sell tax.
- Journal captures setup, reason, confidence, stop, target, mistake and review.
- Analytics calculates correct PnL, win rate, expectancy, profit factor, R-multiple and drawdown for manual sessions.
- Automated backtest MVP runs safe declarative strategies without `eval()`.
- QA gates pass: backend tests, frontend build, frontend lint, frontend tests, Alembic migration.

### Professional v1

- Indicator registry: SMA, EMA, MA, RSI, MACD, Bollinger Bands, ATR, ADX, Ichimoku, Volume MA, Stochastic.
- Drawing persistence: trendline, horizontal line, fibonacci, notes/labels.
- Multi-timeframe support: 1D base, 1W and 1M resampled from daily data.
- Strategy Lab with saved strategies, safe rule DSL, multi-symbol runs and comparison.
- Result slicing by symbol, setup, indicator, signal, pattern, market regime, holding period and period bucket.
- VNINDEX benchmark overlay and sector-relative strength.
- Regime classifier v1: bullish, sideways, bearish, volatile, accumulation, distribution.
- Signal taxonomy: bullish pattern, bullish signal, bearish pattern, bearish signal, neutral/context.
- Outlier analysis: median trade, trimmed expectancy, top-N contribution, worst-N contribution.
- Scanner v1 built from the same signal taxonomy.

### Future / Advanced

- Parameter grid/optimization.
- Portfolio-level allocation and ranking.
- Sector rotation and breadth models.
- Walk-forward testing.
- Monte Carlo resampling.
- Intraday data.
- Python strategy subprocess sandbox for trusted local code.
- Background job runner after sync runner is correct.
- PostgreSQL/TimescaleDB option while preserving SQLite local-first.

## Current Repo Reality

Based on docs and code review:

- The project already has real FastAPI services, SQLAlchemy models, React pages, replay UI, indicator API, limit order work, analytics and backtest skeleton.
- The current QA gate is not passable for handover because:
  - `backend/app/services/backtest_service.py` still uses `eval()`.
  - Multi-round-trip PnL is wrong because executions are aggregated by session and symbol, not by trade lifecycle.
  - Backend analytics equity points use `time`, while frontend expects `timestamp` and `drawdown`.
  - `npm run lint` has known failures.
  - Alembic dependency and migration workflow are not fully operational.
  - CORS and default dev/prod config are too open.

## Product Decisions Recommended

1. Keep local-first SQLite as the MVP default.
2. Treat `docs/SPEC_V2.md` and the V2 child specs as the new planning layer, while old docs remain historical context until explicitly promoted.
3. Stop adding new features until P0/P1 QA blockers are fixed.
4. Make manual replay correctness the non-negotiable foundation.
5. Use one shared broker/accounting engine for manual replay and automated backtest.
6. Replace YAML condition strings plus `eval()` with a safe rule AST/DSL.
7. Implement scanner and regime features as Professional v1, not MVP.
8. Do not build a fancy code editor before Strategy Lab can run deterministic, safe, tested strategies.

