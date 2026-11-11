# SUMI TRADING WORKSTATION — EVIDENCE INDEX

- **Updated By:** `SESSION-R03-20261111T105832+0700` (`SUMI PROGRAM ORCHESTRATOR`)

## Session 01 Evidence Artifacts

| Artifact Path | Category | Verification Status | Description |
| --- | --- | --- | --- |
| [`docs/evidence/workstation_1280x800.png`](file:///Users/mizuhara/workspace/sumi/docs/evidence/workstation_1280x800.png) | Visual GUI Capture | **PASS** | Native macOS Metal (`wgpu`) framebuffer capture at default `1280×800` logical size (`2560×1496` Retina px) showing dark theme, `VNINDEX · 1D` candlesticks, volume histogram, price/time axes, crosshair, top toolbar, and bottom status bar (`PLOT: 1182×632px`). |
| [`docs/evidence/workstation_960x600_resized.png`](file:///Users/mizuhara/workspace/sumi/docs/evidence/workstation_960x600_resized.png) | Visual GUI Capture (Resized) | **PASS** | Native macOS Metal (`wgpu`) framebuffer capture at resized `960×600` logical size (`1920×1200` Retina px) proving resize-aware chart layout, preserved axis alignment, and non-overlapping time/price ticks (`PLOT: 862×484px`). |
| [`docs/evidence/telemetry_1280x800.json`](file:///Users/mizuhara/workspace/sumi/docs/evidence/telemetry_1280x800.json) | Runtime Telemetry | **PASS** | Exact layout rectangle coordinates (`outer_rect`, `plot_rect`, `price_axis_rect`, `time_axis_rect`) and price bounds from the live `1280×800` GUI execution. |
| [`docs/evidence/telemetry_960x600_resized.json`](file:///Users/mizuhara/workspace/sumi/docs/evidence/telemetry_960x600_resized.json) | Runtime Telemetry (Resized) | **PASS** | Exact layout rectangle coordinates and price bounds from the live `960×600` resized GUI execution. |
| [`docs/evidence/rea_setup_dry_run.json`](file:///Users/mizuhara/workspace/sumi/docs/evidence/rea_setup_dry_run.json) | Security & Tooling Audit | **PASS** | Read-only dry-run plan output (`rea setup --client antigravity --dry-run --json`) inspected before applying configuration changes. |
| [`docs/evidence/rea_setup_apply.json`](file:///Users/mizuhara/workspace/sumi/docs/evidence/rea_setup_apply.json) | Security & Tooling Audit | **PASS** | Output of authorized `rea setup --client antigravity --yes --json` confirming backup creation, MCP registration, and skill installation. |
| [`docs/evidence/rea_doctor_antigravity.json`](file:///Users/mizuhara/workspace/sumi/docs/evidence/rea_doctor_antigravity.json) | Security & Tooling Audit | **PASS** | Output of `rea doctor --client antigravity --skill --json` confirming `healthy: true` for the Antigravity client and aligned skill (`v49`, `229` tools). |
| [`docs/evidence/rea_mcp_doctor.json`](file:///Users/mizuhara/workspace/sumi/docs/evidence/rea_mcp_doctor.json) | MCP Protocol Verification | **PASS** | Output of `REA_MCP_DOCTOR_DEADLINE_MS=120000 rea mcp doctor --json` confirming `healthy: true`, `229/229` tools, `6/6` prompts. |

## Session R01 REA Analysis Evidence (Live MCP)

| Evidence ID | REA Tool | Target | Category | Key Finding |
| --- | --- | --- | --- | --- |
| (live session) | `binary_session` | — | REA Capability Discovery | REA MCP server active; `native-macos` + `rea-artifact-graph` available; Ghidra/Hopper/IDA/Kuna not installed |
| (live session) | `open_binary` | `target/debug/sumi-workstation` | Binary Identity | `mach-o`, `arm64`, SHA-256: `bf299c4d...` |
| `ev_93a09f1c...` | `list_architectures` | (open binary) | Architecture | arm64 single-slice; via `lipo` |
| `ev_a42472d0...` | `inspect_macho` | (open binary) | Mach-O Inspection | 30 load commands, 14 dylib deps, 327 imports, 6208 exports, 5 segments; UUID `997FDB88-...` |
| `ev_e95e76c4...` | `inspect_analysis_view` | `ev_a42472d0...` | Summary View | 6587 items examined, complete; confirms Mach-O summary |
| `ev_ac47096a...` | `trace_dylib_resolution` | (open binary) | Dependency Graph | 14 dylib edges; Metal, CoreVideo (CVDisplayLink), AppKit, CoreGraphics confirmed |
| `ev_e209114...` | `analyze_javascript_application` | `ARCHIVE/frontend` | JS Analysis | ChartKernel bundle: 84,465 AST nodes, 16 findings, 1,712 call-flow relations; lightweight-charts v5 architecture confirmed |

## Session R02 REA Analysis Evidence (Live MCP)

| Evidence ID | REA Tool | Target | Category | Key Finding |
| --- | --- | --- | --- | --- |
| `ev_7406bf9f42b407703c6cc89ba194cc9064a0ac9ffec842e96f938b12ad88589c` | `analyze_javascript_application` | LWC 5.2.0 `src/model/` (6 TypeScript files) | JS/TS Analysis | 6 files parsed, 10,463 AST nodes, 2,624 data-flow + 79 call-flow + 114 closure relations; `price-scale.ts` → `price-scale-conversions.ts` import resolved; `markWithGreaterWeight` export recovered at confidence `exact` |
| `ev_e792f7a99c261860eaac5fc53f38d3b2241c3e766489a9a1a4759dc0ca5e203c` | `inspect_analysis_view` (summary) | Same parent as above | Summary View | Coverage: 1,135/1,135 total source items examined within view scope; 18 limitations recorded |
| `ev_b49b23e17a5d5d7437f328552d8e28af2ebcb231145dfe2da44e52e7d02dc197` | `inspect_analysis_view` (javascript-facts) | Same parent as above | Module Relationship Facts | 114 fact rows for `time-scale.ts` + `price-scale.ts`; import graph, export recovery, and unresolved semantic modules all recorded with source-range locations |

## Session R01 Research Documents

| Document | Path |
| --- | --- |
| REA Live Capability Report | [`docs/research/REA_LIVE_CAPABILITY_REPORT.md`](file:///Users/mizuhara/workspace/sumi/docs/research/REA_LIVE_CAPABILITY_REPORT.md) |
| REA Execution Evidence | [`docs/research/REA_EXECUTION_EVIDENCE.md`](file:///Users/mizuhara/workspace/sumi/docs/research/REA_EXECUTION_EVIDENCE.md) |
| Chart Engine Research Map | [`docs/research/CHART_ENGINE_RESEARCH_MAP.md`](file:///Users/mizuhara/workspace/sumi/docs/research/CHART_ENGINE_RESEARCH_MAP.md) |
| TradingView Feasibility & Rights | [`docs/research/TRADINGVIEW_FEASIBILITY_AND_RIGHTS.md`](file:///Users/mizuhara/workspace/sumi/docs/research/TRADINGVIEW_FEASIBILITY_AND_RIGHTS.md) |
| Research to Rust Roadmap | [`docs/research/RESEARCH_TO_RUST_ROADMAP.md`](file:///Users/mizuhara/workspace/sumi/docs/research/RESEARCH_TO_RUST_ROADMAP.md) |

## Session R02 Research Documents

| Document | Path |
| --- | --- |
| Viewport REA Trace Report | [`docs/research/VIEWPORT_REA_TRACE_REPORT.md`](file:///Users/mizuhara/workspace/sumi/docs/research/VIEWPORT_REA_TRACE_REPORT.md) |
| Coordinate Algorithm Spec | [`docs/research/COORDINATE_ALGORITHM_SPEC.md`](file:///Users/mizuhara/workspace/sumi/docs/research/COORDINATE_ALGORITHM_SPEC.md) |
| Legacy Sumi Coordinate Comparison | [`docs/research/LEGACY_SUMI_COORDINATE_COMPARISON.md`](file:///Users/mizuhara/workspace/sumi/docs/research/LEGACY_SUMI_COORDINATE_COMPARISON.md) |
| Rust Viewport Camera Design | [`docs/research/RUST_VIEWPORT_CAMERA_DESIGN.md`](file:///Users/mizuhara/workspace/sumi/docs/research/RUST_VIEWPORT_CAMERA_DESIGN.md) |
| Chart Renderer Architecture Decision | [`docs/research/CHART_RENDERER_ARCHITECTURE_DECISION.md`](file:///Users/mizuhara/workspace/sumi/docs/research/CHART_RENDERER_ARCHITECTURE_DECISION.md) |

## Session R03 REA Analysis Evidence (Live MCP — End-to-End LWC Trace & Release Binary)

| Evidence ID | REA Tool | Target | Category | Key Finding |
| --- | --- | --- | --- | --- |
| `ev_eea4438f22f8e6ae6c2538bd565c1f96a52a5941807bc2ac4a0cab50383bb13c` | `analyze_javascript_application` (`parent`) | LWC v5.2.0 (`19` TS files across `src/gui/`, `src/model/`, `src/model/series/`, `src/renderers/`) | Full 19-File Input → Model → View → Renderer Graph | Artifact SHA-256 `6a26c4ad...`, `282,080` bytes, `39,119` AST nodes, `662` findings, `9,829` data-flow, `1,060` object-flow, `494` closure, `248` call-flow, `3` timer relations; extensionless TS imports documented as `unresolved-semantic-module`. |
| `ev_723345eb98d53cce49f94fe2515458cb5761706192107e634d3f366760013e47` | `analyze_javascript_application` (`summary` receipt) | Parent `ev_eea4438f...` | Summary Projection | Retained summary view receipt for the 19-file LWC v5.2.0 static AST & semantic relation graph. |
| `ev_a95b3afcae1386edab36ef7b425d8d978278b4d346160f70704ce976cdd50757` | `inspect_analysis_view` (`modules/references`, p.0) | Parent `ev_eea4438f...` (`limit: 32`) | Module Reference Page | Inspects `32/392` module-reference nodes across the 19-file LWC graph. |
| `ev_053b26b40935611f1c60510f854586d9db34d28db97251d8eba94e56eb3be63a` | `inspect_analysis_view` (`modules/references`) | `src/model/time-scale.ts` | Export & Import Recovery | Recovers exact `TimeScale` class node (`jag_node_665353cfa1dc...`) & `markWithGreaterWeight` (`jag_node_2ce2b323339f...`). |
| `ev_43b616987aad8f2dddb22f6375dd9dddf591c677c801f7f17ec97679774e3e42` | `inspect_analysis_view` (`modules/references`) | `src/gui/chart-widget.ts` | GUI Module Dependencies | Recovers `chart-widget.ts` source module node (`jag_node_21d8a87dcae7...`) and static references to `chart-model`, `pane-widget`, `time-axis-widget`. |
| `ev_ac00957e74f26e60e51a610407cff63c32b78eecaa0de40f957c785540417fcb` | `inspect_analysis_view` (`modules/references`) | `src/model/series/candlesticks-pane-view.ts` | Series Pane View Recovery | Recovers `SeriesCandlesticksPaneView` (`jag_node_44af9e2b9e44...`) and references to `candlesticks-renderer` & `bars-pane-view-base`. |
| `ev_9e8c029e430053363e4c6cfe831dc284e41c513d633ecd4234c3214af026c0c4` | `trace_application_feature` (`both`) | `jag_node_665353cfa1dc...` (`TimeScale`) | MCP Boundary Finding | Hit `10 MiB` MCP stdio framing cap (`10,577,883 > 10,485,760` bytes) due to incoming repo-wide containment edges while retaining graph evidence. |
| `ev_97bce9491cbee5f4b260837294182078669090580717ef6b9f7b26074b4e22cb` | `trace_application_feature` (`outgoing`) | `jag_node_665353cfa1dc...` (`TimeScale`) | Feature Slice | Anchors `module-export:TimeScale` span at `src/model/time-scale.ts` `L319:1–L1253:1` (`offset 9130..36619`). |
| `ev_61932519fe807df708a84d30cdad6775dd747bccb144af48c5d8e3cd852601a0` | `trace_javascript_semantics` (`forward-influence`) | `property: "_barSpacing"` | Horizontal Spacing Influence | `25` matches across `16` functions in `time-scale.ts` (`setWidth`, `indexToCoordinate`, `indexesToCoordinates`, `_setBarSpacing`, `_correctBarSpacing`, `_updateConflationFactor`, etc.). |
| `ev_e6ce5dc81319a55825cd5c3c764bb62c72412ed67ab4bba309ab4639cb6e134f` | `trace_javascript_semantics` (`backward-provenance`) | `property: "_barSpacing"` | Spacing Write-Site Proof | `25` matches, `35` nodes, `10` relations (`defines: 7`, `reads: 3`) — proves **only 4 mutation sites** write `_barSpacing`. |
| `ev_632b75be2af4581f72306e87d4b8460e9869d99de09a04c11be21cee112b32b99` | `trace_javascript_semantics` (`backward-provenance`) | `property: "_rightOffset"` | Offset Write-Site Proof | `20` matches, `33` nodes, `14` relations (`defines: 10`, `reads: 4`) — proves all write sites to `_rightOffset` (`setWidth`, `setBarSpacing`, `scrollTo`, `setVisibleRange`, `_correctOffset`). |
| `ev_ddc3ea625b03bbf26ed1efb56d08710d434469c9920d73afd2240482974f9785` | `trace_javascript_semantics` (`forward-influence`) | `property: "_priceRange"` | Price Range Aliasing Trace | `8` matches, `12` nodes, `4` `aliases` relations in `price-scale.ts` (`setMode` log conversion, `setPriceRange`, `_recalculatePriceRangeImpl`). |
| `ev_e184c024632ee170529ac0daf91b893ca01525871ca536e6caaae4a98a431e03` | `trace_javascript_semantics` (`forward-influence`) | `property: "zoom"` | Model → Scale Dispatch | `1` match at `chart-model.ts:838` (`ChartModel.zoomTime` → `timeScale.zoom(pointX, scale)`, documenting `jsrg_unknown_*` at TS interface boundary). |
| `ev_9fab762bb55667e6d67548be36e310956b52a700fa36cc6ba927c946a9950d42` | `trace_javascript_semantics` (`forward-influence`) | `property: "bars"` | Renderer Consumption Trace | `2` matches at `chart-widget.ts:806` & `candlesticks-renderer.ts:61` (`PaneRendererCandlesticks._drawImpl` reading `this._data.bars`). |
| `ev_a14d90561ce902cf333009c7725fa2e220430cfa6271e85e30325ca5be0ce9a2` | `trace_javascript_semantics` (`backward-provenance`) | `property: "_bars"` | Negative Control Query | `0` matches (confirms LWC renderer stores bar payload on `_data.bars` rather than `_bars`). |
| `ev_11ff50a38cf41844baddb6607057f289d5c6e1622e54794512c0ca0def753f64` | `inspect_rust_binary` | `target/release/sumi-workstation` | Release Binary Audit | `16,236,624` bytes (`arm64` Mach-O), SHA-256 `9b847fd8c9c676d28b0b03e0d553cced160b57c65b4bc6af7db0d5a7a5115951`, inferred `rustc` commit `58cf8f9c35b355199727627d07ae34a4e2846d67`, `.dep-v0` absent without `cargo-auditable`. |
| `ev_60ae4bfa2620b1643c5a21c032f5be8d3cccfeb9d6dcd11c812c324040a12040` (`ev_b24f106745a38d3661977833d96209dc11e8b07e9ca171f6611fd1db261ba329` view) | `inspect_macho` (`summary`, via `open_binary` / `close_binary`) | `target/release/sumi-workstation` | Release Mach-O Audit | UUID `68A51A2D-3CEB-336B-BDFF-6CE3E7AC5B16`, `30` load commands, `14` dylibs, `324` imports, `4,744` exports, `5` segments. |

## Session R03 Implementation, GPU Capture & Benchmark Artifacts

| Artifact Path | Category | Verification Status | Description |
| --- | --- | --- | --- |
| [`docs/operations/SUMI_ORCHESTRATION_CHARTER.md`](file:///Users/mizuhara/workspace/sumi/docs/operations/SUMI_ORCHESTRATION_CHARTER.md) | Program Governance | **VERIFIED** | Durable repository-native orchestration charter establishing role protocols, subagent capabilities, evidence taxonomy, and PO decision gates. |
| [`docs/evidence/r03_default_1280x800.png`](file:///Users/mizuhara/workspace/sumi/docs/evidence/r03_default_1280x800.png) & [`.json`](file:///Users/mizuhara/workspace/sumi/docs/evidence/r03_default_1280x800.json) | Live Metal GPU Capture | **VERIFIED** | Default `1280×800` (`2560×1496` Retina, PNG SHA-256 `c3bb97d6...`) workstation showing `right_offset = +12.0` future whitespace, `bar_spacing = 8.0`, crisp 2-segment wicks, volume overlay, and status telemetry. |
| [`docs/evidence/r03_zoom_cursor_1280x800.png`](file:///Users/mizuhara/workspace/sumi/docs/evidence/r03_zoom_cursor_1280x800.png) & [`.json`](file:///Users/mizuhara/workspace/sumi/docs/evidence/r03_zoom_cursor_1280x800.json) | Live Metal GPU Capture | **VERIFIED** | Cursor-anchored zoom (`bar_spacing = 14.40`, `right_offset = -10.85`, `85` culled bars, PNG SHA-256 `97569c91...`) proving wide parity-aligned candle bodies and autoscaled right axis. |
| [`docs/evidence/r03_log_100k_fit_1280x800.png`](file:///Users/mizuhara/workspace/sumi/docs/evidence/r03_log_100k_fit_1280x800.png) & [`.json`](file:///Users/mizuhara/workspace/sumi/docs/evidence/r03_log_100k_fit_1280x800.json) | Live Metal GPU Capture (100K Bars) | **VERIFIED** | Full `100,000`-bar dataset fitted on `Logarithmic` scale (`conflation_factor = 32x` → `3,124` conflated candles drawn, `[32x]` HUD `Vol 26.54M`, decade-proportional log ticks, PNG SHA-256 `29b92e58...`). |
| [`docs/evidence/r03_pct_resized_960x600.png`](file:///Users/mizuhara/workspace/sumi/docs/evidence/r03_pct_resized_960x600.png) & [`.json`](file:///Users/mizuhara/workspace/sumi/docs/evidence/r03_pct_resized_960x600.json) | Live Metal GPU Capture (Resized + `%`) | **VERIFIED** | Resized `960×600` (`1920×1200` Retina, `PLOT: 858×445px`, PNG SHA-256 `15b13a1d...`) in `Percentage` scale mode (`-6.00%..+6.00%` ticks, `+3.47%` latest-close badge). |
| [`docs/evidence/r03_viewport_benchmark.json`](file:///Users/mizuhara/workspace/sumi/docs/evidence/r03_viewport_benchmark.json) & [`.md`](file:///Users/mizuhara/workspace/sumi/docs/evidence/r03_viewport_benchmark.md) | Performance Benchmark | **VERIFIED** | Reproducible `--release` benchmark across `10K`, `50K`, and `100K` bars (`0.67 µs` pure math; `0.094–0.102 ms` standard view; `1.472 ms` 100K conflated fit vs `39.570 ms` unconflated; `1.143 ms` 3-pane shared-`TimeScale` dense). |
