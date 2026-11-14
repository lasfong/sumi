# SUMI TRADING WORKSTATION — NEXT SESSION BRIEF (`SESSION R10 / WAVE 9`)

- **Prepared By:** `SESSION-R09-20261114T091018+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Date:** `2026-11-14`
- **Target Milestone (Awaiting PO Approval):** `Milestone R10` — Wave 9 Vietnam Exchange Microstructure & Trade Risk Planner (`HOSE/HNX/UPCOM` Tick/Ceiling-Floor/T+2.5 Rules), Structured Trade Journal & Rolling Walk-Forward Portfolio Analytics

---

## 1. Self-Start Entry Protocol (Zero Copy-Paste Needed)

Any successor Orchestrator session starting in `/Users/mizuhara/workspace/sumi` must first read:

1. [`docs/operations/SUMI_ORCHESTRATION_CHARTER.md`](operations/SUMI_ORCHESTRATION_CHARTER.md) — Role authority, subagent protocol, evidence classification (`VERIFIED` / `INFERRED` / `UNVERIFIED` / `BLOCKED`), per-batch Git commit policy (`DEC-R04-001`), and mandatory Vietnamese communication with the Product Owner.
2. [`docs/EXECUTION_STATE.md`](EXECUTION_STATE.md) — Current baseline (`IMPLEMENTATION_PASS` across Waves 1–8 / Milestones R03–R09).
3. [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) & [`docs/DECISION_LOG.md`](DECISION_LOG.md) — Modular `src/data/`, `src/viewport/`, `src/indicators/`, `src/drawings/`, `src/replay/`, `src/trading/`, `src/scanner/`, `src/alerts/`, `src/backtest/`, 5-pane `MultiPaneLayout`, `OffscreenWgpuHarness`, and `DECISION-RENDERER-002` (`< 4.0 ms` CPU frame budget).
4. [`docs/research/RESEARCH_TO_RUST_ROADMAP.md`](research/RESEARCH_TO_RUST_ROADMAP.md) & [`docs/research/STRATEGY_BACKTEST_LAB_SPEC.md`](research/STRATEGY_BACKTEST_LAB_SPEC.md) — Shipped Wave 1–8 specifications and proposed Wave 9 expansion.

---

## 2. Current Baseline Verified in R09

- **Build, WASM & Tests:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings`, and `cargo test` (`122/122` unit + property + headless multi-pane UI + Critic R03/R04/R05/R06/R07/R08/R09 regression tests) pass with `0` warnings and `0` errors; `Cargo.toml` and `Cargo.lock` have `0` new dependencies.
- **Shipped Subsystems:**
  - `src/data/` (Hinnant O(1) UTC calendar, RFC-4180 CSV & recursive-descent JSON kline/envelope parsers at `103.7–191.1 MiB/s`, `Vn30Basket` + `CafeF`/`SPY`/`BTCUSDT` fixtures, `1m..1W` upward lattice, `aggregate_ohlcv` + causal `aggregate_ohlcv_up_to`)
  - `src/viewport/` (`TimeScale`, `PriceScale` + `sumi_oscillator`, `PaneViewport`, `KineticAnimation`, `candle_metrics`)
  - `src/indicators/` (`SMA`, `EMA`, `BollingerBands`, `RSI`, `MACD`, `ATR`, `IndicatorCache`, `SeriesBlockIndex`, conflated envelopes)
  - `src/drawings/` (`HorizontalLine`, `TrendLine`, $360^\circ$ `Ray`, `Rectangle`, `FibonacciRetracement`, `DrawingAnchor::timestamp`, `Drawing::created_at_timestamp`, `DrawingStore::remap_timeframe`, causal replay visibility, 64-step Undo/Redo, `serde` persistence)
  - `src/replay/` (`ReplayController`, `DEF-001` zero-lookahead prefix materialization, `ActiveBar`/`BaseBar` stepping, `1x..20x` playback)
  - `src/trading/` (`TradingSession`, `Long`/`Short` `Market`/`Limit`/`Stop` orders, `SL`/`TP` brackets, unified Gap-Open + 4-waypoint OHLC matching, `1m == 15m` parity, fee conservation, command-log `rewind_to`)
  - `src/scanner/` (13-symbol Watchlist Universe + `custom_peer_feeds`, single-pass `1.799 ms` Watchlist + Signal scan, cross-symbol `DEF-001` cutoff, bucket-close `BlindPractice` / `SignalReview`)
  - `src/alerts/` (`AlertEngine`, `PriceLevel` 4-waypoint/@Mark, `RSI(14)`, `MACD(12,26,9)`, `DrawingLine` `HorizontalLine`/`TrendLine`/$360^\circ$ `Ray` relative-offset crossings & causal Replay rewind)
  - `src/backtest/` (`6` composable strategy presets, causal `Open[T+1]` fills, pessimistic `SL`-before-`TP`, `MarginLiquidation`, post-close `ATR` trail, continuous single-pass `IS/OOS` partition, `6.318–7.832 ms` `100K` backtest, `77.29 µs` `12`-grid sweep, `639.92 µs` `13`-symbol universe battle)
  - `src/chart.rs`, `src/app.rs` & `src/main.rs` (5-pane `MultiPaneLayout` with synchronized `Equity / Drawdown` baseline `egui::Mesh`, $O(\log T)$ Trade Connectors, Left Watchlist/Scanner Dock, Right Paper Trading / Alerts / Strategy Lab Tabs, Floating `Cmd+K` Command Palette, and headless `OffscreenWgpuHarness` Retina Metal verification)
  - `examples/viewport_bench.rs` (`0.190 ms` active `95K` 5-pane intraday, `0.412 ms` full workstation UI, `1.355 ms` `95K/100K` `1m` full-fit)

