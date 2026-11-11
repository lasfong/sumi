# CSV/JSON OHLCV DATA LOADER & MULTI-TIMEFRAME AGGREGATION SPECIFICATION (`MILESTONE R06 / WAVE 5`)

- **Session Identity:** `SESSION-R06-20261111T194303+0700` (`SUMI PROGRAM ORCHESTRATOR`)
- **Research Gate Status:** `VERIFIED` (`DEC-005` Cleared — Live REA MCP + LWC v5.2.0 Apache 2.0 + Legacy Sumi Archive)
- **Authorized Reference Targets:**
  1. **Lightweight Charts v5.2.0** (`/Users/mizuhara/workspace/sumi/research_repos/lightweight-charts-5.2.0/src/{model/data-layer.ts,model/data-consumer.ts,model/data-validators.ts,model/horz-scale-behavior-time/horz-scale-behavior-time.ts,model/horz-scale-behavior-time/time-scale-point-weight-generator.ts,model/horz-scale-behavior-time/time-utils.ts,formatters/time-formatter.ts}`, commit `868cae27bd1acafa0128d8d868ea740a59ae42ce`, `Apache-2.0`)
  2. **Legacy Sumi Archive** (`/Users/mizuhara/workspace/sumi/ARCHIVE/frontend/src/{types/candle.ts,api/importApi.ts,pages/ImportPage.tsx,components/chart/SeriesManager.ts,components/chart/ChartKernel.ts,utils/date.ts,utils/formatters.ts}`, `/Users/mizuhara/workspace/sumi/ARCHIVE/backend/app/services/{cafef_importer.py,import_classifier.py,import_workflow_service.py,weekly_aggregator.py}`, `/Users/mizuhara/workspace/sumi/ARCHIVE/test-results/p0b/data/CafeF.INDEX.clean.csv`, and `/Users/mizuhara/workspace/sumi/ARCHIVE/research_repos/lightweight-charts-drawing/demo/SPY.csv`)
  3. **Compliance Boundary (`DEC-006`):** Proprietary TradingView Desktop was **not** inspected, hooked, or queried.

---

## 1. Live REA MCP Execution Evidence (`Milestone R06`)

All REA operations below were executed live via the `rea-agents@6.8.0` MCP server during `SESSION-R06-20261111T194303+0700`:

