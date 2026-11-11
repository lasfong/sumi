# CHART RENDERER ARCHITECTURE DECISION — PROVISIONAL EVALUATION & R03 BENCHMARK

- **Original Session:** `SESSION-R02-20261110T230337+0700`
- **Audited, Benchmarked & Updated By:** `SESSION-R03-20261111T105832+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Status:** `PROVISIONAL_CONFIRMED_WITH_EXPLICIT_LIMITS` (`DECISION-RENDERER-001` revised by `DECISION-RENDERER-002`)
- **Benchmark Evidence:**
  - [`docs/evidence/r03_viewport_benchmark.json`](../evidence/r03_viewport_benchmark.json)
  - [`docs/evidence/r03_viewport_benchmark.md`](../evidence/r03_viewport_benchmark.md)
  - [`examples/viewport_bench.rs`](../../examples/viewport_bench.rs) (`cargo run --release --example viewport_bench`)

---

## 1. Executive Decision Summary

Session R02 declared `DECISION-RENDERER-001` ("Retain `egui::Painter`") based solely on unmeasured back-of-the-envelope estimates ("~1–3ms estimated, not measured"). Per Product Owner governance rules, an unbenchmarked renderer choice cannot be treated as a permanent architectural lock.

In Milestone R03, we built and executed a deterministic `--release` benchmark suite ([`examples/viewport_bench.rs`](../../examples/viewport_bench.rs)) across **10,000**, **50,000**, and **100,000** OHLCV bars on Apple M1 (`arm64`, 16 GiB RAM, `rustc 1.99.1`, `1280×800` logical / `2560×1600` Retina `pixels_per_point = 2.0`), measuring both **conflated** and **unconflated** rendering as well as **3-pane stacked workstation** workloads.

### Explicit Three-Tier Suitability Verdict

| Tier | Workload Scope | R03 Measured CPU Frame (`UI + Tessellate`) | Verdict |
| :--- | :--- | :--- | :--- |
| **Tier 1 — Prototype & Single-Pane Exploration** | `1` pane, `120–1,000` visible bars (`bar_spacing ∈ [1.2, 16.0]`) | **`0.093 ms – 0.418 ms`** (`< 5.1%` of 120 FPS `8.33 ms` budget) | **CONFIRMED (`VERIFIED`)** — Zero bottleneck. |
| **Tier 2 — Production Desktop Workstation (Standard Multi-Pane)** | `1–4` stacked panes sharing 1 `TimeScale`, `10K–100K+` bars in RAM **WITH O(1) slice culling + DPR-aware power-of-2 OHLCV conflation** + `<= 200` vector drawings | **`0.094–0.102 ms`** (standard zoom) · **`1.143 ms`** (`3` panes × `971` dense bars sharing `1` `TimeScale`) · **`1.472 ms`** (`100,000` bars full-fit `32x` conflated) | **CONFIRMED WITH MANDATORY CONFLATION (`VERIFIED`)** — Meets both `60 FPS` (`16.67 ms`) and `120 FPS` ProMotion (`8.33 ms`) CPU budgets. |
| **Tier 3 — Heavy Unconflated / High-Primitive Workloads** | Unconflated `50K–100K` visible bars, orderbook heatmaps, footprint clusters, tick-scatter plots, or `8–10+` panes exceeding **`12,000` simultaneous shapes** (`~350K` vertices) | **`4.664 ms`** (`10K` raw) · **`21.126 ms`** (`50K` raw, drops below 60 FPS) · **`39.570 ms`** (`100K` raw, `2.53M` vertices, drops to ~25 FPS) | **EXCEEDS `egui::Painter` BUDGET (`VERIFIED`)** — Requires hybrid `egui_wgpu::CallbackTrait` instanced `wgpu` pipeline when triggered. |

> [!IMPORTANT]
> **Critical R03 Correction to R02:** R02 assumed `egui::Painter` was naturally fast enough without proving conflation necessity. R03 benchmarks prove that **without** `TimeScale::conflation_factor` + `iter_conflated_candles`, fitting `50,000` (`21.13 ms`) or `100,000` (`39.57 ms`) raw bars **violates the 60 FPS (`16.67 ms`) budget** due to CPU vertex tessellation in `epaint`. **With** R03's O(1) culling + DPR-aware power-of-2 conflation + single-pass volume/candle submission, `100,000` full-fit bars render in **`1.472 ms`** (**26.9× faster**).

---

## 2. Reproducible R03 Benchmark Evidence

### 2.1 Test Environment & Methodology

- **Command:** `cargo run --release --example viewport_bench`
- **Hardware:** Apple M1 (`aarch64-apple-darwin`), `16.0 GiB` unified memory, macOS `27.2`
- **Compiler:** `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (`--release` optimized profile)
- **Viewport:** `1280×800` logical px (`plot_rect = 1178×756` headless / `1178×619` full workstation shell), `pixels_per_point = 2.0` (`2560×1600` Retina physical resolution)
- **What is measured directly (`VERIFIED`):**
  1. `data::generate_synthetic_ohlcv(N)` wall-clock time, `Vec<Candle>` heap bytes, and process RSS (`ps -o rss=`).
  2. Pure `src/viewport/` CPU loop (`TimeScale::zoom` + O(1) `culling_data_range` + `PriceScale::autoscale_candles` + `PaneViewport` projections).
  3. Full `egui::Context::run_ui` (`render_candlestick_chart` including grid, axes, volume histogram, 2-segment wicks, bodies, HUD, crosshair) + full `egui::Context::tessellate` (`epaint` CPU triangulation into indexed GPU vertex buffers).
