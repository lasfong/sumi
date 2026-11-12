# RESEARCH TO RUST ROADMAP — SESSION R01

- **Session Identity:** `SESSION-R01-20261110T202837+0700`
- **Updated By:** `SESSION-R07-20261112T093742+0700` (`SUMI PROGRAM ORCHESTRATOR`)
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

## Wave 3 — Drawing Engine Port (`COMPLETE — SESSION R05`)

### Completed Research & Audit Gates (`VERIFIED`)
- [x] Executed live REA MCP JS/TS analysis (`ev_f8b8d8b5...`, `ev_4d2d8fb2...`, `ev_6041f2c6...`, `ev_01525e3b...`, `ev_8c1494cf...`, `ev_25a75e6c...`, `ev_98512b7e...`) on `lightweight-charts-5.2.0` (`iseries-primitive.ts`, `ipane-primitive.ts`, `pane-primitive-wrapper.ts`, `series-primitive-wrapper.ts`, `primitive-wrapper-base.ts`, `pane-widget.ts`) and `ARCHIVE/frontend/src/features/drawings/{drawingDomain,drawingGeometry,drawingMagnet,useDrawingWorkspaceController,SumiPrimitiveDrawingProvider,DrawingCommandHistory}.ts`.
- [x] Documented and rectified all 5 legacy Sumi drawing defects in [`docs/research/DRAWING_ENGINE_SPEC.md`](DRAWING_ENGINE_SPEC.md).

### Shipped Rust Deliverables (`IMPLEMENTATION_PASS`)
| Component | Algorithm Source | Shipped Rust Target | Status |
|---|---|---|---|
| `Drawing`, `DrawingAnchor`, `DrawingPaneTarget` & style schema | Archive `drawingDomain.ts` + multi-pane extension | `src/drawings/mod.rs` | ✅ SHIPPED |
| `PaneDrawingProjection`, `ClipEdge` Liang-Barsky & Fib levels | Archive `drawingGeometry.ts` + analytical clipping | `src/drawings/geometry.rs` | ✅ SHIPPED |
| LWC handle-first hit-test & `O(threshold / bar_spacing)` OHLC magnet | LWC `ipane-primitive.ts` + Archive `drawingMagnet.ts` | `src/drawings/hit_test.rs` | ✅ SHIPPED |
| `DrawingStore`, 1/2-click + drag state machine & 64-step Undo/Redo | Archive `useDrawingWorkspaceController.ts`, `DrawingCommandHistory.ts` | `src/drawings/controller.rs` | ✅ SHIPPED |
| Multi-pane GPU overlay, phase-anchored dashes, toolbar & inspector | `DRAWING_ENGINE_SPEC.md` | `src/chart.rs`, `src/app.rs` | ✅ SHIPPED |

