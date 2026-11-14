# SUMI TRADING WORKSTATION — ARCHITECTURE (`MILESTONE R11`)

- **Updated By:** `SESSION-R11-20261114T231200+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Crate Name:** `sumi-workstation` (`v0.1.0`, Rust Edition 2024, `egui`/`eframe 0.36.2`, `wgpu 30.0.1`)

---

## 1. Crate & Directory Structure

The workstation uses a single-crate library + binary architecture (`src/lib.rs` + `src/main.rs`) with strict module boundaries separating pure `f64`/`i64` domain, CSV/JSON data ingestion, multi-timeframe aggregation, viewport, technical indicator (including Order-Flow Volume Profile, Session/Anchored VWAP, and Causal Relative Strength), interactive drawing, causal bar replay, paper trading execution, multi-symbol watchlist/scanner, causal alert, deterministic strategy backtest / Walk-Forward / multi-asset portfolio, and Vietnam market microstructure risk planner / trade journal engines (`wasm32-unknown-unknown`-verified via `cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings`, zero new crates in `Cargo.toml`) from `egui` immediate-mode UI and native macOS Metal windowing/offscreen rendering:

```text
/Users/mizuhara/workspace/sumi
├── Cargo.toml
├── src/
│   ├── lib.rs                      # Crate root exporting alerts, app, backtest, chart, data, drawings, indicators, journal, model, replay, scanner, trading, viewport
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
│   ├── indicators/                 # Milestone R04 & R11 Technical Indicator + Order-Flow Engine (f64, WASM-safe)
│   │   ├── mod.rs                  # Indicator trait, IndicatorCache, SeriesBlockIndex (first_valid_index) & conflated envelopes
│   │   ├── moving_averages.rs      # SMA (cancellation-safe reseed) & EMA (Sterbenz weights, Optional slice)
│   │   ├── bands.rs                # Bollinger Bands (20, 2.0 population σ, overflow-safe scaled fallback)
│   │   ├── oscillators.rs          # Wilder RSI (14), MACD (12, 26, 9), Wilder ATR (14) & reference levels
│   │   ├── volume_profile.rs       # R11 Order-Flow VRVP/FRVP (OHLCV_PROXY buy/sell, CME 70% VA, POC, HVN/LVN, micro-price guard)
│   │   ├── vwap.rs                 # R11 Exchange-Session VWAP & Anchored VWAP (rescaled Welford ±1σ/±2σ, UTC+7/UTC 1D/1W resets)
│   │   ├── relative_strength.rs    # R11 DEF-001 causal close-aligned RS vs VNINDEX/Peer Benchmark & periodic-recompute RS SMA
│   │   └── tests.rs                # TA-Lib/Wilder, R11 VRVP/VWAP/RS fixtures & Critic R04/R11 regression suites
│   ├── drawings/                   # Milestone R05–R11 Interactive Drawing & Annotation Engine (f64 + UTC + Causal, WASM-safe)
│   │   ├── mod.rs                  # Drawing + 8 geometries (+ RiskRewardBox, FixedRangeVolumeProfile, AnchoredVwap) & causal visibility
│   │   ├── geometry.rs             # PaneDrawingProjection (with_candles), ClipEdge, Fib, RiskRewardBox, FixedRangeVolumeProfile & AVWAP tail
│   │   ├── hit_test.rs             # Causal LWC priority hit-testing (FRVP borders, HandlePart::Anchor + tail_segments, R:R handles) & magnet
│   │   ├── controller.rs           # DrawingStore, non-drifting snapshot handles, AVWAP left clamp, remap_timeframe & Undo/Redo
│   │   └── tests.rs                # Liang-Barsky, Ray, Fib, R:R, FRVP/AVWAP drag/remap & Critic R05/R10/R11 regression suites
│   ├── replay/                     # Milestone R07 Causal Bar Replay Engine (f64/i64, WASM-safe)
│   │   ├── mod.rs                  # ReplaySpeed (1x..20x), ReplayStepUnit (ActiveBar, BaseBar), ReplayStatus & ReplayTransition
│   │   ├── controller.rs           # ReplayController, DEF-001 prefix materialization, step/jump/tick state machine & serde safety
│   │   └── tests.rs                # Multi-timeframe DEF-001 lookahead lock, partial bars, causal drawings & Critic R07 suite
│   ├── trading/                    # Milestone R07 Paper Trading Execution & Portfolio Engine (f64/i64, WASM-safe)
│   │   ├── mod.rs                  # Public exports for AccountConfig, Order, Position, ExecutionFill, ClosedTrade & TradingSession
│   │   ├── orders.rs               # OrderSide, PositionSide, OrderKind (Market/Limit/Stop), FillReason & PortfolioMetrics
│   │   ├── portfolio.rs            # TradingSession, 4-waypoint OHLC matching, fee-conserving ledger, RiskRewardBox ticket extract & rewind_to
│   │   └── tests.rs                # Gap-open/dual-touch brackets, 1m==15m parity, fee conservation, rewind & Critic R07 suites
│   ├── scanner/                    # Milestone R08 Multi-Symbol Watchlist & Technical Scanner Engine (f64/i64, WASM-safe)
│   │   ├── mod.rs                  # MarketRegime, ScanSignalKind, SymbolScanRow, HistoricalSignalHit & ScannerSignalContext
│   │   ├── engine.rs               # Single-pass evaluate_symbol_scan_and_signals, DEF-001 cutoff & None-last sorting
│   │   └── tests.rs                # Causal RVol/regime, BlindPractice bucket-close reveal, 13-symbol universe & Critic R08 suite
│   ├── alerts/                     # Milestone R08 Causal Price, Indicator & Sloped Drawing-Line Alert Engine (f64/i64, WASM-safe)
│   │   ├── mod.rs                  # AlertCondition, AlertRule, AlertTriggerEvent, CrossDirection, AlertFrequency & AlertError
│   │   ├── engine.rs               # AlertEngine, 4-waypoint/@Mark crossings, Wilder RSI/MACD, ray relative-offset & causal rewind
│   │   └── tests.rs                # 4-waypoint/gap, RSI/MACD, parallel-safe ray/trendline, 1m==15m parity & Critic R08 suites
│   ├── backtest/                   # Milestone R09–R10 Strategy Backtest, Walk-Forward & Multi-Asset Portfolio Engine (f64/i64, WASM-safe)
│   │   ├── mod.rs                  # Public exports for BacktestConfig, BacktestRunResult, Sweep, Battle, WalkForward & Portfolio reports
│   │   ├── strategy.rs             # 6 composable presets, sizing modes, SL/TP/ATR trail/T+2 lockup, sanitize & 12-grid sweep builder
│   │   ├── metrics.rs              # Institutional BacktestTearsheet (Sharpe, Sortino, Calmar, CAGR, R-Multiple, Exposure %) & RobustnessVerdict
│   │   ├── engine.rs               # Causal Open[T+1] runner, SimulationCarryState, MarginLiquidation, continuous IS/OOS, sweep & battle
│   │   ├── walk_forward.rs         # K-fold Rolling/Anchored WFO (continuous OOS carry + Pardo WFE) & bar-close Multi-Asset Portfolio
│   │   └── tests.rs                # Next-bar causality, dual-touch, T+2, bankruptcy, WFO carry, mixed-TF portfolio & Critic R09/R10 suites
│   ├── journal/                    # Milestone R10 Vietnam Market Microstructure Risk Planner & Structured Trade Journal (f64/i64, WASM-safe)
│   │   ├── mod.rs                  # Public exports for ExchangeVenue, DailyPriceBand, TradePlanInput/Computed & TradeJournalStore
│   │   ├── market_rules.rs         # HOSE/HNX/UPCOM/VN30F/VNINDEX/US/Crypto ticks, ±7%/±10%/±15% bands, x100k mult, 17% margin & TZ T+2
│   │   ├── planner.rs              # All-in commission/slippage/tax_sell sizer, concentration cap, breakeven, net R & statutory warnings
│   │   ├── ledger.rs               # DEF-001 causal TradeJournalStore, Paper rewind sync, SQN/Discipline %, safe RFC-4180 CSV & Markdown
│   │   └── tests.rs                # HOSE tier-crossing bands, VN30F x100k sizing, local-TZ T+2, CSV formula guard & Critic R10 suite
│   ├── chart.rs                    # MultiPaneLayout (+ Equity & RelativeStrength), R11RenderMemo, VRVP/FRVP/VWAP/AVWAP & Compare Mesh
│   └── app.rs                      # SumiWorkstationApp shell, VRVP/FRVP/AVWAP/Compare/RS controls, Risk/Journal, WFO, Alerts & Cmd+K
├── examples/
│   └── viewport_bench.rs           # Reproducible 10K/50K/100K R03 & --r04..--r11 release benchmarks
└── docs/
    ├── operations/                 # SUMI_ORCHESTRATION_CHARTER.md (durable multi-session orchestration)
    ├── research/                   # REA traces, LWC v5.2.0 & TA-Lib specs, viewport/indicator/drawing/data/replay/scanner/backtest/journal designs
    ├── evidence/                   # Offscreen & windowed Metal PNG captures, runtime JSON telemetry, benchmark reports
    └── sessions/                   # Chronological session registry
