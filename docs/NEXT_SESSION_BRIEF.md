# SUMI TRADING WORKSTATION — NEXT SESSION BRIEF (`SESSION R17 / WAVE 16`)

- **Prepared By:** `SESSION-R16-20261118T155000+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Date:** `2026-11-18`
- **Target Milestone (Awaiting PO Approval):** `Milestone R17` — Wave 16 Replay Practice Coaching (`PracticeScoreboard` & `SessionDebriefModal`), 1-Click `Scanner -> Replay` (`BlindPractice`/`SignalReview`) & Saved Sessions, Normalized `0%` Multi-Strategy Equity Comparison, & Outlier/Drawdown/Regime Tearsheet Analytics (`FEATURE_BACKLOG_AND_PARITY_LEDGER.md` Group A: `A1..A5`)

---

## 1. Self-Start Entry Protocol (Zero Copy-Paste Needed)

Any successor Orchestrator session starting in `/Users/mizuhara/workspace/sumi` must first read:

1. [`docs/operations/SUMI_ORCHESTRATION_CHARTER.md`](operations/SUMI_ORCHESTRATION_CHARTER.md) — Role authority, subagent protocol, evidence classification (`VERIFIED` / `INFERRED` / `UNVERIFIED` / `BLOCKED`), per-batch Git commit policy (`DEC-R04-001`), and mandatory Vietnamese communication with the Product Owner.
2. [`docs/FEATURE_BACKLOG_AND_PARITY_LEDGER.md`](FEATURE_BACKLOG_AND_PARITY_LEDGER.md) — **Mandatory Anti-Miss Parity Ledger** tracking every completed (`S-01..S-10`, `R16-1..R16-3`) and queued item across `Milestone R17` (`A1..A5`), `Milestone R18` (`B1..B4`), and `Milestone R19` (`C1..C4`). Always update item statuses here when closing or proposing milestones so nothing is ever lost.
3. [`docs/EXECUTION_STATE.md`](EXECUTION_STATE.md) — Current baseline (`IMPLEMENTATION_PASS` across Waves 1–15 / Milestones R03–R16).
4. [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) & [`docs/DECISION_LOG.md`](DECISION_LOG.md) — Modular `src/data/`, `src/viewport/` (including `WorkspaceSnapshotStore`), `src/indicators/`, `src/drawings/`, `src/replay/`, `src/trading/`, `src/scanner/` (including `PrecomputedCatalogMatrix`), `src/alerts/`, `src/backtest/` (including `CustomSignalRuleSpec`, `walk_forward.rs`, & `monte_carlo.rs`), `src/journal/`, and `DECISION-RENDERER-002` (`< 4.0 ms` CPU frame budget).
5. [`docs/research/RESEARCH_TO_RUST_ROADMAP.md`](research/RESEARCH_TO_RUST_ROADMAP.md) & [`docs/research/RULE_BUILDER_ALERTS_AND_SNAPSHOT_RECOVERY_SPEC.md`](research/RULE_BUILDER_ALERTS_AND_SNAPSHOT_RECOVERY_SPEC.md) — Shipped Wave 1–15 specifications and proposed Wave 16–18 scope.

---

## 2. Current Baseline Verified in R16

- **Build, WASM & Tests:** `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo clippy --lib --target wasm32-unknown-unknown -- -D warnings`, and `cargo test` (`235/235` unit + property + headless multi-pane/multi-chart/DOM/hotkey/confluence/MC/rule-builder/snapshot UI + Critic R03–R16 regression tests) pass with `0` warnings and `0` errors; `Cargo.toml` and `Cargo.lock` have `0` new dependencies.
- **Shipped Subsystems (`R03–R16`):**
  - `src/data/`, `src/viewport/` (`TimeScale`, `PriceScale` + `CompactVolume` + right-axis polish, `MultiChartLayoutKind`, `HotkeyProfile` `18` actions, `WorkspaceTemplate` `6` presets, & **R16** `16`-slot `WorkspaceSnapshotStore` + `4`-slot `auto_ring` + 64-bit `CorruptSnapshotDigest` + diff + quarantine recovery)
  - `src/indicators/`, `src/drawings/` (`8` geometries + `DrawingPropertyPatch`), `src/replay/`, `src/trading/` (incl. `OHLCV_SYNTH_DOM_PROXY` ladder), & `src/journal/` (`HOSE/HNX/UPCOM/VN30F` microstructure, Risk Planner, & `TradeJournalStore`)
  - `src/scanner/` (`evaluate_symbol_scan_and_signals`, `breadth.rs` VN30 Breadth & `RRG`, & **R15–R16** `17`-entry `SIGNAL_CATALOG` + $O(N)$ `PrecomputedCatalogMatrix`)
  - `src/alerts/` (`PriceLevel`, `RSI`, `MACD`, `DrawingLine`, & **R16** `SignalCatalogTrigger` + `ConfluenceThreshold` + `cooldown_bars` + bounded VN/US/UTC HTF close guards + formula-safe CSV/JSON clipboard export)
  - `src/backtest/` (`7` presets incl. **R16** 4-leg DNF `CustomSignalRules`, `Open[T+1]` fills, `WFO`, `MultiAssetPortfolio`, & `SplitMix64` Monte Carlo fan)
  - `src/chart.rs`, `src/app.rs` & `src/main.rs` (`--r16` holding all UI workloads `< 4.0 ms`: **`0.322–1.332 ms`** mean / **`p99 <= 1.501 ms`**, `"overall_pass": true`)

---

## 3. Recommended R17 Plan (`Wave 16 — Practice Coaching & Debrief, 1-Click Scanner->Replay, & Multi-Strategy Analytics`, `A1..A5`)

### Workstream 1 — Research Gate (`DEC-005` Compliance)
1. Execute live REA MCP JS/TS analysis on `ARCHIVE/frontend/src/{components/replay/PracticeScoreboard.tsx,components/replay/SessionDebriefModal.tsx,components/replay/DecisionJournal.tsx,components/replay/SessionSetup.tsx,components/strategy/MultiStrategyEquityChart.tsx,pages/AnalyticsPage.tsx}` and `LWC v5.2.0` multi-series/line primitives; author the R17 specification in `docs/research/`.

### Workstream 2 — Wave 16 Domain & UI Enhancements (`A1..A5` in `FEATURE_BACKLOG_AND_PARITY_LEDGER.md`)
1. **`A1` + `A2` — Live Replay Practice Scoreboard, `SessionDebriefModal` & `SL-Adherence` Audit (`src/trading/`, `src/journal/`, `src/app.rs`):** Compact live KPI strip (`Closed Trades`, `Win Rate`, `Net R` / `Avg R`, `Reset Practice`) + end-of-session debrief modal (`% Initial Stop-Loss Adherence`, `MistakeTag` breakdown, `SetupTag` expectancy, and causal `Decision Note at Bar`).
2. **`A3` — 1-Click `Scanner -> Replay` (`BlindPractice` / `SignalReview`) & Saved Practice/Scan Sessions (`src/replay/`, `src/scanner/`, `src/app.rs`):** One-click buttons on Scanner signal rows jumping `120` bars prior to `bar_timestamp` in `BlindPractice` (masked until `reveal_at_timestamp`) or `SignalReview`, plus local RON/JSON saved practice sessions.
3. **`A4` + `A5` — UTC-Aligned `0%` Multi-Strategy Equity Comparison Overlay & Advanced Tearsheet Analytics (`src/backtest/`, `src/chart.rs`, `src/app.rs`):** Multi-preset / top-sweep + benchmark `%` return curves aligned strictly by UTC timestamp (rectifying legacy index-misalignment bug `MultiStrategyEquityChart.tsx:97`), plus `OutlierImpact`, discrete `DrawdownPeriod` table, Year/`MarketRegime` performance breakdown, and `RunManifest` reproducibility hash.

### Workstream 3 — `--r17` Release Benchmark, Offscreen Metal Captures, Critic, Audit & Clean Commit
1. Pass all 4 Cargo gates, run `--release` `--r17` (`< 4.0 ms` CPU frame budget) + `SUMI_CAPTURE_BATCH=r17`, update `FEATURE_BACKLOG_AND_PARITY_LEDGER.md` (`A1..A5 -> [x]`), pass `sumi-critic` (`CRITIC_PASS`) and `sumi-auditor` (`AUDIT_PASS`), and commit cleanly on `master` per `DEC-R04-001`.

---

## 4. Standing Constraints & Product Owner Gates

- **Product Owner Authorization Gate:** Wait for explicit Product Owner approval before starting Milestone R17 implementation.
- **Language:** Communicate 100% in **Vietnamese (`tiếng Việt`)** with the Product Owner.
- **Git & File Safety (`DEC-R04-001`):** Never touch `Cargo.toml`, `Cargo.lock`, `src/model.rs`, `ARCHIVE/`, or R01–R16 `docs/evidence/*`; never run `git reset`, `git clean`, `git push`, or `git tag`.
- **Authorized Targets Only (`DEC-006`):** Never inspect or interact with TradingView Desktop.