| Op ID | REA Tool & Query | Target Scope | Actual Verified Output | Exact 64-Hex Evidence ID |
| :--- | :--- | :--- | :--- | :--- |
| `REA-R06-01` | `analyze_javascript_application` (`detail: "summary"`) | `ARCHIVE/frontend` (`7` TS/TSX files: `candle.ts`, `importApi.ts`, `ImportPage.tsx`, `SeriesManager.ts`, `ChartKernel.ts`, `date.ts`, `formatters.ts`) | Root artifact SHA-256 `243b2d6eaac9d00e0cd2e2623da5ea5f84bb751ad89e2ba40a04be7f7b794b92`; `7` files parsed (`0` failures), `61,774` bytes, `8,580` AST nodes, `121` findings, `1,967` `data-flow`, `331` `object-flow`, `151` `closure`, `7` `call-flow`, `7` `promise-ownership`, `5` `boundary` relations (`10,272/10,272` view items). | Parent: `ev_c3081a8a63abede70c0c45b1279f31e25d794b0298ebc7d42edffa2500fe4d1d`<br>Summary: `ev_3e08300a8529df255cfbcd35d7ee133a9042a7cea92f819ad0bdd1c1cc86f51e` |
| `REA-R06-02` | `analyze_javascript_application` (`detail: "summary"`) | `research_repos/lightweight-charts-5.2.0` (`7` TS files: `data-layer.ts`, `data-consumer.ts`, `data-validators.ts`, `horz-scale-behavior-time.ts`, `time-scale-point-weight-generator.ts`, `time-utils.ts`, `time-formatter.ts`) | Root artifact SHA-256 `6a26c4adafe9539017d004f611425bc9692a278cd45d4a41ffe381afd7b6c9d5`; `7` files parsed (`0` failures), `46,072` bytes, `5,618` AST nodes, `158` findings, `1,689` `data-flow`, `234` `object-flow`, `147` `call-flow`, `142` `closure`, `4` `configuration` relations (`1,148/1,148` view items). | Parent: `ev_a4dabc0086039011ac0c37e397c803758acf198d1d65d35fdaa47ff48cd004b6`<br>Summary: `ev_d57067b972e9e6e113af217c7ec3dea155b9c70bcb9cdb29a95fbd01e1708254` |
| `REA-R06-03` | `trace_javascript_semantics` (`property: "candleData"`, `backward-provenance`) | Parent `ev_c3081a8a63abede70c0c45b1279f31e25d794b0298ebc7d42edffa2500fe4d1d` (`ARCHIVE/frontend`) | `4` seed matches, `7` traversed nodes, `3` relations across `3` functions in `src/components/chart/SeriesManager.ts` (`setCandles`, `refreshCandles`, `updateCandle` using $O(N)$ `findIndex`). | `ev_ab7d475a7835ed979da49e0e58d3fe708a56015ba787d68e29fae78744bc138b` |
| `REA-R06-04` | `trace_javascript_semantics` (`property: "timeframe"`, `forward-influence`) | Parent `ev_c3081a8a63abede70c0c45b1279f31e25d794b0298ebc7d42edffa2500fe4d1d` (`ARCHIVE/frontend`) | `3` seed matches, `3` traversed nodes across `3` functions in `src/pages/ImportPage.tsx`, proving `timeframe` was only an HTTP form string with zero client-side rollup. | `ev_288c5960ac4ce12f180a5e0646baa8c56a4392b365691e392b4de64d3a3db13f` |
| `REA-R06-05` | `trace_javascript_semantics` (`property: "aggregateTimeframe"`, `forward-influence`) | Parent `ev_c3081a8a63abede70c0c45b1279f31e25d794b0298ebc7d42edffa2500fe4d1d` (`ARCHIVE/frontend`) | `0` seed matches (`0` traversed nodes/relations) — negative control confirming legacy Sumi lacked client-side multi-timeframe OHLCV aggregation. | `ev_3a7da5258ea0e22ce5f639766aab490cbdbbd9c8673e3e672a5d0aedea47b8cf` |
| `REA-R06-06` | `trace_javascript_semantics` (`property: "timestamp"`, `forward-influence`) | Parent `ev_a4dabc0086039011ac0c37e397c803758acf198d1d65d35fdaa47ff48cd004b6` (`LWC v5.2.0`) | `10` seed matches, `15` traversed nodes, `5` relations across `4` functions in `horz-scale-behavior-time.ts` and `time-scale-point-weight-generator.ts` (`fillWeightsForPoints` UTC boundary hierarchy). | `ev_eeaad96e25f0bfbd51c4978ea247b8a3e4d18e69b445f069e5b99a8ad365f48a` |
| `REA-R06-07` | `trace_javascript_semantics` (`property: "businessDay"`, `forward-influence`) | Parent `ev_a4dabc0086039011ac0c37e397c803758acf198d1d65d35fdaa47ff48cd004b6` (`LWC v5.2.0`) | `1` seed match in `src/model/horz-scale-behavior-time/horz-scale-behavior-time.ts:116` (`businessDay` vs `UTCTimestamp` formatting branch). | `ev_64453400e51d765046ae02e3c20feece6a10f159adda8e80c3ae19a8e59b892c` |