### Verified Acceptance Tests (`cargo test` — 57/57 PASS)
```bash
cargo test drawings::tests::liang_barsky_clips_finite_segments_and_360_degree_rays_accurately
cargo test drawings::tests::fibonacci_levels_match_canonical_archive_ratios_and_direction_reversal
cargo test drawings::tests::hit_test_precedence_prevents_fib_and_unselected_rect_hijack_and_supports_lock_selection
cargo test drawings::tests::ohlc_magnet_snaps_within_threshold_and_breaks_ties_deterministically
cargo test drawings::tests::controller_supports_two_click_and_drag_creation_log_body_drag_and_bounded_undo_redo
cargo test drawings::tests::rectangle_four_corner_resize_and_oscillator_negative_values_and_sanitization_work
cargo test drawings::tests::critic_r05_regression_click_without_drag_preserves_geometry_and_redo_stack
cargo test drawings::tests::critic_r05_regression_liang_barsky_no_phantom_edge_ray_tiny_ray_and_extreme_span
cargo test drawings::tests::critic_r05_regression_locked_rectangle_deselect_pan_passthrough_and_transitive_ties
cargo test drawings::tests::critic_r05_regression_u64_max_id_compaction_and_cross_pane_drag_hover_isolation
cargo test chart::tests::multipane_drawing_arbitrage_creates_drags_selects_locked_and_preserves_pan
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

## Wave 5 — Data Infrastructure (`COMPLETE — SESSION R06`)

### Completed Research & Audit Gates (`VERIFIED`)
- [x] Executed 7 live REA MCP JS/TS operations (`9` exact 64-hex Evidence IDs: `ev_c3081a8a...`, `ev_3e08300a...`, `ev_a4dabc00...`, `ev_d57067b9...`, `ev_ab7d475a...`, `ev_288c5960...`, `ev_3a7da525...`, `ev_eeaad96e...`, `ev_64453400...`) across `ARCHIVE/frontend` data/timeframe pipeline (`7` files) and `lightweight-charts-5.2.0` `horz-scale-behavior-time` + `data-layer` (`7` files), plus 4 final binary REA Evidence IDs (`ev_2acf1d97...`, `ev_fd142ac6...`, `ev_d3f78364...`, `ev_6c511d74...`).
- [x] Authored [`docs/research/DATA_LOADER_AND_TIMEFRAME_SPEC.md`](DATA_LOADER_AND_TIMEFRAME_SPEC.md) defining zero-dependency CSV/JSON schemas, Hinnant O(1) UTC calendar bijection, `1m..1W` upward lattice, `DEF-001` causal replay aggregation, and timestamp-locked drawing continuity.

### Shipped Rust Deliverables (`IMPLEMENTATION_PASS`)
| Component | Research Source | Shipped Rust Target | Status |
|---|---|---|---|
| Hinnant O(1) UTC calendar + multi-format timestamp parser (`ISO-8601`, `CafeF`, epochs) | LWC `horz-scale-behavior-time.ts` + Archive CSVs | `src/data/calendar.rs` | ✅ SHIPPED |
| Streaming RFC-4180 CSV & recursive-descent JSON kline/envelope loaders (`103.7–191.1 MiB/s`) | LWC `data-layer.ts` + `CafeF.INDEX.clean.csv` / `SPY.csv` | `src/data/csv_loader.rs`, `src/data/json_loader.rs` | ✅ SHIPPED |
| Timeframe lattice (`1m`, `5m`, `15m`, `1h`, `4h`, `1D`, `1W`) + `DEF-001` `aggregate_ohlcv_up_to` | `DATA_LOADER_AND_TIMEFRAME_SPEC.md` | `src/data/timeframe.rs` | ✅ SHIPPED |
| Timestamp-locked `DrawingStore::remap_timeframe` + per-instrument `drawings_by_instrument` | `DATA_LOADER_AND_TIMEFRAME_SPEC.md` | `src/drawings/controller.rs`, `src/app.rs` | ✅ SHIPPED |
| Adaptive intraday/daily time axis + offscreen Metal `wgpu` Retina harness (`OffscreenWgpuHarness`) | LWC `horz-scale-behavior-time.ts` / `time-formatter.ts` + `egui_wgpu` | `src/chart.rs`, `src/app.rs`, `src/main.rs` | ✅ SHIPPED |
| WASM gate | `cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings` | Toolchain & CI verification | ✅ COMPLETE (`SESSIONS R04 + R05 + R06`) |

### Verified Acceptance Tests (`cargo test` — 72/72 PASS)
```bash
cargo test data::tests::proleptic_gregorian_calendar_bijection_and_leap_year_invariants
cargo test data::tests::multi_format_timestamp_parser_handles_cafef_iso_tz_and_nanosecond_epochs
cargo test data::tests::csv_loader_parses_rfc4180_delimiters_bom_quotes_dedup_and_envelope_policies
cargo test data::tests::json_loader_parses_objects_tuples_envelopes_escapes_and_rejects_deep_or_malformed_json
cargo test data::tests::timeframe_lattice_associativity_downsample_guard_and_def001_causality
cargo test data::tests::real_archive_cafef_multi_symbol_and_spy_dst_weekly_aggregation_invariants
cargo test data::tests::drawing_store_remap_timeframe_preserves_exact_anchors_across_round_trips_and_undo_redo
cargo test app::tests::r06_workstation_presets_csv_json_import_and_multi_timeframe_switching_preserve_focus_and_drawings
cargo test app::tests::critic_r06_regression_persisted_session_restore_preserves_4h_timeframe_drawings_and_missing_file_fallback
cargo test app::tests::critic_r06_regression_per_instrument_drawing_isolation_in_memory_multisymbol_and_cursor_pill_multiplier
```

---

## Wave 6 — Causal Bar Replay & Paper Trading Execution Engine (`COMPLETE — SESSION R07`)

### Completed Research & Audit Gates (`VERIFIED`)
- [x] Executed 10 live REA MCP JS/TS operations (`12` exact 64-hex Evidence IDs: `ev_10aea63b...`, `ev_d1e347a9...`, `ev_9527ede0...`, `ev_c102b1d9...`, `ev_418c861e...`, `ev_eaab36ba...`, `ev_4f49eb61...`, `ev_e1958292...`, `ev_81fa9171...`, `ev_7e0b29f4...`, `ev_3f2322b6...`, `ev_19311ed5...`) across `ARCHIVE/frontend` replay/trading pipeline (`9` files) and `lightweight-charts-5.2.0` `custom-price-line` + `series-markers` (`8` files), plus 4 final binary REA Evidence IDs (`ev_dbbcdc03...`, `ev_b158e5f3...`, `ev_59ad2ed9...`, `ev_5f9d71b0...`).
- [x] Authored [`docs/research/BAR_REPLAY_AND_PAPER_TRADING_SPEC.md`](BAR_REPLAY_AND_PAPER_TRADING_SPEC.md) rectifying all 6 legacy replay & paper-trading defects (`DEF-001` lookahead lock, partial higher-TF bars, deterministic command-log rewind, `Stop` orders + `Short` selling, 4-waypoint OHLC matching, causal drawing visibility, and type-safe Drawing -> Ticket sync).

### Shipped Rust Deliverables (`IMPLEMENTATION_PASS`)
| Component | Research Source | Shipped Rust Target | Status |
|---|---|---|---|
| Causal bar-by-bar replay controller (`DEF-001` zero-lookahead prefix rollup, `ActiveBar`/`BaseBar`) | `ARCHIVE/frontend` + `ARCHIVE/backend` replay contracts | `src/replay/` | ✅ SHIPPED |
| Paper trading order book (`Market`, `Limit`, `Stop`, `SL/TP`, 4-waypoint OHLC, fee conservation & `rewind_to`) | `BAR_REPLAY_AND_PAPER_TRADING_SPEC.md` | `src/trading/` | ✅ SHIPPED |
| Causal drawing visibility guards (`created_at_timestamp` + `is_causally_visible`) | `BAR_REPLAY_AND_PAPER_TRADING_SPEC.md` | `src/drawings/` | ✅ SHIPPED |
| On-chart Cut Scrubber, Replay Frontier, `egui::Mesh` price lines/fill arrows, Replay Dock & Trading Panel | `LWC v5.2.0` `CustomPriceLine` / `SeriesMarkers` + Archive UI | `src/chart.rs`, `src/app.rs` | ✅ SHIPPED |

### Verified Acceptance Tests (`cargo test` — 91/91 PASS)
```bash
cargo test replay::tests::def_001_zero_lookahead_slicing_across_all_higher_timeframes
cargo test replay::tests::active_bar_vs_base_bar_stepping_and_mid_bucket_completion
cargo test replay::tests::cut_selection_while_active_never_exposes_future_bars
cargo test replay::tests::playback_accumulator_respects_speed_burst_cap_and_end_of_data
cargo test replay::tests::causal_drawing_visibility_masks_future_pre_replay_and_preserves_replay_projections
cargo test replay::tests::critic_r07_regression_causal_undo_redo_clear_unlocked_whitespace_drag_and_dense_hit_test
cargo test trading::tests::pending_limit_and_stop_orders_handle_gap_opens_and_never_fill_on_submission_bar
cargo test trading::tests::dual_touch_bar_resolves_pessimistically_sl_before_tp_and_post_entry_follows_4_waypoint_path
cargo test trading::tests::multi_timeframe_15m_stepping_matches_1m_tick_execution_identically
cargo test trading::tests::deterministic_command_log_rewind_restores_bit_exact_state
cargo test trading::tests::critic_r07_regression_pure_reduce_stale_brackets_and_gap_open_bracket_sanitization
cargo test trading::tests::critic_r07_regression_multi_pending_causal_segment_order_scale_in_r_multiple_and_max_price
cargo test app::tests::r07_workstation_causal_replay_mtf_zero_lookahead_ticket_sync_rewind_and_instrument_isolation
cargo test app::tests::critic_r07_regression_conflated_boundary_fill_markers_empty_click_deselect_and_cut_tool_cancel
```

---

## Wave 7 — Multi-Symbol Watchlist, Technical Scanner, Alert Engine & Command Palette (`PROPOSED — MILESTONE R08`)

### Proposed Deliverables (Awaiting Product Owner Authorization)
| Component | Research Source | Rust Target | Status |
|---|---|---|---|
| Multi-symbol Watchlist & Technical Scanner (`RSI`/`MACD`/`BB`/`SMA-EMA` cross & volume breakout scans across loaded feeds) | `ARCHIVE/frontend/src/pages/ScannerPage.tsx` + `ARCHIVE/backend/app/api/scanner.py` | `src/scanner/` | Proposed (`Milestone R08`) |
| Causal Price, Indicator & Drawing-Ray Alert Engine (non-repainting trigger log + visual alert lines) | `ARCHIVE/frontend` signals/alerts + `LWC v5.2.0` price lines | `src/alerts/` | Proposed (`Milestone R08`) |
| Keyboard Command Palette (`Cmd+K` instant symbol, timeframe, indicator, tool & replay jump) | Native workstation UX + Archive shortcuts | `src/app.rs` | Proposed (`Milestone R08`) |

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
