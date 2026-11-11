# Milestone R03 — Reproducible Viewport & `egui::Painter` Benchmark Evidence

- **Command:** `cargo run --release --example viewport_bench`
- **Hardware:** `Apple M1` (`aarch64-macos`), `16.0 GiB` RAM
- **Toolchain & Profile:** `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (`--release`)
- **Target Resolution:** `1280×800` logical px (`2560×1600` physical Retina px at `pixels_per_point = 2.0`)
- **Measurement Scope:** Measures deterministic CPU viewport math, `egui` UI pass (`render_candlestick_chart`), and full `epaint` CPU mesh tessellation (`Context::tessellate`). Metal GPU swapchain vsync presentation is measured separately via native GUI capture.

## 1. Dataset Generation, Memory Footprint & Pure Viewport Math (`TimeScale` + `PriceScale` + Culling)

| Dataset Bars | Gen Time (`ms`) | Series Heap (`MiB`) | Process RSS (`MiB`) | Pure Zoom+Cull+Autoscale+Project Median (`µs`) | P95 (`µs`) |
| ---: | ---: | ---: | ---: | ---: | ---: |
| `10000` | `2.57` | `0.84` | `7.94` | `0.79 µs` | `0.96 µs` |
| `50000` | `9.77` | `4.20` | `12.17` | `0.75 µs` | `0.92 µs` |
| `100000` | `17.76` | `8.39` | `20.62` | `0.67 µs` | `0.79 µs` |

## 2. End-to-End `egui::Painter` Build + `epaint` Tessellation Across 10K / 50K / 100K Bars

| Bars | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | UI Median (`ms`) | Tessellate Median (`ms`) | Total CPU Median (`ms`) | Total CPU P95 (`ms`) | 60 FPS (`16.67ms`) | 120 FPS (`8.33ms`) |
| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: | :---: |
| `10000` | `standard_8px` (Standard trading view (bar_spacing=8.0, ~150 visible bars)) | `Linear` | `1x` | `137` | `136` | `6012` | `20418` | `0.039` | `0.055` | **`0.094`** | `0.124` | `PASS` | `PASS` |
| `10000` | `dense_1_2px_log` (Dense swing view (bar_spacing=1.2, ~980 visible bars, Log)) | `Log` | `1x` | `971` | `969` | `32704` | `120504` | `0.147` | `0.266` | **`0.413`** | `0.511` | `PASS` | `PASS` |
| `10000` | `fit_all_conflated` (Full-dataset fit WITH DPR-aware power-of-2 conflation) | `Log` | `4x` | `10000` | `2499` | `81116` | `302814` | `0.352` | `0.692` | **`1.050`** | `1.281` | `PASS` | `PASS` |
| `10000` | `fit_all_unconflated` (Full-dataset fit WITHOUT conflation (raw egui::Painter stress)) | `Log` | `1x` | `10000` | `9996` | `320996` | `1202418` | `1.311` | `3.336` | **`4.664`** | `5.291` | `PASS` | `PASS` |
| `50000` | `standard_8px` (Standard trading view (bar_spacing=8.0, ~150 visible bars)) | `Linear` | `1x` | `137` | `136` | `6048` | `20436` | `0.039` | `0.054` | **`0.093`** | `0.104` | `PASS` | `PASS` |
| `50000` | `dense_1_2px_log` (Dense swing view (bar_spacing=1.2, ~980 visible bars, Log)) | `Log` | `1x` | `971` | `969` | `32316` | `119562` | `0.145` | `0.265` | **`0.409`** | `0.479` | `PASS` | `PASS` |
| `50000` | `fit_all_conflated` (Full-dataset fit WITH DPR-aware power-of-2 conflation) | `Log` | `16x` | `50000` | `3124` | `99836` | `372744` | `0.488` | `0.877` | **`1.364`** | `1.661` | `PASS` | `PASS` |
| `50000` | `fit_all_unconflated` (Full-dataset fit WITHOUT conflation (raw egui::Painter stress)) | `Log` | `1x` | `50000` | `49979` | `1384008` | `5188452` | `5.912` | `15.162` | **`21.126`** | `21.443` | `EXCEEDS` | `EXCEEDS` |
| `100000` | `standard_8px` (Standard trading view (bar_spacing=8.0, ~150 visible bars)) | `Linear` | `1x` | `137` | `136` | `5996` | `20322` | `0.042` | `0.060` | **`0.102`** | `0.900` | `PASS` | `PASS` |
| `100000` | `dense_1_2px_log` (Dense swing view (bar_spacing=1.2, ~980 visible bars, Log)) | `Log` | `1x` | `971` | `969` | `32844` | `120750` | `0.149` | `0.269` | **`0.418`** | `0.543` | `PASS` | `PASS` |
| `100000` | `fit_all_conflated` (Full-dataset fit WITH DPR-aware power-of-2 conflation) | `Log` | `32x` | `100000` | `3124` | `92996` | `346914` | `0.587` | `0.870` | **`1.472`** | `1.921` | `PASS` | `PASS` |
| `100000` | `fit_all_unconflated` (Full-dataset fit WITHOUT conflation (raw egui::Painter stress)) | `Log` | `1x` | `100000` | `99958` | `2530144` | `9486282` | `11.331` | `28.257` | **`39.570`** | `40.783` | `EXCEEDS` | `EXCEEDS` |
| `100000` | `multipane_3x_dense` (3 stacked panes sharing 1 TimeScale on 100K bars (bar_spacing=1.2, ~2,940 total visible bars)) | `Linear+Log` | `1x` | `2913` | `2907` | `95996` | `356106` | `0.340` | `0.799` | **`1.143`** | `1.341` | `PASS` | `PASS` |
