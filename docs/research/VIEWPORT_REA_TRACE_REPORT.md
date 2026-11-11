# VIEWPORT REA TRACE REPORT — SESSION R02 (AUDITED & EXPANDED IN SESSION R03)

- **Original Session Identity:** `SESSION-R02-20261110T230337+0700`
- **Audit & Expansion Identity:** `SESSION-R03-20261111` (Sumi Program Orchestrator)
- **Original Status (R02 Self-Report):** `RESEARCH_ACCEPTED`
- **Audited R02 Classification (before R03 rectification):** `ACCEPTED WITH SCOPE LIMITS`
- **Current Rectified Status (after R03 REA traces + source audit):** `VERIFIED & RECTIFIED FOR R03`

---

## 1. Research Targets

| Target | Identity | Authority | License |
|--------|----------|-----------|---------|
| LWC v5.2.0 source | `lightweight-charts-5.2.0` / git commit `868cae27bd1acafa0128d8d868ea740a59ae42ce`, tag `v5.2.0` | Shipped OSS artifact | Apache-2.0 |
| Archive Sumi ChartKernel | `ARCHIVE/frontend/src/components/chart/ChartKernel.ts` | Owned prior-version source | Proprietary / internal archive |
| Archive Sumi PaneManager | `ARCHIVE/frontend/src/components/chart/PaneManager.ts` | Owned prior-version source | Proprietary / internal archive |
| Rust Prototype Baseline | `src/model.rs`, `src/chart.rs`, `src/app.rs` | Owned current workspace source | In-repository |

**Research workspace:** `/Users/mizuhara/workspace/sumi/research_repos/lightweight-charts-5.2.0`

---

## 2. Audit of Inherited Session R02 REA Claims

During Session R03 Task B verification, the inherited R02 trace report was audited against raw REA outputs and Lightweight Charts v5.2.0 TypeScript source:

1. **Scope Limit in REA-01 (`ev_7406bf9f42b407703c6cc89ba194cc9064a0ac9ffec842e96f938b12ad88589c`):** R02 analyzed only 6 files under `src/model/` and did not include GUI input handlers (`chart-widget.ts`, `pane-widget.ts`, `mouse-event-handler.ts`), multi-pane coordinator (`chart-model.ts`, `pane.ts`), pane views (`candlesticks-pane-view.ts`, `bars-pane-view-base.ts`), or canvas renderers (`candlesticks-renderer.ts`, `optimal-bar-width.ts`), nor did it invoke `trace_application_feature` or `trace_javascript_semantics`.
2. **Correction to REA-02 Observation:** R02 originally claimed `price-scale.ts -> ./price-scale-conversions` was recovered by REA as a resolved cross-module semantic link because both files were in scope. In reality (`VERIFIED` in both `REA-02` and `REA-05`), TypeScript source files use extensionless ESM specifiers (`./price-scale-conversions`, `../model/chart-model`), which REA's static JS/TS module resolver records under `unresolved-reference` (`static-import:...`) and `unresolved-semantic-module` (`not-found:...`) when no `.js`/`.ts` extension is written in the specifier. REA still recovers exact intra-module AST definitions, reads, aliases, closures, and call-flow spans inside each selected file.
3. **Corrections to Source Supplement Formulas in R02:**
   - `pointsArrayToCoordinates` (`price-scale.ts:489, 492`) uses `hmm = (internalHeight - 1) / (max - min)`, NOT `internalHeight / (max - min)`.
   - `KineticAnimation` (`pane-widget.ts:780–786, 807`) records `(time, timeScale.rightOffset())` in **bar units** with `dumpingCoeff = 0.997` and `MaxStartDelay = 50` ms, NOT raw pixel `x` coordinates.

---

## 3. REA MCP Operations Executed (R02 Inherited + R03 Full Pipeline Trace)

### 3.1 Inherited R02 Operations (`REA-01` .. `REA-02`)

