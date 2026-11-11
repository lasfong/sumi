# SUMI TRADING WORKSTATION — ARCHITECTURE (`MILESTONE R05`)

- **Updated By:** `SESSION-R05-20261111T163600+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Crate Name:** `sumi-workstation` (`v0.1.0`, Rust Edition 2024, `egui`/`eframe 0.36.0`)

---

## 1. Crate & Directory Structure

The workstation uses a single-crate library + binary architecture (`src/lib.rs` + `src/main.rs`) with strict module boundaries separating pure `f64` domain, viewport, technical indicator, and interactive drawing engines (`wasm32-unknown-unknown`-verified via `cargo clippy --target wasm32-unknown-unknown -- -D warnings`) from `egui` immediate-mode UI and native macOS windowing:

```text
/Users/mizuhara/workspace/sumi
├── Cargo.toml
├── src/
│   ├── lib.rs                      # Crate root exporting app, chart, data, drawings, indicators, model, viewport
│   ├── main.rs                     # Native macOS entry point (.with_active(false) when SUMI_CAPTURE_BATCH set)
│   ├── model.rs                    # Pure OHLCV domain model (Candle, PriceBounds, deprecated ViewportMapping)
│   ├── data.rs                     # O(N) deterministic synthetic OHLCV generator (120 .. 100,000+ bars)
│   ├── viewport/                   # Milestone R03 Modular Viewport & Coordinate Engine (f64, WASM-safe)
│   │   ├── mod.rs                  # Public exports + panic-free lwc_clamp
│   │   ├── time_scale.rs           # Shared horizontal TimeScale (zoom, pan, resize, fit, culling, conflation)
│   │   ├── price_scale.rs          # Per-pane/axis PriceScale (Normal, Log, %, Idx100, sumi_oscillator)
│   │   ├── pane_viewport.rs        # Explicit 3-space bridge (Logical/Data <-> PaneLocal <-> Screen)
│   │   ├── kinetic.rs              # Exponential-decay inertial panning on right_offset (bar units)
│   │   ├── candle_metrics.rs       # Optimal candlestick/wick pixel parity + zero-alloc OHLCV conflation
│   │   └── tests.rs                # LWC v5.2.0 fixtures + 5,100+ property cases + Critic R03 regression suite
│   ├── indicators/                 # Milestone R04 Technical Indicator Engine (f64, WASM-safe)
│   │   ├── mod.rs                  # Indicator trait, IndicatorCache, SeriesBlockIndex & conflated envelopes
│   │   ├── moving_averages.rs      # SMA (cancellation-safe reseed) & EMA (Sterbenz weights, Optional slice)
│   │   ├── bands.rs                # Bollinger Bands (20, 2.0 population σ, overflow-safe scaled fallback)
│   │   ├── oscillators.rs          # Wilder RSI (14), MACD (12, 26, 9), Wilder ATR (14) & reference levels
│   │   └── tests.rs                # TA-Lib/Wilder golden fixtures, 100K pan invariance & Critic R04 suites
│   ├── drawings/                   # Milestone R05 Interactive Drawing & Annotation Engine (f64, WASM-safe)
│   │   ├── mod.rs                  # Drawing, DrawingAnchor, DrawingPaneTarget, 5 geometries & style schema
│   │   ├── geometry.rs             # PaneDrawingProjection, ClipEdge Liang-Barsky ray/segment clipping & Fib levels
│   │   ├── hit_test.rs             # LWC priority hit-testing (SelectedHandle>Handle>Stroke>SelectedBody) & OHLC magnet
│   │   ├── controller.rs           # DrawingStore, 1/2-click + drag state machine, 64-step Undo/Redo & serde safety
│   │   └── tests.rs                # Liang-Barsky, $360^\circ$ ray, Fib, magnet, log-drag & Critic R05 suites
│   ├── chart.rs                    # MultiPaneLayout, ChartViewportState, drawing arbitrage, Mesh batching & UI
│   └── app.rs                      # SumiWorkstationApp shell, indicator/drawing toolbars, persistence & captures
├── examples/
│   └── viewport_bench.rs           # Reproducible 10K/50K/100K R03, --r04 & --r05 release benchmarks
└── docs/
    ├── operations/                 # SUMI_ORCHESTRATION_CHARTER.md (durable multi-session orchestration)
    ├── research/                   # REA traces, LWC v5.2.0 & TA-Lib specs, viewport/indicator/drawing designs
    ├── evidence/                   # Native Metal PNG captures, runtime JSON telemetry, benchmark reports
    └── sessions/                   # Chronological session registry