### Six Legacy Data & Timeframe Defects Discovered & Rectified in Rust (`src/data/`)
1. **No JSON Loader & Rigid 7-Column CafeF CSV Requirement (`cafef_importer.py:176–198`):** Legacy backend rejected JSON files, missed `time` headers (`SPY.csv`), and threw `ValueError` when `symbol` or `volume` columns were omitted (`SPY.csv` has 5 columns `time,open,high,low,close`). **Rust Fix:** Zero-dependency RFC-4180 CSV + RFC-8259 JSON loaders with flexible header alias matching (`time`/`timestamp`/`date`/`datetime`/`<dtyyyymmdd>`, optional `symbol`/`ticker`, optional `volume` defaulting to `0.0`) and automatic multi-symbol isolation.
2. **Destructive Intraday Truncation (`cafef_importer.py:200–206`, `import_classifier.py:195–221`, `utils/date.ts:1–11`, `ChartKernel.ts:87–91`):** Both legacy backend (`.dt.date`) and frontend (`split('T')[0]`, `timeVisible: false`) stripped `HH:MM:SS`, collapsing hundreds of intraday candles per day into colliding date keys. **Rust Fix:** Full `i64` UTC epoch-second precision across all loaders, `Candle` records, and adaptive intraday/daily time-axis formatting (`HH:MM` / `MM-DD HH:MM` vs `YYYY-MM-DD`).
3. **Hardcoded Weekend Rejection (`import_classifier.py:223–227`):** Legacy rejected `weekday >= 5`, blocking 24/7 crypto (`BTCUSDT`) and UTC-shifted Friday US/FX afternoon bars. **Rust Fix:** Accept all valid Gregorian calendar timestamps (`Mon..Sun`).
4. **Missing Intraday Aggregation & `DEF-001` Weekly Lookahead (`import_classifier.py:89`, `weekly_aggregator.py:125–126`, `test_def001_timeframe_isolation.py`):** Legacy allowed only `1D`/`1W`, stamped `1W` bars at Friday close instead of Monday open, and leaked Friday High/Close during mid-week replay. **Rust Fix:** Full upward lattice aggregation across `1m`, `5m`, `15m`, `1h`, `4h`, `1D`, `1W`, stamping ISO-8601 Monday `00:00:00 UTC` on `1W` buckets (immune to `SPY.csv` `14:30`/`13:30 UTC` DST shifts) and providing causal prefix rollup `aggregate_ohlcv_up_to`.
5. **Production Validator Bypass in LWC (`data-validators.ts:19–21, 37–39, 63–85`):** LWC skips `checkItemsAreOrdered` and `checkBarItem` in production and never checks `low <= min(open, close) <= max(open, close) <= high`. **Rust Fix:** Mandatory O(N log N) stable chronological sort, deterministic deduplication (`KeepLast` / `KeepFirst` / `RejectConflict`), and explicit `OhlcEnvelopePolicy` (`RejectRow` / `ClampToBody`) inside `normalize_loaded_candles` before any chart state sees the slice.
6. **Drawing & Viewport Drift Across Timeframe Switches:** Switching `5m -> 1h -> 1D -> 5m` on bar-indexed drawings without timestamp anchoring shifts every annotation. **Rust Fix:** Optional `DrawingAnchor::timestamp` (`f64` UTC seconds) + `DrawingStore::remap_timeframe` and `ChartViewportState` anchor-timestamp preservation keep annotations and viewport focus locked to exact market timestamps across arbitrary timeframe round-trips.

---

## 2. Module Architecture & Algorithms (`src/data/`)

All modules in `src/data/` use **zero external parser/date crates** (`Cargo.toml` unchanged) and **zero OS dependencies** in core logic (`wasm32-unknown-unknown` verified; `std::fs` convenience file reader gated behind `#[cfg(not(target_arch = "wasm32"))]`).

### 2.1 Proleptic Gregorian UTC Calendar & Timestamp Parsing (`src/data/calendar.rs`)
- **Exact Civil <-> Epoch Day Bijection (Howard Hinnant algorithm):**
  - `days_from_civil(year: i32, month: u32, day: u32) -> i64` (`1970-01-01` $\mapsto 0$) and `civil_from_days(days: i64) -> (i32, u32, u32)`, valid across years `0001..=9999` (`[-62_135_596_800 ..= 253_402_300_799]` UTC seconds).
  - `weekday_from_days(days: i64) -> u8`: `(days + 3).rem_euclid(7) as u8` (`0 = Mon .. 6 = Sun`).
