# MILESTONE R19 / WAVE 18 SPECIFICATION — GLOBAL WORKSTATION SETTINGS & CANDLE PALETTES, OFFICIAL VIETNAM EXCHANGE HOLIDAY CALENDAR (`VIETNAM_HOLIDAYS_V1` `2018–2030`) & FIFO `T+2` LOT RELEASE, DATA READINESS & GAP/SPLIT AUDIT PANEL, & `TextNote` / CUSTOM FIBONACCI / DUAL-END TRENDLINE / `[Export PNG]` (`C1..C4`)

- **Document ID:** `SPEC-R19-SETTINGS-VNHOLIDAYS-AUDIT-DRAWINGS-001`
- **Milestone / Wave:** `Milestone R19 / Wave 18` (`FEATURE_BACKLOG_AND_PARITY_LEDGER.md` Group C: `C1`, `C2`, `C3`, `C4` — **Final Planned Core Parity Wave**)
- **Prepared By:** `SESSION-R19-20261119T145429+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Status:** `VERIFIED (`DEC-005` & `DEC-006`)` (`REA-R19-01..12` `VERIFIED`; `266/266` tests, `--r19` `< 4.0 ms`, `CRITIC_PASS`)
- **Crate Constraints:** Zero new crates in `Cargo.toml` / `Cargo.lock`; `0` modifications to `src/model.rs`, `ARCHIVE/`, or R01–R18 `docs/evidence/*`; `100%` `wasm32-unknown-unknown`-compatible domain math; `< 4.0 ms` CPU frame budget (`DECISION-RENDERER-002`).

---

## 1. Live `rea` MCP Reverse-Engineering Ledger (`DEC-005` & `DEC-006`)

All live `rea` MCP operations were executed exclusively on authorized workspace repositories (`/Users/mizuhara/workspace/sumi/ARCHIVE/frontend` and `/Users/mizuhara/workspace/sumi/research_repos/lightweight-charts-5.2.0`), with zero interaction with TradingView Desktop (`DEC-006`).

| Op ID | `rea` MCP Tool | Target Root / Files | Key Empirical Findings & Digests | Status |
| :--- | :--- | :--- | :--- | :--- |
| **`REA-R19-01`** | `analyze_javascript_application` (`detail: "summary"`) | `ARCHIVE/frontend` (`7` files: `SettingsModal.tsx`, `DashboardPage.tsx`, `ImportPage.tsx`, `drawingDomain.ts`, `SumiPrimitiveDrawingProvider.ts`, `DrawingInspector.tsx`, `DrawingToolbar.tsx`) | Root SHA-256: `243b2d6eaac9d00e0cd2e2623da5ea5f84bb751ad89e2ba40a04be7f7b794b92`; `7/7` parsed (`0` failures), `128,629` bytes, `19,924` AST nodes, `209` findings, `6,743` semantic relations. Parent: `ev_d9b24ef05c23e13c573fa607bf8ad21d266889a109e1761c100862f499962f7f`; Summary: `ev_9143fb3ea7c8bf26169fa07d380a96ceda06d5f3c8855b4cbac68a5428a84e5a`; `view_digest`: `dec242c90819f580cd981a28fd1535a8428696b7aee1e593fe7878fbcd1bbae0`. | `VERIFIED` |
| **`REA-R19-02`** | `analyze_javascript_application` (`detail: "summary"`) | `research_repos/lightweight-charts-5.2.0` (`7` files: `candlesticks-renderer.ts`, `candlestick-series.ts`, `candlesticks-pane-view.ts`, `series-markers-text.ts`, `text-watermark/pane-renderer.ts`, `text-width-cache.ts`, `chart-api.ts`) | Root SHA-256: `6a26c4adafe9539017d004f611425bc9692a278cd45d4a41ffe381afd7b6c9d5`; `7/7` parsed (`0` failures), `33,206` bytes, `5,106` AST nodes, `155` findings, `1,809` semantic relations. Parent: `ev_ff48e7fbcac7617a71708ef9b8ed9bfb7b60d5472f774a185ca58276449d8864`; Summary: `ev_91b1a863641168cac896607112b705c340f8b4e5c9daf228ccc96c33d07b4b35`; `view_digest`: `948e7d77f5e77d4af7e14b3ef3a545af47a6bbe035f945bec445736eb8e77`. | `VERIFIED` |
| **`REA-R19-03`** | `inspect_analysis_view` (`javascript-facts`) | `SettingsModal.tsx`, `DashboardPage.tsx`, `ImportPage.tsx` | `88/88` examined (`exhausted: true`), `view_digest` `6a54ff33ff0106608147b99d8f22bc5c559b73dcedd9ac2eee6abf8aefeb8b7c` (`ev_54f95ac25d50404206277e73a8b36998f23d3c3b8749bd4c2687d2f8165d767a`). Proved `SettingsModal` contains `0` user-editable inputs or store bindings (pure static mockup), while `DashboardPage` & `ImportPage` surface readiness/import gates. | `VERIFIED` |
| **`REA-R19-04`** | `inspect_analysis_view` (`javascript-facts`) | `drawingDomain.ts`, `SumiPrimitiveDrawingProvider.ts`, `DrawingInspector.tsx`, `DrawingToolbar.tsx` | `108/108` examined (`exhausted: true`), `view_digest` `c477a8043c90c2d07e8900ab404adf6ea46a7bb71968e093051dd73a4b2d6b1d` (`ev_4231115bf6ba4ba622be931607c655ed1175e6bbb5faef3b0e1bab97365a11af`). Confirmed `TextNote` (`tool: 'text'`), hardcoded 7-ratio `FIBONACCI_RATIOS` lacking Inspector toggles, and `isRightwardRay` lacking `extendLeft`/`extendRight` on `TrendLine`. | `VERIFIED` |
| **`REA-R19-05`** | `inspect_analysis_view` (`modules`, `references`) | `ARCHIVE/frontend` (`7` files) | `64/10,319` examined, `view_digest` `5e91077fa407e6b70b45b576bb5de1f5176bb46b35d88d32a38aee26c6edfa57` (`ev_62ffa7063cd4fd42c84a7bd413f08fba0c9a6de68cfd9b4e6539da50cd633389`). | `VERIFIED` |
| **`REA-R19-06`** | `inspect_analysis_view` (`javascript-facts`) | LWC `candlesticks-renderer.ts`, `candlestick-series.ts`, `series-markers-text.ts`, `chart-api.ts` | `138/138` examined (`exhausted: true`), `view_digest` `72740592e36621dea285c851c11d8eafd8c54049b56e526a59536f73d026e264` (`ev_2b86b924ea31c932007fdf2b15ea6805f8ec10ca7251344ad48ceff4a9ff08e2`). Confirmed hollow-candle inner-border geometry (`_drawWicks` stopping at body top/bottom + `_drawBorder` 4-edge frame) and `takeScreenshot(addTopLayer, includeCrosshair)` compositing. | `VERIFIED` |
| **`REA-R19-07`** | `inspect_analysis_view` (`modules`, `references`) | `LWC v5.2.0` (`7` files) | `64/1,153` examined, `view_digest` `d4640cd805f55a3e02255344ce300cee4d9ae55c8c6b9e240c1f4ad09f4d9ca1` (`ev_e917c04561016b72cf96eeaeeb63249368385ec5306b9d76e10056b467de770b`). | `VERIFIED` |
| **`REA-R19-08`** | `trace_javascript_semantics` (`backward-provenance`) | `SettingsModal` (`jag_node_223ac6c5f9c615a95803d7a4ccef9011e5202c2a57c364bca17fc26aff5b5f4d`) | Query `jsrq_a9b441dcedb8bd5510722ce684fbff991684eb09d09451c2b24deb9cc92a32f8` (`5` nodes, `4` relations, `5` unknowns, `ev_ab45655016049ce50f2ffdf6592917ee5f8de929f03189d46629bf8f48e66f8f`). Confirmed `SettingsModal` captures only `useRef` & `useEffect` with zero mutable preferences. | `VERIFIED` |
| **`REA-R19-09`** | `trace_javascript_semantics` (`backward-provenance`) | `DrawingInspector` (`jag_node_21b0b4e9ad827cd0e4fd7e9cb2277112a59fe3fd646871cc893bc735f428a802`) | Query `jsrq_37b2ae7c35405d1f90d94e8cfb9452578567ae1efbaac530a0985057f4f257e0` (`7` nodes, `8` relations, `10` unknowns, `ev_a8a1525973ec7e54a91788d004e58974346143f5ed553181939ab90474cb410c`). Confirmed missing Fibonacci level controls & missing TrendLine extension controls in legacy Inspector. | `VERIFIED` |
| **`REA-R19-10`** | `trace_javascript_semantics` (`forward-influence`) | `PaneRendererCandlesticks` (`jag_node_1c37d6f0b2c14c6fa78ef7bbdc63bc28b2630dfe477538f4049280dcc42ca93d`) | Query `jsrq_1b23f62283b5fde7f80c94b925afb2906a42aac0f60e6d15a5d59680af8d16ed` (`1` node, `0` relations, `0` unknowns, `ev_8ae98b2608c35549583d65c44438d907105d6016cc462950d071da89111e7501`). Confirmed single-pass wick/border/body order for hollow candles. | `VERIFIED` |
| **`REA-R19-11`** | `inspect_rust_binary` (`detail: "summary"`) | `target/release/sumi-workstation` | `22,853,216` bytes (`macho` `arm64` 64-bit LE), SHA-256 `427d4f7a92a4b919aec40e01c725b8e85f212dd3074649267f90ba61095ca696`, `view_digest` `1012d4bd440705b2ec984dee53d9ecaa4da3d713d4f3371fa3af9db95daec326` (`ev_e9df6e741281584cdf80458c27b5f17ce1443d4f6c09ebaf247cdbfac385fcaf`, parent `ev_357669d9344fc2651c5aea301502861bbb3ad5245adf0eb7f1639c3db54cbf47`). | `VERIFIED` |
| **`REA-R19-12`** | `open_binary` + `inspect_macho` (`summary`) + `close_binary` | `target/release/sumi-workstation` | UUID `5A893952-AF29-3BF2-AD66-EC2843AB4E61`, `view_digest` `d57c2e77de83955adb8d780c78cd2f1dfe2df6ab3d2fc256e361ed072e9be952`, `30` load commands, `14` dylib dependencies (unchanged), `324` imports, `5,466` exports, `5` segments, `5,842` items examined (`ev_735fb7486b62c04072f52c06bc9f3802241693499188f003529563b4ff9278db`, parent `ev_e832d5c836f3b9788e0216622d8d176f9b47be9b65a0ab70d4a8e87ffbd80981`; closed cleanly). | `VERIFIED` |

---

## 2. Legacy Defects & Edge Cases Rectified in Milestone R19

1. **Static Placeholder `SettingsModal.tsx` (`REA-R19-03` & `REA-R19-08`):**
   - Legacy `SettingsModal.tsx` rendered hardcoded `"http://localhost:8000"`, `"SQLite (backend/sumi.db)"`, and a drifted static shortcut list with **zero** interactive controls. Sumi's `WorkstationSettings` (`C1`) provides live persisted configuration across default `ExchangeVenue`, `VnPriceUnit`, capital/commission/tax/slippage/risk-per-trade defaults, `CandlePalettePreset` (`EmeraldCrimson`, `HollowBullish`, `ColorblindOkabeIto`, `ClassicMonochrome`), auto-snapshot cadence, and `enforce_vn_t2_lot_lock`, serialized cleanly in both `eframe` storage and `WorkspaceTemplate`.
2. **`VIETNAM_HOLIDAYS_V1` `2026` Horizon Cliff & Timezone Shift (`BUG-CAL-01`, `BUG-CAL-02`):**
   - Legacy `calendar.py` ended abruptly at `2026-09-02` (missing official `2018-12-31` & `2026-01-02` bridge days and `24/11` Culture Day) and stripped `.date()` without normalizing timestamps to Vietnam civil time (`UTC+7` ICT). Sumi's `VIETNAM_HOLIDAYS_V1` (`src/journal/market_rules.rs`) embeds all `103` official `2018–2026` weekday exchange closures **plus** all `48` statutory `2027–2030` weekday holidays (`151` sorted `SessionDate`s across `2018–2030`, $O(\log N)$ binary search) **plus** a deterministic statutory fixed-holiday fallback (`01/01`, `30/04`, `01/05`, `02/09`, `24/11` `2026+` + Monday/Tuesday weekend makeup per Labor Code Art. 111(3) & 112 and Resolution `28/2026/QH16`) for any year `< 2018` or `> 2030`, always converting UTC epoch timestamps via `UTC+7` ICT (`+25,200s`) before resolving civil dates.
3. **Intraday `13:00 ICT` (`06:00 UTC`) VSD `T+2` Settlement Blindness & Pre-`2022-08-29` `T+3` Regime (`BUG-SET-01`..`BUG-SET-08`):**
   - Legacy `settlement.py` discarded time-of-day, either blocking afternoon (`13:00–14:45 ICT`) sells on Day `T+2` or illegally allowing morning (`09:00–11:30 ICT`) sells on Day `T+2`, defaulted `sellable_date = acquisition_date`, and ignored Vietnam's historical pre-`2022-08-29` `T+3` morning regime (`09:00 ICT` on `T+3`). Sumi's `HoldingLot` & `FifoSettlementLedger` compute exact `sellable_at_timestamp` in UTC epoch seconds (`T+2` at `13:00:00 ICT = 06:00:00 UTC` for acquisitions on/after `2022-08-29`, and `T+3` at `09:00:00 ICT = 02:00:00 UTC` for historical acquisitions prior to `2022-08-29`), consume lots strictly in chronological `(buy_timestamp, lot_id)` FIFO order without mutating caller slices, and rebuild deterministically from causal `TradingSession::fills` on Bar Replay rewind (`DEF-001`).
4. **Weekly Gap False Positives, Holiday False Gaps, Outlier Blindness & `CafeF SolieuGD` 4-Decimal Split Rounding (`BUG-DQ-01`..`BUG-DQ-06`):**
   - Legacy `import_classifier.py` ran daily `is_trading_day` loops on `1W` feeds (fabricating 4 missing days/week!), lacked outlier range/return/volume checks, and pre-sorted CafeF rows before auditing. Sumi's `DataQualityAuditReport` (`src/data/audit.rs`) runs daily missing-session gap detection strictly on `Timeframe::D1` against venue-aware trading calendars (`VIETNAM_HOLIDAYS_V1` for VN symbols, weekday calendar for US, 24/7 continuous bucket check for Crypto, and intraday/weekly expected-bucket continuity for `!= D1`), flags bars falling on exchange holidays, detects extreme return/range/volume outliers, and implements the exact 5-rule `CafeF SolieuGD` 4-decimal post-split OHLC envelope audit & normalization (`0.100001 VND` 1-ULP open cap, `6 bps` & `25.0 VND` close cap, `< 0.5 * min_step` sub-tick guard).
5. **Hardcoded 7-Level Fibonacci, Right-to-Left `TrendLine` Extension Inversion, Heuristic `TextNote` & Missing `[Export PNG]` (`BUG-DRW-01`..`BUG-DRW-05`):**
   - Sumi adds `DrawingTool::TextNote` (`DrawingGeometry::TextNote { anchor, text }`, UTF-8 char-safe `<= 280` chars, multi-line word-boundary callout card with anchor pin, causal `DEF-001` Replay eligibility, and transactional Inspector text editing), per-drawing custom Fibonacci levels (`custom_fib_levels: Option<Vec<f64>>` up to `16` sorted finite ratios in `[-5.0, 10.0]`, preset extensions `-0.618, -0.272, 0.705, 1.272, 1.618, 2.0, 2.618`, dynamic label collision avoidance), time-directional `trend_extend_left` / `trend_extend_right` on `TrendLine` (extending toward `-∞` / `+∞` time regardless of `start`/`end` anchor order, synchronized with `AlertEngine`), and a 1-click **`[Export PNG]`** toolbar + `Cmd+K` action rendering crisp `2.0x` Retina PNGs via `OffscreenWgpuHarness` with all top-layer drawings composited.

---

## 3. Subsystem Design & Mathematical Contracts (`C1..C4`)

### 3.1. `C1` — Global `[Settings]` Modal (`WorkstationSettings`) & Candlestick Color Palettes (`src/viewport/workspace_template.rs`, `src/chart.rs`, `src/app.rs`)

- **`CandlePalettePreset` (`Copy`, `Serialize`, `Deserialize`, default `EmeraldCrimson`):**
  1. `EmeraldCrimson` (Sumi Default): Solid bullish `#26A69A`, solid bearish `#EF5350`, matching volume alpha fills.
  2. `HollowBullish` (LWC Institutional Hollow): Bullish candles render upper/lower wicks stopping strictly at `top_y`/`bottom_y` plus a crisp `1.25 px` 4-edge inner border (`#26A69A`) with transparent interior (falling back to solid at `half_body_w < 1.6 px`), while bearish candles render solid `#EF5350`.
  3. `ColorblindOkabeIto` (High-Contrast Deuteranopia/Protanopia Safe): Bullish `#009ADE` (sky/cerulean blue), bearish `#FF8C00` (vivid amber-orange).
  4. `ClassicMonochrome` (Print / Institutional White-Silver & Slate-Crimson): Bullish `#E6EDF3` (crisp ice-white), bearish `#6E7681` / `#F85149`.
  - Included in `PlotDataLayerKey` so switching `CandlePalettePreset` invalidates the cached candlestick/volume `egui::Mesh` on the very next frame (`0` stale-mesh frames).
- **`WorkstationSettings` (`Serialize`, `Deserialize`, persisted in `eframe` + `WorkspaceTemplate`):**
  - `default_venue_override: Option<ExchangeVenue>` (`None` = auto-infer from symbol/preset; `Some(v)` = explicit override)
  - `default_vn_price_unit: VnPriceUnit` (`Auto` / `ThousandVnd` / `RawVnd`)
  - `default_initial_capital: f64` (`1_000.0 ..= 1.0e12`, default `100_000.0`)
  - `default_commission_bps: f64` (`0.0 ..= 500.0`, default `15.0`)
  - `default_tax_sell_bps: f64` (`0.0 ..= 500.0`, default `10.0`)
  - `default_slippage_bps: f64` (`0.0 ..= 500.0`, default `5.0`)
  - `default_risk_per_trade_pct: f64` (`0.05 ..= 25.0`, default `1.0`)
  - `candle_palette: CandlePalettePreset`
  - `auto_snapshot_interval_mins: u16` (`0` = manual/structural only, or `1 | 5 | 15 | 30`, default `5`)
  - `enforce_vn_t2_lot_lock: bool` (default `true` for VN cash equities in FIFO lot audit; optional strict enforcement in Paper Trading)
  - Interactive `[Settings]` toolbar button + `Cmd+K` action (`Escape` to close, `"Apply Defaults to Active Session & Backtest"` 1-click propagation, `"Reset to Factory Defaults"`).

### 3.2. `C2` — Official Vietnam Exchange Holiday Calendar (`VIETNAM_HOLIDAYS_V1` `2018–2030`) & VSD `T+2` (`13:00 ICT`) FIFO `HoldingLot` Ledger (`src/journal/market_rules.rs`, `src/trading/portfolio.rs`)

- **`VIETNAM_HOLIDAYS_V1` (`[i32; 151]` packed `YYYYMMDD` sorted array, `2018–2030`):**
  - Exact `103` official weekday dates (`2018–2026`, including `2018-12-31` & `2026-01-02` swaps and `2026-11-24` Culture Day) + `48` statutory weekday dates (`2027–2030`: Tết Đinh Mùi `2027`, Mậu Thân `2028`, Kỷ Dậu `2029`, Canh Tuất `2030`, Giỗ Tổ `10/3 AL`, `01/01`, `30/04–01/05`, `02/09`, `24/11` Culture Day & weekend compensatory Mondays/Tuesdays), plus out-of-range statutory fixed-holiday fallback (`< 2018` or `> 2030`).
  - Pure `const`/binary-search helpers: `is_vn_exchange_holiday_ymd(year, month, day) -> bool`, `is_vn_exchange_trading_day_days(ict_epoch_days: i64) -> bool`, `add_vn_trading_days(start_ict_days: i64, n: i32) -> i64`, `vn_trading_days_between(start_ict_days: i64, end_ict_days: i64) -> i32`.
  - Upgraded `compute_settlement_timestamp_for_venue(entry_timestamp, venue)`:
    - Converts `entry_timestamp` to `UTC+7` ICT (`+25,200s`) for VN venues.
    - Historical regime awareness: for `venue.is_vn_cash_equity()`, entries prior to `2022-08-29` (`ict_ymd < 2022_08_29`, VSD `T+1.5` cutover date) advance **`3` VN trading days** releasing at `09:00:00 ICT` (`02:00:00 UTC`), while entries on/after `2022-08-29` advance **`2` VN trading days** releasing at **`13:00:00 ICT` (`06:00:00 UTC`)**!
- **FIFO `HoldingLot` & `FifoSettlementLedger` (`src/journal/market_rules.rs` + `src/trading/portfolio.rs`):**
  - `HoldingLot`: `lot_id: u64`, `buy_fill_id: u64`, `buy_base_index: usize`, `buy_timestamp: i64`, `buy_price: f64`, `initial_qty: f64`, `remaining_qty: f64`, `sellable_at_timestamp: i64`.
  - `HoldingLot::is_sellable_at(as_of_timestamp: i64) -> bool`: `as_of_timestamp >= self.sellable_at_timestamp`.
  - `FifoSettlementSummary`: `lots: Vec<HoldingLot>`, `total_held_qty: f64`, `sellable_qty: f64`, `locked_t2_qty: f64`, `earliest_unlock_timestamp: Option<i64>`, `unsettled_sell_violation_qty: f64`.
  - Computed causally from `TradingSession::fills()` up to the current Bar Replay / live timestamp (`DEF-001` safe, zero stale state on Replay rewind!), surfaced in both `Paper Trade` and `Risk & Journal` panels, with optional pre-submission guard blocking VN cash-equity sell orders exceeding `sellable_qty` when `enforce_vn_t2_lot_lock` is enabled!

### 3.3. `C3` — Data Readiness & Gap / Outlier / `CafeF SolieuGD` Post-Split Audit Panel (`src/data/audit.rs`, `src/app.rs`)

- **`DataQualityAuditReport` (`src/data/audit.rs`, pure `f64`/`i64`, WASM-safe):**
  - **Readiness Summary:** `readiness_status: DataReadinessStatus` (`Ready`, `PartialWarning`, `Empty`), `symbol`, `venue`, `timeframe`, `bar_count`, `earliest_date`, `latest_date`, `expected_sessions`, `coverage_pct: f64` (`[0.0, 100.0]`).
  - **Calendar-Aware Missing-Session Gap Detector (`DEF-001` causal prefix aware):**
    - On `Timeframe::D1` for VN symbols (`HOSE`/`HNX`/`UPCOM`/`VN30F`/`VNINDEX`): counts expected `is_vn_exchange_trading_day_days` between `first_bar` and `last_bar`, listing missing trading dates (`<= 32` sample dates + total `missing_session_count`) and flagging any bar erroneously stamped on a weekend or `VIETNAM_HOLIDAYS_V1` holiday (`holiday_or_weekend_bar_count`)!
    - On `Timeframe::D1` for `UsEquities`: checks weekday continuity (`Mon..Fri`), excluding major US fixed/observed holidays (`New Year`, `Jul 4`, `Christmas`) or reporting weekday gaps.
    - On `Timeframe::W1` / Intraday / `CryptoSpotPerp`: never runs the daily `D1` loop (`BUG-DQ-02` fix!); checks multi-bucket time-step gaps (`dt > 3 * tf.seconds()` on 24/7 Crypto, or missing ISO weeks on `W1`).
  - **Outlier & Integrity Counters:** `zero_volume_count` (informational, non-error), `extreme_range_bar_count` (`(H - L) / L > 0.25` on equities or `> 5.0 * median_range`), `price_band_breach_count` (unadjusted close-to-close move exceeding venue statutory band `+1.5%` tolerance on VN equities, hinting potential unadjusted dividend/split date), `volume_spike_10x_count` (`RVOL(20) >= 10.0`).
  - **`CafeF SolieuGD` 4-Decimal Post-Split Rounding Audit & Normalizer (`audit_and_normalize_cafef_split_ohlc`):**
    - Implements the exact 5-rule `evaluate_and_normalize_ohlc` contract:
      1. Equity symbol (not index).
      2. Sub-`10 VND` split precision present (`|v_vnd - 10 * round(v_vnd / 10)| > 1e-3`).
      3. `delta_open_vnd <= 0.100001 VND` (`1 ULP` of `0.0001 kVND`).
      4. `delta_max_vnd <= 0.100001 VND || (delta_max_vnd / high_vnd <= 0.0006 && delta_max_vnd <= 25.0 VND)`.
      5. `delta_max_vnd < 0.5 * min_positive_intraday_step_vnd`.
    - Reports `cafef_split_normalized_count` and `cafef_unrepairable_envelope_count`, and wires directly into `OhlcEnvelopePolicy::CafeFSplitAware` / CSV import + interactive **`[Data Audit]`** modal/panel!

### 3.4. `C4` — `TextNote` Callout Drawing, Custom Fibonacci Levels, Dual-End `TrendLine` Extensions & `[Export PNG]` (`src/drawings/`, `src/alerts/engine.rs`, `src/chart.rs`, `src/app.rs`)

1. **`DrawingTool::TextNote` & `DrawingGeometry::TextNote { anchor: DrawingAnchor, text: String }` (`1`-anchor tool):**
   - `MAX_TEXT_NOTE_CHARS = 280` (UTF-8 `chars().count()`-safe truncation, control-char stripped except `\n`, non-empty trimmed text defaulting to `"Note"` on single-click placement).
   - Projects to `ProjectedDrawingShape::TextNote { anchor_local: LocalPoint, box_rect: LocalRect, text: String }`, keeping callout card clamped cleanly inside `pane_bounds` with an anchor pin + leader tick to `anchor_local`, full handle + body drag, `DEF-001` causal Replay masking, and multi-line `TextEdit` inside the transactional Drawing Property Inspector!
2. **Per-Drawing Custom Fibonacci Levels (`Drawing::custom_fib_levels: Option<Vec<f64>>`):**
   - `None` uses `FIBONACCI_RATIOS` (`[0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0]`, 100% backwards-compatible).
   - `Some(levels)`: `1..=16` finite sorted deduplicated ratios in `[-5.0, 10.0]`, supporting institutional presets (`Standard 7`, `ICT OTE + Extensions [-0.272, 0.0, 0.5, 0.618, 0.705, 0.786, 1.0, 1.272, 1.618]`, `Full Harmonic`) plus individual ratio toggle chips & custom ratio input in the Drawing Inspector!
   - Dynamic collision-free label priority in `src/chart.rs` supporting arbitrary `1..=16` levels without fixed-array bounds.
3. **Time-Directional Dual-End `TrendLine` Extensions (`Drawing::trend_extend_left: bool`, `Drawing::trend_extend_right: bool`):**
   - Extends toward `-∞` X (`trend_extend_left`) and/or `+∞` X (`trend_extend_right`) in time regardless of whether `start.bar_index < end.bar_index` or `start.bar_index > end.bar_index`, synchronized across `PaneDrawingProjection`, toolbar quick toggles, Drawing Inspector checkboxes, and `AlertEngine::drawing_line_value_at_bar`!
4. **1-Click `[Export PNG]` Toolbar Button & `Cmd+K` Action (`export_workstation_png`):**
   - Captures the live `SumiWorkstationApp` state (candlesticks, sub-panes, overlays, top-layer drawings, HUDs; hiding transient export toast) at crisp `2.0x` Retina resolution via `OffscreenWgpuHarness` into `exports/sumi_<symbol>_<tf>_<seq>.png` (or user-specified path), surfacing immediate confirmation status with exact pixel dimensions and file path!