```

---

## 2. Coordinate Spaces & Non-Monolithic Multi-Pane Boundaries

Per [`docs/research/RUST_VIEWPORT_CAMERA_DESIGN.md`](research/RUST_VIEWPORT_CAMERA_DESIGN.md), [`docs/research/INDICATOR_AND_MULTIPANE_SPEC.md`](research/INDICATOR_AND_MULTIPANE_SPEC.md), and [`docs/research/DRAWING_ENGINE_SPEC.md`](research/DRAWING_ENGINE_SPEC.md), Sumi strictly separates horizontal time state, per-pane vertical price state, indicator/drawing domain state, and screen layout offsets:

1. **Logical / Data Coordinates (`f64` / `i64`):**
   - `bar_index` (`0..total_bars` for loaded candles and aligned `Vec<Option<f64>>` indicator outputs; continuous `f64` `DrawingAnchor::bar_index` across historical `< 0` and future `>= total_bars` whitespace).
   - `raw_price` / `DrawingAnchor::value` (`f64` raw domain price, volume, or oscillator value) and transformed `logical_price` (`Normal`, `Logarithmic` via adaptive `LogFormula`, `Percentage`, `IndexedTo100` on `Main`; independent linear `sumi_oscillator` `PriceScale` per oscillator sub-pane).
2. **Pane-Local Coordinates (`f64`):**
   - `local_x ∈ [0.0, plot_width)` owned by [`TimeScale`](../src/viewport/time_scale.rs) — **shared** across `Main` and all active oscillator sub-panes (`RSI`, `MACD`, `ATR`), using one unified `conflation_factor(dpr)`.
   - `local_y ∈ [0.0, pane_plot_height)` owned by an **independent** [`PriceScale`](../src/viewport/price_scale.rs) per pane/axis (`main_price_scale`, `volume_price_scale`, `rsi_price_scale`, `macd_price_scale`, `atr_price_scale`).
   - [`PaneDrawingProjection`](../src/drawings/geometry.rs) maps `(bar_index, value) <-> LocalPoint` and clips finite segments, $360^\circ$ rays, rectangles, and Fibonacci levels analytically in `f64` against `[0.0, width] × [0.0, height]`.
3. **Screen Coordinates (`f64` → `egui::Pos2` `f32` at paint boundary):**
   - Partitioned vertically by [`MultiPaneLayout`](../src/chart.rs) (`stretch_factors` `4.0` main : `1.0` per active oscillator, `MultiPaneLayout::MIN_PANE_HEIGHT = 44.0`, `5.0 px` interactive splitters via `SEPARATOR_HEIGHT = 5.0`) and bridged per pane via [`PaneViewport`](../src/viewport/pane_viewport.rs).

---

## 3. Indicator, Multi-Pane & Interactive Drawing Pipeline (`src/indicators/`, `src/drawings/` & `src/chart.rs`)

Each frame in `render_workstation_chart_with_drawings` (which shares `render_workstation_chart_inner` with the R04-compatible `render_workstation_chart` entrypoint) executes a deterministic, zero-recompute pipeline:

1. **Full-History Indicator Cache (`IndicatorCache`):** Evaluates `SMA(20)`, `EMA(50)`, `BollingerBands(20, 2.0)`, `RSI(14)`, `MACD(12, 26, 9)`, and `ATR(14)` only on `(dataset_generation, candles.len(), IndicatorConfig)` change and builds 16-bar `SeriesBlockIndex` trees.
2. **Multi-Pane Stretch Partitioning & Kinetic Step:** Partitions stacked pane rects (`MultiPaneLayout`) and advances `KineticAnimation` (`step_shared_kinetic_inertia`) prior to pre-input `O(N/16)` autoscale so `base_value` and `PriceScale` are exact on the current frame.
3. **Drawing Input Arbitrage & Synchronized Viewport Gestures:**
   - Evaluates `DrawingStore` (`pointer_down`, `pointer_move`, `pointer_up`, `hit_test_pane_drawings`, `snap_to_visible_ohlc`) per pane using current-frame `PaneDrawingProjection` (`2.46 µs` project + `3.21 µs` 4-pane hit-test for `25` drawings).
   - Active tool placements and unlocked handle/body drags (`>= EDIT_DRAG_THRESHOLD_PX = 2.0 px`) return `PointerCaptureOutcome::CapturedByDrawing` (suppressing chart drag-pan on that pane while preserving wheel zoom), whereas clicks/drags on empty canvas or locked strokes return `PassThroughToViewport`.
4. **Unified Conflation, `egui::Mesh` Batching & Phase-Anchored Drawing Overlay:** Emits `egui::Mesh` batches for candlesticks, volume, MACD histogram, Bollinger ribbons, `Rectangle` fills, and `FibonacciRetracement` alternating bands, plus phase-anchored dashed/dotted lines (`paint_styled_segment_phased`), priority-spaced Fib labels, selection/hover anchor handles, and cross-pane crosshair (`3.521 ms` median at `100,000` bars across `4` panes + `6` indicators + `25` drawings).

---

## 4. Legacy Compatibility, Background Capture & Portability Rules

- [`ViewportMapping`](../src/model.rs) from Session 01, single-pane `render_candlestick_chart` / `render_shared_pane_candlestick_chart` from Session R03, and multi-pane `render_workstation_chart` from Session R04 remain intact with zero breaking changes.
- Automated multi-scenario GPU screenshot batches (`SUMI_CAPTURE_BATCH=r04` and `SUMI_CAPTURE_BATCH=r05`) run inside a single `.with_active(false)` window (`src/main.rs` + `src/app.rs`) so verification never steals macOS focus or flashes multiple windows.
- `src/model.rs`, `src/data.rs`, `src/viewport/`, `src/indicators/`, and `src/drawings/` use zero OS-specific APIs and pass `cargo clippy --target wasm32-unknown-unknown -- -D warnings`.