- **Supported Timestamp Representations (`parse_timestamp`, `parse_time_of_day_hms`, `resolve_series_timestamps`):**
  1. **CafeF 8-digit `YYYYMMDD` & 8-Column Intraday `<DTYYYYMMDD>,<Time>`:** Explicit `DateYmd` column (with optional companion `Time` column supporting `HH:MM[:SS[.fff|,fff]][Z|±TZ]`, `HHMMSS`, `HMMSS`, `HHMM` via `parse_time_of_day_hms`), or series-wide all-or-nothing 8-digit `1900..=2999` `YYYYMMDD` string detection (`resolve_series_timestamps`) that never hijacks numeric JSON or mixed 1970 Unix-second series.
  2. **ISO-8601 Date** (`YYYY-MM-DD` or `YYYY/MM/DD`) $\rightarrow$ `00:00:00 UTC` (`has_time_component = false`).
  3. **ISO-8601 Date-Time** (`YYYY-MM-DD[T ]HH:MM[:SS[.fff|,fff]][Z|±HH:MM|±HHMM]`) $\rightarrow$ exact UTC seconds (`local_epoch - tz_offset_secs`, `has_time_component = true`), validating `month ∈ 1..=12`, `day ∈ 1..=days_in_month(year, month)`, `hour ∈ 0..=23`, `minute ∈ 0..=59`, `second ∈ 0..=59` (with `:60` restricted strictly to `23:59:60 UTC` leap seconds via `validate_leap_or_normal_second`), and `|tz_offset| < 86_400`.
  4. **Lossless Arbitrary-Precision Numeric Unix Epoch (`TimestampUnit`):**
     - Uses exact digit-level integer scaling (`split_decimal_parts` / `parse_decimal_epoch_token`, zero `f64` precision loss on microsecond/nanosecond decimals) and infers **one uniform `TimestampUnit` per series** (`AUTO_EPOCH_SECONDS_CAP = 10_000_000_000` so 1973–2286 ms timestamps classify cleanly as `Milliseconds`, plus near-1970 `1000 * k` delta disambiguation for pre-1970 / early-1970 ms series). Explicit `Seconds` / `Milliseconds` / `Microseconds` / `Nanoseconds` overrides are also supported.

### 2.2 RFC-4180 CSV & RFC-8259 JSON OHLCV Loaders (`src/data/csv_loader.rs`, `src/data/json_loader.rs`)
- **Resource & Input Bounds:**
  - `MAX_IMPORT_BYTES = 64 * 1024 * 1024` (`64 MiB`, enforced via single-handle `file.take(MAX_IMPORT_BYTES + 1)` in `load_ohlcv_file`), `MAX_IMPORT_RECORDS = 500_000`, `MAX_SCANNED_RECORDS = 2_000_000`, `MAX_DISCOVERED_SYMBOLS = 256` ($O(1)$ `SymbolTracker`), `MAX_JSON_DEPTH = 16`.
  - Strips optional UTF-8 BOM (`\u{FEFF}`), handles both `LF` (`\n`) and `CRLF` (`\r\n`) (`CafeF.INDEX.clean.csv`), auto-detects delimiter (`,`, `;`, `\t`) across quote-aware multiline RFC-4180 headers, and parses RFC-4180 double-quoted fields (`""` escapes, embedded delimiters).
- **CSV Header & Multi-Symbol Handling:**
  - Strips surrounding `<>` brackets (`<Ticker>`, `<DTYYYYMMDD>`, `<Time>`, `<Open>`, ...), collapses multiline/whitespace separators, and maps case-insensitively:
    - `symbol` / `ticker` / `instrument` / `code` / `ticker_symbol` $\rightarrow$ `Symbol` (optional)
    - `dtyyyymmdd` / `yyyymmdd` $\rightarrow$ `DateYmd`
    - `time` / `timestamp` / `date` / `datetime` / `open_time` / `unix` / `trade_date` / `trading_date` / `trade_time` $\rightarrow$ `Timestamp` (combined as time-of-day when paired with `DateYmd`)
    - `open` / `o` / `open_price`, `high` / `h` / `high_price`, `low` / `l` / `low_price`, `close` / `c` / `close_price` (required; `adj close` never overwrites `close`)
    - `volume` / `vol` / `v` (optional; defaults to `0.0` when absent, e.g. `SPY.csv`)
  - When a CSV/JSON contains multiple symbols (e.g. `CafeF.INDEX.clean.csv` with `HNX-INDEX` + `VNINDEX`), `symbol_filter` selects the requested symbol (or defaults to the first symbol encountered) while recording discovered symbols in `DataLoadReport::available_symbols` so distinct instruments are **never** interleaved into one series.
- **JSON Schema Variants Supported (`load_ohlcv_json`):**
  1. Top-level array of candle objects: `[{"time": ..., "open": ..., "high": ..., "low": ..., "close": ..., "volume": ...}, ...]`
  2. Top-level array of OHLC[V] tuples: `[[timestamp, open, high, low, close, optional_volume], ...]` (standard exchange kline format; string or numeric elements)
  3. Envelope object: `{"symbol": "BTCUSDT", "timeframe": "15m", "candles" | "data" | "klines" | "bars" | "ohlcv" | "results": [...]}` (only filtering per-row symbols by outer `envelope_symbol` when `env_sym_matches_any_row` holds).
