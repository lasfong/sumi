# CAUSAL BAR REPLAY & PAPER TRADING EXECUTION SPECIFICATION (`MILESTONE R07 / WAVE 6`)

- **Session Identity:** `SESSION-R07-20261112T093742+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Research Gate Status:** `VERIFIED` (`DEC-005` Cleared — Live REA MCP + LWC v5.2.0 Apache 2.0 + Legacy Sumi Archive)
- **Authorized Reference Targets:**
  1. **Lightweight Charts v5.2.0** (`/Users/mizuhara/workspace/sumi/research_repos/lightweight-charts-5.2.0/src/{model/custom-price-line.ts,model/price-line-options.ts,views/pane/custom-price-line-pane-view.ts,views/price-axis/custom-price-line-price-axis-view.ts,plugins/series-markers/types.ts,plugins/series-markers/primitive.ts,plugins/series-markers/pane-view.ts,plugins/series-markers/series-markers-arrow.ts}`, commit `868cae27bd1acafa0128d8d868ea740a59ae42ce`, `Apache-2.0`)
  2. **Legacy Sumi Archive** (`/Users/mizuhara/workspace/sumi/ARCHIVE/frontend/src/{types/replay.ts,types/trade.ts,types/practice.ts,store/replayStore.ts,features/practice/practiceDomain.ts,components/chart/PositionLineManager.ts,components/replay/ReplayWorkspaceController.tsx,components/replay/TradeControls.tsx,components/replay/ReplayControlDock.tsx}` and `/Users/mizuhara/workspace/sumi/ARCHIVE/backend/app/{services/replay_service.py,services/trade_lifecycle_service.py,domain/accounting.py}`, plus `/Users/mizuhara/workspace/sumi/ARCHIVE/backend/tests/test_b10_replay_lookahead_lock.py`)
  3. **Compliance Boundary (`DEC-006`):** Proprietary TradingView Desktop was **not** inspected, hooked, or queried.

---

## 1. Live REA MCP Execution Evidence (`Milestone R07`)

All REA operations below were executed live via the `rea-agents@6.8.0` MCP server during `SESSION-R07-20261112T093742+0700`:

| Op ID | REA Tool & Query | Target Scope | Actual Verified Output | Exact 64-Hex Evidence ID |
| :--- | :--- | :--- | :--- | :--- |
| `REA-R07-01` | `analyze_javascript_application` (`detail: "summary"`) | `ARCHIVE/frontend` (`9` TS/TSX files: `replay.ts`, `trade.ts`, `practice.ts`, `replayStore.ts`, `practiceDomain.ts`, `PositionLineManager.ts`, `ReplayWorkspaceController.tsx`, `TradeControls.tsx`, `ReplayControlDock.tsx`) | Root artifact SHA-256 `243b2d6eaac9d00e0cd2e2623da5ea5f84bb751ad89e2ba40a04be7f7b794b92`; `9` files parsed (`0` failures), `96,383` bytes, `13,507` AST nodes, `252` findings, `5,579` `data-flow`, `764` `closure`, `612` `object-flow`, `55` `call-flow`, `37` `boundary`, `19` `promise-ownership` relations (`10,350/10,350` view items). | Parent: `ev_10aea63b7d75e5911313a827e425d9b167fc84dcd35cbca2d37cca7ee23ecaa2`<br>Summary: `ev_d1e347a94fa83d540b14f9e1237a505e500330c09334df10b3ca69c56cf3eec7` |
| `REA-R07-02` | `analyze_javascript_application` (`detail: "summary"`) | `research_repos/lightweight-charts-5.2.0` (`8` TS files: `custom-price-line.ts`, `price-line-options.ts`, `custom-price-line-pane-view.ts`, `custom-price-line-price-axis-view.ts`, `types.ts`, `primitive.ts`, `pane-view.ts`, `series-markers-arrow.ts`) | Root artifact SHA-256 `6a26c4adafe9539017d004f611425bc9692a278cd45d4a41ffe381afd7b6c9d5`; `8` files parsed (`0` failures), `29,062` bytes, `4,188` AST nodes, `156` findings, `1,147` `data-flow`, `212` `object-flow`, `71` `closure`, `46` `call-flow` relations (`1,153/1,153` view items). | Parent: `ev_9527ede014f2230fe6d8071cd64bdf52b0c7351002bbc7ded148e71cffa3920e`<br>Summary: `ev_c102b1d95f50a2ffde74c2c0e54051e0ad7514757116b74ffadce3c3f511a8bd` |
| `REA-R07-03` | `trace_javascript_semantics` (`property: "current_index"`, `forward-influence`) | Parent `ev_10aea63b7d75e5911313a827e425d9b167fc84dcd35cbca2d37cca7ee23ecaa2` (`ARCHIVE/frontend`) | `10` seed matches, `10` traversed nodes across `9` functions in `2` modules (`ReplayWorkspaceController.tsx` and `practiceDomain.ts`, `24` relevant unknowns), proving legacy replay coupled cursor progression to HTTP integer indices rather than a local causal UTC timestamp cutoff. | `ev_418c861e8c803e99b91a723e032dc634f14535ca9c3af6c41b24865532708120` |
| `REA-R07-04` | `trace_javascript_semantics` (`property: "stop_loss"`, `forward-influence`) | Parent `ev_10aea63b7d75e5911313a827e425d9b167fc84dcd35cbca2d37cca7ee23ecaa2` (`ARCHIVE/frontend`) | `1` seed match, `1` traversed node in `practiceDomain.ts` (`validateDecision` SL/TP directional ordering checks, `7` relevant unknowns). | `ev_eaab36ba9003e7a59f65337147feb3c96ee337a0bc680c81f720a66993d673b6` |
| `REA-R07-05` | `trace_javascript_semantics` (`property: "createPriceLine"`, `backward-provenance`) | Parent `ev_10aea63b7d75e5911313a827e425d9b167fc84dcd35cbca2d37cca7ee23ecaa2` (`ARCHIVE/frontend`) | `3` seed matches, `3` traversed nodes in `PositionLineManager.ts` (`entryLine` `#58A6FF`, `stopLossLine` `#FF1744`, `takeProfitLine` `#00E676` dashed horizontal price lines with `%` delta titles). | `ev_4f49eb615c9d391d2a85173f9c756db334f85001815ff3388e68305887cc4804` |
| `REA-R07-06` | `trace_javascript_semantics` (`property: "stopPrice"`, `forward-influence`) | Parent `ev_10aea63b7d75e5911313a827e425d9b167fc84dcd35cbca2d37cca7ee23ecaa2` (`ARCHIVE/frontend`) | `0` seed matches (`0` traversed nodes/relations) — negative control confirming legacy Sumi lacked `STOP` entry orders (`trade.ts:47` supported only `MARKET_AT_CLOSE`, `MARKET_NEXT_OPEN`, `LIMIT`, `CUSTOM_PRICE`). | `ev_e1958292c0e30098e2149b6c209d128e3e90117bc13de936b2e1dd7b30b06ce2` |
| `REA-R07-07` | `trace_javascript_semantics` (`property: "aboveBar"`, `forward-influence`) | Parent `ev_9527ede014f2230fe6d8071cd64bdf52b0c7351002bbc7ded148e71cffa3920e` (`LWC v5.2.0`) | `3` seed matches, `3` traversed nodes across `3` functions in `pane-view.ts` and `primitive.ts` (`SeriesMarkersPaneView` stacked `offsets.aboveBar` / `offsets.belowBar` and `invertScale` high/low swap). | `ev_81fa9171fb5a1977cd367d8574171f7ae3dababa2200c602c9d1d3a11bd43a2f` |
| `REA-R07-08` | `trace_javascript_semantics` (`property: "axisLabelVisible"`, `forward-influence`) | Parent `ev_9527ede014f2230fe6d8071cd64bdf52b0c7351002bbc7ded148e71cffa3920e` (`LWC v5.2.0`) | `1` seed match in `custom-price-line-price-axis-view.ts:30` (`CustomPriceLinePriceAxisView::_updateRendererData` axis pill + pane title label coordination). | `ev_7e0b29f4807f4094e4e5b3da66c0b15da95197917d332920308b7ea5de93fc75` |
| `REA-R07-09` | `trace_application_feature` (`direction: "both"`, `jag_node_fc11a5f4479892069883d3c58929dc14deb7269e703302a7f748bc1816002c9c`) | Parent `ev_10aea63b7d75e5911313a827e425d9b167fc84dcd35cbca2d37cca7ee23ecaa2` (`ARCHIVE/frontend`) | Retained evidence generated (`response_bytes_at_least = 10,579,289 > 10,485,760` `REA_MCP_MAX_RESPONSE_BYTES` stdio transport limit due to `14,146` unselected repository frontier unknowns; recorded as explicit transport boundary limitation). | `ev_3f2322b664237765df117aed33208ea92affe9f8e4131f748234b3e444effd30` |
| `REA-R07-10` | `trace_application_feature` (`direction: "outgoing"`, `jag_node_2db1c60e1cb348888000504f804a47b56ab4e17f226c8d013a4daefc107beb62`) | Parent `ev_9527ede014f2230fe6d8071cd64bdf52b0c7351002bbc7ded148e71cffa3920e` (`LWC v5.2.0`) | `1` matched seed, `13` traced nodes, `13` traced edges, `4` observed facts, `24` inferred facts (`0` unknown/unavailable facts) in `CustomPriceLinePriceAxisView`. | `ev_19311ed5338a92d5bc3a6b29746fc527552aafdc63be5fdb3a11b3b5db1c2930` |