| Op ID | Tool | Important Inputs | Result Summary | Evidence ID |
|-------|------|------------------|----------------|-------------|
| `REA-01` | `analyze_javascript_application` | `path`: `research_repos/lightweight-charts-5.2.0`, 6 `src/model/*.ts` files, `detail: "summary"` | 6 TS files parsed, 0 failures, 81,682 bytes, 10,463 AST nodes; `data-flow: 2624`, `object-flow: 195`, `closure: 114`, `call-flow: 79` | Parent: `ev_7406bf9f42b407703c6cc89ba194cc9064a0ac9ffec842e96f938b12ad88589c` / Summary view: `ev_e792f7a99c261860eaac5fc53f38d3b2241c3e766489a9a1a4759dc0ca5e203c` |
| `REA-02` | `inspect_analysis_view` | `parent_evidence_id`: `ev_7406bf9f...`, `view: "javascript-facts"`, paths: `time-scale.ts`, `price-scale.ts` | 64/114 rows (page 1); recovered `module-export: markWithGreaterWeight` (`exact`) and static import inventory | `ev_b49b23e17a5d5d7437f328552d8e28af2ebcb231145dfe2da44e52e7d02dc197` |

---

### 3.2 Session R03 End-to-End Pipeline REA Operations (`REA-03` .. `REA-14`)

#### Operation `REA-03` — `binary_session` (Live Capability & Tooling Audit)
- **Tool:** `rea:binary_session`
- **Input:** `{}`
- **Actual Output:** Server `rea-agents@6.8.0` (`/opt/homebrew/bin/rea mcp`), client `antigravity-client v1.0.0`, 229 lazy MCP tools registered, `combined_sha256: fe938a7c6eba3763b22ae2022de76dc4a7812d35d86f1d4efd36b9962b04da0b`. Available native backends: `native-macos`, `rea-artifact-graph`. Unconfigured external decompilers: `Ghidra`, `Hopper`, `IDA`, `Kuna`.
- **Evidence ID:** Session status payload (no `ev_` artifact emitted for empty session query).
- **Supported Conclusion:** REA MCP is live and healthy in this Antigravity session for static JS/TS semantic analysis and macOS binary inspection.
- **Limitation:** External commercial/GUI decompilers (`Ghidra`/`Hopper`/`IDA`) are not configured on host.

#### Operation `REA-04` — `analyze_javascript_application` (Full 19-File Input → Model → View → Renderer Scope)
- **Tool:** `rea:analyze_javascript_application`
- **Important Inputs:**
  - `path`: `/Users/mizuhara/workspace/sumi/research_repos/lightweight-charts-5.2.0`
  - `detail`: `"summary"`
  - `source_paths` (19 files across all 4 architectural layers):
    - **GUI / Input:** `src/gui/mouse-event-handler.ts`, `src/gui/chart-widget.ts`, `src/gui/pane-widget.ts`, `src/gui/price-axis-widget.ts`, `src/gui/time-axis-widget.ts`
    - **Model / Scales:** `src/model/chart-model.ts`, `src/model/pane.ts`, `src/model/time-scale.ts`, `src/model/time-scale-visible-range.ts`, `src/model/price-scale.ts`, `src/model/price-scale-conversions.ts`, `src/model/price-range-impl.ts`, `src/model/kinetic-animation.ts`, `src/model/coordinate.ts`, `src/model/range-impl.ts`
    - **Pane Views:** `src/model/series/candlesticks-pane-view.ts`, `src/model/series/bars-pane-view-base.ts`
    - **Renderers:** `src/renderers/candlesticks-renderer.ts`, `src/renderers/optimal-bar-width.ts`
- **Actual Output:**
  - Artifact SHA-256: `6a26c4adafe9539017d004f611425bc9692a278cd45d4a41ffe381afd7b6c9d5`
  - Relevant files inventoried: `1,195`; Parsed TS files: `19` (`0` parse failures, `0` skipped); Text bytes read: `282,080`; Visited AST nodes: `39,119`; Findings: `662`.
  - Retained semantic relations across the 19 files:
    - `data-flow`: **9,829**
    - `object-flow`: **1,060**
    - `closure`: **494**
    - `call-flow`: **248**
    - `timer`: **3** (`requestAnimationFrame` render invalidation loop in `chart-widget.ts`)
    - `configuration`: **2**
    - `boundary`: **1**
- **Evidence IDs:**
  - Retained parent Evidence: `ev_eea4438f22f8e6ae6c2538bd565c1f96a52a5941807bc2ac4a0cab50383bb13c`
  - Summary projection receipt: `ev_723345eb98d53cce49f94fe2515458cb5761706192107e634d3f366760013e47`
- **Supported Conclusion:** All 19 TypeScript files spanning input events, `ChartModel`/`Pane` coordination, `TimeScale`/`PriceScale` math, `BarsPaneViewBase` projection, and `PaneRendererCandlesticks` canvas drawing parse cleanly with rich intra-module data-flow (`9,829`) and timer (`3`) relations.
- **Limitation:** 1,176 files remain outside selected scope; static AST analysis does not execute JS or evaluate TypeScript interfaces at runtime.

