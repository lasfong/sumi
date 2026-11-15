# SUMI TRADING WORKSTATION — DECISION LOG

- **Session Identity (Created):** `SESSION-01-20261110T160909+0700`
- **Updated By:** `SESSION-R13-20261115T163000+0700` (`SUMI PROGRAM ORCHESTRATOR`)

## Decisions Recorded in Session 01

### DEC-001 — Archive Legacy Webapp & Reuse Current Repository Root
- **Date:** `2026-11-10`
- **Context:** The repository previously held a Python/TypeScript web application experiment. The Product Owner requested archiving all legacy files into `ARCHIVE/` and building the new native macOS Rust Trading Workstation in the repository root.
- **Decision:** Move all legacy project files into `/Users/mizuhara/workspace/sumi/ARCHIVE/` (preserving `ARCHIVE/.gitignore` and all untracked local files) and initialize the Rust crate directly at `/Users/mizuhara/workspace/sumi/Cargo.toml`. Do not modify or delete `ARCHIVE/`.

### DEC-002 — Official Non-Pipe Rust Toolchain Bootstrap (`brew install rustup`)
- **Date:** `2026-11-10`
- **Context:** Phase A diagnostics showed `rustc`, `cargo`, and `rustup` were not installed, while Xcode Command Line Tools (`/Library/Developer/CommandLineTools`) and Homebrew (`/opt/homebrew/bin/brew`) were present. Security rules prohibit `curl | sh` installers and require Product Owner approval before installing toolchains.
- **Decision:** With explicit Product Owner approval, installed `rustup 1.29.3` via Homebrew bottle (`brew install rustup`) and initialized the `stable-aarch64-apple-darwin` toolchain (`rustc 1.99.1`, `cargo 1.99.1`).

### DEC-003 — Upgrade Host Node.js to `26.13.0_2` for Official REA `6.8.0` Compatibility
- **Date:** `2026-11-10`
- **Context:** Official `rea-agents@6.8.0` requires Node.js `^22.23.4 || ^24.22.1 || >=26.12.1`. Host Node.js was `v26.0.0`, which is unsupported by REA's runtime validator.
- **Decision:** With explicit Product Owner approval, upgraded Homebrew `node` from `26.0.0` to `26.13.0_2`, installed `rea-agents@6.8.0` globally from the official npm registry, ran a dry-run first, and scoped setup exclusively to `--client antigravity` without installing optional native decompilers (Hopper/Ghidra/IDA) or altering macOS TCC permissions.

### DEC-004 — Custom `egui::Painter` Candlestick Renderer Over Heavy Third-Party Chart Crates
- **Date:** `2026-11-10`
- **Context:** Financial charting workstations require exact control over price-to-screen and index-to-screen coordinate mapping, axis gutter layout, candle wick/body crispness, and crosshair HUD overlays.
- **Decision:** Built a clean, deterministic coordinate mapper (`ViewportMapping` in `src/model.rs`) and custom `egui::Painter` renderer (`src/chart.rs`) directly on `egui 0.36.0` and `eframe 0.36.0`. Domain logic in `src/model.rs` and `src/data.rs` has zero OS-specific dependencies to guarantee future `wasm32-unknown-unknown` compatibility.


---

## Decisions Recorded in Session R01

### DEC-005 — REA-First Research Gate Before Rust Implementation
- **Date:** `2026-11-10`
- **Session:** `SESSION-R01-20261110T202837+0700`
- **Decision:** No new Rust features (zoom, pan, indicators, drawings, replay) are implemented until the relevant research gate is cleared via REA analysis and documented algorithm specs. The wave-based implementation roadmap (RESEARCH_TO_RUST_ROADMAP.md) governs implementation order.

### DEC-006 — TradingView Desktop Analysis Not Authorized
- **Date:** `2026-11-10`
- **Session:** `SESSION-R01-20261110T202837+0700`
- **Decision:** No binary decompilation, resource extraction, API interception, or proprietary code analysis of TradingView Desktop will be performed in this project. The blocker is permanently recorded. LWC v5 (Apache 2.0) and open-source TA-Lib are the authorized reference implementations.

---

## Decisions Recorded in Session R02

### DECISION-RESEARCH-001 — LWC v5.2.0 as Authorized Coordinate Reference
- **Date:** `2026-11-10`
- **Session:** `SESSION-R02-20261110T230337+0700`
- **Context:** Need an authoritative, legally usable reference for professional chart coordinate and viewport algorithms. The archived Sumi TypeScript application delegates all coordinate math to LWC internally — it is not an independent reference.
- **Decision:** Pin TradingView `lightweight-charts` v5.2.0 (Apache 2.0, commit `868cae2`, tag `v5.2.0`) as the authorized coordinate algorithm reference. Research clone at `research_repos/lightweight-charts-5.2.0`. This is a research-only asset; no LWC code will be copied into production `src/`.

### DECISION-CAMERA-001 — Replace ViewportMapping with ViewportCamera
- **Date:** `2026-11-10`
- **Session:** `SESSION-R02-20261110T230337+0700`
- **Context:** Research confirmed `ViewportMapping` is a stateless f32 prototype that cannot support: continuous scroll offset (`rightOffset`), cursor-anchored zoom (`baseIndex` + `barSpacing`), future-space at the right edge, kinetic scroll deceleration, or proper price margins. Six defect categories documented in `LEGACY_SUMI_COORDINATE_COMPARISON.md`.
- **Decision:** Implement `ViewportCamera` in `src/viewport.rs` (new module) as the replacement. All coordinate state must use f64. All 11 algorithms in `COORDINATE_ALGORITHM_SPEC.md` must be verified by property-based tests and numerical fixtures before `ViewportMapping` is deleted.

