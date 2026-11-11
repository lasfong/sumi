# INTERACTIVE DRAWING & ANNOTATION ENGINE SPECIFICATION (`MILESTONE R05 / WAVE 3`)

- **Session Identity:** `SESSION-R05-20261111T163600+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Research Gate Status:** `VERIFIED` (`DEC-005` Cleared — Live REA MCP + LWC v5.2.0 Apache 2.0 + Legacy Sumi Archive)
- **Authorized Reference Targets:**
  1. **Lightweight Charts v5.2.0** (`/Users/mizuhara/workspace/sumi/research_repos/lightweight-charts-5.2.0/src/{model/iseries-primitive.ts,model/ipane-primitive.ts,model/primitive-drawing-utils.ts,model/pane-primitive-wrapper.ts,model/series-primitive-wrapper.ts,plugins/primitive-wrapper-base.ts,gui/pane-widget.ts}`, commit `868cae27bd1acafa0128d8d868ea740a59ae42ce`, `Apache-2.0`)
  2. **Legacy Sumi Archive** (`/Users/mizuhara/workspace/sumi/ARCHIVE/frontend/src/features/drawings/{drawingDomain.ts,drawingGeometry.ts,drawingMagnet.ts,useDrawingWorkspaceController.ts,SumiPrimitiveDrawingProvider.ts,DrawingCommandHistory.ts}` & `__tests__/drawingGeometry.test.ts`)
  3. **Compliance Boundary (`DEC-006`):** Proprietary TradingView Desktop was **not** inspected, hooked, or queried.

---

## 1. Live REA MCP Execution Evidence (`Milestone R05`)

All REA operations below were executed live via the `rea-agents@6.8.0` MCP server during `SESSION-R05-20261111T163600+0700`:

| Op ID | REA Tool & Query | Target Scope | Actual Verified Output | Exact 64-Hex Evidence ID |
| :--- | :--- | :--- | :--- | :--- |
| `REA-R05-01` | `analyze_javascript_application` (`detail: "summary"`) | `ARCHIVE/frontend` (`6` TS files: `drawingDomain.ts`, `drawingGeometry.ts`, `drawingMagnet.ts`, `useDrawingWorkspaceController.ts`, `SumiPrimitiveDrawingProvider.ts`, `DrawingCommandHistory.ts`) | Artifact SHA-256 `243b2d6eaac9d00e0cd2e2623da5ea5f84bb751ad89e2ba40a04be7f7b794b92`; `6` files parsed (`0` failures), `67,535` bytes, `12,662` AST nodes, `142` findings, `6,049` `data-flow`, `816` `object-flow`, `392` `closure`, `192` `call-flow`, `21` `promise-ownership`, `16` `boundary`, `12` `event` relations (`10,265/10,265` view items). | Parent: `ev_f8b8d8b5d466e7e82237287e2cfa2fd0c5b5c123ec26ab784043d78479ab9b1e`<br>Summary: `ev_60ee2104d9cbc06da8e4cec18979ce5fe07229d7dbb94fcf7860a18507137ed2` |
| `REA-R05-02` | `analyze_javascript_application` (`detail: "summary"`) | `research_repos/lightweight-charts-5.2.0` (`7` TS files: `iseries-primitive.ts`, `ipane-primitive.ts`, `primitive-drawing-utils.ts`, `pane-primitive-wrapper.ts`, `series-primitive-wrapper.ts`, `primitive-wrapper-base.ts`, `pane-widget.ts`) | Artifact SHA-256 `6a26c4adafe9539017d004f611425bc9692a278cd45d4a41ffe381afd7b6c9d5`; `7` files parsed (`0` failures), `51,380` bytes, `6,055` AST nodes, `165` findings, `1,246` `data-flow`, `159` `object-flow`, `65` `closure`, `13` `call-flow` relations (`1,156/1,156` view items). | Parent: `ev_4d2d8fb26b4008bf9593160240c34dfb8c91e7aa3b035987b20fa54b51b5f33b`<br>Summary: `ev_c5a35abf18aade54d8c83afeefb77a927519dd577b51a815b4cc9e4d6808b9fa` |
| `REA-R05-03` | `trace_javascript_semantics` (`property: "anchors"`, `forward-influence`) | Parent `ev_f8b8d8b5d466e7e82237287e2cfa2fd0c5b5c123ec26ab784043d78479ab9b1e` (`ARCHIVE/frontend`) | `8` seed matches, `9` traversed nodes, `1` traversed relation across `6` functions in `3` modules (`SumiPrimitiveDrawingProvider.ts`, `drawingDomain.ts`, `drawingGeometry.ts`). | `ev_6041f2c6410aa12d1c2f8be00560f53d1fb39373f207e1a4b78f463b10fec90c` |
| `REA-R05-04` | `trace_javascript_semantics` (`property: "creationAnchor"`, `backward-provenance`) | Parent `ev_f8b8d8b5d466e7e82237287e2cfa2fd0c5b5c123ec26ab784043d78479ab9b1e` (`ARCHIVE/frontend`) | `13` seed matches, `16` traversed nodes, `3` traversed relations across `5` functions in `SumiPrimitiveDrawingProvider.ts` (`L193`, `L333`, `L341`, `L344`, `L346`, `L358`, `L359`), proving all write/reset transitions of the 2-click creation state machine. | `ev_01525e3b82c11b3635e8bc577a9984622642ea5163b9544cc609666328939ea8` |
| `REA-R05-05` | `trace_javascript_semantics` (`property: "paneIndex"`, `forward-influence`) | Parent `ev_f8b8d8b5d466e7e82237287e2cfa2fd0c5b5c123ec26ab784043d78479ab9b1e` (`ARCHIVE/frontend`) | `0` seed matches (`0` traversed nodes/relations) — negative control proving legacy Sumi drawings lacked multi-pane routing and hardcoded `paneId: 'price'` (`drawingDomain.ts:13, 119`). | `ev_8c1494cf58444be254a4adfa62316f6d52653317729f6b753a3bd6a77833fb7f` |
| `REA-R05-06` | `trace_javascript_semantics` (`property: "_paneViewsCache"`, `backward-provenance`) | Parent `ev_4d2d8fb26b4008bf9593160240c34dfb8c91e7aa3b035987b20fa54b51b5f33b` (`LWC v5.2.0`) | `1` seed match, `1` traversed node in `src/model/pane-primitive-wrapper.ts:95` (`PrimitiveWrapper.paneViews` reference-equality cache). | `ev_25a75e6cd7634e3e880f09d61b17a8904f94e2924348f97be3968729efbbd46d` |
| `REA-R05-07` | `trace_javascript_semantics` (`property: "hitTest"`, `forward-influence`) | Parent `ev_4d2d8fb26b4008bf9593160240c34dfb8c91e7aa3b035987b20fa54b51b5f33b` (`LWC v5.2.0`) | `1` seed match, `1` traversed node in `src/model/pane-primitive-wrapper.ts:103` (`PrimitiveWrapper.hitTest` delegating pane-local `(x, y)` coordinates to `_primitive.hitTest`). | `ev_98512b7e2e76cbaba58708b11ec527e51c4130b5b13870f12057b783dc66a13a` |

### Documented REA Static Analysis Limitations & Legacy Defects Discovered
- Extensionless TypeScript ESM specifiers (`./drawingDomain`, `./ipane-primitive`) remain `unresolved-semantic-module` under inert JS syntax parsing; all cross-module contracts were directly verified against the TypeScript sources listed above.
- **Five Legacy Sumi Defects Rectified in Rust (`src/drawings/`):**
  1. **Calendar-String Anchors & `250`-Day Extrapolation Cliff (`SumiPrimitiveDrawingProvider.ts:213–258`):** Legacy anchored by `YYYY-MM-DD` strings with weekend-only `countTradingDays` and failed (`null`) beyond `±250` bars or in historical whitespace. **Rust Fix:** Continuous `f64` logical `(bar_index, value)` anchors (`|bar_index| <= MAX_ABS_BAR_INDEX = 1.0e9`) map bijectively via `TimeScale::float_index_to_local_x` / `local_x_to_center_float_index`.
  2. **Rightward-Only Unclipped Rays (`drawingDomain.ts:78–79`, `drawingGeometry.ts:16–20`):** Legacy forbade `x1 <= x0` (`isRightwardRay`) and computed `y` at `x = right` without vertical clipping, exploding `y` to `±1e8` on steep slopes. **Rust Fix:** Full $360^\circ$ directional rays (`P0 != P1`) clipped analytically in `f64` pane-local space via unit-vector Liang-Barsky (`t ∈ [0, +∞)`) with `ClipEdge` boundary snapping against `[0, width] × [0, height]`.
  3. **Broken Logarithmic Body Drag (`SumiPrimitiveDrawingProvider.ts:302–306`):** Legacy rejected whole-drawing drags whenever `|Δprice_i - Δprice_0| > 1e-7`, causing every body drag in `Logarithmic` scale mode to fail and roll back. **Rust Fix:** Translate drag-start pane-local anchor coordinates `(x_i + dx, y_i + dy)` and invert via `PriceScale::local_y_to_price(..., base_value)`, preserving visual shape across `Normal`, `Logarithmic`, `Percentage`, `IndexedTo100`, and inverted scales.
  4. **Full-Bounding-Box Fibonacci Hit Hijack (`drawingGeometry.ts:58–61`):** Legacy hit-tested the entire Fibonacci bounding box as `distance = 0`, hijacking chart drag-panning across the price range. **Rust Fix:** Hit-test Fibonacci strictly on its anchor handles (`Priority 2/3`), visible horizontal level segments (`Priority 1`), and control diagonal (`Priority 1`); hit-test `Rectangle` interiors at `Priority 0` only when the rectangle is already selected and unlocked (`!locked`).
  5. **Positive-Only Single-Pane Restriction (`drawingDomain.ts:13, 84, 119`):** Legacy rejected `price <= 0` and non-`'price'` panes, preventing annotations on `MACD` (`value <= 0`) or `RSI`/`ATR`. **Rust Fix:** `DrawingPaneTarget` (`Main`, `Oscillator(Rsi | Macd | Atr)`) accepts any finite `f64` domain value within `|value| <= MAX_ABS_DRAWING_VALUE = 1.0e15` (`> 0` only when projecting onto an active `Logarithmic` axis).

---

## 2. Coordinate Space Contract & Analytical Geometry (`src/drawings/`)

All domain types, projections, clipping, hit-testing, magnet snapping, and undo/redo controller logic in `src/drawings/` use pure `f64` math with **zero OS or `egui` windowing dependencies** (`wasm32-unknown-unknown` verified).

### 2.1 Three-Space Separation
1. **Logical / Domain Space (`DrawingAnchor`):**
   - `bar_index: f64` — continuous bar-slot coordinate (`0.0` = center of bar `0`; `< 0.0` = historical whitespace; `>= total_bars as f64` = future `right_offset` whitespace; bounded by `MAX_ABS_BAR_INDEX = 1.0e9`).
   - `value: f64` — raw series value on the target pane (`Candle` price in raw currency/points on `Main`, independent of whether `Main` currently displays `Normal`, `Logarithmic`, `Percentage`, or `IndexedTo100`; raw oscillator value on `Rsi`, `Macd`, `Atr`; bounded by `MAX_ABS_DRAWING_VALUE = 1.0e15`).
2. **Pane-Local Space (`LocalPoint { x: f64, y: f64 }`):**
   - Projected via `PaneDrawingProjection`:
     $$x = \text{TimeScale::float\_index\_to\_local\_x}(\text{bar\_index}), \quad y = \text{PriceScale::price\_to\_local\_y}(\text{value}, \text{base\_value})$$
   - Inverted via `PaneDrawingProjection::local_to_anchor`:
     $$\text{bar\_index} = \text{TimeScale::local\_x\_to\_center\_float\_index}(x), \quad \text{value} = \text{PriceScale::local\_y\_to\_price}(y, \text{base\_value})$$
   - Clip rectangle is `[0.0, width] × [0.0, height]` where `(width, height) = (plot_rect.width, plot_rect.height)`.
3. **Screen Space (`egui::Pos2` at Paint Boundary Only):**
   - `screen_x = (plot_rect.left + local.x) as f32`, `screen_y = (plot_rect.top + local.y) as f32`.

### 2.2 Parametric Liang-Barsky Clipping (`clip_parametric_line`)
For $P(t) = P_0 + t \cdot (P_1 - P_0)$ over initial parameter interval $[t_0, t_1]$ ($[0, 1]$ for `TrendLine`, $[0, +\infty)$ for `Ray`) against box $[x_{\min}, x_{\max}] \times [y_{\min}, y_{\max}]$:
- **Semi-infinite rays (`t_max == +∞`):** Normalizes $(dx, dy)$ (using half-scaled $0.5 P_1 - 0.5 P_0$ when $P_1 - P_0$ overflows `f64::MAX`) to unit direction $(u_x, u_y)$ and evaluates $p_k \cdot s \le q_k$ with exact `p_k == 0.0` checks so sub-`1e-12` slopes never collapse to `0 px` or emit phantom border rays.
- **Finite segments (`[t_min, t_max]`):** Uses half-scaled differences when $P_1 - P_0$ overflows `f64::MAX` (`±1.39e308`), exact `p_k == 0.0` checks, and `ClipEdge::{Left, Right, Top, Bottom}` boundary snapping whenever `t_min` / `t_max` (or `s_min` / `s_max`) is tightened, guaranteeing exact border intersections without `0 * Inf = NaN` or catastrophic 53-bit cancellation.
- **Phase-Anchored Dashes (`paint_styled_segment_phased`):** Dashed and dotted strokes anchor dash phase to `raw_start` (or bar `0` for horizontal levels) rather than the clipped border point so panning never induces "marching ants" shimmer.

### 2.3 Core Drawing Primitives (`DrawingGeometry`)
1. **`HorizontalLine` (`1` anchor `[anchor]`):**
   - Horizontal line across `x ∈ [0, width]` at `y = price_to_local_y(anchor.value, base_value)` when `0 <= y <= height`, plus optional `LOCK` badge.
   - Handle positioned at `(clamp(x0, 14.0, width - 14.0), y)` when `width >= 28.0` (else `width * 0.5`) so off-screen horizontal anchors remain selectable/draggable inside the visible viewport.
2. **`TrendLine` (`2` anchors `[start, end]`):**
   - Finite segment clipped over $t \in [0, 1]$; handles at `P0` and `P1`.
3. **`Ray` (`2` anchors `[origin, through]`):**
   - Semi-infinite ray clipped over $t \in [0, +\infty)$ in any $360^\circ$ direction (`|through.bar_index - origin.bar_index| > 1e-9 || |through.value - origin.value| > 1e-12`); handles at `P0` (origin) and `P1` (through-point).
4. **`Rectangle` (`2` anchors `[corner_a, corner_b]`):**
   - Normalized box `[min(x0, x1), max(x0, x1)] × [min(y0, y1), max(y0, y1)]` (`LocalRect::from_corners(p0, p1)`), clipped to `[0, width] × [0, height]` for `egui::Mesh` translucent quad fill, plus 4 individually clipped border segments.
   - `4` deterministic corner handles (`HandlePart::RectCorner(0..4)`): `(a0.bar, a0.val)`, `(a1.bar, a0.val)`, `(a1.bar, a1.val)`, `(a0.bar, a1.val)`.
5. **`FibonacciRetracement` (`2` anchors `[start, end]`, `direction`, `extend_right`):**
   - Canonical ratios `FIBONACCI_RATIOS = [0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0]`.
   - Level value (`fibonacci_level_value`):
     $$v(r) = (1 - r) \cdot v_{\text{start}} + r \cdot v_{\text{end}}$$
     with `(v_start, v_end) = (start.value, end.value)` for `FibDirection::StartToEnd` and `(end.value, start.value)` for `FibDirection::EndToStart`.
   - Horizontal span `[x_left, x_right]` = `[min(x0, x1), width]` when `extend_right == true`, else `[min(x0, x1), max(x0, x1).max(min(x0, x1) + 24.0)]` intersected with `[0, width]`.
   - Alternating zone bands rendered via pre-multiplied `egui::Mesh::add_colored_rect`, dashed control segment `P0..P1`, crisp level lines, and priority-spaced (`11.5 px` minimum vertical clearance) `"0.618 (1284.50)"` labels.

---

## 3. Hit-Testing Hierarchy, OHLC Magnet & State Machine

### 3.1 LWC-Aligned Hit-Test Priority (`hit_test_pane_drawings`)
Evaluated across visible drawings on the active pane:
- **`HitPriority::SelectedHandle` (`3`):** Handle/corner of unlocked `selected_id` within `SELECTED_HANDLE_HIT_RADIUS_PX = 8.0 px`.
- **`HitPriority::Handle` (`2`):** Handle/corner of any visible unlocked drawing within `HANDLE_HIT_RADIUS_PX = 7.5 px`.
- **`HitPriority::Stroke` (`1`):** Clipped line/ray segment, horizontal line, rectangle border edge, or Fibonacci level/diagonal segment within `STROKE_HIT_TOLERANCE_PX = 6.0 px` (including locked drawings so users can click to select and unlock them).
- **`HitPriority::SelectedBody` (`0`):** Interior of `Rectangle` when `drawing.id == selected_id && !drawing.locked`.
- **Order-Independent Transitive Tie-Breaking:** Higher `HitPriority` wins; within equal priority, all candidates within `min_distance + 0.5 px` are treated as near-ties and resolved strictly by highest `z_order` (later index in `drawings`).

### 3.2 OHLC Magnet (`snap_to_visible_ohlc`)
- Active on `DrawingPaneTarget::Main` when `magnet_mode == MagnetMode::Ohlc` (holding `Ctrl`/`Cmd` temporarily toggles magnet relative to the toolbar setting).
- Searches visible candles whose `|x_candle - cursor_x| <= MAGNET_THRESHOLD_PX (12.0 px)` in `O(threshold / bar_spacing)` and checks `[open, high, low, close]` where `|y_field - cursor_y| <= 12.0 px`, selecting minimum `dx^2 + dy^2` with deterministic tie-breaking `(dist_sq, candle_index, field_rank)`.

### 3.3 Interaction State Machine & Chart Pan Arbitrage
- `DrawingInteractionPhase`:
  - `Idle`
  - `PlacingSingle { pane, preview_anchor }`
  - `PlacingTwoAnchor { tool, pane, first_anchor, first_local, preview_anchor, wait_for_second_press, second_press_active }` (supporting both 2-click placement and press-drag-release when drag distance `>= DRAG_COMMIT_DISTANCE_PX = 6.0 px`)
  - `DraggingHandle { drawing_id, pane, handle, start_local, grab_offset, drag_activated, original }`
  - `DraggingBody { drawing_id, pane, start_local, original_anchors_local, drag_activated, original }`
- **Pure-Click vs Edit-Drag Separation (`EDIT_DRAG_THRESHOLD_PX = 2.0 px`):** Handle and body gestures preserve initial `grab_offset` and do not mutate geometry or push `DrawingCommand::Modify` (preserving `redo_stack`) unless pointer displacement reaches `2.0 px`.
- **Locked-Drawing Passthrough:** Clicking a locked stroke selects the drawing (`selected_id = Some(id)`) while returning `PointerCaptureOutcome::PassThroughToViewport` so drag-panning over locked annotations remains completely unobstructed.
- **ID Reservation & Persistence Safety:** `PREVIEW_DRAWING_ID = u64::MAX` is reserved exclusively for live rubber-band previews; persisted stores enforce `0 < id < PREVIEW_DRAWING_ID`, deduplicate IDs via `HashSet`, cap deserialized entries at `MAX_PERSISTED_DRAWINGS = 4_096`, and compact IDs seamlessly across `drawings`, `undo_stack`, and `redo_stack` when `next_id` approaches `u64::MAX`.
- Bounded undo/redo stack (`MAX_UNDO_STEPS = 64`) records `DrawingCommand::{Create, Modify, Delete, ReplaceAll}` transitions cleanly on commit.

---

## 4. Adversarial `sumi-critic` Review & Remediation Log

Adversarial review was conducted by `sumi-critic` (`40ee59a2-90a9-4239-9a3d-46ee717e40b2`) against the initial R05 implementation. All **10 findings (`2 CRITICAL`, `4 MAJOR`, `4 MINOR`)** were remediated and locked in by regression tests prior to `sumi-auditor` verification:

| Finding ID | Severity | Defect Identified by `sumi-critic` | Root-Cause Fix & Regression Coverage |
| :--- | :---: | :--- | :--- |
| `CRITICAL-01` | `CRITICAL` | Clicking a handle without dragging snapped the anchor to cursor/magnet (`<= 8–12 px`) and pushed phantom `1–4 ULP` `Modify` commands, wiping `redo_stack`. | Added `EDIT_DRAG_THRESHOLD_PX = 2.0`, `grab_offset`, and `drag_activated` to `DraggingHandle` / `DraggingBody`, plus press-frame `pointer_move` skip in `src/chart.rs`. Verified by `critic_r05_regression_click_without_drag_preserves_geometry_and_redo_stack`. |
| `CRITICAL-02` | `CRITICAL` | Liang-Barsky `|p| <= 1e-12` tolerance emitted phantom border rays for near-vertical off-screen rays, collapsed tiny valid rays to `0 px`, and suffered `0 * Inf = NaN` / `t_min == t_max == 0.5` cancellation on `±1.39e308` spans. | Tightened `MAX_ABS_DRAWING_VALUE` to `1.0e15`, split `clip_parametric_line` into unit-vector ray clipping + half-scaled segment clipping with exact `p == 0.0` checks and `ClipEdge` boundary snapping. Verified by `critic_r05_regression_liang_barsky_no_phantom_edge_ray_tiny_ray_and_extreme_span`. |
| `MAJOR-01` | `MAJOR` | Selected locked `Rectangle` captured `SelectedBody` across its interior, blocking click-to-deselect and chart drag-panning. | Guarded `SelectedBody` with `is_selected && !projected.locked` in `hit_test.rs` and returned `PassThroughToViewport` on locked hits in `controller.rs`. Verified by `critic_r05_regression_locked_rectangle_deselect_pan_passthrough_and_transitive_ties` & `multipane_drawing_arbitrage_creates_drags_selects_locked_and_preserves_pan`. |
| `MAJOR-02` | `MAJOR` | Deserializing `id = u64::MAX` collided with `preview_drawing` (`u64::MAX`) and saturated `next_id`, causing duplicate `DrawingId`s. | Reserved `PREVIEW_DRAWING_ID = u64::MAX`, enforced `0 < id < PREVIEW_DRAWING_ID`, added `HashSet` deduplication, `MAX_PERSISTED_DRAWINGS = 4_096`, and `compact_ids()` with `undo_stack`/`redo_stack` remapping. Verified by `critic_r05_regression_u64_max_id_compaction_and_cross_pane_drag_hover_isolation`. |
| `MAJOR-03` | `MAJOR` | `"Demo Set"` wiped existing user drawings without Undo, pushed `8` separate `Create` commands, lost `locked = true` across `undo -> redo`, and enabled `"Clear"` when only locked drawings existed. | Replaced `"Demo Set"` population with atomic `DrawingStore::replace_all` (preserving `magnet_mode` and `locked = true` on `undo`/`redo`) and bound `"Clear"` to `has_unlocked_drawings()`. Verified in `app::tests`. |
| `MAJOR-04` | `MAJOR` | Moving the pointer across oscillator panes during an active `Main` drag leaked foreign-pane `hovered_hit` highlights and hover cursors. | Added early cross-pane isolation guard in `DrawingStore::pointer_move` when an interaction phase is active on another pane. Verified by `critic_r05_regression_u64_max_id_compaction_and_cross_pane_drag_hover_isolation`. |
| `MINOR-01` | `MINOR` | Dashed/dotted strokes restarted phase at the clipped viewport edge, causing "marching ants" shimmer during horizontal panning. | Implemented `paint_styled_segment_phased` in `src/chart.rs` anchored to unclipped `raw_start` (or bar `0` for horizontal levels). |
| `MINOR-02` | `MINOR` | `step_shared_kinetic_inertia` ran after pre-input autoscale, causing a 1-frame vertical range lag during kinetic coasting. | Moved `step_shared_kinetic_inertia` ahead of `conflation_factor` and pre-input autoscale in `render_workstation_chart_inner` (`src/chart.rs`). |
| `MINOR-03` | `MINOR` | Vertically compressed `FibonacciRetracement` levels drew overlapping text labels. | Added priority-ordered (`LABEL_PRIORITY_ORDER`: `0.0`, `1.0`, `0.5`, `0.618`, `0.382`, `0.236`, `0.786`) `11.5 px` vertical label collision avoidance in `src/chart.rs`. |
| `MINOR-04` | `MINOR` | Non-transitive `0.5 px` pairwise tie-breaking in `hit_test_pane_drawings`, spec label formatting (`1284.50`), and missing `fit_content` `100K` magnet benchmark. | Replaced pairwise tie-breaking with order-independent two-pass `min_dist + 0.5 px` Z-order tie-breaking in `hit_test.rs`, synced spec text, and added `ohlc_magnet_fit_all` telemetry in `examples/viewport_bench.rs`. |

---

## 5. Measured Release Performance & Binary REA Audit (`Milestone R05`)

### 5.1 Release Benchmark (`cargo run --release --example viewport_bench -- --r05`, Apple M1, `rustc 1.99.1`)
- **Pure `f64` Projection, Liang-Barsky Clipping, 4-Pane Hit-Test & OHLC Magnet (`25` Active Multi-Pane Drawings on `100,000` Bars):**
  - `project_drawing` (`25` drawings): **`2.459 µs` median** (`2.625 µs` P95)
  - `hit_test_pane_drawings` (`4` panes, `25` drawings): **`3.208 µs` median** (`3.417 µs` P95)
  - `snap_to_visible_ohlc` (`bar_spacing = 8.0 px`): **`0.125 µs` median** (`0.250 µs` P95)
  - `snap_to_visible_ohlc` (`100K` `fit_content`, `bar_spacing = 0.01178 px`): **`55.042 µs` median** (`66.375 µs` P95)
- **End-to-End 4-Pane + 6-Indicator + 25-Drawing Headless `egui` UI + Tessellation (`1280 × 800`, `2.0x` Retina):**
  - `100,000` bars standard (`bar_spacing = 8.0`, `Linear`, `1x`, `17,526` vertices): **`0.2100 ms` median** (`0.2327 ms` P95)
  - `100,000` bars dense (`bar_spacing = 1.2`, `Log`, `1x`, `70,668` vertices): **`0.5720 ms` median** (`0.6216 ms` P95)
  - `100,000` bars full-fit conflated (`bar_spacing = 0.01178`, `Log`, `32x`, `323,356` vertices): **`3.5207 ms` median** (`3.8042 ms` P95) — **PASSES `< 4.0 ms` `DECISION-RENDERER-002` hybrid threshold** (`42.3%` of `8.33 ms` `120 FPS` frame budget).

### 5.2 Live REA Binary Audit (`target/release/sumi-workstation`)
- **Artifact:** `/Users/mizuhara/workspace/sumi/target/release/sumi-workstation` (`16,623,888` bytes, Mach-O 64-bit `arm64`, UUID `383E1B48-D730-37AF-84AE-CF943E41E67B`, SHA-256 `59bb4d574f6f70bbd3102e0c3865cfba3d3ef7910fbdec6f220bf47b2a51f1ab`)
- **`REA-R05-08` (`inspect_rust_binary`, `detail: "summary"`):** `1` inferred `rustc` commit hash — Parent `ev_b7c8db4d920e8f946dc2f91cb8f272c3a9dafc63955d112a3e3ae34671a75edb`, Summary `ev_258699dd2d2f72da215a1a70714aaee09948e74ded7c3cb667c0675309bd41f1`
- **`REA-R05-09` (`inspect_macho`, `detail: "summary"`):** `1` entrypoint, `1` architecture (`arm64`), `30` load commands, `14` dylib dependencies, `324` imports, `4,802` exports, `5` segments — Parent `ev_a07daa400f148b6383b439e1c1cb7bd2eeac638a58cc7a2109e082998a3c3c78`, Summary `ev_cfc8717dfcd3df909426dafdfba350066b9f914a51e7ddfaa006ffdf5dc47048`
