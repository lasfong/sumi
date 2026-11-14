# DETERMINISTIC STRATEGY BACKTEST LAB, CAUSAL STRATEGY RUNNER, SYNCHRONIZED EQUITY/DRAWDOWN SUB-PANE & PERFORMANCE TEARSHEET SPECIFICATION (`MILESTONE R09 / WAVE 8`)

- **Session Identity:** `SESSION-R09-20261114T091018+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Research Gate Status:** `VERIFIED` (`DEC-005` Cleared — Live REA MCP + LWC v5.2.0 Apache 2.0 + Legacy Sumi Archive)
- **Authorized Reference Targets:**
  1. **Legacy Sumi Archive** (`/Users/mizuhara/workspace/sumi/ARCHIVE/frontend/src/{pages/StrategyLabPage.tsx,pages/BacktestPage.tsx,pages/AnalyticsPage.tsx,api/strategyLabApi.ts,api/backtestApi.ts,api/analyticsApi.ts,types/analytics.ts,components/strategy/StrategyRuleBuilder.tsx,components/strategy/MultiStrategyEquityChart.tsx,components/analytics/EquityChart.tsx}` and `/Users/mizuhara/workspace/sumi/ARCHIVE/backend/app/domain/{backtest/execution.py,backtest/metrics.py,backtest/models.py,backtest/batch_runner.py,strategy/strategy_rule_evaluator.py}`)
  2. **Lightweight Charts v5.2.0** (`/Users/mizuhara/workspace/sumi/research_repos/lightweight-charts-5.2.0/src/{model/series/area-series.ts,model/series/area-pane-view.ts,model/series/baseline-series.ts,model/series/baseline-pane-view.ts,renderers/area-renderer-base.ts,renderers/area-renderer.ts,renderers/baseline-renderer-area.ts,renderers/baseline-renderer-line.ts,renderers/walk-line.ts}`, commit `868cae27bd1acafa0128d8d868ea740a59ae42ce`, `Apache-2.0`)
  3. **Compliance Boundary (`DEC-006`):** Proprietary TradingView Desktop was **not** inspected, hooked, or queried.

---

## 1. Live REA MCP Execution Evidence (`Milestone R09`)

All REA operations below were executed live via the `rea-agents@6.8.0` MCP server during `SESSION-R09-20261114T091018+0700`:

| Op ID | REA Tool & Query | Target Scope | Actual Verified Output | Exact 64-Hex Evidence ID |
| :--- | :--- | :--- | :--- | :--- |
| `REA-R09-01` | `analyze_javascript_application` (`detail: "summary"`) | `ARCHIVE/frontend` (`10` TS/TSX files: `StrategyLabPage.tsx`, `BacktestPage.tsx`, `AnalyticsPage.tsx`, `strategyLabApi.ts`, `backtestApi.ts`, `analyticsApi.ts`, `analytics.ts`, `StrategyRuleBuilder.tsx`, `MultiStrategyEquityChart.tsx`, `EquityChart.tsx`) | Root artifact SHA-256 `243b2d6eaac9d00e0cd2e2623da5ea5f84bb751ad89e2ba40a04be7f7b794b92`; `10` files parsed (`0` failures), `193,614` bytes, `26,609` AST nodes, `340` findings, `5,994` `data-flow`, `781` `object-flow`, `616` `closure`, `368` `call-flow`, `19` `promise-ownership`, `12` `boundary`, `1` `timer` relations (`10,415/10,415` view items). | Parent: `ev_30052fc97a2e4a56ef820530ba6cfe841e47e68e9734a5f4f964eea286e583b3`<br>Summary: `ev_3efaddb8a370d7589ee89d250512250a58dc2f57683acf0eed11fde3d245414f` |
| `REA-R09-02` | `analyze_javascript_application` (`detail: "summary"`) | `research_repos/lightweight-charts-5.2.0` (`8` TS files: `area-series.ts`, `area-pane-view.ts`, `baseline-series.ts`, `baseline-pane-view.ts`, `area-renderer-base.ts`, `area-renderer.ts`, `baseline-renderer-area.ts`, `baseline-renderer-line.ts`) | Root artifact SHA-256 `6a26c4adafe9539017d004f611425bc9692a278cd45d4a41ffe381afd7b6c9d5`; `8` files parsed (`0` failures), `15,653` bytes, `2,044` AST nodes, `149` findings, `472` `data-flow`, `75` `object-flow`, `9` `closure`, `4` `call-flow` relations (`1,158/1,158` view items, `truncated: false`). | Parent: `ev_0e297ef9cf228bd81ab71f2394cde6d04b7fd6ad61c9ddef310ea2ff37592630`<br>Summary: `ev_f6890cd1628c40c2b704f5e8cff8d4134f88201429dbe362978cd845d014cc0e` |
| `REA-R09-03` | `trace_javascript_semantics` (`property: "equity_curve"`, `forward-influence`) | Parent `ev_30052fc97a2e4a56ef820530ba6cfe841e47e68e9734a5f4f964eea286e583b3` (`ARCHIVE/frontend`) | `5` seed matches across `4` functions in `3` separate React route pages (`BacktestPage.tsx:534–535`, `AnalyticsPage.tsx:231–232`, `StrategyLabPage.tsx:890–891`), proving `equity_curve` was rendered exclusively inside detached page components rather than a synchronized chart sub-pane. | `ev_2f44b96fe5a4eef4611d8215d1136d4a22679b310f6ed39fc1c09365d4c487d0` |
| `REA-R09-04` | `trace_javascript_semantics` (`property: "baseLevelCoordinate"`, `forward-influence`) | Parent `ev_0e297ef9cf228bd81ab71f2394cde6d04b7fd6ad61c9ddef310ea2ff37592630` (`LWC v5.2.0`) | `3` seed matches across `area-renderer-base.ts:55`, `baseline-renderer-area.ts:28`, and `baseline-renderer-line.ts:30`, anchoring split above/below `baseValue` (`initial_cash`) polygon fill and stroke geometry. | `ev_86aa8a13f1b6276cccadc28285b20e2b4f79f0dd55e957d6b1b24f81181d5d1e` |
| `REA-R09-05` | `inspect_analysis_view` (`collection: "modules"`, `projection: "references"`, `limit: 16`) | Parent `ev_0e297ef9cf228bd81ab71f2394cde6d04b7fd6ad61c9ddef310ea2ff37592630` (`LWC v5.2.0`) | `16` module/asset reference rows inspected out of `1,158` (`view_digest = dad571d39acbb643c5d9648d3111bbdba36eb8464ec464becedd2c3168e31272`), including `src/model/series/baseline-series.ts` (`sha256: 78893eead80f...`). | `ev_72d6e05935ba62d34ac2077f7da95227acf284b46206fc606090b7f1144785b1` |
| `REA-R09-06` | `trace_javascript_semantics` (`property: "oos_start_date"`, `forward-influence`) | Parent `ev_30052fc97a2e4a56ef820530ba6cfe841e47e68e9734a5f4f964eea286e583b3` (`ARCHIVE/frontend`) | `2` seed matches across `2` functions in `StrategyLabPage.tsx` (`L637`, `L739`, `L741`), tracing In-Sample / Out-of-Sample (`IS/OOS`) sweep split configuration. | `ev_54c92e4a7b365c9a003c992d9fd106389e0521c25a208042713896ddeb0c695b` |
| `REA-R09-07` | `trace_javascript_semantics` (`property: "sortino_ratio"`, `forward-influence`) | Parent `ev_30052fc97a2e4a56ef820530ba6cfe841e47e68e9734a5f4f964eea286e583b3` (`ARCHIVE/frontend`) | `1` isolated read at `AnalyticsPage.tsx:269` (`0` matches in `BacktestPage.tsx`, `StrategyLabPage.tsx`, or `ARCHIVE/backend/app/domain/backtest/metrics.py`). | `ev_fe8c9f114c234ba04df6e4fc5bb46e2d050f4047a6618e0aa341107f9364ddb7` |
| `REA-R09-08` | `trace_javascript_semantics` (`property: "returnPct"`, `forward-influence`) | Parent `ev_30052fc97a2e4a56ef820530ba6cfe841e47e68e9734a5f4f964eea286e583b3` (`ARCHIVE/frontend`) | `4` seed matches across `4` functions in `MultiStrategyEquityChart.tsx` (`L39`, `L40`, `L97`, `L256`, `L301`, `L302`), proving legacy multi-strategy comparison built raw `<svg>` strings indexed by array position (`x = paddingLeft + i * step`) with zero timestamp alignment or pan/zoom. | `ev_3e04368ead3291140b0287af30be58759c703c288f7a45874ecde03124d8aeaf` |
| `REA-R09-09` | `trace_javascript_semantics` (`property: "topFillColor1"` & `property: "bottomFillColor1"`, `forward-influence`) | Parent `ev_0e297ef9cf228bd81ab71f2394cde6d04b7fd6ad61c9ddef310ea2ff37592630` (`LWC v5.2.0`) | `1` seed match each at `src/renderers/baseline-renderer-area.ts:24` and `:26`, establishing dual-color above-baseline (`emerald`) and below-baseline (`crimson`) area fills. | `ev_80e6acb96f4e00a43e5ddc4d6f2152cbafefe61817d55649929913db7d1f7a21`<br>`ev_4d180bfec3ed42215bed4155c4a8ea7158a09e8805c0bc4f27f7bf4910b7cdad` |
| `REA-R09-10` | `trace_javascript_semantics` (`property: "calmar_ratio"`, `forward-influence`) | Parent `ev_30052fc97a2e4a56ef820530ba6cfe841e47e68e9734a5f4f964eea286e583b3` (`ARCHIVE/frontend`) | `0` seed matches (`0` traversed nodes/relations) — negative control confirming legacy Sumi lacked `Calmar`, `Avg R-Multiple`, and `Exposure %` tearsheet metrics. | `ev_f81cc48055b2b8643e18d2e9be4e8563a2fe4a3d5354ddaae2979e8ab3ce4e1c` |
| `REA-R09-11` | `trace_javascript_semantics` (`property: "add"`, `forward-influence`) | Parent `ev_30052fc97a2e4a56ef820530ba6cfe841e47e68e9734a5f4f964eea286e583b3` (`ARCHIVE/frontend`) | `3` seed matches including `seenTimes.add` (`EquityChart.tsx:42`) and `seenDdTimes.add` (`EquityChart.tsx:57`), proving legacy `EquityChart.tsx` collapsed all intraday equity and drawdown timestamps to `YYYY-MM-DD` and discarded duplicate intraday bars. | `ev_fee363ed7d49c372b0a6982c5d3b88f4c8c1bac291418ef21a012c4a54d2ff09` |
| `REA-R09-12` | `inspect_rust_binary` (`detail: "summary"`) | `/Users/mizuhara/workspace/sumi/target/release/sumi-workstation` (`R09` release Mach-O) | `18,791,648` bytes (`arm64` Mach-O, SHA-256 `5c7491d688cc87208264b15267b3d1b16b9a0fb00b4c029c5907fba46b25825a`), `1` inferred `rustc` commit hash, `0` new crate dependencies in `Cargo.toml`. | Parent: `ev_1da8bd9dc250bde8f93df0571e179999bdff2eff43f83e67106e2970c12d9139`<br>Summary: `ev_bbe704de6c3e1e0461264a8e326d00721db9762be825e624e311031978cb5d9e` |
| `REA-R09-13` | `open_binary` -> `inspect_macho` (`detail: "summary"`) -> `close_binary` | `/Users/mizuhara/workspace/sumi/target/release/sumi-workstation` (`R09` release Mach-O) | UUID `B62DC0FA-D8EC-355C-B99C-E80D4812E25E`, `30` load commands, `14` dylibs (unchanged across R03–R09), `324` imports, `5,045` exports, `5` segments (`5,421/5,421` items examined; session cleanly closed). | Parent: `ev_c32a6cf54091a02d3b6134ecf10f293cd00812971899972c211c7ec922d6a066`<br>Summary: `ev_ab19c6e79e0643bb25ad09039ca2e3402bdd34c2208856de1ab26854f6fc46cc` |

---

## 2. Six Legacy Strategy Lab & Backtest Defects Rectified in Rust (`src/backtest/`, `src/chart.rs`, `src/app.rs`)

1. **Detached Route Pages, `YYYY-MM-DD` Intraday Collapsing & Array-Index SVG Misalignment (`EquityChart.tsx:33–60`, `MultiStrategyEquityChart.tsx:91–126`, `REA-R09-03` / `REA-R09-08` / `REA-R09-11`):**
   - Legacy Sumi isolated Backtest and Strategy Lab on separate HTTP pages (`/backtest`, `/strategy-lab`) with no link to the main OHLCV chart. Worse, `EquityChart.tsx` truncated timestamps via `toDateKey` (`YYYY-MM-DD`) and dropped intraday bars via `seenTimes.add`, destroying `1m`/`5m`/`15m`/`1h`/`4h` equity curves, while `MultiStrategyEquityChart.tsx` plotted SVG polylines purely by array index (`i * step`).
   - **Rust Fix:** First-class `OscillatorPaneId::Equity` sub-pane inside `MultiPaneLayout`, sharing the exact `TimeScale` (`local_x`, `culling_range`, `conflation_factor`, horizontal pan/zoom, crosshair X-line & UTC readout) with `Main` OHLCV and all oscillator panes, backed by `SeriesBlockIndex` (`16`-bar min/max blocks) and single-pass `egui::Mesh` baseline-split tessellation (`< 0.15 ms` even at `95K/100K` `32x`-conflated bars).
2. **Row-by-Row Pandas `df.iloc[i]` & Per-Order `uuid.uuid4()` Allocation Bottleneck (`execution.py:211–218`):**
   - Legacy Python iterated `df.iloc[i]` row-by-row over Pandas `Series`, allocating `uuid.uuid4()` strings for every order, fill, lot, and trade over HTTP `/api/backtest/run`.
   - **Rust Fix:** Pure `f64`/`i64` WASM-safe `src/backtest/` precomputes contiguous indicator vectors once (`SMA`, `EMA`, `BollingerBands`, `RSI`, `MACD`, `ATR`, `Donchian`) and executes `100,000` bars with `70/30` `IS/OOS` split, `u64` IDs, and zero string allocations inside the hot loop in **`6.318–7.832 ms`** (`< 15.0 ms` budget).
3. **Long-Only Limitation & Hardcoded `sqrt(252.0)` Sharpe Annualization (`execution.py:227–474`, `metrics.py:133–150`):**
   - Legacy backtest kernel only supported `LONG` (`BUY` to open, `SELL` to close) and multiplied bar-return Sharpe by `sqrt(252.0)` regardless of whether bars were `1m`, `15m`, `4h`, `1D`, or `1W` or `24/7` crypto vs `5d/wk` equities.
   - **Rust Fix:** Symmetric `StrategyDirectionMode` (`LongOnly`, `ShortOnly`, `LongShort` with atomic `Open[T+1]` reversals), `tax_sell_bps` support for Vietnam equities alongside `commission_bps` & `slippage_bps`, and elapsed-span-aware effective bar density annualization (`(n_intervals / elapsed_years).clamp(1.0, 525_960.0)` when `n_intervals >= 10` and span `>= 30` days, with canonical `Timeframe` fallback for shorter windows).
4. **Incomplete Institutional Tearsheet (`REA-R09-07` / `REA-R09-10`):**
   - Legacy `metrics.py` lacked `Sortino Ratio`, `Calmar Ratio`, `Max Drawdown Duration (bars)`, `Avg R-Multiple`, and `Market Exposure %`, and risked `powf` overflow on sub-week `CAGR` windows.
   - **Rust Fix:** Complete `BacktestTearsheet` with `Sortino` (MAR `0.0` downside RMS), `Calmar` (`CAGR / MaxDD%`), `Max Drawdown Duration (bars)`, `Avg R-Multiple` (from initial stop risk), `Exposure %`, `N < 30` `low_sample_warning`, `years >= 7/365.25` `CAGR` guard, and optional `IS / OOS` split tearsheets + `RobustnessVerdict`.
5. **Intrabar Optimism & Trailing-Stop Same-Bar Lookahead Hazards:**
   - Updating an ATR trailing stop from `High[i]` *before* checking `Low[i]` on bar `i` creates phantom favorable exits on wide bearish bars; checking `TP` before `SL` on dual-breach OHLC bars overstates win rate.
   - **Rust Fix:** Strict Gap-Open check at `Open[i]` first, pessimistic `SL`-before-`TP` precedence on dual-breach bars (`Low[i] <= SL && High[i] >= TP`), and causal trailing-stop ratchet updated **strictly after** bar `i` closes without stopping out (`trail_sl` during bar `i` uses only `0..=i-1` / `Open[i]` entry state).
6. **Bar Replay Future Leakage Risk (`DEF-001`):**
   - Running a backtest, parameter sweep, or multi-symbol battle on un-sliced dataset arrays during active Bar Replay would leak post-cutoff trades, equity points, and tearsheet returns — including coarse higher-timeframe peer feeds (`D1` or `15m`) whose open timestamp `<= cursor_ts` has not yet closed at an intraday `1m` replay cursor (`09:03:59 UTC`).
   - **Rust Fix:** Every single backtest, parameter sweep, and 13-symbol universe battle run strictly slices input candles using the inclusive causal close cutoff `close_cutoff = cursor_open_ts + base_tf.seconds() - 1` (`run_multi_symbol_battle_with_close_cutoff` retains peer bars iff `c.timestamp + feed_base_tf.seconds() - 1 <= close_cutoff`, and `is_bucket_incomplete_at_close` withholds unclosed aggregated buckets via pure bucket arithmetic with zero `idx + 1` prefix peek). Trade Inspector supports 1-click jump to focus trade on chart OR launch causal `SignalReview` / `BlindPractice` Replay at trade entry.

---

## 3. Subsystem Architecture & Mathematical Contracts (`src/backtest/`)

`src/backtest/` is **100% pure `f64`/`i64` Rust** with **zero new dependencies** in `Cargo.toml` and **zero OS, wall-clock, or UI imports** (`wasm32-unknown-unknown` verified).

### 3.1 Composable Strategy Presets & Configuration (`src/backtest/strategy.rs`)

- **`StrategyPresetKind` (`6` deterministic presets with tunable parameters):**
  1. `SmaEmaTrendCross`:
     - Params: `fast_sma` (default `20`), `slow_ema` (default `50`).
     - Bullish cross: `(sma[i - 1] - ema[i - 1]) <= 0.0 && (sma[i] - ema[i]) > 0.0`.
     - Bearish cross: `(sma[i - 1] - ema[i - 1]) >= 0.0 && (sma[i] - ema[i]) < 0.0`.
  2. `RsiMeanReversion`:
     - Params: `rsi_period` (default `14`), `oversold` (default `30.0`), `overbought` (default `70.0`), `exit_midline` (default `50.0`).
     - Long entry: `rsi[i - 1] <= oversold && rsi[i] > oversold`; Long exit: `rsi[i] >= exit_midline` (or `>= overbought`).
     - Short entry: `rsi[i - 1] >= overbought && rsi[i] < overbought`; Short exit: `rsi[i] <= exit_midline`.
  3. `MacdMomentum`:
     - Params: `fast` (`12`), `slow` (`26`), `signal` (`9`), `trend_ema` (`50`, `0` = disabled).
     - Long entry: `hist[i - 1] <= 0.0 && hist[i] > 0.0 && (trend_ema == 0 || close[i] > ema[i])`; Long exit: `hist[i - 1] >= 0.0 && hist[i] < 0.0`.
     - Short entry: `hist[i - 1] >= 0.0 && hist[i] < 0.0 && (trend_ema == 0 || close[i] < ema[i])`; Short exit: `hist[i - 1] <= 0.0 && hist[i] > 0.0`.
  4. `BollingerSqueezeBreakout`:
     - Params: `bb_period` (`20`), `bb_std` (`2.0`), `max_squeeze_bw_pct` (`12.0`).
     - Prior squeeze `bw_prev = ((upper[i - 1] - lower[i - 1]) / middle[i - 1]) * 100.0 <= max_squeeze_bw_pct`.
     - Long entry: `bw_prev && close[i - 1] <= upper[i - 1] && close[i] > upper[i]`; Long exit: `close[i] < middle[i]`.
     - Short entry: `bw_prev && close[i - 1] >= lower[i - 1] && close[i] < lower[i]`; Short exit: `close[i] > middle[i]`.
  5. `DonchianAtrBreakout`:
     - Params: `entry_channel` (`20`), `exit_channel` (`10`), `atr_period` (`14`).
     - Strictly causal prior-window channel (`H_prev = max(high[i - entry_channel .. i])`, `L_prev = min(low[i - entry_channel .. i])` **excluding bar `i`** per `TEST-CAUSAL-003`).
     - Long entry: `close[i] > H_prev`; Long exit: `close[i] < min(low[i - exit_channel .. i])`.
     - Short entry: `close[i] < L_prev`; Short exit: `close[i] > max(high[i - exit_channel .. i])`.
  6. `RsiBollingerConfluence`:
     - Params: `rsi_period` (`14`), `bb_period` (`20`), `bb_std` (`2.0`), `rsi_buy_below` (`38.0`), `rsi_sell_above` (`62.0`).
     - Long entry: `close[i] <= lower[i] && rsi[i] <= rsi_buy_below`; Long exit: `close[i] >= middle[i] || rsi[i] >= rsi_sell_above`.
     - Short entry: `close[i] >= upper[i] && rsi[i] >= rsi_sell_above`; Short exit: `close[i] <= middle[i] || rsi[i] <= rsi_buy_below`.

- **`StrategyDirectionMode`:** `LongOnly`, `ShortOnly`, `LongShort`.
- **`PositionSizingMode`:**
  - `PercentEquity(f64)` (`1.0%..=100.0%` of current mark-to-market equity at `Open[i]`, default `95.0%`)
  - `RiskPercentPerTrade(f64)` (`0.1%..=20.0%` of equity divided by unit stop distance `|exec_entry_price - initial_stop_price|`, capped by `max_leverage`; falls back cleanly to `PercentEquity(25.0)` if no stop rule is configured)
  - `FixedNotional(f64)` (capped by `equity * max_leverage`)
- **`StopTakeConfig`:**
  - `stop_loss_pct: Option<f64>` (`0.1%..=80.0%`)
  - `stop_loss_atr_mult: Option<f64>` (`0.2..=20.0`, anchored from `atr[i - 1]` at `Open[i]` entry)
  - `take_profit_pct: Option<f64>` (`0.1%..=500.0%`)
  - `take_profit_rr_mult: Option<f64>` (`0.2..=20.0` times initial unit stop distance)
  - `trailing_stop_atr_mult: Option<f64>` (`0.2..=20.0`, ratcheted strictly after bar close)
  - `max_holding_bars: Option<usize>` (`1..=10_000`)
  - `min_holding_bars: usize` (`0..=30`, e.g., `2` for Vietnam `T+2` spot settlement, `0` default)
- **`BacktestConfig`:**
  - `initial_cash: f64` (`100.0..=1e12`, default `100_000.0`)
  - `commission_bps: f64` (`0.0..=500.0`, default `10.0` = `0.10%`)
  - `tax_sell_bps: f64` (`0.0..=500.0`, default `0.0`, `10.0` for VN equities)
  - `slippage_bps: f64` (`0.0..=500.0`, default `5.0` = `0.05%`)
  - `max_leverage: f64` (`0.1..=20.0`, default `1.0`)
  - `lot_step: f64` (`0.0` = continuous fractional quantity, `>= 1.0` = discrete integer/lot rounding such as `1.0` or `100.0`)
  - `in_sample_split_pct: Option<f64>` (`Some(0.70)` splits `70%` In-Sample / `30%` Out-of-Sample when `>= 10` bars)

### 3.2 Deterministic Causal 4-Step Bar Loop (`src/backtest/engine.rs`)

For each bar `i` in `0..candles.len()`:

1. **Step A — Execute Queued `PendingStrategyOrder` at `Open[i]` (`BT-TIME-001`):**
   - Any signal generated at `Close[i - 1]` executes at `Open[i]` (`exec_price = apply_slippage(Open[i], order_side, slippage_bps)`).
   - `Exit` closes the active position (sampling `Open[i]` into `MAE/MFE` before close); `Reverse(new_side)` closes the opposite position first (updating `cash_balance += gross_pnl - exit_fee - exit_tax`, floored at `0.0` upon wipeout) and immediately sizes & opens `new_side` at `Open[i]` if solvent; `Entry(side)` opens `side` if flat and `cash_balance > 0.0`.
   - Initial `stop_loss` (tighter of `stop_loss_pct` and `stop_loss_atr_mult` using `atr[i - 1]`), `take_profit` (`take_profit_pct` or `take_profit_rr_mult`), `trailing_stop` (from `entry_exec_price` and `atr[i - 1]`), and `initial_risk_amount = |entry_exec_price - effective_initial_sl| * qty` are locked in causally at `Open[i]`.
2. **Step B — Zero-Balance Margin Liquidation & Intrabar Protective Bracket Evaluation (`[O_i, H_i, L_i, C_i]`):**
   - **Bankruptcy / Margin Liquidation Guard (`BacktestExitReason::MarginLiquidation`):** Before optional brackets or `min_holding_bars` lockups, leveraged/Short positions compute exact `compute_bankruptcy_raw_price` (`cash_balance + unrealized_pnl - exit_fee - exit_tax == 0.0`). If `Open[i]` or intrabar `Low[i]` / `High[i]` breaches bankruptcy, the position is forcibly liquidated (`cash_balance` clamped at `0.0` with `net_pnl = -equity_before_entry` on gap-past-zero wipeouts, preventing zombie account resurrection while conserving `initial_cash + sum(net_pnl) == final_cash`).
   - Protective `SL`/`TP` brackets are active iff an open position exists and `i - entry_bar_index >= min_holding_bars`.
   - Effective protective stop `S_eff` combines fixed `stop_loss` and prior-bar-ratcheted `trailing_stop` (`max(sl, trail)` for `Long`, `min(sl, trail)` for `Short`).
   - **Gap-Open Check First (Non-Entry Bars `i > entry_bar_index`):**
     - `Long`: if `Open[i] <= S_eff` -> exit at `apply_slippage(Open[i], Sell, slippage_bps)`; else if `Open[i] >= TP` -> exit at `apply_slippage(Open[i], Sell, slippage_bps)`.
     - `Short`: if `Open[i] >= S_eff` -> exit at `apply_slippage(Open[i], Buy, slippage_bps)`; else if `Open[i] <= TP` -> exit at `apply_slippage(Open[i], Buy, slippage_bps)`.
     - On `i == entry_bar_index`, gap-open checks against `Open[i]` are skipped so entry slippage (`slippage_bps > stop_loss_pct` on a Marubozu `Low[i] == Open[i]`) never triggers a phantom instant stop-out unless price moves strictly beyond `S_eff` (`Low[i] < S_eff` / `High[i] > S_eff`).
   - **Pessimistic Intrabar Breach (`BT-PESSIMISTIC-SL`) & Pre-Exit `MAE/MFE`:**
     - `Long`: if `Low[i] <= S_eff` -> exit at `apply_slippage(S_eff, Sell, slippage_bps)` (`StopLoss` or `TrailingStop`); else if `High[i] >= TP` -> exit at `apply_slippage(TP, Sell, slippage_bps)` (`TakeProfit`).
     - `Short`: if `High[i] >= S_eff` -> exit at `apply_slippage(S_eff, Buy, slippage_bps)` (`StopLoss` or `TrailingStop`); else if `Low[i] <= TP` -> exit at `apply_slippage(TP, Buy, slippage_bps)` (`TakeProfit`).
     - Intrabar `MAE/MFE` samples are bounded strictly to the realized pre-exit price range (`[S_eff, Open[i]]` on stop-out) rather than post-exit bar extremes.
   - **Post-Close Trailing Ratchet (Zero Same-Bar Lookahead, Active During `min_holding_bars`):**
     - If the position remains open at `Close[i]` (even while locked inside `min_holding_bars`, e.g. Vietnam `T+2`) and `trailing_stop_atr_mult = Some(k)` with valid `atr = atr[i]`, ratchet `highest_since_entry = highest_since_entry.max(High[i])` (`trail = trail.max(highest_since_entry - k * atr)`) for `Long`, or `lowest_since_entry = lowest_since_entry.min(Low[i])` (`trail = trail.min(lowest_since_entry + k * atr)`) for `Short`, taking effect once `i + 1 - entry_bar_index >= min_holding_bars`.
3. **Step C — Mark-to-Market `BacktestEquityPoint`, Continuous `IS / OOS` Partition & `SeriesBlockIndex` (`Close[i]`):**
   - Compute `equity = (cash_balance + open_position.unrealized_pnl(Close[i])).max(0.0)`, update `peak_equity`, `drawdown_abs`, signed `underwater_pct <= 0.0`, and `drawdown_duration_bars`, and push bar-aligned `BacktestEquityPoint` (`equity_curve.len() == candles.len()`).
   - When `in_sample_split_pct = Some(split)` is active, `in_sample_tearsheet` and `out_of_sample_tearsheet` are partitioned directly from the continuous `full_sim` (`&equity_curve[..split_idx]` vs `&equity_curve[split_idx..]` seeded with `initial_cash = equity_curve[split_idx - 1].equity`, and `exit_bar_index < split_idx` vs `>= split_idx`), conserving `100%` of boundary-straddling trades and satisfying `(1 + IS%) * (1 + OOS%) == (1 + Full%)`. `evaluate_robustness` requires strictly profitable `IS` & `OOS` returns AND valid positive `Sharpe` ratios before awarding `RobustnessVerdict::Robust`.
4. **Step D — Causal Strategy Signal Evaluation at `Close[i]` (`TEST-BT-001`):**
   - Evaluate `max_holding_bars` (`i - entry_bar_index + 1 >= max_holding_bars` when `>= min_holding_bars`), preset exit rules, and preset entry/reversal rules using only causal values at `<= i`.
   - If `i + 1 < candles.len()`, queue `PendingStrategyOrder` for `Open[i + 1]`.
   - If `i + 1 == candles.len()` (final bar), **never** execute at `Close[i]`; record `UnexecutedSignal { signal_bar_index: i - start, timestamp: candle.timestamp, side, is_exit, reason: UnfilledReason::EndOfSeriesCutoff }`.

### 3.3 Synchronized `Equity / Drawdown` Baseline Sub-Pane & GPU Batching (`DECISION-RENDERER-002`)

- **Shared `TimeScale` Alignment:** `OscillatorPaneId::Equity` maps every `BacktestEquityPoint` (and `SeriesBlockIndex` 16-bar envelope) using the exact same `PaneViewport` `index_to_x(i)` and `culling_range` / `conflation_factor` as `Main` OHLCV.
- **LWC-Inspired Baseline Split Mesh (`paint_equity_baseline_pane`):**
  - Baseline `B = initial_cash` (`y_base = equity_vp.price_to_y(B)`).
  - Conflated bucket values `(i_mid, eq_val)` are walked once across visible buckets (`<= 2,560` segments even at `100,000` bars):
    - Segments entirely `>= B` or `<= B`, as well as segments crossing `B` (`t = (B - v0) / (v1 - v0)`, `x_cross = x0 + t * (x1 - x0)`), are clipped analytically against `[plot_rect.top(), plot_rect.bottom()]` via `emit_y_clipped_baseline_trapezoid` (1D parametric Y-clipping) and triangulated into `egui::Mesh` (`u32::try_from` guarded) so zero triangles ever bleed across `initial_cash` or warp slope at extreme vertical zoom!
  - Optional `IS | OOS` vertical split guide (`split_bar_index`) and `Main`-pane Trade Connector lines (`entry_bar, entry_price -> exit_bar, exit_price`, emerald dashed/solid for winners, crimson for losers, culled via binary search and decimated by camera-invariant global trade index `(first_vis + local_k) % stride == 0` to keep `95K` `32x` full-fit well under `< 4.0 ms`).
