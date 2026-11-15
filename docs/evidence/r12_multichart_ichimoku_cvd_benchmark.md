# Milestone R12 — Synchronized Multi-Chart Grid, Ichimoku Cloud, Cumulative Volume Delta (`CVD`) & Workspace Templates Benchmark Evidence

- **Command:** `cargo run --release --example viewport_bench -- --r12`
- **Hardware:** `Apple M1` (`aarch64-macos`), `16.0 GiB` RAM
- **Toolchain & Profile:** `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (`--release`)
- **Active Stack:** Multi-Chart Grid (`1x1`/`1x2`/`2x1`/`2x2` + `Sync X`/`Sync T`), Ichimoku Cloud (`(9,26,52,+26)` Kumo Twist triangle-split `egui::Mesh` + causal `DEF-001` `Chikou`), `CVD` (`OHLCV_PROXY` `Session` UTC+7 / `Cumulative` + `9`-SMA), & Versioned `WorkspaceTemplate` (`RON`/`JSON` v1)

## 1. Ichimoku Cloud, CVD & Workspace Template Kernel Throughput

| Case ID | Description | Input Units | Output Items | Median (`µs`) | Median (`ms`) | P95 (`µs`) | Max (`µs`) |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| `r12_ichimoku_50k_cold_compute` | Ichimoku Kinko Hyo (9,26,52,+26) cold O(N) compute on 50,000 1m bars | `50000` | `50026` | **`3287.83 µs`** | **`3.288 ms`** | `3461.29 µs` | `3547.58 µs` |
| `r12_cvd_session_50k_cold_compute` | Session CVD (OHLCV_PROXY UTC+7 daily reset + 9-SMA) cold compute on 50,000 1m bars | `50000` | `197` | **`409.42 µs`** | **`0.409 ms`** | `439.50 µs` | `501.62 µs` |
| `r12_workspace_template_ron_json_roundtrip` | WorkspaceTemplate v1 RON+JSON encode & validated decode (4-slot desk) | `4` | `13214` | **`102.96 µs`** | **`0.103 ms`** | `119.71 µs` | `185.04 µs` |

## 2. End-to-End Workstation Multi-Chart Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)

| Bars (Grid Total) | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | Mean (`ms`) | Median (`ms`) | P99 (`ms`) | `< 4.0ms` Gate |
| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: |
| `50000` | `r12_single_1x1_ichimoku_cvd_50k_frame` (1x1 Single Focus 50k Bars + Ichimoku (+26 Kumo) + Session CVD) | `Linear` | `1x` | `131` | `129` | `16458` | `54330` | **`0.232`** | **`0.227`** | `0.339` | `PASS (<4ms)` |
| `200000` | `r12_quad_2x2_synced_ichimoku_cvd_50k_frame` (2x2 Quad Synced 50k x 4 Slots (200k Total Bars) + Sync X/T + Ichimoku + CVD) | `Linear` | `1x` | `180` | `176` | `30012` | `99942` | **`0.371`** | **`0.352`** | `0.476` | `PASS (<4ms)` |
| `200000` | `r12_quad_2x2_pan_time_sync_50k_frame` (2x2 Quad 50k x 4 Slots Interactive Pan/Zoom + Live Sync T + Linked Crosshair + Ichimoku + CVD) | `Linear` | `1x` | `157` | `150` | `27894` | `92640` | **`0.375`** | **`0.359`** | `0.608` | `PASS (<4ms)` |
| `50360` | `r12_mtf_1x2_conflated_ichimoku_cvd_50k_frame` (1x2 Split MTF Conflated 50k 1m + 15m + Ichimoku Kumo + Session CVD) | `Linear` | `32x` | `50360` | `1584` | `47952` | `112980` | **`0.389`** | **`0.383`** | `0.527` | `PASS (<4ms)` |
| `200000` | `r12_quad_2x2_conflated_ichimoku_vwap_session_cvd_50k_frame` (2x2 Quad Conflated (step>=8) 50k 1m x 4 (200k Bars) + Ichimoku + VWAP + Session CVD) | `Linear` | `32x` | `200000` | `6248` | `162876` | `354984` | **`1.303`** | **`1.278`** | `1.608` | `PASS (<4ms)` |
