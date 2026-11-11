# SUMI TRADING WORKSTATION — DECISION LOG

- **Session Identity (Created):** `SESSION-01-20261110T160909+0700`
- **Updated By:** `SESSION-R03-20261111T105832+0700` (`SUMI PROGRAM ORCHESTRATOR`)

## Decisions Recorded in Session 01

### DEC-001 — Archive Legacy Webapp & Reuse Current Repository Root
- **Date:** `2026-11-10`
- **Context:** The repository previously held a Python/TypeScript web application experiment. The Product Owner requested archiving all legacy files into `ARCHIVE/` and building the new native macOS Rust Trading Workstation in the repository root.
- **Decision:** Move all legacy project files into `/Users/mizuhara/workspace/sumi/ARCHIVE/` (preserving `ARCHIVE/.gitignore` and all untracked local files) and initialize the Rust crate directly at `/Users/mizuhara/workspace/sumi/Cargo.toml`. Do not modify or delete `ARCHIVE/`.

### DEC-002 — Official Non-Pipe Rust Toolchain Bootstrap (`brew install rustup`)
- **Date:** `2026-11-10`
- **Context:** Phase A diagnostics showed `rustc`, `cargo`, and `rustup` were not installed, while Xcode Command Line Tools (`/Library/Developer/CommandLineTools`) and Homebrew (`/opt/homebrew/bin/brew`) were present. Security rules prohibit `curl | sh` installers and require Product Owner approval before installing toolchains.
- **Decision:** With explicit Product Owner approval, installed `rustup 1.29.3` via Homebrew bottle (`brew install rustup`) and initialized the `stable-aarch64-apple-darwin` toolchain (`rustc 1.99.1`, `cargo 1.99.1`).

### DEC-003 — Upgrade Host Node.js to `26.13.0_2` for Official REA `6.8.0` Compatibility
- **Date:** `2026-11-10`
- **Context:** Official `rea-agents@6.8.0` requires Node.js `^22.23.4 || ^24.22.1 || >=26.12.1`. Host Node.js was `v26.0.0`, which is unsupported by REA's runtime validator.
- **Decision:** With explicit Product Owner approval, upgraded Homebrew `node` from `26.0.0` to `26.13.0_2`, installed `rea-agents@6.8.0` globally from the official npm registry, ran a dry-run first, and scoped setup exclusively to `--client antigravity` without installing optional native decompilers (Hopper/Ghidra/IDA) or altering macOS TCC permissions.

### DEC-004 — Custom `egui::Painter` Candlestick Renderer Over Heavy Third-Party Chart Crates
- **Date:** `2026-11-10`
- **Context:** Financial charting workstations require exact control over price-to-screen and index-to-screen coordinate mapping, axis gutter layout, candle wick/body crispness, and crosshair HUD overlays.
- **Decision:** Built a clean, deterministic coordinate mapper (`ViewportMapping` in `src/model.rs`) and custom `egui::Painter` renderer (`src/chart.rs`) directly on `egui 0.36.0` and `eframe 0.36.0`. Domain logic in `src/model.rs` and `src/data.rs` has zero OS-specific dependencies to guarantee future `wasm32-unknown-unknown` compatibility.


---

## Decisions Recorded in Session R01

### DEC-005 — REA-First Research Gate Before Rust Implementation
- **Date:** `2026-11-10`
- **Session:** `SESSION-R01-20261110T202837+0700`
- **Decision:** No new Rust features (zoom, pan, indicators, drawings, replay) are implemented until the relevant research gate is cleared via REA analysis and documented algorithm specs. The wave-based implementation roadmap (RESEARCH_TO_RUST_ROADMAP.md) governs implementation order.

### DEC-006 — TradingView Desktop Analysis Not Authorized
- **Date:** `2026-11-10`
- **Session:** `SESSION-R01-20261110T202837+0700`
- **Decision:** No binary decompilation, resource extraction, API interception, or proprietary code analysis of TradingView Desktop will be performed in this project. The blocker is permanently recorded. LWC v5 (Apache 2.0) and open-source TA-Lib are the authorized reference implementations.

---

## Decisions Recorded in Session R02

### DECISION-RESEARCH-001 — LWC v5.2.0 as Authorized Coordinate Reference
- **Date:** `2026-11-10`
- **Session:** `SESSION-R02-20261110T230337+0700`
- **Context:** Need an authoritative, legally usable reference for professional chart coordinate and viewport algorithms. The archived Sumi TypeScript application delegates all coordinate math to LWC internally — it is not an independent reference.
- **Decision:** Pin TradingView `lightweight-charts` v5.2.0 (Apache 2.0, commit `868cae2`, tag `v5.2.0`) as the authorized coordinate algorithm reference. Research clone at `research_repos/lightweight-charts-5.2.0`. This is a research-only asset; no LWC code will be copied into production `src/`.

### DECISION-CAMERA-001 — Replace ViewportMapping with ViewportCamera
- **Date:** `2026-11-10`
- **Session:** `SESSION-R02-20261110T230337+0700`
- **Context:** Research confirmed `ViewportMapping` is a stateless f32 prototype that cannot support: continuous scroll offset (`rightOffset`), cursor-anchored zoom (`baseIndex` + `barSpacing`), future-space at the right edge, kinetic scroll deceleration, or proper price margins. Six defect categories documented in `LEGACY_SUMI_COORDINATE_COMPARISON.md`.
- **Decision:** Implement `ViewportCamera` in `src/viewport.rs` (new module) as the replacement. All coordinate state must use f64. All 11 algorithms in `COORDINATE_ALGORITHM_SPEC.md` must be verified by property-based tests and numerical fixtures before `ViewportMapping` is deleted.

