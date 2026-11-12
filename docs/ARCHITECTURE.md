# SUMI TRADING WORKSTATION — ARCHITECTURE (`MILESTONE R08`)

- **Updated By:** `SESSION-R08-20261112T140031+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Crate Name:** `sumi-workstation` (`v0.1.0`, Rust Edition 2024, `egui`/`eframe 0.36.2`, `wgpu 30.0.1`)

---

## 1. Crate & Directory Structure

The workstation uses a single-crate library + binary architecture (`src/lib.rs` + `src/main.rs`) with strict module boundaries separating pure `f64`/`i64` domain, CSV/JSON data ingestion, multi-timeframe aggregation, viewport, technical indicator, interactive drawing, causal bar replay, paper trading execution, multi-symbol watchlist/scanner, and causal alert engines (`wasm32-unknown-unknown`-verified via `cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings`, zero new crates in `Cargo.toml`) from `egui` immediate-mode UI and native macOS Metal windowing/offscreen rendering:

```text
/Users/mizuhara/workspace/sumi
├── Cargo.toml
├── src/
│   ├── lib.rs                      # Crate root exporting alerts, app, chart, data, drawings, indicators, model, replay, scanner, trading, viewport
│   ├── main.rs                     # Native macOS entry point (routes SUMI_CAPTURE_BATCH / SUMI_SCREENSHOT_PATH to offscreen Metal)
│   ├── model.rs                    # Pure OHLCV domain model (Candle with i64 UTC timestamp, PriceBounds, ViewportMapping)
│   ├── data/                       # Milestone R06–R08 Zero-Dependency OHLCV Loader & Multi-Timeframe Engine (WASM-safe)
│   │   ├── mod.rs                  # DataLoadOptions, DuplicateTimestampPolicy, SymbolTracker & canonical normalization
│   │   ├── calendar.rs             # Hinnant O(1) proleptic Gregorian UTC calendar, multi-format timestamp parser & adaptive labels
│   │   ├── timeframe.rs            # Timeframe (1m..1W) upward lattice, aggregate_ohlcv & causal DEF-001 aggregate_ohlcv_up_to
│   │   ├── csv_loader.rs           # Streaming RFC-4180 CSV/TSV/semicolon parser, header alias + CafeF 8-col intraday & bounded I/O
│   │   ├── json_loader.rs          # Zero-dependency recursive-descent JSON kline/object/envelope parser (depth <= 16)
│   │   ├── presets.rs              # Bundled VN30F1M 1m, VN30 Basket (8 equities), BTCUSDT 15m JSON, and archived CafeF/SPY CSV loaders
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
│   ├── drawings/                   # Milestone R05–R07 Interactive Drawing & Annotation Engine (f64 + UTC + Causal, WASM-safe)
│   │   ├── mod.rs                  # Drawing (created_at_timestamp + is_causally_visible), DrawingAnchor & 5 geometries
│   │   ├── geometry.rs             # PaneDrawingProjection, ClipEdge Liang-Barsky ray/segment clipping & Fib levels
│   │   ├── hit_test.rs             # Causal LWC priority hit-testing (SelectedHandle>Handle>Stroke>SelectedBody) & OHLC magnet
│   │   ├── controller.rs           # DrawingStore, causal replay cutoff guards, remap_timeframe, 64-step Undo/Redo & serde safety
│   │   └── tests.rs                # Liang-Barsky, $360^\circ$ ray, Fib, magnet, log-drag & Critic R05 suites
│   ├── replay/                     # Milestone R07 Causal Bar Replay Engine (f64/i64, WASM-safe)
│   │   ├── mod.rs                  # ReplaySpeed (1x..20x), ReplayStepUnit (ActiveBar, BaseBar), ReplayStatus & ReplayTransition
│   │   ├── controller.rs           # ReplayController, DEF-001 prefix materialization, step/jump/tick state machine & serde safety
│   │   └── tests.rs                # Multi-timeframe DEF-001 lookahead lock, partial bars, causal drawings & Critic R07 suite
│   ├── trading/                    # Milestone R07 Paper Trading Execution & Portfolio Engine (f64/i64, WASM-safe)
│   │   ├── mod.rs                  # Public exports for AccountConfig, Order, Position, ExecutionFill, ClosedTrade & TradingSession
│   │   ├── orders.rs               # OrderSide, PositionSide, OrderKind (Market/Limit/Stop), FillReason & PortfolioMetrics
│   │   ├── portfolio.rs            # TradingSession, 4-waypoint OHLC matching, fee-conserving ledger & command-log rewind_to
│   │   └── tests.rs                # Gap-open/dual-touch brackets, 1m==15m parity, fee conservation, rewind & Critic R07 suites
│   ├── scanner/                    # Milestone R08 Multi-Symbol Watchlist & Technical Scanner Engine (f64/i64, WASM-safe)
│   │   ├── mod.rs                  # MarketRegime, ScanSignalKind, SymbolScanRow, HistoricalSignalHit & ScannerSignalContext
│   │   ├── engine.rs               # Single-pass evaluate_symbol_scan_and_signals, DEF-001 cutoff & None-last sorting
│   │   └── tests.rs                # Causal RVol/regime, BlindPractice bucket-close reveal, 13-symbol universe & Critic R08 suite
│   ├── alerts/                     # Milestone R08 Causal Price, Indicator & Sloped Drawing-Line Alert Engine (f64/i64, WASM-safe)
│   │   ├── mod.rs                  # AlertCondition, AlertRule, AlertTriggerEvent, CrossDirection, AlertFrequency & AlertError
│   │   ├── engine.rs               # AlertEngine, 4-waypoint/@Mark crossings, Wilder RSI/MACD, ray relative-offset & causal rewind
│   │   └── tests.rs                # 4-waypoint/gap, RSI/MACD, parallel-safe ray/trendline, 1m==15m parity & Critic R08 suites
│   ├── chart.rs                    # MultiPaneLayout, Replay/Trading/Alert Overlays, Trigger Diamonds, Toast & Mesh batching
│   └── app.rs                      # SumiWorkstationApp shell, Watchlist/Scanner Dock, Alerts Panel, Cmd+K Palette & OffscreenWgpuHarness
├── examples/
│   └── viewport_bench.rs           # Reproducible 10K/50K/100K R03, --r04, --r05, --r06, --r07 & --r08 release benchmarks
└── docs/
    ├── operations/                 # SUMI_ORCHESTRATION_CHARTER.md (durable multi-session orchestration)
    ├── research/                   # REA traces, LWC v5.2.0 & TA-Lib specs, viewport/indicator/drawing/data/replay/scanner-alerts designs
    ├── evidence/                   # Offscreen & windowed Metal PNG captures, runtime JSON telemetry, benchmark reports
    └── sessions/                   # Chronological session registry