### DECISION-RENDERER-001 — Retain egui::Painter as Primary Rendering Backend
- **Date:** `2026-11-10`
- **Session:** `SESSION-R02-20261110T230337+0700`
- **Context:** Evaluated egui::Painter, raw wgpu, Skia, and femtovg for the chart rendering backend.
- **Decision:** Retain `egui::Painter` backed by `wgpu` (Metal on macOS). Rationale: already integrated, GPU-accelerated, adequate for 10k+ bars with clipping/conflation, fully capable for 2D drawing tools. Deferred: custom wgpu pipeline — revisit only if CPU tessellation exceeds 3ms budget at target bar count.

---

## Decisions Recorded in Session R03

### DEC-R03-001 — Modular `src/viewport/` Architecture Over Monolithic `ViewportCamera` (Revises `DECISION-CAMERA-001`)
- **Date:** `2026-11-11`
- **Session:** `SESSION-R03-20261111T105832+0700`
- **Context:** Auditing `RUST_VIEWPORT_CAMERA_DESIGN.md` against Lightweight Charts v5.2.0 (`src/model/chart-model.ts`, `pane.ts`, `time-scale.ts`, `price-scale.ts`) revealed that R02's proposed `ViewportCamera` collapsed shared horizontal time state, per-pane/axis vertical price state, and screen offsets into a single struct — which breaks multi-pane synchronization, dual/overlay price axes, and non-zero screen-origin panes. In addition, Product Owner instructions strictly forbid deleting existing repository artifacts or code (`ViewportMapping`).
- **Decision:** Implement the viewport engine as a modular `f64` subsystem in `src/viewport/` (`TimeScale` shared across panes, independent `PriceScale` per pane/axis, explicit 3-space `PaneViewport` bridge, `KineticAnimation` in bar units, and `candle_metrics` with zero-allocation OHLCV conflation). Retain Session 01's `ViewportMapping` in `src/model.rs` with `#[deprecated(...)]` rather than deleting it.

### DEC-R03-002 — Reclassify Session R02 Acceptance to `ACCEPTED WITH SCOPE LIMITS (RECTIFIED IN R03)`
- **Date:** `2026-11-11`
- **Session:** `SESSION-R03-20261111T105832+0700`
- **Context:** Direct source verification against LWC v5.2.0 (`868cae2`) uncovered 16 mathematical, edge-case, and architectural defects in R02's specs (`_coordinateToFloatIndex` precedence bug, half-slot `49.5` vs `50.0` fixture error, `internal_height == 1.0` division by zero, broken negative `to_log` parentheses and missing adaptive `LogFormula`, panicking `f64::clamp` when `min > max`, fabricated `rightBarStaysOnScroll_coefficient`, double-margin `fit_price_range`, wrong `KineticAnimation` units, etc.), plus REA traces in R02 only covered 6 static `src/model/` files without `gui/` or `renderers/` call-flow tracing.
- **Decision:** Record Session R02's pre-audit state honestly as `ACCEPTED WITH SCOPE LIMITS`, rectify all 4 R02 research/design specs in-place with full audit logs, and expand live REA MCP coverage with 18 live R03 operations (`ev_eea4438f...`, `ev_97bce949...`, `ev_632b75be...`, `ev_11ff50a3...`, `ev_60ae4bfa...`, etc.) spanning `gui -> model -> renderers` and the R03 release Mach-O binary.

### DECISION-RENDERER-002 — Provisional Confirmation of `egui::Painter` With Mandatory Conflation & Explicit `wgpu` Triggers (Revises `DECISION-RENDERER-001`)
- **Date:** `2026-11-11`
- **Session:** `SESSION-R03-20261111T105832+0700`
- **Context:** Executed reproducible `--release` benchmarks (`examples/viewport_bench.rs`) across `10,000`, `50,000`, and `100,000` bars on Apple M1 (`2560×1600` Retina). Unconflated full-fit rendering of `50K` (`21.13 ms`) and `100K` (`39.57 ms`, `2.53M` vertices) exceeds the `16.67 ms` 60 FPS budget due to `epaint` CPU tessellation, whereas **with** R03's O(1) culling + DPR-aware power-of-2 conflation, `100K` full-fit renders in **`1.47 ms`** (**26.9× faster**), standard zoom renders in **`0.094–0.102 ms`**, and 3-pane shared-`TimeScale` dense view (`2,907` bars) renders in **`1.14 ms`**.
- **Decision:** Retain `egui::Painter` for Waves 1–4 with **mandatory** O(1) culling + DPR-aware conflation enabled by default. Define quantitative triggers (`> 4.0 ms` CPU frame median, `> 12,000` simultaneous non-conflatable shapes, or `> 350,000` vertices/frame) to introduce hybrid `egui_wgpu::CallbackTrait` instanced `wgpu` rendering for future heavy series without altering `src/viewport/`.

### DEC-R03-004 — Establish Durable Repository-Native Program Orchestration Charter
- **Date:** `2026-11-11`
- **Session:** `SESSION-R03-20261111T105832+0700`
- **Context:** Verified live Antigravity runtime capabilities (`swarm`, `define_subagent`/`invoke_subagent`, `rea` MCP) and needed a zero-copy-paste continuity mechanism across sessions.
- **Decision:** Establish [`docs/operations/SUMI_ORCHESTRATION_CHARTER.md`](operations/SUMI_ORCHESTRATION_CHARTER.md) governing role separation (`Orchestrator`, `Research`, `Engineering`, `Critic`, `Auditor`), Evidence Classification (`VERIFIED` / `INFERRED` / `UNVERIFIED` / `BLOCKED`), and Product Owner decision gates.

---

## Decisions Recorded in Session R04

