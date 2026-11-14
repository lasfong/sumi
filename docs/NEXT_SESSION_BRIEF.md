# SUMI TRADING WORKSTATION — NEXT SESSION BRIEF (`SESSION R11 / WAVE 10`)

- **Prepared By:** `SESSION-R10-20261114T122730+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Date:** `2026-11-14`
- **Target Milestone (Awaiting PO Approval):** `Milestone R11` — Wave 10 Volume Profile (`VRVP` / `FRVP` `POC` / `VAH` / `VAL`), Session & Anchored `VWAP` (`AVWAP`) Standard-Deviation Bands, and Relative Strength (`RS vs VNINDEX` / Peer Benchmark) Comparison Overlay

---

## 1. Self-Start Entry Protocol (Zero Copy-Paste Needed)

Any successor Orchestrator session starting in `/Users/mizuhara/workspace/sumi` must first read:

1. [`docs/operations/SUMI_ORCHESTRATION_CHARTER.md`](operations/SUMI_ORCHESTRATION_CHARTER.md) — Role authority, subagent protocol, evidence classification (`VERIFIED` / `INFERRED` / `UNVERIFIED` / `BLOCKED`), per-batch Git commit policy (`DEC-R04-001`), and mandatory Vietnamese communication with the Product Owner.
2. [`docs/EXECUTION_STATE.md`](EXECUTION_STATE.md) — Current baseline (`IMPLEMENTATION_PASS` across Waves 1–9 / Milestones R03–R10; `100%` of legacy Sumi webapp workflows now ported and upgraded in native Rust).
3. [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) & [`docs/DECISION_LOG.md`](DECISION_LOG.md) — Modular `src/data/`, `src/viewport/`, `src/indicators/`, `src/drawings/`, `src/replay/`, `src/trading/`, `src/scanner/`, `src/alerts/`, `src/backtest/`, `src/journal/`, 5-pane `MultiPaneLayout`, `OffscreenWgpuHarness`, and `DECISION-RENDERER-002` (`< 4.0 ms` CPU frame budget).
4. [`docs/research/RESEARCH_TO_RUST_ROADMAP.md`](research/RESEARCH_TO_RUST_ROADMAP.md) & [`docs/research/RISK_PLANNER_JOURNAL_AND_PORTFOLIO_SPEC.md`](research/RISK_PLANNER_JOURNAL_AND_PORTFOLIO_SPEC.md) — Shipped Wave 1–9 specifications and proposed Wave 10 institutional order-flow & relative-strength expansion.

---

## 2. Current Baseline Verified in R10

- **Build, WASM & Tests:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings`, and `cargo test` (`136/136` unit + property + headless multi-pane UI + Critic R03–R10 regression tests) pass with `0` warnings and `0` errors; `Cargo.toml` and `Cargo.lock` have `0` new dependencies.
- **Shipped Subsystems:**
  - `src/data/` (Hinnant O(1) UTC calendar, RFC-4180 CSV & recursive-descent JSON kline/envelope parsers at `103.7–191.1 MiB/s`, `Vn30Basket` + `CafeF`/`SPY`/`BTCUSDT` fixtures, `1m..1W` upward lattice, `aggregate_ohlcv` + causal `aggregate_ohlcv_up_to`)
  - `src/viewport/` (`TimeScale`, `PriceScale` + `sumi_oscillator`, `PaneViewport`, `KineticAnimation`, `candle_metrics`)
  - `src/indicators/` (`SMA`, `EMA`, `BollingerBands`, `RSI`, `MACD`, `ATR`, `IndicatorCache`, `SeriesBlockIndex`, conflated envelopes)
  - `src/drawings/` (`HorizontalLine`, `TrendLine`, $360^\circ$ `Ray`, `Rectangle`, `FibonacciRetracement`, dual-zone `RiskRewardBox`, `DrawingAnchor::timestamp`, `Drawing::created_at_timestamp`, `DrawingStore::remap_timeframe`, causal replay visibility, 64-step Undo/Redo, `serde` persistence)
  - `src/replay/` (`ReplayController`, `DEF-001` zero-lookahead prefix materialization, `ActiveBar`/`BaseBar` stepping, `1x..20x` playback)
  - `src/trading/` (`TradingSession`, `Long`/`Short` `Market`/`Limit`/`Stop` orders, `SL`/`TP` brackets, unified Gap-Open + 4-waypoint OHLC matching, `1m == 15m` parity, fee conservation, `RiskRewardBox` ticket extraction, command-log `rewind_to`)
  - `src/scanner/` (13-symbol Watchlist Universe + `custom_peer_feeds`, single-pass `1.799 ms` Watchlist + Signal scan, cross-symbol `DEF-001` cutoff, bucket-close `BlindPractice` / `SignalReview`)
  - `src/alerts/` (`AlertEngine`, `PriceLevel` 4-waypoint/@Mark, `RSI(14)`, `MACD(12,26,9)`, `DrawingLine` `HorizontalLine`/`TrendLine`/$360^\circ$ `Ray` relative-offset crossings & causal Replay rewind)
  - `src/backtest/` (`6` composable strategy presets, causal `Open[T+1]` fills, `MarginLiquidation`, continuous `IS/OOS` partition, `77.29 µs` `12`-grid sweep, `639.92 µs` `13`-symbol battle, continuous-carry `K`-fold Rolling/Anchored Walk-Forward `5.958 ms`, & bar-close `EqualWeight`/`CausalInverseVolatility` Multi-Asset Portfolio `106 µs`–`2.855 ms`)
  - `src/journal/` (`HOSE`/`HNX`/`UPCOM`/`VN30F`/`VNINDEX`/`US`/`Crypto` microstructure, tier-crossing ticks, `x100k` futures multiplier + `17%` VSD margin, `UTC+7` `T+2` settlement, `101.12 µs` / `1K` plans, `DEF-001` causal `TradeJournalStore`, `SQN`/`Discipline %`/`Cost of Mistakes`, formula-safe RFC-4180 CSV & Markdown export)
  - `src/chart.rs`, `src/app.rs` & `src/main.rs` (5-pane `MultiPaneLayout` with `SingleBacktest` / `WalkForwardOos` / `MultiAssetPortfolio` Equity baseline `egui::Mesh`, dual-zone `RiskRewardBox`, Right `Trade` / `Alerts` / `Strat` / `Jrn` Docks, Floating `Cmd+K` Command Palette, and headless `OffscreenWgpuHarness` Retina Metal verification in `0.276–0.369 ms`)

