# SUMI TRADING WORKSTATION — NEXT SESSION BRIEF (`SESSION R18 / WAVE 17`)

- **Prepared By:** `SESSION-R17-20261118T213600+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Date:** `2026-11-18`
- **Target Milestone (Awaiting PO Approval):** `Milestone R18` — Wave 17 Complete `72+` Signal Ecosystem (`14` Causal Candlestick Patterns + `CandleGeometry`, `8` Remaining Legacy Indicators, Confirmed-Pivot Divergences `right_bars = 3` + Full `VSA`/`SR`/`Ichimoku` Signals, & Multi-Horizon Money Flow `ProxyBB` `T03..T200` + `Point-in-Time` Universe — `FEATURE_BACKLOG_AND_PARITY_LEDGER.md` Group B: `B1..B4`)

---

## 1. Self-Start Entry Protocol (Zero Copy-Paste Needed)

Any successor Orchestrator session starting in `/Users/mizuhara/workspace/sumi` must first read:

1. [`docs/operations/SUMI_ORCHESTRATION_CHARTER.md`](operations/SUMI_ORCHESTRATION_CHARTER.md) — Role authority, subagent protocol, evidence classification (`VERIFIED` / `INFERRED` / `UNVERIFIED` / `BLOCKED`), per-batch Git commit policy (`DEC-R04-001`), and mandatory Vietnamese communication with the Product Owner.
2. [`docs/FEATURE_BACKLOG_AND_PARITY_LEDGER.md`](FEATURE_BACKLOG_AND_PARITY_LEDGER.md) — **Mandatory Anti-Miss Parity Ledger** tracking every completed (`S-01..S-10`, `R16-1..R16-3`, `A1..A5`) and queued item across `Milestone R18` (`B1..B4`) and `Milestone R19` (`C1..C4`). Always update item statuses here when closing or proposing milestones so nothing is ever lost.
3. [`docs/EXECUTION_STATE.md`](EXECUTION_STATE.md) — Current baseline (`IMPLEMENTATION_PASS` across Waves 1–16 / Milestones R03–R17).
4. [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) & [`docs/DECISION_LOG.md`](DECISION_LOG.md) — Modular `src/data/`, `src/viewport/` (`SignedPercentage`, `WorkspaceSnapshotStore`), `src/indicators/`, `src/drawings/`, `src/replay/`, `src/trading/` (including `coaching.rs` & `ladder.rs`), `src/scanner/` (`PrecomputedCatalogMatrix`), `src/alerts/`, `src/backtest/` (`CustomSignalRuleSpec`, `analytics.rs`, `walk_forward.rs`, `monte_carlo.rs`), `src/journal/`, and `DECISION-RENDERER-002` (`< 4.0 ms` CPU frame budget).
5. [`docs/research/RESEARCH_TO_RUST_ROADMAP.md`](research/RESEARCH_TO_RUST_ROADMAP.md) & [`docs/research/PRACTICE_COACHING_AND_MULTISTRATEGY_ANALYTICS_SPEC.md`](research/PRACTICE_COACHING_AND_MULTISTRATEGY_ANALYTICS_SPEC.md) — Shipped Wave 1–16 specifications and proposed Wave 17–18 scope.

---

## 2. Current Baseline Verified in R17

- **Build, WASM & Tests:** `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --lib --target wasm32-unknown-unknown -- -D warnings`, and `cargo test` (`245/245` unit + property + headless UI + Critic R03–R17 regression tests) pass with `0` warnings and `0` errors; `Cargo.toml` and `Cargo.lock` have `0` new dependencies.
- **Shipped Subsystems (`R03–R17`):**
  - `src/data/`, `src/viewport/` (`TimeScale`, `PriceScale` + `CompactVolume` + `SignedPercentage`, `MultiChartLayoutKind`, `HotkeyProfile`, `WorkspaceTemplate`, & `WorkspaceSnapshotStore`)
  - `src/indicators/`, `src/drawings/` (`8` geometries + `DrawingPropertyPatch`), `src/replay/`, `src/trading/` (`portfolio.rs` tamper-proof `had_initial_sl_at_open`, `ladder.rs`, & **R17** `coaching.rs`: `PracticeScoreboardSummary`, `SessionDebriefReport`, `PracticeDecisionLog`, `PracticeSessionStore`, `ScanSnapshotStore`), & `src/journal/`
  - `src/scanner/` (`INSTITUTIONAL_BLIND_PRACTICE_PRE_ROLL_BARS = 120`, `breadth.rs`, `17`-entry `SIGNAL_CATALOG` + $O(N)$ `PrecomputedCatalogMatrix`)
  - `src/alerts/` (`PriceLevel`, `RSI`, `MACD`, `DrawingLine`, `SignalCatalogTrigger`, `ConfluenceThreshold`, `cooldown_bars`, CSV/JSON export)
  - `src/backtest/` (`7` presets incl. `CustomSignalRules`, `slice_causal_series_for_symbol_and_options`, `WFO`, `Portfolio`, `SplitMix64` Monte Carlo, & **R17** `analytics.rs`: `MultiStrategyComparisonReport`, `OutlierImpactReport`, `DrawdownPeriod`, UTC Year & Signal-Bar `MarketRegime` slices, `MultiPhaseBatchReport`, `RunManifest`)
  - `src/chart.rs`, `src/app.rs` & `src/main.rs` (`--r17` holding all UI workloads `< 4.0 ms`: **`0.336–2.082 ms`** mean / **`p99 <= 2.422 ms`**, `"overall_pass": true`)

---

## 3. Recommended R18 Plan (`Wave 17 — 72+ Signal Ecosystem, 8 Remaining Indicators, ProxyBB & PIT Universe`, `B1..B4`)

### Workstream 1 — Research Gate (`DEC-005` Compliance)
1. Execute live REA MCP JS/TS analysis on `ARCHIVE/frontend/src/components/{chart/TechnicalFlowBBViewer.tsx,signals/SignalCatalog.tsx,signals/SignalInspector.tsx}` and `LWC v5.2.0` histogram/overlay primitives alongside read-only mathematical inspection of `ARCHIVE/backend/app/domain/{engine/indicator_engine.py,signals/*,bb/*,universe/resolver.py}` and `research_repos/ta-lib`; author the R18 specification in `docs/research/`.

### Workstream 2 — Wave 17 Domain & UI Enhancements (`B1..B4` in `FEATURE_BACKLOG_AND_PARITY_LEDGER.md`)
1. **`B1` — `14` Causal Candlestick Patterns & `CandleGeometry` (`src/indicators/`, `src/scanner/catalog.rs`, `src/chart.rs`):** Pure-Rust `CandleGeometry` (`body_ratio`, `upper/lower_wick_ratio`, `close_location`) + all `14` patterns (`bullish/bearish_engulfing`, `hammer`, `shooting_star`, `morning/evening_star`, `piercing_line`, `dark_cloud_cover`, `tweezer_bottom/top`, `inside_bar_breakout_up/down`, `any_bullish/bearish`) + `structure.candle_score`, wired automatically into `SIGNAL_CATALOG`, Rule Builder, and Causal Alerts.
2. **`B2` — `8` Remaining Legacy Indicators (`src/indicators/`, `src/chart.rs`, `src/app.rs`):** `ADX(14)` (`+DI/-DI`), `Stochastic(%K, %D)`, `SuperTrend(10, 3.0)`, `Parabolic SAR`, `Keltner Channels`, `CCI(20)`, `MFI(14)`, and `Volume SMA(20)` — achieving a `100%` superset of all `16` legacy `INDICATOR_REGISTRY` indicators plus Sumi's native `VWAP`/`VRVP`/`TPO`/`CVD`/`RS`.
3. **`B3` — Confirmed-Pivot Divergences (`right_bars = 3`) & Full `VSA`/`SR`/`Ichimoku` Signals (`src/scanner/catalog.rs`):** Causal `confirmed_pivot_high/low` (`causal_delay_bars = 3` with zero future-bar lookahead) powering Regular & Hidden Bullish/Bearish divergences on `RSI`, `MACD Histogram`, and `Stochastic` (`14` signals), plus `vsa.*`, `sr.{near_support,near_resistance}`, and `ichimoku.*` (`72+` total `SIGNAL_CATALOG` entries).
4. **`B4` — Multi-Horizon Money Flow `ProxyBB` (`T03..T200`), Market Flow Breadth & `Point-in-Time` Universe (`src/scanner/`, `src/backtest/`, `src/app.rs`):** `ProxyBBCalculator` across `T03, T05, T10, T20, T50, T200` (`AT05` zero-liquidity `None` invariant) + `8` `bb.*` signals & viewer, `MarketBBAggregator` (`AT09/AT10`), and `UniverseResolver` (`POINT_IN_TIME` vs `RETROSPECTIVE_FIXED`) eliminating survivorship bias in Portfolio/Battle runs.

### Workstream 3 — `--r18` Release Benchmark, Offscreen Metal Captures, Critic, Audit & Clean Commit
1. Pass all 4 Cargo gates, run `--release` `--r18` (`< 4.0 ms` CPU frame budget) + `SUMI_CAPTURE_BATCH=r18`, update `FEATURE_BACKLOG_AND_PARITY_LEDGER.md` (`B1..B4 -> [x]`), pass `sumi-critic` (`CRITIC_PASS`) and `sumi-auditor` (`AUDIT_PASS`), and commit cleanly on `master` per `DEC-R04-001`.

---

## 4. Standing Constraints & Product Owner Gates

- **Product Owner Authorization Gate:** Wait for explicit Product Owner approval before starting Milestone R18 implementation.
- **Language:** Communicate 100% in **Vietnamese (`tiếng Việt`)** with the Product Owner.
- **Git & File Safety (`DEC-R04-001`):** Never touch `Cargo.toml`, `Cargo.lock`, `src/model.rs`, `ARCHIVE/`, or R01–R17 `docs/evidence/*`; never run `git reset`, `git clean`, `git push`, or `git tag`.
- **Authorized Targets Only (`DEC-006`):** Never inspect or interact with TradingView Desktop.
