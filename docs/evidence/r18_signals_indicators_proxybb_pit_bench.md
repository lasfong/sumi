# Milestone R18 — 101-Signal Catalog, Extended Indicators, Multi-Horizon ProxyBB & Point-in-Time Universe Benchmark Evidence

- **Command:** `cargo run --release --example viewport_bench -- --r18`
- **Hardware / OS:** `Apple M1` | `16.0 GiB` RAM | `macos` `aarch64` | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (`release`)
- **Viewport:** `1280x800` logical (`2560x1600` Retina @ `2.0x`)

## 1. Candlestick Geometry, Extended Indicators, 101-Signal Matrix, Multi-Horizon ProxyBB & PIT Universe Throughput

| Case ID | Description | Input Units | Output Items | Median (`µs`) | Median (`ms`) | P95 (`µs`) | Max (`µs`) |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| `b1_candlestick_patterns_and_scores_10k` | B1 `CandleGeometry` + `14` patterns + `candle_score` (`10,000` bars) | `10000` | `1461` | **`292.17 µs`** | **`0.292 ms`** | `317.54 µs` | `427.25 µs` |
| `b2_all_8_extended_indicators_10k` | B2 `ADX+Stoch+SuperTrend+PSAR+Keltner+CCI+MFI+VolSMA` (`10,000` bars) | `10000` | `8` | **`934.67 µs`** | **`0.935 ms`** | `1058.12 µs` | `1107.33 µs` |
| `b3_precomputed_catalog_101_signals_2500b` | B3 `PrecomputedCatalogMatrix` (`101` signals + `12` confirmed divergences, `2,500` bars) | `2500` | `101` | **`5991.96 µs`** | **`5.992 ms`** | `6135.38 µs` | `6153.38 µs` |
| `b4_proxy_bb_6_horizons_10k` | B4 `compute_proxy_bb_series` (`T03..T200` 6 horizons, `10,000` bars) | `10000` | `6` | **`1036.88 µs`** | **`1.037 ms`** | `1136.96 µs` | `1170.92 µs` |
| `b4_market_bb_and_pit_portfolio_vn30` | B4 `MarketBBAggregator` (`PIT`) + `run_portfolio_backtest_with_universe` (`VN30-PIT`) | `13` | `180` | **`2096.08 µs`** | **`2.096 ms`** | `2234.25 µs` | `2268.50 µs` |

## 2. End-to-End Workstation R18 Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)

| Bars (Total) | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | Mean (`ms`) | Median (`ms`) | P99 (`ms`) | `< 4.0ms` Gate |
| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: |
| `180` | `r18_b1_b2_overlays_patterns_3osc_fpt` (FPT 1D + SuperTrend + PSAR + Keltner + VolSMA + Patterns + ADX + Stoch + MFI) | `Linear` | `1x` | `137` | `136` | `20134` | `71517` | **`0.231`** | **`0.227`** | `0.360` | `PASS (<4ms)` |
| `360` | `r18_b3_catalog_101_modal_cci_rsi_macd` (VN30F1M 15m + 101-Signal Catalog Modal (Divergence) + CCI + RSI + MACD + Strategy Lab) | `Linear` | `1x` | `96` | `95` | `24894` | `76743` | **`0.357`** | **`0.353`** | `0.502` | `PASS (<4ms)` |
| `180` | `r18_b4_proxybb_marketbb_pit_portfolio` (SSI 1D + ProxyBB Sub-Pane + Equity (PIT Portfolio) + Breadth (Multi-Horizon ProxyBB & MarketBB)) | `Linear` | `1x` | `54` | `52` | `22926` | `74721` | **`0.430`** | **`0.423`** | `0.607` | `PASS (<4ms)` |
| `100000` | `r18_100k_bars_all_extended_panes_conflated` (100,000 Bars Conflated + SuperTrend + PSAR + Keltner + ADX + Stoch + MFI + ProxyBB) | `Linear` | `32x` | `99832` | `3118` | `189662` | `515172` | **`0.955`** | **`0.925`** | `1.340` | `PASS (<4ms)` |
| `2899` | `r18_2x2_grid_extended_indicators_and_proxybb` (2x2 Synchronized Grid + Extended Overlays + ProxyBB + PIT Breadth Dock) | `Linear` | `1x` | `151` | `146` | `22304` | `74802` | **`0.356`** | **`0.347`** | `0.537` | `PASS (<4ms)` |
