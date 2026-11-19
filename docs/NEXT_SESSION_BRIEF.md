# SUMI TRADING WORKSTATION — NEXT SESSION BRIEF (`SESSION R19 / WAVE 18`)

- **Prepared By:** `SESSION-R18-20261119T085800+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Date:** `2026-11-19`
- **Target Milestone (Awaiting PO Approval):** `Milestone R19` — Wave 18 Interactive Global `[Settings]` Modal & Candle Palettes, Official Vietnam Exchange Holiday Calendar (`VIETNAM_HOLIDAYS_V1`) + FIFO `T+2` Lot Release, Data Readiness & Gap/Split Audit Panel, & `TextNote` Callout Drawing + Custom Fibonacci Levels + Toolbar `[Export PNG]` (`FEATURE_BACKLOG_AND_PARITY_LEDGER.md` Group C: `C1..C4` — **Final Planned Core Parity Wave**)

---

## 1. Self-Start Entry Protocol (Zero Copy-Paste Needed)

Any successor Orchestrator session starting in `/Users/mizuhara/workspace/sumi` must first read:

1. [`docs/operations/SUMI_ORCHESTRATION_CHARTER.md`](operations/SUMI_ORCHESTRATION_CHARTER.md) — Role authority, subagent protocol, evidence classification (`VERIFIED` / `INFERRED` / `UNVERIFIED` / `BLOCKED`), per-batch Git commit policy (`DEC-R04-001`), and mandatory Vietnamese communication with the Product Owner.
2. [`docs/FEATURE_BACKLOG_AND_PARITY_LEDGER.md`](FEATURE_BACKLOG_AND_PARITY_LEDGER.md) — **Mandatory Anti-Miss Parity Ledger** tracking every completed (`S-01..S-10`, `R16-1..R16-3`, `A1..A5`, `B1..B4`) and remaining queued item (`Milestone R19`: `C1..C4`). Always update item statuses here when closing or proposing milestones so nothing is ever lost.
3. [`docs/EXECUTION_STATE.md`](EXECUTION_STATE.md) — Current baseline (`IMPLEMENTATION_PASS` across Waves 1–17 / Milestones R03–R18).
4. [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) & [`docs/DECISION_LOG.md`](DECISION_LOG.md) — Modular `src/data/`, `src/viewport/` (`SignedPercentage`, `WorkspaceSnapshotStore`), `src/indicators/` (`patterns.rs`, `extended.rs`, `proxy_bb.rs`), `src/drawings/`, `src/replay/`, `src/trading/` (`coaching.rs`, `ladder.rs`), `src/scanner/` (`101`-entry `SIGNAL_CATALOG`, `PrecomputedCatalogMatrix`, `proxy_bb.rs`, `universe.rs`), `src/alerts/`, `src/backtest/` (`CustomSignalRuleSpec`, `run_pit_episodes_backtest`, `run_portfolio_backtest_with_universe`, `analytics.rs`, `walk_forward.rs`, `monte_carlo.rs`), `src/journal/`, and `DECISION-RENDERER-002` (`< 4.0 ms` CPU frame budget).
5. [`docs/research/RESEARCH_TO_RUST_ROADMAP.md`](research/RESEARCH_TO_RUST_ROADMAP.md) & [`docs/research/SIGNALS_72_INDICATORS_PROXYBB_AND_PIT_UNIVERSE_SPEC.md`](research/SIGNALS_72_INDICATORS_PROXYBB_AND_PIT_UNIVERSE_SPEC.md) — Shipped Wave 1–17 specifications and proposed Wave 18 (`C1..C4`) scope.

---

## 2. Current Baseline Verified in R18

- **Build, WASM & Tests:** `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --lib --target wasm32-unknown-unknown -- -D warnings`, and `cargo test` (`255/255` unit + property + headless UI + Critic R03–R18 regression tests) pass with `0` warnings and `0` errors; `Cargo.toml` and `Cargo.lock` have `0` new dependencies.
- **Shipped Subsystems (`R03–R18`):**
  - `src/data/`, `src/viewport/` (`TimeScale`, `PriceScale` + `CompactVolume` + `SignedPercentage`, `MultiChartLayoutKind`, `HotkeyProfile`, `WorkspaceTemplate` v3, & `WorkspaceSnapshotStore`)
  - `src/indicators/` (`SMA`, `EMA`, `BB`, `RSI`, `MACD`, `ATR`, `VRVP`/`FRVP`, `VWAP`, `TPO`, `RS`, `Ichimoku`, `CVD`, & **R18** `patterns.rs`, `extended.rs` `ADX`/`Stoch`/`SuperTrend`/`PSAR`/`Keltner`/`CCI`/`MFI`/`VolSMA`, `proxy_bb.rs` `T03..T200`)
  - `src/drawings/` (`8` geometries + `DrawingPropertyPatch`), `src/replay/`, `src/trading/` (`portfolio.rs`, `ladder.rs`, `coaching.rs`), & `src/journal/`
  - `src/scanner/` (`101`-entry `SIGNAL_CATALOG` + confirmed `right_bars = 3` divergences + `PrecomputedCatalogMatrix`, `breadth.rs`, & **R18** `proxy_bb.rs` `MarketBBAggregator`, `universe.rs` `UniverseResolver`)
  - `src/alerts/` (`PriceLevel`, `RSI`, `MACD`, `DrawingLine`, `SignalCatalogTrigger`, `ConfluenceThreshold`, `cooldown_bars`, CSV/JSON export)
  - `src/backtest/` (`7` presets incl. `CustomSignalRules`, `run_pit_episodes_backtest`, `run_portfolio_backtest_with_universe`, `WFO`, `SplitMix64` Monte Carlo, `MultiStrategyComparisonReport`, `AdvancedTearsheetAnalytics`, `RunManifest`)
  - `src/chart.rs`, `src/app.rs` & `src/main.rs` (`--r18` holding all UI workloads `< 4.0 ms`: **`0.231–0.955 ms`** mean / **`p99 <= 1.340 ms`**, `"overall_pass": true`)

---

## 3. Recommended R19 Plan (`Wave 18 — Global Settings, Vietnam Exchange Holidays & FIFO T+2, Data Quality Audit Panel, & TextNote / Custom Fib / Export PNG`, `C1..C4`)

### Workstream 1 — Research Gate (`DEC-005` Compliance)
1. Execute live REA MCP JS/TS analysis on `ARCHIVE/frontend/src/{components/settings/SettingsModal.tsx,pages/DashboardPage.tsx,pages/ImportPage.tsx,features/drawings/drawingDomain.ts,components/chart/DrawingInspector.tsx}` and `LWC v5.2.0` text/screenshot/candlestick-color primitives alongside read-only mathematical inspection of `ARCHIVE/backend/app/{domain/market/calendar.py,domain/market/settlement.py,services/data_quality_service.py,services/readiness_service.py,services/cafef_importer.py}`; author the R19 specification in `docs/research/`.

### Workstream 2 — Wave 18 Domain & UI Enhancements (`C1..C4` in `FEATURE_BACKLOG_AND_PARITY_LEDGER.md`)
1. **`C1` — Interactive Global `[Settings]` Modal & Candle Color Palettes (`src/viewport/`, `src/chart.rs`, `src/app.rs`):** Default venue (`HOSE/HNX/UPCOM/VN30F/US/Crypto`), VN price unit display (`Auto` / `Thousand VND` / `Raw VND`), default capital/fees/tax/slippage/risk-per-trade, candlestick color themes (`Emerald/Crimson`, `Hollow Candles`, `High-Contrast Colorblind`), and auto-snapshot cadence.
2. **`C2` — Official Vietnam Exchange Holiday Calendar (`VIETNAM_HOLIDAYS_V1`) & FIFO `T+2` Holding Lot Release (`src/journal/market_rules.rs`, `src/trading/`, `src/backtest/`):** Official Lunar New Year (`Tết Nguyên Đán`), Hùng Kings (`10/3 AL`), `30/04–01/05`, National Day `02/09`, and swap holidays (`2018–2030`) + FIFO `HoldingLot` `sellable_at` (`13:00 ICT` VSD settlement) tracking.
3. **`C3` — Data Readiness & Gap/Split Audit Panel (`src/data/`, `src/app.rs`):** Per-symbol CSV/JSON health audit: trading-day coverage vs `TradingCalendar`, missing-session detector, outlier range/volume bars, and `CafeF SolieuGD` 4-decimal post-split rounding tolerance audit (`<= 6 bps` / `<= 25 VND`).
4. **`C4` — `TextNote` Callout Drawing, Per-Drawing Custom Fibonacci Levels, Dual-End Trendline Extension & `[Export PNG]` Toolbar Button (`src/drawings/`, `src/chart.rs`, `src/app.rs`, `src/main.rs`):** `DrawingTool::TextNote` (`<= 280` UTF-8 chars anchored at `(bar, price)`), per-drawing custom Fibonacci ratios (`-0.272`, `0.705`, `1.272`, `1.618`, ...), `extend_left` / `extend_right` on `TrendLine`, and 1-click `[Export PNG]` routing through `OffscreenWgpuHarness`.

### Workstream 3 — `--r19` Release Benchmark, Offscreen Metal Captures, Critic, Audit & Clean Commit
1. Pass all 4 Cargo gates, run `--release` `--r19` (`< 4.0 ms` CPU frame budget) + `SUMI_CAPTURE_BATCH=r19`, update `FEATURE_BACKLOG_AND_PARITY_LEDGER.md` (`C1..C4 -> [x]`), pass `sumi-critic` (`CRITIC_PASS`) and `sumi-auditor` (`AUDIT_PASS`), and commit cleanly on `master` per `DEC-R04-001`.

---

## 4. Standing Constraints & Product Owner Gates

- **Product Owner Authorization Gate:** Wait for explicit Product Owner approval before starting Milestone R19 implementation.
- **Language:** Communicate 100% in **Vietnamese (`tiếng Việt`)** with the Product Owner.
- **Git & File Safety (`DEC-R04-001`):** Never touch `Cargo.toml`, `Cargo.lock`, `src/model.rs`, `ARCHIVE/`, or R01–R18 `docs/evidence/*`; never run `git reset`, `git clean`, `git push`, or `git tag`.
- **Authorized Targets Only (`DEC-006`):** Never inspect or interact with TradingView Desktop.
