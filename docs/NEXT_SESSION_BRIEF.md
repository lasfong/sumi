# SUMI TRADING WORKSTATION — NEXT SESSION BRIEF (`SESSION R09 / WAVE 8`)

- **Prepared By:** `SESSION-R08-20261112T140031+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Date:** `2026-11-12`
- **Target Milestone (Awaiting PO Approval):** `Milestone R09` — Wave 8 Deterministic Strategy Backtest Lab, Rule/Signal Strategy Runner, Synchronized Equity/Drawdown Sub-Pane & Quantitative Performance Tearsheet

---

## 1. Self-Start Entry Protocol (Zero Copy-Paste Needed)

Any successor Orchestrator session starting in `/Users/mizuhara/workspace/sumi` must first read:

1. [`docs/operations/SUMI_ORCHESTRATION_CHARTER.md`](operations/SUMI_ORCHESTRATION_CHARTER.md) — Role authority, subagent protocol, evidence classification (`VERIFIED` / `INFERRED` / `UNVERIFIED` / `BLOCKED`), per-batch Git commit policy (`DEC-R04-001`), and mandatory Vietnamese communication with the Product Owner.
2. [`docs/EXECUTION_STATE.md`](EXECUTION_STATE.md) — Current baseline (`IMPLEMENTATION_PASS` across Waves 1–7 / Milestones R03–R08).
3. [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) & [`docs/DECISION_LOG.md`](DECISION_LOG.md) — Modular `src/data/`, `src/viewport/`, `src/indicators/`, `src/drawings/`, `src/replay/`, `src/trading/`, `src/scanner/`, `src/alerts/`, `MultiPaneLayout`, `OffscreenWgpuHarness`, and `DECISION-RENDERER-002` (`< 4.0 ms` CPU frame budget).
4. [`docs/research/RESEARCH_TO_RUST_ROADMAP.md`](research/RESEARCH_TO_RUST_ROADMAP.md) & [`docs/research/SCANNER_ALERTS_AND_COMMAND_PALETTE_SPEC.md`](research/SCANNER_ALERTS_AND_COMMAND_PALETTE_SPEC.md) — Shipped Wave 1–7 specifications and proposed Wave 8 expansion.

---

## 2. Current Baseline Verified in R08

- **Build, WASM & Tests:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings`, and `cargo test` (`105/105` unit + property + headless multi-pane UI + Critic R03/R04/R05/R06/R07/R08 regression tests) pass with `0` warnings and `0` errors; `Cargo.toml` and `Cargo.lock` have `0` new dependencies.
- **Shipped Subsystems:**
  - `src/data/` (Hinnant O(1) UTC calendar, RFC-4180 CSV & recursive-descent JSON kline/envelope parsers at `103.7–191.1 MiB/s`, `Vn30Basket` + `CafeF`/`SPY`/`BTCUSDT` fixtures, `1m..1W` upward lattice, `aggregate_ohlcv` + causal `aggregate_ohlcv_up_to`)
  - `src/viewport/` (`TimeScale`, `PriceScale` + `sumi_oscillator`, `PaneViewport`, `KineticAnimation`, `candle_metrics`)
  - `src/indicators/` (`SMA`, `EMA`, `BollingerBands`, `RSI`, `MACD`, `ATR`, `IndicatorCache`, `SeriesBlockIndex`, conflated envelopes)
  - `src/drawings/` (`HorizontalLine`, `TrendLine`, $360^\circ$ `Ray`, `Rectangle`, `FibonacciRetracement`, `DrawingAnchor::timestamp`, `Drawing::created_at_timestamp`, `DrawingStore::remap_timeframe`, causal replay visibility, 64-step Undo/Redo, `serde` persistence)
  - `src/replay/` (`ReplayController`, `DEF-001` zero-lookahead prefix materialization, `ActiveBar`/`BaseBar` stepping, `1x..20x` playback)
  - `src/trading/` (`TradingSession`, `Long`/`Short` `Market`/`Limit`/`Stop` orders, `SL`/`TP` brackets, unified Gap-Open + 4-waypoint OHLC matching, `1m == 15m` parity, fee conservation, command-log `rewind_to`)
  - `src/scanner/` (13-symbol Watchlist Universe + `custom_peer_feeds`, single-pass `1.799 ms` Watchlist + Signal scan, cross-symbol `DEF-001` cutoff, bucket-close `BlindPractice` / `SignalReview`)
  - `src/alerts/` (`AlertEngine`, `PriceLevel` 4-waypoint/@Mark, `RSI(14)`, `MACD(12,26,9)`, `DrawingLine` `HorizontalLine`/`TrendLine`/$360^\circ$ `Ray` relative-offset crossings & causal Replay rewind)
  - `src/chart.rs`, `src/app.rs` & `src/main.rs` (`MultiPaneLayout`, Left Watchlist/Scanner Dock, Right Paper Trading & Alerts Tabs, Floating `Cmd+K` Command Palette, `egui::Mesh` batching, and headless `OffscreenWgpuHarness` Retina Metal verification)
  - `examples/viewport_bench.rs` (`0.182 ms` active `95K` intraday replay, `0.407 ms` full workstation UI, `0.296 ms` `Cmd+K` modal, `3.067 ms` `95K/100K` `1m` full-fit)

