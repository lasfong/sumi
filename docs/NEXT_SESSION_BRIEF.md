# SUMI TRADING WORKSTATION — NEXT SESSION BRIEF (`v1.1.0 GO FOR RELEASE`)

- **Prepared By:** `SUMI PROGRAM ORCHESTRATOR`
- **Date:** `2026-11-20`
- **Program Status:** **`v1.1.0` (`TradingView Pro & macOS Release`: `Wave 0 -> Wave 5` COMPLETE, `280/280` automated tests, `0` clippy native/`wasm32` warnings, `0` new crates, `4/4` `viewport_bench` `p99 <= 2.883 ms < 4.0 ms`, signed `dist/Sumi Workstation.app` `v1.1.0` & `dist/Sumi-Workstation-v1.1.0-arm64.dmg`) VERIFIED `GO FOR v1.1.0`** -> [`docs/testing/V1_1_RELEASE_AUDIT.md`](testing/V1_1_RELEASE_AUDIT.md)

---

## 1. Self-Start Entry Protocol (Zero Copy-Paste Needed)

Any successor Orchestrator session starting in `/Users/mizuhara/workspace/sumi` must first read:

1. [`docs/operations/SUMI_ORCHESTRATION_CHARTER.md`](operations/SUMI_ORCHESTRATION_CHARTER.md) — Role authority, subagent protocol, evidence classification (`VERIFIED` / `INFERRED` / `UNVERIFIED` / `BLOCKED`), per-batch Git commit policy (`DEC-R04-001`), and mandatory Vietnamese communication with the Product Owner.
2. [`docs/FEATURE_BACKLOG_AND_PARITY_LEDGER.md`](FEATURE_BACKLOG_AND_PARITY_LEDGER.md) — **Master Anti-Miss Parity Ledger** confirming `100%` completion across all planned Core Waves (`S-01..S-10`, `R16-1..R16-3`, `A1..A5`, `B1..B4`, `C1..C4` `[x]`).
3. [`docs/EXECUTION_STATE.md`](EXECUTION_STATE.md) — Current baseline (`IMPLEMENTATION_PASS` across Waves 1–18 / Milestones R03–R19).
4. [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) & [`docs/DECISION_LOG.md`](DECISION_LOG.md) — Complete single-crate architecture (`src/data/`, `src/viewport/`, `src/indicators/`, `src/drawings/`, `src/replay/`, `src/trading/`, `src/scanner/`, `src/alerts/`, `src/backtest/`, `src/journal/`, `src/chart.rs`, `src/app.rs`, `src/main.rs`) and `DECISION-RENDERER-002` (`< 4.0 ms` CPU frame budget).
5. [`docs/research/RESEARCH_TO_RUST_ROADMAP.md`](research/RESEARCH_TO_RUST_ROADMAP.md) & [`docs/research/SETTINGS_VN_HOLIDAYS_DATA_AUDIT_AND_DRAWINGS_SPEC.md`](research/SETTINGS_VN_HOLIDAYS_DATA_AUDIT_AND_DRAWINGS_SPEC.md) — Shipped Wave 1–18 specifications.

---

## 2. Current Baseline Verified in `v1.1.0` (`100%` Core Parity + TradingView Pro & macOS Release)