### Six Legacy Replay & Execution Defects Discovered & Rectified in Rust (`src/replay/`, `src/trading/`)

1. **HTTP Round-Trip Latency & Dropped Partial Higher-Timeframe Bars (`ReplayWorkspaceController.tsx:207–249`, `replay_service.py:172–229`):**
   - Legacy replay issued 4 HTTP calls per candle step, lacked intraday multi-timeframe replay (`1m..4h`), and omitted in-progress weekly candles (`if current_date < last_trading_day: continue`), leaving a multi-day blind gap between Monday and Friday.
   - **Rust Fix:** Pure in-memory `ReplayController` (`src/replay/`) anchored to `base_cursor_index` (`cutoff_timestamp = base_candles[base_cursor_index].timestamp`) materializes any `active_tf >= base_tf` via `aggregate_ohlcv_up_to`, rendering live partial higher-timeframe bars causally with `< 3 ms` latency and zero future leakage (`DEF-001`).
2. **Non-Causal Rewind State Corruption (`replay_service.py:366–378`, `test_b10_replay_lookahead_lock.py:153–160`):**
   - Legacy `previous_candle` decremented `current_index` without reversing future executions on `current_cash` or `Position`, requiring fragile endpoint masking and breaking new trade placement after rewind.
   - **Rust Fix:** Deterministic command-log prefix replay (`TradingSession::rewind_to`) records user trading commands stamped with `base_index` and reconstructs exact causal portfolio state (`cash_balance`, `open_position`, `pending_orders`, `fills`, `closed_trades`, `equity_curve`) when rewinding or jumping backward.
