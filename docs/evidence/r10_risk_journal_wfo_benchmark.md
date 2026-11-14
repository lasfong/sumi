# Milestone R10 — Vietnam Market Microstructure Risk Planner, On-Chart Risk-Reward Tool, Structured Trade Journal & Rolling Walk-Forward Portfolio Benchmark Evidence

- **Command:** `cargo run --release --example viewport_bench -- --r10`
- **Hardware:** `Apple M1` (`aarch64-macos`), `16.0 GiB` RAM
- **Toolchain & Profile:** `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (`--release`)
- **Active Stack:** Vietnam `HOSE / HNX / UPCOM / VN30F` Microstructure Risk Planner + Dual-Zone `RiskRewardBox` + `DEF-001` Causal `TradeJournalStore` (`SQN`, `Discipline %`, RFC-4180 CSV) + `K`-Fold Rolling/Anchored Walk-Forward (`Pardo WFE` + Stitched OOS Equity) + `8`/`13`-Asset Causal Portfolio Simulation

## 1. Risk Planner, Trade Journal, Walk-Forward & Multi-Asset Portfolio Throughput

| Case ID | Description | Input Units | Output Items | Median (`µs`) | Median (`ms`) | P95 (`µs`) | Max (`µs`) |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| `r10_hose_risk_planner_1k_evals` | 1,000 HOSE Risk Planner evaluations (±7% band, 3-tier tick snap, lot 100, T+2, all-in friction) | `1000` | `300` | **`101.12 µs`** | **`0.101 ms`** | `104.88 µs` | `111.67 µs` |
| `r10_journal_analytics_and_csv_500` | 500-entry DEF-001 causal Journal analytics (SQN, Discipline %, Attribution) + RFC-4180 CSV export | `500` | `74263` | **`601.12 µs`** | **`0.601 ms`** | `674.08 µs` | `784.54 µs` |
| `r10_walk_forward_4fold_rolling_fpt_1d` | 4-fold Rolling Walk-Forward (12-variant IS sweep -> stitched OOS curve & Pardo WFE) on FPT 1D | `180` | `3` | **`108.38 µs`** | **`0.108 ms`** | `112.33 µs` | `120.33 µs` |
| `r10_walk_forward_4fold_anchored_15m` | 4-fold Anchored Walk-Forward across 6,667 15m bars (100K 1m rollup) | `6667` | `116` | **`5957.92 µs`** | **`5.958 ms`** | `6319.96 µs` | `6339.88 µs` |
| `r10_portfolio_vn30_8_equal_weight` | 8-symbol VN30 cash equity portfolio simulation (EqualWeight 1/N + diversification DD attribution) | `8` | `180` | **`106.00 µs`** | **`0.106 ms`** | `112.08 µs` | `117.92 µs` |
| `r10_portfolio_universe_13_causal_inv_vol` | 13-feed multi-asset universe portfolio simulation (past-only CausalInverseVolatility(20)) | `13` | `10487` | **`2855.42 µs`** | **`2.855 ms`** | `3039.08 µs` | `3051.08 µs` |

## 2. End-to-End Workstation Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)

| Bars | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | UI Median (`ms`) | Tessellate Median (`ms`) | Total CPU Median (`ms`) | Total CPU P95 (`ms`) | `< 4.0ms` Hybrid Gate | 120 FPS (`8.33ms`) |
| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: | :---: |
| `180` | `r10_full_ui_vn_risk_journal_rr_box` (Full Workstation UI: FPT 1D + HOSE Risk Planner + selected dual-zone RiskRewardBox + Journal KPIs) | `Linear` | `1x` | `83` | `81` | `17970` | `61869` | `0.187` | `0.089` | **`0.276`** | `0.331` | `PASS (<4ms)` | `PASS` |
| `4801` | `r10_full_ui_wfo_stitched_oos_5panes` (Full Workstation UI: 100K 1m->15m + 5 panes (Main+RSI+MACD+ATR+Stitched WFO OOS Equity) + WFO sub-tab) | `Linear` | `1x` | `94` | `93` | `22486` | `73647` | `0.259` | `0.109` | **`0.369`** | `0.431` | `PASS (<4ms)` | `PASS` |
| `180` | `r10_full_ui_portfolio_equity_5panes` (Full Workstation UI: FPT 1D + 5 panes + mapped 8-symbol VN30 Inv-Vol Portfolio Equity Curve + WFO sub-tab) | `Linear` | `1x` | `97` | `96` | `21540` | `70956` | `0.243` | `0.106` | **`0.349`** | `0.369` | `PASS (<4ms)` | `PASS` |
