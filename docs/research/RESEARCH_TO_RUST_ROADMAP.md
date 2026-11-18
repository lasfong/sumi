# RESEARCH TO RUST ROADMAP — SESSION R01

- **Session Identity:** `SESSION-R01-20261110T202837+0700`
- **Updated By:** `SESSION-R17-20261118T213600+0700` (`SUMI PROGRAM ORCHESTRATOR`)
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

## Wave 7 — Multi-Symbol Watchlist, Technical Scanner, Alert Engine & Command Palette (`COMPLETE — SESSION R08`)

### Completed Research & Audit Gates (`VERIFIED`)
- [x] Executed 10 live REA MCP JS/TS operations (`12` exact 64-hex Evidence IDs: `ev_7bebc091...`, `ev_a4990be1...`, `ev_bc68b61c...`, `ev_2dc77f5e...`, `ev_17c740ab...`, `ev_51e9ade8...`, `ev_4f4abed4...`, `ev_294e5ef0...`, `ev_91304935...`, `ev_88c00cb5...`, `ev_db4484c1...`, `ev_a5767e7a...`) across `ARCHIVE/frontend` scanner/signals/shortcut pipeline (`10` files) and `lightweight-charts-5.2.0` `price-line-api` + `up-down-markers-plugin` (`7` files), plus 4 final binary REA Evidence IDs (`ev_35144b6f...`, `ev_e2b74368...`, `ev_2a67be8d...`, `ev_3cd35388...`).
- [x] Authored [`docs/research/SCANNER_ALERTS_AND_COMMAND_PALETTE_SPEC.md`](SCANNER_ALERTS_AND_COMMAND_PALETTE_SPEC.md) rectifying all 6 legacy scanner, alert, and modal navigation defects.

### Shipped Rust Deliverables (`IMPLEMENTATION_PASS`)
| Component | Research Source | Shipped Rust Target | Status |
|---|---|---|---|
| Multi-symbol Watchlist & Technical Scanner (`13` bundled feeds + `custom_peer_feeds`, single-pass `1.799 ms` scan, `DEF-001` peer cutoff, `BlindPractice`) | `ARCHIVE/frontend/src/pages/ScannerPage.tsx` + `ARCHIVE/backend/app/services/scanner_service.py` | `src/scanner/`, `src/data/presets.rs` | ✅ SHIPPED |
| Causal Price, Indicator & Sloped Drawing-Line Alert Engine (`PriceLevel`, `RSI(14)`, `MACD`, `HorizontalLine`/`TrendLine`/$360^\circ$ `Ray` + Replay rewind) | `SCANNER_ALERTS_AND_COMMAND_PALETTE_SPEC.md` + `LWC v5.2.0` `price-line-api.ts` | `src/alerts/` | ✅ SHIPPED |
| Keyboard Command Palette (`Cmd+K` `13.71 µs` fuzzy rank, quick `alert <price>`, `disabled_alpha = 1.0` modal isolation) & Watchlist/Alerts Docks | `ARCHIVE/frontend` shortcuts + Native workstation UX | `src/app.rs`, `src/chart.rs` | ✅ SHIPPED |

### Verified Acceptance Tests (`cargo test` — 105/105 PASS)
```bash
cargo test scanner::tests::regime_classifier_and_causal_rvol_exclude_current_bar_lookahead
cargo test scanner::tests::technical_and_volume_signals_fire_causally_and_sort_none_metrics_last
cargo test scanner::tests::historical_signal_scan_enforces_def001_cutoff_and_blind_practice_masking
cargo test scanner::tests::multi_symbol_universe_vn30_cafef_spy_btc_scans_deterministically
cargo test scanner::tests::critic_r08_regression_single_pass_scan_incomplete_tail_and_reveal_timestamp
cargo test alerts::tests::price_level_4_waypoint_gap_wick_touch_and_flat_continuation_invariants
cargo test alerts::tests::rsi_and_macd_indicator_alerts_match_wilder_and_talib_crossovers
cargo test alerts::tests::sloped_trendline_and_360_ray_relative_offset_prevents_parallel_false_cross
cargo test alerts::tests::causal_replay_stepping_1m_15m_parity_and_rewind_rearms_once_alerts
cargo test alerts::tests::critic_r08_regression_at_mark_boundary_live_whitespace_ray_and_vertical_line_guard
cargo test alerts::tests::critic_r08_regression_intrabucket_coarse_alerts_horizontal_parity_and_causal_active_slice
cargo test app::tests::r08_watchlist_scanner_blind_practice_alerts_and_cmd_k_palette_e2e
cargo test app::tests::critic_r08_regression_utf8_truncation_mtf_blind_practice_and_peer_future_lock
cargo test app::tests::critic_r08_regression_cmd_k_modal_tab_delete_gesture_and_scroll_isolation
```

---

## Wave 8 — Deterministic Strategy Backtest Lab, Equity Curve Sub-Pane & Performance Tearsheet (`COMPLETE — SESSION R09`)

### Completed Research & Audit Gates (`VERIFIED`)
- [x] Executed 11 live REA MCP JS/TS operations (`14` exact 64-hex Evidence IDs: `ev_30052fc9...`, `ev_3efaddb8...`, `ev_0e297ef9...`, `ev_f6890cd1...`, `ev_2f44b96f...`, `ev_86aa8a13...`, `ev_72d6e059...`, `ev_54c92e4a...`, `ev_fe8c9f11...`, `ev_3e04368e...`, `ev_80e6acb9...`, `ev_4d180bfe...`, `ev_f81cc480...`, `ev_fee363ed...`) across `ARCHIVE/frontend` strategy/backtest/analytics (`10` files) and `lightweight-charts-5.2.0` `area`/`baseline` series & renderers (`8` files), plus 4 final binary REA Evidence IDs (`ev_1da8bd9d...`, `ev_bbe704de...`, `ev_c32a6cf5...`, `ev_ab19c6e7...`).
- [x] Authored [`docs/research/STRATEGY_BACKTEST_LAB_SPEC.md`](STRATEGY_BACKTEST_LAB_SPEC.md) rectifying all 6 legacy strategy lab, backtest, tearsheet, and equity-curve rendering defects.