- **Normalization Pipeline (`normalize_loaded_candles`):**
  - Validates finite domain (`|price| <= 1.0e15`, `price > 0.0` when `require_positive_prices`, `0.0 <= volume <= 1.0e18`), applies optional `price_multiplier` (finite `> 0.0`), enforces `OhlcEnvelopePolicy` (`RejectRow` default vs `ClampToBody`), stably sorts ascending by `(timestamp, row_ordinal)`, deduplicates equal timestamps (`DuplicateTimestampPolicy::{KeepLast, KeepFirst, RejectConflict}` with scale-relative `rel_identical` tolerance), infers native `base_timeframe` from lower-quartile positive `Δtimestamp` (defaulting single-bar series to `D1`) or canonicalizes intra-bucket rows when an explicit coarser `base_timeframe` is supplied, and formats canonical `date_label` (`"YYYY-MM-DD"` for `>= 1D`, `"YYYY-MM-DD HH:MM"` for intraday).

### 2.3 Multi-Timeframe Aggregation & Timestamp Remapping (`src/data/timeframe.rs`, `src/drawings/`, `src/app.rs`)
- **Supported Timeframe Lattice (`Timeframe`):**
  `M1 (60s)` $\prec$ `M5 (300s)` $\prec$ `M15 (900s)` $\prec$ `H1 (3600s)` $\prec$ `H4 (14400s)` $\prec$ `D1 (86400s)` $\prec$ `W1 (604800s)`.
- **Bucket Alignment (`Timeframe::bucket_start_timestamp`):**
  - Intraday (`M1..H4`, $S = \text{tf.seconds()}$): $b(t) = t.\text{div\_euclid}(S) \times S$.
  - Daily (`D1`): $b(t) = t.\text{div\_euclid}(86\_400) \times 86\_400$.
  - Weekly (`W1`, ISO-8601 Monday `00:00:00 UTC`):
    $$d = t.\text{div\_euclid}(86\_400), \quad w = (d + 3).\text{rem\_euclid}(7), \quad b(t) = (d - w) \times 86\_400$$
- **Upward Rollup Invariants (`aggregate_ohlcv` / `aggregate_ohlcv_up_to`):**
  - Rejects downsampling (`target.seconds() < source.seconds()`) with `AggregationError::CannotDownsample`.
  - Identity (`target == source`) normalizes bucket labels/timestamps only when needed; upward rollup computes `open = first.open`, `high = max(high)`, `low = min(low)`, `close = last.close`, `volume = sum(volume)` in $O(N)$ single pass.
  - Associativity invariant: `aggregate(aggregate(c_1m, 1m, 15m), 15m, 1h) == aggregate(c_1m, 1m, 1h)` and `aggregate(aggregate(c_15m, 15m, 1D), 1D, 1W) == aggregate(c_15m, 15m, 1W)`.
- **Timestamp-Locked Drawing, Per-Instrument Isolation & Viewport Continuity:**
  - `DrawingAnchor` carries optional `timestamp: Option<f64>` (UTC seconds). On timeframe switch `(old_candles, old_tf) -> (new_candles, new_tf)`, `DrawingStore::remap_timeframe` resolves each anchor's UTC timestamp once and maps it onto `new_candles` via $O(\log N)$ `remap_timestamp_to_bar_index` — snapping coarse `1W`/`1D` bucket-start timestamps (`00:00 UTC`) cleanly to the first actual session candle inside that bucket when switching downward (`1W -> 1m` / `1W -> 1D`), while preserving sub-bar `5m -> 1W -> 5m` anchors to `< 1e-9` bars.
  - `SumiWorkstationApp` isolates `DrawingStore` per instrument (`drawings_by_instrument`), restores persisted `eframe` sessions directly at `active_timeframe` via `restore_persisted_session` before binding `TimeScale` or `DrawingStore`, and locks viewport temporal focus via `TimeScale::local_x_to_center_float_index`.

---

## 3. Adversarial `sumi-critic` Review & Remediation Log

