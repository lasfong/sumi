# SUMI TRADING WORKSTATION — DEPENDENCIES (SESSION 01)

- **Session Identity:** `SESSION-01-20261110T160909+0700`
- **Manifest:** [`Cargo.toml`](file:///Users/mizuhara/workspace/sumi/Cargo.toml)

## 1. Host Toolchain Inventory

| Component | Version / Path | Status | Installation / Origin |
| --- | --- | --- | --- |
| Operating System | macOS `27.2` (`26B123`) | Pre-existing | Apple Silicon `arm64` |
| Xcode Command Line Tools | `/Library/Developer/CommandLineTools` | Pre-existing | Preserved as-is |
| `rustup` | `1.29.3` (`/opt/homebrew/opt/rustup/bin/rustup`) | Authorized & Installed | `brew install rustup` |
| `rustc` | `1.99.1 (58cf8f9c3 2026-10-13)` | Authorized & Installed | `rustup default stable` (`stable-aarch64-apple-darwin`) |
| `cargo` | `1.99.1 (5f94df478 2026-08-27)` | Authorized & Installed | `rustup default stable` |
| `node` | Upgraded from `v26.0.0` to `26.13.0_2` | Authorized & Upgraded | `brew upgrade node` (satisfies REA `>=26.12.1` requirement) |
| `npm` | `11.12.1+` | Pre-existing / Paired with Node | Homebrew `node` |

## 2. Direct Rust Crate Dependencies

All direct dependencies are stable releases on crates.io selected from the official `emilk/eframe_template` compatibility matrix:

| Crate | Version | Features | Target Scope | Rationale |
| --- | --- | --- | --- | --- |
| `egui` | `0.36.0` | Default | All (`native` + `wasm32`) | Immediate-mode GUI & 2D vector `Painter` for high-frequency financial chart rendering. |
| `eframe` | `0.36.0` | `["persistence"]` | All (`native` + `wasm32`) | Official `egui` application framework providing native macOS windowing and state persistence. |
| `serde` | `1.0.219` | `["derive"]` | All (`native` + `wasm32`) | Serialization/deserialization for domain models (`Candle`, `PriceBounds`) and persisted workstation settings. |
| `log` | `0.4.27` | Default | All (`native` + `wasm32`) | Lightweight facade for structured diagnostics. |
| `env_logger` | `0.11.8` | Default | `cfg(not(target_arch = "wasm32"))` | Native stderr logger controlled via `RUST_LOG`. |
| `png` | `0.18` | Default | `cfg(not(target_arch = "wasm32"))` | Native Metal framebuffer screenshot encoding (`--capture-png`, added in Session 01). |

## 3. External Tooling Dependencies

| Package | Version | Source & Integrity | Purpose |
| --- | --- | --- | --- |
| `rea-agents` | `6.8.0` | `https://registry.npmjs.org/rea-agents/-/rea-agents-6.8.0.tgz` (`sha512-9QDeRnGqEDzABZyr04eLnBXBv/c6I9/TWu/6M0MD4pEfkySI/BIgwTODIbAReX3C1x6hjw9+Jlx4A2yUSPZ9ww==`) | Official REA CLI & MCP server for binary/application inspection and diagnostic workflows. |
