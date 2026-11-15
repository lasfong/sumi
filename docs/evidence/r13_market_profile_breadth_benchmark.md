# Milestone R13 — Session Market Profile, Multi-Interval VWAP, VN30 Breadth/RRG & Compact CVD Axis Benchmark Evidence

- **Command:** `cargo run --release --example viewport_bench -- --r13`
- **Hardware:** `Apple M1` (`aarch64-macos`), `16.0 GiB` RAM
- **Toolchain & Profile:** `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (`--release`)
- **Active Stack:** Session TPO / Market Profile (`POC`, 70% `VAH`/`VAL`, `IB`, `Single Prints`, causal `Naked POC`), Multi-Interval `VWAP` (`Session`/`Daily`/`Weekly`/`Monthly`/`Yearly`), Causal VN30 & Universe Market Breadth + `RRG` (`AT05`/`AT09`/`AT17`/`DEF-001`), & Compact `K`/`M`/`B`/`T` `CVD` Right Axis

## 1. Market Profile, VWAP Anchor, Breadth/RRG & Compact CVD Kernel Throughput

| Case ID | Description | Input Units | Output Items | Median (`µs`) | Median (`ms`) | P95 (`µs`) | Max (`µs`) |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| `r13_session_tpo_market_profile_50k_15m_equivalent` | Session TPO / Market Profile (POC, 70% VAH/VAL, IB, Naked POC) on 50,000 15m bars | `50000` | `24` | **`331.54 µs`** | **`0.332 ms`** | `345.17 µs` | `398.75 µs` |
| `r13_multi_anchor_vwap_weekly_50k_cold_compute` | Weekly-anchored Welford VWAP (+-1/+-2 sigma bands) cold compute on 50,000 1m bars | `50000` | `50000` | **`785.75 µs`** | **`0.786 ms`** | `909.00 µs` | `916.04 µs` |
| `r13_vn30_universe_breadth_rrg_cold_compute` | Causal VN30/Universe Breadth + AT05/09 Money Flow + 4-Quadrant RRG trajectories | `13` | `12` | **`858.79 µs`** | **`0.859 ms`** | `988.29 µs` | `1299.21 µs` |
| `r13_compact_volume_axis_tick_format_10k` | 10,000 CompactVolume (K/M/B/T) right-axis formatter calls | `10000` | `51207` | **`2079.33 µs`** | **`2.079 ms`** | `2216.50 µs` | `2247.92 µs` |

## 2. End-to-End Workstation R13 Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)

| Bars (Total) | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | Mean (`ms`) | Median (`ms`) | P99 (`ms`) | `< 4.0ms` Gate |
| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: |
| `3334` | `r13_single_1x1_vn30f1m_15m_tpo_vwap_compact_cvd` (1x1 VN30F1M 15m Session TPO + Naked POC + Weekly VWAP + Compact K/M/B CVD) | `Linear` | `1x` | `131` | `130` | `18088` | `60126` | **`0.259`** | **`0.253`** | `0.382` | `PASS (<4ms)` |
| `540` | `r13_split_1x2_session_profile_flow_template` (1x2 Built-in Template #5 (VN30F1M 15m | FPT 1D) TPO + Weekly VWAP + Compact CVD) | `Linear` | `1x` | `106` | `104` | `21188` | `70332` | **`0.316`** | **`0.322`** | `0.417` | `PASS (<4ms)` |
| `200000` | `r13_quad_2x2_vn30_breadth_rrg_dock_open` (2x2 Quad Desk + Open 340px Breadth/RRG Quadrant Dock + TPO + Compact CVD) | `Linear` | `1x` | `108` | `104` | `30708` | `98778` | **`0.466`** | **`0.465`** | `0.681` | `PASS (<4ms)` |
| `100000` | `r13_100k_conflated_pan_zoom_tpo_vwap_compact_cvd` (1x1 100k Conflated Pan/Zoom + Session TPO + Weekly VWAP + Compact CVD) | `Linear` | `32x` | `97093` | `3033` | `57352` | `117408` | **`2.469`** | **`2.431`** | `2.809` | `PASS (<4ms)` |
