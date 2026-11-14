# Milestone R11 — Order-Flow Volume Profile (`VRVP`/`FRVP`), Session/Anchored VWAP & Causal Relative Strength vs `VNINDEX` Benchmark Evidence

- **Command:** `cargo run --release --example viewport_bench -- --r11`
- **Hardware:** `Apple M1` (`aarch64-macos`), `16.0 GiB` RAM
- **Toolchain & Profile:** `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (`--release`)
- **Active Stack:** Intrabar `VRVP`/`FRVP` (Exact Buy/Sell conservation + 70% VA + POC) + `Session VWAP` (`UTC+7` reset, ±1σ/±2σ Welford bands) + `Anchored VWAP` (`DEF-001` causal) + `RS vs VNINDEX` + Rebased Compare Overlay

## 1. Volume Profile, VWAP & Causal Relative Strength Kernel Throughput

| Case ID | Description | Input Units | Output Items | Median (`µs`) | Median (`ms`) | P95 (`µs`) | Max (`µs`) |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| `r11_vrvp_500_bars_64_rows` | Visible Range Volume Profile (`500` bars, `64` rows, 70% VA + POC) | `500` | `64` | **`14.21 µs`** | **`0.014 ms`** | `18.58 µs` | `80.33 µs` |
| `r11_vrvp_50k_bars_64_rows` | Full-History Volume Profile (`50,000` bars, `64` rows, 70% VA + POC) | `50000` | `64` | **`803.08 µs`** | **`0.803 ms`** | `896.50 µs` | `951.04 µs` |
| `r11_session_vwap_50k_1m_utc7` | Session VWAP (`50,000` 1m bars, `UTC+7` reset, ±1σ/±2σ Welford bands) | `50000` | `197` | **`725.58 µs`** | **`0.726 ms`** | `785.38 µs` | `896.62 µs` |
| `r11_anchored_vwap_50k_bars` | Anchored VWAP (`50,000` 1m bars, anchor `25,000`, ±1σ/±2σ bands) | `50000` | `25000` | **`371.25 µs`** | **`0.371 ms`** | `410.83 µs` | `576.25 µs` |
| `r11_causal_rs_50k_vs_vnindex` | Causal Relative Strength vs `VNINDEX` (`50,000` 1m bars, `DEF-001` close alignment + `20`-SMA) | `50000` | `50000` | **`651.71 µs`** | **`0.652 ms`** | `730.00 µs` | `907.08 µs` |

## 2. End-to-End Workstation Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)

| Bars | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | Mean (`ms`) | Median (`ms`) | P99 (`ms`) | `< 4.0ms` Gate |
| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: |
| `180` | `r11_fpt_vrvp_frvp_vwap_compare_rs` (FPT 1D + VRVP + FRVP + VWAP(±1σ/2σ) + AVWAP + Compare VNINDEX + RS Pane) | `Linear` | `1x` | `114` | `113` | `18508` | `59976` | **`0.256`** | **`0.253`** | `0.396` | `PASS (<4ms)` |
| `50000` | `r11_vn30f_50k_1m_session_vwap_vrvp_rs` (VN30F1M 50k 1m + Session VWAP(UTC+7) + AVWAP + VRVP(64) + FRVP + RS) | `Linear` | `1x` | `124` | `123` | `17908` | `56952` | **`0.266`** | **`0.257`** | `0.411` | `PASS (<4ms)` |
| `50000` | `r11_vn30f_50k_conflated_vwap_vrvp_compare_rs` (VN30F1M 50k 1m Zoom-Out Conflated + VRVP + Session VWAP + Compare + RS) | `Linear` | `16x` | `49497` | `3092` | `199262` | `313203` | **`3.171`** | **`3.204`** | `3.831` | `PASS (<4ms)` |
| `50000` | `r11_vn30f_50k_interactive_pan_zoom_avwap_compare_rs` (VN30F1M 50k 1m Interactive Pan+Zoom + VRVP + FRVP + VWAP + AVWAP + Compare VNINDEX + RS) | `Linear` | `1x` | `624` | `622` | `53756` | `176736` | **`0.586`** | **`0.579`** | `0.768` | `PASS (<4ms)` |