- **Build, WASM & Tests:** `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --lib --target wasm32-unknown-unknown -- -D warnings`, and `cargo test` (`280/280` unit + property + headless UI + QA patch + `v1.1` regression tests) pass with `0` warnings and `0` errors; `0` new dependencies added in `Cargo.toml`/`Cargo.lock`.
- **Complete Shipped Subsystems (`R03–R19` + `v1.0.0` QA Patch + `v1.1.0`):**
  - `src/data/` (`csv_loader`, `json_loader`, `calendar`, `timeframe`, `presets`, `synthetic`, `audit.rs` `DataQualityAuditReport`, `UniverseDataReadinessSummary`, `OhlcEnvelopePolicy::CafeFSplitAware`)
  - `src/viewport/` (`TimeScale`, `PriceScale` + `CompactVolume` + `SignedPercentage`, `MultiChartLayoutKind`, `HotkeyProfile`, `WorkspaceTemplate`, `WorkspaceSnapshotStore`, `WorkstationSettings` + `show_drawing_rail` + `4` `CandlePalettePreset`s)
  - `src/indicators/` (`SMA`, `EMA`, `BB`, `RSI`, `MACD`, `ATR`, `VRVP`/`FRVP`, `VWAP`, `TPO`, `RS`, `Ichimoku`, `CVD`, `patterns.rs` `14` candlestick patterns, `extended.rs` `ADX`/`Stoch`/`SuperTrend`/`PSAR`/`Keltner`/`CCI`/`MFI`/`VolSMA`, `proxy_bb.rs` `T03..T200`)
  - `src/drawings/` (**`11` tools**: `H-Line`, `TrendLine` dual-end, `Ray`, `ParallelChannel` 3-point, `Rectangle`, `Fib` custom `1..=16`, `RiskRewardBox`, `PriceDateRange Measure` + `Shift+Click`, `FRVP`, `AnchoredVwap`, `TextNote`, & `DrawingPropertyPatch`)
  - `src/replay/`, `src/trading/` (`portfolio.rs` + `enforce_vn_t2_lot_lock` `SL`/`TP` bracket cap + deterministic `ModifyOrderPrice` replay command, `ladder.rs`, `coaching.rs`), & `src/journal/` (`VIETNAM_HOLIDAYS_V1` `[i32; 151]` `2018–2030`, `T+3`/`T+2` `13:00 ICT` `HoldingLot` FIFO ledger, `planner.rs`, `ledger.rs`)
  - `src/scanner/` (`101`-entry `SIGNAL_CATALOG` + confirmed `right_bars = 3` divergences + `PrecomputedCatalogMatrix`, `breadth.rs`, `proxy_bb.rs` `MarketBBAggregator`, `universe.rs` `UniverseResolver`)
  - `src/alerts/` (`PriceLevel`, `RSI`, `MACD`, `DrawingLine` synced with dual-end `TrendLine`, `SignalCatalogTrigger`, `ConfluenceThreshold`, `cooldown_bars`, CSV/JSON export)
  - `src/backtest/` (`7` presets incl. `CustomSignalRules`, `run_pit_episodes_backtest`, `run_portfolio_backtest_with_universe`, `WFO`, `SplitMix64` Monte Carlo, `MultiStrategyComparisonReport`, `AdvancedTearsheetAnalytics`, `RunManifest`, & **`v1.1` `report_html.rs` self-contained HTML5/SVG/Print-to-PDF export**)
  - `src/chart.rs`, `src/app.rs` & `src/main.rs` (**`v1.1`** `38 px` Left Vertical Drawing Rail with vector icons, Finder drag-and-drop `.csv`/`.tsv`/`.txt`/`.json` + CLI file argument, Chart-Direct `SL`/`TP`/Pending drag + `[×]` cancel/flatten + crisp Z-order above price lines, `[Settings]`, `[Data Audit]`, `[Export PNG]`, `--r12/--r17/--r18/--r19` all `overall_pass: true`, worst `p99 = 2.883 ms < 4.0 ms`)
  - **macOS Release Packaging (`v1.1.0`):** `scripts/generate_app_icon.py` -> `assets/icons/AppIcon.icns`, `scripts/package_macos_app.sh` -> signed `dist/Sumi Workstation.app` (`v1.1.0`, `CFBundleDocumentTypes` `.csv`/`.json`), `scripts/package_macos_dmg.sh` -> verified `dist/Sumi-Workstation-v1.1.0-arm64.dmg` (`8.8 MB`).

---

## 3. Optional Post-v1.1 Directions (Awaiting Explicit Product Owner Selection)

1. **Git Synchronization & Release Tagging:** Review working tree, create clean commit(s) for `R19` + `v1.0.0 QA Patch` + `v1.1.0`, tag `v1.1.0`, and push to `origin/master` (only upon explicit PO command).
2. **Apple Developer ID Signing & Notarization:** Replace ad-hoc `codesign -` with Developer ID Application certificate + `notarytool` for Gatekeeper distribution outside the local machine.
3. **Standalone External Data-Sync CLI (Decoupled from Core Crate per `D1`):** Build an isolated companion CLI/script fetching daily EOD OHLCV (`CafeF`, `Binance`, `Yahoo`) into workspace CSV/JSON files without adding network crates to `sumi-workstation`.
4. **WASM Browser Build & Deployment (`trunk` / WebGPU-WebGL2):** Package the already `wasm32-unknown-unknown`-clean library for browser preview if needed.

---

## 4. Standing Constraints & Product Owner Gates

- **Product Owner Authorization Gate:** Wait for explicit Product Owner instruction before starting any post-R19 workstream, `git push`, or `git tag`.
- **Language:** Communicate 100% in **Vietnamese (`tiếng Việt`)** with the Product Owner.
- **Git & File Safety (`DEC-R04-001`):** Never touch `src/model.rs`, `ARCHIVE/`, or historical `docs/evidence/*`; never run `git reset`, `git clean`, unrequested `git push`, or `git tag`.
- **Authorized Targets Only (`DEC-006`):** Never inspect or interact with TradingView Desktop.