3. **Missing `STOP` Orders, `100`-Share Lot Lockout & Short-Selling Ban (`trade.ts:47`, `trade_lifecycle_service.py:75, 137, 295–394`):**
   - Legacy rejected fractional crypto (`BTCUSDT`) and single-contract futures (`VN30F1M`), lacked `Buy Stop` / `Sell Stop` breakout orders, and blocked `Short` positions in the backend despite frontend UI labels.
   - **Rust Fix:** Full symmetric `Long` and `Short` paper trading with `Market`, `Limit`, and `Stop` orders, arbitrary finite `quantity > 0.0`, configurable `AccountConfig` (`initial_cash`, `commission_bps`, `slippage_bps`, `allow_short`, `max_leverage`), and bracket `stop_loss` / `take_profit` protection.
4. **Close-Before-Intrabar Margin Call & Incomplete Pending-Fill Bracket Evaluation (`replay_service.py:288–340`, `trade_lifecycle_service.py:40–63`):**
   - Legacy evaluated bankruptcy on `candle.close` *before* checking intrabar `Stop Loss` and never evaluated newly filled pending orders' attached `SL`/`TP` on the Remainder of the entry bar.
   - **Rust Fix:** Strict causal per-bar execution pipeline:
     1. Pre-existing open position checks **Gap-Open (`candle.open`)** first, then **Conservative Pessimistic Intrabar `[low, high]` (`StopLoss` before `TakeProfit` on ambiguous dual-touch bars)**.
     2. Pending `Limit` / `Stop` orders evaluate in deterministic FIFO `order_id` order with gap-open price improvement (`Limit`) and gap-open adverse fill (`Stop`), followed by post-entry remaining-path bracket checks along the deterministic 4-point OHLC path (`[O, L, H, C]` if `C >= O`, `[O, H, L, C]` if `C < O`).
     3. End-of-bar mark-to-market updates `unrealized_pnl`, `equity_curve`, and `max_drawdown` on `candle.close`.
