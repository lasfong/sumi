# CHART ENGINE RESEARCH MAP — SESSION R01

- **Session Identity:** `SESSION-R01-20261110T202837+0700`
- **Purpose:** Research-driven decomposition of a TradingView-class charting workstation into investigable subsystems.

> [!NOTE]
> This map governs what we learn before we build. Each subsystem must be researched with real evidence before its Rust implementation boundary is finalized.

---

## Subsystem 1 — Chart Rendering and Viewport Architecture

### Research Questions
1. How is the price-time coordinate space projected onto screen pixels?
2. How do zoom levels define a "bar slot width" and what are the limits?
3. How is the viewport's right anchor point (the "newest visible bar") maintained across zoom/pan operations?
4. How does sub-pixel panning work without accumulating floating-point drift?
5. What data structures are needed to efficiently compute visible bar ranges from a viewport transform?

### REA Investigation Methods
- **`analyze_javascript_application`** on `lightweight-charts` source: Extract `TimeScale`, `PriceScale`, `Coordinate` classes — these contain the production coordinate transform implementation.
- **`trace_javascript_semantics`** on ARCHIVE `ChartKernel-Bufw8K34.js`: Trace data-flow from canvas resize events through coordinate mapping functions.
- **Source reading:** `lightweight-charts/src/model/time-scale.ts`, `price-scale.ts`, `chart-model.ts`.

### Necessary Evidence
- Coordinate transform formula: `price → pixel_y`, `bar_index → pixel_x` with zoom factor
- Bar slot width computation (pixels per bar)
- Right-anchor invariant maintenance during zoom
- Tick mark interval selection algorithm for axes

### Recoverable Algorithms
- **`lightweight-charts` v5 (Apache 2.0):** Full TypeScript source of `TimeScale`, `PriceScale`, `ICoordinateSpaceTransform` — 100% inspectable.
- Sumi Archive: `ViewportMapping` already implements basic linear projection (confirmed from source + REA string evidence).

### Unknowns and Technical Limits
- Sub-pixel anti-aliasing for axis ticks in a GPU renderer (egui uses wgpu — different from canvas2d)
- Exact "log scale" vs "linear scale" price projection formula in TV's proprietary engine

### Proposed Rust Implementation Boundary
```rust
struct ViewportCamera {
    right_anchor_bar: f64,   // sub-pixel position
    bars_per_pixel: f64,     // zoom level
    price_min: f64,
    price_max: f64,
}
impl ViewportCamera {
    fn bar_to_x(&self, bar: f64, plot_rect: Rect) -> f32;
    fn price_to_y(&self, price: f64, plot_rect: Rect) -> f32;
    fn x_to_bar(&self, x: f32, plot_rect: Rect) -> f64;
    fn y_to_price(&self, y: f32, plot_rect: Rect) -> f64;
    fn zoom_at(&mut self, focal_bar: f64, factor: f64);
    fn pan_by_pixels(&mut self, delta_x: f32, pixels_per_bar: f64);
    fn fit_to_data(&mut self, candle_count: usize, price_range: (f64, f64));
}
```

### Tests Required
- Round-trip: `bar → x → bar` within floating-point epsilon
- Zoom-at-cursor invariant: `bar_at_cursor` unchanged after `zoom_at(cursor_bar, factor)`
- Pan-then-undo: `pan_by_pixels(delta); pan_by_pixels(-delta)` returns to original state
- Fit: `fit_to_data` leaves all bars visible with configurable margin

---

## Subsystem 2 — Drawing Engine and Coordinate Systems

### Research Questions
1. How are drawing anchor points stored (time+price vs pixel)?
2. How are trendlines, horizontal lines, Fibonacci retracements, and rectangles hit-tested?
3. How does "magnet mode" snap to OHLCV levels?
4. What is the minimal state machine for a drawing being created vs. edited vs. locked?

### REA Investigation Methods
- **Source reading (authorized):** `ARCHIVE/frontend/src/features/drawings/drawingGeometry.ts` — fully recoverable (all algorithms are in readable TypeScript source)
- **`analyze_javascript_application`** on `lightweight-charts` source: `PrimitiveRenderer` and `IPrimitivePaneView` interfaces.

### Necessary Evidence
- Hit-test tolerance values (confirmed from source: `9px` anchor radius, `8px` line tolerance)
- Ray endpoint projection formula (confirmed from source: `rayEndpoint`)
- Fibonacci price formula (confirmed from source: `start + (end - start) * ratio`)

