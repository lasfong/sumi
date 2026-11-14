# VIETNAM MARKET MICROSTRUCTURE & RISK PLANNER, ON-CHART RISK-REWARD POSITION TOOL, STRUCTURED TRADE JOURNAL & ROLLING WALK-FORWARD PORTFOLIO ANALYTICS SPECIFICATION (`MILESTONE R10 / WAVE 9`)

- **Session Identity:** `SESSION-R10-20261114T122730+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Research Gate Status:** `VERIFIED` (`DEC-005` Cleared — Live REA MCP + LWC v5.2.0 Apache 2.0 + Legacy Sumi Archive)
- **Authorized Reference Targets:**
  1. **Legacy Sumi Archive** (`/Users/mizuhara/workspace/sumi/ARCHIVE/frontend/src/{pages/JournalPage.tsx,pages/DashboardPage.tsx,api/journalApi.ts,api/decisionApi.ts,types/journal.ts,types/decision.ts,components/replay/DecisionJournal.tsx,components/replay/PracticeJournal.tsx,components/replay/PracticeScoreboard.tsx,components/replay/SessionDebriefModal.tsx,components/strategy/MultiPhaseBatchPanel.tsx}` and `/Users/mizuhara/workspace/sumi/ARCHIVE/backend/app/domain/{market_rules.py,market/rules.py,trade_planning.py,accounting.py}`)
  2. **Lightweight Charts v5.2.0** (`/Users/mizuhara/workspace/sumi/research_repos/lightweight-charts-5.2.0/src/{api/iseries-primitive-api.ts,api/ipane-primitive-api.ts,views/pane/ipane-view.ts,views/price-axis/price-axis-view.ts,renderers/price-axis-view-renderer.ts,renderers/range-hit-test.ts,renderers/hit-test-common.ts,formatters/price-formatter.ts}`, commit `868cae27bd1acafa0128d8d868ea740a59ae42ce`, `Apache-2.0`)
  3. **Compliance Boundary (`DEC-006`):** Proprietary TradingView Desktop was **not** inspected, hooked, or queried.

---

## 1. Live REA MCP Execution Evidence (`Milestone R10`)

All REA operations below were executed live via the `rea-agents@6.8.0` MCP server during `SESSION-R10-20261114T122730+0700`:

| Op ID | REA Tool & Query | Target Scope | Actual Verified Output | Exact 64-Hex Evidence ID |
| :--- | :--- | :--- | :--- | :--- |
| `REA-R10-01` | `analyze_javascript_application` (`detail: "summary"`) | `ARCHIVE/frontend` (`11` TS/TSX files: `JournalPage.tsx`, `DashboardPage.tsx`, `journalApi.ts`, `decisionApi.ts`, `journal.ts`, `decision.ts`, `DecisionJournal.tsx`, `PracticeJournal.tsx`, `PracticeScoreboard.tsx`, `SessionDebriefModal.tsx`, `MultiPhaseBatchPanel.tsx`) | Root artifact SHA-256 `243b2d6eaac9d00e0cd2e2623da5ea5f84bb751ad89e2ba40a04be7f7b794b92`; `11` files parsed (`0` failures), `97,345` bytes, `13,287` AST nodes, `200` findings, `2,528` `data-flow`, `379` `object-flow`, `198` `closure`, `60` `call-flow`, `15` `promise-ownership`, `2` `boundary` relations (`10,351/10,351` view items). | Parent: `ev_d326070e2426cc6917fd59b105528087718d592698d0a75927abeded6aaedf3f`<br>Summary: `ev_338e20db6278637df6e9c104fc31b5ce629312b120e6eee8973733bc92a718e5` |
| `REA-R10-02` | `analyze_javascript_application` (`detail: "summary"`) | `research_repos/lightweight-charts-5.2.0` (`8` TS files: `iseries-primitive-api.ts`, `ipane-primitive-api.ts`, `ipane-view.ts`, `price-axis-view.ts`, `price-axis-view-renderer.ts`, `range-hit-test.ts`, `hit-test-common.ts`, `price-formatter.ts`) | Root artifact SHA-256 `6a26c4adafe9539017d004f611425bc9692a278cd45d4a41ffe381afd7b6c9d5`; `8` files parsed (`0` failures), `21,787` bytes, `2,945` AST nodes, `97` findings, `1,057` `data-flow`, `106` `object-flow`, `39` `closure`, `24` `call-flow` relations (`1,111/1,111` view items, `truncated: false`). | Parent: `ev_ec714173b1741adc3b5253f7d7af09162981dc5296ffcd421c750ba8f320536b`<br>Summary: `ev_9ccc2798ab739dac9aae644e48b20a7cfc7be540ff75e9faeae47f10b4e0b857` |
| `REA-R10-03` | `inspect_analysis_view` (`collection: "modules"`, `projection: "references"`, `limit: 16`) | Parent `ev_ec714173b1741adc3b5253f7d7af09162981dc5296ffcd421c750ba8f320536b` (`LWC v5.2.0`) | `16` module/asset reference rows inspected out of `1,111` (`view_digest = 856ad492790d73509a5cf8d067ec07e1ea921cbed709f388672f513e1b7b8b38`), including `src/model/series-primitive-wrapper.ts` (`sha256: 613e868c7a5b...`). | `ev_0bd6b7fd76e5bb18fc248f0479abce153f34fc52a35844f4d93c2f19f43a09b9` |
| `REA-R10-04` | `trace_javascript_semantics` (`property: "mistake_tag"`, `forward-influence`) | Parent `ev_d326070e2426cc6917fd59b105528087718d592698d0a75927abeded6aaedf3f` (`ARCHIVE/frontend`) | `4` seed matches across `4` functions in `3` modules (`JournalPage.tsx:242,293,298`, `SessionDebriefModal.tsx:62–63`, `DecisionJournal.tsx:65–67`), proving `mistake_tag` was rendered as passive free-form strings without monetary `Cost of Mistakes` or `Discipline Score %` aggregation. | `ev_0b2302d7ef9fbf0bc2d82aa60550aff4cd121bac72aa00a93a9ca40bb3ff356a` |
| `REA-R10-05` | `trace_javascript_semantics` (`property: "confidence_score"`, `forward-influence`) | Parent `ev_d326070e2426cc6917fd59b105528087718d592698d0a75927abeded6aaedf3f` (`ARCHIVE/frontend`) | `1` seed match in `DecisionJournal.tsx:57` (`(${j.confidence_score}/5)`), confirming `confidence_score` (`1..5` conviction) was isolated to replay decision cards and absent from trade-level expectancy analytics. | `ev_0ae4bec78b8c392306533a1017b663c8a6fc0e64cdebb8db060ef6cae74f5920` |
| `REA-R10-06` | `trace_javascript_semantics` (`property: "metric_matrix"`, `forward-influence`) | Parent `ev_d326070e2426cc6917fd59b105528087718d592698d0a75927abeded6aaedf3f` (`ARCHIVE/frontend`) | `1` seed match in `MultiPhaseBatchPanel.tsx:178`, proving legacy multi-phase batch evaluation displayed a static calendar matrix (`2020-2022` / `2023-2024`) with fixed strategy parameters and independent per-symbol cash rather than rolling `IS -> OOS` re-optimization or combined portfolio equity. | `ev_0daaf78dad998fc90634f00aa02420f627fc0405ad5f293022d67356fe0da22d` |
| `REA-R10-07` | `trace_javascript_semantics` (`property: "checklist_snapshot"`, `forward-influence`) | Parent `ev_d326070e2426cc6917fd59b105528087718d592698d0a75927abeded6aaedf3f` (`ARCHIVE/frontend`) | `0` runtime property-read seed matches (`checklist_snapshot` existed only in TypeScript interface declarations `journal.ts:14,32` and `decision.ts:76,103`), proving pre-trade checklist compliance was serialized as an opaque JSON string inside `PracticeJournal` `content` rather than structured typed fields. | `ev_7838e78d45af14f7e6de3b568197837e7bba2a5967c9e3f5e46cb9cec5345381` |
| `REA-R10-08` | `trace_javascript_semantics` (`property: "walk_forward_efficiency"`, `forward-influence`) | Parent `ev_d326070e2426cc6917fd59b105528087718d592698d0a75927abeded6aaedf3f` (`ARCHIVE/frontend`) | `0` seed matches (`0` traversed nodes/relations) — negative control confirming legacy Sumi lacked rolling `Walk-Forward Efficiency (WFE %)` and stitched Out-of-Sample validation. | `ev_590404ccb461c2e1abec0ea4918f5a9661a0a3e58a09d2836853d3438c0b1188` |
| `REA-R10-09` | `trace_javascript_semantics` (`property: "_minMove"`, `forward-influence`) | Parent `ev_ec714173b1741adc3b5253f7d7af09162981dc5296ffcd421c750ba8f320536b` (`LWC v5.2.0`) | `4` seed matches across `3` functions in `src/formatters/price-formatter.ts` (`L56`, `L74`, `L85`, `L98`), anchoring tick-size (`minMove`) decimal precision and step quantization (`base = priceScale / minMove`). | `ev_7eebd97bd3a7a9d66c86958c78a3973c663cc85ce854626db300857b197262f9` |
| `REA-R10-10` | `trace_javascript_semantics` (`property: "from"`, `forward-influence`) | Parent `ev_ec714173b1741adc3b5253f7d7af09162981dc5296ffcd421c750ba8f320536b` (`LWC v5.2.0`) | `1` seed match with `3` source-range reads in `src/renderers/range-hit-test.ts` (`L41`, `L46`, `L56`), grounding horizontal/vertical range box hit-testing (`actualTop = min(y0, y1)`, `actualBottom = max(y0, y1)`, `hitTestTolerance`). | `ev_bc6281a339f7bca56a1fbc4f10a2bfa4abcec0233dc378b1a2d908ebde6c55e1` |
| `REA-R10-11` | `inspect_analysis_view` (`collection: "modules"`, `projection: "references"`, `limit: 16`) | Parent `ev_d326070e2426cc6917fd59b105528087718d592698d0a75927abeded6aaedf3f` (`ARCHIVE/frontend`) | `16` module/asset reference rows inspected out of `10,351` (`view_digest = fc28a498e44ef722814731ed73012e042ca593dfb2b092c25e67421fccad630c`), confirming full artifact inventory integrity (`sha256: 243b2d6eaac9...`). | `ev_72fb8779f9cc33ca8d09f48d8ae2e3fb804335f222dd8e77d84b475b772c144d` |
| `REA-R10-12` | `inspect_rust_binary` (`detail: "summary"`) | `/Users/mizuhara/workspace/sumi/target/release/sumi-workstation` (`R10` Release Mach-O) | `19,302,688` bytes (`arm64` Mach-O, `64`-bit little-endian), SHA-256 `5dd7e72be2b0249a98547397cf5cdcee177af319cde0fdd0b760ea8f899dbfa8`, `1` inferred `rustc` commit hash, `0` new crate dependencies in `Cargo.toml`. | Parent: `ev_76e77e445dd22a5405155d83e1d63db0353469b5084d435ff632b44c2cfd1118`<br>Summary: `ev_cccbd439a778ba81e624ecc982675e2a6e111e2f38da21d1e8982c37933746d8` |
| `REA-R10-13` | `open_binary` → `inspect_macho` (`detail: "summary"`) → `close_binary` | `/Users/mizuhara/workspace/sumi/target/release/sumi-workstation` (`R10` Release Mach-O) | UUID `CA8A7F2F-8C54-3DBB-8C0E-F8A9AA1F0087`, `5,477/5,477` items examined (`30` load commands, `14` dylibs unchanged across R03–R10, `324` imports, `5,101` exports, `5` segments); binary session cleanly closed. | Parent: `ev_29e2668315ec6fbf47775ef281cf44af36f1ea3e23caaab58fdc70742b94338c`<br>Summary: `ev_fb53945936be4a04c25baed3c6cf6b4e0842c921a4b359a6de695d6c96e189ad` |

---

## 2. Six Legacy Risk Planning, Journal & Portfolio Defects Rectified in Rust

1. **Asymmetric Stop-Out Risk Outlay & Missing Breakeven / Slippage in `trade_planning.py:108–138`:**
   - Legacy `calculate_position_size` computed `raw_quantity = max_risk_amount / abs(entry_price - stop_loss)` without deducting entry commission, exit commission (`0.15%`), Vietnam securities transfer sell tax (`0.10%`), or adverse slippage (`bps`). Even its `total_risk_outlay = planned_risk_amount + estimated_buy_fee` omitted the sell fee and `0.10%` sell tax incurred on stop-out — causing a `1.0%` risk trade to lose `> 1.25%` of equity when stopped out.
   - **Rust Fix (`src/journal/planner.rs`):** Computes exact all-in per-unit stop loss `risk_per_unit_net` across entry/exit `commission_bps`, `slippage_bps`, and `tax_sell_bps` (for both `Long` and `Short`), sizes `planned_quantity <= max_risk_amount / risk_per_unit_net` so realized stop-out loss **never** exceeds `max_risk_amount`, and computes exact friction-adjusted `breakeven_price`, `gross_r_multiple`, `net_r_multiple`, and `required_win_rate_pct`.
2. **Vietnam Price-Unit Scale Blindness (`1,000 VND` vs Raw `VND`) & Tier-Crossing Band Rounding (`market/rules.py:67–98`):**
   - Legacy `MarketRules.get_tick_size(price)` compared `price < 10_000.0` / `< 50_000.0` assuming raw `VND` (`48,500`), returning `tick = 10.0` (`+4,169%` of stock price) on CafeF/VN30 quotes denominated in `1,000 VND` (`FPT = 239.86`, `HPG = 28.50`, `SHB = 9.85`). Furthermore, `market_rules.py` did not evaluate tier-appropriate ticks at raw ceiling/floor prices when `reference_price` crossed the `10,000 VND` (`10.00`) or `50,000 VND` (`50.00`) `HOSE` tier boundaries.
   - **Rust Fix (`src/journal/market_rules.rs`):** Supports `VnPriceUnit` (`Auto`, `ThousandVnd`, `RawVnd`), statutory `HOSE` (`< 10k -> 10 VND / 0.01`, `10k..50k -> 50 VND / 0.05`, `>= 50k -> 100 VND / 0.10`), `HNX`/`UPCOM` (`100 VND / 0.10`), `Vn30Futures` (`0.1` pt), `UsEquities`, and `CryptoSpotPerp`, with statutory daily bands (`HOSE ±7%`, `HNX ±10%`, `UPCOM ±15%`, `VN30F ±7%`) rounded **down to the tick at `raw_ceiling`** and **up to the tick at `raw_floor`** (`1e-9` IEEE-754 guard), locked ceiling/floor detection, board-lot rounding (`100` shares), and `T+2` business-day settlement timestamp calculation.
3. **Complete Absence of an Interactive On-Chart `Risk-Reward` Position Box Tool:**
   - Legacy Sumi forced traders to manually type `entry_price`, `stop_loss`, and `target_price` into detached forms with zero visual reward/risk box on the OHLCV chart.
   - **Rust Fix (`src/drawings/` & `src/chart.rs`):** Adds 6th `DrawingGeometry::RiskRewardBox { start, end, stop_price }` (`Main`-pane target) with 4 handles (`RrEntryLeft`, `RrEntryRight`, `RrTarget`, `RrStop`), `Log`/`Linear` invariant body drag, lossless `remap_timeframe`, `Undo/Redo`, `DEF-001` causal visibility, batched translucent emerald target + crimson risk fills + dashed entry/stop/target strokes + `R:R` badge, and 1-click sync to Order Ticket and Risk Planner (`extract_ticket_from_drawing`).
4. **Detached HTTP Session-Bound `/journal` Without Backtest Integration, Causal Replay Masking, or Behavioral Attribution (`JournalPage.tsx`, `REA-R10-04` / `REA-R10-05` / `REA-R10-07`):**
   - Legacy `/journal` required an active SQLite `sessionId`, could not import Strategy Backtest trades, leaked future session entries when rewinding Bar Replay (`DEF-001`), stored checklists as opaque text (`REA-R10-07`), and lacked quantitative behavioral metrics.
   - **Rust Fix (`src/journal/ledger.rs`):** Deterministic `TradeJournalStore` ingesting Paper Trading `ClosedTrade` records, Strategy Backtest `BacktestTrade` records, and pre-trade `TradePlanComputed` setups with idempotent deduplication, structured `SetupTag`, `MistakeTag`, `ConvictionRating` (`1..=5`), typed 6-item pre-trade `ChecklistState`, planned-vs-executed entry/R variance, strict `DEF-001` causal filtering (`effective_timestamp <= replay_close_cutoff`), and `JournalAnalytics` (`Discipline Score %`, `Cost of Mistakes`, Van Tharp `SQN`, `Expectancy R`, setup/mistake attribution).
5. **CSV Formula-Injection & Wall-Clock Export Non-Determinism (`MultiPhaseBatchPanel.tsx:157–163`):**
   - Legacy CSV exports concatenated unescaped strings and `Date.now()` filenames, risking spreadsheet formula execution (`=`, `+`, `-`, `@`, `\t`, `\r`) when exporting user journal notes.
   - **Rust Fix (`src/journal/ledger.rs`):** Deterministic RFC-4180 CSV serializer (`export_journal_csv`) that prefixes formula-triggering leading characters (`=`, `+`, `-`, `@`, `\t`, `\r`) — including after leading ASCII spaces — with `'`, quotes fields containing `,`, `"`, `\n`, or `\r`, and doubles embedded `""`, paired with deterministic Markdown report export (`export_journal_markdown`).