### Shipped Rust Deliverables (`IMPLEMENTATION_PASS`)
| Component | Research Source | Shipped Rust Target | Status |
|---|---|---|---|
| Pure `f64`/`i64` WASM-safe Rule/Signal Strategy Backtest Engine (`6` presets, `Open[T+1]` fills, `MarginLiquidation`, continuous `IS/OOS` partition, `12`-grid sweep & `13`-sym battle) | `ARCHIVE/frontend/src/pages/StrategyLabPage.tsx` + `ARCHIVE/backend/app/domain/backtest/*` | `src/backtest/` | ✅ SHIPPED |
| Synchronized `Equity / Drawdown` 5th Sub-Pane (Y-clipped emerald/crimson baseline `egui::Mesh`) & $O(\log T)$ Trade Entry/Exit Links | `LWC v5.2.0` `baseline-series` / `baseline-renderer-area` + `MultiPaneLayout` | `src/chart.rs` | ✅ SHIPPED |
| Strategy Lab Dock (`Stats` Tearsheet, `Trd` Inspector + `Replay -5b`, `Sweep`, `Battle`) & `Cmd+K` Strategy Actions | `STRATEGY_BACKTEST_LAB_SPEC.md` | `src/app.rs` | ✅ SHIPPED |

### Verified Acceptance Tests (`cargo test` — 122/122 PASS)
```bash
cargo test backtest::tests::next_bar_open_causality_and_final_bar_cutoff_guard
cargo test backtest::tests::pessimistic_sl_before_tp_on_dual_touch_and_post_close_trailing_stop_anti_lookahead
cargo test backtest::tests::min_holding_bars_t_plus_2_lockup_and_lot_step_rounding
cargo test backtest::tests::tearsheet_sortino_cagr_breakeven_and_zero_loss_profit_factor_invariants
cargo test backtest::tests::replay_cutoff_zero_lookahead_and_cash_fee_tax_conservation_across_all_presets
cargo test backtest::tests::parameter_sweep_multi_symbol_battle_and_100k_bar_throughput
cargo test backtest::tests::critic_r09_regression_cross_timeframe_cutoff_prevents_intraday_daily_lookahead
cargo test backtest::tests::critic_r09_regression_robustness_never_rewards_losing_is_and_continuous_oos_conservation
cargo test backtest::tests::critic_r09_regression_margin_liquidation_prevents_zombie_account_resurrection
cargo test backtest::tests::critic_r09_regression_t_plus_2_trailing_ratchet_marubozu_slippage_and_mae_mfe_bounds
cargo test chart::tests::equity_baseline_pane_layout_separator_resize_and_conflated_render
cargo test chart::tests::critic_r09_regression_equity_y_clipped_trapezoid_and_pan_invariant_trade_decimation
cargo test app::tests::strategy_lab_causal_sync_across_bar_replay_and_sweep_invalidation
cargo test app::tests::command_palette_strategy_lab_actions_trade_focus_and_trade_replay
cargo test app::tests::critic_r09_regression_auto_sync_on_replay_transitions_and_headless_trade_replay
```

---

## Wave 9 — Vietnam Market Microstructure & Risk Planner, On-Chart Risk-Reward Position Tool, Structured Trade Journal & Rolling Walk-Forward Portfolio Analytics (`COMPLETE — SESSION R10`)

### Completed Research & Audit Gates (`VERIFIED`)
- [x] Executed 11 live REA MCP JS/TS operations (`13` exact 64-hex Evidence IDs: `ev_d326070e...`, `ev_338e20db...`, `ev_ec714173...`, `ev_9ccc2798...`, `ev_0bd6b7fd...`, `ev_0b2302d7...`, `ev_0ae4bec7...`, `ev_0daaf78d...`, `ev_7838e78d...`, `ev_590404cc...`, `ev_7eebd97b...`, `ev_bc6281a3...`, `ev_72fb8779...`) across `ARCHIVE/frontend` journal/dashboard/decision/batch (`11` files) and `lightweight-charts-5.2.0` primitive/range-hit-test/formatter (`8` files), plus 4 final binary REA Evidence IDs (`ev_76e77e44...`, `ev_cccbd439...`, `ev_29e26683...`, `ev_fb539459...`).
- [x] Authored [`docs/research/RISK_PLANNER_JOURNAL_AND_PORTFOLIO_SPEC.md`](RISK_PLANNER_JOURNAL_AND_PORTFOLIO_SPEC.md) rectifying all 6 legacy risk-sizing, Vietnam microstructure, on-chart R:R, journal causality, CSV formula-injection, and Walk-Forward/portfolio defects.

### Shipped Rust Deliverables (`IMPLEMENTATION_PASS`)
| Component | Research Source | Shipped Rust Target | Status |
|---|---|---|---|
| Pure `f64`/`i64` WASM-safe Exchange Microstructure & Friction-Aware Trade Risk Planner (`HOSE/HNX/UPCOM/VN30F/VNINDEX/US/Crypto`, `±7%/±10%/±15%` bands, `x100k` futures multiplier + `17%` VSD margin, `UTC+7` `T+2` settlement, `101.12 µs` / `1K` plans) | `ARCHIVE/backend/app/domain/{market_rules.py,trade_planning.py,accounting.py}` | `src/journal/market_rules.rs`, `src/journal/planner.rs` | ✅ SHIPPED |
| Interactive On-Chart Dual-Zone `RiskRewardBox` Drawing Tool (`RrEntryLeft/Right`, `RrTarget`, `RrStop`, `Long <-> Short` crossover, Planner & Order Ticket sync) | `LWC v5.2.0` `iseries-primitive-api.ts` + `range-hit-test.ts` | `src/drawings/`, `src/chart.rs`, `src/trading/portfolio.rs` | ✅ SHIPPED |
| Causal `DEF-001` `TradeJournalStore` (`SetupTag`, `MistakeTag`, `ConvictionRating`, 6-item `ChecklistState`, `SQN`, `Discipline %`, Paper rewind sync, formula-safe RFC-4180 CSV & Markdown) | `ARCHIVE/frontend/src/pages/{JournalPage.tsx,DashboardPage.tsx}` | `src/journal/ledger.rs`, `src/app.rs` | ✅ SHIPPED |
| Continuous-Carry Rolling/Anchored Walk-Forward (`WFO`, Pardo `WFE %`) & Bar-Close Multi-Asset Portfolio (`EqualWeight` / `CausalInverseVolatility`) in `OscillatorPaneId::Equity` | `ARCHIVE/frontend/src/components/strategy/MultiPhaseBatchPanel.tsx` + `RISK_PLANNER_JOURNAL_AND_PORTFOLIO_SPEC.md` | `src/backtest/walk_forward.rs`, `src/backtest/engine.rs`, `src/chart.rs`, `src/app.rs` | ✅ SHIPPED |

