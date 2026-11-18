# Milestone R16 — Composable Signal Rule Builder, Causal Signal/Confluence Alerts & Workspace Snapshot Checkpoints Benchmark Evidence

- **Command:** `cargo run --release --example viewport_bench -- --r16`
- **Hardware / OS:** `Apple M1` | `16.0 GiB` RAM | `macos` `aarch64` | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (`release`)
- **Viewport:** `1280x800` logical (`2560x1600` Retina @ `2.0x`)

## 1. Custom Signal Rule Builder, Causal Alert Ledger & Snapshot Store Kernel Throughput

| Case ID | Description | Input Units | Output Items | Median (`µs`) | Median (`ms`) | P95 (`µs`) | Max (`µs`) |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| `r16_custom_signal_rule_backtest_btc_15m` | O(N) 17-signal PrecomputedCatalogMatrix + 2-level CustomSignalRuleSpec Open[T+1] backtest on BTCUSDT 15m | `1344` | `32` | **`784.12 µs`** | **`0.784 ms`** | `869.58 µs` | `883.17 µs` |
| `r16_causal_signal_confluence_alerts_and_csv_json_export` | 3-rule SignalCatalog & Confluence closed-bucket alert scan + cooldown + formula-safe CSV/JSON export on FPT 1D | `180` | `7` | **`71.08 µs`** | **`0.071 ms`** | `72.38 µs` | `76.00 µs` |
| `r16_snapshot_store_ron_roundtrip_diff_and_quarantine` | WorkspaceSnapshotStore RON encode/decode + field-level diff + corrupt-payload quarantine | `3` | `4` | **`822.08 µs`** | **`0.822 ms`** | `864.00 µs` | `921.71 µs` |

## 2. End-to-End Workstation R16 Frame Budget (`DECISION-RENDERER-002` Gate: `< 4.0 ms`)

| Bars (Total) | Workload | Scale | Conflation | Culled | Drawn | Vertices | Indices | Mean (`ms`) | Median (`ms`) | P99 (`ms`) | `< 4.0ms` Gate |
| ---: | :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: |
| `1344` | `r16_btc_15m_custom_rule_builder_and_mc_fan` (1x1 BTCUSDT 15m + Strategy Lab Rule Builder (CustomSignalRules) + MC Equity Fan) | `Linear` | `1x` | `1344` | `1343` | `141068` | `459558` | **`1.332`** | **`1.317`** | `1.501` | `PASS (<4ms)` |
| `180` | `r16_fpt_signal_confluence_alerts_and_markers` (1x1 FPT 1D + SignalCatalog & Confluence Alerts Dock + Cooldown + Stacked Markers) | `Linear` | `1x` | `104` | `103` | `21208` | `72900` | **`0.322`** | **`0.319`** | `0.384` | `PASS (<4ms)` |
| `540` | `r16_1x2_snapshots_diff_and_quarantine_modal` (1x2 Multi-Chart + Snapshot Checkpoints Modal + Diff Preview + Quarantine Banner) | `Linear` | `1x` | `72` | `70` | `31542` | `100143` | **`0.455`** | **`0.450`** | `0.531` | `PASS (<4ms)` |
| `100000` | `r16_100k_conflated_custom_rules_and_alerts` (100,000-bar conflated chart + CustomSignalRules Rule Builder + Equity Fan + Alerts) | `Linear` | `32x` | `96713` | `3020` | `115810` | `232278` | **`1.093`** | **`1.070`** | `1.310` | `PASS (<4ms)` |
| `200000` | `r16_quad_2x2_50k_rule_builder_and_snapshots_stress` (Quad 2x2 (4x50k = 200k bars) conflated stress + Rule Builder + Named Snapshots) | `Linear` | `64x` | `192624` | `3008` | `92304` | `211314` | **`1.218`** | **`1.212`** | `1.287` | `PASS (<4ms)` |
