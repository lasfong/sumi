# SUMI TRADING WORKSTATION — NEXT SESSION BRIEF (`SESSION R16 / WAVE 15`)

- **Prepared By:** `SESSION-R15-20261118T074500+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Date:** `2026-11-18`
- **Target Milestone (Awaiting PO Approval):** `Milestone R16` — Wave 15 Composable Signal-Catalog Rule Strategy Builder, Multi-Condition Causal Alert Ledger & Export, & Crash-Safe Workspace Snapshot Auto-Recovery

---

## 1. Self-Start Entry Protocol (Zero Copy-Paste Needed)

Any successor Orchestrator session starting in `/Users/mizuhara/workspace/sumi` must first read:

1. [`docs/operations/SUMI_ORCHESTRATION_CHARTER.md`](operations/SUMI_ORCHESTRATION_CHARTER.md) — Role authority, subagent protocol, evidence classification (`VERIFIED` / `INFERRED` / `UNVERIFIED` / `BLOCKED`), per-batch Git commit policy (`DEC-R04-001`), and mandatory Vietnamese communication with the Product Owner.
2. [`docs/EXECUTION_STATE.md`](EXECUTION_STATE.md) — Current baseline (`IMPLEMENTATION_PASS` across Waves 1–14 / Milestones R03–R15).
3. [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) & [`docs/DECISION_LOG.md`](DECISION_LOG.md) — Modular `src/data/`, `src/viewport/`, `src/indicators/`, `src/drawings/`, `src/replay/`, `src/trading/`, `src/scanner/` (including `breadth.rs` & `catalog.rs`), `src/alerts/`, `src/backtest/` (including `walk_forward.rs` & `monte_carlo.rs`), `src/journal/`, multi-pane `MultiPaneLayout` + synchronized multi-chart grid, `PlotDataLayerMemo`, `OffscreenWgpuHarness`, and `DECISION-RENDERER-002` (`< 4.0 ms` CPU frame budget).
4. [`docs/research/RESEARCH_TO_RUST_ROADMAP.md`](research/RESEARCH_TO_RUST_ROADMAP.md) & [`docs/research/SIGNALS_INSPECTOR_AND_MONTE_CARLO_SPEC.md`](research/SIGNALS_INSPECTOR_AND_MONTE_CARLO_SPEC.md) — Shipped Wave 1–14 specifications and proposed Wave 15 scope.

---

## 2. Current Baseline Verified in R15

- **Build, WASM & Tests:** `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --lib --target wasm32-unknown-unknown -- -D warnings`, and `cargo test` (`216/216` unit + property + headless multi-pane/multi-chart/DOM/hotkey/confluence/MC UI + Critic R03–R15 regression tests) pass with `0` warnings and `0` errors; `Cargo.toml` and `Cargo.lock` have `0` new dependencies.
- **Shipped Subsystems (`R03–R15`):**
  - `src/data/` (Hinnant O(1) UTC calendar, RFC-4180 CSV & recursive-descent JSON kline/envelope parsers at `103.7–191.1 MiB/s`, `Vn30Basket` + `CafeF`/`SPY`/`BTCUSDT` fixtures, tail-coverage-scored benchmark lookup, `1m..1W` upward lattice, `aggregate_ohlcv` + causal `aggregate_ohlcv_up_to`)
  - `src/viewport/` (`TimeScale`, `PriceScale` + `CompactVolume` + `axis_edge_pad_px` / `scale_decimals_for_range_and_step` / `format_price_for_tick_density`, `PaneViewport`, `KineticAnimation`, `MultiChartLayoutKind` + `MultiChartSyncConfig` + `causal_closed_prefix_len`, `WorkspaceTemplate` `schema_version = 1` strict `RON`/`JSON` codecs + `6` built-in templates, & `HotkeyProfile` conflict-free `17`-action shortcut engine)
  - `src/indicators/` (`SMA`, `EMA`, `BollingerBands`, `RSI`, `MACD`, `ATR`, `IndicatorCache`, `SeriesBlockIndex` & `CandleBlockIndex`, `VRVP`/`FRVP`, Welford Multi-Interval/Anchored `VWAP` $\pm 1\sigma/\pm 2\sigma$, `DEF-001` causal `RS vs VNINDEX`, `Ichimoku Cloud`, `CVD`, & `Session TPO / Market Profile` + `HistoricalNakedPoc`)
  - `src/scanner/` (`evaluate_symbol_scan_and_signals`, `breadth.rs` causal `AT05`/`AT09`/`AT17` Breadth & JdK `RRG`, & **R15** `catalog.rs` `17`-entry `SIGNAL_CATALOG`, `SignalQuality`, 4-family `evaluate_causal_confluence`, `explain_historical_signal`, & `ScannerSignalContext::revealed_explanation`)
  - `src/drawings/` (`8` geometries across `4` panes, `Drawing::is_causally_eligible` vs `is_causally_visible`, & **R15** transactional `DrawingPropertyPatch` + `apply_drawing_property_patch` on 64-step Undo/Redo)
  - `src/backtest/` (`6` strategy presets, `Open[T+1]` causal fills, `walk_forward.rs` `WFO` + `MultiAssetPortfolio`, & **R15** `monte_carlo.rs` deterministic `SplitMix64` bootstrap `VaR`/`CVaR` `95%`/`99%` + bar-aligned `MonteCarloFanSeries` across Single BT, `WalkForwardOos`, and `MultiAssetPortfolio`)
  - `src/replay/`, `src/trading/` (incl. `ladder.rs`), `src/alerts/`, & `src/journal/` (all Pure `f64`/`i64` WASM-safe domain engines)
  - `src/chart.rs`, `src/app.rs` & `src/main.rs` (Synchronized multi-chart slots, `DOM Ladder`, `[Keys]` & `Signal Catalog` modals, `Drawing Inspector`, `OscillatorPaneId::Equity` `p5..p95`/`p25..p75`/dashed `p50` fan + `+0.00%` baseline badge, `collect_trading_axis_pill_ys` -> `main_occluding_ys`, and `--r15` holding all workloads `< 4.0 ms`: `0.337–1.557 ms` mean / `p99 <= 2.098 ms`, `"overall_pass": true`)

---

## 3. Recommended R16 Plan (`Wave 15 — Composable Signal Strategy Builder, Multi-Condition Alert Ledger & Snapshot Auto-Recovery`)

### Workstream 1 — Research Gate (`DEC-005` Compliance)
1. Execute live REA MCP JS/TS analysis on `ARCHIVE/frontend/src/components/strategy/{RuleBuilder,ConditionGroup,StrategyEditorModal}.tsx` (or equivalent strategy-AST / alert-rule / persistence modules in `ARCHIVE/frontend` + `LWC v5.2.0` marker/series primitives); author the R16 specification in `docs/research/`.

### Workstream 2 — Wave 15 Domain & UI Enhancements
1. **Composable Signal-Catalog Rule Strategy Builder (`src/backtest/`, `src/scanner/catalog.rs`, `src/app.rs`):** Allow combining `SIGNAL_CATALOG` predicates (`AND`/`OR` entry & exit clauses with tunable thresholds and `ConfluenceScore >= X` filters) directly inside `Strategy Lab`, feeding `Open[T+1]` backtests, `WFO`, and `SplitMix64` Monte Carlo risk envelopes with zero look-ahead (`DEF-001`).
2. **Multi-Condition Causal Alert Ledger & Formula-Safe Export (`src/alerts/`, `src/app.rs`):** Extend `AlertEngine` with `SignalCatalog` / `ConfluenceScore` alert conditions, cooldown bars, structured trigger audit log, and formula-safe RFC-4180 CSV / JSON alert ledger export.
3. **Workspace Snapshot Checkpoints & Corrupt-State Recovery (`src/viewport/workspace_template.rs`, `src/app.rs`):** Named local snapshot slots + schema-validated import/export diff preview & deterministic fallback diagnostics.

### Workstream 3 — `--r16` Release Benchmark, Offscreen Metal Captures, Critic, Audit & Clean Commit
1. Pass all 4 Cargo gates (`fmt`, native + `wasm32` `clippy -D warnings`, `cargo test`), run `--release` `--r16` (`< 4.0 ms` CPU frame budget) + `SUMI_CAPTURE_BATCH=r16`, pass `sumi-critic` (`CRITIC_PASS`) and `sumi-auditor` (`AUDIT_PASS`), and commit cleanly on `master` per `DEC-R04-001`.

---

## 4. Standing Constraints & Product Owner Gates

- **Product Owner Authorization Gate:** Wait for explicit Product Owner approval before starting Milestone R16 implementation.
- **Language:** Communicate 100% in **Vietnamese (`tiếng Việt`)** with the Product Owner.
- **Git & File Safety (`DEC-R04-001`):** Never touch `Cargo.toml`, `Cargo.lock`, `src/model.rs`, `ARCHIVE/`, or R01–R15 `docs/evidence/*`; never run `git reset`, `git clean`, `git push`, or `git tag`.
- **Authorized Targets Only (`DEC-006`):** Never inspect or interact with TradingView Desktop.
