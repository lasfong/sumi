# SUMI TRADING WORKSTATION — NEXT SESSION BRIEF (`SESSION R12 / WAVE 11`)

- **Prepared By:** `SESSION-R11-20261114T231200+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Date:** `2026-11-15`
- **Target Milestone (Awaiting PO Approval):** `Milestone R12` — Wave 11 Synchronized Multi-Chart Grid Layout (`1x1` / `1x2` / `2x1` / `2x2` Linked Crosshair/Time/Symbol Views), `Ichimoku Cloud` + `Cumulative Volume Delta (CVD)`, & Persistent Workspace Layout Templates

---

## 1. Self-Start Entry Protocol (Zero Copy-Paste Needed)

Any successor Orchestrator session starting in `/Users/mizuhara/workspace/sumi` must first read:

1. [`docs/operations/SUMI_ORCHESTRATION_CHARTER.md`](operations/SUMI_ORCHESTRATION_CHARTER.md) — Role authority, subagent protocol, evidence classification (`VERIFIED` / `INFERRED` / `UNVERIFIED` / `BLOCKED`), per-batch Git commit policy (`DEC-R04-001`), and mandatory Vietnamese communication with the Product Owner.
2. [`docs/EXECUTION_STATE.md`](EXECUTION_STATE.md) — Current baseline (`IMPLEMENTATION_PASS` across Waves 1–10 / Milestones R03–R11).
3. [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) & [`docs/DECISION_LOG.md`](DECISION_LOG.md) — Modular `src/data/`, `src/viewport/`, `src/indicators/`, `src/drawings/`, `src/replay/`, `src/trading/`, `src/scanner/`, `src/alerts/`, `src/backtest/`, `src/journal/`, multi-pane `MultiPaneLayout`, `OffscreenWgpuHarness`, and `DECISION-RENDERER-002` (`< 4.0 ms` CPU frame budget).
4. [`docs/research/RESEARCH_TO_RUST_ROADMAP.md`](research/RESEARCH_TO_RUST_ROADMAP.md) & [`docs/research/VOLUME_PROFILE_VWAP_AND_REL_STRENGTH_SPEC.md`](research/VOLUME_PROFILE_VWAP_AND_REL_STRENGTH_SPEC.md) — Shipped Wave 1–10 specifications and proposed Wave 11 Multi-Chart Grid / Ichimoku / CVD workspace suite.

---

## 2. Current Baseline Verified in R11

- **Build, WASM & Tests:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --lib --target wasm32-unknown-unknown -- -D warnings`, and `cargo test` (`152/152` unit + property + headless multi-pane UI + Critic R03–R11 regression tests) pass with `0` warnings and `0` errors; `Cargo.toml` and `Cargo.lock` have `0` new dependencies.
- **Shipped Subsystems:**
  - `src/data/` (Hinnant O(1) UTC calendar, RFC-4180 CSV & recursive-descent JSON kline/envelope parsers at `103.7–191.1 MiB/s`, `Vn30Basket` + `CafeF`/`SPY`/`BTCUSDT` fixtures, tail-coverage-scored benchmark lookup, `1m..1W` upward lattice, `aggregate_ohlcv` + causal `aggregate_ohlcv_up_to`)
  - `src/viewport/` (`TimeScale`, `PriceScale` + `sumi_oscillator`, `PaneViewport`, `KineticAnimation`, `candle_metrics`)
  - `src/indicators/` (`SMA`, `EMA`, `BollingerBands`, `RSI`, `MACD`, `ATR`, `IndicatorCache`, `SeriesBlockIndex::first_valid_index`, conflated envelopes, plus R11 Order-Flow `VRVP`/`FRVP` (`14.21–803.08 µs`), rescaled Welford Session/Anchored `VWAP` $\pm 1\sigma/\pm 2\sigma$ (`371.25–725.58 µs`), & `DEF-001` causal `RS vs VNINDEX` (`651.71 µs`))
  - `src/drawings/` (`HorizontalLine`, `TrendLine`, $360^\circ$ `Ray`, `Rectangle`, `FibonacciRetracement`, dual-zone `RiskRewardBox`, `FixedRangeVolumeProfile`, draggable `AnchoredVwap`, `DrawingAnchor::timestamp`, `Drawing::created_at_timestamp`, `DrawingStore::remap_timeframe`, causal replay visibility, 64-step Undo/Redo, `serde` persistence)
  - `src/replay/` (`ReplayController`, `DEF-001` zero-lookahead prefix materialization, `ActiveBar`/`BaseBar` stepping, `1x..20x` playback)
  - `src/trading/` (`TradingSession`, `Long`/`Short` `Market`/`Limit`/`Stop` orders, `SL`/`TP` brackets, unified Gap-Open + 4-waypoint OHLC matching, `1m == 15m` parity, fee conservation, `RiskRewardBox` ticket extraction, command-log `rewind_to`)
  - `src/scanner/` (13-symbol Watchlist Universe + `custom_peer_feeds`, single-pass `1.799 ms` Watchlist + Signal scan, cross-symbol `DEF-001` cutoff, bucket-close `BlindPractice` / `SignalReview`)
  - `src/alerts/` (`AlertEngine`, `PriceLevel` 4-waypoint/@Mark, `RSI(14)`, `MACD(12,26,9)`, `DrawingLine` `HorizontalLine`/`TrendLine`/$360^\circ$ `Ray` relative-offset crossings & causal Replay rewind)
  - `src/backtest/` (`6` composable strategy presets, causal `Open[T+1]` fills, `MarginLiquidation`, continuous `IS/OOS` partition, `12`-grid sweep, `13`-symbol battle, continuous-carry `K`-fold Rolling/Anchored Walk-Forward, & bar-close `EqualWeight`/`CausalInverseVolatility` Multi-Asset Portfolio)
  - `src/journal/` (`HOSE`/`HNX`/`UPCOM`/`VN30F`/`VNINDEX`/`US`/`Crypto` microstructure, tier-crossing ticks, `x100k` futures multiplier + `17%` VSD margin, `UTC+7` `T+2` settlement, all-in friction sizer, `DEF-001` causal `TradeJournalStore`, `SQN`/`Discipline %`/`Cost of Mistakes`, formula-safe RFC-4180 CSV & Markdown export)
  - `src/chart.rs`, `src/app.rs` & `src/main.rs` (Multi-pane `MultiPaneLayout` with `RelativeStrength` & `Equity`, `R11RenderMemo`, `VRVP`/`FRVP`, conflation-split `VWAP`/`AVWAP` ribbons, `O(1)` Rebased Compare Overlay, dual-zone `RiskRewardBox`, Right Docks, Floating `Cmd+K` Command Palette, and headless `OffscreenWgpuHarness` Retina Metal verification in `0.256–3.171 ms`, `"overall_pass": true`)

