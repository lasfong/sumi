# SUMI TRADING WORKSTATION — NEXT SESSION BRIEF (`SESSION R13 / WAVE 12`)

- **Prepared By:** `SESSION-R12-20261115T103000+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Date:** `2026-11-15`
- **Target Milestone (Awaiting PO Approval):** `Milestone R13` — Wave 12 Quantitative Session Market Profile / Institutional Multi-Interval VWAP & VN30 Rotation Breadth / Workstation Polish

---

## 1. Self-Start Entry Protocol (Zero Copy-Paste Needed)

Any successor Orchestrator session starting in `/Users/mizuhara/workspace/sumi` must first read:

1. [`docs/operations/SUMI_ORCHESTRATION_CHARTER.md`](operations/SUMI_ORCHESTRATION_CHARTER.md) — Role authority, subagent protocol, evidence classification (`VERIFIED` / `INFERRED` / `UNVERIFIED` / `BLOCKED`), per-batch Git commit policy (`DEC-R04-001`), and mandatory Vietnamese communication with the Product Owner.
2. [`docs/EXECUTION_STATE.md`](EXECUTION_STATE.md) — Current baseline (`IMPLEMENTATION_PASS` across Waves 1–11 / Milestones R03–R12).
3. [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) & [`docs/DECISION_LOG.md`](DECISION_LOG.md) — Modular `src/data/`, `src/viewport/` (including `multi_chart.rs` & `workspace_template.rs`), `src/indicators/` (including `ichimoku.rs` & `cvd.rs`), `src/drawings/`, `src/replay/`, `src/trading/`, `src/scanner/`, `src/alerts/`, `src/backtest/`, `src/journal/`, multi-pane `MultiPaneLayout` + synchronized multi-chart grid, `PlotDataLayerMemo`, `OffscreenWgpuHarness`, and `DECISION-RENDERER-002` (`< 4.0 ms` CPU frame budget).
4. [`docs/research/RESEARCH_TO_RUST_ROADMAP.md`](research/RESEARCH_TO_RUST_ROADMAP.md) & [`docs/research/MULTI_CHART_ICHIMOKU_CVD_SPEC.md`](research/MULTI_CHART_ICHIMOKU_CVD_SPEC.md) — Shipped Wave 1–11 specifications and proposed Wave 12 institutional workflow enhancements.

---

## 2. Current Baseline Verified in R12

- **Build, WASM & Tests:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings`, and `cargo test` (`174/174` unit + property + headless multi-pane/multi-chart UI + Critic R03–R12 regression tests) pass with `0` warnings and `0` errors; `Cargo.toml` and `Cargo.lock` have `0` new dependencies.
- **Shipped Subsystems (`R03–R12`):**
  - `src/data/` (Hinnant O(1) UTC calendar, RFC-4180 CSV & recursive-descent JSON kline/envelope parsers at `103.7–191.1 MiB/s`, `Vn30Basket` + `CafeF`/`SPY`/`BTCUSDT` fixtures, tail-coverage-scored benchmark lookup, `1m..1W` upward lattice, `aggregate_ohlcv` + causal `aggregate_ohlcv_up_to`)
  - `src/viewport/` (`TimeScale`, `PriceScale` + `sumi_oscillator`, `PaneViewport`, `KineticAnimation`, `candle_metrics`, plus **R12** `MultiChartLayoutKind` `1x1`/`1x2`/`2x1`/`2x2` + `MultiChartSyncConfig` + `SyncedTimeWindow` + `causal_closed_prefix_len` + `WorkspaceTemplate` `schema_version = 1` strict `RON`/`JSON` codecs & institutional presets)
  - `src/indicators/` (`SMA`, `EMA`, `BollingerBands`, `RSI`, `MACD`, `ATR`, `IndicatorCache`, hybrid `SeriesBlockIndex` & `CandleBlockIndex` `16`-bar acceleration, `VRVP`/`FRVP`, rescaled Welford Session/Anchored `VWAP` $\pm 1\sigma/\pm 2\sigma$, `DEF-001` causal `RS vs VNINDEX`, plus **R12** `Ichimoku Cloud` `9/26/52/+26` & `Cumulative Volume Delta` `OHLCV_PROXY` `Cumulative`/`Session` + `9`-SMA)
  - `src/drawings/`, `src/replay/`, `src/trading/`, `src/scanner/`, `src/alerts/`, `src/backtest/`, `src/journal/` (all Pure `f64`/`i64` WASM-safe domain engines + `DEF-001` causal replay propagation across MTF secondary slots while preserving `custom_file` provenance)
  - `src/chart.rs`, `src/app.rs` & `src/main.rs` (`MultiPaneLayout` including `RelativeStrength`, `Equity`, and **R12** `Cvd` with independent `cvd_price_scale`, triangle-split Kumo `egui::Mesh`, `PlotDataLayerMemo` inside `R11RenderMemo`, Workspace Templates UI modal + `Cmd+K` presets, and headless `OffscreenWgpuHarness` Metal verification holding `Quad 2x2` `200k`-bar `32x` conflated Kumo + Session VWAP + Session CVD at **`1.3035 ms`** mean / **`1.6077 ms`** `p99 < 4.0 ms`, `"overall_pass": true`)

---

## 3. Recommended R13 Plan (`Wave 12 — Liquidity Market Profile, VN30 Breadth Rotation & Non-Blocking Critic Polish`)

### Workstream 1 — Research Gate (`DEC-005` Compliance)
1. Execute live REA MCP JS/TS analysis on `ARCHIVE/frontend` breadth/market-internality components and `lightweight-charts-5.2.0` multi-series primitive patterns; author the R13 specification in `docs/research/`.

### Workstream 2 — Minor Non-Blocking Critic Cleanups Carried Forward from R12 + Wave 12 Domain
1. Apply the two non-blocking Minor polish items observed by `sumi-critic` in R12 (`cvd_price_scale.reset_auto_scale()` across all primary state transitions in `src/app.rs`, and overflow-safe `0.5 * ea.entry_val + 0.5 * ea.exit_val` at `src/chart.rs:4751`).
2. Implement Wave 12 pure `f64`/`i64` WASM-safe Liquidity Session TPO / Market Profile & VN30 Rotation Breadth Heatmap + Multi-Slot Template Extensions in `src/indicators/`, `src/scanner/`, `src/viewport/`, `src/chart.rs`, and `src/app.rs`.

### Workstream 3 — `--r13` Release Benchmark, Offscreen Metal Captures, Critic, Audit & Clean Commit
1. Run `--release` `--r13` benchmark (`< 4.0 ms` CPU frame budget) + `SUMI_CAPTURE_BATCH=r13` offscreen Retina Metal captures, remediate all `sumi-critic` findings, obtain independent `sumi-auditor` `AUDIT_PASS`, and create one clean local commit on `master` per `DEC-R04-001`.

---

## 4. Standing Constraints & Product Owner Gates

- **Product Owner Authorization Gate:** Wait for explicit Product Owner approval before starting Milestone R13 implementation.
- **Language:** Communicate 100% in **Vietnamese (`tiếng Việt`)** with the Product Owner.
- **Git & File Safety (`DEC-R04-001`):** Never touch `Cargo.toml`, `Cargo.lock`, `src/model.rs`, `ARCHIVE/`, or R01–R12 `docs/evidence/*`; never run `git reset`, `git clean`, `git push`, or `git tag`.
- **Authorized Targets Only (`DEC-006`):** Never inspect or interact with TradingView Desktop.
