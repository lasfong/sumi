# Milestone R17 — Practice Scoreboard, Session Debrief Coaching, 1-Click Scanner->Replay, Multi-Strategy `0%` Overlay & Advanced Analytics Benchmark Evidence

- **Command:** `cargo run --release --example viewport_bench -- --r17`
- **Hardware / OS:** `Apple M1` | `16.0 GiB` RAM | `macos` `aarch64` | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (`release`)
- **Viewport:** `1280x800` logical (`2560x1600` Retina @ `2.0x`)

## 1. Practice Coaching, Multi-Strategy `0%` Overlay & Advanced Analytics Kernel Throughput

| Case ID | Description | Input Units | Output Items | Median (`µs`) | Median (`ms`) | P95 (`µs`) | Max (`µs`) |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| `r17_multistrategy_pct_comparison_btc_15m` | UTC bar-close aligned 0% Multi-Strategy Comparison (7 presets + B&H + SPY Bench) on BTCUSDT 15m | `1344` | `9` | **`1253.96 µs`** | **`1.254 ms`** | `1347.58 µs` | `1348.21 µs` |
| `r17_advanced_tearsheet_analytics_and_manifest` | OutlierImpact + DrawdownPeriods + UTC Year & Signal-Bar MarketRegime slices + MultiPhase Tertiles + SplitMix64 RunManifest | `1344` | `9` | **`252.67 µs`** | **`0.253 ms`** | `274.25 µs` | `341.25 µs` |
| `r17_debrief_coaching_and_practice_store_json_roundtrip` | Tamper-proof SL Adherence Debrief + Causal DecisionLog + PracticeSessionStore JSON encode/decode & SplitMix64 verify | `6` | `4` | **`46.29 µs`** | **`0.046 ms`** | `49.00 µs` | `54.71 µs` |

## 2. End-to-End Workstation R17 Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)

| Bars (Total) | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | Mean (`ms`) | Median (`ms`) | P99 (`ms`) | `< 4.0ms` Gate |
| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: |
| `75` | `r17_fpt_practice_scoreboard_and_debrief_modal` (1x1 FPT 1D Replay + PracticeScoreboard + Causal Decision Notes + SessionDebriefModal) | `Linear` | `1x` | `75` | `75` | `23766` | `76626` | **`0.336`** | **`0.331`** | `0.399` | `PASS (<4ms)` |
| `1344` | `r17_btc_15m_multistrategy_pct_and_analytics_slices` (1x1 BTCUSDT 15m + MultiStrategyPct 0% (9 curves) + Strategy Lab Analytics Slices) | `Linear` | `1x` | `1321` | `1319` | `121914` | `426282` | **`1.327`** | **`1.305`** | `1.499` | `PASS (<4ms)` |
| `960` | `r17_vn30f_scanner_1click_120b_and_saved_sessions` (1x1 VN30F1M 5m +120b BlindPractice + Scanner Watchlist 1-Click + Saved Practice/Scan Docks) | `Linear` | `1x` | `62` | `61` | `23906` | `84297` | **`0.461`** | **`0.453`** | `0.558` | `PASS (<4ms)` |
| `100000` | `r17_100k_conflated_multistrategy_pct_and_slices` (100,000-bar conflated chart + 9-curve MultiStrategyPct single-Mesh Equity + Analytics Slices) | `Linear` | `32x` | `97101` | `3033` | `51992` | `120822` | **`1.516`** | **`1.473`** | `1.880` | `PASS (<4ms)` |
| `200000` | `r17_quad_2x2_50k_practice_and_multistrategy_stress` (Quad 2x2 (4x50k = 200k bars) conflated stress + PracticeScoreboard + MultiStrategyPct) | `Linear` | `64x` | `193444` | `3020` | `99234` | `223704` | **`2.082`** | **`2.036`** | `2.422` | `PASS (<4ms)` |
