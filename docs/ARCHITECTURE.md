# SUMI TRADING WORKSTATION — ARCHITECTURE (`MILESTONE R06`)

- **Updated By:** `SESSION-R06-20261111T194303+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Crate Name:** `sumi-workstation` (`v0.1.0`, Rust Edition 2024, `egui`/`eframe 0.36.2`, `wgpu 30.0.1`)

---

## 1. Crate & Directory Structure

The workstation uses a single-crate library + binary architecture (`src/lib.rs` + `src/main.rs`) with strict module boundaries separating pure `f64`/`i64` domain, CSV/JSON data ingestion, multi-timeframe aggregation, viewport, technical indicator, and interactive drawing engines (`wasm32-unknown-unknown`-verified via `cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings`, zero new crates in `Cargo.toml`) from `egui` immediate-mode UI and native macOS Metal windowing/offscreen rendering:

```text
/Users/mizuhara/workspace/sumi
├── Cargo.toml
├── src/
│   ├── lib.rs                      # Crate root exporting app, chart, data, drawings, indicators, model, viewport
│   ├── main.rs                     # Native macOS entry point (routes SUMI_CAPTURE_BATCH / SUMI_SCREENSHOT_PATH to offscreen Metal)
│   ├── model.rs                    # Pure OHLCV domain model (Candle with i64 UTC timestamp, PriceBounds, ViewportMapping)
│   ├── data/                       # Milestone R06 Zero-Dependency OHLCV Loader & Multi-Timeframe Engine (WASM-safe)
│   │   ├── mod.rs                  # DataLoadOptions, DuplicateTimestampPolicy, SymbolTracker & canonical normalization
│   │   ├── calendar.rs             # Hinnant O(1) proleptic Gregorian UTC calendar, multi-format timestamp parser & adaptive labels
│   │   ├── timeframe.rs            # Timeframe (1m..1W) upward lattice, aggregate_ohlcv & causal DEF-001 aggregate_ohlcv_up_to
│   │   ├── csv_loader.rs           # Streaming RFC-4180 CSV/TSV/semicolon parser, header alias + CafeF 8-col intraday & bounded I/O
│   │   ├── json_loader.rs          # Zero-dependency recursive-descent JSON kline/object/envelope parser (depth <= 16)
│   │   ├── presets.rs              # Bundled VN30F1M 1m, BTCUSDT 15m JSON, and archived CafeF/SPY CSV loaders
│   │   ├── synthetic.rs            # O(N) deterministic synthetic OHLCV generator (120 .. 100,000+ bars, R03 prefix-preserving)
│   │   └── tests.rs                # Calendar bijection, RFC-4180, JSON klines, CafeF/SPY fixtures, drawing remap & Critic R06 suites
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
│   ├── drawings/                   # Milestone R05+R06 Interactive Drawing & Annotation Engine (f64 + UTC timestamp, WASM-safe)
│   │   ├── mod.rs                  # Drawing, DrawingAnchor (bar_index + Option<f64> timestamp), DrawingPaneTarget & 5 geometries
│   │   ├── geometry.rs             # PaneDrawingProjection, ClipEdge Liang-Barsky ray/segment clipping & Fib levels
│   │   ├── hit_test.rs             # LWC priority hit-testing (SelectedHandle>Handle>Stroke>SelectedBody) & OHLC magnet
│   │   ├── controller.rs           # DrawingStore, remap_timeframe, 1/2-click + drag state machine, 64-step Undo/Redo & serde safety
│   │   └── tests.rs                # Liang-Barsky, $360^\circ$ ray, Fib, magnet, log-drag & Critic R05 suites
│   ├── chart.rs                    # MultiPaneLayout, adaptive intraday/daily time axis, drawing arbitrage, Mesh batching & UI
│   └── app.rs                      # SumiWorkstationApp shell, data/TF/indicator/drawing toolbars, per-instrument drawings & OffscreenWgpuHarness
├── examples/
│   └── viewport_bench.rs           # Reproducible 10K/50K/100K R03, --r04, --r05 & --r06 release benchmarks
└── docs/
    ├── operations/                 # SUMI_ORCHESTRATION_CHARTER.md (durable multi-session orchestration)
    ├── research/                   # REA traces, LWC v5.2.0 & TA-Lib specs, viewport/indicator/drawing/data-timeframe designs
    ├── evidence/                   # Offscreen & windowed Metal PNG captures, runtime JSON telemetry, benchmark reports
    └── sessions/                   # Chronological session registry