5. **Pre-Replay Future Drawing Leakage vs In-Replay Whitespace Projections:**
   - Rewinding on a chart containing pre-drawn annotations anchored to future peaks/troughs leaks future market structure, whereas hiding *all* `anchor.timestamp > cutoff_timestamp` would break projections drawn into right-hand whitespace *during* replay.
   - **Rust Fix:** Stamp `Drawing::created_at_timestamp: Option<i64>` (`Some(cutoff_timestamp)` when created during active replay, `None` otherwise). During active replay at `T_cut`:
     - Drawings with `Some(created_ts)` are visible iff `created_ts <= T_cut` (preserving causal right-offset projections while hiding drawings created before a rewind).
     - Pre-replay drawings (`None`) are visible iff every `anchor.timestamp` is `<= T_cut as f64` (hiding annotations anchored to future candles without deleting them; full restoration on `Exit Replay`).
6. **DOM-Scraped Risk-Reward Sync (`TradeControls.tsx:86–115`):**
   - Legacy parsed `document.querySelector('[data-drawing-interaction-state]')` JSON strings to copy prices into the order ticket.
   - **Rust Fix:** Type-safe Rust inspection of the selected `DrawingGeometry` (`FibonacciRetracement` / `Rectangle` / `HorizontalLine`) directly populates `entry` / `stop_loss` / `take_profit` in the workstation trading ticket with zero serialization overhead.

---

## 2. Subsystem Architecture & Mathematical Contracts

Both `src/replay/` and `src/trading/` are **100% pure `f64`/`i64` Rust** with **zero new crates** in `Cargo.toml` and **zero OS/clock dependencies** (`wasm32-unknown-unknown` verified; timer progression receives explicit `dt_secs: f64` from the host UI loop).

### 2.1 Causal Bar Replay Engine (`src/replay/`)

- **Core Types (`src/replay/mod.rs`, `src/replay/controller.rs`):**
  - `ReplaySpeed`: `X1` (`1.0` step/s), `X2` (`2.0` steps/s), `X5` (`5.0` steps/s), `X10` (`10.0` steps/s), `X20` (`20.0` steps/s).
  - `ReplayStepUnit`:
    - `ActiveBar` (default): each step advances `base_cursor_index` to the final base bar of the current incomplete `active_tf` bucket, or if already at a bucket's final base bar, to the final base bar of the next `active_tf` bucket.
    - `BaseBar` ("Sub-bar Tick"): each step advances `base_cursor_index` by `+1` base candle so higher-timeframe (`15m`, `4h`, `1D`, `1W`) candles form dynamically tick-by-tick.
  - `ReplayStatus`: `Inactive`, `Paused`, `Playing`, `Completed` (`base_cursor_index == base_candles.len() - 1`). Interactive cut-bar selection (`selecting_cut_bar: bool`) is kept strictly orthogonal to `ReplayStatus` on `ReplayController` so activating `Cut Bar` while already paused in Replay never drops `status.is_active()` or exposes future bars (`DEF-001`).
  - `ReplayController`:
    - `status: ReplayStatus`
    - `selecting_cut_bar: bool`
    - `start_base_index: usize`
    - `base_cursor_index: usize`
    - `speed: ReplaySpeed`
    - `step_unit: ReplayStepUnit`
    - `playback_accumulator_secs: f64` (clamped per frame so background tab stalls never burst more than `MAX_STEPS_PER_FRAME = 8` steps in one UI frame).
- **Zero-Lookahead Materialization (`ReplayController::materialize_candles`):**
  - Given `base_candles: &[Candle]`, `base_tf: Timeframe`, `active_tf: Timeframe`:
    - `cutoff_timestamp = base_candles[base_cursor_index].timestamp`.
    - Calls `aggregate_ohlcv_up_to(base_candles, base_tf, active_tf, cutoff_timestamp)`.
    - **Theorem (`DEF-001` Zero-Lookahead):** Because `aggregate_ohlcv_up_to` slices `&base_candles[..end]` via `partition_point(|c| c.timestamp <= cutoff_timestamp)`, no candle with `timestamp > cutoff_timestamp` can influence `self.candles`, `PriceBounds`, `IndicatorCache` (`SMA`, `EMA`, `BB`, `RSI`, `MACD`, `ATR`), or crosshair OHLCV readouts.
    - **Duplicate-Timestamp Guard:** Because `normalize_loaded_candles` enforces strictly increasing timestamps on deduplicated series, `end == base_cursor_index + 1`. If caller-constructed test slices contain duplicate timestamps at `cutoff_timestamp`, slicing `&base_candles[..=base_cursor_index]` directly guarantees exact bar-index causality even on synthetic duplicate timestamps! Accordingly, `ReplayController::materialize_candles` slices `&base_candles[..=self.base_cursor_index]` and passes that exact prefix into `aggregate_ohlcv(prefix, base_tf, active_tf)`.

