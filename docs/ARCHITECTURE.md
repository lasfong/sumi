# SUMI TRADING WORKSTATION — ARCHITECTURE (`MILESTONE R16`)

- **Updated By:** `SESSION-R16-20261118T155000+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Crate Name:** `sumi-workstation` (`v0.1.0`, Rust Edition 2024, `egui`/`eframe 0.36.2`, `wgpu 30.0.1`)

---

## 1. Crate & Directory Structure

The workstation uses a single-crate library + binary architecture (`src/lib.rs` + `src/main.rs`) with strict module boundaries separating pure `f64`/`i64`/`u64` domain, CSV/JSON data ingestion, multi-timeframe aggregation, viewport (multi-chart grid, conflict-free hotkeys, templates, snapshot checkpoints & corrupt-state recovery), technical indicator (Order-Flow Volume Profile, Session/Anchored VWAP, Market Profile TPO, Ichimoku, CVD, RS), interactive drawing & transactional property inspector, causal bar replay, paper trading & synthetic DOM ladder execution, multi-symbol watchlist/scanner/breadth/RRG & 17-entry explainable signal catalog (`PrecomputedCatalogMatrix`), multi-condition causal alert & formula-safe export, deterministic strategy backtest (`CustomSignalRules` 4-leg composable builder) / Walk-Forward / multi-asset portfolio / `SplitMix64` Monte Carlo bootstrap, and Vietnam market microstructure risk planner / trade journal engines (`wasm32-unknown-unknown`-verified via `cargo clippy --lib --target wasm32-unknown-unknown -- -D warnings`, zero new crates in `Cargo.toml`) from `egui` immediate-mode UI and native macOS Metal windowing/offscreen rendering:

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
│   ├── viewport/                   # Milestone R03 & R12–R16 Modular Viewport, Multi-Chart Grid, Hotkeys & Snapshot Recovery Engine (f64, WASM-safe)
│   │   ├── mod.rs                  # Public exports + panic-free lwc_clamp
│   │   ├── time_scale.rs           # Shared horizontal TimeScale (zoom, pan, resize, fit, culling, conflation)
│   │   ├── price_scale.rs          # Per-pane/axis PriceScale (Normal, Log, %, Idx100, CompactVolume K/M/B/T, axis_edge_pad_px, scale_decimals_for_range_and_step, format_price_for_tick_density)
│   │   ├── pane_viewport.rs        # Explicit 3-space bridge (Logical/Data <-> PaneLocal <-> Screen)
│   │   ├── kinetic.rs              # Exponential-decay inertial panning on right_offset (bar units)
│   │   ├── candle_metrics.rs       # Optimal candlestick/wick pixel parity + zero-alloc OHLCV conflation
│   │   ├── multi_chart.rs          # R12 Synchronized Multi-Chart Grid (1x1/1x2/2x1/2x2, Sync X/T/S/TF, DEF-001 MTF cutoffs)
│   │   ├── hotkeys.rs              # R14 Conflict-free 18-action HotkeyProfile engine, base-preset preservation, reserved keys & modal/focus eligibility
│   │   ├── workspace_template.rs   # R12–R16 WorkspaceTemplate + 16-slot WorkspaceSnapshotStore, 4-slot auto-ring, 64-bit digest verify, diff & quarantine recovery
│   │   └── tests.rs                # LWC v5.2.0 fixtures + 5,100+ property cases + R12–R16 grid, CompactVolume, Hotkeys, Snapshot Recovery & Critic suites
│   ├── indicators/                 # Milestone R04, R11–R13 Technical Indicator, Order-Flow & Market Profile Engine (f64, WASM-safe)
│   │   ├── mod.rs                  # Indicator trait, IndicatorCache, SeriesBlockIndex, CandleBlockIndex (16-bar) & conflated envelopes
│   │   ├── moving_averages.rs      # SMA (cancellation-safe reseed) & EMA (Sterbenz weights, Optional slice)
│   │   ├── bands.rs                # Bollinger Bands (20, 2.0 population σ, overflow-safe scaled fallback)
│   │   ├── oscillators.rs          # Wilder RSI (14), MACD (12, 26, 9), Wilder ATR (14) & reference levels
│   │   ├── volume_profile.rs       # R11 Order-Flow VRVP/FRVP (OHLCV_PROXY buy/sell, CME 70% VA, POC, HVN/LVN, micro-price guard)
│   │   ├── vwap.rs                 # R11–R13 Multi-Interval VWAP (AutoSession/Daily/Weekly/Monthly/Yearly, Welford ±1σ/±2σ, D1/W1 UTC guard)
│   │   ├── market_profile.rs       # R13 Session TPO / Market Profile (POC, 70% VAH/VAL, IB, Single Prints, causal Naked & HistoricalNakedPoc)
│   │   ├── relative_strength.rs    # R11 DEF-001 causal close-aligned RS vs VNINDEX/Peer Benchmark & periodic-recompute RS SMA
│   │   ├── ichimoku.rs             # R12 Ichimoku Cloud (Tenkan 9, Kijun 26, Senkou A/B +26, strict None tail Chikou -26)
│   │   ├── cvd.rs                  # R12 Cumulative Volume Delta (OHLCV_PROXY Cumulative & TZ-aware Session reset + 9-SMA)
│   │   └── tests.rs                # TA-Lib/Wilder, R11 VRVP/VWAP/RS, R12 Ichimoku/CVD, R13 TPO/VWAP & Critic suites
│   ├── drawings/                   # Milestone R05–R15 Interactive Drawing, Annotation & Property Inspector Engine (f64 + UTC + Causal, WASM-safe)
│   │   ├── mod.rs                  # Drawing + 8 geometries, is_causally_eligible vs is_causally_visible, DrawingPropertyPatch & validation
│   │   ├── geometry.rs             # PaneDrawingProjection (with_candles), ClipEdge, Fib, RiskRewardBox, FixedRangeVolumeProfile & AVWAP tail
│   │   ├── hit_test.rs             # Causal LWC priority hit-testing (FRVP borders, HandlePart::Anchor + tail_segments, R:R handles) & magnet
│   │   ├── controller.rs           # DrawingStore, apply_drawing_property_patch, snapshot handles, AVWAP clamp, remap_timeframe & Undo/Redo
│   │   └── tests.rs                # Liang-Barsky, Ray, Fib, R:R, FRVP/AVWAP, Inspector patch/undo & Critic R05/R10/R11/R15 suites
│   ├── replay/                     # Milestone R07 Causal Bar Replay Engine (f64/i64, WASM-safe)
│   │   ├── mod.rs                  # ReplaySpeed (1x..20x), ReplayStepUnit (ActiveBar, BaseBar), ReplayStatus & ReplayTransition
│   │   ├── controller.rs           # ReplayController, DEF-001 prefix materialization, step/jump/tick state machine & serde safety
│   │   └── tests.rs                # Multi-timeframe DEF-001 lookahead lock, partial bars, causal drawings & Critic R07 suite
│   ├── trading/                    # Milestone R07 & R14 Paper Trading Execution & Synthetic DOM Ladder Engine (f64/i64, WASM-safe)
│   │   ├── mod.rs                  # Public exports for AccountConfig, Order, Position, ExecutionFill, ClosedTrade, TradingSession & DOM ladder
│   │   ├── orders.rs               # OrderSide, PositionSide, OrderKind (Market/Limit/Stop), FillReason & PortfolioMetrics
│   │   ├── portfolio.rs            # TradingSession, 4-waypoint OHLC matching, fee-conserving ledger, RiskRewardBox ticket extract & rewind_to
│   │   ├── ladder.rs               # R14 Causal OHLCV_SYNTH_DOM_PROXY 21-row Price Ladder, tier-crossing VN ticks, band/short guards & click-to-stage
│   │   └── tests.rs                # Gap-open/dual-touch brackets, 1m==15m parity, fee conservation, rewind & Critic R07/R14 suites
│   ├── scanner/                    # Milestone R08, R13, R15 & R16 Watchlist, Scanner, Causal Breadth/RRG & Precomputed Signal Matrix (WASM-safe)
│   │   ├── mod.rs                  # MarketRegime (6-bit bitmask), ScanSignalKind, ScannerSignalContext, Breadth & PrecomputedCatalogMatrix exports
│   │   ├── engine.rs               # Single-pass evaluate_symbol_scan_and_signals, DEF-001 cutoff & None-last sorting
│   │   ├── breadth.rs              # R13 Causal VN30/Universe Breadth (AT05/AT09/AT17) & JdK 4-Quadrant RRG trajectories
│   │   ├── catalog.rs              # R15–R16 17-entry SIGNAL_CATALOG, PrecomputedCatalogMatrix (O(1) point_at_with_profile), SignalQuality & confluence
│   │   └── tests.rs                # Causal RVol/regime, BlindPractice reveal, AT05/09/17 Breadth/RRG, PrecomputedCatalogMatrix & Critic R08/R13/R15/R16 suites
│   ├── alerts/                     # Milestone R08 & R16 Multi-Condition Causal Alert Ledger & Formula-Safe Export Engine (f64/i64, WASM-safe)
│   │   ├── mod.rs                  # AlertCondition (Price, RSI, MACD, Drawing, SignalCatalogTrigger, ConfluenceThreshold), cooldown_bars & AlertTriggerEvent
│   │   ├── engine.rs               # AlertEngine, exchange-schedule HTF close guard, cooldown suppression, rewind & formula-safe CSV / RFC-8259 JSON export
│   │   └── tests.rs                # 4-waypoint/gap, RSI/MACD, ray/trendline, SignalCatalog/Confluence, HTF session close, CSV/JSON & Critic R08/R16 suites
│   ├── backtest/                   # Milestone R09–R10, R15 & R16 Strategy Backtest, Composable Rule Builder, WFO, Portfolio & Monte Carlo (WASM-safe)
│   │   ├── mod.rs                  # Public exports for BacktestConfig, CustomSignalRuleSpec, BacktestRunResult, Sweep, Battle, WFO, Portfolio & MC reports
│   │   ├── strategy.rs             # 7 presets (incl. 4-leg DNF CustomSignalRules + 12-grid threshold/regime/confluence sweep), sizing, SL/TP/T+2 & sanitize
│   │   ├── metrics.rs              # Institutional BacktestTearsheet (Sharpe, Sortino, Calmar, CAGR, R-Multiple, Exposure %) & RobustnessVerdict
│   │   ├── engine.rs               # Causal Open[T+1] runner, exchange-schedule HTF tail guard, 4-leg exit isolation, MarginLiquidation, sweep & battle
│   │   ├── walk_forward.rs         # K-fold Rolling/Anchored WFO (continuous OOS carry + Pardo WFE) & bar-close Multi-Asset Portfolio
│   │   ├── monte_carlo.rs          # R15 Deterministic SplitMix64 bootstrap, sequential equity returns, VaR/CVaR 95%/99% & bar-aligned fan series
│   │   └── tests.rs                # Next-bar causality, dual-touch, T+2, bankruptcy, CustomSignalRules, SPY/VN30F HTF guards & Critic R09/R10/R15/R16 suites
│   ├── journal/                    # Milestone R10 & R14 Vietnam Market Microstructure Risk Planner & Structured Trade Journal (f64/i64, WASM-safe)
│   │   ├── mod.rs                  # Public exports for ExchangeVenue, ResolvedVnPriceUnit, DailyPriceBand, TradePlanInput/Computed & TradeJournalStore
│   │   ├── market_rules.rs         # HOSE/HNX/UPCOM/VN30F/VNINDEX/US/Crypto ticks, resolved units, ±7%/±10%/±15% bands, x100k mult, 17% margin & TZ T+2
│   │   ├── planner.rs              # All-in commission/slippage/tax_sell sizer, concentration cap, breakeven, net R & statutory warnings
│   │   ├── ledger.rs               # DEF-001 causal TradeJournalStore, Paper rewind sync, SQN/Discipline %, safe RFC-4180 CSV & Markdown
│   │   └── tests.rs                # HOSE tier-crossing bands, VN30F x100k sizing, local-TZ T+2, CSV formula guard & Critic R10 suite
│   ├── chart.rs                    # MultiPaneLayout, Market Profile/Kumo/CVD/VWAP/MC Fan Mesh, stacked alert markers & append_alert_axis_pill_ys tick occlusion
│   └── app.rs                      # SumiWorkstationApp shell, Multi-Chart Sync, RuleBuilder, Alerts + Export, Snapshot Checkpoints + Diff/Quarantine & DOM Ladder
├── examples/
│   └── viewport_bench.rs           # Reproducible 10K/50K/100K R03 & --r04..--r16 release benchmarks
└── docs/
    ├── FEATURE_BACKLOG_AND_PARITY_LEDGER.md # Master parity & post-R16 roadmap ledger (S-01..S-10, R16-1..R16-3, R17..R19)
    ├── operations/                 # SUMI_ORCHESTRATION_CHARTER.md (durable multi-session orchestration)
    ├── research/                   # REA traces, LWC v5.2.0 & TA-Lib specs, R03–R16 design specifications
    ├── evidence/                   # Offscreen & windowed Metal PNG captures, runtime JSON telemetry, benchmark reports
    └── sessions/                   # Chronological session registry
```