### DEC-R04-001 — Clean Per-Batch Git Commit Policy (Authorized by Product Owner)
- **Date:** `2026-11-11`
- **Session:** `SESSION-R04-20261111T140100+0700`
- **Context:** On `2026-11-11`, the Product Owner explicitly directed the Orchestrator to commit code cleanly upon completion of each verified batch/milestone plan rather than leaving cumulative uncommitted changes across sessions.
- **Decision:** Create one clean, verified local commit on `master` at the end of each milestone/batch after all `cargo fmt --check`, `cargo clippy` (native + `wasm32-unknown-unknown`), `cargo test`, `--release` benchmark, `sumi-critic` remediation, and `sumi-auditor` checks pass (Milestone R03 baseline committed at `98ec576`; Milestone R04 committed at completion of Session R04). Destructive Git commands (`git reset`, `git clean`, `git checkout --`) and remote pushes remain strictly prohibited without explicit Product Owner instruction.

### DEC-R04-002 — Authorized Toolchain Additions: `wasm32-unknown-unknown` Target & BSD `TA-Lib` Reference Pin
- **Date:** `2026-11-11`
- **Session:** `SESSION-R04-20261111T140100+0700`
- **Context:** Product Owner approved installing necessary development/verification components for Milestone R04 while preserving `DEC-006` (zero contact with TradingView Desktop).
- **Decision:** Install `wasm32-unknown-unknown` via `rustup target add wasm32-unknown-unknown` to enforce compile-time WASM portability across `src/model.rs`, `src/data.rs`, `src/viewport/`, and `src/indicators/`, and clone official BSD-licensed `TA-Lib/ta-lib` (`fd57d4cfd6c812004d897feb0025fe273b1192e2`) into gitignored `research_repos/ta-lib` as a read-only mathematical reference alongside `lightweight-charts-5.2.0` (`868cae27bd1acafa0128d8d868ea740a59ae42ce`). Add **zero** new Rust crate dependencies in `Cargo.toml`.

### DEC-R04-003 — Pure `f64` `src/indicators/` Engine, `SeriesBlockIndex`, `egui::Mesh` Batching & Stretch-Factor Multi-Pane Layout
- **Date:** `2026-11-11`
- **Session:** `SESSION-R04-20261111T140100+0700`
- **Context:** Implementing Wave 2 (`SMA`, `EMA`, `BollingerBands`, `RSI`, `MACD`, `ATR`) and Wave 4 (synchronized multi-pane oscillator layout) at `100,000` bars requires: (a) pan-invariant indicator warmup with zero per-frame recomputation, (b) `O(N / 16)` min/max autoscale + conflated envelope lookups, (c) identical horizontal `conflation_factor` across all panes, and (d) `< 4.0 ms` 4-pane `100K` CPU rendering per `DECISION-RENDERER-002`.
- **Decision:**
  1. Compute all indicators once over the full dataset (`0..total_bars`) in pure `f64` (`src/indicators/`), guarded by `IndicatorCache` (`(dataset_generation, candles.len(), IndicatorConfig)`), Sterbenz-exact EMA/Wilder complementary weights, SMA cancellation reseeding, and overflow-safe high-magnitude fallbacks.
  2. Index every indicator output line with a 16-bar `SeriesBlockIndex` (`first`, `last`, `min`, `max`, `dom`, `valid_count`) enabling sub-microsecond visible autoscale and exact `[first, min, max, last]` 2-column vertical-envelope conflation with sub-bucket `None`-gap preservation.
  3. Implement LWC-inspired stretch-factor `MultiPaneLayout` (`stretch_factors` defaulting to `4.0` main : `1.0` per oscillator, `MultiPaneLayout::MIN_PANE_HEIGHT = 44.0`, monotonic pass-snapshot water-filling, and interactive `5.0 px` separator stretch transfer via `SEPARATOR_HEIGHT = 5.0`) in `src/chart.rs` sharing `ChartViewportState::time_scale` while isolating per-pane `PriceScale` (`PriceScaleOptions::sumi_oscillator`).
  4. Batch axis-aligned candlesticks, volume bars, MACD histogram bars, and zero-bleed Bollinger ribbons into `egui::Mesh` primitives so `100,000`-bar 4-pane full-fit renders in **`3.141 ms`** median (`< 4.0 ms` `DECISION-RENDERER-002` ceiling) with a single unified `32x` `conflation_factor` across all panes, and run automated GPU batch captures in background window mode (`.with_active(false)` via `SUMI_CAPTURE_BATCH`).

---

## Decisions Recorded in Session R05

### DEC-R05-001 — Pure `f64` Multi-Pane Interactive Drawing Engine, `ClipEdge` Liang-Barsky Clipping & `< 4.0 ms` Validation
- **Date:** `2026-11-11`
- **Session:** `SESSION-R05-20261111T163600+0700`
- **Context:** Wave 3 (`Milestone R05`) ports and rectifies the interactive drawing & annotation engine after live REA MCP analysis (`ev_f8b8d8b5...`, `ev_4d2d8fb2...`, `ev_01525e3b...`, `ev_8c1494cf...`, `ev_98512b7e...`) exposed 5 structural defects in legacy Sumi (`YYYY-MM-DD` `250`-day cliff, rightward-only unclipped rays, broken `Logarithmic` body drag, full-bounding-box Fibonacci hit hijack, and positive-only `'price'` pane lock).
- **Decision:**
  1. Implement `src/drawings/` (`mod.rs`, `geometry.rs`, `hit_test.rs`, `controller.rs`, `tests.rs`) in pure `f64` (`wasm32-unknown-unknown`-verified, zero new crates in `Cargo.toml`), anchoring `HorizontalLine`, `TrendLine`, $360^\circ$ `Ray`, `Rectangle`, and `FibonacciRetracement` in continuous logical `(bar_index: f64, value: f64)` coordinates (`|bar_index| <= 1.0e9`, `|value| <= 1.0e15`) scoped to `DrawingPaneTarget` (`Main` + `Oscillator(Rsi | Macd | Atr)`).
  2. Clip finite segments and $360^\circ$ rays analytically via unit-vector + half-scaled `clip_parametric_line` with exact `p == 0.0` checks and `ClipEdge` boundary snapping, render phase-anchored dashes (`paint_styled_segment_phased`) and `egui::Mesh` translucent zone/Fib bands, and enforce order-independent LWC hit-test priority (`SelectedHandle > Handle > Stroke > SelectedBody`) with `EDIT_DRAG_THRESHOLD_PX = 2.0 px` and locked-stroke viewport pan passthrough.
  3. Confirm `egui::Painter` + `egui::Mesh` remains well below the `< 4.0 ms` `DECISION-RENDERER-002` hybrid ceiling with **25 active multi-pane drawings + 6 indicators + 4 panes on `100,000` bars** (`0.2100 ms` standard `8.0 px`, `0.5720 ms` dense `1.2 px`, **`3.5207 ms` median** / `3.8042 ms` P95 full-fit `32x` conflated).

