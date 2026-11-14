# Milestone R09 — Deterministic Strategy Backtest Lab, Synchronized Equity/Drawdown Sub-Pane & Quantitative Tearsheet Benchmark Evidence

- **Command:** `cargo run --release --example viewport_bench -- --r09`
- **Hardware:** `Apple M1` (`aarch64-macos`), `16.0 GiB` RAM
- **Toolchain & Profile:** `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (`--release`)
- **Active Stack:** `100,000` `1m` Base Bars + Causal `run_backtest_with_options` (`Open[T+1]` fills, pessimistic `SL`-before-`TP`, post-close ATR trail, `70/30` `IS/OOS` split) + `12`-Grid Parameter Sweep + `13`-Symbol Universe Battle + 5 Synchronized Panes (`Main + RSI + MACD + ATR + Equity` 2-`Mesh` Baseline) + `O(log T)` Trade Links

## 1. Strategy Backtest Engine, Parameter Sweep & Multi-Symbol Battle Throughput (`< 15.0 ms` 100K Gate)

| Case ID | Description | Input Bars | Output Trades / Rows | Median (`µs`) | Median (`ms`) | P95 (`µs`) | Max (`µs`) |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| `r09_backtest_100k_sma_ema_oos` | 100,000-bar 1m SmaEmaTrendCross + 70/30 IS/OOS + SeriesBlockIndex + Tearsheet (<15ms gate) | `100000` | `2863` | **`6318.29 µs`** | **`6.318 ms`** | `9071.33 µs` | `10702.88 µs` |
| `r09_backtest_100k_donchian_atr_oos` | 100,000-bar 1m O(N) monotonic-deque DonchianAtrBreakout + ATR trail + 70/30 IS/OOS | `100000` | `1636` | **`7832.17 µs`** | **`7.832 ms`** | `8325.00 µs` | `8521.83 µs` |
| `r09_sweep_12_grid_fpt_1d` | 12-variant deterministic parameter sweep + IS/OOS robustness on FPT 1D | `2160` | `12` | **`77.29 µs`** | **`0.077 ms`** | `95.38 µs` | `133.88 µs` |
| `r09_6_preset_battle_btc_4h` | All 6 strategy presets battle on BTCUSDT 4h with IS/OOS split | `900` | `6` | **`44.12 µs`** | **`0.044 ms`** | `44.29 µs` | `49.83 µs` |
| `r09_universe_13_symbol_battle` | 13-symbol multi-asset Watchlist Universe strategy battle (VN30 + CafeF + SPY + BTC + VN30F1M) | `20797` | `13` | **`639.92 µs`** | **`0.640 ms`** | `747.25 µs` | `1067.12 µs` |

## 2. End-to-End Workstation Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)

| Bars | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | UI Median (`ms`) | Tessellate Median (`ms`) | Total CPU Median (`ms`) | Total CPU P95 (`ms`) | `< 4.0ms` Hybrid Gate | 120 FPS (`8.33ms`) |
| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: | :---: |
| `95000` | `r09_95k_1m_recent_5pane_equity` (95K 1m causal bars, 5 active panes (Main+RSI+MACD+ATR+Equity) + Trade Links + Alerts + Trading) | `Linear` | `1x` | `137` | `136` | `16614` | `47040` | `0.121` | `0.069` | **`0.190`** | `0.246` | `PASS (<4ms)` | `PASS` |
| `95000` | `r09_95k_1m_full_fit_equity_conflated` (95K 1m full-fit conflated (>=32x), Main + Equity SeriesBlockIndex M4 baseline Mesh + Trade Links) | `Log` | `32x` | `95000` | `2968` | `131236` | `205710` | `1.158` | `0.191` | **`1.355`** | `1.935` | `PASS (<4ms)` | `PASS` |
| `4801` | `r09_workstation_1280x800_tearsheet_equity` (Full 1280x800 workstation UI (Watchlist + Main + RSI + Equity + Strategy Lab Tearsheet & IS/OOS)) | `Linear` | `1x` | `64` | `62` | `21240` | `68217` | `0.305` | `0.104` | **`0.412`** | `0.506` | `PASS (<4ms)` | `PASS` |
| `4801` | `r09_workstation_1280x800_sweep_battle` (Full 1280x800 workstation UI (Main + MACD + Equity + Strategy Lab 12-Grid Sweep & 6-Preset Battle)) | `Linear` | `1x` | `96` | `95` | `23994` | `74679` | `0.262` | `0.108` | **`0.371`** | `0.448` | `PASS (<4ms)` | `PASS` |