---

## 2. Coordinate Spaces, Timeframe Continuity & Causal Replay Boundaries

Per [`docs/research/RUST_VIEWPORT_CAMERA_DESIGN.md`](research/RUST_VIEWPORT_CAMERA_DESIGN.md), [`docs/research/INDICATOR_AND_MULTIPANE_SPEC.md`](research/INDICATOR_AND_MULTIPANE_SPEC.md), [`docs/research/DRAWING_ENGINE_SPEC.md`](research/DRAWING_ENGINE_SPEC.md), [`docs/research/DATA_LOADER_AND_TIMEFRAME_SPEC.md`](research/DATA_LOADER_AND_TIMEFRAME_SPEC.md), [`docs/research/BAR_REPLAY_AND_PAPER_TRADING_SPEC.md`](research/BAR_REPLAY_AND_PAPER_TRADING_SPEC.md), [`docs/research/SCANNER_ALERTS_AND_COMMAND_PALETTE_SPEC.md`](research/SCANNER_ALERTS_AND_COMMAND_PALETTE_SPEC.md), [`docs/research/STRATEGY_BACKTEST_LAB_SPEC.md`](research/STRATEGY_BACKTEST_LAB_SPEC.md), [`docs/research/RISK_PLANNER_JOURNAL_AND_PORTFOLIO_SPEC.md`](research/RISK_PLANNER_JOURNAL_AND_PORTFOLIO_SPEC.md), [`docs/research/VOLUME_PROFILE_VWAP_AND_REL_STRENGTH_SPEC.md`](research/VOLUME_PROFILE_VWAP_AND_REL_STRENGTH_SPEC.md), [`docs/research/MULTI_CHART_ICHIMOKU_CVD_SPEC.md`](research/MULTI_CHART_ICHIMOKU_CVD_SPEC.md), [`docs/research/MARKET_PROFILE_BREADTH_AND_AXIS_SPEC.md`](research/MARKET_PROFILE_BREADTH_AND_AXIS_SPEC.md), [`docs/research/EXECUTION_LADDER_HOTKEYS_AND_AXIS_POLISH_SPEC.md`](research/EXECUTION_LADDER_HOTKEYS_AND_AXIS_POLISH_SPEC.md), [`docs/research/SIGNALS_INSPECTOR_AND_MONTE_CARLO_SPEC.md`](research/SIGNALS_INSPECTOR_AND_MONTE_CARLO_SPEC.md), and [`docs/research/RULE_BUILDER_ALERTS_AND_SNAPSHOT_RECOVERY_SPEC.md`](research/RULE_BUILDER_ALERTS_AND_SNAPSHOT_RECOVERY_SPEC.md), Sumi strictly separates horizontal time state, per-pane vertical price state, UTC + exchange-local civil time, causal replay frontiers, synchronized grid links, and screen layout offsets:

1. **Logical / Data, Canonical UTC & Causal Cutoff Coordinates (`f64` / `i64`):**
   - `bar_index` (`0..total_bars` for active `Candle` slices, aligned `Vec<Option<f64>>` indicator/VWAP/AVWAP/Ichimoku/CVD/RS/Compare outputs, `PrecomputedCatalogMatrix` (`17 x N`), and bar-aligned `BacktestEquityPoint` / `stitched_oos_curve` / `MonteCarloFanSeries` curves; continuous `f64` `DrawingAnchor::bar_index` across historical `< 0` and future `>= total_bars` whitespace).
   - `Candle::timestamp: i64` and `DrawingAnchor::timestamp: Option<f64>`, mapped bijectively via Hinnant O(1) calendar math (`src/data/calendar.rs`) across `Timeframe` (`1m`, `5m`, `15m`, `1h`, `4h`, `1D`, `1W`), with exchange-local civil timezone offsets (`UTC+7` ICT / `UTC-5` EST) for statutory `T+2`/`T+1` settlement, prior-session `REF`/`CEIL`/`FLOOR` ladder bands, intraday Session VWAP / Session CVD resets (`1D`/`1W` MTD/YTD resets use UTC civil dates), and exchange-schedule-aware HTF completion checks (`is_bucket_incomplete_at_close_for_symbol`, recognizing Vietnam `04:29:59`/`07:44:59` UTC closes and `D1 -> W1` `last_base_open_ts` Friday closes on non-midnight feeds like `SPY 14:30 UTC`).
   - When `ReplayController` is active at `base_cursor_index`, `materialize_candles` slices `&base_candles[..=base_cursor_index]` before rolling up into `active_timeframe`, and `replay_causal_close_timestamp` (`cursor_open_ts + base_tf.seconds() - 1`) gates `scanner/` (`evaluate_causal_confluence`, `explain_historical_signal`, `revealed_explanation`), `AlertEngine::evaluate_causal` + `export_triggers_{csv,json}`, `TradeJournalStore::visible_entries`, `compute_causal_dom_ladder` (`OHLCV_SYNTH_DOM_PROXY`), `compute_relative_strength` + Rebased Compare overlays, synchronized Multi-Chart MTF [`SecondaryChartSlot`](../src/app.rs) panes (`causal_closed_prefix_len`), `run_backtest_with_options`, `run_walk_forward_validation`, `run_portfolio_backtest`, and `run_monte_carlo_bootstrap` / `run_monte_carlo_portfolio_mapped` so zero future or unclosed coarse-timeframe bars ever leak (`DEF-001`).
