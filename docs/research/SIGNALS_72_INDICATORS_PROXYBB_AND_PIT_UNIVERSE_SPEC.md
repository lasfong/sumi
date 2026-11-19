# MILESTONE R18 / WAVE 17 SPECIFICATION — COMPLETE `101`-SIGNAL ECOSYSTEM, `8` EXTENDED INDICATORS, MULTI-HORIZON MONEY FLOW `ProxyBB` (`T03..T200`) & `POINT-IN-TIME` UNIVERSE (`B1..B4`)

- **Document ID:** `SPEC-R18-SIGNALS-INDICATORS-PROXYBB-PIT-001`
- **Milestone / Wave:** `Milestone R18 / Wave 17` (`FEATURE_BACKLOG_AND_PARITY_LEDGER.md` Group B: `B1`, `B2`, `B3`, `B4`)
- **Prepared By:** `SESSION-R18-20261119T085800+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Status:** `VERIFIED RESEARCH & POST-BUILD BINARY AUDIT (DEC-005 & DEC-006)` (`REA-R18-01..12` `VERIFIED`, `CRITIC_PASS`)
- **Crate Constraints:** Zero new crates in `Cargo.toml` / `Cargo.lock`; `0` modifications to `src/model.rs`, `ARCHIVE/`, or R01–R17 `docs/evidence/*`; `100%` `wasm32-unknown-unknown`-compatible domain math; `< 4.0 ms` CPU frame budget (`DECISION-RENDERER-002`).

---

## 1. Live `rea` MCP Reverse-Engineering Ledger (`DEC-005` & `DEC-006`)

All live `rea` MCP operations were executed exclusively on authorized workspace repositories (`/Users/mizuhara/workspace/sumi/ARCHIVE/frontend` and `/Users/mizuhara/workspace/sumi/research_repos/lightweight-charts-5.2.0`), with zero interaction with TradingView Desktop (`DEC-006`).

| Op ID | `rea` MCP Tool | Target Root / Files | Key Empirical Findings & Digests | Status |
| :--- | :--- | :--- | :--- | :--- |
| **`REA-R18-01`** | `analyze_javascript_application` (`detail: "summary"`) | `ARCHIVE/frontend` (`8` files: `TechnicalFlowBBViewer.tsx`, `IndicatorSelector.tsx`, `IndicatorManager.tsx`, `IndicatorRenderRegistry.ts`, `SignalCatalog.tsx`, `SignalInspector.tsx`, `SignalExplanationInspector.tsx`, `StrategyLabPage.tsx`) | Root SHA-256: `243b2d6eaac9d00e0cd2e2623da5ea5f84bb751ad89e2ba40a04be7f7b794b92`; `8/8` parsed (`0` failures), `199,622` bytes, `25,593` AST nodes, `301` findings (`761` `closure`, `583` `call-flow`, `7,531` `data-flow`, `906` `object-flow`, `36` `boundary`, `14` `promise-ownership`). Parent: `ev_0fffca836a4b37fcac7a7e1080c958b600947a0ff6acdb5b27f536ff80e4a5c3`; Summary: `ev_e9c8a51b3c3962368a256dca6dbfd7aafa209cec35cfd08c78bd42e31ca5d19d`; `view_digest`: `83147d70cd15ac713dd2c62f3ae3d579192c1d43e626e147cc7da93a070b03fc`. | `VERIFIED` |
| **`REA-R18-02`** | `analyze_javascript_application` (`detail: "summary"`) | `research_repos/lightweight-charts-5.2.0` (`7` files: `histogram-series.ts`, `histogram-pane-view.ts`, `histogram-renderer.ts`, `draw-series-point-markers.ts`, `marks-renderer.ts`, `series-bar-colorer.ts`, `candlesticks-pane-view.ts`) | Root SHA-256: `6a26c4adafe9539017d004f611425bc9692a278cd45d4a41ffe381afd7b6c9d5`; `7/7` parsed (`0` failures), `20,521` bytes, `3,019` AST nodes, `116` findings (`38` `closure`, `6` `call-flow`, `839` `data-flow`, `129` `object-flow`). Parent: `ev_1fe0c86ffd298b06d5f7a0d9b70242c0f1aa1e09cf1a4271963d359c64db9b05`; Summary: `ev_8abf26e7be86db9701da4509ed501da70d36585b4caa3f35f7ea41301c66d613`; `view_digest`: `625772b7739f69fc956af7b5dc670c737a674102fc52ad85ef8dccb3fc705bb6`. | `VERIFIED` |
| **`REA-R18-03`** | `inspect_analysis_view` (`javascript-facts`) | `TechnicalFlowBBViewer.tsx`, `IndicatorSelector.tsx`, `IndicatorRenderRegistry.ts` | `52/52` examined (`exhausted: true`), `view_digest` `7b8cb9741727d8fbbb2cb721c79827862912f5de2ef75a583cbc2fe70cf67f13` (`ev_f0704206438ecb7b8c6e45769f428abc6ef27c7f5dccad008d7424829bee75a6`). Confirmed `TechnicalFlowBBViewer` renders multi-horizon `T03..T200` `bb_value` (`[0, 100]`, equilibrium `50.0`), `direction`, `regime`, `regime_run_length`, and `OHLCV_PROXY` badge, while `IndicatorRenderRegistry` lacked `ADX`, `Stoch`, `SuperTrend`, `PSAR`, `Keltner`, `CCI`, `MFI`, and `Volume SMA` renderers despite backend `INDICATOR_REGISTRY` declaring them. | `VERIFIED` |
| **`REA-R18-04`** | `inspect_analysis_view` (`javascript-facts`) | `SignalCatalog.tsx`, `SignalInspector.tsx`, `SignalExplanationInspector.tsx` | `64/65` examined, `view_digest` `48ea3bf40d4a32f6aa0bffeefe6f89923dea3e617b5e2dbd5f4f7106b0ecc6ec` (`ev_df0aa9ca79a30875a7975d6ca5c06017bdb22838cb537d75d0da2048c0fa7f66`). Confirmed 12-tab `SignalCategory` filter, `causal_delay_bars` badge (`+3B` confirmation lag disclosure on pivot/divergence signals), and quality-gated `SignalEvaluation` inspector. | `VERIFIED` |
| **`REA-R18-05`** | `inspect_analysis_view` (`modules`, `references`) | `ARCHIVE/frontend` (`8` files) | `64/10,414` examined, `view_digest` `055e1f0b3f7476cee3b362ad0cd1ab7deedca91feff1b06787ff256ed9cc4bc4` (`ev_353cdfebafd0e16992d050f7abaaf400fff00327d39cea4d898ac4c05129c42c`). Confirmed HTTP fetch bottlenecks in `TechnicalFlowBBViewer` and `SignalInspector` eliminated by Sumi's in-memory pure-Rust evaluation. | `VERIFIED` |
| **`REA-R18-06`** | `inspect_analysis_view` (`javascript-facts`) | LWC `histogram-series.ts`, `histogram-pane-view.ts`, `histogram-renderer.ts`, `draw-series-point-markers.ts` | `64/67` examined, `view_digest` `bf199830ea8f98620469d40870ece5ba1ee3812c23dfd8dad95b080169ce59f9` (`ev_0902e0060ebe95100dfff22aeafe7dabc0afae967e3198c6123fd0e2fa6cda88`). Confirmed crisp pixel-snapped histogram baseline & point-marker rendering geometry for `Parabolic SAR` dots and candlestick pattern badges. | `VERIFIED` |
| **`REA-R18-07`** | `inspect_analysis_view` (`modules`, `references`) | `LWC v5.2.0` (`7` files) | `64/1,120` examined, `view_digest` `7f9d27b40d55ac65f34730f209f07b4cb28c8e0a148fedf5124b540f83c36d72` (`ev_e45728b99bdc4cd2bc6b3fa04fcc9e903a5f15ecacc3bd3c1876c5840905faa5`). | `VERIFIED` |
| **`REA-R18-08`** | `trace_javascript_semantics` (`backward-provenance`) | `TechnicalFlowBBViewer` (`jag_node_538a9992905c97156758c6710064a91777514a592cafa92badc1a00fc20e707d`) | Query `jsrq_3a10530ce2d3e16a2486c2d64b33b195b2bbc4a2f4acefc775ba4acf6e676454` (`17` nodes, `24` relations, `37` unknowns, `ev_643cbdadbf1ecd862fb8f1616c20147a38fac9614b54d024810cc1dfc13c4417`). | `VERIFIED` |
| **`REA-R18-09`** | `trace_javascript_semantics` (`backward-provenance`) | `IndicatorSelector` (`jag_node_4bc8e74ba5cd683ee3e8cb8e2d4cb5726b0adbf9ac24d7492ce1680a48ef60fc`) | Query `jsrq_00cffd13f6a03b6beeb0232621492aafd94ac6bd5eb73ab425a1a62b9796e4c5` (`7` nodes, `6` relations, `8` unknowns, `ev_1858424c64ba70caee297079a08adff41e43aabdb60000e9d87ab54d08f0edf6`). | `VERIFIED` |
| **`REA-R18-10`** | `trace_javascript_semantics` (`forward-influence`) | `PaneRendererHistogram` (`jag_node_a4940a5a7d8ed5fcdcd20f81af1a522b3703462e1633c18e7beee06ee4b4e8d9`) | Query `jsrq_d8465af411488bb75c19b84779b9cbcaf0386749ae1c7865b8de88371057bf3e` (`1` node, `0` relations, `0` unknowns, `ev_9fdd2ab8df49bd25dabc0d25ee4a7eadabc3c42717bef57a8d9cc5c747ba27bf`). | `VERIFIED` |
| **`REA-R18-11`** | `inspect_rust_binary` (`detail: "summary"`) | `target/release/sumi-workstation` | `22,553,008` bytes (`macho` `arm64` 64-bit LE), SHA-256 `6fef1e5140f285f53c39501d10e1330a858658deacf0db91d2a79c926d273ced`, `view_digest` `27ead6bb27879db9f5e595a3be6e8a48312b2779a9392e9adf6d59d266b69cbb`, `auditable_metadata_available: false`, `1` inferred `rustc` commit (`ev_0550f094bd0e3f6d2dc4d937bb7a32008026dc91fe11b45d69894466c912590f`; parent `ev_43964e3c4e4d1c22f3e0c007b3968543a13e4a80dfe234b2c24ffda72ad33afd`). | `VERIFIED` |
| **`REA-R18-12`** | `open_binary` + `inspect_macho` (`summary`) + `close_binary` | `target/release/sumi-workstation` | UUID `E2216528-5E0F-3223-ABFA-350E8000E9F7`, `view_digest` `92b0047ee13ad421f4d21f1bfd840657f1d7bfd8ce708b4bcca2f03c81bf0cd0`, `30` load commands, `14` dylib dependencies (unchanged), `324` imports, `5,431` exports, `5` segments, `5,807` examined (`ev_c8661ba1fd3499e18294b0b8e2182c9c711aad672c617d71e317475e8f534d13`; parent `ev_1664783f5299e82dd24c53db607ceb31a1f7802a649efba5d69f1328c829ade9`; session closed cleanly). | `VERIFIED` |

---

## 2. Legacy Defects & Edge Cases Rectified in Milestone R18

1. **`available_at_index` Double-Delay Fix (`src/scanner/catalog.rs` vs `divergence.py`):**
   - Previously, `SignalEvaluation` computed `available_at_index: t.saturating_add(entry.causal_delay_bars)`. Because `causal_delay_bars = 0` for all R15 entries, this was benign. For `divergence.*` (`causal_delay_bars = 3`), candidate pivot `p = t - 3` is **already evaluated at confirmation bar `t = p + 3`**! Adding `+3` again would erroneously backdate/forward-shift availability to `t + 3`. In R18, `available_at_index` is strictly `t` (`available_at_timestamp = candles[t].timestamp`), while `pivot_bar_index = p` (`t - right_bars`) is recorded explicitly in `reasons` and `factors`.
2. **Legacy `_compute_cci` Zero-Division & Negative `offset` Lookahead (`indicator_engine.py:36-38, 393-407`):**
   - Legacy `_compute_cci` divided `(tp - mean) / (0.015 * mean_deviation)` without a zero guard (`NaN`/`Inf` on flat-price windows) and exposed an unrestricted signed `offset` parameter (`result.shift(-k)` future lookahead). Sumi's `compute_cci` enforces TA-Lib's scale-relative guard (`(tp - sma_tp).abs() <= 1e-14 * sma_tp.abs().max(1e-30) || mean_dev <= 1e-14 * sma_tp.abs().max(1e-30) => 0.0`) and forbids negative lookahead shifts.
3. **Flat / Zero-Volume `MFI(14)` Neutrality vs False Oversold (`ta_MFI.c` vs `RSI(14)`):**
   - When both rolling positive and negative money flow sums over `14` bars equal `0.0` (e.g., halted/zero-volume bars), `compute_mfi` emits neutral `50.0` (matching Sumi's `RSI(14)` convention rather than `0.0`, preventing false `< 20` oversold signals on zero-liquidity sessions); when `neg_mf == 0 && pos_mf > 0` -> `100.0`; when `pos_mf == 0 && neg_mf > 0` -> `0.0`.
4. **Scale-Relative `CandleGeometry` & `TA_ADX` Guards (`candle_features.py:44`, `patterns.py:168`, `ta_ADX.c:378`):**
   - Legacy `candle_features.py` hardcoded `c_range <= 1e-8` and `max(geom.body, 1e-6)`, misclassifying micro-priced crypto instruments (`< 1e-6`). Sumi's `CandleGeometry::from_candle` uses scale-relative `price_scale_eps = 1e-12 * (high.abs() + low.abs()).max(1e-30)` for flat-range detection and `body_floor = 1e-6 * c_range` for pinbar wick-to-body ratios, and `compute_adx` tests `prev_tr > 0.0` exactly per TA-Lib issue `#253`.
5. **Symmetric EMA Support/Resistance Self-Contradiction (`support_resistance.py:65-76, 129-140`):**
   - Legacy `calculate_near_support` and `calculate_near_resistance` both checked `|close - ema| <= 0.5 * atr`, causing exact EMA touches to trigger `near_support` and `near_resistance` simultaneously. Sumi qualifies EMA support touches with `close >= ema` (price testing EMA from above) and EMA resistance touches with `close <= ema` (price testing EMA from below), alongside causal prior-window `[t-20..t)` rolling low/high proximity.
6. **`MACD` Divergence Oscillator Warmup (`divergence.py:238-241`):**
   - `MACD(12, 26, 9)` histogram is `None` until index `33`, so `divergence.macd_*` enforces `warmup_bars = 36` (`33 + right_bars`) and pairs consecutive confirmed pivots `(p1, p2)` (`5 <= p2 - p1 <= 60`) strictly when both `osc[p1]` and `osc[p2]` are `Some(finite)`.
7. **`AT05` Zero-Liquidity `None` & `PointInTime` Pre-Effective Cash Guard (`calculator.py:189`, `resolver.py:76`):**
   - When rolling `H`-bar trading value `den_sum <= 0.0`, `ProxyBBCalculator` emits `bb_value = None` (`AT05`, never `50.0`), `bb.*` catalog signals return `SignalQuality::ZeroBaseline` (`value = None`), and `evaluate_causal_confluence` preserves `bb_flow_absent_neutral = true` (`"bb_absent_neutral"`). When `ProxyBB(T20)` is valid (`den_sum > 0`), Family 4 (`Participation`) upgrades `bb_flow_absent_neutral = false` and blends `RVOL(20) * CLV` with `ProxyBB(T20)`.
   - In `run_portfolio_backtest_with_universe` under `UniverseMode::PointInTime`, sessions prior to `universe.effective_from` (or with zero active PIT members) hold **100% cash (`0.0` return)** rather than falling back to future constituent lists.

---

## 3. Subsystem Design & Mathematical Contracts (`B1..B4`)

### 3.1. `B1` — `CandleGeometry`, `14` Causal Candlestick Patterns, `structure.candle_score` & Chart Badges (`src/indicators/patterns.rs`, `src/scanner/catalog.rs`, `src/chart.rs`)

- **`CandleGeometry` (`Copy`, pure `f64`):**
  - `range = (high - low).max(0.0)`, `body = (close - open).abs()`, `upper_wick = (high - open.max(close)).max(0.0)`, `lower_wick = (open.min(close) - low).max(0.0)`, `midpoint = 0.5 * (open + close)`.
  - `is_flat_range = range <= 1e-12 * (high.abs() + low.abs()).max(1e-30)`.
  - When `is_flat_range`: `body_ratio = 0.0`, `upper_wick_ratio = 0.0`, `lower_wick_ratio = 0.0`, `close_location = 0.5`. Otherwise clamped to `[0.0, 1.0]`.
  - `is_bullish = close > open`, `is_bearish = close < open`, `is_flat = close == open`.
- **`14` Causal Candlestick Patterns (`category = SignalCategory::Pattern`, `causal_delay_bars = 0`, `SIGNAL_AVAILABILITY_EVENT_BAR_CLOSE`):**
  1. `pattern.bullish_engulfing` (`warmup = 1`): `g[t-1].is_bearish && g[t].is_bullish && open[t] <= close[t-1] && close[t] >= open[t-1] && g[t].body_ratio >= 0.55 && g[t].close_location >= 0.70`.
  2. `pattern.bearish_engulfing` (`warmup = 1`): `g[t-1].is_bullish && g[t].is_bearish && open[t] >= close[t-1] && close[t] <= open[t-1] && g[t].body_ratio >= 0.55 && g[t].close_location <= 0.30`.
  3. `pattern.hammer` (`warmup = 0`): `!g[t].is_flat_range && g[t].lower_wick >= 2.0 * g[t].body.max(1e-6 * g[t].range) && g[t].upper_wick_ratio <= 0.25 && g[t].close_location >= 0.65 && g[t].body_ratio <= 0.50`.
  4. `pattern.shooting_star` (`warmup = 0`): `!g[t].is_flat_range && g[t].upper_wick >= 2.0 * g[t].body.max(1e-6 * g[t].range) && g[t].lower_wick_ratio <= 0.25 && g[t].close_location <= 0.35 && g[t].body_ratio <= 0.50`.
  5. `pattern.morning_star` (`warmup = 2`): `g[t-2].is_bearish && g[t-2].body_ratio >= 0.55 && g[t-1].body <= 0.40 * g[t-2].body.max(1e-12) && g[t].is_bullish && close[t] >= g[t-2].midpoint && g[t].close_location >= 0.65`.
  6. `pattern.evening_star` (`warmup = 2`): `g[t-2].is_bullish && g[t-2].body_ratio >= 0.55 && g[t-1].body <= 0.40 * g[t-2].body.max(1e-12) && g[t].is_bearish && close[t] <= g[t-2].midpoint && g[t].close_location <= 0.35`.
  7. `pattern.piercing_line` (`warmup = 1`): `g[t-1].is_bearish && g[t].is_bullish && open[t] <= close[t-1] && close[t] >= g[t-1].midpoint && close[t] < open[t-1] && g[t].close_location >= 0.60`.
  8. `pattern.dark_cloud_cover` (`warmup = 1`): `g[t-1].is_bullish && g[t].is_bearish && open[t] >= close[t-1] && close[t] <= g[t-1].midpoint && close[t] > open[t-1] && g[t].close_location <= 0.40`.
  9. `pattern.tweezer_bottom` (`warmup = 14`): `(low[t] - low[t-1]).abs() <= 0.15 * atr14[t] && (g[t-1].is_bearish || g[t-1].is_flat) && g[t].is_bullish`.
  10. `pattern.tweezer_top` (`warmup = 14`): `(high[t] - high[t-1]).abs() <= 0.15 * atr14[t] && (g[t-1].is_bullish || g[t-1].is_flat) && g[t].is_bearish`.
  11. `pattern.inside_bar_breakout_up` (`warmup = 14`): `high[t-1] < high[t-2] && low[t-1] > low[t-2] && close[t] > high[t-2] + 0.05 * atr14[t]`.
  12. `pattern.inside_bar_breakout_down` (`warmup = 14`): `high[t-1] < high[t-2] && low[t-1] > low[t-2] && close[t] < low[t-2] - 0.05 * atr14[t]`.
  13. `pattern.any_bullish` (`warmup = 0`): logical OR over valid `true` results among `{1, 3, 5, 7, 9, 11}`, listing matched child pattern names in `reasons`.
  14. `pattern.any_bearish` (`warmup = 0`): logical OR over valid `true` results among `{2, 4, 6, 8, 10, 12}`, listing matched child pattern names in `reasons`.
- **`structure.candle_score` (`Float` in `[-100.0, +100.0]`, `warmup = 0`):**
  - `0.0` on `is_flat_range`; else `100.0 * (0.45 * dir * body_ratio + 0.35 * (2.0 * close_location - 1.0) + 0.20 * ((lower_wick - upper_wick) / range).clamp(-1.0, 1.0)).clamp(-1.0, 1.0)`.
- **On-Chart Pattern Badges (`IndicatorConfig::show_candle_patterns`):**
  - Renders compact non-overlapping pill/triangle badges (`BE+`, `BE-`, `HMR`, `SSR`, `MST`, `EST`, `PRC`, `DCC`, `TWB`, `TWT`, `IBU`, `IBD`) below bullish bars / above bearish bars when zoomed in (`conflation_factor <= 4`), strictly masked beyond the causal Bar Replay frontier.

### 3.2. `B2` — `8` Extended Technical Indicators (`src/indicators/extended.rs`, `src/indicators/mod.rs`, `src/chart.rs`, `src/app.rs`)

1. **`ADX(14)` (`compute_adx`, `OscillatorPaneId::Adx`, refs `[20.0, 25.0]`, bounds hint `(0.0, 100.0)`):**
   - Wilder directional movement (`+DM`, `-DM`, `TR`), emitting `+DI` & `-DI` from `t = period` (`14`) and `ADX` from `t = 2 * period - 1` (`27`) per `ta_ADX.c`.
2. **`Stochastic(14, 3, 3)` (`compute_stochastic`, `OscillatorPaneId::Stochastic`, refs `[20.0, 50.0, 80.0]`, bounds hint `(0.0, 100.0)`):**
   - `fast_k[t] = 100 * (C[t] - LL_k) / (HH_k - LL_k)` (`50.0` when `HH_k <= LL_k`) from `t = k_period - 1`, `slow_k = SMA(fast_k, k_smooth)` from `k_period + k_smooth - 2`, `slow_d = SMA(slow_k, d_period)` from `k_period + k_smooth + d_period - 3`.
3. **`SuperTrend(10, 3.0)` (`compute_supertrend`, `PriceOverlay`):**
   - Uses Wilder `ATR(period)` (`hl2 = 0.5 * (H + L)`), stateful ratcheting `final_upper` / `final_lower` bands and `direction` (`+1` bullish emerald lower-band support, `-1` bearish crimson upper-band resistance) starting at `t = period - 1`, breaking polyline color cleanly on flips.
4. **`Parabolic SAR(0.02, 0.02, 0.20)` (`compute_psar`, `PriceOverlay`):**
   - Exact Wilder/TA-Lib `ta_SAR.c` state machine (`sar: Vec<Option<f64>>`, `is_long: Vec<Option<bool>>`) from `t = 1`, rendered as crisp emerald/amber directional dots on the main price pane.
5. **`Keltner Channels(20, 10, 2.0)` (`compute_keltner_channels`, `PriceOverlay`):**
   - `middle = EMA(close, ema_period)`, `atr = ATR(atr_period)`, `upper/lower = middle ± mult * atr` from `max(ema_period, atr_period) - 1`.
6. **`CCI(20)` (`compute_cci`, `OscillatorPaneId::Cci`, refs `[-100.0, 0.0, +100.0]`):**
   - `(tp - sma_tp) / (0.015 * mean_abs_dev)` from `t = period - 1` with scale-relative zero guard.
7. **`MFI(14)` (`compute_mfi`, `OscillatorPaneId::Mfi`, refs `[20.0, 50.0, 80.0]`, bounds hint `(0.0, 100.0)`):**
   - Typical-price raw money flow classified by `tp[t] > tp[t-1]` vs `tp[t] < tp[t-1]` over `period` changes (`t >= period`), emitting `50.0` on zero-flow windows.
8. **`Volume SMA(20)` (`compute_volume_sma`, `PriceOverlay` on main-pane volume histogram):**
   - Causal rolling `SMA(volume, period)` rejecting negative/non-finite volume windows (`None`), rendered directly over the main-pane volume bars using the volume sub-band scale.

### 3.3. `B3` — Confirmed-Pivot Divergences (`right_bars = 3`) & `101`-Entry `SIGNAL_CATALOG` Superset (`src/scanner/catalog.rs`)

- **Confirmed Swing Pivots (`left_bars = 3`, `right_bars = 3`, `causal_delay_bars = 3`):**
  - At bar `t >= 6`, candidate `p = t - 3` uses strictly `candles[p-3..=p+3]` (`p + 3 == t`, **zero `[t+1..]` lookahead**!).
  - Strict tie-breaking matching `divergence.py:89-121`: `is_pivot_high(p)` iff `H[p] > H[p-k]` (`k=1..=3`), `H[p] >= H[p+m]` (`m=1..3`), and `H[p] > H[p+3]`; symmetric for `is_pivot_low(p)`.
- **`12` Four-Way Confirmed Divergences (`min_sep = 5`, `max_sep = 60`):**
  - Across `RSI(14)`, `MACD Histogram(12, 26, 9)`, and `Stochastic Fast %K(14)`:
    - `regular_bullish` (`LOW` pivots: `L[p2] < L[p1] && osc[p2] > osc[p1]`)
    - `hidden_bullish` (`LOW` pivots: `L[p2] > L[p1] && osc[p2] < osc[p1]`)
    - `regular_bearish` (`HIGH` pivots: `H[p2] > H[p1] && osc[p2] < osc[p1]`)
    - `hidden_bearish` (`HIGH` pivots: `H[p2] < H[p1] && osc[p2] > osc[p1]`)
- **Complete `101`-Entry `SIGNAL_CATALOG` Superset:**
  - Indices `0..17`: All `17` R15/R16 entries preserved verbatim (`0` breaking changes to saved `CustomSignalRuleSpec` or `SignalCatalogTrigger` alerts).
  - Indices `17..89`: All `72` exact IDs from `ARCHIVE/backend/app/domain/signals/registry.py` (`volume.*` 4, `pattern.*` 14, `tech.*` 6, `regime.*` 7, `sr.*` 2, `structure.candle_score` 1, `vsa.*` 6, `ichimoku.*` 7, `divergence.*` 14, `health.*` 3, `bb.*` 8).
  - Indices `89..101`: `12` native triggers for the `B2` extended indicators (`technical.adx_strong_trend`, `technical.di_bull_cross`, `technical.di_bear_cross`, `technical.stoch_bull_cross`, `technical.stoch_bear_cross`, `technical.supertrend_flip_bull`, `technical.supertrend_flip_bear`, `technical.psar_flip_bull`, `technical.psar_flip_bear`, `technical.keltner_upper_breakout`, `technical.cci_oversold_cross`, `technical.mfi_oversold_cross`).

### 3.4. `B4` — Multi-Horizon Money Flow `ProxyBB` (`T03..T200`), `MarketBBAggregator`, & `Point-in-Time` `UniverseResolver` (`src/scanner/proxy_bb.rs`, `src/scanner/universe.rs`, `src/backtest/`)

- **`ProxyBBCalculator` (`src/scanner/proxy_bb.rs`):**
  - Horizons `BBHorizon::ALL = [T03(3), T05(5), T10(10), T20(20), T50(50), T200(200)]`.
  - Per-bar `OHLCV_PROXY` decomposition: `tp = (H + L + C) / 3.0`, `val = if vol > 0.0 { tp * vol } else { 0.0 }`, `TH = H.max(prev_C)`, `TL = L.min(prev_C)`, `P = if TH > TL { ((2*C - TH - TL) / (TH - TL)).clamp(-1.0, 1.0) } else { 0.0 }`, `buy_val = 0.5 * (1.0 + P) * val`, `sell_val = 0.5 * (1.0 - P) * val`, `sp = P * val`.
  - Prefix-sum $O(N)$ rolling windows for all 6 horizons: `t + 1 < H` -> `is_warmup = true, bb_value = None`; `den_sum <= 0.0` -> `is_warmup = false, bb_value = None` (`AT05`); else `oib = (num_sum / den_sum).clamp(-1.0, 1.0)`, `bb_value = (50.0 * (1.0 + oib)).clamp(0.0, 100.0)`, `BBDirection` (`±1e-7` hysteresis vs prior valid `bb_value`), `BBRegime` (`> 50 + 1e-7` `Positive`, `< 50 - 1e-7` `Negative`, else `Neutral`), and `regime_run_length`.
  - Rendered both in an interactive multi-horizon `T03..T200` `ProxyBB` inspector table AND optional `OscillatorPaneId::ProxyBb` chart sub-pane (`T05` fast + `T20` baseline + `T50` structural lines around `50.0` equilibrium).
- **`MarketBBAggregator` (`src/scanner/proxy_bb.rs`):**
  - Aggregates constituent `buy_val` and `sell_val` across `UniverseResolver`-eligible active symbols on each UTC session date **before** computing `100.0 * rolling_buy / rolling_total` (`AT09`, `AT10`), alongside independent count & trading-value `FlowBreadthHorizonPoint` (`AT17`) and `MarketPublicationStatus` (`CanonicalPublished`, `ResearchRetrospective`, `UnavailableDegraded`).
- **`UniverseResolver` (`src/scanner/universe.rs`) & Survivorship-Bias-Free Backtesting (`src/backtest/`):**
  - `SessionDate` (`YYYY-MM-DD` `Copy` packed integer `year * 10_000 + month * 100 + day` with exact UTC timestamp conversion), `UniverseDefinition`, `UniverseMember`, `UniverseMode::{PointInTime, RetrospectiveFixed}`, `UniverseStatus`, `MemberStatus`, `CoverageStatus`, non-overlapping episode validator, deterministic `SplitMix64` `content_hash`, and zero-dependency JSON/RON loader compatible with `pit_test_universe.json` & `sumi420_v1_candidate.json` plus bundled `VN30_PIT_V1` universe.
  - Wired into `run_portfolio_backtest_with_universe` (`src/backtest/walk_forward.rs`), `run_multi_symbol_battle_with_universe` (`src/backtest/engine.rs`), and the `Strategy Lab -> Portfolio / Battle` + `Scanner -> Breadth` workstation panels!
