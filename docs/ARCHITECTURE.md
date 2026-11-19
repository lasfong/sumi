# SUMI TRADING WORKSTATION — ARCHITECTURE (`MILESTONE R19 — 100% CORE PARITY`)

- **Updated By:** `SESSION-R19-20261119T145429+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Crate Name:** `sumi-workstation` (`v0.1.0`, Rust Edition 2024, `egui`/`eframe 0.36.2`, `wgpu 30.0.1`)

---

## 1. Crate & Directory Structure

The workstation uses a single-crate library + binary architecture (`src/lib.rs` + `src/main.rs`) with strict module boundaries separating pure `f64`/`i64`/`u64` domain, CSV/JSON data ingestion + `DataQualityAuditReport` / `CafeF SolieuGD` 4-decimal post-split normalization, multi-timeframe aggregation, viewport (multi-chart grid, conflict-free hotkeys, `WorkstationSettings` + `CandlePalettePreset`, templates, snapshot checkpoints & corrupt-state recovery, `SignedPercentage` axis formatting), technical indicator (`14` causal candlestick patterns, `ADX`/`Stochastic`/`SuperTrend`/`PSAR`/`Keltner`/`CCI`/`MFI`/`Volume SMA`, multi-horizon `ProxyBB` `T03..T200`, Order-Flow Volume Profile, Session/Anchored VWAP, Market Profile TPO, Ichimoku, CVD, RS), interactive drawing (`9` tools incl. `TextNote` callouts, per-drawing custom Fibonacci levels, dual-end `TrendLine` extensions & transactional property inspector), causal bar replay, paper trading / VSD `T+2` (`13:00 ICT`) FIFO `HoldingLot` lock / synthetic DOM ladder / live practice coaching (`PracticeScoreboard`, `SessionDebriefModal`, tamper-proof `SL-Adherence`, `PracticeSessionStore` & `ScanSnapshotStore`), multi-symbol watchlist/scanner (`+120b` `BlindPractice` pre-roll)/breadth/RRG, `101`-entry explainable signal catalog (`PrecomputedCatalogMatrix` + confirmed `right_bars = 3` divergences), whole-market `MarketBBAggregator`, & `Point-in-Time` `UniverseResolver`, multi-condition causal alert & formula-safe export, deterministic strategy backtest (`CustomSignalRules`, episodic PIT `run_pit_episodes_backtest`, UTC-aligned `0%` `MultiStrategyComparisonReport`, `AdvancedTearsheetAnalytics`, `RunManifest`) / Walk-Forward / survivorship-bias-free multi-asset portfolio (`run_portfolio_backtest_with_universe`) / `SplitMix64` Monte Carlo bootstrap, and Vietnam market microstructure (`VIETNAM_HOLIDAYS_V1` `151` official `2018–2030` closures) risk planner / trade journal engines (`wasm32-unknown-unknown`-verified via `cargo clippy --lib --target wasm32-unknown-unknown -- -D warnings`, zero new crates in `Cargo.toml`) from `egui` immediate-mode UI and native macOS Metal windowing/offscreen PNG rendering:

```text
/Users/mizuhara/workspace/sumi
├── Cargo.toml
├── src/
│   ├── lib.rs                      # Crate root exporting alerts, app, backtest, chart, data, drawings, indicators, journal, model, replay, scanner, trading, viewport
│   ├── main.rs                     # Native macOS entry point (routes SUMI_CAPTURE_BATCH / SUMI_SCREENSHOT_PATH to offscreen Metal)
│   ├── model.rs                    # Pure OHLCV domain model (Candle with i64 UTC timestamp, PriceBounds, ViewportMapping)
│   ├── data/                       # Milestone R06–R08 & R19 Zero-Dependency OHLCV Loader, Multi-Timeframe & Data Readiness Audit Engine (WASM-safe)
│   │   ├── mod.rs                  # DataLoadOptions (OhlcEnvelopePolicy::CafeFSplitAware), DuplicateTimestampPolicy, SymbolTracker & normalization
│   │   ├── audit.rs                # R19 DataQualityAuditReport, UniverseDataReadinessSummary, VN/NYSE/ISO-W1 session gap detector & 5-rule CafeF post-split gate
│   │   ├── calendar.rs             # Hinnant O(1) proleptic Gregorian UTC calendar, multi-format timestamp parser & adaptive labels
│   │   ├── timeframe.rs            # Timeframe (1m..1W) upward lattice, aggregate_ohlcv & causal DEF-001 aggregate_ohlcv_up_to
│   │   ├── csv_loader.rs           # Streaming RFC-4180 CSV/TSV/semicolon parser, header alias + CafeF 8-col intraday & bounded I/O
│   │   ├── json_loader.rs          # Zero-dependency recursive-descent JSON kline/object/envelope parser (depth <= 16)
│   │   ├── presets.rs              # Bundled VN30F1M 1m, VN30 Basket (8 equities), BTCUSDT 15m JSON, and archived CafeF/SPY CSV loaders
│   │   ├── synthetic.rs            # O(N) deterministic synthetic OHLCV generator (120 .. 100,000+ bars, R03 prefix-preserving)
│   │   └── tests.rs                # Calendar bijection, RFC-4180, JSON klines, CafeF/SPY fixtures, Data Audit, CafeF split & Critic R06/R19 suites
│   ├── viewport/                   # Milestone R03 & R12–R16, R19 Modular Viewport, Multi-Chart Grid, Hotkeys, Settings & Snapshot Recovery Engine (WASM-safe)
│   │   ├── mod.rs                  # Public exports + panic-free lwc_clamp
│   │   ├── time_scale.rs           # Shared horizontal TimeScale (zoom, pan, resize, fit, culling, conflation)
│   │   ├── price_scale.rs          # Per-pane/axis PriceScale (Normal, Log, %, Idx100, CompactVolume K/M/B/T, SignedPercentage, axis_edge_pad_px)
│   │   ├── pane_viewport.rs        # Explicit 3-space bridge (Logical/Data <-> PaneLocal <-> Screen)
│   │   ├── kinetic.rs              # Exponential-decay inertial panning on right_offset (bar units)
│   │   ├── candle_metrics.rs       # Optimal candlestick/wick pixel parity + zero-alloc OHLCV conflation
│   │   ├── multi_chart.rs          # R12 Synchronized Multi-Chart Grid (1x1/1x2/2x1/2x2, Sync X/T/S/TF, DEF-001 MTF cutoffs)
│   │   ├── hotkeys.rs              # R14 Conflict-free 18-action HotkeyProfile engine, base-preset preservation, reserved keys & modal/focus eligibility
│   │   ├── workspace_template.rs   # R12–R19 WorkstationSettings, CandlePalettePreset, WorkspaceTemplate + 16-slot WorkspaceSnapshotStore & quarantine recovery
│   │   └── tests.rs                # LWC v5.2.0 fixtures + 5,100+ property cases + R12–R19 grid, Hotkeys, Settings, Snapshot Recovery & Critic suites
│   ├── indicators/                 # Milestone R04, R11–R13 & R18 Technical Indicator, Pattern, ProxyBB, Order-Flow & TPO Engine (WASM-safe)
│   │   ├── mod.rs                  # Indicator trait, IndicatorCache, SeriesBlockIndex, CandleBlockIndex (16-bar) & conflated envelopes
│   │   ├── moving_averages.rs      # SMA (cancellation-safe reseed) & EMA (Sterbenz weights, Optional slice)
│   │   ├── bands.rs                # Bollinger Bands (20, 2.0 population σ, overflow-safe scaled fallback)
│   │   ├── oscillators.rs          # Wilder RSI (14), MACD (12, 26, 9), Wilder ATR (14) & reference levels
│   │   ├── patterns.rs             # R18 CandleGeometry (scale-relative flat guard), 14 causal candlestick patterns & candle_score [-100, +100]
│   │   ├── extended.rs             # R18 ADX(14) +DI/-DI, O(N) Stochastic(14,3,3), SuperTrend(10,3), PSAR, Keltner, CCI(20), MFI(14), VolSMA(20)
│   │   ├── proxy_bb.rs             # R18 Multi-horizon OHLCV_PROXY Money Flow ProxyBBCalculator (T03, T05, T10, T20, T50, T200, AT05 None)
│   │   ├── volume_profile.rs       # R11 Order-Flow VRVP/FRVP (OHLCV_PROXY buy/sell, CME 70% VA, POC, HVN/LVN, micro-price guard)
│   │   ├── vwap.rs                 # R11–R13 Multi-Interval VWAP (AutoSession/Daily/Weekly/Monthly/Yearly, Welford ±1σ/±2σ, D1/W1 UTC guard)
│   │   ├── market_profile.rs       # R13 Session TPO / Market Profile (POC, 70% VAH/VAL, IB, Single Prints, causal Naked & HistoricalNakedPoc)
│   │   ├── relative_strength.rs    # R11 DEF-001 causal close-aligned RS vs VNINDEX/Peer Benchmark & periodic-recompute RS SMA
│   │   ├── ichimoku.rs             # R12 Ichimoku Cloud (Tenkan 9, Kijun 26, Senkou A/B +26, strict None tail Chikou -26)
│   │   ├── cvd.rs                  # R12 Cumulative Volume Delta (OHLCV_PROXY Cumulative & TZ-aware Session reset + 9-SMA)
│   │   └── tests.rs                # TA-Lib/Wilder, R11 VRVP/VWAP/RS, R12 Ichimoku/CVD, R13 TPO/VWAP, R18 Patterns/Extended/ProxyBB & Critic suites
│   ├── drawings/                   # Milestone R05–R15 & R19 Interactive Drawing, TextNote, Custom Fib & Property Inspector Engine (f64 + UTC + Causal, WASM-safe)
│   │   ├── mod.rs                  # Drawing + 9 geometries (incl. TextNote, custom_fib_levels 1..=16, trend_extend_left/right), DrawingPropertyPatch
│   │   ├── geometry.rs             # PaneDrawingProjection, [-∞, +∞] Liang-Barsky, TextNote word-wrap card, custom Fib OTE/Std/Full, R:R, FRVP & AVWAP
│   │   ├── hit_test.rs             # Causal LWC priority hit-testing (TextNote interior 0-dist + 6px halo, FRVP, HandlePart::Anchor, R:R) & magnet
│   │   ├── controller.rs           # DrawingStore, create_text_note_at, apply_drawing_property_patch, AVWAP clamp, remap_timeframe & Undo/Redo
│   │   └── tests.rs                # Liang-Barsky, Ray, Fib, R:R, FRVP/AVWAP, TextNote, dual-end TrendLine, custom Fib & Critic R05..R19 suites
│   ├── replay/                     # Milestone R07 Causal Bar Replay Engine (f64/i64, WASM-safe)
│   │   ├── mod.rs                  # ReplaySpeed (1x..20x), ReplayStepUnit (ActiveBar, BaseBar), ReplayStatus & ReplayTransition
│   │   ├── controller.rs           # ReplayController, DEF-001 prefix materialization, step/jump/tick state machine & serde safety
│   │   └── tests.rs                # Multi-timeframe DEF-001 lookahead lock, partial bars, causal drawings & Critic R07 suite
│   ├── trading/                    # Milestone R07, R14, R17 & R19 Paper Trading, VSD T+2 Bracket Guard, DOM Ladder & Practice Coaching (WASM-safe)
│   │   ├── mod.rs                  # Public exports for AccountConfig, Order, Position, ClosedTrade, TradingSession, DOM ladder & coaching stores
│   │   ├── orders.rs               # OrderSide, PositionSide, OrderKind (Market/Limit/Stop), FillReason & PortfolioMetrics (incl. sl_adherence_pct)
│   │   ├── portfolio.rs            # TradingSession, immutable had_initial_sl_at_open, enforce_vn_t2_lot_lock SL/TP cap, 4-waypoint OHLC & rewind_to
│   │   ├── ladder.rs               # R14 Causal OHLCV_SYNTH_DOM_PROXY 21-row Price Ladder, tier-crossing VN ticks, band/short guards & click-to-stage
│   │   ├── coaching.rs             # R17–R19 PracticeScoreboardSummary, SessionDebriefReport, DEF-001 PracticeDecisionLog, PracticeSessionStore & ScanSnapshotStore
│   │   └── tests.rs                # Gap-open/dual-touch brackets, 1m==15m parity, SL-adherence, VSD T+2 bracket cap, session stores & Critic suites
│   ├── scanner/                    # Milestone R08, R13, R15–R18 Watchlist, Scanner (+120b BlindPractice), Breadth/RRG, 101-Signal Matrix, MarketBB & PIT Universe
│   │   ├── mod.rs                  # MarketRegime (6-bit bitmask), ScanSignalKind, ScannerSignalContext, Breadth, MarketBB, Universe & PrecomputedCatalogMatrix
│   │   ├── engine.rs               # Single-pass evaluate_symbol_scan_and_signals, INSTITUTIONAL_BLIND_PRACTICE_PRE_ROLL_BARS = 120 & compute_regime_series
│   │   ├── breadth.rs              # R13 Causal VN30/Universe Breadth (AT05/AT09/AT17) & JdK 4-Quadrant RRG trajectories
│   │   ├── catalog.rs              # R15–R18 101-entry SIGNAL_CATALOG (confirmed right_bars=3 divergences + VSA/SR/Pattern/BB/B2), PrecomputedCatalogMatrix & confluence
│   │   ├── proxy_bb.rs             # R18 Intraday-to-D1 venue-aware MarketBBAggregator (AT09 aggregate-before-ratio + AT17 FlowBreadth + publication status)
│   │   ├── universe.rs             # R18 Point-in-Time vs RetrospectiveFixed UniverseResolver, SessionDate, CoverageStatus & JSON/RON loader
│   │   └── tests.rs                # Causal RVol/regime, BlindPractice reveal, Breadth/RRG, 101-entry Catalog, MarketBB, PIT Universe & Critic suites
│   ├── alerts/                     # Milestone R08, R16 & R19 Multi-Condition Causal Alert Ledger & Formula-Safe Export Engine (f64/i64, WASM-safe)
│   │   ├── mod.rs                  # AlertCondition (Price, RSI, MACD, Drawing, SignalCatalogTrigger, ConfluenceThreshold), cooldown_bars & AlertTriggerEvent
│   │   ├── engine.rs               # AlertEngine (synced with dual-end TrendLine extensions), HTF close guard, cooldown, rewind & CSV/JSON export
│   │   └── tests.rs                # 4-waypoint/gap, RSI/MACD, ray/trendline, SignalCatalog/Confluence, HTF session close, CSV/JSON & Critic suites
│   ├── backtest/                   # Milestone R09–R10, R15–R18 Strategy Backtest, Rule Builder, PIT Portfolio, Multi-Strategy 0% Overlay, Analytics & MC
│   │   ├── mod.rs                  # Public exports for BacktestConfig, CustomSignalRuleSpec, MultiStrategyComparisonReport, AdvancedTearsheetAnalytics & RunManifest
│   │   ├── strategy.rs             # 7 presets (incl. 4-leg DNF CustomSignalRules + 12-grid threshold/regime/confluence sweep), sizing, SL/TP/T+2 & sanitize
│   │   ├── metrics.rs              # Institutional BacktestTearsheet (Sharpe, Sortino, Calmar, CAGR, R-Multiple, Exposure %) & RobustnessVerdict
│   │   ├── engine.rs               # Causal Open[T+1] runner, run_pit_episodes_backtest, slice_causal_series_for_symbol_and_options, sweep & battle
│   │   ├── analytics.rs            # R17 UTC-aligned 0% MultiStrategyComparisonReport, OutlierImpact, DrawdownPeriods, Year/Regime slices, MultiPhaseBatch & RunManifest
│   │   ├── walk_forward.rs         # K-fold Rolling/Anchored WFO & bar-close survivorship-bias-free Multi-Asset Portfolio (run_portfolio_backtest_with_universe)
│   │   ├── monte_carlo.rs          # R15 Deterministic SplitMix64 bootstrap, sequential equity returns, VaR/CVaR 95%/99% & bar-aligned fan series
│   │   └── tests.rs                # Next-bar causality, dual-touch, T+2, CustomSignalRules, PIT Portfolio, Multi-Strat 0% UTC, Analytics & Critic suites
│   ├── journal/                    # Milestone R10, R14 & R19 Vietnam Exchange Holiday Calendar, VSD T+2 FIFO Lots, Risk Planner & Trade Journal (WASM-safe)
│   │   ├── mod.rs                  # Public exports for ExchangeVenue, VIETNAM_HOLIDAYS_V1, HoldingLot, FifoSettlementSummary, TradePlanInput & TradeJournalStore
│   │   ├── market_rules.rs         # R19 VIETNAM_HOLIDAYS_V1 ([i32; 151] 2018–2030 + statutory fallback), pre-2022 T+3 vs T+2 13:00 ICT & FIFO HoldingLot ledger
│   │   ├── planner.rs              # All-in commission/slippage/tax_sell sizer, concentration cap, breakeven, net R & statutory warnings
│   │   ├── ledger.rs               # DEF-001 causal TradeJournalStore, Paper rewind sync, SQN/Discipline %, safe RFC-4180 CSV & Markdown
│   │   └── tests.rs                # HOSE tier-crossing bands, VN30F x100k sizing, VIETNAM_HOLIDAYS_V1 T+2/T+3 FIFO lots, CSV guard & Critic suites
│   ├── chart.rs                    # MultiPaneLayout, ExtendedOscillatorScales, CandlePalettePreset (HollowBullish/OkabeIto/Monochrome), TextNote & Custom Fib Mesh
│   └── app.rs                      # SumiWorkstationApp shell, [Settings], [Data Audit], [Export PNG] (OffscreenWgpuHarness), VSD T+2 FIFO Lock & 101-Signal Catalog
├── examples/
│   └── viewport_bench.rs           # Reproducible 10K/50K/100K R03 & --r04..--r19 release benchmarks
└── docs/
    ├── FEATURE_BACKLOG_AND_PARITY_LEDGER.md # Master parity & anti-miss roadmap ledger — 100% Core Parity Complete (S-01..S-10, R16, A1..A5, B1..B4, C1..C4 [x])
    ├── operations/                 # SUMI_ORCHESTRATION_CHARTER.md (durable multi-session orchestration)
    ├── research/                   # REA traces, LWC v5.2.0 & TA-Lib specs, R03–R19 design specifications
    ├── evidence/                   # Offscreen & windowed Metal PNG captures, runtime JSON telemetry, benchmark reports
    └── sessions/                   # Chronological session registry