### Verified Acceptance Tests (`cargo test` — 136/136 PASS)
```bash
cargo test journal::tests::hose_hnx_upcom_vn30f_tick_ladders_and_tier_crossing_price_bands
cargo test journal::tests::t_plus_2_settlement_skips_weekends_deterministically
cargo test journal::tests::friction_aware_long_and_short_position_sizer_never_exceeds_risk_budget
cargo test journal::tests::causal_journal_analytics_sqn_discipline_and_csv_formula_injection_guard
cargo test journal::tests::critic_r10_regression_vn30f_multiplier_hose_tier_step_local_tz_and_csv_newline
cargo test drawings::tests::risk_reward_box_hit_test_log_drag_timeframe_remap_and_ticket_extract
cargo test drawings::tests::critic_r10_regression_rr_box_crossover_drag_never_drifts_fixed_anchor
cargo test backtest::tests::walk_forward_rolling_and_anchored_zero_oos_lookahead_and_stitched_compounding
cargo test backtest::tests::portfolio_backtest_equal_and_causal_inverse_vol_with_replay_cutoff
cargo test backtest::tests::critic_r10_regression_wfo_open_position_carry_conservation_and_intraday_portfolio_causality
cargo test app::tests::r10_workstation_risk_planner_rr_box_journal_walk_forward_and_portfolio_e2e
cargo test app::tests::critic_r10_regression_zero_lot_ticket_guard_reseed_venue_reset_and_1w_ref_price
```

---

## Wave 10 — Order-Flow Volume Profile (`VRVP` / `FRVP` `POC` / `VAH` / `VAL`), Session & Anchored `VWAP` (`AVWAP`) Bands & Causal Relative Strength (`RS vs VNINDEX`) Overlay (`COMPLETE — SESSION R11`)

### Completed Research & Audit Gates (`VERIFIED`)
- Executed `13` live REA MCP operations (`19` exact 64-hex Evidence IDs: `15` JS/TS across `ARCHIVE/frontend` + `LWC v5.2.0` histogram/pane-primitive files and `4` binary on final R11 release Mach-O `e9dcb085a076ad12d46981a88dbde0459536675deee843c65641fc2c7f0ff6bc`).
- Authored [`VOLUME_PROFILE_VWAP_AND_REL_STRENGTH_SPEC.md`](VOLUME_PROFILE_VWAP_AND_REL_STRENGTH_SPEC.md) with zero contact with TradingView Desktop (`DEC-006`) and `0` new crate dependencies in `Cargo.toml`.

### Shipped Rust Deliverables (`IMPLEMENTATION_PASS`)
| Component | Research Source | Rust Target | Status |
|---|---|---|---|
| Pure `f64`/`i64` WASM-safe Order-Flow Volume Profile (`VRVP` & draggable `FRVP` drawing with `OHLCV_PROXY` conservation, TPO fallback, `POC`, CME `70%` `VAH`/`VAL`, `HVN`/`LVN`) | `LWC v5.2.0` `histogram-renderer.ts` + `pane-primitive-wrapper.ts` | `src/indicators/volume_profile.rs`, `src/drawings/`, `src/chart.rs` | ✅ SHIPPED |
| Exchange-Session `VWAP` (`UTC+7`/`UTC-5`/`UTC` intraday + UTC civil `1D`/`1W` boundaries) & Interactive `Anchored VWAP` (`AVWAP`) rescaled Welford $\pm 1\sigma, \pm 2\sigma$ bands | `VOLUME_PROFILE_VWAP_AND_REL_STRENGTH_SPEC.md` | `src/indicators/vwap.rs`, `src/drawings/`, `src/chart.rs` | ✅ SHIPPED |
| `DEF-001` Causal Close-Aligned `Relative Strength vs VNINDEX` (`OscillatorPaneId::RelativeStrength` + `64`-bar periodic SMA recompute) & `O(1)` Rebased Benchmark Compare Overlay | `ARCHIVE/frontend` `indicator_engine.py` + `LWC v5.2.0` `price-scale.ts` | `src/indicators/relative_strength.rs`, `src/data/presets.rs`, `src/chart.rs`, `src/app.rs` | ✅ SHIPPED |