Following initial Wave 5 implementation (`65/65` tests), `sumi-critic` conducted an independent code-level adversarial audit across `src/data/*`, `src/drawings/*`, `src/chart.rs`, `src/app.rs`, and `examples/viewport_bench.rs`, identifying **17 findings** (`3 CRITICAL`, `6 HIGH`, `5 MEDIUM`, `3 LOW`). **All 17/17 findings (`100%`) were remediated and locked down with dedicated `critic_r06_regression_*` unit tests (`72/72` tests passing):**

| ID | Severity | Finding Summary | Verified Remediation (`src/` & `tests`) |
| :--- | :--- | :--- | :--- |
| `CRITICAL-01` | `CRITICAL` | `eframe` session restore loaded only default `SyntheticVnindex` `120B` `1D` candles instead of persisted `data_source` + `active_timeframe`, shifting `right_offset` and double-remapping `4h`/`1W` drawings. | Implemented `SumiWorkstationApp::restore_persisted_session` (`src/app.rs`), loading the persisted preset/custom file, aggregating `report.candles -> persisted_tf` *before* `TimeScale::set_total_bars`, stamping drawings in-place at `resolved_tf`, and falling back cleanly on missing files. |
| `CRITICAL-02` | `CRITICAL` | 8-column intraday CafeF CSVs (`<Ticker>,<DTYYYYMMDD>,<Time>,...`) skipped `<Time>` or threw duplicate time column error. | Added `parse_time_of_day_hms` (`HH:MM[:SS[.fff|,fff]][Z|±TZ]`, `HHMMSS`, `HMMSS`, `HHMM`) in `src/data/calendar.rs` and combined `(Some(ymd_idx), Some(time_idx))` in `src/data/csv_loader.rs`. |
| `CRITICAL-03` | `CRITICAL` | Per-row `Auto` timestamp heuristic hijacked 1970 epoch seconds (`19_710_101..=21_991_231`) as `YYYYMMDD`, treated pre-1970/1975 ms as seconds (`1e12` cutoff), and allowed mixed units in one series. | Implemented `resolve_series_timestamps` (`src/data/calendar.rs`): removed `YYYYMMDD` from `parse_epoch_i128`, enforced all-or-nothing `1900..=2999` 8-digit string `YYYYMMDD`, lowered `AUTO_EPOCH_SECONDS_CAP` to `10_000_000_000`, and added `1000 * k` step disambiguation for near-1970 ms series. |
| `HIGH-01` | `HIGH` | Drawings created directly on `1W` (`Monday 00:00 UTC`) extrapolated backward into weekend/holiday gaps when switching down to `1m` or `SPY 1D`. | Added `remap_timestamp_to_bar_index` (`src/data/timeframe.rs`), snapping exact coarse bucket-start timestamps (`source_tf > target_tf`) to the first actual session bar inside the `[bucket_start, bucket_end)` window. |
| `HIGH-02` | `HIGH` | `set_timeframe` used `plot_w * 0.5 / bs` + `local_x_to_float_index` (`+0.5` bar drift), ignored sparse `1W` historical pans (`right_offset > 0`), and kept stale `viewport_focus_lock` after zoom/pan. | Switched `set_timeframe` (`src/app.rs`) to `TimeScale::local_x_to_center_float_index` + `(plot_w - 1.0 - center_local_x) / bs` (`< 1e-12` bar round-trip), detected sparse `1W` historical center pans, and cleared `viewport_focus_lock` on `zoom_in`, `zoom_out`, and interactive canvas pan/zoom. |
| `HIGH-03` | `HIGH` | Switching symbols/presets left previous ticker's drawings on screen (`VNINDEX` drawings rendered at `1,280` on `BTCUSDT` `64,000`), and `select_symbol_in_source` ignored in-memory multi-symbol docs. | Added per-instrument `drawings_by_instrument` (`src/app.rs`) normalizing outgoing drawings to `base_timeframe` on swap, plus full `in_memory_custom_doc` (`load_ohlcv_bytes`) multi-symbol switching on both native and `wasm32`. |
| `HIGH-04` | `HIGH` | Time-axis cursor pill in future/past whitespace showed `+4m` instead of `+60m` on `15m` (`+3h` instead of `+12h` on `4h`). | Added `Timeframe::unit_multiplier` (`1, 5, 15, 1, 4, 1, 1`) and `format_time_axis_cursor_pill_label` (`src/chart.rs`) multiplying bar offsets by `unit_multiplier`. |
| `HIGH-05` | `HIGH` | `json_loader.rs` missed `"klines"` / `"results"` envelope keys and dropped all rows when outer `"BINANCE:BTCUSDT"` differed from inner `"BTCUSDT"`. | Added `"klines" | "results"` envelope keys and verified `env_sym_matches_any_row` before filtering row symbols by outer `envelope_symbol` (`src/data/json_loader.rs`). |
| `HIGH-06` | `HIGH` | `load_ohlcv_file` had TOCTOU `metadata` + unbounded `read_to_end` if a file grew concurrently past `64 MiB`. | Opened a single `File` handle and read via `file.take((MAX_IMPORT_BYTES as u64) + 1)` (`src/data/mod.rs`). |
| `MEDIUM-01` | `MEDIUM` | `Ray` clamp at `±MAX_ABS_BAR_INDEX` could invert/verticalize direction, and vertical-only handle/body drags wiped `anchor.timestamp`. | Preserved horizontal sign & explicit timestamps on clamped `Ray`s and preserved exact `(bar_index, timestamp)` during pure vertical `DraggingHandle` & `DraggingBody` gestures (`src/drawings/controller.rs`). |
| `MEDIUM-02` | `MEDIUM` | `detect_delimiter` stopped at the first `\n` inside multiline quoted CSV header cells, and `classify_header_name` didn't collapse embedded newlines. | Made `detect_delimiter` quote-aware across multiline headers and normalized internal whitespace/separators in `classify_header_name` (`src/data/csv_loader.rs`). |
| `MEDIUM-03` | `MEDIUM` | `DuplicateTimestampPolicy::RejectConflict` used fixed `1e-9` absolute tolerance (blind at `1e-13` micro-prices, overly strict at `1e12`). | Replaced with scale-aware `rel_identical(a, b, rel_tol)` (`1e-9 * max(1, |a|, |b|)` with `1e-12` relative check for tiny magnitudes) in `src/data/mod.rs`. |
| `MEDIUM-04` | `MEDIUM` | Supplying coarse `explicit_timeframe` (`1W`) on finer rows (`1D`) left multiple bars inside one weekly bucket without aggregating them. | Canonicalized sorted/deduplicated candles via `aggregate_ohlcv` in `normalize_loaded_candles` whenever adjacent candles share a `base_timeframe` bucket (`src/data/mod.rs`). |
| `MEDIUM-05` | `MEDIUM` | `f64` scaling in `parse_numeric_timestamp_token` lost precision on 16–19 digit µs/ns decimals; comma fractions (`00,500Z`) failed; `:60` accepted at `12:00:60`. | Replaced `f64` scaling with exact digit-string floor division (`parse_decimal_epoch_token`), accepted `.` and `,` in `parse_hms_fraction`, and restricted `:60` to `23:59:60 UTC` (`src/data/calendar.rs`). |
| `LOW-01` | `LOW` | Unbounded `available_symbols` $O(S)$ scan & toolbar pill overflow on 200-ticker CSVs; unfiltered rows before `MAX_IMPORT_RECORDS` cap. | Added $O(1)$ `SymbolTracker` (`MAX_DISCOVERED_SYMBOLS = 256`), `MAX_SCANNED_RECORDS = 2_000_000`, and compact `egui::ComboBox` when `available_symbols.len() > 6` (`src/data/mod.rs`, `src/app.rs`). |
| `LOW-02` | `LOW` | Single-bar CSV inferred `M1` instead of `D1`, and median `Δt` over holiday-heavy intraday series could skew upward. | Defaulted `< 2` bars to `Timeframe::D1` and used lower-quartile positive step `deltas[(len - 1) / 4]` (`src/data/timeframe.rs`). |
| `LOW-03` | `LOW` | Conflated crosshair `cross_x` snapped to raw bar instead of `conflated_candle_at(...).center_logical_index`; `Demo Set` silently no-op'd on `< 10` `1W` bars; presets relied on CWD. | Aligned conflated `cross_x` to bucket center (`src/chart.rs`), enabled `populate_showcase_drawings` down to `2` bars (`src/app.rs`), and added `CARGO_MANIFEST_DIR` fixture fallback (`src/data/presets.rs`). |

