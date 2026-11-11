# RUST VIEWPORT ARCHITECTURE DESIGN — MODULAR `src/viewport/` ENGINE

- **Original Session:** `SESSION-R02-20261110T230337+0700`
- **Audited, Redesigned & Implemented In:** `SESSION-R03-20261111`
- **Target Module:** [`src/viewport/`](file:///Users/mizuhara/workspace/sumi/src/viewport/mod.rs)
- **Status:** `DESIGN_RECTIFIED_AND_IMPLEMENTED`

---

## 1. R02 Design Audit & Rectification Summary

Independent source verification in Session R03 identified critical mathematical and architectural flaws in the inherited R02 monolithic `ViewportCamera` draft, requiring design rectification prior to Milestone R03 coding:

| Defect ID | R02 Flaw | R03 Rectification in `src/viewport/` |
|-----------|----------|--------------------------------------|
| `CRITICAL-01` | Operator precedence bug in `x_to_float_index`: `(base + ro - delta * 1e6).round() / 1e6` | Fixed to `((base + ro - delta) * 1_000_000.0).round() / 1_000_000.0` in `TimeScale::local_x_to_float_index` |
| `CRITICAL-02` | Half-slot (`0.5`) contradiction in R02 fixtures (`x_to_float_index(454.0) == 50.0`) | Documented & tested exact LWC invariant: `local_x_to_float_index(index_to_local_x(i)) == i - 0.5` and `local_x_to_index(454.0) == ceil(49.5) == 50` |
| `CRITICAL-03` | Division-by-zero in `y_to_price` when `internal_height() == 1.0` (`1.0 - 1.0 == 0.0`) | Explicit guard `internal_height() <= 1.0` returning safe fallback (`None` / midpoint) with zero `NaN`/`Inf` |
| `CRITICAL-04` | Sign bug in `to_log` (`sign * log10(...) + offset` missing outer parens) + missing `\|x\| < 1e-15` guard & dynamic `LogFormula` | Exact LWC `to_log` / `from_log` / `log_formula_for_price_range` implemented and property-tested across positive, zero, negative, and micro-price (`< 1e-4`) ranges |
| `CRITICAL-05` | `f64::clamp(min, max)` runtime panic when `min > max` (e.g. `1` bar with `fix_left_edge && fix_right_edge` or narrow width) | `lwc_clamp(v, min, max)` (`v.max(min).min(max)`) matching LWC `mathex.ts` — never panics |
| `HIGH-06..10` | Double-margin in `fit_price_range`; missing `rightOffset` in `fitContent`; wrong `setWidth` clamp order; pixel-space vs bar-space `KineticAnimation` missing `MaxStartDelay = 50ms` | Unpadded logical `PriceRange` (margins applied once in pixel space); exact `fit_content` preserving `options.right_offset`; `set_width` updates width before clamping; bar-space `KineticAnimation` with `50ms` stale-drag cutoff |
| `ARCH-15..16` | Monolithic `ViewportCamera` fusing `TimeScale` + `PriceScale` and blurring pane-local vs screen coordinates | Clean separation into shared `TimeScale`, per-pane/axis `PriceScale`, `PaneViewport` (`Logical <-> PaneLocal <-> Screen`), `KineticAnimation`, and `candle_metrics` |

---

## 2. Module Boundaries & Responsibilities

```text
src/viewport/
├── mod.rs             # Public API, coordinate-space documentation, safe math (`lwc_clamp`)
├── time_scale.rs      # Shared horizontal scale across all panes (`TimeScale`, `HorzScaleOptions`)
├── price_scale.rs     # Per-pane / per-axis vertical scale (`PriceScale`, `PriceScaleMode`, `LogFormula`)
├── pane_viewport.rs   # 3-space coordinate mapper (`Logical/Data <-> PaneLocal <-> Screen`)
├── kinetic.rs         # Inertial pan engine on `right_offset` bar units (`KineticAnimation`)
└── candle_metrics.rs  # LWC `optimal_candlestick_width`, wick parity, and sub-pixel OHLCV conflation
```

### Separation of Concerns

1. **Domain Data (`src/model.rs`):** Owns `Candle` (`OHLCV`), `MarketDataset`, `Timeframe` — zero viewport or UI state.
2. **Shared Horizontal Viewport (`TimeScale` in `src/viewport/time_scale.rs`):**
   - **Scope:** One instance per chart workspace, shared across all vertically stacked panes (Main OHLCV pane, Volume overlay, future RSI/MACD panes).
   - **Responsibilities:** `bar_spacing`, `right_offset`, `width`, `base_index`, cursor-anchored `zoom`, horizontal `start_scroll`/`scroll_to`/`end_scroll`, time-axis `start_scale`/`scale_to`, `set_width`, `fit_content`, `set_visible_range`, `visible_logical_range`, `visible_strict_range`, O(1) `strict_visible_data_range` / `culling_data_range` (`Range<usize>`), and `conflation_factor`.
3. **Scoped Vertical Viewport (`PriceScale` in `src/viewport/price_scale.rs`):**
   - **Scope:** One instance per axis per pane. Even a single-pane candlestick + volume chart uses **two** independent `PriceScale` instances sharing one `TimeScale`:
     - `main_price_scale` (`scale_margin_top = 0.10`, `scale_margin_bottom = 0.20` when volume is shown or `0.10` when hidden, `Normal`/`Logarithmic`/`Percentage`/`IndexedTo100`).
     - `volume_price_scale` (`scale_margin_top = 0.82`, `scale_margin_bottom = 0.00`, `Normal`, pinned bottom `0.0`).
   - **Responsibilities:** Logical `PriceRange`, `LogFormula`, pixel margins, `price_to_local_y`, `local_y_to_price`, `autoscale_candles` / `autoscale_values`, vertical axis drag-zoom (`start_scale`/`scale_to`), manual vertical pan (`start_scroll`/`scroll_to`), and tick generation.
4. **Coordinate Bridge (`PaneViewport` in `src/viewport/pane_viewport.rs`):**
   - **Scope:** Ephemeral zero-cost borrow `(&TimeScale, &PriceScale, PlotRect, base_value)` constructed during layout/input/rendering for a specific pane and series.
   - **Explicit 3-Tier Coordinate Spaces:**
     - **Logical / Data Space:** `bar_index: i64`, `float_index: f64`, `price: f64` (`f64`, WASM-safe, UI-agnostic).
     - **Pane-Local Space:** `local_x ∈ [0, plot_rect.width)`, `local_y ∈ [0, plot_rect.height)` (`f64`, origin `(0, 0)` at pane plot top-left).
     - **Screen Space:** `screen_x = plot_rect.left + local_x`, `screen_y = plot_rect.top + local_y` (`f64`, converted to `egui::Pos2` `f32` strictly at the `egui::Painter` submission boundary).
5. **Interaction & Inertia (`KineticAnimation` in `src/viewport/kinetic.rs`):**
   - Pure state machine operating on `(time_ms: f64, right_offset_bars: f64)` with zero dependency on `std::time::Instant` or windowing APIs (`wasm32-unknown-unknown` compatible).
6. **Rendering (`src/chart.rs`):**
   - Consumes `&MarketDataset`, `&mut ChartViewportState`, and `egui::Ui` to translate `egui::Response` / pointer events into `PaneViewport` local coordinates, update scales, cull invisible bars in O(1), conflate sub-pixel bars when `bar_spacing < 1.0`, and submit triangles to `egui::Painter`.

---

## 3. Core Testable Invariants

All invariants are enforced by deterministic automated unit and property-based tests in `src/viewport/`:

1. **Discrete Slot Round-Trip:** For every integer bar index `i` and valid `TimeScale` (`width > 0, total_bars > 0`), `local_x_to_index(index_to_local_x(i)) == Some(i)` and `|local_x_to_float_index(index_to_local_x(i)) - (i as f64 - 0.5)| < 1e-5`.
2. **Screen ↔ Pane-Local Offset Exactness:** `screen_x_to_local_x(index_to_screen_x(i)) == index_to_local_x(i)` for arbitrary non-zero `plot_rect.left` and `plot_rect.top` (preventing cursor-zoom drift when left/top toolbars or gutters are present).
3. **Cursor-Anchored Zoom Lock:** For any `local_zoom_x ∈ [1.0, width]` and `scale ∈ [-0.9, 2.0]`, when `!right_bar_stays_on_scroll` and `right_offset` / `bar_spacing` are strictly inside their clamp bounds before and after `zoom`, `|float_index_after - float_index_before| <= 2e-5`.
4. **Price Round-Trip Across All 4 Modes:** For `Normal`, `Logarithmic` (including negative and `< 1e-4` prices), `Percentage`, and `IndexedTo100`, `|local_y_to_price(price_to_local_y(p, base), base) - p| / max(1.0, |p|) < 1e-7` whenever `internal_height > 2.0` and `range.len() > 0`.
5. **Zero-Panic & Finite Output Guarantee:** Across empty datasets (`0` bars), single-bar datasets (`1` bar), degenerate flat prices (`min == max`), extreme dimensions (`width = 0.0, 0.5, 1.0, NaN, Inf`; `height = 0.0, 1.0`), and `fix_left_edge && fix_right_edge`, no method ever panics or emits `NaN` / `±Infinity`.