```

---

## 2. Coordinate Spaces, Timeframe Continuity & Causal Replay Boundaries

Per [`docs/research/RUST_VIEWPORT_CAMERA_DESIGN.md`](research/RUST_VIEWPORT_CAMERA_DESIGN.md), [`docs/research/INDICATOR_AND_MULTIPANE_SPEC.md`](research/INDICATOR_AND_MULTIPANE_SPEC.md), [`docs/research/DRAWING_ENGINE_SPEC.md`](research/DRAWING_ENGINE_SPEC.md), [`docs/research/DATA_LOADER_AND_TIMEFRAME_SPEC.md`](research/DATA_LOADER_AND_TIMEFRAME_SPEC.md), [`docs/research/BAR_REPLAY_AND_PAPER_TRADING_SPEC.md`](research/BAR_REPLAY_AND_PAPER_TRADING_SPEC.md), [`docs/research/SCANNER_ALERTS_AND_COMMAND_PALETTE_SPEC.md`](research/SCANNER_ALERTS_AND_COMMAND_PALETTE_SPEC.md), [`docs/research/STRATEGY_BACKTEST_LAB_SPEC.md`](research/STRATEGY_BACKTEST_LAB_SPEC.md), [`docs/research/RISK_PLANNER_JOURNAL_AND_PORTFOLIO_SPEC.md`](research/RISK_PLANNER_JOURNAL_AND_PORTFOLIO_SPEC.md), and [`docs/research/VOLUME_PROFILE_VWAP_AND_REL_STRENGTH_SPEC.md`](research/VOLUME_PROFILE_VWAP_AND_REL_STRENGTH_SPEC.md), Sumi strictly separates horizontal time state, per-pane vertical price state, UTC + exchange-local civil time, causal replay frontiers, and screen layout offsets:

1. **Logical / Data, Canonical UTC & Causal Cutoff Coordinates (`f64` / `i64`):**
   - `bar_index` (`0..total_bars` for active `Candle` slices, aligned `Vec<Option<f64>>` indicator/VWAP/AVWAP/RS/Compare outputs, and bar-aligned `BacktestEquityPoint` / `stitched_oos_curve` / `map_equity_curve_to_candles` curves; continuous `f64` `DrawingAnchor::bar_index` across historical `< 0` and future `>= total_bars` whitespace).
   - `Candle::timestamp: i64` and `DrawingAnchor::timestamp: Option<f64>`, mapped bijectively via Hinnant O(1) calendar math (`src/data/calendar.rs`) across `Timeframe` (`1m`, `5m`, `15m`, `1h`, `4h`, `1D`, `1W`), with exchange-local civil timezone offsets (`UTC+7` ICT / `UTC-5` EST) for statutory `T+2`/`T+1` settlement, prior-session reference prices, and intraday Session VWAP resets (`1D`/`1W` MTD/YTD resets use UTC civil dates).
   - When `ReplayController` is active at `base_cursor_index`, `materialize_candles` slices `&base_candles[..=base_cursor_index]` before rolling up into `active_timeframe`, and `replay_causal_close_timestamp` (`cursor_open_ts + base_tf.seconds() - 1`) gates `scanner/`, `AlertEngine::evaluate_causal`, `TradeJournalStore::visible_entries`, `compute_relative_strength` + Rebased Compare overlays, `run_backtest_with_options`, `run_walk_forward_validation`, and `run_portfolio_backtest` so zero future or unclosed coarse-timeframe bars ever leak (`DEF-001`).
2. **Pane-Local Coordinates (`f64`):**
   - `local_x ∈ [0.0, plot_width)` owned by [`TimeScale`](../src/viewport/time_scale.rs) — **shared** across `Main` and all active sub-panes (`RSI`, `MACD`, `ATR`, `RelativeStrength`, `Equity`), using one unified `conflation_factor(dpr)`.
   - `local_y ∈ [0.0, pane_plot_height)` owned by an **independent** [`PriceScale`](../src/viewport/price_scale.rs) per pane/axis (`main_price_scale`, `volume_price_scale`, `rsi_price_scale`, `macd_price_scale`, `atr_price_scale`, `rs_price_scale`, `equity_price_scale`).
3. **Screen Coordinates (`f64` → `egui::Pos2` `f32` at paint boundary):**
   - Partitioned vertically by [`MultiPaneLayout`](../src/chart.rs) (`stretch_factors` `4.0` main : `1.0` default per oscillator including `rs_stretch` (`0.85` in R11 capture presets) : `1.25` default `equity_stretch`, `MultiPaneLayout::MIN_PANE_HEIGHT = 44.0`, `5.0 px` interactive splitters) and bridged per pane via [`PaneViewport`](../src/viewport/pane_viewport.rs).

---

## 3. Domain & Rendering Subsystem Pipeline

1. **Zero-Dependency CSV/JSON Ingestion & 13-Symbol Universe (`src/data/`):** Streams CSV/TSV (`103.7–191.1 MiB/s`) and JSON kline/envelope feeds (`134.9 MiB/s`) with multi-symbol discovery, `Vn30Basket`, tail-coverage-scored benchmark lookup, and upward aggregation (`1m..1W`).
2. **Single-Pass Multi-Symbol Watchlist & Signal Scanner (`src/scanner/`):** Evaluates `SymbolScanRow` + `HistoricalSignalHit` across all `13` bundled feeds (`20,797` bars) in **`1.799 ms`**, enforcing `<= replay_cutoff` (`DEF-001`) and launching `BlindPractice` / `SignalReview` replay sessions in one click.
3. **Causal Price, Indicator & Drawing-Line Alert Engine (`src/alerts/`):** Evaluates `PriceLevel`, `RsiThreshold`, `MacdHistogramZeroCross`, and `DrawingLine` (`HorizontalLine`, `TrendLine`, $360^\circ$ `Ray`) causally across live feeds and Bar Replay.
4. **Causal Replay & Timeframe-Invariant Paper Trading (`src/replay/`, `src/trading/`):** Advances `base_cursor_index` (`1.25 µs` per `15m` step), matches `Market`/`Limit`/`Stop` orders and `SL`/`TP` brackets (`1m == 15m` parity), extracts Order Tickets from `RiskRewardBox` drawings, and rewinds deterministically.
5. **Vietnam Microstructure Risk Planner & Structured Trade Journal (`src/journal/`):** Evaluates `HOSE`/`HNX`/`UPCOM`/`VN30F`/`US`/`Crypto` ticks, daily price bands, `T+2` settlement, and all-in friction sizing (`101.12 µs` per `1,000` plans) alongside `DEF-001` causal `TradeJournalStore` (`SQN`, `Discipline %`, `Cost of Mistakes`, formula-safe RFC-4180 CSV & Markdown export in `601.12 µs` for `500` entries).
6. **Strategy Backtest, Walk-Forward (`WFO`) & Multi-Asset Portfolio (`src/backtest/`):** Runs `100,000`-bar single backtests (`6.318–7.832 ms`), continuous-carry `K`-fold Rolling/Anchored Walk-Forward (`5.958 ms` across `6,667` `15m` bars), and bar-close `VN30` (`106.00 µs`) / `13`-feed `CausalInverseVolatility` (`2.855 ms`) portfolio simulations.
7. **Order-Flow Volume Profile, Session/Anchored VWAP & Causal RS (`src/indicators/`, `src/drawings/`, `src/chart.rs`):** Computes visible/fixed-range `VRVP`/`FRVP` (`14.21 µs` `500`-bar / `803.08 µs` `50k`-bar), rescaled Welford Session/Anchored `VWAP` $\pm 1\sigma, \pm 2\sigma$ (`725.58 µs` / `371.25 µs` at `50k` bars), and `DEF-001` `RS vs VNINDEX` (`651.71 µs` at `50k` bars) + `O(1)` live Rebased Compare scaling.
8. **Synchronized Multi-Pane Layout, Drawing Geometry & `egui::Mesh` Batching (`src/chart.rs`, `src/app.rs`):** Renders `VRVP`/`FRVP`, conflation-split `VWAP`/`AVWAP` ribbons, `RiskRewardBox`, `RelativeStrength`, and `Equity` inside `0.256–3.171 ms` full R11 workstation frames (`< 4.0 ms` `DECISION-RENDERER-002`).

---

## 4. Legacy Compatibility, Offscreen Metal Capture & Portability Rules

- [`ViewportMapping`](../src/model.rs), `generate_synthetic_ohlcv`, `render_candlestick_chart`, `render_workstation_chart`, `render_workstation_chart_with_drawings`, and `render_workstation_chart_full` remain intact with zero breaking changes.
- Automated verification captures (`SUMI_CAPTURE_BATCH=r04|r05|r06|r07|r08|r09|r10|r11` and `SUMI_SCREENSHOT_PATH`) execute offscreen on Apple M1 Metal via `OffscreenWgpuHarness` (`eframe::egui_wgpu::RenderState`, `Rgba8Unorm`, `2.0x` Retina), opening **zero** windows and never touching macOS focus or Dock.
- `src/model.rs`, `src/data/`, `src/viewport/`, `src/indicators/`, `src/drawings/`, `src/replay/`, `src/trading/`, `src/scanner/`, `src/alerts/`, `src/backtest/`, and `src/journal/` use zero OS-specific APIs and pass `cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings`.