### Verified Acceptance Tests (`cargo test` — 152/152 PASS)
```bash
cargo test indicators::tests::r11_volume_profile_conserves_exact_total_buy_sell_volume_and_identifies_poc_vah_val
cargo test indicators::tests::r11_session_vwap_resets_at_vietnam_utc7_day_boundary_and_monthly_on_d1
cargo test indicators::tests::r11_anchored_vwap_masks_pre_anchor_bars_and_accumulates_welford_bands
cargo test indicators::tests::r11_relative_strength_enforces_def001_bar_close_causality_and_holiday_carry_forward
cargo test indicators::tests::critic_r11_regression_session_bucket_d1_w1_negative_utc_offset
cargo test indicators::tests::critic_r11_regression_vwap_extreme_magnitude_no_inf_or_nan_collapse
cargo test indicators::tests::critic_r11_regression_vp_micro_price_and_dust_volume_va
cargo test indicators::tests::critic_r11_regression_rs_sma_spike_drift_and_benchmark_coverage
cargo test drawings::tests::r11_frvp_and_avwap_creation_hit_test_log_linear_drag_timeframe_remap_and_causal_masking
cargo test drawings::tests::critic_r11_regression_frvp_timeframe_remap_preserves_validity_and_vertical_handles
cargo test drawings::tests::critic_r11_regression_avwap_tail_hit_test_and_left_drag_clamp
cargo test chart::tests::r11_render_memo_invalidates_on_mid_array_last_bar_and_generation_changes
cargo test chart::tests::critic_r11_regression_vwap_zoomed_out_session_breaks
cargo test chart::tests::critic_r11_regression_r11_memo_no_xor_cancel_and_avwap_pan_invariance
cargo test app::tests::r11_workstation_vrvp_frvp_vwap_avwap_and_rs_vnindex_e2e
cargo test app::tests::critic_r11_regression_benchmark_cache_invalidation_and_compare_pan_o1
```

---

## Wave 11 — Synchronized Multi-Chart Grid Layout (`1x1`/`1x2`/`2x1`/`2x2`), Ichimoku Cloud + `Cumulative Volume Delta (CVD)`, & Workspace Templates (`COMPLETE — SESSION R12`)

### Shipped Deliverables & Verification (`cargo test indicators::tests::r12_`, `cargo test viewport::tests::r12_`, `cargo test chart::tests::r12_`, `cargo test app::tests::r12_`, `cargo test critic_r12_`, `--r12`, `SUMI_CAPTURE_BATCH=r12`)
| Component | Research Source | Rust Target | Status |
|---|---|---|---|
| Pure `f64`/`i64` WASM-safe Synchronized Multi-Chart Grid (`MultiChartLayoutKind` `1x1`/`1x2`/`2x1`/`2x2`, `MultiChartSyncConfig` `Sync X`/`Sync T`/`Sync S`/`Sync TF`, `SyncedTimeWindow`, `resolve_linked_crosshair_bar`, `causal_closed_prefix_len`, `SecondaryChartSlot`) & Serializable `WorkspaceTemplate` (`WORKSPACE_TEMPLATE_SCHEMA_VERSION = 1`, strict RFC-8259 JSON & RON codecs, presets + modal) | `ARCHIVE/frontend/src/components/chart/MultiChartLayout.tsx` + `LWC v5.2.0` (`time-scale.ts`) + [`docs/research/MULTI_CHART_ICHIMOKU_CVD_SPEC.md`](MULTI_CHART_ICHIMOKU_CVD_SPEC.md) | `src/viewport/multi_chart.rs`, `src/viewport/workspace_template.rs`, `src/viewport/tests.rs`, `src/chart.rs`, `src/app.rs` | Shipped & Critic-Hardened (`Milestone R12`, `CRITIC-R12-01..14` verified) |
| `Ichimoku Cloud` (`Tenkan 9`/`Kijun 26`/`Senkou A-B +26`/strict `None` tail `Chikou -26`, overflow-safe midpoint, triangle-split Kumo `egui::Mesh`), `Cumulative Volume Delta (CVD)` (`OHLCV_PROXY` `Cumulative` / TZ-aware `Session` + `9`-SMA, `OscillatorPaneId::Cvd`), `CandleBlockIndex` (`16`-bar acceleration) & `PlotDataLayerMemo` (`Quad 2x2` `200k` conflated `1.3035 ms` / `p99 1.6077 ms < 4.0 ms`) | `ARCHIVE/frontend/src/components/chart/plugins/IchimokuCloudPlugin.ts` + `src/indicators/volume_profile.rs` + [`docs/research/MULTI_CHART_ICHIMOKU_CVD_SPEC.md`](MULTI_CHART_ICHIMOKU_CVD_SPEC.md) | `src/indicators/ichimoku.rs`, `src/indicators/cvd.rs`, `src/indicators/mod.rs`, `src/indicators/tests.rs`, `src/chart.rs`, `src/app.rs` | Shipped & Critic-Hardened (`Milestone R12`, `174/174` tests + `wasm32` Clippy) |

---

## Wave 12 — Session TPO / Market Profile, Multi-Interval `VWAP` Anchors, Causal VN30 Breadth & 4-Quadrant `RRG` Heatmap, Compact `K`/`M`/`B`/`T` `CVD` Axis & R12 Carry-Over Polish (`COMPLETE — SESSION R13`)

