# SUMI TRADING WORKSTATION — NEXT SESSION BRIEF (`SESSION R08 / WAVE 7`)

- **Prepared By:** `SESSION-R07-20261112T093742+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Date:** `2026-11-12`
- **Target Milestone (Awaiting PO Approval):** `Milestone R08` — Wave 7 Multi-Symbol Watchlist & Technical Scanner Dock, Causal Price/Indicator Alert Engine & `Cmd+K` Command Palette

---

## 1. Self-Start Entry Protocol (Zero Copy-Paste Needed)

Any successor Orchestrator session starting in `/Users/mizuhara/workspace/sumi` must first read:

1. [`docs/operations/SUMI_ORCHESTRATION_CHARTER.md`](operations/SUMI_ORCHESTRATION_CHARTER.md) — Role authority, subagent protocol, evidence classification (`VERIFIED` / `INFERRED` / `UNVERIFIED` / `BLOCKED`), per-batch Git commit policy (`DEC-R04-001`), and mandatory Vietnamese communication with the Product Owner.
2. [`docs/EXECUTION_STATE.md`](EXECUTION_STATE.md) — Current baseline (`IMPLEMENTATION_PASS` across Waves 1–6 / Milestones R03–R07).
3. [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) & [`docs/DECISION_LOG.md`](DECISION_LOG.md) — Modular `src/data/`, `src/viewport/`, `src/indicators/`, `src/drawings/`, `src/replay/`, `src/trading/`, `MultiPaneLayout`, `OffscreenWgpuHarness`, and `DECISION-RENDERER-002` (`< 4.0 ms` CPU frame budget).
4. [`docs/research/RESEARCH_TO_RUST_ROADMAP.md`](research/RESEARCH_TO_RUST_ROADMAP.md) & [`docs/research/BAR_REPLAY_AND_PAPER_TRADING_SPEC.md`](research/BAR_REPLAY_AND_PAPER_TRADING_SPEC.md) — Shipped Wave 1–6 specifications and proposed Wave 7 expansion.

---

## 2. Current Baseline Verified in R07

- **Build, WASM & Tests:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings`, and `cargo test` (`91/91` unit + property + headless multi-pane UI + Critic R03/R04/R05/R06/R07 regression tests) pass with `0` warnings and `0` errors; `Cargo.toml` and `Cargo.lock` have `0` new dependencies.
- **Shipped Subsystems:**
  - `src/data/` (Hinnant O(1) UTC calendar, RFC-4180 CSV & recursive-descent JSON kline/envelope parsers at `103.7–191.1 MiB/s`, `CafeF`/`SPY` archive fixtures, `1m..1W` upward lattice, `aggregate_ohlcv` + causal `aggregate_ohlcv_up_to`)
  - `src/viewport/` (`TimeScale`, `PriceScale` + `sumi_oscillator`, `PaneViewport`, `KineticAnimation`, `candle_metrics`)
  - `src/indicators/` (`SMA`, `EMA`, `BollingerBands`, `RSI`, `MACD`, `ATR`, `IndicatorCache`, `SeriesBlockIndex`, conflated envelopes)
  - `src/drawings/` (`HorizontalLine`, `TrendLine`, $360^\circ$ `Ray`, `Rectangle`, `FibonacciRetracement`, `DrawingAnchor::timestamp`, `Drawing::created_at_timestamp`, `DrawingStore::remap_timeframe`, causal replay visibility, 64-step Undo/Redo, `serde` persistence)
  - `src/replay/` (`ReplayController`, `DEF-001` zero-lookahead prefix materialization, `ActiveBar`/`BaseBar` stepping, `1x..20x` playback)
  - `src/trading/` (`TradingSession`, `Long`/`Short` `Market`/`Limit`/`Stop` orders, `SL`/`TP` brackets, unified Gap-Open + 4-waypoint OHLC matching, `1m == 15m` parity, fee conservation, command-log `rewind_to`)
  - `src/chart.rs`, `src/app.rs` & `src/main.rs` (`MultiPaneLayout`, Replay Transport Dock, Cut Scrubber, Paper Trading Ticket/Blotter, `egui::Mesh` dashed lines & fill arrows, and headless `OffscreenWgpuHarness` Retina Metal verification)
  - `examples/viewport_bench.rs` (`0.174 ms` active `95K` intraday replay, `0.291 ms` full workstation UI, `3.923 ms` `95K/100K` `1m` 4-pane + 6-indicator + 25-drawing + Replay + Trading full-fit)

---

## 3. Recommended R08 Execution Plan (`Wave 7 — Multi-Symbol Scanner, Alert Engine & Command Palette`)

### Workstream 1 — Research Gate (`DEC-005` Compliance)
1. Execute live REA MCP JS/TS analysis on `ARCHIVE/frontend/src/pages/ScannerPage.tsx`, `ARCHIVE/frontend/src/pages/StrategyLabPage.tsx`, `ARCHIVE/frontend/src/types/signals.ts`, and `ARCHIVE/backend/app/domain/signals/*` to catalog legacy Sumi's multi-symbol scanner rules, regime/signal definitions, and UX bottlenecks.
2. Author `docs/research/SCANNER_ALERTS_AND_COMMAND_PALETTE_SPEC.md` defining pure-Rust WASM-safe multi-symbol scanning, causal alert evaluation (both live bar-close/intrabar and during Bar Replay), and instant `Cmd+K` workstation navigation.

### Workstream 2 — Pure `f64` WASM-Safe `src/scanner/` & `src/alerts/` Implementation
1. Implement `src/scanner/` evaluating multi-symbol snapshots (`Last`, `%Chg`, `Volume`, `RSI(14)`, `MACD` cross, `BB` squeeze/breakout, `SMA/EMA` trend alignment) across all symbols in bundled/imported CSV/JSON catalogs in `< 2 ms`.
2. Implement `src/alerts/` supporting price-cross, indicator-threshold (`RSI < 30` / `> 70`, `MACD` zero/signal cross), and horizontal/trendline ray-cross alerts with causal replay integration (rewinding replay rewinds alert trigger state cleanly).

### Workstream 3 — Workstation UI (`Watchlist/Scanner Dock`, `Alert Manager`, `Cmd+K` Palette), Critic, Audit & Clean Commit
1. Integrate collapsible Watchlist/Scanner Dock (one-click symbol jump preserving per-instrument drawings/replay/trading state), on-chart alert lines + trigger toast/log, and keyboard-first `Cmd+K` Command Palette.
2. Run `--release` `--r08` benchmark + offscreen `SUMI_CAPTURE_BATCH=r08` Retina Metal captures, remediate all `sumi-critic` findings, verify via `sumi-auditor`, and create one clean commit on `master` per `DEC-R04-001`.

---

## 4. Standing Constraints & Product Owner Gates

- **Product Owner Authorization Gate:** Wait for explicit Product Owner approval before starting Milestone R08 implementation.
- **Language:** Communicate 100% in **Vietnamese (`tiếng Việt`)** with the Product Owner.
- **Git & File Safety (`DEC-R04-001`):** Create one clean commit at the end of each verified batch/milestone; never run `git reset`, `git clean`, `git push`, or delete `ARCHIVE/`, `ViewportMapping`, or prior `docs/evidence/*`.
- **Authorized Targets Only (`DEC-006`):** Never inspect or interact with TradingView Desktop.