2. **Pane-Local Coordinates (`f64`):**
   - `local_x ∈ [0.0, plot_width)` owned by [`TimeScale`](../src/viewport/time_scale.rs) — **shared** across `Main` and all active sub-panes (`RSI`, `MACD`, `ATR`, `RelativeStrength`, `Cvd`, `Equity`), synchronized across Multi-Chart slots by UTC timestamp via `Sync T` (`SyncedTimeWindow`) / crosshair `Sync X` (`resolve_linked_crosshair_bar`) without false whitespace pinning, using unified `conflation_factor(dpr)` and `overlay_conflation_step`.
   - `local_y ∈ [0.0, pane_plot_height)` owned by an **independent** [`PriceScale`](../src/viewport/price_scale.rs) per pane/axis and per chart slot (`main_price_scale`, `volume_price_scale`, `rsi_price_scale`, `macd_price_scale`, `atr_price_scale`, `rs_price_scale`, `cvd_price_scale`, `equity_price_scale`), formatted uniformly with `axis_edge_pad_px` clearance and scale-step-locked decimals (`scale_decimals_for_range_and_step` + `format_price_for_tick_density`, passing `vp.base_value` on Equity baseline badges).
3. **Screen Coordinates (`f64` → `egui::Pos2` `f32` at paint boundary):**
   - Partitioned across `MultiChartLayoutKind` grid cells (`1x1`, `1x2`, `2x1`, `2x2` over primary + `SecondaryChartSlot`s) and vertically inside each slot by [`MultiPaneLayout`](../src/chart.rs) (`stretch_factors` `4.0` main : `1.0` default per oscillator including `rs_stretch` & `cvd_stretch` : `1.25` default `equity_stretch`, `MultiPaneLayout::MIN_PANE_HEIGHT = 44.0`, `5.0 px` interactive splitters) and bridged per pane via [`PaneViewport`](../src/viewport/pane_viewport.rs).

