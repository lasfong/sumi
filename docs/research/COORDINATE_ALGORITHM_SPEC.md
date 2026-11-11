# COORDINATE ALGORITHM SPECIFICATION — LIGHTWEIGHT CHARTS v5.2.0

- **Original Session:** `SESSION-R02-20261110T230337+0700`
- **Audited & Rectified In:** `SESSION-R03-20261111` (Verified line-by-line against LWC `v5.2.0`, commit `868cae27bd1acafa0128d8d868ea740a59ae42ce`)
- **Primary Reference Files:**
  - `src/model/time-scale.ts` (1,270 lines)
  - `src/model/time-scale-visible-range.ts` (29 lines)
  - `src/model/price-scale.ts` (1,137 lines)
  - `src/model/price-scale-conversions.ts` (130 lines)
  - `src/model/price-range-impl.ts` (105 lines)
  - `src/model/kinetic-animation.ts` (143 lines)
  - `src/model/chart-model.ts` (1,285 lines) & `src/model/pane.ts`
  - `src/gui/chart-widget.ts` & `src/gui/pane-widget.ts`
  - `src/renderers/optimal-bar-width.ts` & `src/renderers/candlesticks-renderer.ts`
  - `src/helpers/mathex.ts` (`clamp(v, min, max) = Math.min(Math.max(v, min), max)`)

> [!IMPORTANT]
> **Coordinate Space Distinction (`VERIFIED`):**
> All coordinates returned by `TimeScale` (`[0, width)`) and `PriceScale` (`[0, height)`) in Lightweight Charts are **Pane-Local CSS Pixel Coordinates** relative to the top-left corner `(0, 0)` of a single pane's plot area (because LWC mounts a dedicated `<canvas>` per `PaneWidget`). In a single-surface renderer such as `egui::Painter`, screen coordinates are offset by `(plot_rect.left(), plot_rect.top())`:
> - `local_x = screen_x - plot_left`, `screen_x = plot_left + local_x`
> - `local_y = screen_y - plot_top`, `screen_y = plot_top + local_y`

---

## 1. Horizontal Coordinate System (`TimeScale`)

### 1.1 State Model (`src/model/time-scale.ts:324–358`)

| Parameter | LWC Field | Default | Meaning |
|-----------|-----------|---------|---------|
| `width` | `_width` | `0` | Width of the plot area in logical pixels (`width > 0` when active). Shared across all vertically stacked panes (`chart-widget.ts:505, 558`). |
| `base_index` | `_baseIndexOrNull` | `null` | Integer index of the last (rightmost) data bar (`total_bars - 1`), or `null` when dataset is empty (`L870–876`). |
| `right_offset` | `_rightOffset` | `0` (`12.0` in Sumi workstation) | Signed bar shift at the right edge: `> 0` adds empty future bar slots to the right; `< 0` scrolls into historical past. |
| `bar_spacing` | `_barSpacing` | `6.0` | Horizontal slot width per bar in logical pixels (`minBarSpacing = 0.5`). |

`isEmpty()` (`L450–452`) is `true` iff `width === 0 || points.length === 0 || baseIndexOrNull === null`.

### 1.2 Bar Index → Pane-Local X (`indexToCoordinate`, `L551–573`)

**Guard:** If `isEmpty()` or `!isInteger(index)`, returns `0`.

$$\Delta_{\text{right}} = \text{baseIndex} + \text{rightOffset} - \text{index}$$

$$x_{\text{local}}(\text{index}) = \text{width} - (\Delta_{\text{right}} + 0.5) \cdot \text{barSpacing} - 1$$

- `+ 0.5` places the bar's center in the middle of its `barSpacing` slot.
- `- 1` aligns the right-most pixel column at `width - 1`.
- **Fixture (`VERIFIED`):** `width = 1000, barSpacing = 10, baseIndex = 99, rightOffset = 5, index = 50` $\implies \Delta_{\text{right}} = 54 \implies x_{\text{local}}(50) = 1000 - 54.5 \times 10 - 1 = \mathbf{454.0}$.

### 1.3 Pane-Local X → Float Index & Discrete Bar Index (`_coordinateToFloatIndex`, `L996–1008`; `coordinateToIndex`, `L575–585`)

$$\Delta_{\text{right}}(x_{\text{local}}) = \frac{\text{width} - 1 - x_{\text{local}}}{\text{barSpacing}}$$

$$\text{floatIndex}(x_{\text{local}}) = \frac{\text{round}\!\left((\text{baseIndex} + \text{rightOffset} - \Delta_{\text{right}}(x_{\text{local}})) \times 10^6\right)}{10^6}$$

