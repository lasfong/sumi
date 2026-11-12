# Milestone R07 — Causal Bar Replay & Paper Trading Execution Engine Benchmark Evidence

- **Command:** `cargo run --release --example viewport_bench -- --r07`
- **Hardware:** `Apple M1` (`aarch64-macos`), `16.0 GiB` RAM
- **Toolchain & Profile:** `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (`--release`)
- **Active Stack:** `100,000` `1m` Base Bars (`VN30F1M`) + Causal Replay Cut (`90,000..=95,000`) + Paper Trading Portfolio (`Market`/`Limit`/`SL`/`TP` Brackets + Execution Fill Markers) + 4 Panes (`Main` + `RSI(14)` + `MACD(12,26,9)` + `ATR(14)`) + 6 Indicators + 25 Causal Drawings + Replay Dock & Right Trading Panel

## 1. Causal Replay Slicing, Step-Forward Matching, 5,000-Bar Simulation & Bit-Exact Rewind

| Case ID | Description | Base Bars | Output Bars / Fills | Median (`µs`) | P95 (`µs`) | Max (`µs`) |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: |
| `replay_materialize_95k_1m_to_15m` | Causal zero-lookahead materialize (`95,001` 1m -> `15m`) | `95001` | `6334` | **`1344.67 µs`** | `1511.50 µs` | `1735.46 µs` |
| `replay_step_15m_plus_order_matching` | `1x 15m` ActiveBar step + intrabar OHLC bracket/limit matching | `15` | `30` | **`1.25 µs`** | `1.29 µs` | `1.38 µs` |
| `trading_sim_5000_1m_bars_full` | 5,000-bar `1m` causal paper-trading run (~65 orders + SL/TP brackets) | `5001` | `30` | **`304.17 µs`** | `339.75 µs` | `396.38 µs` |
| `trading_causal_rewind_4500_bars` | Bit-exact command-log `rewind_to` (`90,000..=94,500` rebuild) | `4501` | `27` | **`256.71 µs`** | `276.83 µs` | `329.08 µs` |

## 2. End-to-End 4-Pane + 6-Indicator + 25-Drawing + Replay + Paper Trading Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)

| Bars | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | UI Median (`ms`) | Tessellate Median (`ms`) | Total CPU Median (`ms`) | Total CPU P95 (`ms`) | `< 4.0ms` Hybrid Gate | 120 FPS (`8.33ms`) |
| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: | :---: |
| `95001` | `r07_replay_1m_recent_4pane_25draw_trading` (95K/100K 1m Replay + 4 panes + 6 ind + 25 draw + Position/SL/TP/Fills (bar_spacing=8.0)) | `Linear` | `1x` | `141` | `140` | `15822` | `46095` | `0.114` | `0.059` | **`0.174`** | `0.192` | `PASS (<4ms)` | `PASS` |
| `6334` | `r07_replay_15m_cut_scrubber_4pane_25draw_trading` (95K/100K -> 15m Replay + Cut Scrubber + 4 panes + 6 ind + 25 draw + Fills (bar_spacing=9.0)) | `Linear` | `1x` | `124` | `123` | `18892` | `52764` | `0.122` | `0.074` | **`0.196`** | `0.224` | `PASS (<4ms)` | `PASS` |
| `95001` | `r07_replay_1m_fit_conflated_4pane_25draw_trading` (95K/100K 1m full-fit conflated + Replay Frontier + Trading Overlays (Logarithmic)) | `Log` | `32x` | `95001` | `2968` | `401860` | `1572324` | `2.096` | `1.832` | **`3.923`** | `4.256` | `PASS (<4ms)` | `PASS` |
| `4541` | `r07_full_workstation_ui_dock_ticket_4pane` (Full SumiWorkstationApp (Toolbar + Replay Dock + Right Trading Panel + 4 Panes + Overlays)) | `Linear` | `1x` | `103` | `101` | `21942` | `68808` | `0.192` | `0.099` | **`0.291`** | `0.373` | `PASS (<4ms)` | `PASS` |