#### Operations `REA-05` .. `REA-08` — `inspect_analysis_view` (`modules/references`)
- **Tool:** `rea:inspect_analysis_view` on parent `ev_eea4438f22f8e6ae6c2538bd565c1f96a52a5941807bc2ac4a0cab50383bb13c`
- **Executions & Evidence IDs:**
  1. `REA-05` (All modules page 0, `limit: 32`): `ev_a95b3afcae1386edab36ef7b425d8d978278b4d346160f70704ce976cdd50757` (32/392 module-reference nodes).
  2. `REA-06` (`where.path = "src/model/time-scale.ts"`): `ev_053b26b40935611f1c60510f854586d9db34d28db97251d8eba94e56eb3be63a` — recovered exact exported `TimeScale` class node `jag_node_665353cfa1dc0f90417d39da21bed901cc65abd3c622d287a396f3beb437f048` and `markWithGreaterWeight` (`jag_node_2ce2b323339f209c00f39e8c33622217de160a6c3a4ab696a5646f351899ed02`), while documenting that extensionless TS imports (`./chart-model`, `./range-impl`) are classified as `unresolved-semantic-module` (`not-found:...`).
  3. `REA-07` (`where.path = "src/gui/chart-widget.ts"`): `ev_43b616987aad8f2dddb22f6375dd9dddf591c677c801f7f17ec97679774e3e42` — recovered source module node `jag_node_21d8a87dcae7db114a47801eb7496ce3f1054806a1f21d3a75715fd995233a51` and static dependency references to `../model/chart-model`, `./pane-widget`, `./time-axis-widget`, `./pane-separator`.
  4. `REA-08` (`where.path = "src/model/series/candlesticks-pane-view.ts"`): `ev_ac00957e74f26e60e51a610407cff63c32b78eecaa0de40f957c785540417fcb` — recovered `module-export: SeriesCandlesticksPaneView` (`jag_node_44af9e2b9e44a79d89b5b4515c078ba0f564b0eae58e27fd83fa3213385962c2`) and static references to `../../renderers/candlesticks-renderer` and `./bars-pane-view-base`.