$$\text{coordinateToIndex}(x_{\text{local}}) = \lceil \text{floatIndex}(x_{\text{local}}) \rceil$$

**Exact Half-Slot Round-Trip Invariant (`VERIFIED`):**
Evaluating `floatIndex` at the exact center `x_local(i)` of bar `i` yields:

$$\text{floatIndex}(x_{\text{local}}(i)) = i - 0.5, \qquad \lceil \text{floatIndex}(x_{\text{local}}(i)) \rceil = \lceil i - 0.5 \rceil = i$$

Across the entire pixel slot $(x_{\text{local}}(i) - 0.5 \cdot \text{barSpacing},\; x_{\text{local}}(i) + 0.5 \cdot \text{barSpacing}]$, $\text{floatIndex}(x_{\text{local}}) \in (i - 1, i]$, so `ceil(floatIndex)` maps the entire slot cleanly to integer bar `i`!
- **Fixture (`VERIFIED`):** At `x_local = 454.0` (the center of bar `50`), `floatIndex(454.0) = 49.5` and `coordinateToIndex(454.0) = ceil(49.5) = 50`.

---

## 2. Cursor-Anchored Zoom & Time-Axis Drag Scale

### 2.1 Cursor-Anchored Wheel/Pinch Zoom (`ChartModel.zoomTime`, `chart-model.ts:829–841`; `TimeScale.zoom`, `time-scale.ts:732–745`)

1. Guard: if `timeScale.isEmpty() || scale === 0`, return.
2. Clamp anchor coordinate inside `[1, width]`: `pointX = max(1, min(pointX, width))`.
3. Record float index under cursor before spacing change:
   $$f_0 = \text{\_coordinateToFloatIndex}(\text{pointX})$$
4. Compute candidate bar spacing (`scale > 0` zooms in, `scale < 0` zooms out; wheel handler clamps `scale ∈ [-1, 1]` at `chart-widget.ts:613`):
   $$\text{barSpacing}_{\text{cand}} = \text{barSpacing} + \text{scale} \cdot \frac{\text{barSpacing}}{10}$$
5. Call `setBarSpacing(barSpacing_cand)` (`L599–614`), which clamps `_barSpacing` via `_correctBarSpacing()`, rescales `_rightOffset` if `rightOffsetPixels !== undefined`, and runs `_correctOffset()`.
6. If `!options.rightBarStaysOnScroll`:
   $$\text{setRightOffset}\!\left(\text{rightOffset} + \left(f_0 - \text{\_coordinateToFloatIndex}(\text{pointX})\right)\right)$$
   which re-runs `_correctOffset()`.

**Cursor-Lock Invariant:** When `!rightBarStaysOnScroll`, `rightOffsetPixels === undefined`, and `_correctOffset()` does not hit `[minRightOffset, maxRightOffset]`, $|\text{\_coordinateToFloatIndex}_{\text{after}}(\text{pointX}) - f_0| \le 10^{-6}$.

### 2.2 Time-Axis Horizontal Drag Scale (`startScale` / `scaleTo` / `endScale`, `time-scale.ts:747–787`)

Dragging horizontally on the time axis scales `barSpacing` relative to the right edge:
- `startLengthFromRight = clamp(width - x_local, 0, width)`
- `currentLengthFromRight = clamp(width - scaleStartX, 0, width)`
- If both `> 0`: `setBarSpacing(startBarSpacing * startLengthFromRight / currentLengthFromRight)`.

---

## 3. Horizontal Scrolling & Clamping Bounds

### 3.1 Drag Scroll (`startScroll` / `scrollTo` / `endScroll`, `time-scale.ts:789–822`)

1. `startScroll(x0)` snapshots `{ barSpacing, rightOffset }` and `_scrollStartPoint = x0`.
2. `scrollTo(x)`:
   $$\Delta_{\text{logical}} = \frac{x_0 - x}{\text{barSpacing}}, \qquad \text{rightOffset} = \text{startRightOffset} + \Delta_{\text{logical}}$$
   followed by `_correctOffset()`.
   - Dragging **right** (`x > x0` $\implies \Delta_{\text{logical}} < 0$) decreases `rightOffset`, shifting bars right to reveal **older (past)** bars.
   - Dragging **left** (`x < x0` $\implies \Delta_{\text{logical}} > 0$) increases `rightOffset`, shifting bars left to add **future** space.

### 3.2 Spacing & Offset Clamping (`_correctBarSpacing`, `L1044–1070`; `_correctOffset`, `L1100–1134`)