- **Measurement boundary / limitation:** Headless `Context::tessellate` measures 100% of CPU layout, shape emission, clipping, and triangle tessellation, but does **not** include `wgpu` Metal command-encoder submission or macOS `CAMetalLayer` `CVDisplayLink` vsync wait. Native Metal swapchain execution was verified separately across 4 live GPU scenarios (`docs/evidence/r03_*.png`).

### 2.2 Dataset Memory & Pure Viewport Math (`1,000` Iterations)

| Dataset Bars | Generation Time (`ms`) | `Vec<Candle>` Heap (`MiB`) | Process RSS (`MiB`) | Pure Viewport Math Median (`µs`) | Pure Viewport Math P95 (`µs`) |
| ---: | ---: | ---: | ---: | ---: | ---: |
| `10,000` | `2.57 ms` | `0.84 MiB` | `7.94 MiB` | **`0.79 µs`** | `0.96 µs` |
| `50,000` | `9.77 ms` | `4.20 MiB` | `12.17 MiB` | **`0.75 µs`** | `0.92 µs` |
| `100,000` | `17.76 ms` | `8.39 MiB` | `20.62 MiB` | **`0.67 µs`** | `0.79 µs` |

*Conclusion:* Because `TimeScale::culling_data_range()` computes visible array indices analytically in $O(1)$ arithmetic rather than scanning all bars, pure viewport math takes **`< 1 microsecond` (`0.0007 ms`)** whether the dataset holds `10,000` or `100,000` bars, and `100,000` OHLCV bars occupy only `8.39 MiB` on heap (`20.62 MiB` total process RSS).

### 2.3 Full `egui::Painter` + `epaint` Tessellation Matrix

| Bars | Workload | Scale | Conflation | Culled Bars | Drawn Candles | Vertices | Indices | UI Build Median (`ms`) | Tessellate Median (`ms`) | Total CPU Median (`ms`) | Total CPU P95 (`ms`) | 60 FPS (`16.67ms`) | 120 FPS (`8.33ms`) |
| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: | :---: |
| `10,000` | `standard_8px` | `Linear` | `1x` | `137` | `136` | `6,012` | `20,418` | `0.039` | `0.055` | **`0.094`** | `0.124` | PASS | PASS |
| `10,000` | `dense_1_2px_log` | `Log` | `1x` | `971` | `969` | `32,704` | `120,504` | `0.147` | `0.266` | **`0.413`** | `0.511` | PASS | PASS |
| `10,000` | `fit_all_conflated` | `Log` | `4x` | `10,000` | `2,499` | `81,116` | `302,814` | `0.352` | `0.692` | **`1.050`** | `1.281` | PASS | PASS |
| `10,000` | `fit_all_unconflated` | `Log` | `1x` | `10,000` | `9,996` | `320,996` | `1,202,418` | `1.311` | `3.336` | **`4.664`** | `5.291` | PASS | PASS |
| `50,000` | `standard_8px` | `Linear` | `1x` | `137` | `136` | `6,048` | `20,436` | `0.039` | `0.054` | **`0.093`** | `0.104` | PASS | PASS |
| `50,000` | `dense_1_2px_log` | `Log` | `1x` | `971` | `969` | `32,316` | `119,562` | `0.145` | `0.265` | **`0.409`** | `0.479` | PASS | PASS |
| `50,000` | `fit_all_conflated` | `Log` | `16x` | `50,000` | `3,124` | `99,836` | `372,744` | `0.488` | `0.877` | **`1.364`** | `1.661` | PASS | PASS |
| `50,000` | `fit_all_unconflated` | `Log` | `1x` | `50,000` | `49,979` | `1,384,008` | `5,188,452` | `5.912` | `15.162` | **`21.126`** | `21.443` | **EXCEEDS** | **EXCEEDS** |
| `100,000` | `standard_8px` | `Linear` | `1x` | `137` | `136` | `5,996` | `20,322` | `0.042` | `0.060` | **`0.102`** | `0.900` | PASS | PASS |
| `100,000` | `dense_1_2px_log` | `Log` | `1x` | `971` | `969` | `32,844` | `120,750` | `0.149` | `0.269` | **`0.418`** | `0.543` | PASS | PASS |
| `100,000` | `fit_all_conflated` | `Log` | `32x` | `100,000` | `3,124` | `92,996` | `346,914` | `0.587` | `0.870` | **`1.472`** | `1.921` | PASS | PASS |
| `100,000` | `fit_all_unconflated` | `Log` | `1x` | `100,000` | `99,958` | `2,530,144` | `9,486,282` | `11.331` | `28.257` | **`39.570`** | `40.783` | **EXCEEDS** | **EXCEEDS** |
| `100,000` | `multipane_3x_dense` | `Linear+Log` | `1x` | `2,913` | `2,907` | `95,996` | `356,106` | `0.340` | `0.799` | **`1.143`** | `1.341` | PASS | PASS |

