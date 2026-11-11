# SUMI TRADING WORKSTATION — NEXT SESSION BRIEF (`SESSION R07 / WAVE 6`)

- **Prepared By:** `SESSION-R06-20261111T194303+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Date:** `2026-11-12`
- **Target Milestone:** `Milestone R07` — Wave 6 Causal Bar Replay & Paper Trading Execution Engine (`DEF-001` Zero-Lookahead Multi-Timeframe Replay + Simulated Order Book & Position PnL)

---

## 1. Self-Start Entry Protocol (Zero Copy-Paste Needed)

Any successor Orchestrator session starting in `/Users/mizuhara/workspace/sumi` must first read:

1. [`docs/operations/SUMI_ORCHESTRATION_CHARTER.md`](operations/SUMI_ORCHESTRATION_CHARTER.md) — Role authority, subagent protocol, evidence classification (`VERIFIED` / `INFERRED` / `UNVERIFIED` / `BLOCKED`), per-batch Git commit policy (`DEC-R04-001`), and mandatory Vietnamese communication with the Product Owner.
2. [`docs/EXECUTION_STATE.md`](EXECUTION_STATE.md) — Current baseline (`IMPLEMENTATION_PASS` on Waves 1–5 / Milestone R06).
3. [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) & [`docs/DECISION_LOG.md`](DECISION_LOG.md) — Modular `src/data/`, `src/viewport/`, `src/indicators/`, `src/drawings/`, `MultiPaneLayout`, `OffscreenWgpuHarness`, and `DECISION-RENDERER-002` (`< 4.0 ms` CPU frame budget).
4. [`docs/research/RESEARCH_TO_RUST_ROADMAP.md`](research/RESEARCH_TO_RUST_ROADMAP.md) & [`docs/research/DATA_LOADER_AND_TIMEFRAME_SPEC.md`](research/DATA_LOADER_AND_TIMEFRAME_SPEC.md) — Wave 6 specifications and `aggregate_ohlcv_up_to` causal primitive.

---

## 2. Current Baseline Verified in R06

- **Build, WASM & Tests:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings`, and `cargo test` (`72/72` unit + property + headless multi-pane UI + Critic R03/R04/R05/R06 regression tests) pass with `0` warnings and `0` errors; `Cargo.toml` and `Cargo.lock` have `0` new dependencies.
- **Shipped Subsystems:**
  - `src/data/` (Hinnant O(1) UTC calendar, RFC-4180 CSV & recursive-descent JSON kline/envelope parsers at `103.7–191.1 MiB/s`, `CafeF`/`SPY` archive fixtures, `1m..1W` upward lattice, `aggregate_ohlcv` + causal `aggregate_ohlcv_up_to`)
  - `src/viewport/` (`TimeScale`, `PriceScale` + `sumi_oscillator`, `PaneViewport`, `KineticAnimation`, `candle_metrics`)
  - `src/indicators/` (`SMA`, `EMA`, `BollingerBands`, `RSI`, `MACD`, `ATR`, `IndicatorCache`, `SeriesBlockIndex`, conflated envelopes)
  - `src/drawings/` (`HorizontalLine`, `TrendLine`, $360^\circ$ `Ray`, `Rectangle`, `FibonacciRetracement`, `DrawingAnchor::timestamp`, `DrawingStore::remap_timeframe`, `ClipEdge` Liang-Barsky, LWC hit-test, OHLC magnet, 64-step Undo/Redo, `serde` persistence)
  - `src/chart.rs`, `src/app.rs` & `src/main.rs` (`MultiPaneLayout`, adaptive UTC time axis, per-instrument `drawings_by_instrument`, CSV/JSON import bar, and headless `OffscreenWgpuHarness` Retina Metal verification)
  - `examples/viewport_bench.rs` (`0.185 ms` intraday standard, `3.761 ms` `100K` `1m` 4-pane + 6-indicator + 25-drawing full-fit)

---

## 3. Recommended R07 Execution Plan (`Wave 6 — Causal Bar Replay & Paper Trading`)

### Workstream 1 — Research Gate (`DEC-005` Compliance)
1. Execute live REA MCP JS/TS analysis on `ARCHIVE/frontend/src/features/replay/*` (and any paper-trading/backtest modules in `ARCHIVE/frontend` + `ARCHIVE/backend`) plus `LWC v5.2.0` price-line / series-marker views.
2. Author `docs/research/BAR_REPLAY_AND_PAPER_TRADING_SPEC.md` defining:
   - Strict `DEF-001` causal multi-timeframe replay state machine using `aggregate_ohlcv_up_to(&base_candles, base_tf, active_tf, cutoff_timestamp)` so incomplete higher-timeframe bars reflect only base candles `<= T_replay` (zero future high/low/close leakage).
   - Deterministic OHLC bar-fill rules for `Market`, `Limit`, `Stop`, `StopLoss`, and `TakeProfit` orders (gap-open fill handling, intrabar pessimistic/deterministic tie-breaking, commission/slippage basis points, realized/unrealized PnL, win-rate & drawdown).

### Workstream 2 — Pure `f64` WASM-Safe `src/replay/` Implementation
1. Implement `wasm32-unknown-unknown`-compatible `src/replay/` (`controller.rs`, `orders.rs`, `portfolio.rs`, `tests.rs`) with zero new crates in `Cargo.toml`.
2. Guarantee causal indicator recalculation (`IndicatorCache` synced strictly to the causally truncated/aggregated replay slice) and future-drawing visibility rules.

### Workstream 3 — Workstation UI Integration, Critic, Audit & Clean Commit
1. Integrate replay transport controls (`Jump to Bar`, `Play/Pause`, `Step +1`, speed `1x..20x`, `Exit Replay`), paper-trading ticket (`Buy Market/Limit/Stop`, `Sell`, `Flatten`, `SL/TP` lines, equity/trade blotter HUD), and on-chart execution arrows + position price lines.
2. Run `--release` `--r07` benchmark + offscreen `SUMI_CAPTURE_BATCH=r07` Retina Metal captures, remediate all `sumi-critic` findings, verify via `sumi-auditor`, and create one clean commit on `master` per `DEC-R04-001`.

---

## 4. Standing Constraints & Product Owner Gates

- **Language:** Communicate 100% in **Vietnamese (`tiếng Việt`)** with the Product Owner.
- **Git & File Safety (`DEC-R04-001`):** Create one clean commit at the end of each verified batch/milestone; never run `git reset`, `git clean`, `git push`, or delete `ARCHIVE/`, `ViewportMapping`, or prior `docs/evidence/*`.
- **Authorized Targets Only (`DEC-006`):** Never inspect or interact with TradingView Desktop.