- **Safe clamp (`src/helpers/mathex.ts:1–3`):** `clamp(v, minVal, maxVal) = min(max(v, minVal), maxVal)`. Note: when `minVal > maxVal`, `maxVal` wins without throwing/panicking!
- **`_minBarSpacing()`:** `(fixLeftEdge && fixRightEdge && N > 0) ? (width / N) : options.minBarSpacing`.
- **`_maxBarSpacing()`:** `(options.maxBarSpacing > 0) ? options.maxBarSpacing : (width * 0.5)`.
- **`_minRightOffset()`:** `null` if empty; else `firstIndex - baseIndex - 1 + (fixLeftEdge ? width / barSpacing : min(2, N))` where `MinVisibleBarsCount = 2`.
- **`_maxRightOffset()`:** `fixRightEdge ? 0 : (width / barSpacing) - min(2, N)`.

---

## 4. Width Resize (`setWidth`, `time-scale.ts:507–549`)

1. Guard: if `!isFinite(newWidth) || newWidth <= 0 || _width === newWidth`, return.
2. Capture `previousVisibleRange = visibleLogicalRange()` and `oldWidth = _width`.
3. **Update `_width = newWidth` FIRST** (before any clamping).
4. If `options.lockVisibleTimeRangeOnResize && oldWidth !== 0`: `_barSpacing = _barSpacing * newWidth / oldWidth`.
5. If `options.fixLeftEdge && previousVisibleRange !== null && previousVisibleRange.left() <= 0`:
   `_rightOffset -= Math.round((oldWidth - newWidth) / _barSpacing) + 1`.
6. **Always call** `_correctBarSpacing()` followed by `_correctOffset()`.

---

## 5. Visible Ranges, `setVisibleRange`, and `fitContent`

### 5.1 `visibleLogicalRange` & `visibleStrictRange` (`time-scale.ts:1023–1042`, `time-scale-visible-range.ts:11–20`)

If `isEmpty()`, both return `null`. Otherwise:
$$\text{barsLength} = \frac{\text{width}}{\text{barSpacing}}, \qquad \text{rightBorder} = \text{baseIndex} + \text{rightOffset}, \qquad \text{leftBorder} = \text{rightBorder} - \text{barsLength} + 1$$
- **Logical range (continuous `f64`):** `[leftBorder, rightBorder]`.
- **Strict bar range (integer indices):** `[floor(leftBorder), ceil(rightBorder)]`.
- **Data slice range (for O(1) array culling):** Intersect `[floor(leftBorder), ceil(rightBorder)]` with `[0, total_bars - 1]`. If `ceil(rightBorder) < 0` or `floor(leftBorder) >= total_bars`, 0 data bars are visible.

### 5.2 `setVisibleRange` & `fitContent` (`time-scale.ts:878–905`)

- **`setVisibleRange(range, applyDefaultOffset)`:**
  - `length = range.right - range.left + 1` (requires `range.left <= range.right`).
  - `pixelOffset = (applyDefaultOffset && options.rightOffsetPixels) || 0`.
  - `_setBarSpacing((width - pixelOffset) / length)`.
  - `_rightOffset = range.right - baseIndex`.
  - If `applyDefaultOffset`: `_rightOffset = pixelOffset ? (pixelOffset / barSpacing) : options.rightOffset`.
  - `_correctOffset()`.
- **`fitContent()`:**
  - If `firstIndex === null || lastIndex === null`, return.
  - `rightOffsetBars = (!options.rightOffsetPixels && options.rightOffset) || 0`.
  - Calls `setVisibleRange(RangeImpl(firstIndex, lastIndex + rightOffsetBars), true)`.

---

## 6. Vertical Coordinate System (`PriceScale`)

### 6.1 Margins & Internal Height (`price-scale.ts:431–439, 911–921`)

Default `scaleMargins` in LWC (`price-scale-options-defaults.ts:13–16`): `top = 0.2, bottom = 0.1` (requiring `0 <= top <= 1`, `0 <= bottom <= 1`, `top + bottom < 1`).
- When `!invertScale`:
  $$\text{topMarginPx} = m_{\text{top}} \cdot H + \text{marginAbovePx}, \qquad \text{bottomMarginPx} = m_{\text{bottom}} \cdot H + \text{marginBelowPx}$$
- When `invertScale`: `topMarginPx` and `bottomMarginPx` swap roles (`L911–921`).
- `internalHeight = H - topMarginPx - bottomMarginPx`.

### 6.2 `priceToCoordinate` & `coordinateToPrice` (`price-scale.ts:473–481, 565–578, 934–959`)