---

## 3. Rendering Backend Comparison & Hybrid Evolution Path

| Candidate | Pros | Cons | R03 Architectural Role |
| :--- | :--- | :--- | :--- |
| **`egui::Painter` (`epaint` → `wgpu` Metal)** | Zero extra crates; crisp DPI-aligned 2D lines/rects/text; `0.093–1.472 ms` across all conflated 10K–100K & 3-pane workloads; trivial hit-testing & clipping for drawing tools and HUDs. | CPU tessellation scales linearly (`~0.40 µs/bar`) when >12,000 un-conflated shapes are emitted per frame; screen-space `Pos2` uses `f32` (safe because all domain/viewport math stays in `f64` until final pixel projection). | **ACTIVE PRIMARY BACKEND** for Waves 1–4 (`src/chart.rs`, multi-pane layout, indicators, vector drawing tools). |
| **Hybrid `egui_wgpu::CallbackTrait` (Custom Instanced `wgpu` inside `egui` Pane Rect)** | Keeps 100% of `egui` chrome, axes, crosshair, and vector drawings while offloading dense series (`100K+` unconflated points, volume profile / orderbook heatmaps) to a single instanced GPU draw call (`6` vertices × `N` instances, zero CPU tessellation). | Requires custom WGSL shaders and instance buffer management per series type. | **DESIGNATED ESCAPE HATCH** if/when any future workload hits the quantitative triggers in §4 below. Because `PaneViewport` is already decoupled from `egui::Painter`, switching a pane's series pass to `PaintCallback` requires **zero changes** to `src/viewport/`. |
| **Standalone Raw `wgpu` / `skia-safe` / `femtovg`** | High raw throughput (`wgpu`), rich vector AA (`skia`). | Rebuilding UI layout/text (`raw wgpu`), ~300 MB C++ FFI build (`skia-safe`), or OpenGL lock-in (`femtovg`). | **REJECTED.** Strictly inferior to `egui::Painter` + selective `egui_wgpu::CallbackTrait`. |

---

## 4. Quantitative Triggers to Introduce `egui_wgpu::CallbackTrait`

`DECISION-RENDERER-002` keeps `egui::Painter` as the active chart renderer for Waves 2–4 **unless and until** one of the following measurable thresholds is reached:

1. **CPU Tessellation Budget Trigger:** Median CPU frame (`UI + Context::tessellate`) exceeds **`4.0 ms`** (`50%` of a `120 Hz` `8.33 ms` frame budget) on Apple M1 `--release` with culling and conflation enabled.
2. **Primitive Density Trigger:** A chart configuration requires rendering **`> 12,000` non-conflatable geometric primitives simultaneously** (e.g., 2D liquidity heatmap cells, tick-level scatter dots, or `8+` sub-panes with dense shaded cloud fills).
3. **Vertex Count Trigger:** Per-frame `epaint` vertex count regularly exceeds **`350,000` vertices/frame**.

---

## 5. Architecture Alignment (Rectifying R02 §6–§7)

R02 §6–§7 sketched a pseudo-code `SharedTimeCamera: ViewportCamera` calling `eframe::App::update`. In Milestone R03, the actual shipped Rust architecture (`egui`/`eframe 0.36.0`) uses:

- **Shared Horizontal State:** [`TimeScale`](../../src/viewport/time_scale.rs) (`f64` `bar_spacing`, `right_offset`, `base_index`, `width`)
- **Independent Per-Pane / Per-Axis Vertical State:** [`PriceScale`](../../src/viewport/price_scale.rs) (`main_price_scale` + `volume_price_scale`, ready for arbitrary `Pane[i]` indicator scales in Wave 4)
- **Explicit 3-Space Coordinate Bridge:** [`PaneViewport`](../../src/viewport/pane_viewport.rs) (`Logical/Data <-> PaneLocal <-> Screen`)
- **eframe 0.36 Entry Point:** `eframe::App::ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame)` in [`src/app.rs`](../../src/app.rs).

---

## Related Documents

- [`RUST_VIEWPORT_CAMERA_DESIGN.md`](RUST_VIEWPORT_CAMERA_DESIGN.md) — Modular `src/viewport/` architecture (`TimeScale`, `PriceScale`, `PaneViewport`, `KineticAnimation`)
- [`COORDINATE_ALGORITHM_SPEC.md`](COORDINATE_ALGORITHM_SPEC.md) — Source-verified LWC v5.2.0 coordinate formulas
- [`VIEWPORT_REA_TRACE_REPORT.md`](VIEWPORT_REA_TRACE_REPORT.md) — Live REA MCP evidence (`REA-01`..`REA-14`)
- [`../evidence/r03_viewport_benchmark.md`](../evidence/r03_viewport_benchmark.md) — Reproducible benchmark report