---

## Decisions Recorded in Session R06

### DEC-R06-001 — Zero-Dependency `src/data/` CSV/JSON OHLCV Engine, `1m..1W` Upward Lattice + Causal `DEF-001` Rollup, Timestamp-Locked Drawings & Offscreen Metal `wgpu` Verification
- **Date:** `2026-11-12`
- **Session:** `SESSION-R06-20261111T194303+0700`
- **Context:** Milestone R06 (`Wave 5`) replaces the synthetic-only `src/data.rs` file after live REA MCP analysis (`ev_c3081a8a...`, `ev_a4dabc00...`, `ev_ab7d475a...`, `ev_288c5960...`, `ev_3a7da525...`, `ev_eeaad96e...`, `ev_64453400...`) proved that legacy Sumi lacked client-side OHLCV timeframe aggregation (`0` matches on `aggregateTimeframe`), crashed on multi-symbol `CafeF` CSVs without symbol isolation, and dropped drawing continuity across timeframe switches; additionally, windowed macOS `CAMetalLayer` screenshots stalled with `Skipping frame due to occlusion.` when the physical display slept.
- **Decision:**
  1. Implement a zero-new-dependency, `wasm32-unknown-unknown`-verified `src/data/` module (`mod.rs`, `calendar.rs`, `timeframe.rs`, `csv_loader.rs`, `json_loader.rs`, `presets.rs`, `synthetic.rs`, `tests.rs`) featuring Hinnant O(1) proleptic Gregorian UTC calendar math, single-pass RFC-4180 CSV & bounded recursive-descent JSON kline/envelope parsers (`MAX_IMPORT_BYTES = 64 MiB`, `MAX_SCANNED_RECORDS = 2_000_000`, `MAX_IMPORT_RECORDS = 500_000`, `MAX_JSON_DEPTH = 16`), multi-symbol discovery (`SymbolTracker`), and duplicate-timestamp policies (`KeepLast`, `KeepFirst`, scale-aware `RejectConflict`).
  2. Enforce an upward-only timeframe lattice (`1m`, `5m`, `15m`, `1h`, `4h`, `1D`, `1W`; `Timeframe::can_aggregate_to` requires `target.seconds() >= self.seconds()`, rejecting downward disaggregation while stamping ISO-8601 Monday `00:00:00 UTC` on `1W` buckets) plus `aggregate_ohlcv_up_to(..., cutoff_timestamp)` to guarantee zero lookahead bias (`DEF-001`) for future replay waves.
  3. Require canonical UTC epoch seconds `Candle::timestamp: i64` and attach fractional-capable `DrawingAnchor::timestamp: Option<f64>` so `DrawingStore::remap_timeframe` preserves exact in-bar and inter-bar anchors across `1m <-> 5m <-> 15m <-> 1h <-> 4h <-> 1D <-> 1W` switches (`0.75–4.29 µs` for `25` drawings on `100K` bars) and isolate drawings per `(DataSourcePreset, symbol)` via `drawings_by_instrument`.
  4. Replace windowed `ViewportCommand::Screenshot` automation with `OffscreenWgpuHarness` (`eframe::egui_wgpu::RenderState` + `Rgba8Unorm` `2.0x` Retina texture readback on Apple M1 Metal) so `SUMI_CAPTURE_BATCH` and `SUMI_SCREENSHOT_PATH` execute in `< 1s` with zero windows, zero Dock icons, zero focus stealing, and complete immunity to macOS display sleep.


---

## Decisions Recorded in Session R07