6. **Static Calendar Batch Matrix Without Rolling Walk-Forward Optimization or Portfolio Weighting (`MultiPhaseBatchPanel.tsx`, `REA-R10-06` / `REA-R10-08`):**
   - Legacy `MultiPhaseBatchPanel` evaluated fixed parameters on hardcoded `2020-2022` / `2023-2024` slices with independent capital per symbol — zero `IS -> OOS` rolling parameter optimization and zero combined multi-asset portfolio equity/drawdown or volatility-scaled allocation.
   - **Rust Fix (`src/backtest/walk_forward.rs`):** Implements `run_walk_forward_validation` (`K`-fold Rolling or Anchored `IS` grid optimization -> disjoint causal `OOS` test, stitched compounding `OOS` equity curve, `Walk-Forward Efficiency %`, parameter stability score, `WalkForwardVerdict`) and `run_portfolio_backtest` (`EqualWeight` & strictly past-only `CausalInverseVolatility` weighting across `VN30` or full universe feeds gated by `replay_causal_close_timestamp`, `Portfolio Tearsheet`, `Diversification DD Reduction %`, per-symbol attribution, and 1-click synchronized `OscillatorPaneId::Equity` plotting).

---

## 3. Subsystem Architecture & Mathematical Contracts

All domain modules (`src/journal/`, `src/drawings/`, `src/backtest/walk_forward.rs`) are **100% pure `f64`/`i64` Rust** with **zero new dependencies** in `Cargo.toml` and **zero OS, wall-clock, or UI imports** (`wasm32-unknown-unknown` verified).

