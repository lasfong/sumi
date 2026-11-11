# SUMI TRADING WORKSTATION — ARCHITECTURE (`MILESTONE R04`)

- **Updated By:** `SESSION-R04-20261111T140100+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Crate Name:** `sumi-workstation` (`v0.1.0`, Rust Edition 2024, `egui`/`eframe 0.36.0`)

---

## 1. Crate & Directory Structure

The workstation uses a single-crate library + binary architecture (`src/lib.rs` + `src/main.rs`) with strict module boundaries separating pure `f64` domain, viewport, and technical indicator engines (`wasm32-unknown-unknown`-verified via `cargo clippy --target wasm32-unknown-unknown -- -D warnings`) from `egui` immediate-mode UI and native macOS windowing:

```text
/Users/mizuhara/workspace/sumi
├── Cargo.toml
├── src/
│   ├── lib.rs                      # Crate root exporting app, chart, data, indicators, model, viewport
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
│   ├── chart.rs                    # MultiPaneLayout, ChartViewportState, egui::Mesh batching & multi-pane UI
│   └── app.rs                      # SumiWorkstationApp shell, indicator toggles, SUMI_CAPTURE_BATCH & telemetry
├── examples/
│   └── viewport_bench.rs           # Reproducible 10K/50K/100K R03 & --r04 multi-pane release benchmarks
└── docs/
    ├── operations/                 # SUMI_ORCHESTRATION_CHARTER.md (durable multi-session orchestration)
    ├── research/                   # REA traces, LWC v5.2.0 & TA-Lib specs, viewport/indicator/renderer designs
    ├── evidence/                   # Native Metal PNG captures, runtime JSON telemetry, benchmark reports
    └── sessions/                   # Chronological session registry
```

---

## 2. Coordinate Spaces & Non-Monolithic Multi-Pane Boundaries

Per [`docs/research/RUST_VIEWPORT_CAMERA_DESIGN.md`](research/RUST_VIEWPORT_CAMERA_DESIGN.md) and [`docs/research/INDICATOR_AND_MULTIPANE_SPEC.md`](research/INDICATOR_AND_MULTIPANE_SPEC.md), Sumi strictly separates horizontal time state, per-pane vertical price state, indicator computation, and screen layout offsets:

1. **Logical / Data Coordinates (`f64` / `i64`):**
   - `bar_index` (`0..total_bars` for loaded candles and aligned `Vec<Option<f64>>` indicator outputs; `< 0` historical whitespace; `>= total_bars` future `right_offset` whitespace).
   - `raw_price` (`f64` domain price, volume, or oscillator value) and transformed `logical_price` (`Normal`, `Logarithmic` via adaptive `LogFormula`, `Percentage`, `IndexedTo100` on the main pane; independent linear `sumi_oscillator` `PriceScale` per oscillator sub-pane).
2. **Pane-Local Coordinates (`f64`):**
   - `local_x ∈ [0.0, plot_width)` owned by [`TimeScale`](../src/viewport/time_scale.rs) — **shared** across the main price pane and all active oscillator sub-panes (`RSI`, `MACD`, `ATR`), using one unified `conflation_factor(dpr)` so bar `i` and bucket `[b_start..b_end]` align pixel-identically across every stacked pane.
   - `local_y ∈ [0.0, pane_plot_height)` owned by an **independent** [`PriceScale`](../src/viewport/price_scale.rs) per pane/axis (`main_price_scale`, `volume_price_scale`, `rsi_price_scale`, `macd_price_scale`, `atr_price_scale`).
3. **Screen Coordinates (`f64` → `egui::Pos2` `f32` at paint boundary):**
   - Partitioned vertically by [`MultiPaneLayout`](../src/chart.rs) using LWC-style `stretch_factors` (`4.0` main : `1.0` per active oscillator, `MultiPaneLayout::MIN_PANE_HEIGHT = 44.0`, monotonic pass-snapshot water-filling, `5.0 px` interactive horizontal splitters via `SEPARATOR_HEIGHT = 5.0`) and bridged per pane via [`PaneViewport`](../src/viewport/pane_viewport.rs).

---

## 3. Indicator Caching & Multi-Pane Rendering Pipeline (`src/indicators/` & `src/chart.rs`)

Each frame in `render_workstation_chart` executes a deterministic, zero-recompute pipeline:

1. **Full-History Indicator Cache (`IndicatorCache`):** Evaluates `SMA(20)`, `EMA(50)`, `BollingerBands(20, 2.0)`, `RSI(14)`, `MACD(12, 26, 9)`, and `ATR(14)` only when `(dataset_generation, candles.len(), IndicatorConfig)` changes (`0.041 µs` warm check; `9.63 ms` cold all-6 compute on `100,000` bars) and builds 16-bar associative `SeriesBlockIndex` trees.
2. **Multi-Pane Stretch Partitioning (`MultiPaneLayout`):** Splits total workspace into stacked `main_plot_rect` + `SubPaneLayout` rows (`oscillators`: optional `RSI`, `MACD`, `ATR`), interactive `PaneSeparatorLayout` splitters (`separators`), per-pane right `price_axis_rect` (`86 px`), and shared bottom `time_axis_rect` (`28 px`). Dragging separator `i` transfers normalized stretch weight exclusively between adjacent panes `i` and `i + 1`.
3. **Same-Frame Synchronized Input Handling:**
   - Wheel zoom, trackpad swipe, horizontal drag pan (`KineticAnimation`), and plot double-click reset on **any** pane operate on the single shared `TimeScale` (plot double-click resets `TimeScale` + all active panes' `auto_scale`).
   - Vertical manual drag-pan (when `!auto_scale`) and right-gutter drag-scaling/double-click stay strictly scoped to the interacted pane's own `PriceScale`.
4. **O(N/16) Visible Autoscale:** Queries `SeriesBlockIndex::range_bounds` across visible candles + active overlays (`SMA`, `EMA`, `BB` upper/lower) on the main pane, `[0, 100]` union visible RSI (`min_span = 10.0`), zero-inclusive MACD (`line`, `signal`, `histogram`), and visible ATR.
5. **Unified Conflation & `egui::Mesh` Batching:** Uses `TimeScale::conflation_factor(dpr)` across all panes, emitting `egui::Mesh` batches for candlesticks, volume bars, MACD 4-tone histogram bars, and zero-bleed Bollinger ribbons (`add_colored_rect` / 2-triangle quads), plus exact 2-column `[first, min, max, last]` conflated indicator polylines, oscillator reference bands (`70/50/30` RSI, `0` MACD), synchronized cross-pane vertical crosshair line, active-pane horizontal price pill, and bucket-consistent top-left HUDs (`3.141 ms` median at `100,000` bars across 4 panes).

---

## 4. Legacy Compatibility, Background Capture & Portability Rules

- [`ViewportMapping`](../src/model.rs) from Session 01 and single-pane `render_candlestick_chart` / `render_shared_pane_candlestick_chart` from Session R03 remain intact with zero breaking changes.
- Automated multi-scenario GPU screenshot batches (`SUMI_CAPTURE_BATCH=r04`) run inside a single `.with_active(false)` window (`src/main.rs` + `src/app.rs`) so verification never steals macOS focus or flashes multiple windows.
- `src/model.rs`, `src/data.rs`, `src/viewport/`, and `src/indicators/` use zero OS-specific APIs and pass `cargo clippy --target wasm32-unknown-unknown -- -D warnings`.
