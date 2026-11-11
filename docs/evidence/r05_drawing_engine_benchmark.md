# Milestone R05 — Interactive Drawing & Multi-Pane Workstation Benchmark Evidence

- **Command:** `cargo run --release --example viewport_bench -- --r05`
- **Hardware:** `Apple M1` (`aarch64-macos`), `16.0 GiB` RAM
- **Toolchain & Profile:** `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (`--release`)
- **Active Stack:** Main Candlestick + Volume + `SMA(20)` + `EMA(50)` + `BB(20, 2.0)` + 3 Stacked Oscillator Panes (`RSI(14)`, `MACD(12, 26, 9)`, `ATR(14)`) + **25 Active Multi-Pane Drawings** (`5` `HorizontalLine`, `5` `TrendLine`, `5` $360^\circ$ `Ray`, `5` `Rectangle`, `5` `FibonacciRetracement`)

## 1. Pure `f64` Projection, Liang-Barsky Clipping, 4-Pane Hit-Testing & OHLC Magnet Throughput

| Dataset Bars | Active Drawings | Project + Clip 25 Drawings (`µs`) | 4-Pane Hit-Test 25 Drawings (`µs`) | OHLC Magnet `bar_spacing = 8.0` (`µs`) | OHLC Magnet `fit_content` (`µs`) |
| ---: | ---: | ---: | ---: | ---: | ---: |
| `10000` | `25` | **`4.08 µs`** (P95 `4.21 µs`) | **`4.75 µs`** (P95 `5.29 µs`) | **`0.17 µs`** (P95 `0.29 µs`) | **`6.83 µs`** (P95 `9.21 µs`) |
| `50000` | `25` | **`2.62 µs`** (P95 `2.71 µs`) | **`3.38 µs`** (P95 `3.54 µs`) | **`0.12 µs`** (P95 `0.21 µs`) | **`27.54 µs`** (P95 `33.25 µs`) |
| `100000` | `25` | **`2.46 µs`** (P95 `2.62 µs`) | **`3.21 µs`** (P95 `3.42 µs`) | **`0.12 µs`** (P95 `0.25 µs`) | **`55.04 µs`** (P95 `66.38 µs`) |

## 2. End-to-End 4-Pane + 6-Indicator + 25-Drawing Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)

| Bars | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | UI Median (`ms`) | Tessellate Median (`ms`) | Total CPU Median (`ms`) | Total CPU P95 (`ms`) | `< 4.0ms` Hybrid Gate | 120 FPS (`8.33ms`) |
| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: | :---: |
| `10000` | `r05_standard_8px_4pane_25draw` (4 panes + 6 indicators + 25 drawings (bar_spacing=8.0, Linear)) | `Linear` | `1x` | `137` | `136` | `17622` | `56955` | `0.132` | `0.095` | **`0.229`** | `0.247` | `PASS (<4ms)` | `PASS` |
| `10000` | `r05_dense_1_2px_4pane_25draw` (4 panes + 6 indicators + 25 drawings (bar_spacing=1.2, Log)) | `Log` | `1x` | `971` | `969` | `70304` | `234510` | `0.343` | `0.232` | **`0.575`** | `0.644` | `PASS (<4ms)` | `PASS` |
| `10000` | `r05_fit_all_conflated_4pane_25draw` (4 panes + 6 indicators + 25 drawings (100% full-fit conflated, Log)) | `Log` | `4x` | `10000` | `2499` | `233468` | `859200` | `1.712` | `0.848` | **`2.559`** | `2.783` | `PASS (<4ms)` | `PASS` |
| `50000` | `r05_standard_8px_4pane_25draw` (4 panes + 6 indicators + 25 drawings (bar_spacing=8.0, Linear)) | `Linear` | `1x` | `137` | `136` | `17690` | `56901` | `0.124` | `0.089` | **`0.213`** | `0.245` | `PASS (<4ms)` | `PASS` |
| `50000` | `r05_dense_1_2px_4pane_25draw` (4 panes + 6 indicators + 25 drawings (bar_spacing=1.2, Log)) | `Log` | `1x` | `971` | `969` | `70684` | `235158` | `0.345` | `0.243` | **`0.588`** | `0.667` | `PASS (<4ms)` | `PASS` |
| `50000` | `r05_fit_all_conflated_4pane_25draw` (4 panes + 6 indicators + 25 drawings (100% full-fit conflated, Log)) | `Log` | `16x` | `50000` | `3124` | `334508` | `1281324` | `1.718` | `1.230` | **`2.953`** | `3.180` | `PASS (<4ms)` | `PASS` |
| `100000` | `r05_standard_8px_4pane_25draw` (4 panes + 6 indicators + 25 drawings (bar_spacing=8.0, Linear)) | `Linear` | `1x` | `137` | `136` | `17526` | `56685` | `0.121` | `0.089` | **`0.210`** | `0.233` | `PASS (<4ms)` | `PASS` |
| `100000` | `r05_dense_1_2px_4pane_25draw` (4 panes + 6 indicators + 25 drawings (bar_spacing=1.2, Log)) | `Log` | `1x` | `971` | `969` | `70668` | `234936` | `0.342` | `0.230` | **`0.572`** | `0.622` | `PASS (<4ms)` | `PASS` |
| `100000` | `r05_fit_all_conflated_4pane_25draw` (4 panes + 6 indicators + 25 drawings (100% full-fit conflated, Log)) | `Log` | `32x` | `100000` | `3124` | `323356` | `1243260` | `2.263` | `1.247` | **`3.521`** | `3.804` | `PASS (<4ms)` | `PASS` |