### 2.2 Paper Trading Execution & Portfolio Engine (`src/trading/`)

- **Core Domain Types (`src/trading/mod.rs`, `src/trading/orders.rs`, `src/trading/portfolio.rs`):**
  - `AccountConfig`:
    - `initial_cash: f64` (default `100_000.0`, sanitized to `[100.0, 1.0e12]`)
    - `commission_bps: f64` (default `5.0` bps = `0.05%`, sanitized to `[0.0, 500.0]`)
    - `slippage_bps: f64` (default `2.0` bps = `0.02%`, sanitized to `[0.0, 500.0]`)
    - `max_leverage: f64` (default `1.0`, sanitized to `[1.0, 100.0]`)
    - `allow_short: bool` (default `true`)
  - `OrderSide`: `Buy`, `Sell`
  - `PositionSide`: `Long`, `Short`
  - `OrderKind`:
    - `Market`
    - `Limit { limit_price: f64 }`
    - `Stop { stop_price: f64 }`
  - `OrderStatus`: `Pending`, `Filled`, `Cancelled`, `Rejected { reason: String }`
  - `FillReason`: `MarketImmediate`, `LimitTriggered`, `StopTriggered`, `StopLossExit`, `TakeProfitExit`, `ManualFlatten`
  - `Order`: `id: u64`, `side: OrderSide`, `kind: OrderKind`, `quantity: f64`, `stop_loss: Option<f64>`, `take_profit: Option<f64>`, `submitted_base_index: usize`, `submitted_timestamp: i64`, `status: OrderStatus`, `filled_price: Option<f64>`, `filled_timestamp: Option<i64>`
  - `Position`:
    - `side: PositionSide`
    - `quantity: f64`
    - `avg_entry_price: f64`
    - `opened_timestamp: i64`
    - `opened_base_index: usize`
    - `stop_loss: Option<f64>`
    - `take_profit: Option<f64>`
    - `initial_stop_loss: Option<f64>`
    - `initial_risk_amount: Option<f64>`
    - `total_entry_qty: f64`
    - `total_entry_notional: f64`
    - `total_entry_fees: f64`
    - `open_entry_fees: f64`
    - `closed_qty_acc: f64`
    - `exit_notional_acc: f64`
    - `realized_gross_pnl_acc: f64`
    - `realized_exit_fees_acc: f64`
  - `ExecutionFill`: `fill_id: u64`, `order_id: u64`, `side: OrderSide`, `reason: FillReason`, `quantity: f64`, `price: f64`, `fee: f64`, `realized_net_pnl: Option<f64>`, `base_index: usize`, `timestamp: i64`
  - `ClosedTrade`: `trade_id: u64`, `side: PositionSide`, `quantity: f64`, `entry_price: f64`, `exit_price: f64`, `entry_timestamp: i64`, `exit_timestamp: i64`, `gross_pnl: f64`, `total_fees: f64`, `net_pnl: f64`, `return_pct: f64`, `r_multiple: Option<f64>`
  - `EquityPoint`: `base_index: usize`, `timestamp: i64`, `equity: f64`, `drawdown_pct: f64`
  - `PortfolioMetrics`: `cash_balance: f64`, `unrealized_pnl: f64`, `equity: f64`, `net_pnl: f64`, `return_pct: f64`, `max_drawdown_pct: f64`, `pending_order_count: usize`, `fill_count: usize`, `closed_trade_count: usize`, `win_count: usize`, `loss_count: usize`, `win_rate_pct: f64`, `profit_factor: Option<f64>`, `avg_r_multiple: Option<f64>`

- **Exact Accounting Invariants:**
  1. **Slippage:** $s = \text{slippage\_bps} \times 10^{-4}$; `Buy` executes at $p_{\text{raw}}(1 + s)$, `Sell` executes at $p_{\text{raw}}(1 - s)$.
  2. **Commission:** $f = p_{\text{exec}} \cdot q \cdot (\text{commission\_bps} \times 10^{-4})$.
  3. **Exact Fee Conservation Across Scale-Ins & Partial Closes:**
     - On entry / scale-in of $+q$: `cash_balance -= f_entry`; `position.open_entry_fees += f_entry`.
     - On partial close of $q_{\text{close}} < q_{\text{open}}$: allocated entry fee is $f_{\text{alloc}} = \text{open\_entry\_fees} \cdot (q_{\text{close}} / q_{\text{open}})$, `open_entry_fees -= f_alloc`.
     - On final close ($q_{\text{close}} == q_{\text{open}}$): $f_{\text{alloc}} = \text{open\_entry\_fees}$, `open_entry_fees = 0.0`.
     - **Conservation Theorem:** Whenever `open_position.is_none()`, $\text{cash\_balance} \equiv \text{initial\_cash} + \sum_{i} \text{closed\_trades}[i].\text{net\_pnl}$ within $\le 10^{-9}$!
  4. **Timeframe-Invariant Execution Theorem:** Because `TradingSession` processes newly revealed `base_candles[(prev_cursor + 1)..=new_cursor]` in strict chronological base-bar order regardless of `active_timeframe`, switching `1m <-> 15m <-> 4h <-> 1D <-> 1W` during replay produces **bit-for-bit identical** fills, cash, PnL, and closed trades!

