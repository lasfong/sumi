# SUMI TRADING WORKSTATION — ARCHITECTURE (`MILESTONE R03`)

- **Updated By:** `SESSION-R03-20261111T105832+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Crate Name:** `sumi-workstation` (`v0.1.0`, Rust Edition 2024, `egui`/`eframe 0.36.0`)

---

## 1. Crate & Directory Structure

The workstation uses a single-crate library + binary architecture (`src/lib.rs` + `src/main.rs`) with strict module boundaries separating pure `f64` domain/viewport math (`wasm32-unknown-unknown`-portable) from `egui` immediate-mode UI and native macOS windowing:

```text
/Users/mizuhara/workspace/sumi
├── Cargo.toml
├── src/
│   ├── lib.rs                      # Crate root exporting app, chart, data, model, viewport
│   ├── main.rs                     # Native macOS entry point (eframe::run_native)
│   ├── model.rs                    # Pure OHLCV domain model (Candle, PriceBounds, deprecated ViewportMapping)
│   ├── data.rs                     # O(N) deterministic synthetic OHLCV generator (120 .. 100,000+ bars)
│   ├── viewport/                   # Milestone R03 Modular Viewport & Coordinate Engine (f64, WASM-safe)
│   │   ├── mod.rs                  # Public exports + panic-free lwc_clamp
│   │   ├── time_scale.rs           # Shared horizontal TimeScale (zoom, pan, resize, fit, culling, conflation)
│   │   ├── price_scale.rs          # Per-pane/axis PriceScale (Normal, Log, %, Idx100, autoscale, drag-scale)
│   │   ├── pane_viewport.rs        # Explicit 3-space bridge (Logical/Data <-> PaneLocal <-> Screen)
│   │   ├── kinetic.rs              # Exponential-decay inertial panning on right_offset (bar units)
│   │   ├── candle_metrics.rs       # Optimal candlestick/wick pixel parity + zero-alloc OHLCV conflation
│   │   └── tests.rs                # LWC v5.2.0 fixtures + 5,100+ property cases + Critic regression suite
│   ├── chart.rs                    # ChartViewportState, shared-TimeScale multi-pane & egui::Painter renderer
│   └── app.rs                      # SumiWorkstationApp shell, toolbar presets, telemetry & GPU capture
├── examples/
│   └── viewport_bench.rs           # Reproducible 10K/50K/100K release benchmark suite
└── docs/
    ├── operations/                 # SUMI_ORCHESTRATION_CHARTER.md (durable multi-session orchestration)
    ├── research/                   # REA traces, LWC v5.2.0 specs, Rust viewport design, renderer decision
    ├── evidence/                   # Native Metal PNG captures, runtime JSON telemetry, benchmark reports
    └── sessions/                   # Chronological session registry
```

---

## 2. Coordinate Spaces & Non-Monolithic Viewport Boundaries

Per [`docs/research/RUST_VIEWPORT_CAMERA_DESIGN.md`](research/RUST_VIEWPORT_CAMERA_DESIGN.md), Sumi strictly separates horizontal time state, vertical price state, and screen layout offsets:

1. **Logical / Data Coordinates (`f64` / `i64`):**
   - `bar_index` (`0..total_bars` for loaded candles; `< 0` historical whitespace; `>= total_bars` future `right_offset` whitespace).
   - `raw_price` (`f64` domain price or volume) and transformed `logical_price` (`Normal`, `Logarithmic` via adaptive `LogFormula`, `Percentage`, `IndexedTo100` relative to dynamic visible `base_value`).
2. **Pane-Local Coordinates (`f64`):**
   - `local_x ∈ [0.0, plot_width)` owned by [`TimeScale`](../src/viewport/time_scale.rs) — **shared** across all vertically stacked panes in a chart workspace so bar `i` aligns at the exact horizontal coordinate in every pane.
   - `local_y ∈ [0.0, plot_height)` owned by [`PriceScale`](../src/viewport/price_scale.rs) — **independent** per pane and per axis (`main-right`, `volume-overlay`, and future oscillator panes).
3. **Screen Coordinates (`f64` → `egui::Pos2` `f32` at paint boundary):**
   - Composed by [`PaneViewport`](../src/viewport/pane_viewport.rs) using precision `PlotRect` (`screen_x = plot_rect.left + local_x`, `screen_y = plot_rect.top + local_y`).

---

## 3. Rendering & Interaction Pipeline (`src/chart.rs`)

Each frame in `render_candlestick_chart` executes a deterministic, zero-lag pipeline:

1. **Resize-Aware Partitioning (`ChartLayout`):** Splits available area into `plot_rect`, `price_axis_rect` (`86 px`), `time_axis_rect` (`28 px`), and `corner_rect`, creating three isolated `ui.interact` zones (`sumi_plot_canvas`, `sumi_price_axis`, `sumi_time_axis`).
2. **State & Kinetic Synchronization:** Updates `TimeScale::set_total_bars`, `TimeScale::set_width`, and `PriceScale::set_height`; advances `KineticAnimation` on `right_offset` when active and schedules `ctx.request_repaint()`.
3. **Same-Frame Input Handling:**
   - **Plot Canvas:** Cursor-anchored wheel zoom (`TimeScale::zoom(local_x, scale)`), horizontal trackpad swipe (`scroll_by_pixels`), sub-pixel drag pan (`start_scroll`/`scroll_to`/`end_scroll`) feeding `KineticAnimation`, manual vertical pan when `!auto_scale`, and double-click reset.
   - **Right Price Axis:** Vertical drag-scaling around range center (`PriceScale::start_scale`/`scale_to`/`end_scale`) + double-click `reset_auto_scale`.
   - **Bottom Time Axis:** Horizontal drag-scaling (`TimeScale::start_scale`/`scale_to`/`end_scale`) + double-click `restore_default`.
4. **O(1) Slice Culling & Autoscale:** Computes `strict_visible_data_range()` and `culling_data_range()` analytically in `< 1 µs`, derives dynamic `base_value` (`firstValue`), and updates `main_price_scale` and `volume_price_scale` with zero heap allocation.
5. **DPR-Aware Power-of-2 Conflation & Crisp `egui::Painter` Submission:** Aggregates sub-pixel candles via `iter_conflated_candles` (`1x..512x`), computes pixel-parity body/wick widths via `CandlestickBarMetrics`, and emits 2-segment wicks, crisp candle bodies, volume bars, decade-aware log/linear/%/Idx100 grid ticks, latest-price badge, 2-axis crosshair pills, and top-left OHLCV HUD.

---

## 4. Legacy Compatibility & Portability Rules

- [`ViewportMapping`](../src/model.rs) from Session 01 is retained intact under `#[deprecated]` (never deleted, preserving Session 01 regression tests).
- Native screenshot/filesystem telemetry capture in [`src/app.rs`](../src/app.rs) is gated behind `#[cfg(not(target_arch = "wasm32"))]`; `src/model.rs`, `src/data.rs`, and `src/viewport/` use zero OS-specific APIs.