### DEC-R07-001 — Pure `f64`/`i64` WASM-Safe Causal Bar Replay (`src/replay/`), 4-Waypoint Paper Trading Engine (`src/trading/`), Causal Drawing Guards & `egui::Mesh` Overlay Batching
- **Date:** `2026-11-12`
- **Session:** `SESSION-R07-20261112T093742+0700`
- **Context:** Milestone R07 (`Wave 6`) implements Bar Replay and Paper Trading after live REA MCP analysis (`ev_10aea63b...`, `ev_9527ede0...`, `ev_418c861e...`, `ev_eaab36ba...`, `ev_4f49eb61...`, `ev_e1958292...`, `ev_81fa9171...`, `ev_7e0b29f4...`, `ev_3f2322b6...`, `ev_19311ed5...`) uncovered 6 structural defects in legacy Sumi (`4`-HTTP-call step latency & dropped partial weekly bars, non-causal rewind cash/position corruption, missing `STOP` orders + `100`-share/no-short lockouts, close-before-intrabar bankruptcy & skipped post-entry brackets, future drawing leakage, and DOM-scraped ticket sync).
- **Decision:**
  1. Implement `src/replay/` (`mod.rs`, `controller.rs`, `tests.rs`) in pure `f64`/`i64` (`wasm32-unknown-unknown`-verified, zero new crates in `Cargo.toml`), anchoring replay at `base_cursor_index` (`&base_candles[..=base_cursor_index]` -> `aggregate_ohlcv(..., base_tf, active_tf)`) to guarantee zero future lookahead (`DEF-001`) across candles, incomplete higher-timeframe buckets, and `IndicatorCache`, with `ActiveBar` and `BaseBar` stepping (`1.25 µs` per `15m` step).
  2. Implement `src/trading/` (`mod.rs`, `orders.rs`, `portfolio.rs`, `tests.rs`) supporting symmetric `Long`/`Short` `Market`, `Limit`, and `Stop` orders, `SL`/`TP` brackets, unified Gap-Open (`t = Open`) + 3-segment causal 4-waypoint OHLC matching (`[O, L, H, C]` vs `[O, H, L, C]`, pessimistic `SL`-before-`TP` on pre-existing dual-touch bars), proportional entry-fee conservation, timeframe-invariant base-bar execution, and deterministic command-log `rewind_to` (`256.71 µs` across `4,501` bars).
  3. Stamp `Drawing::created_at_timestamp: Option<i64>` and enforce `is_causally_visible` across rendering, hit-testing, `selected_drawing()`, `undo()`, `redo()`, `clear_unlocked()`, and Drawing -> Order Ticket sync so future drawings remain hidden without deletion during replay while right-whitespace projections created during replay stay visible and interactive.
  4. Batch axis-aligned dashed/dotted lines (`paint_styled_segment_phased`) and execution fill-marker triangles (`paint_execution_fill_markers`) into single `egui::Mesh` primitives so `95,001`-bar `1m` full-fit 4-pane + 6-indicator + 25-drawing + Replay + Trading renders in **`3.923 ms`** median (`0.174 ms` standard intraday, `0.291 ms` full workstation UI), remaining within the `< 4.0 ms` `DECISION-RENDERER-002` ceiling.


---

## Decisions Recorded in Session R08

### DEC-R08-001 — Pure `f64`/`i64` WASM-Safe Multi-Symbol Watchlist & Scanner (`src/scanner/`), Causal Price/Indicator/Drawing Alert Engine (`src/alerts/`), & `Cmd+K` Command Palette
- **Date:** `2026-11-12`
- **Session:** `SESSION-R08-20261112T140031+0700`
- **Context:** Milestone R08 (`Wave 7`) implements multi-symbol scanning, causal alerts, and instant keyboard navigation after live REA MCP analysis (`ev_7bebc091...`, `ev_bc68b61c...`, `ev_17c740ab...`, `ev_51e9ade8...`, `ev_4f4abed4...`, `ev_294e5ef0...`, `ev_91304935...`, `ev_88c00cb5...`, `ev_db4484c1...`, `ev_a5767e7a...`) exposed 6 legacy architectural defects (SQLite/Pandas `/scanner` route bottleneck, HTTP-coupled `BlindPractice` session mutation, total absence of an Alert Engine alongside LWC `Date.now()`/`setTimeout` wall-clock marker coupling, symbol-only HTTP switcher modal with `50ms` `setTimeout` focus hack, DOM `querySelector('[role="dialog"]')` shortcut collision, and `None`-first metric sort contamination).
- **Decision:**
  1. Implement `src/scanner/` (`mod.rs`, `engine.rs`, `tests.rs`) and expand `src/data/presets.rs` (`DataSourcePreset::Vn30Basket` `8` VN30 tickers + `custom_peer_feeds`) in pure `f64`/`i64` (`wasm32-unknown-unknown`-verified, zero new crates in `Cargo.toml`). Single-pass `evaluate_symbol_scan_and_signals` computes Watchlist rows and historical signals across all `13` bundled feeds (`20,797` bars) in **`1.799 ms`**, strictly enforcing `<= replay_cutoff` across all peer feeds, withholding unclosed higher-timeframe tail signals, and pinning `ScannerSignalContext` `BlindPractice` reveal timestamps to bucket close.
  2. Implement `src/alerts/` (`mod.rs`, `engine.rs`, `tests.rs`) supporting `PriceLevel` (4-waypoint OHLC + stateful `@Mark` departure), `RsiThreshold` (`RSI(14)`), `MacdHistogramZeroCross` (`MACD(12,26,9)`), and `DrawingLine` (`HorizontalLine` `1m==15m` parity, finite `TrendLine`, and $360^\circ$ `Ray` relative-offset crossings with linear slope extrapolation), chronologically sorted `eval_base_timestamp` trigger logs, and bit-exact causal Replay rewind (`Once` alerts automatically re-arm when rewound prior to their trigger).
  3. Integrate the keyboard-first `Cmd+K` / `Ctrl+K` Command Palette (`13.71 µs` fuzzy/prefix ranking + dynamic `alert <price>` synthesis) with full modal isolation (`disabled_alpha = 1.0` background panel lock, gesture/kinetic cancellation, `Tab`/`Delete`/`MouseWheel` consumption) and horizontal dashed-line + conflated line `egui::Mesh` batching so `95,001`-bar `1m` full-fit 4-pane + Replay + Trading + Alerts renders in **`3.067 ms`** median (`0.182 ms` recent `1m`, `0.407 ms` full workstation, `0.296 ms` `Cmd+K` modal), well inside the `< 4.0 ms` `DECISION-RENDERER-002` ceiling.

---

## Decisions Recorded in Session R09