### DECISION-RENDERER-001 — Retain egui::Painter as Primary Rendering Backend
- **Date:** `2026-11-10`
- **Session:** `SESSION-R02-20261110T230337+0700`
- **Context:** Evaluated egui::Painter, raw wgpu, Skia, and femtovg for the chart rendering backend.
- **Decision:** Retain `egui::Painter` backed by `wgpu` (Metal on macOS). Rationale: already integrated, GPU-accelerated, adequate for 10k+ bars with clipping/conflation, fully capable for 2D drawing tools. Deferred: custom wgpu pipeline — revisit only if CPU tessellation exceeds 3ms budget at target bar count.

---

## Decisions Recorded in Session R03

### DEC-R03-001 — Modular `src/viewport/` Architecture Over Monolithic `ViewportCamera` (Revises `DECISION-CAMERA-001`)
- **Date:** `2026-11-11`
- **Session:** `SESSION-R03-20261111T105832+0700`
- **Context:** Auditing `RUST_VIEWPORT_CAMERA_DESIGN.md` against Lightweight Charts v5.2.0 (`src/model/chart-model.ts`, `pane.ts`, `time-scale.ts`, `price-scale.ts`) revealed that R02's proposed `ViewportCamera` collapsed shared horizontal time state, per-pane/axis vertical price state, and screen offsets into a single struct — which breaks multi-pane synchronization, dual/overlay price axes, and non-zero screen-origin panes. In addition, Product Owner instructions strictly forbid deleting existing repository artifacts or code (`ViewportMapping`).
- **Decision:** Implement the viewport engine as a modular `f64` subsystem in `src/viewport/` (`TimeScale` shared across panes, independent `PriceScale` per pane/axis, explicit 3-space `PaneViewport` bridge, `KineticAnimation` in bar units, and `candle_metrics` with zero-allocation OHLCV conflation). Retain Session 01's `ViewportMapping` in `src/model.rs` with `#[deprecated(...)]` rather than deleting it.

### DEC-R03-002 — Reclassify Session R02 Acceptance to `ACCEPTED WITH SCOPE LIMITS (RECTIFIED IN R03)`
- **Date:** `2026-11-11`
- **Session:** `SESSION-R03-20261111T105832+0700`
- **Context:** Direct source verification against LWC v5.2.0 (`868cae2`) uncovered 16 mathematical, edge-case, and architectural defects in R02's specs (`_coordinateToFloatIndex` precedence bug, half-slot `49.5` vs `50.0` fixture error, `internal_height == 1.0` division by zero, broken negative `to_log` parentheses and missing adaptive `LogFormula`, panicking `f64::clamp` when `min > max`, fabricated `rightBarStaysOnScroll_coefficient`, double-margin `fit_price_range`, wrong `KineticAnimation` units, etc.), plus REA traces in R02 only covered 6 static `src/model/` files without `gui/` or `renderers/` call-flow tracing.
- **Decision:** Record Session R02's pre-audit state honestly as `ACCEPTED WITH SCOPE LIMITS`, rectify all 4 R02 research/design specs in-place with full audit logs, and expand live REA MCP coverage with 18 live R03 operations (`ev_eea4438f...`, `ev_97bce949...`, `ev_632b75be...`, `ev_11ff50a3...`, `ev_60ae4bfa...`, etc.) spanning `gui -> model -> renderers` and the R03 release Mach-O binary.

### DECISION-RENDERER-002 — Provisional Confirmation of `egui::Painter` With Mandatory Conflation & Explicit `wgpu` Triggers (Revises `DECISION-RENDERER-001`)
- **Date:** `2026-11-11`
- **Session:** `SESSION-R03-20261111T105832+0700`
- **Context:** Executed reproducible `--release` benchmarks (`examples/viewport_bench.rs`) across `10,000`, `50,000`, and `100,000` bars on Apple M1 (`2560×1600` Retina). Unconflated full-fit rendering of `50K` (`21.13 ms`) and `100K` (`39.57 ms`, `2.53M` vertices) exceeds the `16.67 ms` 60 FPS budget due to `epaint` CPU tessellation, whereas **with** R03's O(1) culling + DPR-aware power-of-2 conflation, `100K` full-fit renders in **`1.47 ms`** (**26.9× faster**), standard zoom renders in **`0.094–0.102 ms`**, and 3-pane shared-`TimeScale` dense view (`2,907` bars) renders in **`1.14 ms`**.
- **Decision:** Retain `egui::Painter` for Waves 1–4 with **mandatory** O(1) culling + DPR-aware conflation enabled by default. Define quantitative triggers (`> 4.0 ms` CPU frame median, `> 12,000` simultaneous non-conflatable shapes, or `> 350,000` vertices/frame) to introduce hybrid `egui_wgpu::CallbackTrait` instanced `wgpu` rendering for future heavy series without altering `src/viewport/`.

### DEC-R03-004 — Establish Durable Repository-Native Program Orchestration Charter
- **Date:** `2026-11-11`
- **Session:** `SESSION-R03-20261111T105832+0700`
- **Context:** Verified live Antigravity runtime capabilities (`swarm`, `define_subagent`/`invoke_subagent`, `rea` MCP) and needed a zero-copy-paste continuity mechanism across sessions.
- **Decision:** Establish [`docs/operations/SUMI_ORCHESTRATION_CHARTER.md`](operations/SUMI_ORCHESTRATION_CHARTER.md) governing role separation (`Orchestrator`, `Research`, `Engineering`, `Critic`, `Auditor`), Evidence Classification (`VERIFIED` / `INFERRED` / `UNVERIFIED` / `BLOCKED`), and Product Owner decision gates.
