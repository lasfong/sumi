# LEGACY SUMI vs LIGHTWEIGHT CHARTS COORDINATE COMPARISON

- **Original Session:** `SESSION-R02-20261110T230337+0700`
- **Audited & Updated In:** `SESSION-R03-20261111`
- **Compared Implementations:**
  1. **Lightweight Charts v5.2.0** (`research_repos/lightweight-charts-5.2.0/src/`) — Canonical reference math.
  2. **Archived TypeScript Sumi** (`ARCHIVE/frontend/src/components/chart/ChartKernel.ts`, `PaneManager.ts`) — Wrapped `lightweight-charts` via npm with `rightOffset: 12`, `barSpacing: 8`, `minBarSpacing: 1`.
  3. **Session 01 Rust Prototype (`ViewportMapping` in `src/model.rs` & `SumiWorkstationApp` in `src/app.rs`)** — Minimal linear prototype replaced in Session R03 by `src/viewport/`.

---

## 1. Architectural & Algorithmic Comparison Matrix

| Dimension | LWC v5.2.0 (`VERIFIED`) | Archived TS Sumi (`ARCHIVE/`) | Session 01 Rust Prototype (`ViewportMapping`) | Session R03 Rust Viewport (`src/viewport/`) |
|-----------|-------------------------|-------------------------------|-----------------------------------------------|---------------------------------------------|
| **Multi-Pane Topology** | 1 shared `TimeScale` on `ChartModel`; per-pane `left`/`right`/`overlay` `PriceScale`s | `PaneManager` synchronized multiple LWC instances via `subscribeVisibleLogicalRangeChange` (fragile re-entrancy guards) | Single coupled `ViewportMapping` constructed ad-hoc per frame | 1 shared `TimeScale` + independent `PriceScale` instances per pane/axis (`right` main + `overlay` volume) bridged via `PaneViewport` |
| **Coordinate Spaces** | Pane-local `[0, width) × [0, height)` per `<canvas>` | LWC DOM canvas + custom SVG/Canvas overlay | Mixed screen `plot_rect` `f32` directly into linear formulas | Explicit 3-tier separation: **Logical/Data** (`i64`/`f64`) ↔ **Pane-Local** (`f64`) ↔ **Screen** (`egui::Pos2` `f32`) |
| **Numeric Precision** | `f64` (`number`) | `f64` (`number`) | `f32` (`egui::Rect`) + `f64` prices | Pure `f64` across all scales & transformations; `f32` only at final `egui::Pos2` emission |
| **Anchor Reference** | Right-anchored (`baseIndex + rightOffset`) | Delegated to LWC (`rightOffset: 12`) | Left-anchored slice `[start..end]` with uniform `width / visible_len` | Right-anchored (`base_index + right_offset`, default `right_offset = 12.0` future bars) |
| **Cursor-Anchored Zoom** | `zoom(pointX, scale)` preserving `_coordinateToFloatIndex(pointX)` | Delegated to LWC | **None** — wheel changed `visible_count ± 10` anchored only at right edge | Exact LWC cursor-anchored `TimeScale::zoom` locking float index under pointer |
| **Horizontal Panning** | Sub-pixel `startScroll`/`scrollTo` + bar-space `KineticAnimation` (`k = 0.997`, `MaxStartDelay = 50ms`) | Delegated to LWC | Integer-bar `drag_accum_px` stepping 1 bar at a time, zero inertia | Continuous sub-pixel `start_scroll`/`scroll_to` + `KineticAnimation` inertial decay |
| **Price Scale Modes** | `Normal`, `Logarithmic` (dynamic `LogFormula`), `Percentage`, `IndexedTo100`, `invertScale` | `Normal` & `Logarithmic` via LWC | Linear only (`price_min..price_max` padded in data space) | `Normal`, `Logarithmic` (dynamic `LogFormula`), `Percentage`, `IndexedTo100`, `invert_scale`, manual vertical axis drag-scale & pan |
| **Large Dataset Handling (10K–100K bars)** | `visibleStrictRange` O(1) index slice + power-of-2 conflation | Capped around ~500–2,000 bars per symbol fetch | Sliced `candles[start..end]`, no sub-pixel conflation | O(1) `visible_data_range` culling + automatic sub-pixel OHLCV bucket conflation when `bar_spacing < 1.0` |

---

## 2. Critical Migration Hazard: Sign Convention of `right_offset` (`VERIFIED`)

> [!WARNING]
> **Opposite Sign Convention Between Session 01 `SumiWorkstationApp::right_offset` and LWC `TimeScale::right_offset`:**
> - In Session 01 [`src/app.rs`](file:///Users/mizuhara/workspace/sumi/src/app.rs), `pub right_offset: usize` counted **bars scrolled backward into the past** (`end = total.saturating_sub(self.right_offset)`), so `right_offset > 0` meant viewing historical bars and future whitespace was impossible (`usize >= 0`).
> - In LWC v5.2.0 (`src/model/time-scale.ts:326, 557`) and Session R03 `crate::viewport::TimeScale`, `right_offset: f64` is signed:
>   - `right_offset > 0.0` = **future whitespace** to the right of the latest bar (e.g., `12.0` bars of breathing room for live price action and drawings).
>   - `right_offset == 0.0` = latest bar (`base_index`) sits flush at the right edge of the plot.
>   - `right_offset < 0.0` = viewport is scrolled **backward into historical data** (`-(legacy_right_offset as f64)`).

Any adapter or state migration from Session 01 must negate the legacy offset (`lwc_right_offset = -(legacy_right_offset as f64)`), never pass it with positive sign.

---

## 3. Deprecation & Coexistence Strategy

Per Product Owner workspace rules (no file deletion without explicit permission and zero regressions on existing tests):
1. `ViewportMapping` in [`src/model.rs`](file:///Users/mizuhara/workspace/sumi/src/model.rs) is retained with `#[deprecated]` attributes and its original unit tests preserved intact.
2. Active chart rendering, hit-testing, crosshair snapping, axis ticks, zoom, pan, and kinetic inertia in [`src/chart.rs`](file:///Users/mizuhara/workspace/sumi/src/chart.rs) and [`src/app.rs`](file:///Users/mizuhara/workspace/sumi/src/app.rs) run exclusively on the modular `crate::viewport` engine (`TimeScale` + `PriceScale` + `PaneViewport` + `KineticAnimation`).