### Shipped Deliverables & Verification (`cargo test market_profile`, `cargo test vwap`, `cargo test r13_`, `cargo test critic_r13_`, `--r13`, `SUMI_CAPTURE_BATCH=r13`)
| Component | Research Source | Rust Target | Status |
|---|---|---|---|
| Pure `f64`/`i64` WASM-safe Session TPO / Market Profile (`POC`, CME 70% `VAH`/`VAL`, `IB`, `Single Prints`, `Naked POC` + `HistoricalNakedPoc`, `ProfileSessionPeriod`), Multi-Interval `VwapAnchorMode` (`AutoSession`/`Daily`/`Weekly`/`Monthly`/`Yearly`, negative-UTC `D1`/`W1` guard), 5th built-in `WorkspaceTemplate` (`"VN30 Session Profile & Flow"`), & per-slot `TPO`/`VWAP`/`Sync Ind` toolbar + `Cmd+K` sync | `ARCHIVE/frontend` `TechnicalFlowBBViewer.tsx` + `LWC v5.2.0` + [`docs/research/MARKET_PROFILE_BREADTH_AND_AXIS_SPEC.md`](MARKET_PROFILE_BREADTH_AND_AXIS_SPEC.md) | `src/indicators/market_profile.rs`, `src/indicators/vwap.rs`, `src/indicators/mod.rs`, `src/viewport/workspace_template.rs`, `src/chart.rs`, `src/app.rs` | Shipped & Critic-Hardened (`Milestone R13`, `331.54 µs` `50k` raw `15m` TPO, `785.75 µs` `50k` Weekly VWAP) |
| Causal VN30 & Universe Market Breadth (`AT05`/`AT09` Aggregate-Before-Ratio Money Flow, `AT17` participation ratios, `DEF-001` `replay_causal_close_timestamp` `0`-lag gate, `causal_peer_open_cutoff`, `is_active_tail_incomplete()`) + JdK 4-quadrant `RRG` + 340px `Watch`/`Sig`/`Breadth` Dock (obstacle-seeded multi-ring `RRG` labels + symmetric `allocate_exact_size` Heatmap tiles) | `ARCHIVE/backend/app/domain/bb/market_aggregator.py` + `ARCHIVE/frontend` `ScannerPage.tsx`/`AnalyticsPage.tsx`/`DashboardPage.tsx` + [`MARKET_PROFILE_BREADTH_AND_AXIS_SPEC.md`](MARKET_PROFILE_BREADTH_AND_AXIS_SPEC.md) | `src/scanner/breadth.rs`, `src/scanner/mod.rs`, `src/scanner/tests.rs`, `src/app.rs` | Shipped & Critic-Hardened (`Milestone R13`, `858.79 µs` Breadth/RRG, `CRITIC_PASS`) |
| Compact `K`/`M`/`B`/`T` `CVD` Right Axis (`PriceAxisFormatMode::CompactVolume`), Clipped Axis Pills, `WorldBinIntervalAccumulator` Pan-Invariant Sub-Pixel Envelopes, Dual-Sided Conflated `CVD` Histogram, `cvd_price_scale.reset_auto_scale()` across Primary Transitions, & `safe_midpoint_f64` | `ARCHIVE/frontend/src/utils/formatters.ts` + `LWC v5.2.0` `volume-formatter.ts`/`price-axis-view*` + [`MARKET_PROFILE_BREADTH_AND_AXIS_SPEC.md`](MARKET_PROFILE_BREADTH_AND_AXIS_SPEC.md) | `src/viewport/price_scale.rs`, `src/chart.rs`, `src/app.rs`, `src/main.rs` | Shipped & Critic-Hardened (`181/181` tests + `wasm32` Clippy, `--r13` `0.2588–2.4689 ms` mean / `p99 <= 2.8089 ms < 4.0 ms`) |

---

## Wave 13 — Synthetic Depth-of-Market / Footprint Execution Ladder, Custom Hotkey Profiles, Dock Layout Persistence & Right-Axis Polish (`COMPLETE — SESSION R14`)

### Completed Research & Audit Gates (`VERIFIED`)
- [x] Executed `10` live REA MCP JS/TS operations (`REA-R14-01`..`10` across `ARCHIVE/frontend` `TradeControls.tsx`/`PendingOrdersPanel.tsx`/`PositionPanel.tsx`/`KeyboardShortcutsModal.tsx`/`globalShortcutPolicy.ts`/`PositionLineManager.ts` & `LWC v5.2.0` `price-formatter.ts`/`price-tick-mark-builder.ts`/`price-scale.ts`/`price-tick-span-calculator.ts`) plus `2` post-build binary REA ops (`REA-R14-11`..`12` on final R14 release Mach-O `52ef3d8c86297c33d451ff389945a5deeb469ac4dddff6f680323ca5180ab33f`, UUID `F6C4D83C-C21B-3D9A-8F94-28BB912C4BF5`) -> [`docs/research/EXECUTION_LADDER_HOTKEYS_AND_AXIS_POLISH_SPEC.md`](EXECUTION_LADDER_HOTKEYS_AND_AXIS_POLISH_SPEC.md).

