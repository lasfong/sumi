# Milestone R15 — Explainable Signal Catalog, Causal Confluence, Drawing Inspector & Monte Carlo Benchmark Evidence

- **Command:** `cargo run --release --example viewport_bench -- --r15`
- **Hardware / OS:** `Apple M1` | `16.0 GiB` RAM | `macos` `aarch64` | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (`release`)
- **Viewport:** `1280x800` logical (`2560x1600` Retina @ `2.0x`)

## 1. Signal Catalog & Confluence, Monte Carlo Bootstrap & Drawing Inspector Kernel Throughput

| Case ID | Description | Input Units | Output Items | Median (`µs`) | Median (`ms`) | P95 (`µs`) | Max (`µs`) |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| `r15_signal_catalog_and_confluence_fpt_1d` | 17-entry Signal Catalog filter + 4-family Causal Confluence & Quality evaluation on FPT 1D | `180` | `34` | **`23.75 µs`** | **`0.024 ms`** | `27.67 µs` | `133.21 µs` |
| `r15_monte_carlo_splitmix64_1000sim_btc_15m` | 1,000-simulation SplitMix64 bootstrap on BTCUSDT 15m (VALID N>=30, VaR/CVaR 95%/99% + p5..p95 fan) | `1000` | `54` | **`1343.08 µs`** | **`1.343 ms`** | `1447.88 µs` | `1494.62 µs` |
| `r15_drawing_inspector_atomic_patch_undo_redo_1k` | 1,000 validated DrawingPropertyPatch Apply + Undo + Redo transactions | `1000` | `17` | **`261.96 µs`** | **`0.262 ms`** | `265.12 µs` | `359.38 µs` |

## 2. End-to-End Workstation R15 Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)

| Bars (Total) | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | Mean (`ms`) | Median (`ms`) | P99 (`ms`) | `< 4.0ms` Gate |
| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: |
| `180` | `r15_fpt_signals_confluence_and_drawing_inspector` (1x1 FPT 1D + Signals Confluence Inspector + Interactive Drawing Property Inspector) | `Linear` | `1x` | `70` | `69` | `24038` | `83403` | **`0.425`** | **`0.413`** | `0.708` | `PASS (<4ms)` |
| `1344` | `r15_btc_15m_tearsheet_and_monte_carlo_equity_fan` (1x1 BTCUSDT 15m + Tearsheet Monte Carlo Card (VALID N>=30) + Percentage Equity Fan) | `Linear` | `1x` | `1344` | `1343` | `139884` | `450801` | **`1.372`** | **`1.343`** | `1.672` | `PASS (<4ms)` |
| `2150` | `r15_vn30f_replay_and_signal_catalog_modal` (1x1 VN30F1M 5m BlindPractice Replay + 17-Entry Signal Catalog Modal) | `Linear` | `1x` | `133` | `132` | `28074` | `86412` | **`0.337`** | **`0.331`** | `0.507` | `PASS (<4ms)` |
| `100000` | `r15_100k_conflated_mc_equity_fan_and_inspector` (100,000-bar conflated chart + Monte Carlo Equity Fan + Drawing Property Inspector) | `Linear` | `64x` | `100000` | `1562` | `73906` | `162537` | **`1.060`** | **`1.011`** | `1.575` | `PASS (<4ms)` |
| `200000` | `r15_quad_2x2_50k_signals_and_inspector_stress` (Quad 2x2 (4x50k = 200k bars) conflated stress + Signals Confluence + Drawing Inspector) | `Linear` | `64x` | `200000` | `3124` | `95856` | `217584` | **`1.557`** | **`1.531`** | `2.098` | `PASS (<4ms)` |