### DEC-R09-001 — Pure `f64`/`i64` WASM-Safe Deterministic Strategy Backtest Engine (`src/backtest/`), Continuous Causal `IS/OOS` Partition, Zero-Balance `MarginLiquidation`, & Synchronized 5th `Equity / Drawdown` Baseline Sub-Pane
- **Date:** `2026-11-14`
- **Session:** `SESSION-R09-20261114T091018+0700`
- **Context:** Milestone R09 (`Wave 8`) implements Strategy Lab, causal backtesting, parameter sweeps, multi-symbol comparison, and synchronized equity/drawdown rendering after live REA MCP analysis (`ev_30052fc9...`, `ev_0e297ef9...`, `ev_2f44b96f...`, `ev_86aa8a13...`, `ev_72d6e059...`, `ev_54c92e4a...`, `ev_fe8c9f11...`, `ev_3e04368e...`, `ev_80e6acb9...`, `ev_4d180bfe...`, `ev_f81cc480...`, `ev_fee363ed...`) uncovered 6 legacy defects (detached `/backtest` & `/strategy-lab` pages with `YYYY-MM-DD` intraday equity collapsing and array-index SVG plots, row-by-row Pandas `df.iloc[i]` + `uuid.uuid4()` overhead, Long-only + hardcoded `sqrt(252)` Sharpe, missing `Sortino`/`Calmar`/`MaxDD Duration`/`Avg R`/`Exposure %`, intrabar optimism, and Bar Replay `DEF-001` future leakage).
- **Decision:**
  1. Implement `src/backtest/` (`mod.rs`, `strategy.rs`, `metrics.rs`, `engine.rs`, `tests.rs`) in pure `f64`/`i64` (`wasm32-unknown-unknown`-verified, zero new crates in `Cargo.toml`) executing `100,000` bars with `70/30` `IS/OOS` split in **`6.318–7.832 ms`** (`< 15.0 ms` budget). Enforce strict `Close[i] -> Open[i + 1]` causal fills (`EndOfSeriesCutoff` on final bar), pessimistic `SL`-before-`TP` intrabar resolution, post-close `ATR` trailing-stop ratcheting (active during Vietnam `T+2` `min_holding_bars`), and `BacktestExitReason::MarginLiquidation` (`compute_bankruptcy_raw_price`) so bankrupt leveraged/Short accounts liquidate at `0.0` without zombie resurrection while conserving `initial_cash + sum(net_pnl) == final_cash`.
  2. Partition `in_sample_tearsheet` and `out_of_sample_tearsheet` directly from the single continuous `full_sim` (`&equity_curve[..split_idx]` vs `&equity_curve[split_idx..]` seeded with `initial_cash = equity_curve[split_idx - 1].equity`, and `exit_bar_index < split_idx` vs `>= split_idx`), conserving `100%` of boundary-straddling trades and `(1 + IS%) * (1 + OOS%) == (1 + Full%)`, and enforce inclusive causal close cutoffs (`replay_causal_close_timestamp = cursor_open_ts + base_tf.seconds() - 1`) across single runs, `12`-grid sweeps (`77.29 µs`), and `13`-symbol universe battles (`639.92 µs`).
  3. Extend `MultiPaneLayout` to 5 synchronized panes (`Main + RSI + MACD + ATR + Equity`) via `OscillatorPaneId::Equity`, rendering `SeriesBlockIndex`-conflated emerald/crimson baseline-split trapezoids clipped analytically via `emit_y_clipped_baseline_trapezoid` into `egui::Mesh` + $O(\log T)$ pan-invariant Trade Connectors (`0.190 ms` active 5-pane `95K` intraday, `0.412 ms` full workstation UI, **`1.355 ms`** `95K/100K` `1m` full-fit — well below `< 4.0 ms` `DECISION-RENDERER-002`).

---

## Decisions Recorded in Session R10

### DEC-R10-001 — Pure `f64`/`i64` WASM-Safe Vietnam Market Microstructure & Risk Planner (`src/journal/`), Interactive On-Chart `RiskRewardBox`, Causal Trade Journal, Continuous-Carry Rolling Walk-Forward (`WFO`), & Bar-Close Multi-Asset Portfolio Engine
- **Date:** `2026-11-14`
- **Session:** `SESSION-R10-20261114T122730+0700`
- **Context:** Milestone R10 (`Wave 9`) ports and upgrades the remaining legacy Sumi workflows (`/journal`, `/dashboard`, `market_rules.py`, `trade_planning.py`, `MultiPhaseBatchPanel.tsx`) after live REA MCP analysis (`ev_d326070e...`, `ev_ec714173...`, `ev_0bd6b7fd...`, `ev_0b2302d7...`, `ev_0ae4bec7...`, `ev_0daaf78d...`, `ev_7838e78d...`, `ev_590404cc...`, `ev_7eebd97b...`, `ev_bc6281a3...`, `ev_72fb8779...`) uncovered 6 legacy defects (stop-out risk sizing omitting sell commission/slippage/`0.10%` VN transfer tax, `1,000 VND` vs raw `VND` tick blindness & `HOSE` `10k`/`50k` tier-crossing band rounding errors, absence of an on-chart `Risk-Reward` tool, HTTP session-bound `/journal` without causal Replay masking or quantitative `SQN`/`Discipline %` attribution, CSV formula-injection exposure, and static calendar batch evaluation without rolling `IS -> OOS` Walk-Forward or multi-asset portfolio weighting).
- **Decision:**
  1. Implement `src/journal/` (`mod.rs`, `market_rules.rs`, `planner.rs`, `ledger.rs`, `tests.rs`) in pure `f64`/`i64` (`wasm32-unknown-unknown`-verified, zero new crates in `Cargo.toml`). Model `HOSE`, `HNX`, `UPCOM`, `Vn30Futures` (`x100,000 VND`/`x100 kVND` point multiplier + `17%` VSD initial margin), `VnIndexBenchmark` (no statutory band), `UsEquities`, and `CryptoSpotPerp` with exchange-local civil timezones (`UTC+7` ICT / `UTC-5` EST), tier-crossing `HOSE` tick stepping, all-in friction sizing (`101.12 µs` per `1,000` evaluations), `0`-lot Ticket guards, `DEF-001` causal `TradeJournalStore` with Paper Trading rewind pruning, Van Tharp `SQN` / `Discipline %` / `Cost of Mistakes`, and formula-injection-hardened single-line RFC-4180 CSV + Markdown export (`601.12 µs` for `500` entries).
  2. Add `DrawingGeometry::RiskRewardBox { start, end, stop_price }` to `src/drawings/` and `src/chart.rs` with 4 snapshot-evaluated non-drifting handles (`RrEntryLeft`, `RrEntryRight`, `RrTarget`, `RrStop`), `Long <-> Short` target crossover reflection, `Log`/`Linear`-invariant body drag, batched emerald/crimson zone meshes, and bidirectional Planner / Order Ticket synchronization.
  3. Implement `src/backtest/walk_forward.rs` with continuous `SimulationCarryState` across `OOS[k] -> OOS[k+1]` boundaries (preserving open trades, `VN T+2` `min_holding_bars`, cash, and peak equity; `5.958 ms` across `6,667` `15m` bars / `116` OOS trades) and bar-close-stamped `run_portfolio_backtest` (`EqualWeight` & zero-dilution-free annualized `CausalInverseVolatility`; `106.00 µs` `VN30`, `2.855 ms` `13`-feed universe), rendering directly inside `OscillatorPaneId::Equity` in **`0.276–0.369 ms`** full workstation CPU frame time (`< 4.0 ms` `DECISION-RENDERER-002`).