#### Operation `REA-09` — `trace_application_feature` (`TimeScale` Class Slice)
- **Tool:** `rea:trace_application_feature`
- **Important Inputs:** `evidence_id`: `ev_eea4438f22f8e6ae6c2538bd565c1f96a52a5941807bc2ac4a0cab50383bb13c`, `seed`: `jag_node_665353cfa1dc0f90417d39da21bed901cc65abd3c622d287a396f3beb437f048` (`module-export:TimeScale`), `direction`: `"outgoing"` (after `direction: "both"` hit MCP stdio's 10 MiB response limit at `10,577,883` bytes while retaining `ev_9e8c029e430053363e4c6cfe831dc284e41c513d633ecd4234c3214af026c0c4`).
- **Actual Output & Evidence ID:** `ev_97bce9491cbee5f4b260837294182078669090580717ef6b9f7b26074b4e22cb` — anchored `TimeScale` export span at `src/model/time-scale.ts` L319:1–L1253:1 (`offset 9130..36619`).
- **Limitation:** Because TypeScript extensionless imports are not cross-file linked in the application graph, `direction: "outgoing"` stops at module boundary; fine-grained intra-method state transitions require `trace_javascript_semantics`.

#### Operations `REA-10` .. `REA-14` — `trace_javascript_semantics` (State Mutation, Provenance & Render Consumption)
All queries executed against parent `ev_eea4438f22f8e6ae6c2538bd565c1f96a52a5941807bc2ac4a0cab50383bb13c`:

| Op ID | Query & Mode | Matches / Relations | Exact Verified Code Locations | Evidence ID |
|-------|--------------|---------------------|-------------------------------|-------------|
| `REA-10` | `property: "_barSpacing"`, `mode: "forward-influence"` | 25 matches across 16 functions in `time-scale.ts` | Constructor (`L357`), `applyOptions` (`L406`), `setWidth` (`L528–529`), `indexToCoordinate` (`L558`), `indexesToCoordinates` (`L570`), `setBarSpacing` (`L600–605`), `marks` (`L630, L684`), `_rightOffsetForCoordinate` (`L997`), `_setBarSpacing` (`L1011–1016`), `_updateVisibleRange` (`L1036`), `_correctBarSpacing` (`L1045–1047`), `_updateConflationFactor` (`L1087, L1093`), `_minRightOffset` (`L1124`), `_maxRightOffset` (`L1133`), `_checkRightOffsetPixels` (`L1249`) | `ev_61932519fe807df708a84d30cdad6775dd747bccb144af48c5d8e3cd852601a0` |
| `REA-11` | `property: "_barSpacing"`, `mode: "backward-provenance"` | 25 matches, 35 nodes, 10 relations (`defines: 7`, `reads: 3`) | Provenance proves **only 4 mutation sites** write `_barSpacing`: constructor (`L357`), `setWidth` (`L528–529`), `_setBarSpacing` (`L1011–1012`), and `_correctBarSpacing` (`L1045–1047`) | `ev_e6ce5dc81319a55825cd5c3c764bb62c72412ed67ab4bba309ab4639cb6e134f` |
| `REA-12` | `property: "_rightOffset"`, `mode: "backward-provenance"` | 20 matches, 33 nodes, 14 relations (`defines: 10`, `reads: 4`) | Provenance proves **all write sites** to `_rightOffset`: constructor (`L356`), `setWidth` (`L541`), `setRightOffset` (`L589`), `setBarSpacing` (`L605–606`), `scrollTo` (`L808`), `setVisibleRange` (`L882, L884`), and `_correctOffset` (`L1104, L1111`) | `ev_632b75be2af4581f72306e87d4b8460e9869d99de09a04c11be21cee112b32b99` |
| `REA-13` | `property: "_priceRange"`, `mode: "forward-influence"` | 8 matches, 12 nodes, 4 `aliases` relations in `price-scale.ts` | `setMode` (`L372–373`, `L385` log-space conversion), `priceRange` (`L443`), `setPriceRange` (`L447, L456`), `isEmpty` (`L466`), `_recalculatePriceRangeImpl` (`L1071`) | `ev_ddc3ea625b03bbf26ed1efb56d08710d434469c9920d73afd2240482974f9785` |
| `REA-14` | `property: "zoom"` & `property: "bars"`, `mode: "forward-influence"` | `zoom`: 1 match (`chart-model.ts:838`); `bars`: 2 matches (`chart-widget.ts:806`, `candlesticks-renderer.ts:61`) | `ChartModel.zoomTime` → `timeScale.zoom(pointX, scale)` (`chart-model.ts:838`, recording `jsrg_unknown_029804...` at TS interface boundary); `PaneRendererCandlesticks._drawImpl` reading `this._data.bars` (`candlesticks-renderer.ts:61`) before `_drawWicks` / `_drawBorder` / `_drawCandles` | `ev_e184c024632ee170529ac0daf91b893ca01525871ca536e6caaae4a98a431e03` (`zoom`), `ev_9fab762bb55667e6d67548be36e310956b52a700fa36cc6ba927c946a9950d42` (`bars`), `ev_a14d90561ce902cf333009c7725fa2e220430cfa6271e85e30325ca5be0ce9a2` (`_bars`: 0 matches) |

---

## 4. End-to-End Trace Synthesis (Input Event → State Transition → Coordinate Conversion → Rendering)

Combining REA static provenance (`REA-04`..`REA-14`) with direct source verification across the 19 files establishes the complete 4-stage pipeline (`VERIFIED`):

1. **Input Capture (`src/gui/`):**
   - **Wheel Zoom / Scroll:** `ChartWidget._onMousewheel` (`src/gui/chart-widget.ts:597–621`) normalizes `deltaX`/`deltaY`, bounds `zoomScale = sign(deltaY) * min(1, |deltaY|)`, computes `scrollPosition = clientX - rect.left`, and calls `model.zoomTime(scrollPosition, zoomScale)` or `model.scrollChart(deltaX * -80)`.
   - **Drag Pan & Kinetic Inertia:** `PaneWidget._pressedMouseTouchMoveEvent` (`src/gui/pane-widget.ts:737–810`) starts `KineticAnimation(0.2/bs, 7/bs, 0.997, 15/bs)` seeded with `timeScale.rightOffset()`, invokes `model.startScrollTime(event.localX)` + `model.startScrollPrice(pane, priceScale, event.localY)`, and on each move calls `model.scrollTimeTo(event.localX)` and feeds `timeScale.rightOffset()` into `_scrollXAnimation.addPosition(...)`. On release, `_endScroll` (`L681–705`) starts `_scrollXAnimation` and schedules it on `ChartModel` via `requestAnimationFrame` (`chart-widget.ts:697–728`).
2. **State Transition & Clamping (`src/model/`):**
   - `ChartModel` (`src/model/chart-model.ts:829–871`) owns **one shared `TimeScale`** across all `Pane` instances (`L507–511`). Every horizontal mutation (`zoomTime`, `scrollTimeTo`, `scaleTimeTo`, `setWidth`) mutates `_barSpacing` and/or `_rightOffset` exclusively through the write sites proven in `REA-11` and `REA-12`, enforces `_correctBarSpacing()` + `_correctOffset()`, and triggers `recalculateAllPanes()`.
   - Each `Pane` (`src/model/pane.ts:55–58, 470–488`) recalculates its own independent `_leftPriceScale`, `_rightPriceScale`, and overlay `PriceScale`s over `timeScale.visibleStrictRange()`.
3. **Batch Coordinate Projection (`src/model/series/bars-pane-view-base.ts`):**
   - `BarsPaneViewBase._convertToCoordinates` (`L40–43`) calls `timeScale.indexesToCoordinates(this._items, visibleRange)` followed by `priceScale.barPricesToCoordinates(this._items, firstValue, visibleRange)`, populating pane-local `x`, `openY`, `highY`, `lowY`, `closeY` only for the visible bar index slice.
4. **Bitmap Rendering (`src/renderers/candlesticks-renderer.ts`):**
   - `PaneRendererCandlesticks._drawImpl` (`L40–75`, proven consumer of `this._data.bars` in `REA-14`) computes `optimalCandlestickWidth(barSpacing, horizontalPixelRatio)`, enforces parity alignment with `wickWidth`, and draws 2-segment non-overlapping wicks (`_drawWicks`), inner borders (`_drawBorder`), and candle bodies (`_drawCandles`).

---

## 5. Explicit REA Limitations for TypeScript Source Analysis

| Limitation | Technical Cause (`VERIFIED`) | Mitigation Applied |
|------------|------------------------------|--------------------|
| Extensionless TS `import` specifiers remain `unresolved-semantic-module` | LWC `.ts` sources write `import { ... } from './price-scale-conversions'` without `.ts`/`.js`; REA's inert JS bundler/ESM resolver does not execute `tsconfig.json` path/extension resolution | Cross-module call edges (`ChartWidget -> ChartModel -> TimeScale -> BarsPaneViewBase -> PaneRendererCandlesticks`) were verified directly in source at exact file/line spans |
| TypeScript interface dispatch produces `jsrg_unknown_*` | Calls through `ITimeScale` / `IChartModelBase` interfaces cannot be devirtualized by syntax-only JS AST pass | Paired `trace_javascript_semantics` (`REA-10`..`REA-14`) with direct class implementation verification |
| `trace_application_feature` (`direction: "both"`) exceeded 10 MiB MCP stdio frame | Incoming containment edges pulled in all 1,195 inventoried repository files (`10,577,883` bytes) | Switched to `direction: "outgoing"` (`REA-09`) + targeted `trace_javascript_semantics` queries |
| Static AST analysis does not execute runtime branches | Inert text parsing by design | Verified runtime invariants and edge cases via deterministic automated Rust unit + property tests in `src/viewport/` |

---

## 6. Complete Evidence Registry (`REA-01` .. `REA-14`)

| Evidence ID | Operation | Subject / Query | Session |
|-------------|-----------|-----------------|---------|
| `ev_7406bf9f42b407703c6cc89ba194cc9064a0ac9ffec842e96f938b12ad88589c` | `analyze_javascript_application` | LWC 5.2.0 `src/model/` (6 files) | R02 |
| `ev_e792f7a99c261860eaac5fc53f38d3b2241c3e766489a9a1a4759dc0ca5e203c` | `inspect_analysis_view` (`summary`) | Parent `ev_7406bf9f...` | R02 |
| `ev_b49b23e17a5d5d7437f328552d8e28af2ebcb231145dfe2da44e52e7d02dc197` | `inspect_analysis_view` (`javascript-facts`) | Parent `ev_7406bf9f...` | R02 |
| `ev_eea4438f22f8e6ae6c2538bd565c1f96a52a5941807bc2ac4a0cab50383bb13c` | `analyze_javascript_application` (parent) | LWC 5.2.0 full 19-file GUI + Model + View + Renderer scope | R03 |
| `ev_723345eb98d53cce49f94fe2515458cb5761706192107e634d3f366760013e47` | `analyze_javascript_application` (summary receipt) | Parent `ev_eea4438f...` | R03 |
| `ev_a95b3afcae1386edab36ef7b425d8d978278b4d346160f70704ce976cdd50757` | `inspect_analysis_view` (`modules/references`, p.0) | Parent `ev_eea4438f...` | R03 |
| `ev_053b26b40935611f1c60510f854586d9db34d28db97251d8eba94e56eb3be63a` | `inspect_analysis_view` (`time-scale.ts`) | Export node `jag_node_665353cfa1dc...` (`TimeScale`) | R03 |
| `ev_43b616987aad8f2dddb22f6375dd9dddf591c677c801f7f17ec97679774e3e42` | `inspect_analysis_view` (`chart-widget.ts`) | Module references for `ChartWidget` | R03 |
| `ev_ac00957e74f26e60e51a610407cff63c32b78eecaa0de40f957c785540417fcb` | `inspect_analysis_view` (`candlesticks-pane-view.ts`) | Export `SeriesCandlesticksPaneView` | R03 |
| `ev_9e8c029e430053363e4c6cfe831dc284e41c513d633ecd4234c3214af026c0c4` | `trace_application_feature` (`direction: "both"`, retained >10MiB) | `TimeScale` full incoming/outgoing graph | R03 |
| `ev_97bce9491cbee5f4b260837294182078669090580717ef6b9f7b26074b4e22cb` | `trace_application_feature` (`direction: "outgoing"`) | `TimeScale` class span `L319–L1253` | R03 |
| `ev_61932519fe807df708a84d30cdad6775dd747bccb144af48c5d8e3cd852601a0` | `trace_javascript_semantics` (`_barSpacing`, `forward-influence`) | 25 matches across 16 functions in `time-scale.ts` | R03 |
| `ev_e6ce5dc81319a55825cd5c3c764bb62c72412ed67ab4bba309ab4639cb6e134f` | `trace_javascript_semantics` (`_barSpacing`, `backward-provenance`) | 4 exact mutation sites (`defines: 7, reads: 3`) | R03 |
| `ev_632b75be2af4581f72306e87d4b8460e9869d99de09a04c11be21cee112b32b99` | `trace_javascript_semantics` (`_rightOffset`, `backward-provenance`) | 20 matches, all write sites (`defines: 10, reads: 4`) | R03 |
| `ev_ddc3ea625b03bbf26ed1efb56d08710d434469c9920d73afd2240482974f9785` | `trace_javascript_semantics` (`_priceRange`, `forward-influence`) | 8 matches, 4 `aliases` relations in `price-scale.ts` | R03 |
| `ev_e184c024632ee170529ac0daf91b893ca01525871ca536e6caaae4a98a431e03` | `trace_javascript_semantics` (`zoom`, `forward-influence`) | `chart-model.ts:838` `timeScale.zoom(...)` | R03 |
| `ev_9fab762bb55667e6d67548be36e310956b52a700fa36cc6ba927c946a9950d42` | `trace_javascript_semantics` (`bars`, `forward-influence`) | `chart-widget.ts:806` & `candlesticks-renderer.ts:61` | R03 |
| `ev_a14d90561ce902cf333009c7725fa2e220430cfa6271e85e30325ca5be0ce9a2` | `trace_javascript_semantics` (`_bars`, `backward-provenance`) | 0 matches (negative control) | R03 |
| `ev_11ff50a38cf41844baddb6607057f289d5c6e1622e54794512c0ca0def753f64` | `inspect_rust_binary` | Final R03 `target/release/sumi-workstation` (`arm64`, SHA-256 `9b847fd8...`, `rustc` `58cf8f9c3`) | R03 |
| `ev_60ae4bfa2620b1643c5a21c032f5be8d3cccfeb9d6dcd11c812c324040a12040` / `ev_b24f106745a38d3661977833d96209dc11e8b07e9ca171f6611fd1db261ba329` | `inspect_macho` (`summary`) | Final R03 `target/release/sumi-workstation` (UUID `68A51A2D-3CEB-336B-BDFF-6CE3E7AC5B16`, 30 load cmds, 14 dylibs) | R03 |