---

## 3. Recommended R09 Execution Plan (`Wave 8 — Deterministic Strategy Backtest Lab, Equity Curve Sub-Pane & Performance Tearsheet`)

### Workstream 1 — Research Gate (`DEC-005` Compliance)
1. Execute live REA MCP JS/TS analysis on `ARCHIVE/frontend/src/pages/StrategyLabPage.tsx`, `ARCHIVE/frontend/src/api/backtestApi.ts` (or related strategy/analytics modules), and `LWC v5.2.0` area/baseline series views to catalog legacy Sumi's backtest rules, lookahead pitfalls, and equity-curve rendering requirements.
2. Author `docs/research/STRATEGY_BACKTEST_LAB_SPEC.md` defining pure-Rust WASM-safe causal strategy execution (zero bar-close lookahead, next-bar open / 4-waypoint limit-stop fills via `TradingSession`), multi-symbol batch runs, and quantitative tearsheet metrics.

### Workstream 2 — Pure `f64` WASM-Safe `src/backtest/` Implementation
1. Implement `src/backtest/` composing indicator/regime/price-action entry & exit presets (`Trend Following`, `Mean Reversion RSI/BB`, `MACD Momentum`, `Breakout + ATR Stop`), fixed-fractional / ATR risk sizing, and deterministic execution over `100,000` bars in `< 15 ms`.
2. Compute institutional tearsheet metrics (`Total Return`, `CAGR`, `Max Drawdown % & Duration`, `Sharpe`, `Sortino`, `Calmar`, `Win Rate`, `Profit Factor`, `Expectancy`, `Avg R-Multiple`, `Exposure %`) plus bar-aligned `equity_curve` & `underwater_drawdown_pct` series.

### Workstream 3 — Workstation UI (`Strategy Lab` Dock, Synchronized `Equity/DD` Sub-Pane & Trade Inspector), Critic, Audit & Clean Commit
1. Add synchronized `Equity / Drawdown` sub-pane (sharing `TimeScale` pan/zoom/crosshair with Main OHLCV), clickable trade-to-chart jump & Replay inspection, Strategy Lab parameter runner + multi-symbol comparison table, and `Cmd+K` strategy actions.
2. Run `--release` `--r09` benchmark (`< 4.0 ms` CPU frame budget) + offscreen `SUMI_CAPTURE_BATCH=r09` Retina Metal captures, remediate all `sumi-critic` findings, verify via `sumi-auditor`, and create one clean commit on `master` per `DEC-R04-001`.

---

## 4. Standing Constraints & Product Owner Gates

- **Product Owner Authorization Gate:** Wait for explicit Product Owner approval before starting Milestone R09 implementation.
- **Language:** Communicate 100% in **Vietnamese (`tiếng Việt`)** with the Product Owner.
- **Git & File Safety (`DEC-R04-001`):** Create one clean commit at the end of each verified batch/milestone; never run `git reset`, `git clean`, `git push`, or delete `ARCHIVE/`, `ViewportMapping`, or prior `docs/evidence/*`.
- **Authorized Targets Only (`DEC-006`):** Never inspect or interact with TradingView Desktop.