---

## Decisions Recorded in Session R11

### DEC-R11-001 — Pure `f64`/`i64` WASM-Safe Order-Flow Volume Profile (`VRVP`/`FRVP`), Exchange-Session & Anchored `VWAP` ($\pm 1\sigma, \pm 2\sigma$) & `DEF-001` Causal `Relative Strength vs VNINDEX` + Live Rebased Compare Overlay
- **Date:** `2026-11-15`
- **Session:** `SESSION-R11-20261114T231200+0700`
- **Context:** Milestone R11 (`Wave 10`) delivers Volume Profile, VWAP/AVWAP bands, and causal Relative Strength & Rebased Comparison after live REA MCP analysis (`ev_3a6157b7...`, `ev_c78c7f15...`, `ev_4ba60dd6...`, `ev_65d9ad93...`, `ev_ae62bcb4...`, `ev_6f87d84a...`, `ev_bcea32b4...`, `ev_8eff53c9...`, `ev_a620817a...`, `ev_621eb9d1...`, `ev_bc43e378...`, `ev_8058963b...`) confirmed legacy Sumi lacked `VRVP`/`FRVP`, Session/Anchored `VWAP`, and client-side `DEF-001` causal Relative Strength / Rebased Benchmark overlays.
- **Decision:**
  1. Implement `src/indicators/volume_profile.rs`, `src/indicators/vwap.rs`, and `src/indicators/relative_strength.rs` in pure `f64`/`i64` (`wasm32-unknown-unknown`-verified, `0` new crate dependencies). Enforce exact `OHLCV_PROXY` buy/sell volume conservation, TPO fallback, scale-invariant micro-price floor & `1e-18` Value Area dust threshold, CME dual-row `70%` Value Area (`VAH`/`VAL`) + `POC`, rescaled overflow-safe `WelfordVwapAcc` (`1e160`/`1e-160`), venue intraday offsets vs UTC civil `1D`/`1W` boundaries (`B2`/`B3`), periodic `64`-bar `RS SMA` recomputation (`M5`), and tail-coverage-scored benchmark lookup (`src/data/presets.rs`).
  2. Add `DrawingGeometry::FixedRangeVolumeProfile` (`DrawingTool::FixedRangeVolumeProfile`, `2`-anchor fixed-range Volume Profile with clamped offscreen handles) and `DrawingGeometry::AnchoredVwap` (interactive $\pm 1\sigma, \pm 2\sigma$ tail projection with `tail_segments` hit-testing and left-drag `bar_index >= 0.0` clamping, `M4`) inside `src/drawings/`.
  3. Add conflation-safe per-session/gap `egui::Mesh` VWAP & AVWAP batching (`B1`/`M2`), asymmetric `65`-sample/`<= 2,048` exact-hash `R11RenderMemo` (`M1`), `O(1)` live-scaled Rebased Compare Overlay over `SeriesBlockIndex::first_valid_index` (`M3`), and synchronized `OscillatorPaneId::RelativeStrength` (`src/chart.rs`, `src/app.rs`), holding full workstation frames within `0.256–3.171 ms` (`< 4.0 ms` `DECISION-RENDERER-002`).

---

## Decisions Recorded in Session R12