```

---

## 2. Coordinate Spaces, Timeframe Continuity & Causal Replay Boundaries

Per [`docs/research/RUST_VIEWPORT_CAMERA_DESIGN.md`](research/RUST_VIEWPORT_CAMERA_DESIGN.md), [`docs/research/INDICATOR_AND_MULTIPANE_SPEC.md`](research/INDICATOR_AND_MULTIPANE_SPEC.md), [`docs/research/DRAWING_ENGINE_SPEC.md`](research/DRAWING_ENGINE_SPEC.md), [`docs/research/DATA_LOADER_AND_TIMEFRAME_SPEC.md`](research/DATA_LOADER_AND_TIMEFRAME_SPEC.md), [`docs/research/BAR_REPLAY_AND_PAPER_TRADING_SPEC.md`](research/BAR_REPLAY_AND_PAPER_TRADING_SPEC.md), [`docs/research/SCANNER_ALERTS_AND_COMMAND_PALETTE_SPEC.md`](research/SCANNER_ALERTS_AND_COMMAND_PALETTE_SPEC.md), [`docs/research/STRATEGY_BACKTEST_LAB_SPEC.md`](research/STRATEGY_BACKTEST_LAB_SPEC.md), [`docs/research/RISK_PLANNER_JOURNAL_AND_PORTFOLIO_SPEC.md`](research/RISK_PLANNER_JOURNAL_AND_PORTFOLIO_SPEC.md), [`docs/research/VOLUME_PROFILE_VWAP_AND_REL_STRENGTH_SPEC.md`](research/VOLUME_PROFILE_VWAP_AND_REL_STRENGTH_SPEC.md), [`docs/research/MULTI_CHART_ICHIMOKU_CVD_SPEC.md`](research/MULTI_CHART_ICHIMOKU_CVD_SPEC.md), [`docs/research/MARKET_PROFILE_BREADTH_AND_AXIS_SPEC.md`](research/MARKET_PROFILE_BREADTH_AND_AXIS_SPEC.md), [`docs/research/EXECUTION_LADDER_HOTKEYS_AND_AXIS_POLISH_SPEC.md`](research/EXECUTION_LADDER_HOTKEYS_AND_AXIS_POLISH_SPEC.md), [`docs/research/SIGNALS_INSPECTOR_AND_MONTE_CARLO_SPEC.md`](research/SIGNALS_INSPECTOR_AND_MONTE_CARLO_SPEC.md), [`docs/research/RULE_BUILDER_ALERTS_AND_SNAPSHOT_RECOVERY_SPEC.md`](research/RULE_BUILDER_ALERTS_AND_SNAPSHOT_RECOVERY_SPEC.md), and [`docs/research/PRACTICE_COACHING_AND_MULTISTRATEGY_ANALYTICS_SPEC.md`](research/PRACTICE_COACHING_AND_MULTISTRATEGY_ANALYTICS_SPEC.md), Sumi strictly separates horizontal time state, per-pane vertical price state, UTC + exchange-local civil time, causal replay frontiers, synchronized grid links, and screen layout offsets:

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

1. **Zero-Dependency CSV/JSON Ingestion, 13-Symbol Universe & Data Readiness / `CafeF SolieuGD` Audit (`src/data/`):** Streams CSV/TSV (`103.7–191.1 MiB/s`) and JSON kline/envelope feeds (`134.9 MiB/s`), upward aggregation (`1m..1W`), `DataQualityAuditReport` + 13-feed `UniverseDataReadinessSummary` (`3,557.29 µs` on `10,000` `D1` bars + `13` feeds + `1,000` `CafeF SolieuGD` 4-decimal post-split `OhlcEnvelopePolicy::CafeFSplitAware` evaluations), `VIETNAM_HOLIDAYS_V1` (`VN`) & `is_us_nyse_holiday_ymd` (`NYSE`) session-gap detection, and ISO-week `1W` continuity checks.
2. **Watchlist, Scanner, Causal Breadth/RRG, `101`-Entry Signal Catalog, `MarketBBAggregator` & `Point-in-Time` Universe (`src/scanner/`):** Evaluates `SymbolScanRow` + `HistoricalSignalHit` across all `13` bundled feeds (`20,797` bars) in `1.799 ms`, `AT05`/`AT09`/`AT17` Money Flow & JdK 4-quadrant `RRG` (`858.79 µs`), `101`-entry `SIGNAL_CATALOG` + `PrecomputedCatalogMatrix` (`5,991.96 µs` on `101 x 2,500`), intraday-to-`D1` `MarketBBAggregator` & `UniverseResolver` (`2,096.08 µs`).
3. **Interactive Drawing Engine (`9` Tools incl. `TextNote` & Custom Fibs), Property Inspector & Causal Alert Engine (`src/drawings/`, `src/alerts/`):** Separates `Drawing::is_causally_eligible` from `Drawing::is_causally_visible`, projects `TextNote` word-wrapped callout cards + custom `1..=16` Fibonacci ratios + dual-end `[-∞, +∞]` `TrendLine` extensions (`1.71 µs`), applies transactional `DrawingPropertyPatch` edits (`<= 1` `Modify` on 64-step Undo/Redo), and evaluates `PriceLevel`, `RsiThreshold`, `MacdHistogramZeroCross`, `DrawingLine` (synced with dual-end `TrendLine` extensions), `SignalCatalogTrigger`, and `ConfluenceThreshold` alerts causally with `cooldown_bars` and formula-safe CSV/JSON export.
4. **Causal Replay, Paper Trading, VSD `T+2` (`13:00 ICT`) FIFO Lock, Synthetic DOM Ladder & Practice Coaching (`src/replay/`, `src/trading/`):** Advances `base_cursor_index` (`1.25 µs` per `15m` step), matches `Market`/`Limit`/`Stop` orders and `SL`/`TP` brackets (`1m == 15m` parity, immutable `had_initial_sl_at_open`, `enforce_vn_t2_lot_lock` capping `SL`/`TP` exits to settled shares), computes causal 21-row `OHLCV_SYNTH_DOM_PROXY` ladders, and drives `PracticeScoreboardSummary`, `SessionDebriefReport`, `PracticeDecisionLog`, and `PracticeSessionStore` / `ScanSnapshotStore`.
5. **Vietnam Exchange Holiday Calendar (`VIETNAM_HOLIDAYS_V1` `2018–2030`), FIFO `HoldingLot` Ledger, Risk Planner & Structured Trade Journal (`src/journal/`):** Evaluates `HOSE`/`HNX`/`UPCOM`/`VN30F`/`US`/`Crypto` ticks (`ResolvedVnPriceUnit`), `151` official `2018–2030` Vietnam exchange closures + statutory `< 2018` / `> 2030` fallback, historical pre-`2022-08-29` `T+3` (`09:00 ICT`) vs VSD `T+2` (`13:00 ICT = 06:00 UTC`) afternoon clearance, coarse-bar-aware `compute_fifo_holding_lots_with_bar_secs` (`95.21 µs` across `500` T+2 lookups + `500`-fill FIFO ledger), all-in friction sizing, and `DEF-001` causal `TradeJournalStore`.
6. **Composable Rule Strategy Builder, Backtest, PIT Portfolio, Multi-Strategy `0%` UTC Overlay, Tearsheet Analytics, `WFO` & `SplitMix64` Monte Carlo (`src/backtest/`):** Runs `100,000`-bar single backtests (`6.318–7.832 ms`), `CustomSignalRules` 4-leg DNF `OR-of-AND` strategies, `run_pit_episodes_backtest`, `run_portfolio_backtest_with_universe`, 9-curve `0%` UTC `MultiStrategyComparisonReport`, `AdvancedTearsheetAnalytics` + `RunManifest`, continuous-carry `K`-fold `WFO`, and deterministic `1,000`-simulation `SplitMix64` bootstrap risk envelopes.
7. **Extended Indicators, `14` Candlestick Patterns, Multi-Horizon `ProxyBB` (`T03..T200`), Volume Profile, Session TPO, VWAP & Causal RS (`src/indicators/`, `src/chart.rs`):** Computes `14` causal candlestick patterns (`292.17 µs` at `10k`), `ADX`/`Stochastic`/`SuperTrend`/`PSAR`/`Keltner`/`CCI`/`MFI`/`Volume SMA` (`934.67 µs` at `10k`), 6-horizon `ProxyBBCalculator` (`1,036.88 µs` at `10k`), `VRVP`/`FRVP`, Session TPO / Market Profile, Welford `VWAP`, and `DEF-001` `RS vs VNINDEX`.
8. **Synchronized Multi-Chart Grid, Global `WorkstationSettings`, `CandlePalettePreset`, Snapshot Recovery & `[Export PNG]` (`src/viewport/`, `src/chart.rs`, `src/app.rs`):** Manages `1x1`/`1x2`/`2x1`/`2x2` synchronized slots, `WorkstationSettings` + `4` `CandlePalettePreset`s (`EmeraldCrimson`, `HollowBullish`, `ColorblindOkabeIto`, `ClassicMonochrome`, `189.58 µs` RON/JSON roundtrip), `16`-slot `WorkspaceSnapshotStore` + quarantine recovery, 1-click `[Export PNG]` (`OffscreenWgpuHarness` `2.0x` Retina), and right-axis anti-collision culling, holding all `--r19` UI workloads at **`0.253–0.372 ms` mean (`p99 <= 0.592 ms`)** (`< 4.0 ms` `DECISION-RENDERER-002`).

---

## 4. Legacy Compatibility, Offscreen Metal Capture & Portability Rules

- [`ViewportMapping`](../src/model.rs), `generate_synthetic_ohlcv`, `render_candlestick_chart`, `render_workstation_chart`, `render_workstation_chart_with_drawings`, and `render_workstation_chart_full` remain intact with zero breaking changes.
- Automated verification captures (`SUMI_CAPTURE_BATCH=r04..r19`, `SUMI_SCREENSHOT_PATH`, and interactive 1-click `[Export PNG]`) execute offscreen on Apple M1 Metal via `OffscreenWgpuHarness` (`eframe::egui_wgpu::RenderState`, `Rgba8Unorm`, `2.0x` Retina), opening **zero** extra windows and never touching macOS focus or Dock.
- `src/model.rs`, `src/data/`, `src/viewport/`, `src/indicators/`, `src/drawings/`, `src/replay/`, `src/trading/`, `src/scanner/`, `src/alerts/`, `src/backtest/`, and `src/journal/` use zero OS-specific APIs and pass `cargo clippy --lib --target wasm32-unknown-unknown -- -D warnings`.
