# Milestone R19 — Global `[Settings]` Modal, `VIETNAM_HOLIDAYS_V1` `T+2` FIFO Ledger, Data Readiness Audit & Drawings Benchmark Evidence

- **Command:** `cargo run --release --example viewport_bench -- --r19`
- **Hardware / OS:** `Apple M1` | `16.0 GiB` RAM | `macos` `aarch64` | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (`release`)
- **Viewport:** `1280x800` logical (`2560x1600` Retina @ `2.0x`)

## 1. Workstation Settings, `VIETNAM_HOLIDAYS_V1` `T+2` FIFO Lots, Data Audit & Drawings Kernel Throughput

| Case ID | Description | Input Units | Output Items | Median (`µs`) | Median (`ms`) | P95 (`µs`) | Max (`µs`) |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| `c1_workstation_settings_template_ron_json_roundtrip` | C1 `WorkstationSettings` + `CandlePalettePreset` RON & JSON full roundtrip | `4` | `2` | **`189.58 µs`** | **`0.190 ms`** | `205.92 µs` | `347.62 µs` |
| `c2_vn_holidays_t2_settlement_and_fifo_lots` | C2 `500` `VIETNAM_HOLIDAYS_V1` T+2 lookups + `500`-fill FIFO `HoldingLot` ledger | `649` | `167` | **`95.21 µs`** | **`0.095 ms`** | `98.33 µs` | `99.17 µs` |
| `c3_data_quality_audit_10k_and_cafef_split_gate` | C3 `audit_ohlcv_series` (`10,000` D1 bars) + `13`-feed readiness + `1,000` CafeF split evals | `10013` | `11` | **`3557.29 µs`** | **`3.557 ms`** | `3795.42 µs` | `3829.04 µs` |
| `c4_textnote_dual_trend_custom_fib_projection` | C4 `TextNote` + Dual-Extend `TrendLine` + Custom `OTE` Fib projection & hit-test | `3` | `3` | **`1.71 µs`** | **`0.002 ms`** | `1.79 µs` | `3.96 µs` |

## 2. End-to-End Workstation R19 Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)

| Bars (Total) | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | Mean (`ms`) | Median (`ms`) | P99 (`ms`) | `< 4.0ms` Gate |
| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: |
| `180` | `r19_fpt_hollow_candles_textnote_custom_fib_1280x800` (FPT 1D HollowBullish candles + TextNote + Dual TrendLine + OTE 9-Fib + Drawing Inspector) | `Linear` | `1x` | `60` | `59` | `17636` | `62232` | **`0.266`** | **`0.259`** | `0.435` | `PASS (<4ms)` |
| `155` | `r19_fpt_okabe_ito_vn_t2_fifo_lots_1280x800` (FPT 1D Okabe-Ito palette + Causal Bar Replay + VSD T+2 (13:00 ICT) FIFO Holding Lots) | `Linear` | `1x` | `96` | `95` | `18290` | `60666` | **`0.275`** | **`0.263`** | `0.476` | `PASS (<4ms)` |
| `2839` | `r19_settings_modal_open_quad_2x2_1280x800` (Quad 2x2 Desk + Global [Settings] Modal (Candle Palettes, Venue, Capital, Fees & T+2 Lock)) | `Linear` | `1x` | `165` | `160` | `26374` | `86592` | **`0.345`** | **`0.337`** | `0.581` | `PASS (<4ms)` |
| `217` | `r19_data_readiness_audit_modal_1280x800` (2x1 FPT Desk + [Data Audit] Modal (13-Feed Readiness, VIETNAM_HOLIDAYS_V1 & CafeF Split)) | `Linear` | `1x` | `159` | `157` | `29418` | `92295` | **`0.372`** | **`0.357`** | `0.592` | `PASS (<4ms)` |
| `100000` | `r19_100k_conflated_hollow_bullish_1280x800` (100K bars conflated (>=8x) HollowBullish candlesticks + TextNote + Extended TrendLine) | `Linear` | `32x` | `99832` | `3118` | `51294` | `121458` | **`0.253`** | **`0.245`** | `0.427` | `PASS (<4ms)` |
