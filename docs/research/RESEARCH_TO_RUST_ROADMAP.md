# RESEARCH TO RUST ROADMAP — SESSION R01

- **Session Identity:** `SESSION-R01-20261110T202837+0700`
- **Updated By:** `SESSION-R04-20261111T140100+0700` (`SUMI PROGRAM ORCHESTRATOR`)
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

## Wave 2 — Indicator Engine (`COMPLETE — SESSION R04`)

### Completed Research & Audit Gates (`VERIFIED`)
- [x] Cloned official BSD-licensed `TA-Lib/ta-lib` (`fd57d4cfd6c812004d897feb0025fe273b1192e2`) to `research_repos/ta-lib` and verified `ta_SMA.c`, `ta_EMA.c`, `ta_BBANDS.c`, `ta_RSI.c`, `ta_MACD.c`, `ta_ATR.c`, and `ta_TRANGE.c`.
- [x] Executed live REA MCP JS/TS analysis (`ev_8737910237dddc3c6f846a2eb10db4023cf433ad7e2b8f5974081656c20e73c5`, `ev_380c6a728de9a73e5437c9baa4313a71002783978dea75c9d997b2ff1a7a1a89`, `ev_f46a1dfda7613ec07a0ea58f94b9b5e1df7fe89319a4df95bf3e5a00d11c92c5`) on archived `indicatorDomain.ts`, `PaneManager.ts`, `IndicatorRenderRegistry.ts`, and `IndicatorPaneChrome.tsx`.
- [x] Authored complete mathematical & rendering specification in [`docs/research/INDICATOR_AND_MULTIPANE_SPEC.md`](INDICATOR_AND_MULTIPANE_SPEC.md) and defined pure `f64` WASM-safe `Indicator` trait before concrete implementation.

### Shipped Rust Deliverables (`IMPLEMENTATION_PASS`)
| Component | Algorithm Source | Shipped Rust Target | Status |
|---|---|---|---|
| `Indicator` trait, `IndicatorCache`, `SeriesBlockIndex` & conflated envelopes | `INDICATOR_AND_MULTIPANE_SPEC.md` | `src/indicators/mod.rs` | ✅ SHIPPED |
| `SmaIndicator` (cancellation reseed), `EmaIndicator` (Sterbenz weights) | TA-Lib `ta_SMA.c`, `ta_EMA.c` | `src/indicators/moving_averages.rs` | ✅ SHIPPED |
| `BollingerBandsIndicator` (`20, 2.0` population $\sigma$, scaled fallback) | TA-Lib `ta_BBANDS.c` | `src/indicators/bands.rs` | ✅ SHIPPED |
| `RsiIndicator`, `MacdIndicator`, `AtrIndicator` & reference levels | Wilder 1978, TA-Lib `ta_RSI.c`, `ta_MACD.c`, `ta_ATR.c` | `src/indicators/oscillators.rs` | ✅ SHIPPED |

### Verified Acceptance Tests (`cargo test` — 46/46 PASS)
```bash
cargo test indicators::tests::sma_basic_and_long_series_drift_invariance
cargo test indicators::tests::ema_warm_up_seed_and_zero_ulp_flat_invariance
cargo test indicators::tests::bollinger_population_stddev_and_volatility_expansion
cargo test indicators::tests::rsi_wilder_reference_and_ta_lib_edge_cases
cargo test indicators::tests::macd_alignment_period_swap_and_signal_one_branch
cargo test indicators::tests::atr_true_range_gaps_and_wilder_smoothing
cargo test indicators::tests::large_100k_cache_and_right_aligned_conflation_pan_invariance
cargo test indicators::tests::series_block_index_matches_raw_slice_scans_bit_for_bit
```

---

## Wave 3 — Drawing Engine Port (`NEXT — MILESTONE R05 TARGET`)