Let `range = [P_min, P_max]` be the current price range stored in **mode-transformed logical space** (unpadded!).
1. Transform raw `price` into logical space (`transform(price, baseValue)`):
   - `Normal`: `logical = price`
   - `Logarithmic`: `logical = toLog(price, logFormula)`
   - `Percentage`: `logical = toPercent(price, baseValue)`
   - `IndexedTo100`: `logical = toIndexedTo100(price, baseValue)`
2. Guard degenerate state (`H <= 0`, `internalHeight <= 1`, or `P_max - P_min <= 0`).
3. Project to pane-local `y_local`:
   $$\text{invY} = \text{bottomMarginPx} + (\text{internalHeight} - 1) \cdot \frac{\text{logical} - P_{\min}}{P_{\max} - P_{\min}}$$
   $$y_{\text{local}} = \begin{cases} \text{invY} & \text{if } \text{invertScale} \\ H - 1 - \text{invY} & \text{if } !\text{invertScale} \end{cases}$$
4. Inverse (`coordinateToPrice`):
   $$\text{invY} = \begin{cases} y_{\text{local}} & \text{if } \text{invertScale} \\ H - 1 - y_{\text{local}} & \text{if } !\text{invertScale} \end{cases}$$
   $$\text{logical} = P_{\min} + (P_{\max} - P_{\min}) \cdot \frac{\text{invY} - \text{bottomMarginPx}}{\text{internalHeight} - 1}$$
   then apply inverse mode transform (`fromLog` / `fromPercent` / `fromIndexedTo100`).

**Boundary Invariants (`!invertScale`, `marginAbovePx = marginBelowPx = 0`):**
- `priceToCoordinate(raw_max) = topMarginPx`
- `priceToCoordinate(raw_min) = H - 1 - bottomMarginPx`

### 6.3 Logarithmic, Percentage, and IndexedTo100 Conversions (`price-scale-conversions.ts:8–130`)

- **Default `LogFormula`:** `{ logicalOffset: 4.0, coordOffset: 0.0001 }`.
- **Dynamic `logFormulaForPriceRange(rawRange)` (`L108–126`):**
  If `rawRange !== null` and `1e-15 <= |max - min| < 1.0`:
  $$d = \lceil |\log_{10}(\Delta)| \rceil, \qquad \text{logicalOffset} = 4 + d, \qquad \text{coordOffset} = 10^{-\text{logicalOffset}}$$
- **`toLog(price, f)` (`L52–60`):**
  If `|price| < 1e-15`, return `0.0`. Let `res = log10(|price| + f.coordOffset) + f.logicalOffset`. Return `price < 0 ? -res : res`.
- **`fromLog(logical, f)` (`L62–70`):**
  If `|logical| < 1e-15`, return `0.0`. Let `res = 10^(|logical| - f.logicalOffset) - f.coordOffset`. Return `logical < 0 ? -res : res`.
- **`toPercent(v, base)` & `fromPercent(pct, base)` (`L13–24`):**
  `base` is the first visible bar value of the series (`firstValue`, dynamic on scroll!). Guard `|base| < 1e-15`.
  `res = 100 * (v - base) / base`; if `base < 0`, negate `res`. Inverse: if `base < 0`, negate `pct`; return `(pct / 100) * base + base`.
- **`toIndexedTo100(v, base)` & `fromIndexedTo100(idx, base)` (`L32–44`):**
  Exact invertible form across positive and negative `base`: `toPercent(v, base) + 100.0`, with inverse `fromPercent(idx - 100.0, base)`. *(Note: LWC `L42–43` has a minor sign quirk when `base < 0` because it negates `(result + 100)` instead of `result` before `+ 100`; using `toPercent(v, base) + 100.0` preserves exact invertibility for all non-zero `base` while matching LWC identically for all `base > 0`.)*

### 6.4 Price Autoscale & Manual Vertical Scroll/Scale (`price-scale.ts:671–778, 967–1076`)

- **Autoscale (`_recalculatePriceRangeImpl`):**
  1. Collect finite `[min, max]` of visible series slices within `visibleStrictRange()`. Do **not** add fractional margins to `priceRange` (margins live in `topMarginPx` / `bottomMarginPx`).
  2. If `min == max` (flat-price candle/range): expand in **raw linear space** by `±5 * minMove` (`min - 5 * minMove`, `max + 5 * minMove`).
  3. In `Logarithmic` mode: update `logFormula = logFormulaForPriceRange(rawRange)` and store `[toLog(rawMin, logFormula), toLog(rawMax, logFormula)]`.
  4. In `Percentage` / `IndexedTo100` mode: transform `[rawMin, rawMax]` using `firstValue`.
