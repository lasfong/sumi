# Milestone R08 — Multi-Symbol Watchlist Scanner, Causal Alert Engine & `Cmd+K` Command Palette Benchmark Evidence

- **Command:** `cargo run --release --example viewport_bench -- --r08`
- **Hardware:** `Apple M1` (`aarch64-macos`), `16.0 GiB` RAM
- **Toolchain & Profile:** `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (`--release`)
- **Active Stack:** `13`-Symbol Multi-Asset Watchlist Universe (`VN30` + `CafeF` + `SPY` + `BTCUSDT` + `VN30F1M`) + `100,000` `1m` Base Bars + Causal `AlertEngine` (`PriceLevel`, `RSI(14)`, `MACD(12,26,9)`, `DrawingLine`) + `Cmd+K` Command Palette + 4 Panes + 6 Indicators + 25 Drawings + Paper Trading

## 1. Multi-Symbol Scanner, Causal Alert Evaluation/Rewind & Command Palette Throughput

| Case ID | Description | Input Bars / Items | Output Rows / Events | Median (`µs`) | P95 (`µs`) | Max (`µs`) |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: |
| `scanner_13_symbols_watchlist_and_signals` | 13-symbol multi-asset Watchlist snapshot + 80-bar Historical Signal scan + sort | `20797` | `147` | **`1799.08 µs`** | `2156.42 µs` | `2357.96 µs` |
| `alerts_eval_5000_1m_bars_5_rules` | Causal evaluation of 5 rules (`Price`×2 + `RSI` + `MACD` + `DrawingLine`) on 95K `1m` series | `5001` | `256` | **`3108.50 µs`** | `4076.79 µs` | `4477.38 µs` |
| `alerts_causal_rewind_rearm_once` | Causal Replay rewind (`95,000 -> 92,000`) re-arming `Once` alerts & rebuilding trigger log | `2001` | `187` | **`2916.12 µs`** | `3928.00 µs` | `4417.88 µs` |
| `command_palette_fuzzy_rank_cycle` | `Cmd+K` candidate generation & fuzzy ranking across symbol, timeframe, indicator & quick-alert queries | `42` | `3` | **`13.71 µs`** | `14.33 µs` | `27.29 µs` |

## 2. End-to-End Workstation Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)

| Bars | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | UI Median (`ms`) | Tessellate Median (`ms`) | Total CPU Median (`ms`) | Total CPU P95 (`ms`) | `< 4.0ms` Hybrid Gate | 120 FPS (`8.33ms`) |
| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: | :---: |
| `95001` | `r08_chart_1m_recent_4pane_25draw_trading_alerts` (95K 1m Replay + 4 panes + 6 ind + 25 draw + Trading + Active/Triggered Alerts (bar_spacing=8.0)) | `Linear` | `1x` | `141` | `140` | `16274` | `47241` | `0.117` | `0.064` | **`0.182`** | `0.208` | `PASS (<4ms)` | `PASS` |
| `95001` | `r08_chart_1m_fit_conflated_4pane_trading_alerts` (95K 1m full-fit conflated + Replay + Trading + Batched Alert Diamonds & Lines (Logarithmic)) | `Log` | `32x` | `95001` | `2968` | `298492` | `458022` | `2.566` | `0.460` | **`3.067`** | `3.896` | `PASS (<4ms)` | `PASS` |
| `4543` | `r08_full_workstation_watchlist_alerts_4pane` (Full SumiWorkstationApp (Left 13-Sym Watchlist + Right Alerts Dock + Replay + 4 Panes + 100K Feed)) | `Linear` | `1x` | `69` | `68` | `22646` | `72249` | `0.297` | `0.111` | **`0.407`** | `0.602` | `PASS (<4ms)` | `PASS` |
| `4543` | `r08_full_workstation_cmd_k_modal_open` (Full SumiWorkstationApp + Floating `Cmd+K` Command Palette Modal (`alert 1295`) over 4-Pane Chart) | `Linear` | `1x` | `102` | `101` | `23968` | `75060` | `0.186` | `0.110` | **`0.296`** | `0.409` | `PASS (<4ms)` | `PASS` |
