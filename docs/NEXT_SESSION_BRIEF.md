# SUMI TRADING WORKSTATION — NEXT SESSION BRIEF (`SESSION R14 / WAVE 13`)

- **Prepared By:** `SESSION-R13-20261115T163000+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Date:** `2026-11-15`
- **Target Milestone (Awaiting PO Approval):** `Milestone R14` — Wave 13 Synthetic Depth-of-Market / Footprint Execution Ladder, Native Hotkey & Dock Layout Profiles, & Non-Blocking R13 Axis Polish

---

## 1. Self-Start Entry Protocol (Zero Copy-Paste Needed)

Any successor Orchestrator session starting in `/Users/mizuhara/workspace/sumi` must first read:

1. [`docs/operations/SUMI_ORCHESTRATION_CHARTER.md`](operations/SUMI_ORCHESTRATION_CHARTER.md) — Role authority, subagent protocol, evidence classification (`VERIFIED` / `INFERRED` / `UNVERIFIED` / `BLOCKED`), per-batch Git commit policy (`DEC-R04-001`), and mandatory Vietnamese communication with the Product Owner.
2. [`docs/EXECUTION_STATE.md`](EXECUTION_STATE.md) — Current baseline (`IMPLEMENTATION_PASS` across Waves 1–12 / Milestones R03–R13).
3. [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) & [`docs/DECISION_LOG.md`](DECISION_LOG.md) — Modular `src/data/`, `src/viewport/` (including `multi_chart.rs` & `workspace_template.rs`), `src/indicators/` (including `market_profile.rs`, `ichimoku.rs`, `cvd.rs`, `vwap.rs`, `volume_profile.rs`, `relative_strength.rs`), `src/drawings/`, `src/replay/`, `src/trading/`, `src/scanner/` (including `breadth.rs`), `src/alerts/`, `src/backtest/`, `src/journal/`, multi-pane `MultiPaneLayout` + synchronized multi-chart grid, `PlotDataLayerMemo`, `OffscreenWgpuHarness`, and `DECISION-RENDERER-002` (`< 4.0 ms` CPU frame budget).
4. [`docs/research/RESEARCH_TO_RUST_ROADMAP.md`](research/RESEARCH_TO_RUST_ROADMAP.md) & [`docs/research/MARKET_PROFILE_BREADTH_AND_AXIS_SPEC.md`](research/MARKET_PROFILE_BREADTH_AND_AXIS_SPEC.md) — Shipped Wave 1–12 specifications and proposed Wave 13 enhancements.

---

## 2. Current Baseline Verified in R13

- **Build, WASM & Tests:** `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --lib --target wasm32-unknown-unknown -- -D warnings`, and `cargo test` (`181/181` unit + property + headless multi-pane/multi-chart UI + Critic R03–R13 regression tests) pass with `0` warnings and `0` errors; `Cargo.toml` and `Cargo.lock` have `0` new dependencies.
- **Shipped Subsystems (`R03–R13`):**
  - `src/data/` (Hinnant O(1) UTC calendar, RFC-4180 CSV & recursive-descent JSON kline/envelope parsers at `103.7–191.1 MiB/s`, `Vn30Basket` + `CafeF`/`SPY`/`BTCUSDT` fixtures, tail-coverage-scored benchmark lookup, `1m..1W` upward lattice, `aggregate_ohlcv` + causal `aggregate_ohlcv_up_to`)
  - `src/viewport/` (`TimeScale`, `PriceScale` + `PriceAxisFormatMode::CompactVolume` `K`/`M`/`B`/`T`, `PaneViewport`, `KineticAnimation`, `candle_metrics`, `MultiChartLayoutKind` `1x1`/`1x2`/`2x1`/`2x2` + `MultiChartSyncConfig` + `SyncedTimeWindow` + `causal_closed_prefix_len`, `WorkspaceTemplate` `schema_version = 1` strict `RON`/`JSON` codecs + **R13** `5` built-in templates & `mp_session_period`)
  - `src/indicators/` (`SMA`, `EMA`, `BollingerBands`, `RSI`, `MACD`, `ATR`, `IndicatorCache`, hybrid `SeriesBlockIndex` & `CandleBlockIndex` `16`-bar acceleration, `VRVP`/`FRVP`, rescaled Welford Session/Anchored `VWAP` $\pm 1\sigma/\pm 2\sigma$ with **R13** `VwapAnchorMode` & negative-UTC `D1`/`W1` guard, `DEF-001` causal `RS vs VNINDEX`, `Ichimoku Cloud` `9/26/52/+26`, `Cumulative Volume Delta` `Cumulative`/`Session` + `9`-SMA, & **R13** `Session TPO / Market Profile` with `POC`, CME 70% `VAH`/`VAL`, `IB`, `Single Prints`, `Naked POC`, & `HistoricalNakedPoc` up to `96` prior sessions)
  - `src/scanner/` (`SymbolScanRow` / `HistoricalSignalHit` + **R13** `breadth.rs` implementing `AT05`/`AT09` Aggregate-Before-Ratio Money Flow, `AT17` VN30 vs Universe participation ratios, and causal `DEF-001` 4-quadrant `RRG`)
  - `src/drawings/`, `src/replay/`, `src/trading/`, `src/alerts/`, `src/backtest/`, `src/journal/` (all Pure `f64`/`i64` WASM-safe domain engines)
  - `src/chart.rs`, `src/app.rs` & `src/main.rs` (Synchronized multi-chart slots, `WorldBinIntervalAccumulator` sub-pixel `VWAP`/`CVD` pan-invariant envelopes, dual-sided conflated `CVD` delta bars, batched Market Profile `Mesh`, compact `K`/`M`/`B`/`T` `CVD` right axis, obstacle-seeded `RRG` + symmetric Heatmap inside 340px `Breadth` dock, per-slot `TPO`/`VWAP`/`Sync Ind` controls, `cvd_price_scale.reset_auto_scale()` across primary transitions, `safe_midpoint_f64`, and `--r13` holding all workloads `< 4.0 ms`: `0.2588–2.4689 ms` mean / `p99 <= 2.8089 ms`, `"overall_pass": true`)

---

## 3. Recommended R14 Plan (`Wave 13 — Execution Ladder, Hotkey Profiles & Non-Blocking R13 Axis Polish`)

### Workstream 1 — Research Gate (`DEC-005` Compliance)
1. Execute live REA MCP JS/TS analysis on `ARCHIVE/frontend` order-ticket/hotkey/dock components and `lightweight-charts-5.2.0` price-format & axis-tick boundary conventions; author the R14 specification in `docs/research/`.

### Workstream 2 — Non-Blocking R13 Polish & Wave 13 Domain
1. Apply the two non-blocking polish items noted by `sumi-critic` in R13 (`edge_pad` top-margin clearance on short quad panes so top tick labels never graze the pane upper border, and uniform scale-step decimal formatting across `< 100.0` vs `>= 100.0` boundaries in `format_price_with_step`).
2. Implement pure `f64`/`i64` WASM-safe **Order-Book / Footprint Ladder (`DOM`) & Price-Ladder Click-to-Stage** integrated with `TradingSession` and `Journal` risk sizing, plus **Customizable Hotkey Profiles & Dock Layout Presets** serialized inside `WorkspaceTemplate`.

### Workstream 3 — `--r14` Release Benchmark, Offscreen Metal Captures, Critic, Audit & Clean Commit
1. Run `--release` `--r14` benchmark (`< 4.0 ms` CPU frame budget) + `SUMI_CAPTURE_BATCH=r14` offscreen Retina Metal captures, remediate all `sumi-critic` findings (`CRITIC_PASS`), obtain independent `sumi-auditor` `AUDIT_PASS`, and create one clean local commit on `master` per `DEC-R04-001`.

---

## 4. Standing Constraints & Product Owner Gates

- **Product Owner Authorization Gate:** Wait for explicit Product Owner approval before starting Milestone R14 implementation.
- **Language:** Communicate 100% in **Vietnamese (`tiếng Việt`)** with the Product Owner.
- **Git & File Safety (`DEC-R04-001`):** Never touch `Cargo.toml`, `Cargo.lock`, `src/model.rs`, `ARCHIVE/`, or R01–R13 `docs/evidence/*`; never run `git reset`, `git clean`, `git push`, or `git tag`.
- **Authorized Targets Only (`DEC-006`):** Never inspect or interact with TradingView Desktop.