### DEC-R12-001 — Synchronized Multi-Chart Grid (`1x1`/`1x2`/`2x1`/`2x2`), `Ichimoku Cloud` + `Cumulative Volume Delta (CVD)` (`DEF-001`), and `schema_version = 1` `WorkspaceTemplate` (`RON`/`JSON`)
- **Date:** 2026-11-15
- **Status:** Accepted & Shipped (`Milestone R12 / Wave 11`, verified via `174/174` Cargo unit tests, native + `wasm32` `clippy -D warnings`, `--r12` release benchmark `0.2320–1.3035 ms` / `p99 <= 1.6077 ms < 4.0 ms`, `SUMI_CAPTURE_BATCH=r12`, `sumi-critic` `CRITIC_PASS`, and post-build `rea` audit `REA-R12-09`..`10`).
- **Context:** Milestone R11 established order-flow overlays (`VRVP`/`FRVP`, session/anchored VWAP, causal RS) on a single chart workspace. Institutional multi-timeframe and multi-asset execution requires synchronized side-by-side chart panes (`1x1`/`1x2`/`2x1`/`2x2`), equilibrium/momentum cloud overlays (`Ichimoku Cloud`), directional delta pressure tracking (`CVD`), and deterministic serializable workspace presets (`RON`/`JSON`) without native-only crates or look-ahead leakage (`DEF-001`).
- **Decision:**
  1. Add pure `f64`/`i64` WASM-safe `MultiChartLayoutKind` (`MultiChartLayoutKind::{Single1x1, SplitVertical1x2, SplitHorizontal2x1, Quad2x2}`, `MultiChartSyncConfig` for `Sync X`/`Sync T`/`Sync S`/`Sync TF`, `SyncedTimeWindow`, `resolve_linked_crosshair_bar`) and `WorkspaceTemplate` (`WORKSPACE_TEMPLATE_SCHEMA_VERSION = 1`, strict linear-scan RFC 8259 JSON & RON codecs rejecting duplicate/unknown keys and negative/out-of-range dimensions) under `src/viewport/` paired with `SecondaryChartSlot` in `src/app.rs`.
  2. Enforce `DEF-001` causal Bar Replay across multi-timeframe secondary slots via `causal_closed_prefix_len` (`time + span <= replay_cutoff_ts`) while preserving `custom_file` provenance (`CRITIC-R12-13`), zero false crosshair pinning over future whitespace (`CRITIC-R12-04`), and UTC `Sync T` alignment (`CRITIC-R12-06`).
  3. Add `src/indicators/ichimoku.rs` (`Tenkan 9`, `Kijun 26`, `Senkou Span A/B` projected `+26`, strict `None` tail on `Chikou` `-26`) with triangle-split Kumo crossover (`emit_ichimoku_kumo_segment_triangles`), overflow-safe midpoint `(0.5 * lo + 0.5 * hi)`, and `src/indicators/cvd.rs` (`OHLCV_PROXY` `Cumulative` / timezone-aware `Session` reset + `9`-SMA + `OscillatorPaneId::Cvd` + `cvd_price_scale` + `CandleBlockIndex` `16`-bar acceleration and `PlotDataLayerMemo` inside `R11RenderMemo`), holding `Quad 2x2` `200,000`-bar `32x` conflated frames with Kumo + Session VWAP + Session CVD at `1.3035 ms` mean / `1.6077 ms` `p99` (`< 4.0 ms`).

---

## Decisions Recorded in Session R13

### DEC-R13-001 — Session TPO / Market Profile, Multi-Interval `VWAP`, Causal VN30 Breadth / `RRG` (`AT05`/`AT09`/`AT17`/`DEF-001`), Compact `K`/`M`/`B`/`T` `CVD` Right Axis, & Pan-Invariant `WorldBinIntervalAccumulator`
- **Date:** `2026-11-15`
- **Status:** Accepted & Shipped (`Milestone R13 / Wave 12`, verified via `181/181` Cargo unit tests, native + `wasm32` `clippy -D warnings`, `--r13` release benchmark `0.2588–2.4689 ms` / `p99 <= 2.8089 ms < 4.0 ms`, `SUMI_CAPTURE_BATCH=r13`, `sumi-critic` `CRITIC_PASS`, and post-build `rea` audit `REA-R13-11`..`12`).
- **Context:** Milestone R12 left two non-blocking Minor items (`cvd_price_scale.reset_auto_scale()` across primary transitions and an overflow-safe midpoint helper) alongside raw uncompacted `CVD` right-axis numbers overflowing the `86 px` gutter at `10M+`/`1B+` scales. Institutional flow analysis also required session TPO / Market Profile (`POC`, 70% `VAH`/`VAL`, `IB`, `Single Prints`, `Naked POC`), multi-interval `VWAP` anchors, and causal whole-universe Market Breadth + `RRG` rotation (`AT05`/`AT09`/`AT17`/`DEF-001`).
- **Decision:**
  1. Add `PriceAxisFormatMode::CompactVolume` (`format_compact_volume_axis`) with post-rounding tier promotion (`K`/`M`/`B`/`T`, signed `-`, `<= 8` chars) on `PriceScaleOptions::sumi_cvd_oscillator()` across grid ticks, cursor pills, linked crosshairs, and CVD drawing badges, alongside clipped axis pills and R12 Minor carry-overs (`cvd_price_scale.reset_auto_scale()` across all primary-slot transitions + `CustomFile` restore, and `safe_midpoint_f64` in `src/chart.rs`).
  2. Implement pure `f64`/`i64` WASM-safe `src/indicators/market_profile.rs` (`compute_session_market_profiles`, `HistoricalNakedPoc` tracking across `up to 96` prior sessions, `ProfileSessionPeriod` on `IndicatorConfig` & `5` built-in `WorkspaceTemplate` presets, valid-only `ib_bars` counting, batched `egui::Mesh` rendering with `session_w >= 3.0` and off-left `IB` culling), extend `src/indicators/vwap.rs` with `VwapAnchorMode` (`AutoSession`/`Daily`/`Weekly`/`Monthly`/`Yearly`, `"1mo"` monthly alias, zeroed negative-UTC `effective_off` on `D1`/`W1`), and add `WorldBinIntervalAccumulator` + dual-sided `SeriesBlockIndex::range_bounds` conflated `CVD` delta histograms in `src/chart.rs` for pan-invariant sub-pixel envelopes without spike dropping or disjoint session bridging.
  3. Implement pure `f64`/`i64` WASM-safe `src/scanner/breadth.rs` (`AT09` Aggregate-Before-Ratio value-weighted Money Flow, `AT05` zero-value `50.0` fallback (`money_flow_buy_pct: f64`, `vn30_money_flow_buy_pct: f64`), `AT17` VN30/Universe participation ratios, JdK `RS-Ratio`/`RS-Momentum` 4-quadrant `RRG` trajectories gated strictly by `replay_causal_close_timestamp` (`0` bar lag on `1D`) + `causal_peer_open_cutoff` + `is_active_tail_incomplete()`), and render inside the 340px `Watch`/`Sig`/`Breadth` dock with obstacle-seeded multi-ring `RRG` labels and symmetric `allocate_exact_size` Liquidity Heatmap tiles.


