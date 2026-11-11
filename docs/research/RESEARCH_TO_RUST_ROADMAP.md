# RESEARCH TO RUST ROADMAP — SESSION R01

- **Session Identity:** `SESSION-R01-20261110T202837+0700`
- **Updated By:** `SESSION-R03-20261111T105832+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Principle:** No Rust implementation begins without prior REA or reference-source research evidence.

---

## Wave 0 — Current State (Session 01 Foundation — COMPLETE)

| Component | Status | Evidence |
|---|---|---|
| `model.rs` — `Candle`, `PriceBounds`, `ViewportMapping` | ✅ SHIPPED | GUI PNG + telemetry JSON |
| `data.rs` — deterministic 120-bar VNINDEX synthetic feed | ✅ SHIPPED | cargo test 6/6 |
| `chart.rs` — candlestick, volume, axes, crosshair | ✅ SHIPPED | GUI PNG |
| `app.rs` — SumiWorkstationApp, zoom/pan toolbar | ✅ SHIPPED | GUI PNG |
| REA integration | ✅ REGISTERED | rea doctor healthy |

---

## Wave 1 — Modular Viewport Engine Research, Audit & Implementation (`COMPLETE — SESSION R02 + R03`)

### Completed Research & Audit Gates (`VERIFIED`)
- [x] Cloned `lightweight-charts` v5.2.0 (`868cae2`, Apache 2.0) to `research_repos/lightweight-charts-5.2.0`.
- [x] Executed 15 live REA MCP JS/TS operations across `src/model/`, `src/gui/`, and `src/renderers/` (`ev_eea4438f...`, `ev_97bce949...`, `ev_632b75be...`, etc.), plus 3 live Mach-O/Rust operations on the R03 release binary (`18` total REA MCP operations in R03).
- [x] Audited and rectified all 11+ coordinate, autoscale, conflation, and kinetic algorithms in `COORDINATE_ALGORITHM_SPEC.md` and `RUST_VIEWPORT_CAMERA_DESIGN.md`.
- [x] Benchmarked `egui::Painter` across `10K`, `50K`, and `100K` bars (`examples/viewport_bench.rs` → `CHART_RENDERER_ARCHITECTURE_DECISION.md`).

### Shipped Rust Deliverables (`IMPLEMENTATION_PASS`)
| Component | Source Reference | Shipped Rust Target | Status |
|---|---|---|---|
| Shared `TimeScale` + O(1) culling + DPR conflation | LWC `time-scale.ts`, `time-scale-visible-range.ts` | `src/viewport/time_scale.rs` | ✅ SHIPPED |
| Per-pane/axis `PriceScale` (`Linear`, `Log`, `%`, `Idx100`) | LWC `price-scale.ts`, `price-scale-conversions.ts` | `src/viewport/price_scale.rs` | ✅ SHIPPED |
| Explicit 3-space `PaneViewport` (`Logical <-> Local <-> Screen`) | LWC `pane.ts`, `pane-widget.ts` | `src/viewport/pane_viewport.rs` | ✅ SHIPPED |
| `KineticAnimation` inertial scroll (`right_offset` bar units) | LWC `kinetic-animation.ts`, `pane-widget.ts` | `src/viewport/kinetic.rs` | ✅ SHIPPED |
| Optimal candlestick/wick parity + zero-alloc conflation | LWC `optimal-bar-width.ts`, `candlesticks-renderer.ts` | `src/viewport/candle_metrics.rs` | ✅ SHIPPED |
| End-to-end 3-zone interactive chart + 100K dataset presets | `ChartViewportState` + `SumiWorkstationApp` | `src/chart.rs`, `src/app.rs`, `src/data.rs` | ✅ SHIPPED |

### Verified Acceptance Tests (`cargo test` — 29/29 PASS)
```bash
cargo test viewport::tests::lwc_fixture_index_to_local_x_and_half_slot_float_index
cargo test viewport::tests::lwc_fixture_cursor_anchored_zoom_exact
cargo test viewport::tests::property_time_scale_slot_round_trip_across_2000_states
cargo test viewport::tests::property_cursor_anchored_zoom_locks_bar_under_pointer
cargo test viewport::tests::property_price_scale_round_trip_all_modes_and_inversions
cargo test viewport::tests::multi_pane_shared_time_scale_and_screen_offset_isolation
cargo test viewport::tests::large_dataset_100k_bars_culling_and_conflation_invariants
cargo test viewport::tests::critic_regression_extreme_zoom_out_clamps_to_min_bar_spacing_not_6px
cargo test chart::tests::headless_ui_hover_wheel_zoom_and_drag_pan_interactions
cargo test chart::tests::headless_inverted_scale_preserves_candlestick_wick_segments
```

---

## Wave 2 — Indicator Engine (Session 04 Target, parallelizable with Wave 1)

### Pre-condition: Research Gate
- [ ] Read TA-Lib C source for: SMA, EMA, RSI, MACD, Bollinger Bands, ATR
- [ ] Read archived `indicatorDomain.ts` for placement model (already recovered — see CHART_ENGINE_RESEARCH_MAP.md §Subsystem 3)
- [ ] Define Rust `Indicator` trait before any concrete implementation

### Deliverables
| Component | Algorithm Source | Rust Target |
|---|---|---|
| `Indicator` trait | Defined from research | `src/indicators.rs` |
| `Sma`, `Ema` | TA-Lib source, Wilder formula | `src/indicators/moving_averages.rs` |
| `Rsi` | Wilder 1978, TA-Lib | `src/indicators/oscillators.rs` |
| `BollingerBands` | Keltner/Bollinger papers, TA-Lib | `src/indicators/bands.rs` |
| `Macd` | Standard formula, TA-Lib | `src/indicators/macd.rs` |

### Acceptance Tests
```
cargo test indicators::sma_basic
cargo test indicators::ema_warm_up
cargo test indicators::rsi_wilder_reference
cargo test indicators::bollinger_expansion
```

---

## Wave 3 — Drawing Engine Port (Session 05 Target)

### Pre-condition: Research Gate
- [ ] REA `analyze_javascript_application` on `lightweight-charts` source: Extract `ISeriesPrimitive`, `PrimitivePaneView` interfaces
- [ ] Confirm drawing anchor schema from archived `drawingDomain.ts`
- [ ] Review `drawingGeometry.ts` algorithms (all already recovered — see REA_EXECUTION_EVIDENCE.md §Record 4)

### Deliverables
| Component | Algorithm Source | Rust Target |
|---|---|---|
| `Drawing` enum + anchor model | Archive `drawingDomain.ts` | `src/drawings.rs` |
| Projection + hit-test | Archive `drawingGeometry.ts` (fully recovered) | `src/drawings.rs` |
| Creation state machine | Archive `useDrawingWorkspaceController.ts` | `src/drawing_input.rs` |
| Fibonacci retracement | Archive `fibonacciPrice` formula | `src/drawings.rs` |

### Acceptance Tests
```
cargo test drawings::hit_test_trendline
cargo test drawings::hit_test_anchor_priority
cargo test drawings::fibonacci_levels
cargo test drawings::ray_projection
```

---

## Wave 4 — Multi-Pane Layout (Session 06 Target)

### Pre-condition: Research Gate
- [ ] REA `analyze_javascript_application` on lightweight-charts source: Extract `IPane`, `chart.panes()`, `stretchFactor` APIs
- [ ] Confirm `PaneManager.ts` from archive

### Deliverables
| Component | Research Source | Rust Target |
|---|---|---|
| `PaneLayout` struct | lightweight-charts IPane + archive PaneManager | `src/panes.rs` |
| Oscillator pane rendering | Archive `IndicatorRenderInput` schema | `src/chart.rs` refactor |
| Shared time axis | lightweight-charts time-scale sync | `src/chart.rs` |
| Crosshair sync | lightweight-charts crosshair | `src/chart.rs` |

---

## Wave 5 — Data Infrastructure (Parallelizable, Session 03+)

### Deliverables
| Component | Research Source | Rust Target |
|---|---|---|
| CSV/JSON OHLCV loader | `lightweight-charts` `SeriesData` format docs | `src/data.rs` extension |
| Timeframe aggregation | Standard OHLCV rollup (open-source) | `src/data.rs` |
| WASM gate | cargo check with `wasm32-unknown-unknown` | CI verification |

---

## Rust Architecture Invariants (Session R01 Decision)

The following architectural decisions are locked from Session 01 and confirmed by Session R01 research:

1. **Single flat crate** (`sumi-workstation`) — no workspace split until complexity justifies it.
2. **`egui`/`eframe` for rendering** — confirmed as the right choice by REA binary evidence (Metal backend via wgpu, CVDisplayLink timing).
3. **`model.rs` and future `indicators.rs` must be `wasm32-unknown-unknown` compatible** — zero OS dependencies in domain logic.
4. **RON persistence** — already compiled in (confirmed by REA symbol recovery); continue using eframe's built-in persistence.
5. **No architectural rewrites before research gates are cleared** — each Wave begins only after the specified REA investigation is complete.

---

## Tools Justified for Future Sessions

| Tool | Justification | Priority |
|---|---|---|
| **Ghidra 12.1.x** (free) | Unlocks `list_procedures`, `procedure_pseudo_code`, `search_strings` on Rust binaries — required for release-build analysis and future wgpu shader inspection | High |
| **lightweight-charts local clone** | Authorized Apache 2.0 target for `analyze_javascript_application` — provides complete reference architecture for Waves 1-4 | ✅ Completed (`research_repos/lightweight-charts-5.2.0`, `v5.2.0` `868cae2`) |
| **TA-Lib source** | BSD-licensed reference for all 18 indicator implementations | Immediate (Wave 2) |