### Pre-condition: Research Gate
- [ ] REA `analyze_javascript_application` on `lightweight-charts` source: Extract `ISeriesPrimitive`, `PrimitivePaneView` interfaces
- [ ] Confirm drawing anchor schema from archived `drawingDomain.ts`
- [ ] Review `drawingGeometry.ts` algorithms (all already recovered — see REA_EXECUTION_EVIDENCE.md §Record 4)

### Deliverables
| Component | Algorithm Source | Rust Target |
|---|---|---|
| `Drawing` enum + anchor model | Archive `drawingDomain.ts` | `src/drawings/mod.rs` |
| Projection + hit-test | Archive `drawingGeometry.ts` (fully recovered) | `src/drawings/geometry.rs` |
| Creation state machine | Archive `useDrawingWorkspaceController.ts` | `src/drawings/controller.rs` |
| Fibonacci retracement | Archive `fibonacciPrice` formula | `src/drawings/fibonacci.rs` |

### Acceptance Tests
```
cargo test drawings::hit_test_trendline
cargo test drawings::hit_test_anchor_priority
cargo test drawings::fibonacci_levels
cargo test drawings::ray_projection
```

---

## Wave 4 — Multi-Pane Layout (`COMPLETE — SESSION R04`)

### Completed Research & Audit Gates (`VERIFIED`)
- [x] Executed live REA MCP JS/TS analysis (`ev_3a1d7aeb2dbcb801ac28e38de3618addf25fb6216f117f3f4b1d9048e9591111`, `ev_4507f2ade5d47a54b59a7589f180e0ec5012d6e6ff86d4fc59cd5f74a1d72565`, `ev_5f12ebbfe8f69dabc129ef25dab6fff37eea11945af331e34760cda658e6260f`) on LWC v5.2.0 `pane.ts`, `pane-separator.ts`, `chart-widget.ts`, `line-pane-view.ts`, `histogram-pane-view.ts`, `histogram-renderer.ts`, and `walk-line.ts`.
- [x] Verified archived `PaneManager.ts` vs LWC single-canvas `stretchFactor` architecture (`DEC-R04-003`).

### Shipped Rust Deliverables (`IMPLEMENTATION_PASS`)
| Component | Research Source | Rust Target | Status |
|---|---|---|---|
| `MultiPaneLayout` (`stretch_factors`, water-fill `MultiPaneLayout::MIN_PANE_HEIGHT = 44.0`, splitters) | LWC `pane.ts`, `pane-separator.ts`, `chart-widget.ts` | `src/chart.rs` | ✅ SHIPPED |
| Oscillator pane rendering (`RSI`, `MACD`, `ATR`) + `egui::Mesh` batching | `INDICATOR_AND_MULTIPANE_SPEC.md` | `src/chart.rs`, `src/app.rs` | ✅ SHIPPED |
| Shared `TimeScale` + unified `conflation_factor` across all panes | LWC `chart-model.ts`, `time-scale.ts` | `src/chart.rs` | ✅ SHIPPED |
| Synchronized cross-pane vertical crosshair & per-pane price pill | LWC `crosshair.ts`, `pane-widget.ts` | `src/chart.rs` | ✅ SHIPPED |

---

## Wave 5 — Data Infrastructure (Parallelizable, Session R05+)

### Deliverables
| Component | Research Source | Rust Target | Status |
|---|---|---|---|
| CSV/JSON OHLCV loader | `lightweight-charts` `SeriesData` format docs | `src/data.rs` extension | Pending (Wave 5) |
| Timeframe aggregation | Standard OHLCV rollup (open-source) | `src/data.rs` | Pending (Wave 5) |
| WASM gate | `cargo clippy --target wasm32-unknown-unknown -- -D warnings` | Toolchain & CI verification | ✅ COMPLETE (`SESSION R04`) |

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
| **TA-Lib source** | BSD-licensed reference for all 18 indicator implementations | ✅ Completed (`research_repos/ta-lib`, `fd57d4cfd6c812004d897feb0025fe273b1192e2`) |