### 3.1 Vietnam & Multi-Asset Exchange Microstructure (`src/journal/market_rules.rs`)

- **`ExchangeVenue`:**
  1. `Hose`: `±7.0%` daily price band, board lot `100`, `T+2` settlement, `tax_sell_bps = 10.0` (`0.10%`), `allow_short = false`, 3-tier tick ladder:
     - `ThousandVnd`: `< 10.0` -> `0.01`, `10.0 <= p < 50.0` -> `0.05`, `p >= 50.0` -> `0.10`.
     - `RawVnd`: `< 10_000.0` -> `10.0`, `10_000.0 <= p < 50_000.0` -> `50.0`, `p >= 50_000.0` -> `100.0`.
  2. `Hnx`: `±10.0%` daily band, board lot `100`, `T+2` settlement, `tax_sell_bps = 10.0`, `allow_short = false`, tick `0.10` (`ThousandVnd`) / `100.0` (`RawVnd`).
  3. `Upcom`: `±15.0%` daily band, board lot `100`, `T+2` settlement, `tax_sell_bps = 10.0`, `allow_short = false`, tick `0.10` (`ThousandVnd`) / `100.0` (`RawVnd`).
  4. `Vn30Futures`: `±7.0%` daily band, contract lot `1.0`, `T+0` settlement, `tax_sell_bps = 0.0`, `allow_short = true`, tick `0.10`.
  5. `UsEquities`: no statutory daily band (`None`), lot `1.0`, `T+1` settlement, `tax_sell_bps = 0.0`, `allow_short = true`, tick `0.01` (`>= 1.0`) / `0.0001` (`< 1.0`).
  6. `CryptoSpotPerp`: no daily band (`None`), fractional lot `0.001`, `T+0` settlement, `tax_sell_bps = 0.0`, `allow_short = true`, tick `0.01` (`>= 100.0`), `0.0001` (`>= 1.0`), `1e-6` (`< 1.0`).
