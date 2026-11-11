# SUMI TRADING WORKSTATION — NEXT SESSION BRIEF (`SESSION R04 / WAVE 2`)

- **Prepared By:** `SESSION-R03-20261111T105832+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Date:** `2026-11-11`
- **Target Milestone:** `Milestone R04` — Wave 2 Indicator Engine (`SMA`, `EMA`, `RSI`, `MACD`, `BollingerBands`, `ATR`) & Multi-Pane Oscillator Rendering

---

## 1. Self-Start Entry Protocol (Zero Copy-Paste Needed)

Any successor Orchestrator session starting in `/Users/mizuhara/workspace/sumi` must first read:

1. [`docs/operations/SUMI_ORCHESTRATION_CHARTER.md`](operations/SUMI_ORCHESTRATION_CHARTER.md) — Role authority, subagent protocol, evidence classification (`VERIFIED` / `INFERRED` / `UNVERIFIED` / `BLOCKED`), and mandatory Vietnamese communication with the Product Owner.
2. [`docs/EXECUTION_STATE.md`](EXECUTION_STATE.md) — Current baseline (`IMPLEMENTATION_PASS` on Wave 1 / Milestone R03).
3. [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) & [`docs/DECISION_LOG.md`](DECISION_LOG.md) — Active modular `src/viewport/` boundaries and `DECISION-RENDERER-002` rules.
4. [`docs/research/RESEARCH_TO_RUST_ROADMAP.md`](research/RESEARCH_TO_RUST_ROADMAP.md) — Wave 2 & Wave 4 specifications.

---

## 2. Current Baseline Verified in R03

- **Build & Lint:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` (`29/29` unit + property + headless UI + critic regression tests) pass cleanly (`0` warnings, `0` errors).
- **Shipped Modules:**
  - `src/viewport/` (`TimeScale`, `PriceScale`, `PaneViewport`, `KineticAnimation`, `candle_metrics`, `tests`)
  - `src/chart.rs` (`ChartViewportState`, `render_shared_pane_candlestick_chart`, 3-zone interaction, O(1) culling, DPR conflation, `Linear`/`Log`/`%`/`Idx100` axes, crosshair & HUD)
  - `src/app.rs` (`120B` / `10K` / `50K` / `100K` dataset switcher, scale mode controls, live Metal GPU screenshot + JSON telemetry capture)
  - `examples/viewport_bench.rs` (`0.094–0.102 ms` standard view, `1.143 ms` 3-pane shared-`TimeScale` dense, `1.472 ms` 100K conflated fit)

---

## 3. Recommended R04 Execution Plan

### Workstream 1 — Research Gate (`DEC-005` Compliance)
1. Inspect archived `ARCHIVE/frontend/src/domain/indicatorDomain.ts` (read-only) via REA `analyze_javascript_application` / `trace_javascript_semantics` and direct source verification to confirm indicator parameter schemas, warmup semantics (`NaN` / `Option<f64>` prefix), and overlay vs separate-pane placement rules.
2. Document exact mathematical definitions, initialization/warmup edge cases (Wilder smoothing for `RSI`/`ATR` vs standard EMA `2/(N+1)`), and reference numerical test vectors in `docs/research/INDICATOR_ALGORITHM_SPEC.md`.

### Workstream 2 — Pure `f64` WASM-Safe `src/indicators/` Implementation
1. Implement `src/indicators/mod.rs` with zero OS/UI dependencies (`wasm32-unknown-unknown`-safe):
   - Overlay series: `Sma`, `Ema`, `BollingerBands` (middle, upper, lower).
   - Sub-pane oscillators: `Rsi` (Wilder 14), `Macd` (`12, 26, 9` line + signal + histogram), `Atr` (Wilder 14).
2. Write unit + property tests covering empty/short series (`len < period`), flat prices (zero stddev / zero loss in `RSI`), `100,000`-bar throughput, and exact reference fixtures.

### Workstream 3 — Multi-Pane Synchronization & Rendering Integration
1. Extend `ChartViewportState` to manage optional stacked sub-panes (`Vec<IndicatorPaneState>`), **sharing the single `TimeScale`** while giving each sub-pane its own `PriceScale` (`PaneViewport::new(&shared_time_scale, &sub_pane.price_scale, sub_rect, base_val)`).
2. Synchronize vertical crosshair `slot_idx` and `cross_x` simultaneously across the main candlestick pane and all active oscillator sub-panes.
3. Re-run `viewport_bench` (extended with active indicators) to confirm CPU frame time stays well below the `4.0 ms` `DECISION-RENDERER-002` trigger.

---

## 4. Standing Constraints & Product Owner Gates

- **Language:** Communicate 100% in **Vietnamese (`tiếng Việt`)** with the Product Owner.
- **Git & File Safety:** Never run `git commit`, `git tag`, `git reset`, `git clean`, or delete any files/code (`ARCHIVE/`, `ViewportMapping`, prior `docs/evidence/*`).
- **Authorized Targets Only:** Never inspect proprietary TradingView Desktop binaries (`DEC-006`).
- **Crate Gate:** Ask Product Owner approval before adding any new heavy external crate to `Cargo.toml`.