---

## 4. Measured Release Performance & Binary REA Audit (`Milestone R06`)

### 4.1 Release Benchmark (`cargo run --release --example viewport_bench -- --r06`, Apple M1)
- **Zero-Dependency Streaming CSV & JSON Loader Throughput:**
  - `100,000` ISO-8601 `1m` CSV (`5.53 MiB`): **`45.303 ms`** median (`45.717 ms` P95, **`122.1 MiB/s`**)
  - `100,000` Unix-ms JSON klines (`5.90 MiB`): **`43.703 ms`** median (`44.243 ms` P95, **`134.9 MiB/s`**)
  - Real `CafeF.INDEX.clean.csv` (`2,179` `VNINDEX` bars isolated from multi-ticker `0.25 MiB` file): **`1.302 ms`** median (**`191.1 MiB/s`**)
  - Real `SPY.csv` (`8,280` 5-col NYSE daily bars, `0.32 MiB`): **`3.130 ms`** median (**`103.7 MiB/s`**)
- **`100,000` `1m` Bars Multi-Timeframe Rollup & 25-Drawing Timestamp Remapping:**
  - `1m -> 5m` (`20,000` bars): **`2.613 ms`** rollup | **`4.29 µs`** 25-drawing remap
  - `1m -> 15m` (`6,667` bars): **`1.142 ms`** rollup | **`3.71 µs`** 25-drawing remap
  - `1m -> 1h` (`1,961` bars): **`0.636 ms`** rollup | **`3.25 µs`** 25-drawing remap
  - `1m -> 4h` (`785` bars): **`0.504 ms`** rollup | **`3.04 µs`** 25-drawing remap
  - `1m -> 1D` (`393` bars): **`0.454 ms`** rollup | **`2.83 µs`** 25-drawing remap
  - `1m -> 1W` (`79` bars): **`0.502 ms`** rollup | **`0.75 µs`** 25-drawing remap