### Recoverable Algorithms
All drawing geometry algorithms are fully available in the archived TypeScript source — no additional REA extraction needed.

| Algorithm | Source File | Status |
|---|---|---|
| `distanceToSegment` | `drawingGeometry.ts:8` | ✅ Fully recovered |
| `pointInRectangle` | `drawingGeometry.ts:14` | ✅ Fully recovered |
| `rayEndpoint` | `drawingGeometry.ts:16` | ✅ Fully recovered |
| `fibonacciPrice` | `drawingGeometry.ts:37` | ✅ Fully recovered |
| `hitProjectedDrawing` | `drawingGeometry.ts:41` | ✅ Fully recovered |
| `layoutDrawingText` | `drawingGeometry.ts:24` | ✅ Fully recovered |

### Proposed Rust Implementation Boundary
```rust
// In src/drawings.rs
pub struct DrawingAnchor { pub bar_index: f64, pub price: f64 }
pub enum DrawingTool { TrendLine, HorizontalLine, Rectangle, Ray, FibonacciRetracement, Text }
pub struct Drawing { pub id: DrawingId, pub tool: DrawingTool, pub anchors: Vec<DrawingAnchor>, pub style: DrawingStyle }
impl Drawing {
    pub fn project(&self, camera: &ViewportCamera, rect: Rect) -> ProjectedDrawing;
    pub fn hit_test(&self, screen: Pos2, camera: &ViewportCamera, rect: Rect) -> Option<HitPart>;
}
```

### Tests Required
- Hit-test: each drawing type at tolerance boundary
- Projection round-trip: anchor → screen → back within epsilon
- Fibonacci price at ratios 0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0

---

## Subsystem 3 — Indicator Execution Engine

### Research Questions
1. What is the minimal streaming interface for an indicator (sliding window vs. full recalculation)?
2. How are multi-output indicators (MACD = line + signal + histogram) modeled?
3. How is Ichimoku's future-shifted cloud handled with a time-indexed series?
4. What are the warm-up period semantics for each indicator?

### REA Investigation Methods
- **Source reading:** `ARCHIVE/frontend/src/features/indicators/indicatorDomain.ts` — placement model and style schema (fully recovered)
- **TA-Lib source** (C, BSD license): Reference implementations for all 18 required indicators
- **lightweight-charts v5 source:** Series data model for multi-output indicators

### Recoverable from Archive (IndicatorDomain)
18 supported indicators with placement metadata:
```
ema, sma, bbands, macd, rsi, cci, atr, volume, volume_sma,
mfi, stoch, adx, kc, psar, supertrend, ichimoku, relative_strength
```

### Proposed Rust Implementation Boundary
```rust
// In src/indicators.rs
pub trait Indicator: Send + Sync {
    fn name(&self) -> &str;
    fn warm_up_period(&self) -> usize;
    fn calculate(&self, candles: &[Candle], params: &IndicatorParams) -> IndicatorOutput;
}
pub struct IndicatorOutput { pub series: Vec<NamedSeries>, pub placement: Placement }
pub struct NamedSeries { pub key: String, pub values: Vec<Option<f64>>, pub style: SeriesStyle }
pub enum Placement { Price, Oscillator, Volume }
```

### Tests Required
- SMA(5) of `[1,2,3,4,5,6,7]` = `[None,None,None,None,3.0,4.0,5.0]`
- EMA crossover detection
- RSI at known values (RSI(14) on standard test dataset from Wilder 1978)
- Bollinger Band width expansion during high-volatility series

---

## Subsystem 4 — Time Series and Market Data Models

### Research Questions
1. What is the canonical OHLCV model for handling weekend gaps, market holidays, and intraday sessions?
2. How does the time axis handle irregular time intervals (e.g., 1-minute bars with missing sessions)?
3. How is timeframe aggregation implemented (1D → 1W rollup)?

### REA Investigation Methods
- **`lightweight-charts` source:** `SeriesData`, `WhitespaceData`, `Time` type definitions — the library's canonical data model.
- **Source reading:** `ARCHIVE/frontend/src/components/chart/workspaceTypes.ts` — `CandleData` schema (confirmed: `{time, open, high, low, close}`)

