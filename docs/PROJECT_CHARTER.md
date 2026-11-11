# SUMI TRADING WORKSTATION — PROJECT CHARTER

- **Session Identity:** `SESSION-01-20261110T160909+0700`
- **Repository Root:** `/Users/mizuhara/workspace/sumi`
- **Archived Legacy Codebase:** `/Users/mizuhara/workspace/sumi/ARCHIVE` (preserved read-only; out of active runtime scope)

## 1. Mission Statement

Build an independent, high-quality, TradingView-inspired financial charting workstation in Rust:

1. **Native macOS Desktop Application:** Built with Rust, `egui`, and `eframe` using hardware-accelerated native rendering (`wgpu` / `glow`) without Electron, Tauri, React, or Next.js.
2. **Professional Financial Chart Interactions:** Stable coordinate mapping, crisp candlestick and volume rendering, responsive crosshair/HUD inspection, and smooth pan/zoom.
3. **WebAssembly-Compatible Domain Core:** Keep domain models (`Candle`, `PriceBounds`, `ViewportMapping`) and future indicator/drawing engines free of OS-specific dependencies so they can compile to `wasm32-unknown-unknown` in future waves.
4. **Evidence-Driven Engineering:** Every session produces compiled artifacts, automated unit/lint test evidence, real GUI launch verification, and repository-native handoff state.

## 2. Clean-Room & Security Principles

- TradingView Desktop serves strictly as a visual and UX quality benchmark, **never** as an authorized target for binary reverse engineering, proprietary source extraction, credential/cookie inspection, or market-feed scraping.
- No destructive Git cleanup (`git reset --hard`, `git clean`, `git checkout --`) or unapproved Git commits/tags.
- No `curl | sh` installers, automatic `sudo`, or automatic macOS TCC permission modifications.
- All shared developer tool or MCP configuration changes require explicit Product Owner approval and backup preservation.

## 3. Session 01 Scope Boundary

- **In Scope:**
  - Phase A: Environment discovery & authorized Rust (`rustup` / `stable-aarch64-apple-darwin`) + Node (`26.13.0_2`) bootstrap.
  - Phase B: Authorized `rea-agents@6.8.0` installation, dry-run inspection, and Antigravity-scoped MCP/skill registration.
  - Phase C & D: Clean modular Rust architecture (`model`, `data`, `chart`, `app`) and first runnable native macOS dark-themed candlestick chart with deterministic synthetic OHLCV data, time/price axes, zoom/pan, and crosshair HUD.
  - Phase E: Full verification (`cargo fmt --check`, `cargo check`, `cargo clippy`, `cargo test`, `cargo build`) and real native GUI execution with captured visual evidence.
- **Out of Scope for Session 01:**
  - Technical indicators, multi-pane layouts, bar replay, broker/live market feed integration, or complex drawing toolkits.
