# SUMI TRADING WORKSTATION — NEXT SESSION BRIEF (`SESSION R15 / WAVE 14`)

- **Prepared By:** `SESSION-R14-20261115T191000+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Date:** `2026-11-15`
- **Target Milestone (Awaiting PO Approval):** `Milestone R15` — Wave 14 Explainable Signal Catalog & Causal Confluence Inspector, Interactive Drawing Property Inspector, & Monte Carlo Bootstrap Tearsheet Risk Envelopes

---

## 1. Self-Start Entry Protocol (Zero Copy-Paste Needed)

Any successor Orchestrator session starting in `/Users/mizuhara/workspace/sumi` must first read:

1. [`docs/operations/SUMI_ORCHESTRATION_CHARTER.md`](operations/SUMI_ORCHESTRATION_CHARTER.md) — Role authority, subagent protocol, evidence classification (`VERIFIED` / `INFERRED` / `UNVERIFIED` / `BLOCKED`), per-batch Git commit policy (`DEC-R04-001`), and mandatory Vietnamese communication with the Product Owner.
2. [`docs/EXECUTION_STATE.md`](EXECUTION_STATE.md) — Current baseline (`IMPLEMENTATION_PASS` across Waves 1–13 / Milestones R03–R14).
3. [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) & [`docs/DECISION_LOG.md`](DECISION_LOG.md) — Modular `src/data/`, `src/viewport/` (including `multi_chart.rs`, `workspace_template.rs`, & `hotkeys.rs`), `src/indicators/`, `src/drawings/`, `src/replay/`, `src/trading/` (including `ladder.rs`), `src/scanner/` (including `breadth.rs`), `src/alerts/`, `src/backtest/`, `src/journal/`, multi-pane `MultiPaneLayout` + synchronized multi-chart grid, `PlotDataLayerMemo`, `OffscreenWgpuHarness`, and `DECISION-RENDERER-002` (`< 4.0 ms` CPU frame budget).
4. [`docs/research/RESEARCH_TO_RUST_ROADMAP.md`](research/RESEARCH_TO_RUST_ROADMAP.md) & [`docs/research/EXECUTION_LADDER_HOTKEYS_AND_AXIS_POLISH_SPEC.md`](research/EXECUTION_LADDER_HOTKEYS_AND_AXIS_POLISH_SPEC.md) — Shipped Wave 1–13 specifications and proposed Wave 14 scope.

---

## 2. Current Baseline Verified in R14

- **Build, WASM & Tests:** `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --lib --target wasm32-unknown-unknown -- -D warnings`, and `cargo test` (`204/204` unit + property + headless multi-pane/multi-chart/DOM/hotkey UI + Critic R03–R14 regression tests) pass with `0` warnings and `0` errors; `Cargo.toml` and `Cargo.lock` have `0` new dependencies.
- **Shipped Subsystems (`R03–R14`):**
  - `src/data/` (Hinnant O(1) UTC calendar, RFC-4180 CSV & recursive-descent JSON kline/envelope parsers at `103.7–191.1 MiB/s`, `Vn30Basket` + `CafeF`/`SPY`/`BTCUSDT` fixtures, tail-coverage-scored benchmark lookup, `1m..1W` upward lattice, `aggregate_ohlcv` + causal `aggregate_ohlcv_up_to`)
  - `src/viewport/` (`TimeScale`, `PriceScale` + `PriceAxisFormatMode::CompactVolume` + **R14** `axis_edge_pad_px` / `scale_decimals_for_range_and_step` / `format_price_for_tick_density`, `PaneViewport`, `KineticAnimation`, `candle_metrics`, `MultiChartLayoutKind` + `MultiChartSyncConfig` + `causal_closed_prefix_len`, `WorkspaceTemplate` `schema_version = 1` strict `RON`/`JSON` codecs + **R14** `dock_layout` + `hotkey_profile` + `hotkey_base_preset` + `6` built-in templates, & **R14** `HotkeyProfile` conflict-free `17`-action shortcut engine)
  - `src/indicators/` (`SMA`, `EMA`, `BollingerBands`, `RSI`, `MACD`, `ATR`, `IndicatorCache`, `SeriesBlockIndex` & `CandleBlockIndex`, `VRVP`/`FRVP`, Welford Session/Anchored `VWAP` $\pm 1\sigma/\pm 2\sigma$, `DEF-001` causal `RS vs VNINDEX`, `Ichimoku Cloud`, `CVD`, & `Session TPO / Market Profile` + `HistoricalNakedPoc`)
  - `src/trading/` (`TradingSession`, symmetric `Long`/`Short` `Market`/`Limit`/`Stop`, `SL`/`TP` brackets, Gap-Open + 4-waypoint OHLC matching, `1m == 15m` parity, `rewind_to`, & **R14** `ladder.rs` causal `OHLCV_SYNTH_DOM_PROXY` 21-row footprint/DOM execution ladder with `HOSE`/`HNX`/`UPCOM`/`VN30F` dynamic ticks, daily bands, short guards & risk-budget sizing)
  - `src/drawings/`, `src/replay/`, `src/scanner/` (incl. `breadth.rs`), `src/alerts/`, `src/backtest/` (`walk_forward.rs` WFO + Portfolio), & `src/journal/` (all Pure `f64`/`i64` WASM-safe domain engines)
  - `src/chart.rs`, `src/app.rs` & `src/main.rs` (Synchronized multi-chart slots, batched DOM Ladder dock + `[Keys]` Hotkey Profiles modal, pill/badge collision avoidance & time-cursor occlusion, and `--r14` holding all workloads `< 4.0 ms`: `0.378–1.391 ms` mean / `p99 <= 2.198 ms`, `"overall_pass": true`)

---

## 3. Recommended R15 Plan (`Wave 14 — Explainable Signal Catalog, Interactive Drawing Inspector & Monte Carlo Tearsheet`)

### Workstream 1 — Research Gate (`DEC-005` Compliance)
1. Execute live REA MCP JS/TS analysis on `ARCHIVE/frontend/src/components/signals/{SignalCatalog,SignalCatalogModal,SignalInspector,SignalExplanationInspector}.tsx`, `ARCHIVE/frontend/src/components/chart/DrawingInspector.tsx`, and `ARCHIVE/frontend/src/components/analytics/{EquityChart,MetricResultValue}.tsx` + `LWC v5.2.0` histogram/baseline area band primitives; author the R15 specification in `docs/research/`.

### Workstream 2 — Wave 14 Domain & UI Enhancements
1. **Explainable Signal Catalog & Causal Confluence Inspector (`src/scanner/`, `src/app.rs`):** Rich catalog browser, causal feature attribution breakdown per `HistoricalSignalHit` (`DEF-001`), multi-signal confluence filter, and click-to-jump chart inspection.
2. **Interactive Drawing Property Inspector (`src/drawings/`, `src/app.rs`):** Live color/opacity/line-width/dash/extend-right/price-level/anchor-coordinate editing with full 64-step Undo/Redo command history.
3. **Deterministic Monte Carlo Bootstrap Tearsheet Risk Envelopes (`src/backtest/`, `src/chart.rs`, `src/app.rs`):** Seeded (`SplitMix64`) WASM-safe trade-return bootstrap computing p5/p50/p95 terminal equity, max-drawdown VaR/CVaR, and fan-band overlay inside `OscillatorPaneId::Equity` (plus resolving non-blocking R14 Minor polish: passing `equity_base_val` to `paint_equity_baseline_pane` badge formatting and optionally widening `main_occluding_ys` to include working order pills).

### Workstream 3 — `--r15` Release Benchmark, Offscreen Metal Captures, Critic, Audit & Clean Commit
1. Pass all 4 Cargo gates (`fmt`, native + `wasm32` `clippy -D warnings`, `cargo test`), run `--release` `--r15` (`< 4.0 ms` CPU frame budget) + `SUMI_CAPTURE_BATCH=r15`, pass `sumi-critic` (`CRITIC_PASS`) and `sumi-auditor` (`AUDIT_PASS`), and commit cleanly on `master` per `DEC-R04-001`.

---

## 4. Standing Constraints & Product Owner Gates

- **Product Owner Authorization Gate:** Wait for explicit Product Owner approval before starting Milestone R15 implementation.
- **Language:** Communicate 100% in **Vietnamese (`tiếng Việt`)** with the Product Owner.
- **Git & File Safety (`DEC-R04-001`):** Never touch `Cargo.toml`, `Cargo.lock`, `src/model.rs`, `ARCHIVE/`, or R01–R14 `docs/evidence/*`; never run `git reset`, `git clean`, `git push`, or `git tag`.
- **Authorized Targets Only (`DEC-006`):** Never inspect or interact with TradingView Desktop.
