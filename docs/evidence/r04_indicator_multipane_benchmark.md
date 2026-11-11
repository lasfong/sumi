# Milestone R04 — Indicator Engine & 4-Pane Workstation Benchmark Evidence

- **Command:** `cargo run --release --example viewport_bench -- --r04`
- **Hardware:** `Apple M1` (`aarch64-macos`), `16.0 GiB` RAM
- **Toolchain & Profile:** `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (`--release`)
- **Active Stack:** Main Candlestick + Volume + `SMA(20)` + `EMA(50)` + `BB(20, 2.0)` + 3 Stacked Oscillator Panes (`RSI(14)`, `MACD(12, 26, 9)`, `ATR(14)`) = **4 Panes, 10 Concurrent Series**

## 1. Cold All-6 Indicator Computation vs Warm Dirty-Cache Sync

| Dataset Bars | Cold All-6 Compute Median (`ms`) | Cold All-6 P95 (`ms`) | Warm Pan/Zoom Cache Sync (`µs`) | Indicator Cache Heap (`MiB`) |
| ---: | ---: | ---: | ---: | ---: |
| `10000` | **`0.956 ms`** | `1.092 ms` | **`0.041 µs`** | `2.05 MiB` |
| `50000` | **`4.733 ms`** | `4.879 ms` | **`0.041 µs`** | `10.25 MiB` |
| `100000` | **`9.626 ms`** | `9.746 ms` | **`0.041 µs`** | `20.50 MiB` |

## 2. End-to-End 4-Pane `egui::Painter` Build + `epaint` Tessellation (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)

| Bars | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | UI Median (`ms`) | Tessellate Median (`ms`) | Total CPU Median (`ms`) | Total CPU P95 (`ms`) | `< 4.0ms` Hybrid Gate | 120 FPS (`8.33ms`) |
| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: | :---: |
| `10000` | `r04_4pane_std_8px` (4 panes (Main+SMA+EMA+BB + RSI + MACD + ATR), bar_spacing=8.0) | `Linear` | `1x` | `137` | `136` | `11368` | `36324` | `0.092` | `0.044` | **`0.136`** | `0.152` | `PASS (<4ms)` | `PASS` |
| `10000` | `r04_4pane_dense_1_2px` (4 panes + all 6 indicators, dense swing view (bar_spacing=1.2, Log)) | `Log` | `1x` | `971` | `969` | `63940` | `213210` | `0.306` | `0.189` | **`0.495`** | `0.547` | `PASS (<4ms)` | `PASS` |
| `10000` | `r04_4pane_fit_conflated` (4 panes + all 6 indicators, full-dataset fit WITH bucket-aligned conflation) | `Log` | `4x` | `10000` | `2499` | `227228` | `838122` | `1.602` | `0.778` | **`2.387`** | `2.541` | `PASS (<4ms)` | `PASS` |
| `50000` | `r04_4pane_std_8px` (4 panes (Main+SMA+EMA+BB + RSI + MACD + ATR), bar_spacing=8.0) | `Linear` | `1x` | `137` | `136` | `11364` | `36198` | `0.091` | `0.044` | **`0.135`** | `0.152` | `PASS (<4ms)` | `PASS` |
| `50000` | `r04_4pane_dense_1_2px` (4 panes + all 6 indicators, dense swing view (bar_spacing=1.2, Log)) | `Log` | `1x` | `971` | `969` | `64032` | `213408` | `0.308` | `0.207` | **`0.515`** | `0.560` | `PASS (<4ms)` | `PASS` |
| `50000` | `r04_4pane_fit_conflated` (4 panes + all 6 indicators, full-dataset fit WITH bucket-aligned conflation) | `Log` | `16x` | `50000` | `3124` | `328328` | `1260336` | `1.558` | `1.164` | **`2.735`** | `2.946` | `PASS (<4ms)` | `PASS` |
| `100000` | `r04_4pane_std_8px` (4 panes (Main+SMA+EMA+BB + RSI + MACD + ATR), bar_spacing=8.0) | `Linear` | `1x` | `137` | `136` | `11392` | `36216` | `0.091` | `0.043` | **`0.134`** | `0.155` | `PASS (<4ms)` | `PASS` |
| `100000` | `r04_4pane_dense_1_2px` (4 panes + all 6 indicators, dense swing view (bar_spacing=1.2, Log)) | `Log` | `1x` | `971` | `969` | `63972` | `213138` | `0.306` | `0.191` | **`0.497`** | `0.548` | `PASS (<4ms)` | `PASS` |
| `100000` | `r04_4pane_fit_conflated` (4 panes + all 6 indicators, full-dataset fit WITH bucket-aligned conflation) | `Log` | `32x` | `100000` | `3124` | `317200` | `1222308` | `1.945` | `1.180` | **`3.141`** | `3.323` | `PASS (<4ms)` | `PASS` |
| `100000` | `r04_4pane_resized_sep` (4 panes + all 6 indicators after interactive separator stretch transfer) | `Linear` | `1x` | `137` | `136` | `11476` | `36414` | `0.092` | `0.043` | **`0.135`** | `0.153` | `PASS (<4ms)` | `PASS` |