### Shipped Deliverables & Verification (`cargo test r14_`, `cargo test critic_r14_`, `--r14`, `SUMI_CAPTURE_BATCH=r14`)
| Component | Research Source | Rust Target | Status |
|---|---|---|---|
| Pure `f64`/`i64` WASM-safe Causal 21-Row `OHLCV_SYNTH_DOM_PROXY` Price Ladder (`HOSE`/`HNX`/`UPCOM`/`VN30F` tier-crossing ticks, `ResolvedVnPriceUnit`, `CEIL`/`REF`/`FLOOR` bands, `CLV` footprint, 1-click staging, working-order cancel, short guard, risk-budget sizing, & `compute_active_dom_ladder()`-synced `shift_dom_center_ticks`) | `ARCHIVE/frontend` `TradeControls.tsx`/`PendingOrdersPanel.tsx`/`PositionPanel.tsx` + `EXECUTION_LADDER_HOTKEYS_AND_AXIS_POLISH_SPEC.md` | `src/trading/ladder.rs`, `src/trading/mod.rs`, `src/journal/market_rules.rs`, `src/app.rs` | Shipped & Critic-Hardened (`277.12 µs` `50k` `VN30F1M` ladder, `CRITIC_PASS`) |
| Conflict-Free `17`-Action `HotkeyProfile` (`from_preset_with_base_and_overrides`, `is_global_hotkey_eligible` + `Space` focus guard, `[Keys]` modal), & Backwards-Compatible `WorkspaceTemplate` (`dock_layout`, `hotkey_profile`, `hotkey_base_preset`, `hotkey_overrides`, Preset #5 Breadth & Preset #6 `"VN30F DOM Execution & Flow"`) | `ARCHIVE/frontend` `KeyboardShortcutsModal.tsx`/`globalShortcutPolicy.ts` + `EXECUTION_LADDER_HOTKEYS_AND_AXIS_POLISH_SPEC.md` | `src/viewport/hotkeys.rs`, `src/viewport/workspace_template.rs`, `src/app.rs` | Shipped & Critic-Hardened (`96.96 µs` `10k` resolves/rebinds, `RON`/`JSON` base-preset roundtrip verified) |
| Non-Blocking R13 Right-Axis Edge & Decimal Polish (`axis_edge_pad_px`, `scale_decimals_for_range_and_step`, `format_price_for_tick_density`, `safe_clamp_f32`, pill/badge collision avoidance & time-cursor occlusion) | `LWC v5.2.0` `price-formatter.ts`/`price-tick-mark-builder.ts`/`price-scale.ts` + `EXECUTION_LADDER_HOTKEYS_AND_AXIS_POLISH_SPEC.md` | `src/viewport/price_scale.rs`, `src/chart.rs`, `src/main.rs` | Shipped & Critic-Hardened (`204/204` tests + `wasm32` Clippy, `--r14` `0.378–1.391 ms` mean / `p99 <= 2.198 ms < 4.0 ms`) |

### Verified Acceptance Tests (`cargo test` — 204/204 PASS)
```bash
cargo test trading::ladder::tests::r14_
cargo test trading::ladder::tests::critic_r14_
cargo test viewport::tests::r14_
cargo test viewport::tests::critic_r14_
cargo test chart::tests::critic_r14_
cargo test app::tests::r14_
cargo test app::tests::critic_r14_
```

---

## Wave 14 — Explainable Signal Catalog & Causal Confluence Inspector, Interactive Drawing Property Inspector, & Monte Carlo Bootstrap Tearsheet Risk Envelopes (`COMPLETE — SESSION R15`)

### Completed Research & Audit Gates (`VERIFIED`)
- [x] Executed `10` live REA MCP JS/TS operations (`REA-R15-01`..`10` across `ARCHIVE/frontend` `SignalCatalog.tsx`/`SignalCatalogModal.tsx`/`SignalExplanationInspector.tsx`/`SignalInspector.tsx`/`DrawingInspector.tsx`/`EquityChart.tsx`/`MetricResultValue.tsx`/`MultiStrategyEquityChart.tsx` & `LWC v5.2.0` `area-renderer.ts`/`baseline-renderer-area.ts`/`baseline-renderer-line.ts`/`area-pane-view.ts`/`baseline-pane-view.ts`/`primitive-drawing-utils.ts`/`ipane-primitive.ts`) plus `2` post-build binary REA ops (`REA-R15-11`..`12` on final R15 release Mach-O `d0ba48a581c3fd1c73da2f11d8deb47e46f2a094d15c62f4b1fe008de7de76ff`, UUID `F2CD9DB2-AC30-37C9-B10E-1168E6B17A88`) -> [`docs/research/SIGNALS_INSPECTOR_AND_MONTE_CARLO_SPEC.md`](SIGNALS_INSPECTOR_AND_MONTE_CARLO_SPEC.md).

### Shipped Deliverables & Verification (`cargo test r15_`, `cargo test critic_r15_`, `--r15`, `SUMI_CAPTURE_BATCH=r15`)
| Component | Research Source | Rust Target | Status |
|---|---|---|---|
| Pure `f64`/`i64`/`u64` WASM-safe `17`-Entry `SIGNAL_CATALOG` (`12` tabs, EN/VI/AST search, `SplitMix64` `params_digest`, `OHLCV_PROXY`), `SignalQuality` (`NaN`/`Inf` guards), 4-family `evaluate_causal_confluence` (`35/25/20/20` & `50/20/15/15`, `"bb_absent_neutral"`), `explain_historical_signal` (`1W` Friday & `VN30F1M` session-close aware), `ScannerSignalContext::revealed_explanation`, virtualized `ScrollArea::show_rows` Signals dock & Signal Catalog Modal | `ARCHIVE/frontend/src/components/signals/{SignalCatalog,SignalCatalogModal,SignalInspector,SignalExplanationInspector}.tsx` + `SIGNALS_INSPECTOR_AND_MONTE_CARLO_SPEC.md` | `src/scanner/catalog.rs`, `src/scanner/mod.rs`, `src/scanner/tests.rs`, `src/app.rs` | Shipped & Critic-Hardened (`23.75 µs` `FPT 1D`, `CRITIC_PASS`) |
| Transactional Interactive Drawing Property Inspector (`Drawing::is_causally_eligible` vs `is_causally_visible`, `DrawingPropertyPatch`, `apply_drawing_property_patch` `<= 1` `Modify` on 64-step Undo/Redo, `Escape` dirty-draft cancel, same-frame Alerts & Planner sync) | `ARCHIVE/frontend/src/components/chart/DrawingInspector.tsx` + `SIGNALS_INSPECTOR_AND_MONTE_CARLO_SPEC.md` | `src/drawings/mod.rs`, `src/drawings/controller.rs`, `src/drawings/tests.rs`, `src/app.rs` | Shipped & Critic-Hardened (`261.96 µs` `1k` patches, `CRITIC_PASS`) |
| Deterministic `SplitMix64` Monte Carlo Bootstrap (`DEFAULT_MONTE_CARLO_SEED = 202_615`, sequential equity returns, `VaR`/`CVaR` `95%`/`99%`, `WalkForwardOos` & `MultiAssetPortfolio` bar-aligned `MonteCarloFanSeries`, log-safe `autoscale_equity_bounds_with_fan`, `p5..p95` + `p25..p75` + dashed `p50` `egui::Mesh`) & R14 Right-Axis Carry-Overs (`vp.base_value` `+0.00%` badge + `collect_trading_axis_pill_ys`) | `ARCHIVE/frontend/src/components/analytics/{EquityChart,MetricResultValue}.tsx` + `LWC v5.2.0` `baseline-renderer-area.ts` + `SIGNALS_INSPECTOR_AND_MONTE_CARLO_SPEC.md` | `src/backtest/monte_carlo.rs`, `src/backtest/mod.rs`, `src/chart.rs`, `src/app.rs` | Shipped & Critic-Hardened (`216/216` tests + `wasm32` Clippy, `--r15` `0.337–1.557 ms` mean / `p99 <= 2.098 ms < 4.0 ms`) |

### Verified Acceptance Tests (`cargo test` — 216/216 PASS)
```bash
cargo test scanner::tests::r15_
cargo test scanner::tests::critic_r15_
cargo test drawings::tests::r15_
cargo test backtest::tests::r15_
cargo test backtest::tests::critic_r15_
cargo test chart::tests::r15_
cargo test chart::tests::critic_r15_
cargo test app::tests::r15_
cargo test app::tests::critic_r15_
```

---

## Wave 15 — Composable Signal-Catalog Rule Strategy Builder, Multi-Condition Causal Alert Ledger & Export, & Named Workspace Snapshot Checkpoints + Corrupt-State Recovery (`COMPLETE — SESSION R16`)

### Completed Research & Audit Gates (`VERIFIED`)
- [x] Executed `10` live REA MCP JS/TS operations (`REA-R16-01`..`10` across `ARCHIVE/frontend` `StrategyRuleBuilder.tsx`/`StrategyLabPage.tsx`/`MultiPhaseBatchPanel.tsx`/`SettingsModal.tsx`/`workspaceTypes.ts`/`DrawingRepository.ts`/`PracticeScoreboard.tsx`/`SessionDebriefModal.tsx` & `LWC v5.2.0` `series-markers/{primitive,pane-view,renderer,types,utils}.ts`/`expiring-markers-manager.ts`/`chart-options-defaults.ts`) plus `2` post-build binary REA ops (`REA-R16-11`..`12` on final R16 release Mach-O `d32c891c1abc364e9d709e769984a4bd9ef64a2553927e78bc64304aea875750`, UUID `03E16800-6F00-3FE2-944C-FE5D499D6FD0`) -> [`docs/research/RULE_BUILDER_ALERTS_AND_SNAPSHOT_RECOVERY_SPEC.md`](RULE_BUILDER_ALERTS_AND_SNAPSHOT_RECOVERY_SPEC.md) and [`docs/FEATURE_BACKLOG_AND_PARITY_LEDGER.md`](../FEATURE_BACKLOG_AND_PARITY_LEDGER.md).

### Shipped Deliverables & Verification (`cargo test r16_`, `cargo test critic_r16_`, `--r16`, `SUMI_CAPTURE_BATCH=r16`)
| Component | Research Source | Rust Target | Status |
|---|---|---|---|
| Vectorized $O(N)$ `PrecomputedCatalogMatrix` (`point_at_with_profile`) & Composable 4-Leg DNF `CustomSignalRuleSpec` (`StrategyPresetKind::CustomSignalRules`, fail-closed `SignalQuality::Valid`, 6-bit `MarketRegime` mask, `ConfluenceThresholdGate`, 12-grid sweep, `RuleBuilderMatchCache`, feeding `Open[T+1]` Backtest, `WFO`, `Portfolio`, & `SplitMix64` Monte Carlo) | `ARCHIVE/frontend/src/components/strategy/StrategyRuleBuilder.tsx` + `ARCHIVE/backend/app/domain/strategy/{rule_evaluator,signal_binding}.py` + `RULE_BUILDER_ALERTS_AND_SNAPSHOT_RECOVERY_SPEC.md` | `src/scanner/catalog.rs`, `src/backtest/strategy.rs`, `src/backtest/engine.rs`, `src/app.rs` | Shipped & Critic-Hardened (`784.12 µs` `BTCUSDT 15m`, `CRITIC_PASS`) |
| Multi-Condition Causal Alert Ledger (`SignalCatalogTrigger`, `ConfluenceThreshold`, `cooldown_bars`, bounded VN/US/UTC HTF close guards, independent `above`/`below` chart stacking, `append_alert_axis_pill_ys`, formula-safe CSV & RFC-8259 JSON clipboard export) | `ARCHIVE/backend/app/services/event_logging_service.py` + `LWC v5.2.0` `series-markers` + `RULE_BUILDER_ALERTS_AND_SNAPSHOT_RECOVERY_SPEC.md` | `src/alerts/mod.rs`, `src/alerts/engine.rs`, `src/chart.rs`, `src/app.rs` | Shipped & Critic-Hardened (`71.08 µs` `FPT 1D`, `CRITIC_PASS`) |
| Named Workspace Snapshot Checkpoints (`16` named + `pinned`, `4` `auto_ring` + `"Pre-Restore Undo"`, 64-bit `CorruptSnapshotDigest`, `compute_workspace_diff`, `2 MiB` buffer, & non-panicking `QuarantinedPayloadDiagnostic` recovery) | `ARCHIVE/frontend/src/features/drawings/DrawingRepository.ts` + `workspaceTypes.ts` + `RULE_BUILDER_ALERTS_AND_SNAPSHOT_RECOVERY_SPEC.md` | `src/viewport/workspace_template.rs`, `src/viewport/mod.rs`, `src/app.rs` | Shipped & Critic-Hardened (`235/235` tests + `wasm32` Clippy, `--r16` `0.322–1.332 ms` mean / `p99 <= 1.501 ms < 4.0 ms`) |

### Verified Acceptance Tests (`cargo test` — 235/235 PASS)
```bash
cargo test scanner::tests::r16_
cargo test backtest::tests::r16_
cargo test backtest::tests::critic_r16_
cargo test alerts::tests::r16_
cargo test alerts::tests::critic_r16_
cargo test viewport::tests::r16_
cargo test viewport::tests::critic_r16_
cargo test chart::tests::r16_
cargo test app::tests::r16_
cargo test app::tests::critic_r16_
```

---

## Wave 16 — Replay Practice Coaching (`PracticeScoreboard` & `SessionDebriefModal`), 1-Click `+120b` `Scanner -> Replay` & Saved Sessions, UTC-Aligned `0%` Multi-Strategy Equity Overlay, & Advanced Tearsheet Analytics (`COMPLETE — SESSION R17`)

### Completed Research & Audit Gates (`VERIFIED`)
- [x] Executed `10` live REA MCP JS/TS operations (`REA-R17-01`..`10` across `ARCHIVE/frontend` `PracticeScoreboard.tsx`/`SessionDebriefModal.tsx`/`DecisionJournal.tsx`/`SessionSetup.tsx`/`ScannerSourceContext.tsx`/`practiceDomain.ts`/`MultiStrategyEquityChart.tsx`/`AnalyticsPage.tsx` & `LWC v5.2.0` `line-series.ts`/`line-pane-view.ts`/`line-renderer.ts`/`walk-line.ts`/`percentage-formatter.ts`/`crosshair.ts`/`crosshair-marks-pane-view.ts`) plus `2` post-build binary REA ops (`REA-R17-11`..`12` on final R17 release Mach-O `e40757f26f463b553122e3f150e29b1c05a6670a500c79c0668f6a0dad80688f`, UUID `53E28E8D-58EC-357C-97BD-0A47875228BC`) -> [`docs/research/PRACTICE_COACHING_AND_MULTISTRATEGY_ANALYTICS_SPEC.md`](PRACTICE_COACHING_AND_MULTISTRATEGY_ANALYTICS_SPEC.md).

### Shipped Deliverables & Verification (`cargo test r17_`, `cargo test critic_r17_`, `--r17`, `SUMI_CAPTURE_BATCH=r17`)
| Component | Research Source | Rust Target | Status |
|---|---|---|---|
| Tamper-Proof Initial SL Adherence (`had_initial_sl_at_open`, `sl_adherence_pct: Option<f64>`), Live `PracticeScoreboardSummary` + `reset_practice_session()`, `SessionDebriefReport` (`1`-to-`1` `note_claimed`, curated `JournalEntry` override incl. `MistakeTag::None`, `MistakeCostBreakdown`, `SetupEdgeBreakdown`), & `DEF-001` `PracticeDecisionLog` | `ARCHIVE/frontend/src/components/replay/{PracticeScoreboard,SessionDebriefModal,DecisionJournal}.tsx` + `PRACTICE_COACHING_AND_MULTISTRATEGY_ANALYTICS_SPEC.md` | `src/trading/portfolio.rs`, `src/trading/coaching.rs`, `src/journal/ledger.rs`, `src/app.rs` | Shipped & Critic-Hardened (`46.29 µs` debrief + store JSON round-trip, `CRITIC_PASS`) |
| 1-Click `+120b` `Scanner -> Replay` (`INSTITUTIONAL_BLIND_PRACTICE_PRE_ROLL_BARS = 120`, `+15b/+60b/+120b` selector across `Watchlist` & `Signals`, non-leaking `ScannerSignalContext` banner) & `PracticeSessionStore` / `ScanSnapshotStore` (`<= 16` slots, `[★/☆]` pin, `snap.timeframe` restore, `DatasetMismatch`, `SplitMix64` RON/JSON codecs) | `ARCHIVE/frontend/src/components/replay/{SessionSetup,ScannerSourceContext}.tsx` + `PRACTICE_COACHING_AND_MULTISTRATEGY_ANALYTICS_SPEC.md` | `src/scanner/{engine,mod}.rs`, `src/trading/coaching.rs`, `src/app.rs` | Shipped & Critic-Hardened (`CRITIC_PASS`) |
| Symbol-Aware UTC `0%` Multi-Strategy Equity Comparison (`slice_causal_series_for_symbol_and_options`, `7` presets + `Asset Buy & Hold` + causal bar-close `VNINDEX`/`SPY` benchmark, `SignedPercentage`, pre-gesture `Logarithmic -> Normal` sync, `overlay_conflation_step` `SeriesBlockIndex` `Mesh`, whitespace-safe/conflated/linked crosshair dots) & `AdvancedTearsheetAnalytics` (`OutlierImpact`, `DrawdownPeriod`s, UTC Year & Signal-Bar `MarketRegime` slices, `MultiPhaseBatch`, `RunManifest`) | `ARCHIVE/frontend/src/{components/strategy/MultiStrategyEquityChart.tsx,pages/AnalyticsPage.tsx}` + `LWC v5.2.0` `walk-line.ts`/`percentage-formatter.ts` + `PRACTICE_COACHING_AND_MULTISTRATEGY_ANALYTICS_SPEC.md` | `src/backtest/{analytics,engine,walk_forward,mod}.rs`, `src/viewport/price_scale.rs`, `src/chart.rs`, `src/app.rs` | Shipped & Critic-Hardened (`245/245` tests + `wasm32` Clippy, `--r17` `0.336–2.082 ms` mean / `p99 <= 2.422 ms < 4.0 ms`) |

### Verified Acceptance Tests (`cargo test` — 245/245 PASS)
```bash
cargo test trading::tests::r17_
cargo test trading::tests::critic_r17_
cargo test backtest::tests::r17_
cargo test backtest::tests::critic_r17_
cargo test chart::tests::r17_
cargo test chart::tests::critic_r17_
cargo test app::tests::r17_
cargo test app::tests::critic_r17_
```

---

## Waves 17–18 (`Milestones R18–R19`) — Remaining Master Anti-Miss Parity Roadmap

All remaining features and sub-branches across `ARCHIVE/` (`208` frontend + `474` backend items) are systematically tracked in [`docs/FEATURE_BACKLOG_AND_PARITY_LEDGER.md`](../FEATURE_BACKLOG_AND_PARITY_LEDGER.md) (`A1..A5` shipped in `R17`):

| Wave / Milestone | Ledger Items | Key Scope (Awaiting Product Owner Authorization) |
|---|---|---|
| **Wave 17 (`Milestone R18`)** | `B1..B4` | `14` Causal Candlestick Patterns (`CandleGeometry`), `8` Remaining Legacy Indicators (`ADX`, `Stochastic`, `SuperTrend`, `PSAR`, `Keltner`, `CCI`, `MFI`, `Volume SMA`), Confirmed-Pivot Divergences (`right_bars = 3`) + Full `VSA`/`SR`/`Ichimoku` Signals (`72+` Catalog Entries), & Multi-Horizon `ProxyBB` (`T03..T200`) + `Point-in-Time` Universe |
| **Wave 18 (`Milestone R19`)** | `C1..C4` | Interactive Global `[Settings]` Modal & Candle Palettes, Official Vietnam Exchange Holiday Calendar (`VIETNAM_HOLIDAYS_V1`) + FIFO `T+2` Lot Release, Data Readiness & Gap/Split Audit Panel, & `TextNote` Drawing + Custom Fib Levels + Toolbar `[Export PNG]` |

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