```

---

## 2. Coordinate Spaces, Timeframe Continuity & Causal Replay Boundaries

Per [`docs/research/RUST_VIEWPORT_CAMERA_DESIGN.md`](research/RUST_VIEWPORT_CAMERA_DESIGN.md), [`docs/research/INDICATOR_AND_MULTIPANE_SPEC.md`](research/INDICATOR_AND_MULTIPANE_SPEC.md), [`docs/research/DRAWING_ENGINE_SPEC.md`](research/DRAWING_ENGINE_SPEC.md), [`docs/research/DATA_LOADER_AND_TIMEFRAME_SPEC.md`](research/DATA_LOADER_AND_TIMEFRAME_SPEC.md), [`docs/research/BAR_REPLAY_AND_PAPER_TRADING_SPEC.md`](research/BAR_REPLAY_AND_PAPER_TRADING_SPEC.md), and [`docs/research/SCANNER_ALERTS_AND_COMMAND_PALETTE_SPEC.md`](research/SCANNER_ALERTS_AND_COMMAND_PALETTE_SPEC.md), Sumi strictly separates horizontal time state, per-pane vertical price state, UTC timestamp continuity, causal replay frontiers, and screen layout offsets:

1. **Logical / Data, Canonical UTC & Causal Cutoff Coordinates (`f64` / `i64`):**
   - `bar_index` (`0..total_bars` for active `Candle` slices and aligned `Vec<Option<f64>>` indicator outputs; continuous `f64` `DrawingAnchor::bar_index` across historical `< 0` and future `>= total_bars` whitespace).
   - `Candle::timestamp: i64` and `DrawingAnchor::timestamp: Option<f64>`, mapped bijectively via Hinnant O(1) calendar math (`src/data/calendar.rs`) across `Timeframe` (`1m`, `5m`, `15m`, `1h`, `4h`, `1D`, `1W`).
   - When `ReplayController` is active at `base_cursor_index`, `materialize_candles` slices `&base_candles[..=base_cursor_index]` before rolling up into `active_timeframe`, guaranteeing zero future lookahead (`DEF-001`) across candles, incomplete higher-timeframe bars, `IndicatorCache`, cross-symbol `scanner/` snapshots/signals, `ScannerSignalContext` `BlindPractice` reveal gates, and `AlertEngine::evaluate_causal`.
2. **Pane-Local Coordinates (`f64`):**
   - `local_x ∈ [0.0, plot_width)` owned by [`TimeScale`](../src/viewport/time_scale.rs) — **shared** across `Main` and all active oscillator sub-panes (`RSI`, `MACD`, `ATR`), using one unified `conflation_factor(dpr)`.
   - `local_y ∈ [0.0, pane_plot_height)` owned by an **independent** [`PriceScale`](../src/viewport/price_scale.rs) per pane/axis (`main_price_scale`, `volume_price_scale`, `rsi_price_scale`, `macd_price_scale`, `atr_price_scale`).
3. **Screen Coordinates (`f64` → `egui::Pos2` `f32` at paint boundary):**
   - Partitioned vertically by [`MultiPaneLayout`](../src/chart.rs) (`stretch_factors` `4.0` main : `1.0` per active oscillator, `MultiPaneLayout::MIN_PANE_HEIGHT = 44.0`, `5.0 px` interactive splitters) and bridged per pane via [`PaneViewport`](../src/viewport/pane_viewport.rs).

---

## 3. Data, Scanner, Alert, Replay, Paper Trading, Indicator & Drawing Pipeline

1. **Zero-Dependency CSV/JSON Ingestion & 13-Symbol Universe (`src/data/`):** Streams CSV/TSV (`103.7–191.1 MiB/s`) and JSON kline/envelope feeds (`134.9 MiB/s`) with multi-symbol discovery (`SymbolTracker`, `custom_peer_feeds`), `Vn30Basket`, and upward aggregation (`1m..1W`).
2. **Single-Pass Multi-Symbol Watchlist & Signal Scanner (`src/scanner/`):** Evaluates `SymbolScanRow` + `HistoricalSignalHit` across all `13` bundled feeds (`20,797` bars) in **`1.799 ms`**, gating every symbol to `<= replay_cutoff` (`DEF-001`) and launching `BlindPractice` / `SignalReview` replay sessions in one click.
3. **Causal Price, Indicator & Drawing-Line Alert Engine (`src/alerts/`):** Evaluates `PriceLevel`, `RsiThreshold`, `MacdHistogramZeroCross`, and `DrawingLine` (`HorizontalLine`, `TrendLine`, $360^\circ$ `Ray`) causally across live feeds and Bar Replay (`Once` alerts automatically re-arm on rewind), sorted monotonically by `eval_base_timestamp`.
4. **Causal Replay & Timeframe-Invariant Paper Trading (`src/replay/`, `src/trading/`):** Advances `base_cursor_index` (`ActiveBar` or `BaseBar`, `1.25 µs` per `15m` step), matches `Market`/`Limit`/`Stop` orders and `SL`/`TP` brackets (`1m == 15m` bit-exact fill parity), conserves commissions, and reconstructs exact historical state on `rewind_to`.
5. **Multi-Pane Layout, `Cmd+K` Modal Isolation & `egui::Mesh` Batching (`src/chart.rs`, `src/app.rs`):** Batches candlesticks, conflated indicator lines, volume, MACD histograms, Bollinger bands, horizontal dashed lines, execution fill arrows, and alert trigger diamonds into `egui::Mesh` primitives (`0.182 ms` active `95K` intraday replay, `0.407 ms` full workstation UI, `0.296 ms` `Cmd+K` modal, **`3.067 ms`** median at `95K/100K` `1m` full-fit — well below the `< 4.0 ms` `DECISION-RENDERER-002` ceiling).

---

## 4. Legacy Compatibility, Offscreen Metal Capture & Portability Rules

- [`ViewportMapping`](../src/model.rs), `generate_synthetic_ohlcv`, `render_candlestick_chart`, `render_workstation_chart`, `render_workstation_chart_with_drawings`, and `render_workstation_chart_full` remain intact with zero breaking changes.
- Automated verification captures (`SUMI_CAPTURE_BATCH=r04|r05|r06|r07|r08` and `SUMI_SCREENSHOT_PATH`) execute offscreen on Apple M1 Metal via `OffscreenWgpuHarness` (`eframe::egui_wgpu::RenderState`, `Rgba8Unorm`, `2.0x` Retina), opening **zero** windows and never touching macOS focus or Dock.
- `src/model.rs`, `src/data/`, `src/viewport/`, `src/indicators/`, `src/drawings/`, `src/replay/`, `src/trading/`, `src/scanner/`, and `src/alerts/` use zero OS-specific APIs and pass `cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings`.
