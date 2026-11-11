# Milestone R06 — CSV/JSON OHLCV Data Loader & Multi-Timeframe Aggregation Benchmark Evidence

- **Command:** `cargo run --release --example viewport_bench -- --r06`
- **Hardware:** `Apple M1` (`aarch64-macos`), `16.0 GiB` RAM
- **Toolchain & Profile:** `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (`--release`)
- **Active Stack:** Pure-Rust Streaming RFC-4180 CSV & RFC-8259 JSON Loaders + `1m..1W` Upward Aggregation + `25` Timestamp-Locked Multi-Pane Drawings + 4-Pane 6-Indicator Adaptive Intraday/Daily Workstation

## 1. Streaming RFC-4180 CSV & RFC-8259 JSON Loader Throughput (`10K`, `50K`, `100K` + Real Archive Fixtures)

| Loader Case | Format | Raw Rows | Parsed Bars | Payload (`MiB`) | Parse Median (`ms`) | Parse P95 (`ms`) | Throughput (`MiB/s`) |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| `csv_iso8601_10k` | `CSV (ISO-8601 1m)` | `10000` | `10000` | `0.55` | **`4.341 ms`** | `4.571 ms` | **`127.4 MiB/s`** |
| `json_kline_10k` | `JSON (Unix-ms kline)` | `10000` | `10000` | `0.56` | **`4.300 ms`** | `4.507 ms` | **`130.1 MiB/s`** |
| `csv_iso8601_50k` | `CSV (ISO-8601 1m)` | `50000` | `50000` | `2.77` | **`22.390 ms`** | `22.735 ms` | **`123.5 MiB/s`** |
| `json_kline_50k` | `JSON (Unix-ms kline)` | `50000` | `50000` | `2.91` | **`22.036 ms`** | `23.158 ms` | **`132.0 MiB/s`** |
| `csv_iso8601_100k` | `CSV (ISO-8601 1m)` | `100000` | `100000` | `5.53` | **`45.303 ms`** | `45.717 ms` | **`122.1 MiB/s`** |
| `json_kline_100k` | `JSON (Unix-ms kline)` | `100000` | `100000` | `5.90` | **`43.703 ms`** | `44.243 ms` | **`134.9 MiB/s`** |
| `archive_cafef_vnindex_csv` | `CSV (CafeF Multi-Ticker)` | `2179` | `2179` | `0.25` | **`1.302 ms`** | `1.466 ms` | **`191.1 MiB/s`** |
| `archive_spy_5col_csv` | `CSV (5-Col NYSE SPY)` | `8280` | `8280` | `0.32` | **`3.130 ms`** | `3.307 ms` | **`103.7 MiB/s`** |

## 2. `100,000` `1m` Bars Multi-Timeframe Aggregation & 25-Drawing Exact Timestamp Remapping

| Source (`100K`) | Target TF | Output Bars | Rollup Median (`ms`) | Rollup P95 (`ms`) | 25-Drawing Timestamp Remap (`µs`) |
| :--- | :--- | ---: | ---: | ---: | ---: |
| `100000 1m` | **`5m`** | `20000` | **`2.613 ms`** | `2.787 ms` | **`4.29 µs`** (P95 `4.38 µs`) |
| `100000 1m` | **`15m`** | `6667` | **`1.142 ms`** | `1.284 ms` | **`3.71 µs`** (P95 `3.75 µs`) |
| `100000 1m` | **`1h`** | `1961` | **`0.636 ms`** | `0.702 ms` | **`3.25 µs`** (P95 `3.25 µs`) |
| `100000 1m` | **`4h`** | `785` | **`0.504 ms`** | `0.547 ms` | **`3.04 µs`** (P95 `3.04 µs`) |
| `100000 1m` | **`1D`** | `393` | **`0.454 ms`** | `0.484 ms` | **`2.83 µs`** (P95 `2.88 µs`) |
| `100000 1m` | **`1W`** | `79` | **`0.502 ms`** | `0.526 ms` | **`0.75 µs`** (P95 `0.79 µs`) |

## 3. End-to-End 4-Pane + 6-Indicator + 25-Drawing Intraday & Aggregated Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)

| Bars | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | UI Median (`ms`) | Tessellate Median (`ms`) | Total CPU Median (`ms`) | Total CPU P95 (`ms`) | `< 4.0ms` Hybrid Gate | 120 FPS (`8.33ms`) |
| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: | :---: |
| `100000` | `r06_100k_1m_recent_4pane_25draw` (100K 1m intraday window (bar_spacing=8.0, HH:MM/Day axis, 4 panes + 6 ind + 25 draw)) | `Linear` | `1x` | `137` | `136` | `14464` | `46788` | `0.112` | `0.073` | **`0.185`** | `0.223` | `PASS (<4ms)` | `PASS` |
| `100000` | `r06_100k_1m_fit_conflated_4pane_25draw` (100K 1m full-fit conflated (Logarithmic, 4 panes + 6 ind + 25 draw)) | `Log` | `32x` | `100000` | `3124` | `418136` | `1648776` | `2.206` | `1.546` | **`3.761`** | `4.112` | `PASS (<4ms)` | `PASS` |
| `6667` | `r06_100k_to_15m_recent_4pane_25draw` (100K 1m aggregated to 15m + 25 remapped drawings (bar_spacing=8.0, Normal)) | `Linear` | `1x` | `137` | `136` | `16864` | `53976` | `0.123` | `0.084` | **`0.207`** | `0.242` | `PASS (<4ms)` | `PASS` |
| `393` | `r06_100k_to_1d_fit_4pane_25draw` (100K 1m aggregated to 1D + 25 remapped drawings (fit_all, Logarithmic)) | `Log` | `1x` | `393` | `393` | `31136` | `104496` | `0.181` | `0.124` | **`0.305`** | `0.334` | `PASS (<4ms)` | `PASS` |