- **Statutory Daily Band Rounding (`compute_daily_price_band`):**
  - `raw_ceiling = reference_price * (1.0 + band_pct)` -> `ceiling_price = round_down_to_tick(raw_ceiling, venue, unit)` using the tick tier at `raw_ceiling` (`((raw_ceiling / tick) + 1e-9).floor() * tick`).
  - `raw_floor = reference_price * (1.0 - band_pct)` -> `floor_price = round_up_to_tick(raw_floor, venue, unit)` using the tick tier at `raw_floor` (`((raw_floor / tick) - 1e-9).ceil() * tick`).
  - Guaranteed to remain strictly inside `[raw_floor, raw_ceiling]` while landing on valid exchange order-book ticks across `HOSE` `10k`/`50k` tier transitions.
- **Business-Day Settlement (`compute_settlement_timestamp`):**
  - Advances `entry_timestamp` by `settlement_days` (`2` for VN equities, `1` for US equities, `0` for futures/crypto) skipping Saturday (`weekday == 5`) and Sunday (`weekday == 6`) via `crate::data::weekday_from_days`.

### 3.2 Friction-Aware Risk-Reward Position Planner (`src/journal/planner.rs`)

- **Tick Snapping:** Snaps `entry_price` to nearest valid exchange tick; snaps `stop_loss` conservatively to valid tick (rounded down for `Long`, up for `Short` or nearest valid tick preserving `stop != entry`); snaps `target_price` conservatively inside daily/tick grid (rounded down for `Long`, up for `Short`).
- **All-In Friction & Exact Breakeven:**
  - Let $c = \text{commission\_bps} \cdot 10^{-4}$, $s = \text{slippage\_bps} \cdot 10^{-4}$, $t_{\text{sell}} = \text{tax\_sell\_bps} \cdot 10^{-4}$.
  - For `Long`:
    $$\text{risk\_per\_unit\_net} = E(1+s)(1+c) - S(1-s)(1 - c - t_{\text{sell}})$$
    $$\text{reward\_per\_unit\_net} = T(1-s)(1 - c - t_{\text{sell}}) - E(1+s)(1+c)$$
    $$B_{\text{long}} = E \cdot \frac{(1+s)(1+c)}{(1-s)(1 - c - t_{\text{sell}})}$$
  - For `Short`:
    $$\text{risk\_per\_unit\_net} = S(1+s)(1+c) - E(1-s)(1 - c - t_{\text{sell}})$$
    $$\text{reward\_per\_unit\_net} = E(1-s)(1 - c - t_{\text{sell}}) - T(1+s)(1+c)$$
    $$B_{\text{short}} = E \cdot \frac{(1-s)(1 - c - t_{\text{sell}})}{(1+s)(1+c)}$$