```

---

## 2. Coordinate Spaces, Timeframe Continuity & Multi-Pane Boundaries

Per [`docs/research/RUST_VIEWPORT_CAMERA_DESIGN.md`](research/RUST_VIEWPORT_CAMERA_DESIGN.md), [`docs/research/INDICATOR_AND_MULTIPANE_SPEC.md`](research/INDICATOR_AND_MULTIPANE_SPEC.md), [`docs/research/DRAWING_ENGINE_SPEC.md`](research/DRAWING_ENGINE_SPEC.md), and [`docs/research/DATA_LOADER_AND_TIMEFRAME_SPEC.md`](research/DATA_LOADER_AND_TIMEFRAME_SPEC.md), Sumi strictly separates horizontal time state, per-pane vertical price state, UTC timestamp continuity, and screen layout offsets:

1. **Logical / Data & Canonical UTC Time Coordinates (`f64` / `i64`):**
   - `bar_index` (`0..total_bars` for active `Candle` slices and aligned `Vec<Option<f64>>` indicator outputs; continuous `f64` `DrawingAnchor::bar_index` across historical `< 0` and future `>= total_bars` whitespace).
   - `Candle::timestamp: i64` (canonical UTC epoch seconds on every candle) and `DrawingAnchor::timestamp: Option<f64>` (fractional-capable UTC epoch seconds preserving exact in-bar and inter-bar offsets), mapped bijectively via Hinnant O(1) calendar math (`src/data/calendar.rs`) and bucketed across `Timeframe` (`1m`, `5m`, `15m`, `1h`, `4h`, `1D`, `1W`). When switching timeframes on an instrument, `DrawingStore::remap_timeframe` resolves exact in-series or fractional out-of-range `bar_index` positions in $O(\log N)$ per anchor without drift across arbitrary round-trips.
   - `raw_price` / `DrawingAnchor::value` (`f64` raw domain price, volume, or oscillator value) and transformed `logical_price` (`Normal`, `Logarithmic`, `Percentage`, `IndexedTo100` on `Main`; independent linear `sumi_oscillator` `PriceScale` per oscillator sub-pane).
2. **Pane-Local Coordinates (`f64`):**
   - `local_x ∈ [0.0, plot_width)` owned by [`TimeScale`](../src/viewport/time_scale.rs) — **shared** across `Main` and all active oscillator sub-panes (`RSI`, `MACD`, `ATR`), using one unified `conflation_factor(dpr)`.
   - `local_y ∈ [0.0, pane_plot_height)` owned by an **independent** [`PriceScale`](../src/viewport/price_scale.rs) per pane/axis (`main_price_scale`, `volume_price_scale`, `rsi_price_scale`, `macd_price_scale`, `atr_price_scale`).
   - [`PaneDrawingProjection`](../src/drawings/geometry.rs) maps `(bar_index, value) <-> LocalPoint` and clips finite segments, $360^\circ$ rays, rectangles, and Fibonacci levels analytically in `f64` against `[0.0, width] × [0.0, height]`.
3. **Screen Coordinates (`f64` → `egui::Pos2` `f32` at paint boundary):**
   - Partitioned vertically by [`MultiPaneLayout`](../src/chart.rs) (`stretch_factors` `4.0` main : `1.0` per active oscillator, `MultiPaneLayout::MIN_PANE_HEIGHT = 44.0`, `5.0 px` interactive splitters) and bridged per pane via [`PaneViewport`](../src/viewport/pane_viewport.rs), with collision-free adaptive UTC time-axis tick formatting (`YYYY-MM-DD HH:MM`, `MM-DD HH:MM`, `HH:MM`, `YYYY-MM-DD`).

---

## 3. Data Ingestion, Multi-Timeframe, Indicator & Drawing Pipeline

1. **Zero-Dependency CSV/JSON Ingestion & Timeframe Lattice (`src/data/`):**
   - Streams CSV/TSV/semicolon files (`122.1–191.1 MiB/s`) and JSON kline/object/envelope feeds (`134.9 MiB/s`) with strict resource bounds (`MAX_IMPORT_BYTES = 64 MiB`, `MAX_SCANNED_RECORDS = 2_000_000`, `MAX_IMPORT_RECORDS = 500_000`, `MAX_JSON_DEPTH = 16`), multi-symbol discovery (`CafeF.INDEX.clean.csv` `HNX-INDEX` / `VNINDEX`), and canonical upward aggregation (`aggregate_ohlcv` in `0.45–2.61 ms` for `100K` `1m` bars; causal `aggregate_ohlcv_up_to` enforcing `DEF-001`).
2. **Full-History Indicator Cache (`IndicatorCache`):** Evaluates `SMA(20)`, `EMA(50)`, `BollingerBands(20, 2.0)`, `RSI(14)`, `MACD(12, 26, 9)`, and `ATR(14)` only on `(dataset_generation, candles.len(), IndicatorConfig)` change and builds 16-bar `SeriesBlockIndex` trees.
3. **Multi-Pane Layout, Drawing Arbitrage & `egui::Mesh` Batching (`src/chart.rs`):**
   - Partitions stacked panes, advances `KineticAnimation`, evaluates `DrawingStore` (`pointer_down`, `pointer_move`, `pointer_up`, `hit_test_pane_drawings`, `snap_to_visible_ohlc`), and emits `egui::Mesh` batches + phase-anchored overlays (`0.185 ms` intraday standard, **`3.761 ms`** median at `100,000` `1m` bars full-fit across `4` panes + `6` indicators + `25` drawings — below the `< 4.0 ms` `DECISION-RENDERER-002` ceiling).

---

## 4. Legacy Compatibility, Offscreen Metal Capture & Portability Rules

- [`ViewportMapping`](../src/model.rs) from Session 01, `generate_synthetic_ohlcv` from Session R03, `render_candlestick_chart` / `render_workstation_chart`, and `render_workstation_chart_with_drawings` remain intact with zero breaking changes.
- Automated verification captures (`SUMI_CAPTURE_BATCH=r04|r05|r06` and `SUMI_SCREENSHOT_PATH`) execute offscreen on Apple M1 Metal via `OffscreenWgpuHarness` (`eframe::egui_wgpu::RenderState`, `Rgba8Unorm`, `2.0x` Retina), opening **zero** windows, never touching macOS focus or Dock, and operating reliably even when the physical display is asleep.
- `src/model.rs`, `src/data/`, `src/viewport/`, `src/indicators/`, and `src/drawings/` use zero OS-specific APIs (with native file-path helpers cleanly gated behind `#[cfg(not(target_arch = "wasm32"))]`) and pass `cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings`.