### 2.3 On-Chart Visual Overlays (`src/chart.rs`) & Workstation HUD (`src/app.rs`)

1. **Replay Cut Scrubber (`ReplayController::selecting_cut_bar`):**
   - Translucent right-side future curtain + vertical `#FFB300` cut line + floating `"<< CUT REPLAY HERE · <date>"` pill; primary click on main/oscillator plot sets the replay cut bar without triggering drawing creation or panning.
2. **Position & Working Order Price Lines (`ChartReplayTradingOverlay`):**
   - Rendered on the main price pane using `PaneViewport` `price_to_local_y` (supporting both `Linear`, `Logarithmic`, `Percentage`, `IndexedTo100`, and `invert_scale`):
     - Open Position Entry (`#58A6FF` dashed), Stop Loss (`#FF1744` dashed), Take Profit (`#00E676` dashed) with live PnL / `%` distance badges and right price-axis pills.
     - Working Pending `Limit` / `Stop` orders (`#26A69A` / `#EF5350` dotted-dashed) with order quantity & trigger price tags.
3. **Causal Execution Markers (LWC `SeriesMarkers` Parity):**
   - Binary-searched to the visible `[strict.left ..= strict.right]` range (`O(log F + V)` where `F` = fills, never iterating offscreen fills!), bucket-aligned to `active_tf`, stacked `belowBar` (`Buy` `#00E676` `arrowUp`) and `aboveBar` (`Sell` `#FF5252` `arrowDown`, swapping automatically when `invert_scale` is true), capped at `3` stacked arrows per bar (`+N` overflow counter) to preserve `< 4.0 ms` CPU frame times at `100K` full-fit.
4. **Workstation Replay Dock & Paper Trading Panel (`src/app.rs`):**
   - Top/dock Replay Transport Bar: `✂ Replay`, `|< Start`, `-1`, `▶ Play / ⏸ Pause` (`Space`), `+1` (`→` / `N`), `Speed (1x..20x)`, `Step Unit (Bar / Base Tick)`, progress readout `#idx / total (UTC)`, and `✕ Exit Replay`.
   - Collapsible Paper Trading Ticket & Blotter Bar/Side Panel: `Qty`, `Order Kind (Market / Limit / Stop)`, `Price`, `SL`, `TP`, `Sync from Selected Drawing`, `Buy / Long`, `Sell / Short`, `Flatten`, Pending Orders cancel buttons, and live `Cash | Unrealized | Equity | Net PnL | Max DD | Win Rate` scoreboard.


---

## 3. Adversarial `sumi-critic` Review & 10-Finding Remediation Ledger

Independent adversarial subagent `sumi-critic` (`conversationId: 4115161c-0ed8-46c9-8e1f-4f64c3438c7a`) audited all Milestone R07 modules (`src/replay/`, `src/trading/`, `src/drawings/`, `src/chart.rs`, `src/app.rs`). All **10/10 findings** (`2 CRITICAL`, `2 HIGH`, `3 MEDIUM`, `3 LOW`) were remediated in code and locked by dedicated `critic_r07_regression_*` automated tests:

| ID | Severity | Finding Summary | Rust Remediation & Regression Test |
| :--- | :--- | :--- | :--- |
| `CRITICAL-01` | `CRITICAL` | `DrawingStore::undo()`, `redo()`, `selected_drawing()`, and `clear_unlocked()` bypassed causal replay visibility, allowing future Fib/Rectangle anchors to leak into `Order Ticket` via Undo/Redo or wiping hidden future drawings on `Clear`. | Added `causal_visible_candles` to `DrawingStore`; gated `selected_drawing()`, `has_unlocked_drawings()`, `delete_selected()`, `mutate_selected()`, and `clear_unlocked()` with `is_causally_visible`, and invoked `sanitize_causal_selection_and_hover()` after `undo()`, `redo()`, and `remap_timeframe()`. Verified in `replay::tests::critic_r07_regression_causal_undo_redo_clear_unlocked_whitespace_drag_and_dense_hit_test`. |
| `CRITICAL-02` | `CRITICAL` | Pure reduce/close orders (`Sell` against `Long` or `Buy` against `Short`) inherited stale opposite-side Ticket `SL`/`TP` inputs, and pending/flip fills lacked post-fill directional bracket sanitization, risking immediate phantom `StopLossExit`/`TakeProfitExit`. | Stripped bracket `SL`/`TP` on `is_pure_reduce` orders in `apply_submit_order_internal` and added defense-in-depth `sanitize_brackets_for_position` inside `execute_fill_mutation`. Verified in `trading::tests::critic_r07_regression_pure_reduce_stale_brackets_and_gap_open_bracket_sanitization`. |
| `HIGH-01` | `HIGH` | Dragging a pre-replay drawing (`created_at_timestamp == None`) into right-hand replay whitespace (`bar_index > last`) caused it to vanish mid-drag because `anchor.timestamp` became `None`. | Stamped `d.created_at_timestamp = Some(cutoff_ts)` on first drag mutation (`DraggingHandle` / `DraggingBody`) during active replay while preserving `snapshot_before` on `undo_stack`. Verified in `replay::tests::critic_r07_regression_causal_undo_redo_clear_unlocked_whitespace_drag_and_dense_hit_test`. |
| `HIGH-02` | `HIGH` | Empty-canvas deselect click in `src/chart.rs` called uncausal `hit_test_pane_drawings` (allowing invisible future zones to block deselection), and the `> 32` dense-stack fallback in `hit_test.rs` skipped `is_causally_visible`. | Switched empty-canvas check to `hit_test_pane_drawings_causal` and enforced `drawing.is_causally_visible(...)` in the `> 32` fallback path. Verified in `app::tests::critic_r07_regression_conflated_boundary_fill_markers_empty_click_deselect_and_cut_tool_cancel` and `replay::tests::critic_r07_regression_causal_undo_redo_clear_unlocked_whitespace_drag_and_dense_hit_test`. |
| `MEDIUM-01` | `MEDIUM` | Evaluating all `Pending` orders in submission `order_id` FIFO order before traversing the 4-point OHLC path could violate intrabar price-crossing chronology (e.g., `Buy Stop @ 105` submitted before `Buy Limit @ 95` on a bullish `[O=100, L=94, H=106, C=104]` bar). | Upgraded `TradingSession::process_bar_arrival` to unified `t = Open` gap evaluation followed by 3-segment causal 4-waypoint traversal (`[O, L, H, C]` vs `[O, H, L, C]`, retaining pessimistic `SL`-before-`TP` on pre-existing dual-touch bars). Verified in `trading::tests::critic_r07_regression_multi_pending_causal_segment_order_scale_in_r_multiple_and_max_price`. |
| `MEDIUM-02` | `MEDIUM` | `paint_execution_fill_markers` sliced `culling_range` directly instead of bucket-aligned `[aligned_start..aligned_end]`, dropping markers inside partially visible boundary conflation buckets. | Aligned `culling_range` to `(start / cf) * cf .. end.div_ceil(cf) * cf` matching `iter_conflated_candles`, and batched fill-arrow triangles + axis-aligned dashed lines into `egui::Mesh` primitives. Verified in `app::tests::critic_r07_regression_conflated_boundary_fill_markers_empty_click_deselect_and_cut_tool_cancel`. |
| `MEDIUM-03` | `MEDIUM` | In-place `remap_single_anchor` synthesized extrapolated timestamps onto right-whitespace anchors (`bar_index > old_last`, `timestamp == None`) across overnight/weekend gaps, and `refresh_materialized_anchor_timestamps` missed `undo_stack`/`redo_stack`. | Preserved `timestamp == None` on unmaterialized right-whitespace anchors during same-timeframe extensions until `bar_index <= new_last` materializes, and refreshed `undo_stack`/`redo_stack` alongside `drawings`. Verified in `replay::tests::critic_r07_regression_causal_undo_redo_clear_unlocked_whitespace_drag_and_dense_hit_test`. |
| `LOW-01` | `LOW` | Position scale-ins preserved the initial lot's `initial_risk_amount` instead of scaling risk to total open quantity, inflating `ClosedTrade::r_multiple`. | Recomputed `Position::initial_risk_amount` across updated total `pos.quantity` on scale-ins (and initialized `initial_stop_loss` if newly attached). Verified in `trading::tests::critic_r07_regression_multi_pending_causal_segment_order_scale_in_r_multiple_and_max_price`. |
| `LOW-02` | `LOW` | Selecting a drawing tool while `selecting_cut_bar` was active left cut-bar scrubber mode blocking drawing placement, and `ChartReplayTradingOverlay` read a 1-frame-stale `selecting_cut_bar` flag. | Cancelled `selecting_cut_bar` upon non-`Select` drawing tool activation and read live `self.replay.selecting_cut_bar`. Verified in `app::tests::critic_r07_regression_conflated_boundary_fill_markers_empty_click_deselect_and_cut_tool_cancel`. |
| `LOW-03` | `LOW` | `extract_ticket_from_drawing` checked `low > 0.0` without `high <= MAX_ORDER_PRICE` on 2-anchor shapes, and backward replay transitions retained stale `last_trade_status` banners. | Enforced `high <= MAX_ORDER_PRICE` across `FibonacciRetracement`/`Rectangle`/`TrendLine`/`Ray` sync and cleared `last_trade_status` on replay start, reset, and backward steps. Verified in `trading::tests::critic_r07_regression_multi_pending_causal_segment_order_scale_in_r_multiple_and_max_price` and `app::tests::r07_workstation_causal_replay_mtf_zero_lookahead_ticket_sync_rewind_and_instrument_isolation`. |