- **Sizing Invariant:** `planned_quantity = venue.floor_to_lot(min(max_risk_amount / risk_per_unit_net, max_cash_quantity))`, guaranteeing `planned_risk_amount = planned_quantity * risk_per_unit_net <= max_risk_amount + 1e-9`.

### 3.3 On-Chart `RiskRewardBox` Drawing Tool (`src/drawings/`)

- **`DrawingGeometry::RiskRewardBox { start, end, stop_price }`:**
  - `Main`-pane exclusive (`DrawingPaneTarget::Main`), `entry_price = start.value`, `target_price = end.value`, `side = Long` when `end.value > start.value && stop_price < start.value` else `Short` when `end.value < start.value && stop_price > start.value`.
  - Four handles: `HandlePart::RrEntryLeft`, `HandlePart::RrEntryRight`, `HandlePart::RrTarget`, `HandlePart::RrStop`.
  - Body translation projects `[p_entry_left, p_target_right, p_stop_right]` through `(dx, dy)` in pane-local space so `gross_r_multiple` stays invariant under both `Linear` and `Logarithmic` price scales.

### 3.4 Structured Trade Journal & Behavioral Analytics (`src/journal/ledger.rs`)

- **Causal Visibility (`DEF-001`):** `visible_entries(replay_close_cutoff)` withholds any entry whose causal creation/completion timestamp (`exit_timestamp` for closed trades, `created_at_timestamp` for planned setups) exceeds `replay_close_cutoff`.
- **Behavioral Metrics (`compute_journal_analytics`):**
  - `discipline_score_pct`: mean compliance across closed/planned entries (`100%` when `mistake == MistakeTag::None` and checklist pass ratio, penalized when a `MistakeTag` is recorded).
  - `cost_of_mistakes`: sum of negative `net_pnl` (`|net_pnl|`) across closed trades tagged with `mistake != MistakeTag::None`.
  - `sqn` (Van Tharp System Quality Number): when `r_count >= 5` and sample std dev $\sigma_R > 10^{-12}$, $\text{SQN} = \sqrt{\min(N, 100)} \cdot \frac{\bar{R}}{\sigma_R}$ (capped at $\sqrt{100}$ per Van Tharp's large-$N$ normalization so high trade counts do not artificially inflate `SQN`).

### 3.5 Rolling Walk-Forward & Multi-Asset Portfolio Engine (`src/backtest/walk_forward.rs`)

- **`run_walk_forward_validation`:** Partitions `causal_candles` into `K` (`2..=8`) contiguous, non-overlapping `OOS` folds `[oos_start_k .. oos_end_k)` preceded by Rolling (`[oos_start_k - L_is .. oos_start_k)`) or Anchored (`[0 .. oos_start_k)`) `IS` folds; optimizes `build_preset_sweep_configs` strictly on `IS[k]`, tests winning parameters on `OOS[k]` with continuous compounding capital across stitched `OOS` folds, and reports `wfe_pct`, `param_stability_pct`, and `WalkForwardVerdict`.
- **`run_portfolio_backtest`:** Slices every universe feed by `replay_causal_close_timestamp` and `is_bucket_incomplete_at_close`, aligns constituent wealth indices on a sorted causal UTC timestamp grid via past-only forward fill, applies strictly past-window weights (`EqualWeight` or `CausalInverseVolatility { lookback_bars }` over `..m`, excluding step `m`), and outputs `PortfolioBacktestReport` (`portfolio_equity_curve`, `portfolio_tearsheet`, `diversification_dd_reduction_pct`, `constituents`, and `map_equity_curve_to_candles`).

---

## 4. Adversarial `sumi-critic` Hardening (`19/19` Findings Remediated)

Following initial implementation, `sumi-critic` executed an independent adversarial audit and reported `19` findings (`C-01..C-05`, `M-01..M-09`, `N-01..N-05`). All `19/19` were remediated with `4` dedicated `critic_r10_regression_*` suites (`136/136` automated tests passing):

- **`C-01` (Mixed-Frequency Portfolio Bar-Close Causality & Zero-Dilution-Free Inverse Volatility):** Stamped constituent wealth points at causal bar-close instants (`bar_open_ts + tf.seconds() - 1`, clamped to `replay_causal_close_timestamp`) and computed `CausalInverseVolatility` over genuine per-constituent observation returns scaled by $\sqrt{B_{\text{year}}(s)}$ (`constituent_annualization_root`) rather than zero-padded forward-filled timestamps.
- **`C-02` (Continuous `SimulationCarryState` Across Walk-Forward `OOS` Folds):** Replaced isolated per-fold `OOS` runs (`EndOfSeriesCutoff` forced exits) with `simulate_window_with_carry`, carrying open positions (including Vietnam `T+2` `min_holding_bars` lockup), pending `Open[T+1]` orders, cash, peak equity, and monotonic `trade_id` across `OOS[k] -> OOS[k+1]`.
- **`C-03` (`VnIndexBenchmark` Separation, `VN30F` `x100,000` / `x100` Multiplier, `17%` Initial Margin & `max_position_pct` Concentration Cap):** Added `ExchangeVenue::VnIndexBenchmark` (`no ±7% band`, `lot 1.0`, `T+0`) for `VNINDEX`/`HNX-INDEX`, automatic `VN30F` point multiplier (`100,000 VND/pt` in `RawVnd`, `100 kVND/pt` in `ThousandVnd`) + `17.0%` statutory VSD/HNX initial margin, and `max_position_pct` concentration cap.
- **`C-04` (`0`-Lot Order Ticket Sync Guard):** Blocked `sync_ticket_from_risk_planner` when `!plan.affordable || plan.planned_quantity <= 0.0` so unaffordable or venue-prohibited plans never silently inject a `100`-share `board_lot()` order into the Order Ticket.
- **`C-05` & `M-07` (`RiskRewardBox` Non-Drifting Handle Drag & Smooth `Long <-> Short` Crossover):** Evaluated `apply_handle_drag` strictly from immutable `original.geometry` snapshots on every pointer move (eliminating `RrEntryLeft`/`RrEntryRight` crossover drift) and reflected `stop_price` across `entry` when `RrTarget` crosses `entry`.
- **`M-01..M-09` & `N-01..N-05`:** Fixed `HOSE` `10k`/`50k` tier-crossing `step_one_tick_below`/`above` + sub-tick `0.0` floor (`M-01`), `Short` entry `tax_sell_bps` parity + `ShortDisallowedOnVenue` `0`-qty clamp (`M-02`), bar-close `map_equity_curve_to_candles` + `grid_tf` Sharpe/Sortino annualization + compounded dollar attribution (`M-03`, `N-04`), hardened `WalkForwardVerdict` sample-size/WFE gates (`M-04`), Bar Replay rewind pruning on `sync_paper_closed_trades` (`M-05`), `TradeJournalStore::sanitize_deserialized` + single-line RFC-4180 `sanitize_csv_cell` (`M-06`), `risk_venue_override` reset on ticker switch + `Reseed` `RiskRewardBox` sync (`M-08`), exchange-local civil timezone (`UTC+7` ICT / `UTC-5` EST) `causal_reference_price` (`base_candles` on `1W`) and `compute_settlement_timestamp_for_venue` (`M-09`), breakeven `current_streak = 0` (`N-01`), `WalkForwardOos` trade links on chart (`N-02`), and `1e-8` micro-priced crypto tick steps (`N-03`).