---

## 3. Recommended R12 Execution Plan (`Wave 11 — Synchronized Multi-Chart Grid, Ichimoku Cloud + CVD, & Workspace Templates`)

### Workstream 1 — Research Gate (`DEC-005` Compliance)
1. Execute live REA MCP JS/TS analysis on `ARCHIVE/frontend/src/components/chart/{MultiChartLayout.tsx,plugins/IchimokuCloudPlugin.ts}` and `lightweight-charts-5.2.0` multi-chart/pane-synchronization primitives.
2. Author `docs/research/MULTI_CHART_ICHIMOKU_CVD_SPEC.md` defining linked multi-chart grid synchronization (time range, crosshair, symbol/timeframe slots), `DEF-001` causal `Ichimoku Cloud` (`Tenkan`, `Kijun`, `Senkou A/B` + `Chikou` safe projection), `Cumulative Volume Delta (CVD)`, and versioned Workspace Templates (`RON`/`JSON`).

### Workstream 2 — Pure `f64`/`i64` WASM-Safe Engine Extensions
1. Implement synchronized multi-chart state coordinator linking crosshair/time/symbol slots across `1x1`/`1x2`/`2x1`/`2x2` grids while keeping strict per-chart `PriceScale` & causal Replay isolation.
2. Implement WASM-safe `IchimokuCloud` & `CumulativeVolumeDelta` (`OHLCV_PROXY` signed volume flow + cumulative session/lifetime reset modes) in `src/indicators/` + `WorkspaceTemplate` serializer/loader.

### Workstream 3 — Workstation UI, `--r12` Benchmark, Critic, Audit & Clean Commit
1. Integrate Multi-Chart layout selector (`1x1`/`1x2`/`2x1`/`2x2`), Ichimoku & CVD toggles, and Workspace Template save/restore + `Cmd+K` commands into `SumiWorkstationApp`.
2. Run `--release` `--r12` benchmark (`< 4.0 ms` CPU frame budget) + offscreen `SUMI_CAPTURE_BATCH=r12` Retina Metal captures, remediate all `sumi-critic` findings, verify via `sumi-auditor`, and create one clean commit on `master` per `DEC-R04-001`.

---

## 4. Standing Constraints & Product Owner Gates

- **Product Owner Authorization Gate:** Wait for explicit Product Owner approval before starting Milestone R12 implementation.
- **Language:** Communicate 100% in **Vietnamese (`tiếng Việt`)** with the Product Owner.
- **Git & File Safety (`DEC-R04-001`):** Create one clean commit at the end of each verified batch/milestone; never run `git reset`, `git clean`, `git push`, or delete `ARCHIVE/`, `ViewportMapping`, or prior `docs/evidence/*`.
- **Authorized Targets Only (`DEC-006`):** Never inspect or interact with TradingView Desktop.