---

## 4. Final Native Release Binary REA Verification & Benchmark Gate (`Milestone R07`)

- **Final Release Artifact (`target/release/sumi-workstation`):**
  - **SHA-256:** `3b346ece0734771157a9b55b8519d3bec25ae88c357e51be4966cc8dbb13c234` (`17,687,760` bytes, `arm64` Mach-O, UUID `6F4AFBDB-A7A0-3F2C-8938-51EABD7C875D`, `0` new crate dependencies in `Cargo.toml` / `Cargo.lock`).
  - **`inspect_rust_binary` (`REA-R07-11`):** Parent `ev_dbbcdc034a79c2fb2f5ca9517071860887bb2fc7b8dc13d6abd91999e7a21e31`, Summary `ev_b158e5f3771f238f024ff0cc50e19909d6b46a12e8e387f566e5c2892ca29b83` (`1` inferred `rustc` commit hash).
  - **`inspect_macho` (`REA-R07-12`, cleanly closed via `close_binary`):** Parent `ev_59ad2ed90e0e44a43742aad1459723bda4b060a88d47feb578d1994906e71edf`, Summary `ev_5f9d71b0734923f361ab23f2f208808e42aa57b85f8778d740ab6abbba0df82c` (`5,323` items examined: `30` load commands, `14` dylib dependencies unchanged since R03, `324` imports, `4,947` exports, `5` segments).
- **Measured `--release -- --r07` Performance (`docs/evidence/r07_replay_trading_benchmark.md`):**
  - `95,001` `1m -> 15m` zero-lookahead `materialize_candles`: **`1.345 ms`** median (`1.512 ms` P95).
  - `1x 15m` `ActiveBar` step + causal 4-waypoint order matching: **`1.25 µs`** median (`1.29 µs` P95).
  - `5,001`-bar `1m` causal paper-trading run (`~65` orders + brackets → `30` fills): **`304.17 µs`** median (`339.75 µs` P95).
  - `4,501`-bar bit-exact command-log `rewind_to`: **`256.71 µs`** median (`276.83 µs` P95).
  - `95K/100K` `1m` active Replay + 4 panes + 6 indicators + 25 drawings + Position/SL/TP/Fills (`bar_spacing = 8.0`): **`0.174 ms`** median (**PASS `< 4.0 ms`**).
  - `95K/100K -> 15m` Cut Scrubber + 4 panes + 6 indicators + 25 drawings + Fills (`bar_spacing = 9.0`): **`0.196 ms`** median (**PASS `< 4.0 ms`**).
  - `95K/100K` `1m` full-fit `32x` conflated (`Log`, `401,860` vertices) + Replay Frontier + Trading Overlays: **`3.923 ms`** median (**PASS `< 4.0 ms`** `DECISION-RENDERER-002` hybrid gate).
  - Full `SumiWorkstationApp` UI (`Toolbar + Replay Dock + Right Paper Trading Panel + 4 Panes + Overlays`): **`0.291 ms`** median (**PASS `< 4.0 ms`**).
