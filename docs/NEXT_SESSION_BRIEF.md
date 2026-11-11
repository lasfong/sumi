# SUMI TRADING WORKSTATION — NEXT SESSION BRIEF (`SESSION R05 / WAVE 3`)

- **Prepared By:** `SESSION-R04-20261111T140100+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Date:** `2026-11-11`
- **Target Milestone:** `Milestone R05` — Wave 3 Interactive Drawing & Annotation Engine (`HorizontalLine`, `TrendLine`, `Ray`, `Rectangle`, `FibonacciRetracement`)

---

## 1. Self-Start Entry Protocol (Zero Copy-Paste Needed)

Any successor Orchestrator session starting in `/Users/mizuhara/workspace/sumi` must first read:

1. [`docs/operations/SUMI_ORCHESTRATION_CHARTER.md`](operations/SUMI_ORCHESTRATION_CHARTER.md) — Role authority, subagent protocol, evidence classification (`VERIFIED` / `INFERRED` / `UNVERIFIED` / `BLOCKED`), per-batch Git commit policy (`DEC-R04-001`), and mandatory Vietnamese communication with the Product Owner.
2. [`docs/EXECUTION_STATE.md`](EXECUTION_STATE.md) — Current baseline (`IMPLEMENTATION_PASS` on Waves 1, 2, and 4 / Milestone R04).
3. [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) & [`docs/DECISION_LOG.md`](DECISION_LOG.md) — Modular `src/viewport/`, `src/indicators/`, `MultiPaneLayout`, and `DECISION-RENDERER-002` (`< 4.0 ms` CPU frame budget).
4. [`docs/research/RESEARCH_TO_RUST_ROADMAP.md`](research/RESEARCH_TO_RUST_ROADMAP.md) — Wave 3 & Wave 5 specifications.

---

## 2. Current Baseline Verified in R04

- **Build, WASM & Tests:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --target wasm32-unknown-unknown -- -D warnings`, and `cargo test` (`46/46` unit + property + headless multi-pane UI + Critic R03/R04 regression tests) pass with `0` warnings and `0` errors.
- **Shipped Subsystems:**
  - `src/viewport/` (`TimeScale`, `PriceScale` + `sumi_oscillator`, `PaneViewport`, `KineticAnimation`, `candle_metrics`)
  - `src/indicators/` (`SMA`, `EMA`, `BollingerBands`, `RSI`, `MACD`, `ATR`, `IndicatorCache`, `SeriesBlockIndex`, conflated envelopes)
  - `src/chart.rs` (`MultiPaneLayout`, `render_workstation_chart`, interactive `5.0 px` splitters, `egui::Mesh` batching, synchronized cross-pane crosshair & HUDs)
  - `src/app.rs` & `src/main.rs` (indicator toggles, dataset/scale presets, `.with_active(false)` `SUMI_CAPTURE_BATCH` automation)
  - `examples/viewport_bench.rs` (`0.134 ms` 4-pane standard, `3.141 ms` `100K` 4-pane full-fit)

---

## 3. Recommended R05 Execution Plan (`Wave 3 — Drawing Engine`)

### Workstream 1 — Research Gate (`DEC-005` Compliance)
1. Execute live REA MCP `analyze_javascript_application` and `trace_javascript_semantics` on archived `ARCHIVE/frontend/src/domain/drawingDomain.ts`, `drawingGeometry.ts`, `useDrawingWorkspaceController.ts`, and `LWC v5.2.0` primitive pane-view interfaces (`src/model/ipane-primitive.ts`, `src/model/iseries-primitive.ts`).
2. Author `docs/research/DRAWING_ENGINE_SPEC.md` specifying logical `(bar_index: f64, price: f64)` anchor coordinates (so drawings stay locked to candles across zoom, pan, `Log`/`%`/`Idx100` scale switches, and `right_offset` future whitespace), analytical ray/line clipping to `PlotRect`, hit-test tolerances (anchor handle priority over segment/body), and Fibonacci retracement levels (`0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0`).

### Workstream 2 — Pure `f64` WASM-Safe `src/drawings/` Implementation
1. Implement `src/drawings/` with zero OS dependencies (`wasm32-unknown-unknown`-verified) + `serde::Serialize`/`Deserialize`:
   - Primitives: `HorizontalLine`, `TrendLine`, `Ray`, `Rectangle`, `FibonacciRetracement`.
   - Exact screen-space projection through `PaneViewport` (respecting `Logarithmic`, `Percentage`, and `IndexedTo100` transforms on any target pane).
   - Analytical hit-testing (`AnchorHandle` priority `<= 7 px`, segment/edge `<= 5 px`, rectangle interior) and deterministic creation/selection/drag-move state machine.
2. Add comprehensive unit, property, and headless multi-pane interaction tests.

### Workstream 3 — Workstation UI Integration, Critic, Audit & Clean Commit
1. Add drawing tool palette (`Cursor`, `H-Line`, `Trend`, `Ray`, `Rect`, `Fib`, `Delete` / `Clear`), live rubber-band preview during 2-click placement, hover/selection anchor handles, keyboard `Delete`/`Backspace`/`Escape` shortcuts, and `eframe` persistence.
2. Run `--release` benchmark + `.with_active(false)` `SUMI_CAPTURE_BATCH=r05` Retina captures, execute `sumi-critic` + `sumi-auditor`, and create a single clean commit on `master` per `DEC-R04-001`.

---

## 4. Standing Constraints & Product Owner Gates

- **Language:** Communicate 100% in **Vietnamese (`tiếng Việt`)** with the Product Owner.
- **Git & File Safety (`DEC-R04-001`):** Create one clean commit at the end of each verified batch/milestone; never run `git reset`, `git clean`, `git push`, or delete `ARCHIVE/`, `ViewportMapping`, or prior `docs/evidence/*`.
- **Authorized Targets Only (`DEC-006`):** Never inspect or interact with TradingView Desktop.
