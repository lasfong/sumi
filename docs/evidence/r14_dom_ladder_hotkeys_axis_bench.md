# Milestone R14 — Synthetic DOM Price Ladder, Conflict-Free Hotkey Profiles & Right-Axis Polish Benchmark Evidence

- **Command:** `cargo run --release --example viewport_bench -- --r14`
- **Hardware:** `Apple M1` (`aarch64-macos`), `16.0 GiB` RAM
- **Toolchain & Profile:** `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (`--release`)
- **Active Stack:** Causal `OHLCV_SYNTH_DOM_PROXY` 21-Row DOM Execution Ladder (`HOSE`/`HNX`/`UPCOM`/`VN30F` dynamic tick/band/lot/short guards), Conflict-Free `HotkeyProfile` + Modal/TextEdit Focus Eligibility, Built-in Template #6 (`VN30F DOM Execution & Flow`), & Edge-Padded Uniform Right-Axis Decimals

## 1. Synthetic DOM Ladder, Hotkey Engine & Right-Axis Precision Kernel Throughput

| Case ID | Description | Input Units | Output Items | Median (`µs`) | Median (`ms`) | P95 (`µs`) | Max (`µs`) |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| `r14_dom_ladder_21row_120bar_50k_vn30f1m` | Causal 21-row OHLCV_SYNTH_DOM_PROXY ladder (120-bar footprint + band/working join) on 50,000 bars | `50000` | `21` | **`277.12 µs`** | **`0.277 ms`** | `310.04 µs` | `334.17 µs` |
| `r14_hotkey_profile_resolve_and_rebind_10k` | 10,000 conflict-free HotkeyProfile::resolve + custom rebind operations | `10000` | `6250` | **`96.96 µs`** | **`0.097 ms`** | `99.38 µs` | `118.17 µs` |
| `r14_axis_edge_pad_nice_ticks_uniform_10k` | 10,000 edge-padded PriceScale::nice_ticks + uniform boundary decimal formats | `10000` | `7` | **`14751.71 µs`** | **`14.752 ms`** | `15050.67 µs` | `15703.25 µs` |

## 2. End-to-End Workstation R14 Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)

| Bars (Total) | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | Mean (`ms`) | Median (`ms`) | P99 (`ms`) | `< 4.0ms` Gate |
| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: |
| `180` | `r14_single_1x1_fpt_dom_ladder_working_orders` (1x1 FPT 1D + 298px 21-Row DOM Ladder (OHLCV_SYNTH_DOM_PROXY) + Staged Orders + TPO + CVD) | `Linear` | `1x` | `93` | `92` | `35596` | `121113` | **`0.459`** | **`0.435`** | `0.848` | `PASS (<4ms)` |
| `540` | `r14_split_1x2_template6_vn30f_dom_execution` (1x2 Built-in Template #6 (VN30F DOM Execution & Flow) + DOM Ladder + InstitutionalDomFast) | `Linear` | `1x` | `83` | `80` | `22648` | `78474` | **`0.378`** | **`0.359`** | `0.636` | `PASS (<4ms)` |
| `2899` | `r14_quad_2x2_axis_polish_and_breadth_dock` (2x2 Quad Desk (FPT/VNINDEX/VCB/VN30F1M) Edge-Padded & Uniform Axis Precision + Breadth Dock) | `Linear` | `1x` | `144` | `139` | `26470` | `86976` | **`0.444`** | **`0.420`** | `1.277` | `PASS (<4ms)` |
| `100000` | `r14_100k_conflated_dom_ladder_and_hotkeys_modal` (1x1 100k Conflated Chart + Active DOM Ladder + Open Keyboard Shortcuts & Hotkey Profiles Modal) | `Linear` | `32x` | `100000` | `3123` | `60916` | `144216` | **`1.372`** | **`1.298`** | `2.198` | `PASS (<4ms)` |
| `200000` | `r14_quad_2x2_50k_conflated_stress` (Quad2x2 50k bars/slot (200k total) conflated fit_all + CVD + DOM Ladder) | `Linear` | `32x` | `200000` | `6244` | `97752` | `193548` | **`1.391`** | **`1.318`** | `1.996` | `PASS (<4ms)` |