- **Vertical Axis Drag-Scale (`startScale` / `scaleTo`, `L671–724`):**
  Disabled in `Percentage` / `IndexedTo100`. Sets `autoScale = false`. Let `y_inv = max(0, H - y_local)` and `y0_inv = H - start_y_local`:
  $$\text{coeff} = \max\!\left(0.1,\; \frac{y_{0,\text{inv}} + 0.2(H - 1)}{y_{\text{inv}} + 0.2(H - 1)}\right)$$
  Scales snapshot logical `[min, max]` around its midpoint `(min + max) * 0.5`.
- **Vertical Pan (`startScroll` / `scrollTo`, `L726–778`):**
  Active when `!autoScale`. Shifts snapshot logical `[min, max]` by `delta = (invertScale ? -1 : 1) * (y_local - start_y_local) * range.length() / (internalHeight - 1)`.

---

## 7. Kinetic Scrolling (`src/model/kinetic-animation.ts` & `src/gui/pane-widget.ts`)

- **Coordinate Unit (`VERIFIED` at `pane-widget.ts:780–786, 807`):** Operates on **`rightOffset` (logical bar units)**, NOT raw pixels:
  - `minSpeed = 0.2 / barSpacing` (bars/ms)
  - `maxSpeed = 7.0 / barSpacing` (bars/ms)
  - `dumpingCoeff = 0.997` (per ms)
  - `minMove = 15.0 / barSpacing` (bars)
  - `MaxStartDelay = 50` ms, `EpsilonDistance = 1.0` *(Note: upstream `kinetic-animation.ts` uses literal `1.0` in bar units, whereas Sumi's `KineticAnimation::for_bar_spacing` scales `epsilon_distance_px / barSpacing` (`1.0 / barSpacing` bars) so inertial coasting settles cleanly at `1 px` precision even when zoomed in with large `barSpacing`)*
- **Sampling (`addPosition(pos, time_ms)`):**
  - If `pos1.time == time_ms`: overwrite `pos1.position = pos` and return (prevents `0 / 0` division!).
  - If `|pos1.position - pos| < minMove`: ignore jitter.
  - Shift 4-sample ring buffer `(pos1 -> pos2 -> pos3 -> pos4)`.
- **Launch (`start(pos, time_ms)`):**
  - Requires at least 2 samples AND `time_ms - pos1.time <= 50.0` ms (`MaxStartDelay`).
  - Up to 3 consecutive segments `(1->2)`, `(2->3)`, `(3->4)` are combined **only while `sign(speed_k) == sign(speed_1)`**, each clamped to `±maxSpeed`, weighted by segment distance `d_k / sum(d)`.
  - If `|v_0| < minSpeed`, animation does not start.
  - Duration `T = max(0, ln((EpsilonDistance * ln(k)) / -|v_0|) / ln(k))` where `k = 0.997`.
  - Position at elapsed `dt = min(t - t_0, T)`:
    $$\text{rightOffset}(t) = \text{startOffset} + v_0 \cdot \frac{k^{\Delta t} - 1}{\ln k}$$

---

## 8. Candlestick Width, Wick Parity & Conflation (`optimal-bar-width.ts`, `candlesticks-renderer.ts`, `time-scale.ts`)

1. **`optimalCandlestickWidth(barSpacing, pixelRatio)` (`optimal-bar-width.ts:5–19`):**
   - For `2.5 <= barSpacing <= 4.0`: `floor(3.0 * pixelRatio)`.
   - Otherwise: `coeff = 1.0 - 0.2 * atan(max(4.0, barSpacing) - 4.0) / (0.5 * PI)`, returning `max(floor(pixelRatio), min(floor(barSpacing * coeff * pixelRatio), floor(barSpacing * pixelRatio)))`.
   - **Symmetric Parity Adjustment (`candlesticks-renderer.ts:54–59`):** With `wickWidth = floor(pixelRatio)` (clamped by `barSpacing`), if `barWidth >= 2` and `(wickWidth % 2) != (barWidth % 2)`, decrement `barWidth -= 1` so the candle body extends an equal integer number of pixels on both sides of the centered wick and crosshair!
2. **Power-of-2 Data Conflation (`time-scale.ts:1076–1098`):**
   - When `barSpacing < (1.0 / devicePixelRatio) * smoothingFactor`, `conflationFactor = min(512, 2^floor(log2(threshold / barSpacing)))`, merging `conflationFactor` consecutive OHLCV bars (`open = first.open`, `high = max(high)`, `low = min(low)`, `close = last.close`, `volume = sum(volume)`).