- **4-Pane + 6-Indicator + 25-Drawing Intraday & Aggregated Frame Budget (`DECISION-RENDERER-002` `< 4.0 ms` Gate):**
  - `100K` `1m` standard intraday window (`bar_spacing = 8.0`, adaptive `HH:MM` / `MM-DD HH:MM` axis): **`0.185 ms`** median (`0.223 ms` P95 — **PASS**)
  - `100K` `1m` full-fit conflated (`Logarithmic`, `32x` conflated, `418,136` vertices): **`3.761 ms`** median (`4.112 ms` P95 — **PASS `< 4.0 ms` median gate**)
  - `100K 1m -> 15m` (`6,667` bars + `25` remapped drawings, `bar_spacing = 8.0`): **`0.207 ms`** median (`0.242 ms` P95 — **PASS**)
  - `100K 1m -> 1D` (`393` bars + `25` remapped drawings, `Logarithmic` full-fit): **`0.305 ms`** median (`0.334 ms` P95 — **PASS**)

### 4.2 Live REA MCP Release Binary Audit (`target/release/sumi-workstation`)
- **Artifact:** `/Users/mizuhara/workspace/sumi/target/release/sumi-workstation` (`17,336,320` bytes, `arm64` 64-bit Mach-O, SHA-256 `d397be8d0c99356d02f2d116a64c8daf11ac66dc635cbe12a30e6c16f889f92f`, UUID `CE6F8FC6-E08D-36CB-98AB-55B34D1A0CCA`).
- **`inspect_rust_binary` (`detail: "summary"`):**
  - Parent Evidence ID: `ev_2acf1d978d0dd960e9987e42409777c3bbdd76d0f461ed2c792127f8e91e96b1`
  - Summary Evidence ID: `ev_fd142ac6cc9e7ddc300fe9a98a26e8dd6424c0335aa76bde66c90c86f81e9b19`
- **`inspect_macho` (`detail: "summary"`, via `open_binary` / `close_binary`):**
  - Parent Evidence ID: `ev_d3f783641f1d126191676cc6f2f030c14fbd735e15313de6c6794974c6bb6293`
  - Summary Evidence ID: `ev_6c511d740d72d4a49f1828e4d595efbb2c2c4285034fe4251a49dfbaf87ea724`
  - Verified `30` load commands, `14` dylib dependencies (identical to R03/R04/R05 — **zero new crate or system dependencies**), `324` imports, `4,910` exports, and `5` segments (`5,286/5,286` items examined; session cleanly closed).