---

## 3. Domain & Rendering Subsystem Pipeline

1. **Zero-Dependency CSV/JSON Ingestion & 13-Symbol Universe (`src/data/`):** Streams CSV/TSV (`103.7–191.1 MiB/s`) and JSON kline/envelope feeds (`134.9 MiB/s`) with multi-symbol discovery, `Vn30Basket`, tail-coverage-scored benchmark lookup, and upward aggregation (`1m..1W`).
2. **Watchlist, Scanner, Causal Breadth/RRG & Precomputed 17-Entry Signal Catalog (`src/scanner/`):** Evaluates `SymbolScanRow` + `HistoricalSignalHit` across all `13` bundled feeds (`20,797` bars) in `1.799 ms`, `AT05`/`AT09`/`AT17` Money Flow & JdK 4-quadrant `RRG` (`858.79 µs`), and the `17`-entry `SIGNAL_CATALOG` + `PrecomputedCatalogMatrix` (`O(N)` vectorized build + `O(1)` `point_at_with_profile`, `OHLCV_BAR_CLOSE_PROXY`, `SignalQuality`, `explain_historical_signal` & `revealed_explanation`), rendered via virtualized `ScrollArea::show_rows` and the `12`-tab Signal Catalog Modal.
3. **Interactive Drawing Engine, Property Inspector & Multi-Condition Causal Alert Engine (`src/drawings/`, `src/alerts/`):** Separates `Drawing::is_causally_eligible` from `Drawing::is_causally_visible`, applies transactional `DrawingPropertyPatch` edits (`<= 1` `Modify` on 64-step Undo/Redo, `Escape` dirty-draft cancel), and evaluates `PriceLevel`, `RsiThreshold`, `MacdHistogramZeroCross`, `DrawingLine`, `SignalCatalogTrigger`, and `ConfluenceThreshold` alerts causally with `cooldown_bars`, bounded exchange-schedule HTF close suppression, and formula-safe RFC-4180 CSV + RFC-8259 JSON export (`71.08 µs` across `180` `FPT 1D` bars).
4. **Causal Replay, Paper Trading & Synthetic DOM Execution Ladder (`src/replay/`, `src/trading/`):** Advances `base_cursor_index` (`1.25 µs` per `15m` step), matches `Market`/`Limit`/`Stop` orders and `SL`/`TP` brackets (`1m == 15m` parity), computes causal 21-row `OHLCV_SYNTH_DOM_PROXY` footprint/depth ladders (`277.12 µs` across `50k` `VN30F1M` bars) with dynamic `HOSE`/`HNX`/`UPCOM`/`VN30F` ticks, `CEIL`/`REF`/`FLOOR` bands, 1-click staging, working-order row cancellation, and deterministic `rewind_to`.
5. **Vietnam Microstructure Risk Planner & Structured Trade Journal (`src/journal/`):** Evaluates `HOSE`/`HNX`/`UPCOM`/`VN30F`/`US`/`Crypto` ticks (`ResolvedVnPriceUnit`), daily price bands, `T+2` settlement, and all-in friction sizing (`101.12 µs` per `1,000` plans) alongside `DEF-001` causal `TradeJournalStore` (`SQN`, `Discipline %`, `Cost of Mistakes`, formula-safe RFC-4180 CSV & Markdown export).
6. **Composable Rule Strategy Builder, Backtest, `WFO`, Portfolio & `SplitMix64` Monte Carlo (`src/backtest/`):** Runs `100,000`-bar single backtests (`6.318–7.832 ms`), `CustomSignalRules` 4-leg DNF `OR-of-AND` strategies (`784.12 µs` on `1,344` `BTCUSDT 15m` bars with `PrecomputedCatalogMatrix`, strict 4-leg exit isolation, 6-bit `MarketRegime` mask, confluence gate & 12-grid sweep), continuous-carry `K`-fold `WFO`, `13`-feed `CausalInverseVolatility` portfolios, and deterministic `1,000`-simulation `SplitMix64` bootstrap risk envelopes (`VaR`/`CVaR` `95%`/`99%`, `MonteCarloFanSeries`).
7. **Order-Flow Volume Profile, Session TPO / Market Profile, Multi-Anchor VWAP & Causal RS (`src/indicators/`, `src/drawings/`, `src/chart.rs`):** Computes visible/fixed-range `VRVP`/`FRVP` (`14.21 µs` `500`-bar / `803.08 µs` `50k`-bar), Session TPO / Market Profile (`POC`, CME 70% `VAH`/`VAL`, `IB`, `Single Prints`, `Naked POC` + `HistoricalNakedPoc`, `331.54 µs` across `50,000` raw `15m` bars), rescaled Welford Session/Anchored `VWAP` (`VwapAnchorMode`, `785.75 µs` Weekly at `50k` bars), and `DEF-001` `RS vs VNINDEX` (`651.71 µs` at `50k` bars).
8. **Synchronized Multi-Chart Grid, Snapshot Recovery & Polished Right Axis (`src/viewport/`, `src/chart.rs`, `src/app.rs`):** Manages `1x1`/`1x2`/`2x1`/`2x2` synchronized slots, `16`-slot `WorkspaceSnapshotStore` + `4`-slot auto-checkpoint ring + 64-bit hex digest verification + pre-restore diff + corrupt-state quarantine recovery (`822.08 µs`), and `collect_trading_axis_pill_ys` + `append_alert_axis_pill_ys` -> `main_occluding_ys` right-axis tick culling, holding `--r16` `Quad 2x2` `200,000`-bar `64x` conflated stress at **`1.218 ms` mean (`p99 = 1.287 ms`)** and all `--r16` UI workloads at **`0.322–1.332 ms` mean (`p99 <= 1.501 ms`)** (`< 4.0 ms` `DECISION-RENDERER-002`).

---

## 4. Legacy Compatibility, Offscreen Metal Capture & Portability Rules

- [`ViewportMapping`](../src/model.rs), `generate_synthetic_ohlcv`, `render_candlestick_chart`, `render_workstation_chart`, `render_workstation_chart_with_drawings`, and `render_workstation_chart_full` remain intact with zero breaking changes.
- Automated verification captures (`SUMI_CAPTURE_BATCH=r04..r16` and `SUMI_SCREENSHOT_PATH`) execute offscreen on Apple M1 Metal via `OffscreenWgpuHarness` (`eframe::egui_wgpu::RenderState`, `Rgba8Unorm`, `2.0x` Retina), opening **zero** windows and never touching macOS focus or Dock.
- `src/model.rs`, `src/data/`, `src/viewport/`, `src/indicators/`, `src/drawings/`, `src/replay/`, `src/trading/`, `src/scanner/`, `src/alerts/`, `src/backtest/`, and `src/journal/` use zero OS-specific APIs and pass `cargo clippy --lib --target wasm32-unknown-unknown -- -D warnings`.
