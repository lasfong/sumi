# SUMI TRADING WORKSTATION — NEXT SESSION BRIEF (`SESSION R06 / WAVE 5`)

- **Prepared By:** `SESSION-R05-20261111T163600+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Date:** `2026-11-11`
- **Target Milestone:** `Milestone R06` — Wave 5 CSV/JSON OHLCV Data Loader & Multi-Timeframe Aggregation (`1m`, `5m`, `15m`, `1h`, `4h`, `1D`, `1W`)

---

## 1. Self-Start Entry Protocol (Zero Copy-Paste Needed)

Any successor Orchestrator session starting in `/Users/mizuhara/workspace/sumi` must first read:

1. [`docs/operations/SUMI_ORCHESTRATION_CHARTER.md`](operations/SUMI_ORCHESTRATION_CHARTER.md) — Role authority, subagent protocol, evidence classification (`VERIFIED` / `INFERRED` / `UNVERIFIED` / `BLOCKED`), per-batch Git commit policy (`DEC-R04-001`), and mandatory Vietnamese communication with the Product Owner.
2. [`docs/EXECUTION_STATE.md`](EXECUTION_STATE.md) — Current baseline (`IMPLEMENTATION_PASS` on Waves 1, 2, 3, and 4 / Milestone R05).
3. [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) & [`docs/DECISION_LOG.md`](DECISION_LOG.md) — Modular `src/viewport/`, `src/indicators/`, `src/drawings/`, `MultiPaneLayout`, and `DECISION-RENDERER-002` (`< 4.0 ms` CPU frame budget).
4. [`docs/research/RESEARCH_TO_RUST_ROADMAP.md`](research/RESEARCH_TO_RUST_ROADMAP.md) — Wave 5 specifications.

---

## 2. Current Baseline Verified in R05

- **Build, WASM & Tests:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --target wasm32-unknown-unknown -- -D warnings`, and `cargo test` (`57/57` unit + property + headless multi-pane UI + Critic R03/R04/R05 regression tests) pass with `0` warnings and `0` errors.
- **Shipped Subsystems:**
  - `src/viewport/` (`TimeScale`, `PriceScale` + `sumi_oscillator`, `PaneViewport`, `KineticAnimation`, `candle_metrics`)
  - `src/indicators/` (`SMA`, `EMA`, `BollingerBands`, `RSI`, `MACD`, `ATR`, `IndicatorCache`, `SeriesBlockIndex`, conflated envelopes)
  - `src/drawings/` (`HorizontalLine`, `TrendLine`, $360^\circ$ `Ray`, `Rectangle`, `FibonacciRetracement`, `ClipEdge` Liang-Barsky clipping, LWC hit-test priority, OHLC magnet, 64-step Undo/Redo, `serde` persistence)
  - `src/chart.rs` (`MultiPaneLayout`, `render_workstation_chart_with_drawings` / `render_workstation_chart`, multi-pane drawing arbitrage & phase-anchored/Mesh GPU rendering, interactive `5.0 px` splitters, synchronized crosshair & HUDs)
  - `src/app.rs` & `src/main.rs` (indicator + drawing toolbars, contextual inspector, `.with_active(false)` `SUMI_CAPTURE_BATCH` automation)
  - `examples/viewport_bench.rs` (`0.210 ms` standard, `0.572 ms` dense, `3.521 ms` `100K` 4-pane + 6-indicator + 25-drawing full-fit)

---

## 3. Recommended R06 Execution Plan (`Wave 5 — Data Infrastructure`)

### Workstream 1 — Research Gate (`DEC-005` Compliance)
1. Execute live REA MCP analysis on archived `ARCHIVE/frontend` market-data / timeframe / series parsing modules and `LWC v5.2.0` `SeriesData` / time-tick formatting contracts (`src/model/horz-scale-behavior-time/*`), plus inspect `ARCHIVE/backend` CSV/JSON schemas if present.
2. Author `docs/research/DATA_LOADER_AND_TIMEFRAME_SPEC.md` defining:
   - Zero-panic streaming/in-memory CSV & JSON OHLCV parsers (`ISO-8601`, `YYYY-MM-DD`, Unix seconds/milliseconds, strict `high >= max(open, close)` & `low <= min(open, close)` validation, chronological sorting & duplicate-timestamp policy).
   - Deterministic OHLCV timeframe aggregation (`1m`, `5m`, `15m`, `1h`, `4h`, `1D`, `1W`) with UTC/session bucket alignment and timestamp-anchored `DrawingAnchor` remapping across timeframe switches.

### Workstream 2 — Pure `f64` WASM-Safe `src/data/` Implementation
1. Extend `src/data.rs` (or modular `src/data/`) with `wasm32-unknown-unknown`-compatible CSV & JSON parsers (`serde_json` + zero-dependency RFC-4180 CSV parser) and `Timeframe` (`1m`, `5m`, `15m`, `1h`, `4h`, `1D`, `1W`) OHLCV rollup engine.
2. Preserve timestamp continuity for drawings and viewport visible range when switching timeframes on the same symbol.

### Workstream 3 — Workstation UI Integration, Critic, Audit & Clean Commit
1. Add timeframe selector (`1m`, `5m`, `15m`, `1h`, `4h`, `1D`, `1W`), local CSV/JSON file import + bundled sample market fixtures, and adaptive intraday/daily time-axis formatting (`HH:MM` vs `YYYY-MM-DD`).
2. Run `--release` `--r06` benchmark + `.with_active(false)` `SUMI_CAPTURE_BATCH=r06` Retina captures, execute `sumi-critic` + `sumi-auditor`, and create a single clean commit on `master` per `DEC-R04-001`.

---

## 4. Standing Constraints & Product Owner Gates

- **Language:** Communicate 100% in **Vietnamese (`tiếng Việt`)** with the Product Owner.
- **Git & File Safety (`DEC-R04-001`):** Create one clean commit at the end of each verified batch/milestone; never run `git reset`, `git clean`, `git push`, or delete `ARCHIVE/`, `ViewportMapping`, or prior `docs/evidence/*`.
- **Authorized Targets Only (`DEC-006`):** Never inspect or interact with TradingView Desktop.