### Current State
Sumi's `model.rs` already has `Candle` struct (confirmed from REA string evidence: field names `symbol`, `timeframe`, `visible_count`, `right_offset` recovered from binary). This is a good foundation.

### Tests Required
- Weekend gap: bar sequence with time jump correctly renders gap
- Aggregation: 5× 1D bars aggregate to 1W OHLCV correctly (open=first.open, high=max(high), low=min(low), close=last.close, vol=sum(vol))

---

## Subsystem 5 — Multi-Pane and Multi-Chart Synchronization

### Research Questions
1. How is pane height managed (stretch factor vs. absolute pixel height)?
2. How is the time axis shared across price + indicator panes?
3. How does crosshair synchronization work across panes?

### REA Investigation Methods
- **Source reading:** `ARCHIVE/frontend/src/components/chart/PaneManager.ts`, `SeriesManager.ts`
- **`lightweight-charts` source:** `IPane` API, `IChartApi.panes()`

### Recoverable from Archive (IndicatorChartSnapshot)
```typescript
// From workspaceTypes.ts — fully confirmed
panes: Array<{ id: PaneId; index: number; height: number; stretchFactor: number; seriesCount: number }>
```

### Proposed Rust Implementation Boundary
```rust
// In src/panes.rs
pub struct PaneLayout {
    pub panes: Vec<PaneDescriptor>,
}
pub struct PaneDescriptor {
    pub id: PaneId,
    pub stretch_factor: f32,  // relative height weight
    pub rect: Rect,            // computed each frame
}
impl PaneLayout {
    pub fn compute_rects(&mut self, total_rect: Rect, time_axis_height: f32);
    pub fn crosshair_y_for_pane(&self, pane_id: PaneId, price: f64) -> f32;
}
```

---

## Subsystem 6 — Replay and Historical Execution

### Research Questions
1. What is the state model for a "cursor" in replay mode (current bar index + sub-bar position)?
2. How does replay reconstruct indicator values at a past point in time?
3. How does replay interact with drawing tools (drawings placed before cursor vs. after)?

### REA Investigation Methods
- **Source reading:** `ARCHIVE/frontend/src/features/replay/` (directory confirmed in archive)
- **Source reading:** `ARCHIVE/frontend/src/components/chart/ChartKernel.ts:70-120` — `createReplayKernel` initialization

### Notes
Replay is a complex deferred subsystem. Research should precede implementation.

---

## Subsystem 7 — UI State and Workspace Persistence

### Research Questions
1. What is the minimal serialization schema for workspace state?
2. How does eframe's native persistence interact with our custom state?

### Current State (confirmed from REA binary evidence)
RON serialization for `SumiWorkstationApp` is already compiled in (symbols `ron::options::Options::from_str_seed` and `Options::to_string` recovered for `SumiWorkstationApp`). eframe uses RON for the `persistence` feature.

**Recovered field schema (from binary strings):** `symbol`, `timeframe`, `show_grid`, `show_crosshair`, `show_volume`, `visible_count`, `right_offset`.

---

## Subsystem 8 — Performance and GPU Rendering

### Research Questions
1. What is wgpu's tessellation cost for large numbers of candlestick primitives?
2. What is the threshold where `egui::Painter` calls become a performance bottleneck?
3. Is GPU instanced rendering viable for candlestick bars in wgpu?

### REA Investigation Methods
- **REA `observe_native_calls`** (future, with TCC Screen Recording permission): Profile render loop frame timings.
- **wgpu source** (Apache 2.0): Understand tessellation pipeline.

### Notes
This is research for >1000-bar rendering scenarios. For the first 500-bar MVP, the current egui Painter approach is proven adequate (confirmed by Session 01 GUI evidence).

---

## Priority Order for Session 03+

1. **Subsystem 1 (Viewport)** — Highest impact; blocks all other interactive chart work. Research `lightweight-charts v5` source with REA before writing Rust.
2. **Subsystem 3 (Indicators)** — Pure computation; can be implemented in parallel with Subsystem 1.
3. **Subsystem 2 (Drawings)** — Algorithms fully recovered from archive; can be ported to Rust once viewport is stable.
4. **Subsystem 4 (Data Model)** — Enhance existing `model.rs`; low risk.
5. **Subsystem 5 (Multi-pane)** — Depends on Subsystem 1 and 3 being stable.
6. **Subsystems 6, 7, 8** — Later waves.