---

## 3. Recommended R11 Execution Plan (`Wave 10 — Volume Profile VRVP/FRVP, Session & Anchored VWAP Bands & Relative Strength vs VNINDEX`)

### Workstream 1 — Research Gate (`DEC-005` Compliance)
1. Execute live REA MCP JS/TS analysis on `lightweight-charts-5.2.0` histogram/pane-primitive/price-scale comparison modules (`histogram-series.ts`, `histogram-renderer.ts`, `ipane-primitive-api.ts`, `price-scale.ts` percentage/indexed modes) and `TA-Lib` volume/typical-price accumulation references.
2. Author `docs/research/VOLUME_PROFILE_VWAP_AND_REL_STRENGTH_SPEC.md` defining institutional horizontal Volume Profile (`VRVP` + `FRVP` `POC`, `70%` Value Area `VAH`/`VAL`, Up/Down volume delta), exchange-session & user-anchored `VWAP` (`AVWAP` $\pm 1\sigma, \pm 2\sigma$ bands), and causal `RS vs VNINDEX` multi-series comparison overlays.

### Workstream 2 — Pure `f64`/`i64` WASM-Safe Volume Profile, `AVWAP` & Relative Strength Engine
1. Implement `Visible Range Volume Profile (VRVP)` and interactive `Fixed Range Volume Profile (FRVP)` (`POC`, `70%` Value Area expansion around `POC`, High/Low Volume Nodes, Up/Down candle volume split) with $O(B + K)$ bucket aggregation and `< 4.0 ms` `egui::Mesh` rendering.
2. Implement Session `VWAP` (`UTC+7` Vietnam session / `UTC` crypto daily reset) and 1-click `Anchored VWAP (AVWAP)` drawing/indicator with volume-weighted variance $\pm 1\sigma, \pm 2\sigma$ bands, plus causal `RS vs VNINDEX` (and `SPY`/`BTCUSDT`) comparison overlay & oscillator sub-pane gated by `replay_causal_close_timestamp` (`DEF-001`).

### Workstream 3 — Workstation UI, `--r11` Benchmark, Critic, Audit & Clean Commit
1. Integrate `VRVP` toggle, `FRVP` & `AVWAP` chart tools, `Compare / RS (VNINDEX)` selector, and `Cmd+K` commands into `SumiWorkstationApp`.
2. Run `--release` `--r11` benchmark (`< 4.0 ms` CPU frame budget) + offscreen `SUMI_CAPTURE_BATCH=r11` Retina Metal captures, remediate all `sumi-critic` findings, verify via `sumi-auditor`, and create one clean commit on `master` per `DEC-R04-001`.

---

## 4. Standing Constraints & Product Owner Gates

- **Product Owner Authorization Gate:** Wait for explicit Product Owner approval before starting Milestone R11 implementation.
- **Language:** Communicate 100% in **Vietnamese (`tiếng Việt`)** with the Product Owner.
- **Git & File Safety (`DEC-R04-001`):** Create one clean commit at the end of each verified batch/milestone; never run `git reset`, `git clean`, `git push`, or delete `ARCHIVE/`, `ViewportMapping`, or prior `docs/evidence/*`.
- **Authorized Targets Only (`DEC-006`):** Never inspect or interact with TradingView Desktop.