---

## 3. Recommended R10 Execution Plan (`Wave 9 — Vietnam Market Microstructure & Risk Planner, Structured Trade Journal & Rolling Walk-Forward Portfolio Analytics`)

### Workstream 1 — Research Gate (`DEC-005` Compliance)
1. Execute live REA MCP JS/TS analysis on the final unported legacy Sumi workflows — `ARCHIVE/frontend/src/pages/JournalPage.tsx`, `ARCHIVE/frontend/src/pages/DashboardPage.tsx`, and related API/component modules, alongside `ARCHIVE/backend/app/domain/{market_rules.py,trade_planning.py,accounting.py}` and `LWC v5.2.0` risk-reward / box-overlay primitives.
2. Author `docs/research/RISK_PLANNER_JOURNAL_AND_PORTFOLIO_SPEC.md` defining exchange-accurate Vietnam (`HOSE`/`HNX`/`UPCOM`/`VN30F`) & global (`US`/`Crypto`) microstructure rules, visual `Long/Short Risk-Reward` chart planner boxes, structured post-trade journaling, and rolling Walk-Forward / multi-symbol portfolio simulation.

### Workstream 2 — Pure `f64`/`i64` WASM-Safe `src/journal/` & Microstructure Risk Engine
1. Implement `HOSE`/`HNX`/`UPCOM` tick-size ladders (`10 / 50 / 100 VND`), daily reference/ceiling/floor (`±7% / ±10% / ±15%`) clamping, `T+2` share / cash settlement ledger tracking, and interactive `Risk-Reward Position Sizer` (`Entry`, `Stop`, `Target`, `Account Risk %` -> exact board-lot shares, fee+tax breakeven, and `R-Multiple`).
2. Implement structured `TradeJournalStore` (linking closed Paper Trading & Backtest trades with setup tags, mistake taxonomy, conviction ratings, causal Replay review links, and deterministic CSV/Markdown export) plus rolling Walk-Forward (`K`-fold anchored/rolling `IS -> OOS` windows) & multi-symbol equal/volatility-weighted portfolio equity curves.

### Workstream 3 — Workstation UI (`Risk-Reward` Chart Tool, `Journal & Portfolio` Workspace), Critic, Audit & Clean Commit
1. Add on-chart visual `Long/Short Risk-Reward` position-box drawing/overlay (with instant 1-click `Send to Order Ticket` & `Log to Journal`), `Journal & Portfolio` workstation panel/dock, and `Cmd+K` risk/journal/export commands.
2. Run `--release` `--r10` benchmark (`< 4.0 ms` CPU frame budget) + offscreen `SUMI_CAPTURE_BATCH=r10` Retina Metal captures, remediate all `sumi-critic` findings, verify via `sumi-auditor`, and create one clean commit on `master` per `DEC-R04-001`.

---

## 4. Standing Constraints & Product Owner Gates

- **Product Owner Authorization Gate:** Wait for explicit Product Owner approval before starting Milestone R10 implementation.
- **Language:** Communicate 100% in **Vietnamese (`tiếng Việt`)** with the Product Owner.
- **Git & File Safety (`DEC-R04-001`):** Create one clean commit at the end of each verified batch/milestone; never run `git reset`, `git clean`, `git push`, or delete `ARCHIVE/`, `ViewportMapping`, or prior `docs/evidence/*`.
- **Authorized Targets Only (`DEC-006`):** Never inspect or interact with TradingView Desktop.
