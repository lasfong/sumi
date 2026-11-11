# INDICATOR ENGINE & MULTI-PANE SYNCHRONIZATION SPECIFICATION (`MILESTONE R04 / WAVE 2`)

- **Session Identity:** `SESSION-R04-20261111T140100+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Research Gate Status:** `VERIFIED` (`DEC-005` Cleared — Live REA MCP + TA-Lib BSD C Source + LWC v5.2.0 Apache 2.0 + Legacy Sumi Archive)
- **Authorized Reference Targets:**
  1. **TA-Lib C Source** (`/Users/mizuhara/workspace/sumi/research_repos/ta-lib/src/ta_func/ta_{SMA,EMA,RSI,MACD,BBANDS,ATR}.c`, BSD License)
  2. **Lightweight Charts v5.2.0** (`/Users/mizuhara/workspace/sumi/research_repos/lightweight-charts-5.2.0/src/{model/pane.ts,gui/pane-separator.ts,gui/chart-widget.ts,model/series/line-pane-view.ts,model/series/histogram-pane-view.ts,renderers/histogram-renderer.ts,renderers/walk-line.ts}`, commit `868cae27bd1acafa0128d8d868ea740a59ae42ce`, Apache 2.0)
  3. **Legacy Sumi Archive** (`/Users/mizuhara/workspace/sumi/ARCHIVE/frontend/src/{features/indicators/indicatorDomain.ts,components/chart/IndicatorRenderRegistry.ts,components/chart/PaneManager.ts,components/chart/IndicatorPaneChrome.tsx}` & `ARCHIVE/backend/app/domain/engine/indicator_engine.py`)
  4. **Compliance Boundary (`DEC-006`):** Proprietary TradingView Desktop was **not** inspected, hooked, or queried regardless of host login/process state.

---

## 1. Live REA MCP Execution Evidence (`Milestone R04`)

All REA operations below were executed live via the `rea-agents@6.8.0` MCP server during `SESSION-R04-20261111T140100+0700`:

| Op ID | REA Tool & Query | Target Scope | Actual Verified Output | Exact 64-Hex Evidence ID |
| :--- | :--- | :--- | :--- | :--- |
| `REA-R04-01` | `analyze_javascript_application` (`detail: "summary"`) | `ARCHIVE/frontend` (`4` TS/TSX files: `indicatorDomain.ts`, `IndicatorRenderRegistry.ts`, `PaneManager.ts`, `IndicatorPaneChrome.tsx`) | Artifact SHA-256 `243b2d6eaac9d00e0cd2e2623da5ea5f84bb751ad89e2ba40a04be7f7b794b92`; `4` files parsed (`0` failures), `40,772` bytes, `6,816` AST nodes, `53` findings, `2,799` `data-flow`, `342` `object-flow`, `317` `call-flow`, `143` `closure`, `16` `boundary` relations (`10,237/10,237` view items). | Parent: `ev_8737910237dddc3c6f846a2eb10db4023cf433ad7e2b8f5974081656c20e73c5`<br>Summary: `ev_aaef332f6730a1a633abc64cfb785d0ec74c8c9a7aee9ab81a24aa38de63acd0` |
| `REA-R04-02` | `analyze_javascript_application` (`detail: "summary"`) | `research_repos/lightweight-charts-5.2.0` (`7` TS files: `pane.ts`, `pane-separator.ts`, `chart-widget.ts`, `line-pane-view.ts`, `histogram-pane-view.ts`, `histogram-renderer.ts`, `walk-line.ts`) | Artifact SHA-256 `6a26c4adafe9539017d004f611425bc9692a278cd45d4a41ffe381afd7b6c9d5`; `7` files parsed (`0` failures), `69,831` bytes, `10,313` AST nodes, `201` findings, `2,764` `data-flow`, `362` `object-flow`, `138` `closure`, `90` `call-flow`, `1` `boundary` relation (`1,197/1,197` view items). | Parent: `ev_3a1d7aeb2dbcb801ac28e38de3618addf25fb6216f117f3f4b1d9048e9591111`<br>Summary: `ev_424a42af159d237bfae336e1e28b729a367fbf62c655cd76c9cc3993fd4792b0` |
| `REA-R04-03` | `trace_javascript_semantics` (`property: "placement"`, `forward-influence`) | Parent `ev_8737910237dddc3c6f846a2eb10db4023cf433ad7e2b8f5974081656c20e73c5` (`ARCHIVE/frontend`) | `2` seed matches across `2` functions/modules (`indicatorDomain.ts` `instance.placement` & `IndicatorPaneChrome.tsx` `instance.placement`). | `ev_380c6a728de9a73e5437c9baa4313a71002783978dea75c9d997b2ff1a7a1a89` |
| `REA-R04-04` | `trace_javascript_semantics` (`property: "referenceLines"`, `forward-influence`) | Parent `ev_8737910237dddc3c6f846a2eb10db4023cf433ad7e2b8f5974081656c20e73c5` (`ARCHIVE/frontend`) | `0` matches (negative control proving legacy Sumi named the field `references` / `referencesFor()` in `IndicatorRenderRegistry.ts:50–59` rather than `referenceLines`). | `ev_f46a1dfda7613ec07a0ea58f94b9b5e1df7fe89319a4df95bf3e5a00d11c92c5` |
| `REA-R04-05` | `trace_javascript_semantics` (`property: "_stretchFactor"`, `backward-provenance`) | Parent `ev_3a1d7aeb2dbcb801ac28e38de3618addf25fb6216f117f3f4b1d9048e9591111` (`LWC v5.2.0`) | `2` seed matches, `5` traversed nodes, `3` provenance relations in `src/model/pane.ts` proving `_stretchFactor` is written exclusively at declaration/init and `setStretchFactor(factor)` (`pane.ts:143–145`). | `ev_4507f2ade5d47a54b59a7589f180e0ec5012d6e6ff86d4fc59cd5f74a1d72565` |
| `REA-R04-06` | `trace_javascript_semantics` (`property: "stretchFactor"`, `forward-influence`) | Parent `ev_3a1d7aeb2dbcb801ac28e38de3618addf25fb6216f117f3f4b1d9048e9591111` (`LWC v5.2.0`) | `5` seed matches across `src/gui/pane-separator.ts` (`L181`, `L194`) and `src/gui/chart-widget.ts` (`_adjustSizeImpl` `L496`, `L532`), proving height normalization and adjacent-pane drag transfer. | `ev_5f12ebbfe8f69dabc129ef25dab6fff37eea11945af331e34760cda658e6260f` |
| `REA-R04-07` | `inspect_rust_binary` (`detail: "summary"`) | `target/release/sumi-workstation` (`arm64`, `16,369,008` bytes, SHA-256 `581a553f9f507f19fadf4e99301d7b309b377385c1663a69634e0a07bb605914`) | Verified shipped `arm64` Mach-O 64-bit release binary (`rustc_commit` hash count `1`). | Parent: `ev_aca14f82ca8b3b3e8943b74911c25c40532f8536443b69ca1bfefd33bee7c223`<br>Summary: `ev_884970108d3c3d37a22f5c30bbf7bf3900f0aaee3aeb9ee569591dbc0c96d8d7` |
| `REA-R04-08` | `inspect_macho` (`detail: "summary"`) | `target/release/sumi-workstation` (`UUID AE7FAD2C-7CC3-3A05-B182-C4F73CA41F62`) | `1` entrypoint, `1` architecture (`arm64`), `30` load commands, `14` dylib dependencies, `324` imports, `4,765` exports, `5` segments (`5,141/5,141` view items). | Parent: `ev_533d65b5dc5e3db61aa3db163011926c4b6a68562bf7885d79c07a95c6baa894`<br>Summary: `ev_69be1f6a79dd531854995a3f2980a56f771ddc521aaa79f9c2bd186f052e7a9b` |

### Documented REA Static Analysis Limitations
- As established in R03, extensionless TypeScript imports (`./workspaceTypes`, `./chart-widget`) remain `unresolved-semantic-module` under inert JS syntax parsing, and TypeScript interface calls (`IPaneApi.setStretchFactor`) are not statically devirtualized across files. All cross-module invariants and C numerical formulas were directly verified line-by-line against `ta_*.c`, `pane-separator.ts`, `chart-widget.ts`, `PaneManager.ts`, `indicatorDomain.ts`, and `IndicatorRenderRegistry.ts`.
- Note on `sumi-researcher`: During parallel research dispatch, subagent `sumi-researcher` (`9ed207c0-...`) experienced a transient upstream OAuth DNS resolution failure (`lookup oauth2.googleapis.com: no such host`). Per `SUMI_ORCHESTRATION_CHARTER.md` §2.2, Orchestrator performed 100% of direct C/TS/Python source verification directly without fabricating subagent completion.

---

## 2. Verified Indicator Mathematical Specifications (`src/indicators/`)

All domain computations use `f64`, are `wasm32-unknown-unknown` compatible (`0` OS/UI dependencies), and align index-for-index (`len == candles.len()`) using `Option<f64>` (`None` during warmup or non-finite inputs, `Some(v)` once warmed up).

### 2.1 Simple Moving Average — `SMA(N)`
- **Source Verified:** `research_repos/ta-lib/src/ta_func/ta_SMA.c:144–158`, `ta_BBANDS.c:267–296`
- **Parameters & Domain:** `period: usize` (`period >= 1`, default `20`), input `close`.
- **Warmup / First Valid Index:** `period - 1` (`0` when `period == 1`; returns all `None` if `candles.len() < period`).
- **Exact Formula & Drift Prevention:**
  $$\text{SMA}_t = \frac{1}{N}\sum_{k = t - N + 1}^{t} C_k \quad (t \ge N - 1)$$
  Maintains an $O(1)$ sliding sum `period_total += C[t] - C[t - N]` and **reseeds** `period_total` directly from the `N`-element window every `32 * N` bars (matching `ta_BBANDS.c:267`), whenever `!sum.is_finite()`, or whenever `|sum| < 1e-6 * peak_abs_val` (catastrophic-cancellation recovery after extreme spikes) so `100,000+` bar series never accumulate IEEE-754 subtraction drift. If any input inside a window is non-finite, the window resets cleanly after the non-finite bar.

### 2.2 Exponential Moving Average — `EMA(N)`
- **Source Verified:** `research_repos/ta-lib/src/ta_func/ta_EMA.c:62–69`, `130–190`
- **Parameters & Domain:** `period: usize` (`period >= 1`, defaults `20` / `50`), input `close`.
- **Warmup / First Valid Index:** `period - 1` (with standard `unstable_period = 0`).
- **Sterbenz-Exact Coefficient Pair (`ta_EMA.c:130–135`):**
  ```rust
  let mut beta = ((period - 1) as f64) / ((period + 1) as f64);
  let k = 1.0 - beta;
  if beta < 0.5 {
      beta = 1.0 - k;
  }
  ```
  Guarantees `beta + k == 1.0` bit-for-bit across all `period >= 2`. When `period == 1`, output copies finite inputs directly (`ta_EMA.c:156–167`).
- **Seed & Monotonic Fixed-Point Recurrence (`ta_EMA.c:171–188`):**
  - Seed at `t = N - 1`: $\text{EMA}_{N-1} = \frac{1}{N}\sum_{j=0}^{N-1} C_j$ (with scaled `/ N` window fallback if `seed_sum` overflows `f64::MAX` on both `compute_ema_slices` and `compute_ema_optional_slice`).
  - Step for `t >= N`: `k.mul_add(C_t - prev, prev)` when `C_t - prev` is finite (guaranteeing exact zero-ULP fixed-point invariance on flat price series and within `<= 1 ULP` of `ta_EMA.c` `fma(beta, prev, k * C_t)`), falling back to Sterbenz-exact `beta.mul_add(prev, k * C_t)` when `C_t - prev` overflows `f64::MAX`.

### 2.3 Bollinger Bands — `BollingerBands(N, std_mult)`
- **Source Verified:** `research_repos/ta-lib/src/ta_func/ta_BBANDS.c:155–386`, `ARCHIVE/backend/app/domain/engine/indicator_engine.py:83–95`
- **Parameters & Domain:** `period: usize` (**`period >= 2`**, matching `ta_BBANDS.c:157` and `IndicatorConfig::sanitize_deserialized`, default `20`), `std_mult: f64` (`>= 0.0`, default `2.0`).
- **Warmup / First Valid Index:** `period - 1`.
- **Exact Variance Convention & Overflow Protection (`ta_BBANDS.c:251, 277–353`):**
  TA-Lib uses **population variance (`1 / N`, `ddof = 0`)** around the SMA middle band, shifted by a reference value `shift` to avoid catastrophic cancellation (`var = E[(X - shift)^2] - (E[X - shift])^2`), reseeded every `32 * N` bars or whenever `variance < 1e-6 * (peak_sum2 / N)`, with exact non-negativity guard (`if variance < 1e-12 * (var_sum2 / N) { variance = 0.0 }`) and **max-abs scaled standard deviation fallback** whenever `(C_j - \mu_t)^2` overflows `f64::MAX` (`|C_j - \mu_t| > 1.34e154`), never collapsing to a bogus zero-width band:
  $$\mu_t = \text{SMA}_N(C)_t, \quad \sigma_t = \sqrt{\max\!\left(0,\, \frac{1}{N}\sum_{j=t-N+1}^{t}(C_j - \mu_t)^2\right)}$$
  $$\text{Upper}_t = \mu_t + m \cdot \sigma_t, \quad \text{Middle}_t = \mu_t, \quad \text{Lower}_t = \mu_t - m \cdot \sigma_t$$

### 2.4 Relative Strength Index — `RSI(N)` (Wilder 1978)
- **Source Verified:** `research_repos/ta-lib/src/ta_func/ta_RSI.c:70–77`, `143–251`
- **Parameters & Domain:** `period: usize` (`period >= 2`, default `14`).
- **Warmup / First Valid Index:** **`period`** (index `14` for `RSI(14)`, consuming `15` closes `C_0..=C_14` to form `14` deltas).
- **Seed & Wilder Recurrence (`ta_RSI.c:147–248`):**
  - For $t = 1 \dots N$: $\Delta_t = C_t - C_{t-1}$, $g_t = \max(\Delta_t, 0)$, $\ell_t = g_t - \Delta_t = \max(-\Delta_t, 0)$.
  - Initial Wilder averages at $t = N$: $\bar{G}_N = \frac{1}{N}\sum_{t=1}^{N} g_t$, $\bar{L}_N = \frac{1}{N}\sum_{t=1}^{N} \ell_t$.
  - For $t > N$:
    $$\bar{G}_t = \frac{\bar{G}_{t-1}(N - 1) + g_t}{N}, \quad \bar{L}_t = \frac{\bar{L}_{t-1}(N - 1) + \ell_t}{N}$$
  - **Exact Edge-Case Contract (`ta_RSI.c:180–201`, issues #253 & #480):**
    Let $S_t = \bar{G}_t + \bar{L}_t$:
    - If $S_t > 0.0$ (exact comparison to `0.0`, never an arbitrary fixed epsilon): $\text{RSI}_t = 100.0 \times \frac{\bar{G}_t}{S_t} \in [0, 100]$ (yields `100.0` if $\bar{L}_t = 0, \bar{G}_t > 0$, and `0.0` if $\bar{G}_t = 0, \bar{L}_t > 0$).
    - If $S_t == 0.0$ (completely flat series, $0/0$): **`RSI_t = 50.0`** (neutral).

### 2.5 Moving Average Convergence Divergence — `MACD(fast, slow, signal)`
- **Source Verified:** `research_repos/ta-lib/src/ta_func/ta_MACD.c:66–97`, `174–359`; `ARCHIVE/frontend/src/components/chart/IndicatorRenderRegistry.ts:66–96`
- **Parameters & Domain:** `fast_period` (default `12`, `>= 2`), `slow_period` (default `26`, `>= 2`), `signal_period` (default `9`, `>= 1`). If `slow_period < fast_period`, swap them (`ta_MACD.c:174–180`).
- **Chart Series vs Stateless Window Nuance (Explicitly Documented):**
  - When `TA_MACD` is called on a sub-range `[startIdx..endIdx]`, `ta_MACD.c:275` offsets `fastToday` to `slow - fast` (`index 14`) so it reads no further back than `lookbackSlow + lookbackSignal`.
  - In a full-series charting workstation (`IndicatorRenderRegistry.ts:66–96` / `pandas_ta` / TradingView), `EMA(fast)` and `EMA(slow)` are both anchored at bar `0` of the series so `MACD_line[t] == EMA(fast)[t] - EMA(slow)[t]` holds identically from bar `slow - 1` (`index 25`) onward, and `Signal` is `EMA(signal_period)` over the finite `MACD_line` sequence starting at `slow - 1`!
  - We provide **full-series chart `MACD`** (`macd_line` valid from `slow - 1 = 25`; `signal_line` and `histogram = macd_line - signal_line` valid from `(slow - 1) + (signal - 1) = 33`), plus `signal_period == 1` exact handling (`signal = macd`, `histogram = 0.0`, `ta_MACD.c:212–215`).

### 2.6 Average True Range — `ATR(N)` (Wilder 1978)
- **Source Verified:** `research_repos/ta-lib/src/ta_func/ta_ATR.c:64–78`, `167–274`
- **Parameters & Domain:** `period: usize` (`period >= 1`, default `14`), inputs `(high, low, close)`.
- **Warmup / First Valid Index:** **`period`** (`index 14` for `ATR(14)`; returns all `None` if `candles.len() <= period`).
- **Exact True Range & Wilder Smoothing (`ta_ATR.c:167–240`):**
  - True Range requires `close[t - 1]`, so `TR` starts at `t = 1`:
    $$\text{TR}_t = \max\bigl(H_t - L_t,\; |C_{t-1} - H_t|,\; |C_{t-1} - L_t|\bigr) \quad (t \ge 1)$$
  - Seed at `t = N`: $\text{ATR}_N = \frac{1}{N}\sum_{t=1}^{N} \text{TR}_t$ (with scaled `/ N` fallback if `tr_sum` overflows `f64::MAX`).
  - Sterbenz-exact Wilder coefficients (`w_beta = (N - 1) as f64 / N as f64`, `w_alpha = 1.0 - w_beta`, `if w_beta < 0.5 { w_beta = 1.0 - w_alpha; }`) for $t > N$: uses `w_alpha.mul_add(TR_t - prev, prev)` when `TR_t - prev` is finite (zero-ULP flat invariance, within `<= 1 ULP` of `ta_ATR.c`) and `w_beta.mul_add(prev, w_alpha * TR_t)` on overflow.

---

## 3. Multi-Pane Layout, Separator Resize & Sub-Pixel Series Conflation

### 3.1 Stretch-Factor Pane Stack (`chart-widget.ts:507–556`, `pane-separator.ts:10–223`, `PaneManager.ts:76–77`)
- **Shared Horizontal Axis:** `ChartViewportState::time_scale` + `kinetic` are shared across the top Main Price Pane and `0..=3` stacked Oscillator Sub-Panes (`RSI`, `MACD`, `ATR`). The bottom `time_axis_rect` (`28px`) sits below the lowest visible pane.
- **Independent Vertical Axes:**
  - Main Price Pane owns `main_price_scale` (`Linear`/`Log`/`%`/`Idx100`) + `volume_price_scale`.
  - Each Oscillator Sub-Pane (`RSI`, `MACD`, `ATR`) owns its own `PriceScale` (`PriceScaleMode::Normal`, `auto_scale = true` by default, supporting vertical drag-to-scale & double-click autoscale reset on its own right axis gutter, while double-clicking ANY plot canvas resets `time_scale` and all pane `PriceScale`s synchronously).
- **Stretch Factor Allocation & Interactive Separator Drag:**
  - Default stretch factors match legacy Sumi (`PaneManager.ts:76–77`): Main Pane = `4.0`, each active Oscillator Sub-Pane = `1.0`.
  - Minimum pane height: `MIN_PANE_HEIGHT = 44.0` logical px (with pass-snapshot monotonic water-filling clamp when constrained, exceeding LWC's `30px` minimum so HUD + axis labels remain legible).
  - Adjacent panes are separated by a `SEPARATOR_HEIGHT = 5.0` logical px interactive splitter bar (`CursorIcon::ResizeVertical`). Dragging splitter `i` syncs effective rendered stretch shares on `drag_started()` and transfers stretch factor strictly between `pane[i]` and `pane[i + 1]` (`pane-separator.ts:181–220`) with `0.0 px` jump on uninvolved panes. Double-clicking a splitter restores default stretch weights.

### 3.2 `100,000`-Bar Performance Architecture (`DECISION-RENDERER-002` Preservation)
To guarantee CPU frame times stay well below the `4.0 ms` hybrid-`wgpu` trigger even at `100,000` bars with all 6 indicators active across 4 panes:
1. **Dirty-Tracked Indicator Cache (`IndicatorCache`) + Associative 16-Bar `SeriesBlockIndex`:** Indicators are evaluated **once** in $O(N)$ (`9.626 ms` cold all-6 on `100K` bars) when the dataset or indicator configuration changes (`0.041 µs` warm sync during pan, zoom, resize, or crosshair movement). Each series maintains a 16-bar `SeriesBlockIndex` (`BlockExtrema` with `(first, last, min, max, dom, valid_count)`) accelerating visible-range autoscale and conflated envelope queries by $16\times$ with bit-for-bit identical output.
2. **Unified `conflation_factor` + Pre-Tessellated `egui::Mesh` Batching:**
   - Every visual series (`Candlesticks`, `Volume`, `BollingerBands` fill + 3 lines, `SMA`, `EMA`, `RSI`, `MACD` histogram + 2 lines, `ATR`) shares the **exact same `TimeScale::conflation_factor(ppp)`** (`1x..32x`) and global `[k * step, (k + 1) * step)` bucket boundaries — zero horizontal center or width discrepancy across panes at any zoom level.
   - Axis-aligned `Candlestick` bodies/wicks, `Volume` bars, `MACD` histogram bars, and `BollingerBands` chronological ribbon quads emit directly into pre-tessellated `egui::Mesh` buffers (`0.0 ms` feathering overhead), while smooth sub-pixel line jogs (`< 0.65px` vertical excursion at identical `x`) collapse to a single vertex while preserving every `>= 0.65px` M4 spike and every `None`-gap break.

---

## 4. `sumi-critic` Adversarial Audit & 13/13 Remediation Log

All 13 findings identified during the Milestone R04 adversarial mathematical & layout review (`sumi-critic`) were remediated in code and locked down with dedicated `critic_r04_regression_*` unit tests:

| Finding ID | Severity | Subsystem | Root Issue Identified | Verified Remediation & Regression Test |
| :--- | :---: | :--- | :--- | :--- |
| `CRITICAL-R04-01` | `CRITICAL` | `bands.rs` | `d * d` overflow when `|w - mean| > 1.34e154` collapsed `stddev` to `0.0` (bogus zero-width bands), `peak_sum2` could stick at `+Inf`, and `period == 1` diverged from `ta_BBANDS.c:157`. | Enforced `period >= 2`, guarded `peak_sum2` finiteness, and added max-abs scaled `stddev` fallback (`NaN -> None` only when true bands exceed `f64::MAX`). Tested in `critic_r04_regression_bollinger_extreme_magnitude_and_period_domain`. |
| `CRITICAL-R04-02` | `CRITICAL` | `chart.rs` | Mutating `remaining_h` mid-pass while `free_stretch` stayed frozen broke stretch monotonicity (`[0.75, 0.75, 1.45, 0.75]` at `200px`); `split_x`/`bottom_y` could exceed tiny `outer_rect`. | Snapshotted `pass_remaining_h` per water-filling pass and clamped `split_x`/`split_y`/`bottom_y` + adaptive `sep_h` on tiny rects. Tested in `critic_r04_regression_multipane_waterfill_monotonicity_and_tiny_rect`. |
| `MAJOR-R04-01` | `MAJOR` | `chart.rs` | Separate `series_conflation_factor` (`128x` vs `32x` at `100K`, `2x` vs `1x` at `0.75px` Retina) shifted overlay/MACD centers relative to candlesticks. | Removed `series_conflation_factor` completely; unified `conflation_factor` (`32x`) across all panes and batched axis-aligned rects into `egui::Mesh` (`3.141 ms` at `100K`). |
| `MAJOR-R04-02` | `MAJOR` | `chart.rs` | Bollinger fill `Mesh` used `(max_u, min_l)` per bucket instead of exact chronological border `(entry, exit)`, bleeding outside `upper`/`lower` lines under conflation. | Shared `bucket_line_envelope_indexed` + `resolve_bucket_screen_ys` so ribbon quads connect `(prev_x, u_exit, l_exit) -> (x, u_entry, l_entry)` identically to border lines. |
| `MAJOR-R04-03` | `MAJOR` | `chart.rs` | Dragging a separator after one pane hit `MIN_PANE_HEIGHT` caused an initial height jump on uninvolved panes; missing `top_pane`/`bottom_pane` drag identity check. | Normalized stored stretches to `pre_layout` rendered height shares at `drag_started()` and verified `(top_pane, bottom_pane)` on `dragged()`. Tested in `critic_r04_regression_cross_pane_double_click_and_clamped_separator_drag`. |
| `MAJOR-R04-04` | `MAJOR` | `moving_averages.rs`, `oscillators.rs` | High-magnitude (`~3e307..1e308`) inputs could overflow `compute_ema_optional_slice` seed, Wilder `RSI` `(avg * decay + gain)`, or cancel `SMA` sliding sums after spikes. | Added `/ period_f` scaled seed fallbacks, `wilder_step` overflow-safe difference/scaled fallback, `rsi_from_averages` max-scale fallback, and `peak_abs_val` cancellation reseed in `SMA`. Tested in `critic_r04_regression_high_magnitude_overflow_sma_ema_rsi_and_sterbenz`. |
| `MAJOR-R04-05` | `MAJOR` | `moving_averages.rs`, `oscillators.rs` | `EMA`/`ATR` weight pair and spec wording needed exact TA-Lib Sterbenz sequence (`if beta < 0.5 { beta = 1.0 - k; }`) + documented `k.mul_add(delta, prev)` contract. | Implemented `ema_sterbenz_weights` (`ta_EMA.c:130–135`) and Sterbenz `ATR` weights, and aligned §2.2 & §2.6 with exact code. |
| `MAJOR-R04-06` | `MAJOR` | `chart.rs` | `paint_macd_pane` clamped off-screen `0.0` line onto pane border and biased `0.0` / small positive histogram bars downward by `+0.75px`. | Guarded `0.0` reference line to inside `plot_rect`, rendered `0.0` bars symmetrically `(zy - 0.5..zy + 0.5)`, and extended non-zero bars strictly toward `val_y` via `resolve_macd_histogram_bar_ys`. Tested in `critic_r04_regression_macd_zero_bar_symmetry_and_offscreen_clipping`. |
| `MAJOR-R04-07` | `MAJOR` | `chart.rs` | Double-clicking an oscillator sub-pane reset `time_scale` + that oscillator's `PriceScale` while leaving `main_price_scale` manually panned off-screen. | Centralized plot double-click detection in `render_workstation_chart` to invoke `viewport.reset_to_default()` synchronously across all panes. Tested in `critic_r04_regression_cross_pane_double_click_and_clamped_separator_drag`. |
| `MINOR-R04-01` | `MINOR` | `chart.rs`, `mod.rs` | HUD labels read raw `indicators` before sanitization and inspected `start_index` rather than `end_index` of conflated buckets. | Exposed `IndicatorCache::applied_config()` and resolved `hud_series_index = conflated_candle_at(...).end_index`. |
| `MINOR-R04-02` | `MINOR` | `mod.rs` | `visit_conflated_line_segments_indexed` bridged across single-bar `None` gaps inside or at boundaries of `step >= 2` buckets. | Added `(first_idx, last_idx, valid_count)` to `BlockExtrema` and `has_leading_gap` / `has_internal_gap` / `has_trailing_gap` to `BucketLineEnvelope`. Tested in `critic_r04_regression_sub_bucket_gap_breaks_conflated_segments`. |
| `MINOR-R04-03` | `MINOR` | `SPEC.md`, `oscillators.rs` | Spec listed `42.0`/`4.0` instead of `44.0`/`5.0` and `RsiIndicator::warmup_bars()` returned `1` when `period == 1`. | Updated spec §3.1 (`MIN_PANE_HEIGHT = 44.0`, `SEPARATOR_HEIGHT = 5.0`) and clamped `RsiIndicator::warmup_bars` to `self.period.max(2)`. |
| `NIT-R04-01` | `NIT` | `chart.rs`, `oscillators.rs` | Unused `let _ = WHITE_UV;` and duplicated `RSI`/`MACD` reference level literals. | Removed `WHITE_UV` import and wired shared `RSI_REFERENCE_LEVELS` / `MACD_REFERENCE_LEVELS` constants into both trait outputs and pane painters. |

---

## 5. Measured Release Benchmark (`Apple M1`, `--release`, 4 Panes + All 6 Indicators)

Full machine-readable output: [`docs/evidence/r04_indicator_multipane_benchmark.json`](file:///Users/mizuhara/workspace/sumi/docs/evidence/r04_indicator_multipane_benchmark.json) and [`docs/evidence/r04_indicator_multipane_benchmark.md`](file:///Users/mizuhara/workspace/sumi/docs/evidence/r04_indicator_multipane_benchmark.md).

- **Cold All-6 Indicator Evaluation (`100,000` bars):** **`9.626 ms`** median (`9.746 ms` P95), **`0.041 µs`** warm dirty-cache sync (`20.50 MiB` heap including all 9 `SeriesBlockIndex` accelerators).
- **4-Pane Standard Zoom (`100,000` bars, `bar_spacing = 8.0`, `1x`):** **`0.134 ms`** total CPU (`0.091 ms` UI + `0.043 ms` Tessellation).
- **4-Pane Dense Swing View (`100,000` bars, `bar_spacing = 1.2`, `Log`, `1x`):** **`0.497 ms`** total CPU (`0.306 ms` UI + `0.191 ms` Tessellation).
- **4-Pane Full-Dataset Fit (`100,000` bars, `Log`, unified `32x` conflation, `3,124` buckets across all 10 series):** **`3.141 ms`** median (`3.323 ms` P95) — **`PASS (< 4.0 ms` `DECISION-RENDERER-002` pure-`egui::Painter` gate and `2.65×` faster than the `8.33 ms` 120 FPS budget)**.

