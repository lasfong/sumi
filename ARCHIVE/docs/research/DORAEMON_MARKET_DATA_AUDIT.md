# Doraemon Market Data Audit

**Audit date:** 2026-09-12  
**Scope:** cross-repository read-only discovery, provider/API validation, semantic classification and recommendation; no production implementation  
**Repositories:** `E:\Workspace\sumi`, `E:\Workspace\Doraemon`, and candidate `E:\Workspace\Doraemi`  
**Production API:** authenticated read-only requests to `https://api.hieupb.io.vn/api/v1`; no direct workstation connection to the production database

## Executive conclusion

Doraemon is the active production data/backend repository. It adds real KBS daily OHLCV, partially populated actual trading value, recent foreign buy/sell volume, current-session trade prints, a current price board, and point-in-time universe infrastructure. This is materially better than Sumi's local database, but it does **not** currently provide a validated historical aggressor-side series, synchronized historical quotes, historical order books, foreign trading value, proprietary trading, or a sufficiently complete point-in-time whole-market universe.

The highest-integrity Sumi MVP remains an explicitly labelled `OHLCV_PROXY`:

- use Doraemon daily OHLCV and actual matched/trading value only where the row is populated;
- otherwise use the BB specification's explicit estimated-value fallback and publish the value source;
- do not splice proxy and later true/classified flow into an unlabeled continuous series;
- do not call Doraemon's existing price/volume proxy “Blackbox” or “actual money flow”; and
- keep Market BB and Flow Breadth unavailable until survivorship-safe universe coverage is established.

KBS current-session trades make `TICK_TEST_ESTIMATE_RESEARCH_ONLY` technically possible for a bounded live research spike. Its `LC` buy/sell label is `UNKNOWN` until KBS documents the meaning or it is validated against a labeled reference. It is not evidence of `EXECUTED_ORDER_FLOW`.

## 1. Doraemon Architecture/Data Map

### Active repository determination

| Candidate | Evidence | Decision |
| --- | --- | --- |
| `E:\Workspace\Doraemon` | README describes the production data/compute/API service; git remote is `lasfong/Doraemon`; local `main` commit `f860c6430f9cd8965966a0ae445ad497f11fdefc`; production health reports the same release commit | **Active backend/data platform** |
| `E:\Workspace\Doraemi` | Separate `lasfong/Doraemi` repository and service documentation oriented to AI functionality; older unrelated commit | Not the market-data backend |

The Doraemon checkout was not modified. Its only observed working-tree item was pre-existing untracked `login_fail.png`.

### Data and ingestion boundaries

| Concern | Concrete location | Audited behavior |
| --- | --- | --- |
| Provider contract | `E:\Workspace\Doraemon\src\vn_fin_dw\providers\base.py` | Strict fetch → parse → normalize → upsert boundary |
| Daily orchestration | `src\vn_fin_dw\jobs\daily.py` | Provider jobs write normalized records; raw snapshots are retained where a job uses the snapshot contract |
| KBS market prices | `src\vn_fin_dw\providers\kbs\quote.py`, `kbs_provider.py`, KBS quote job | Historical bars plus current-session trade history and board data |
| Foreign-volume enrichment | `src\vn_fin_dw\jobs\foreign_flow_inputs.py` | Updates explicit KBS share volume only when the board date equals the target date |
| Foreign-value bridge | `src\vn_fin_dw\jobs\foreign_flow_value_bridge.py` | Accepts only reviewed/licensed explicit VND observations; it does not synthesize value |
| Market storage | `src\vn_fin_dw\db\models.py` | PostgreSQL/Timescale models for daily prices, foreign-value observations, raw snapshots, calendar/events and universe state |
| Read API | `src\api\market\router.py` | Authenticated market history, flow, diagnostics and provider-independent computation endpoints |
| Existing money-flow computation | `src\vn_fin_dw\services\independent_method_v1.py` | OBV/MFI/AD/CMF/turnover/up-down-volume proxy; API explicitly returns `not_qmv_blackbox` |
| Point-in-time universe | `src\vn_fin_dw\services\effective_universe.py`; universe tables/migrations | Current memberships plus episode history and validation metadata; current production population is curated, not proven complete exchange history |

`market.stock_prices_daily` contains `symbol`, date, OHLC, adjusted close, volume, total trading value, foreign buy/sell volume and foreign buy/sell value. There is no persisted trade-tick, quote/order-book, execution-side, or proprietary-flow table. `market.raw_snapshots` can retain source text and contract/version metadata, but the checkout contains no market payload archive or local production database.

### Actual providers

| Provider | Implementation | Purpose / endpoint family | Historical / realtime | Status |
| --- | --- | --- | --- | --- |
| KBS | `providers\kbs\*`, `kbs_provider.py` | `https://kbbuddywts.kbsec.com.vn/iis-server/investment`; daily/minute bars, board, current trade history | Historical bars; realtime/current-session board and prints | Real, active, no configured secret required |
| VCI IQ | `providers\vci_iq*` | `https://iq.vietcap.com.vn` and GraphQL financial endpoints | Historical financial statements | Real, active for financials; not a flow source |
| FireAnt | `providers\fireant*` | Financial summaries, ratios and dividends | Historical/reference financial data | Real when runtime bearer token is available; not a flow source |
| SBV | `providers\sbv*` | Money-market/macro sources | Historical/current macro | Real, partial; irrelevant to executed equity flow |
| NSO | `providers\nso*` | Vietnam statistics | Historical macro | Real, partial; irrelevant to executed equity flow |
| VBMA | `providers\vbma*` | Bond-market data | Historical/current | Real; irrelevant to equity execution flow |
| World Bank | provider modules/jobs | Public macro series | Historical macro | Real; irrelevant to equity execution flow |
| VSDC | rights/corporate-action modules | Corporate rights/events | Event history | Real/reviewed; useful for adjustment policy, not flow |

No implemented SSI FastConnect, CafeF, TCBS, HOSE/HNX direct-feed, vnstock, or proprietary-trading provider was found in Doraemon. SSI-like names in generic mappings or research documents are not integrations. Test fixtures and fake database/provider objects remain test-only and were not counted as market sources.

No upstream-published request quota was found in the audited provider code or repository documentation. KBS/FireAnt paths implement local pacing and KBS has retry/backoff for rate-limited responses; those safeguards are not evidence of a contractual rate limit or SLA.

## 2. Credential/Provider Status

Only variable names and credential locations were inspected; no secret value was copied into this report.

| Provider/system | Authentication design | Configured status observed | Safe audit result |
| --- | --- | --- | --- |
| Doraemon production read API | `DORAEMON_API_KEY` supplied to the existing client/header mechanism | Available in an existing local project environment and successfully used for read-only GETs | Operational; value not displayed or persisted |
| KBS market data | Browser-like public request headers; no account credential in provider | No secret required | Operational in direct minimal probes |
| VCI IQ | Public/session HTTP behavior in provider | No long-lived market-data secret identified | Existing production registry says active for financials |
| FireAnt | Runtime secret record named `fireant_bearer_token`; guest-token refresh path | Local token value/status intentionally not exposed; production registry says active when configured | No live diagnostic invoked because refresh can write token/auth state |
| SSI FastConnect | Consumer ID/secret → bearer token according to official docs | No SSI credential/configured provider found | Anonymous probe reached service but returned logical status 401, “Missing Authorization header” |
| Database/JWT/application | Environment variables and `.env` loading | Names present as expected | Values not printed; no direct production DB connection attempted |

The repository's tracked-secret scanner passed with zero high-confidence findings. Manual source/config inspection found no hard-coded market-data credential. A provider token refresh that could mutate database state was deliberately excluded from this read-only audit.

## 3. Verified API Test Results

All tests were minimal and read-only. No bulk payload was saved.

### Doraemon production API

| Endpoint purpose | Result | Evidence |
| --- | --- | --- |
| Health/release identity | HTTP success; database connected; deployed release commit matches active Doraemon checkout | Production is the audited code line; identity marked incomplete only because image digest is absent |
| Independent data audit, minimum 1,260 sessions | `degraded` at 2026-09-11 | 487,487 daily price rows; 470 active symbols; zero symbols meet 1,260 days; mean 1,037.2 and minimum 57 days; adjusted close 2,398/487,487 rows (0.49%, two symbols); only five reviewed corporate events |
| Data-source/foreign-flow coverage | `degraded` | Recent audited slice has 25,880 rows; 11,670 rows/465 symbols have foreign volume; **zero** rows have foreign value |
| Long-lookback foreign diagnostics | Success with degraded quality | 487,801 rows/473 symbols checked; 11,670 foreign-volume rows (2.39%); source `kbs`; latest 2026-09-11; zero foreign-value rows |
| Live KBS source diagnostics for FPT/SHS/BSR | Success, `degraded` | Board returns exchange, 3-level bid/ask, total value, put-through quantity/value, foreign buy/sell volume/count/room; recent history returns time/OHLCV/value; no foreign value; nothing persisted by this diagnostic |
| Foreign-value bridge preflight | Success | Zero eligible reviewed observations |
| Existing money-flow endpoint | Success | Explicit `source_type=local_proxy`, `method=money_flow_daily_v1`, `equivalence=not_qmv_blackbox` |

Representative persisted history:

| Instrument | Exchange | Rows / date range returned | Non-null field observations |
| --- | --- | --- | --- |
| FPT | HOSE | 1,239; 2021-09-13 → 2026-08-28 | total value 66; foreign volume 27; foreign value 0; adjusted close 0 |
| SHS | HNX | 1,246; 2021-09-13 → 2026-09-11 | total value 90; foreign volume 28; foreign value 0; adjusted close 0 |
| BSR | UPCoM | 1,238; 2021-09-13 → 2026-09-11 | total value 55; foreign volume 28; foreign value 0; adjusted close 0 |
| VNINDEX | index | 154; 2026-01-26 → 2026-09-11 | no value, investor-flow or adjusted-close fields |

The stale FPT end date is reported as observed, not normalized away. It is a quality issue requiring provider/job investigation before the series is treated as current.

### Direct KBS provider probes

| Endpoint | Result and schema | Frequency/history | Semantic verdict |
| --- | --- | --- | --- |
| `/trade/history/{symbol}` through `Quote.intraday()` | Success for FPT, SHS and BSR; raw keys `t`, `TD`, `SB`, `FT`, `LC`, `FMP`, `FCV`, `FV`, `AVO`, `AVA`; normalized to time, price, volume, `match_type`, synthetic ID, trading date, price change, accumulated volume/value | Current/last session only; no date parameter; pagination pages were distinct; second precision after adapter removes KBS pseudo-millisecond suffix | Real trade prints support `TICK_TEST_ESTIMATE_RESEARCH_ONLY`. `LC` values `b`/`s`/empty are `UNKNOWN`, not proven aggressor side. Synthetic timestamp+price+volume ID may collide. |
| `/stock/matched-by-price/{symbol}` | Success; price, buy/sell/unknown/total volume | Current-session price buckets only | Totals reconciled for all sampled levels. Buy/sell meaning is undocumented in-repo; candidate classified statistic, not executed order flow. |
| Historical quote endpoint | Success | FPT request 2000-01-01 → 2026-09-12 returned 2,570 daily rows, actual 2016-05-30 → 2026-09-11; SHS/BSR and VNINDEX/HNXINDEX/UPCOMINDEX recent seven-day probes succeeded | Daily OHLCV; FPT actual value populated on only 73 returned rows. Tier D `OHLCV_PROXY` input. |
| Current price board | Success through Doraemon diagnostics | Snapshot/realtime | Three bid/ask levels, total and put-through value/volume, foreign volume. No historical quote archive or synchronized per-trade quote pair. |

Matched-by-price consistency on 2026-09-11:

| Symbol | Price levels | Buy volume | Sell volume | Unknown volume | Total volume | Reconciles? |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| FPT | 18 | 1,495,400 | 3,523,200 | 636,600 | 5,655,200 | Yes |
| SHS | 8 | 2,865,500 | 9,376,400 | 1,196,700 | 13,438,600 | Yes |
| BSR | 16 | 8,913,100 | 7,314,700 | 998,100 | 17,225,900 | Yes |

Arithmetic consistency does not validate economic semantics. In particular, `buyVol`/`sellVol` and `LC` must not be re-labelled as trustworthy buyer-/seller-initiated executions without a contract or labeled-validation study.

### SSI anonymous control probe

The documented `DailyStockPrice` endpoint returned HTTP 200 with an application payload `{status: 401, message: Missing Authorization header}` when called without credentials. This confirms reachability and authentication enforcement, not data access. No SSI credential exists in the audited projects, so no authenticated claim is made.

## 4. Money Flow Capability Matrix

The expanded machine-readable matrix is [data_capability_matrix.csv](data_capability_matrix.csv). Summary:

| Source/segment | Best honest classification | Why |
| --- | --- | --- |
| Sumi local CafeF/Yahoo daily rows | `OHLCV_PROXY` | Stored OHLCV only; see Sumi-only audit for quality limits |
| Doraemon KBS persisted/direct daily bars | `OHLCV_PROXY` | Real OHLCV; actual value is sparse, so value-source metadata is mandatory |
| Doraemon KBS current trade fields excluding `LC` | `TICK_TEST_ESTIMATE_RESEARCH_ONLY` | Trade timestamp/price/volume exist for current session only |
| Doraemon KBS `LC` and matched-by-price buy/sell labels | `UNKNOWN` | No audited semantic contract or labeled benchmark |
| Doraemon current board quotes | `UNKNOWN` as a standalone flow source | Useful live quotes, but not archived/synchronized with trades and not themselves executions |
| Doraemon independent money-flow API | `OHLCV_PROXY` conceptually, but **not BB-compatible as-is** | It is a multi-indicator price/volume proxy with explicit `not_qmv_blackbox`; BB must keep one authoritative formula |
| SSI daily endpoints (external candidate) | `OHLCV_PROXY` | Official docs expose actual matched value/volume, put-through totals and foreign value; order totals are not aggressor flow |
| SSI X-TRADE/X-QUOTE (external candidate) | `UNKNOWN` now; candidate `CLASSIFIED_ORDER_FLOW` after validation | Side is provider-labeled BU/SD/unknown and live quotes exist; no historical archive/synchronization or validation was established |
| FiinQuant (external candidate) | `UNKNOWN` now; candidate Tier B/C | Official marketing states tick trades, order book, API/WebSocket and historical data, but does not prove historical tick/book depth or aggressor semantics |
| HOSE/HNX direct products | `UNKNOWN` pending product dictionary/contract | Highest-authority sources, but fields, archive depth and redistribution rights must be quoted and validated |

No audited source qualifies as `EXECUTED_ORDER_FLOW`. No source qualifies today as production `CLASSIFIED_ORDER_FLOW`.

## 5. Blackbox Mode Recommendation

| Product output | Implement now? | Mode / boundary |
| --- | --- | --- |
| Symbol BB, daily historical | **Yes, bounded MVP** | `OHLCV_PROXY`; Doraemon daily bars are preferred. Use actual matched/trading value only where explicitly present; otherwise explicit `ESTIMATED_TP_X_VOLUME`. Separate/label capability and value-source segments. |
| Symbol BB, current session | **Research spike only** | KBS prints allow `TICK_TEST_ESTIMATE_RESEARCH_ONLY`. Ignore `LC` for canonical calculation until validated. Do not merge with daily proxy as if one method. |
| Market BB | **No compliant implementation yet** | Current 470-symbol membership is insufficient to prove survivorship-safe historical aggregation; index volume is not a substitute. |
| Flow Breadth | **No compliant implementation yet** | Requires the same point-in-time eligible universe plus comparable per-symbol flow primitives and missingness policy. |
| Historical true/classified BB | **No** | No historical executed-side feed or synchronized trade/quote archive is available. |
| Realtime true/classified BB | **No production mode** | KBS and SSI/FiinQuant are candidates; semantic validation, persistence, clock analysis, coverage and licensing gates remain. |

This does not block the Price/Volume Signal Engine or a causal Backtest Engine. It only constrains the claims attached to Money Flow outputs.

## 6. Data Gaps

- Trustworthy exchange-defined aggressor side or a validated provider-side classification.
- Historical trade ticks across HOSE, HNX and UPCoM; KBS tested history is session-only.
- Historical synchronized best bid/ask and clock/sequence guarantees suitable for classification.
- Stable upstream transaction identifiers, auction/continuous-session semantics and put-through flags on individual prints.
- Broad, consistently populated actual matched value in Doraemon history.
- Foreign buy/sell **value**; current production has only sparse recent volume.
- Proprietary buy/sell value/volume.
- A full, validated point-in-time security lifecycle and exchange/index universe covering listings, delistings, suspensions and security type.
- Complete corporate-action/adjusted-close history; five reviewed events and 0.49% adjusted-close coverage are inadequate.
- Published/observed rate limits and service-level guarantees for KBS.
- Licensing/redistribution permission for persisting and using broker/exchange data in Sumi, Doraemon and Mizuhara.

Orders placed, daily buy/sell order counts and order volumes are not executed aggressive flow. Foreign values approximated from volume × last price are estimates, not canonical exchange execution value.

## 7. Alternative Data Sources

Research was limited to sources needed to close the identified gaps and used official/provider documentation.

| Candidate | Verified public evidence | Remaining diligence | BB suitability / effort |
| --- | --- | --- | --- |
| [SSI FastConnect API](https://guide.ssi.com.vn/ssi-products/fastconnect-data/api-specs) | Securities/exchange metadata, daily and historical 1-minute OHLC, daily matched/put-through totals, foreign buy/sell volume/value and index breadth fields. Consumer ID/secret authentication. | Contract, price/rate limits, historical depth, redistribution; confirm exact field semantics. `TotalBuyTrade*`/`TotalSellTrade*` are order statistics and must not be treated as aggressor executions. | Strongest low-effort Tier D/PIT discovery candidate. |
| [SSI streaming](https://guide.ssi.com.vn/ssi-products/fastconnect-data/streaming-data) | Live `X-TRADE` with time, last price/volume and Side=`BU`/`SD`/unknown; live `X-QUOTE` with three HOSE and up-to-ten HNX/UPCoM depth levels. | Persist an overlap sample; establish provider-side rule, timestamps/sequence, dropped-message recovery, auction/put-through behavior and labeled agreement. Docs expose no historical trade/quote API. | Candidate `CLASSIFIED_ORDER_FLOW`, not Tier A; medium effort. |
| [FiinQuant](https://dff.fiingroup.vn/NewsInsights/Detail/11599078?lang=vi-vn) | Provider states official KRX-connected realtime tick trades, order book, smart-money statistics, API/WebSocket, and historical/realtime merging. | Obtain trial schema and commercial quote; prove historical tick/book depth, side semantics, exchange coverage, rate limits, retention and redistribution. Marketing alone is insufficient. | Promising Tier B/C procurement spike; medium effort/cost unknown. |
| [FiinGroup API Datafeed](https://datafeed.fiingroup.vn/api-giao-dich/tu-doanh/tu-doanh/giao-dich-tu-doanh) | Documented proprietary trading API with buy/sell matched volume/value fields for HOSE/HNX. | Authentication, price, start date, correction policy, UPCoM coverage, licensing and sample validation. Investor-group daily data is context, not aggressor flow. | Valuable proprietary-flow enrichment; not Symbol BB Tier A/B by itself. |
| [HOSE official market-data price sheet](https://staticfile.hsx.vn/Uploads/UploadDocuments/2406142/Bieu%20gia%20dich%20vu%20cung%20cap%20tin.pdf) | Direct connection and online/delayed/EOD products with explicit display fees. Current sheet lists VND 150m/year for online webservice market feed, VND 300m/year standard feed, plus connection/end-user charges. | Request field dictionary, non-display/research rights, historical tick/deal archive quote, correction protocol and redistribution terms. HOSE only. | Highest authority; high cost/integration effort; tier remains `UNKNOWN` until fields prove side/classification. |
| [HNX information service](https://hnx.vn/vi-vn/dich-vu-cctt/huong-dan-yeu-cau-ky-thuat-shdk.html) | Official service requires a product request; HNX then supplies fee schedule and sample contract. Public datafeed offers delayed market data and disclosures. | Request product catalog for HNX/UPCoM ticks, order book, history, lifecycle, corrections and non-display rights. | Required official complement to HOSE; cost/tier unknown. |
| [VNDIRECT Datafeed terms](https://datafeed.vndirect.com.vn/term-full) | Paid Datafeed product exists for technical-analysis software. Terms restrict unauthorized copying/distribution. | No public evidence found for supported API schema, historical tick/book depth or aggressor side. Obtain a written technical/commercial offer before considering. | Lower-priority candidate; capability `UNKNOWN`. |

FiinTrade's public user manual describes current-session Time & Sales classified as Buy Up/Sell Down/Other and a post-close proprietary heatmap, but no auditable API/archive/redistribution contract was found. It is a product-observation lead, not a source recommendation.

## 8. Recommended MVP Data Strategy

1. Build the first Sumi BB research slice only as daily `OHLCV_PROXY`, consuming a provider-neutral bar contract. Prefer Doraemon's authenticated read API over duplicating KBS access in Sumi.
2. Persist/expose per observation: provider, source endpoint/table, exchange, session date, adjustment policy, `flow_method`, `value_source`, quality, missingness and calculation version. Never silently impute an absent actual value.
3. Segment results when method or value source changes. A future `CLASSIFIED_ORDER_FLOW` series must be a different series/method, with overlap comparison rather than backfilled relabeling.
4. Keep Market BB and breadth feature-flagged unavailable until the PIT-universe gate passes. Continue Price/Volume Signal and causal backtest work independently.
5. Run one procurement spike in order: authenticated SSI daily + streaming trial first; FiinQuant trial second if SSI cannot supply licensed persistence/adequate historical evidence. Contact official exchanges only if production requirements justify the cost and rights.
6. Treat the KBS live feed as a low-cost research dataset: retain a small future overlap sample only after authorization, then test `LC`, tick rule and quote classifiers against SSI/Fiin labeled data. No claim upgrade without documented thresholds.

## 9. Production Migration Strategy

- Define a Sumi-side `MarketDataPort` around canonical observations, not provider payloads. Provider adapters own KBS/SSI/Fiin field mappings.
- Keep BB math as one authoritative, versioned domain module. Doraemon's current `money_flow_daily_v1` remains a separate proxy and must not become a second Blackbox formula.
- Make Doraemon the eventual source of canonical market observations and capability manifests; Sumi remains a research consumer, and Mizuhara consumes stable product APIs.
- Require `as_of`/current-index boundaries for every research and replay query so Doraemon never sends future candles or future membership state.
- Add future storage by immutable observation family: daily bars/value, trades, quotes, investor flows, and universe episodes. Retain raw source hashes/contracts and correction lineage.
- Promote a source/method only after schema, license, missingness, timestamp synchronization, cross-source reconciliation and rollback gates pass. Do not overwrite proxy history with a better method.
- Keep credentials in the existing environment/secret mechanisms; no credential belongs in Sumi datasets, reports or frontend code.

## 10. Decisions Required

1. **MVP authorization:** approve daily Symbol BB as explicitly labelled `OHLCV_PROXY`, with Market BB/Breadth unavailable, or defer all BB implementation.
2. **Value policy:** approve row/segment-level use of actual matched value when populated and `ESTIMATED_TP_X_VOLUME` otherwise, or require actual value for an entire series before BB is shown.
3. **Procurement budget/rights:** authorize an SSI FastConnect trial and request written persistence/non-display terms; decide whether FiinQuant is the fallback commercial trial.
4. **Research-only live spike:** decide whether to authorize future retention of a small KBS/SSI overlap sample to validate side/timestamp semantics. This audit did not persist such data.
5. **Market scope:** define whether the eventual historical universe is all eligible HOSE/HNX/UPCoM equities or a narrower, versioned research universe; this determines PIT data procurement.

## Operational reuse and freshness rule

This audit is a dated evidence snapshot, not a permanent assertion that every source remains unchanged. Before an implementation batch depends on Doraemon data or a provider capability:

1. Read this audit and `data_capability_matrix.csv`.
2. Run only a read-only targeted freshness check for the exact source/endpoint, fields, symbols or universe, and date range required by that batch. Do not repeat the full cross-repository audit.
3. Record the checked source identity, endpoint/table, observation time, latest available market date, relevant row/field coverage, and the semantic/method classification used by the batch. Include access and retention/redistribution status when they affect the capability.
4. If the observed schema, freshness, coverage, semantics, access, or rights differ from this snapshot, update the evidence here and the corresponding capability-matrix row before coding. Escalate when a mandatory capability has regressed or remains unverified.

### Targeted freshness check record: Batch P5-DATA-01 (2026-09-18)

- **Execution time:** 2026-09-18T12:43:58Z (read-only, zero DB mutation).
- **Target source:** Doraemon production API (`https://api.hieupb.io.vn/api/v1/market/prices/{symbol}/history/full`).
- **Target fields:** `symbol`, `trading_date`, `open_price`, `high_price`, `low_price`, `close_price`, `adjusted_close_price`, `volume`, `total_trade_value`, `source`.
- **Target symbols & date range:** `FPT`, `HPG`, `SSI` from `2026-08-01` to `2026-09-18`.
- **Findings:**
  - 32 trading sessions returned per symbol (`2026-08-03` to `2026-09-18`).
  - **FPT freshness resolved:** The previously noted stale FPT end date (`2026-08-28`) is resolved upstream; FPT is current to `2026-09-18`.
  - **Actual trading value:** Populated on 31/32 rows for `FPT` and `HPG`, 22/32 rows for `SSI`. The latest intraday/in-progress row has `total_trade_value=None`, reinforcing the mandatory fallback to `ESTIMATED_TP_X_VOLUME` when value is missing.
  - **Adjusted close:** 0/32 rows populated in recent slice, confirming unadjusted price status.
  - **Classification:** Confirms `OHLCV_PROXY` is the only validated and honest method for daily bars.

### Targeted freshness check record: Batch P6-MEASURE-01 (2026-09-18)

- **Execution time:** 2026-09-18T15:01:07Z (read-only, zero DB mutation).
- **Target source:** Doraemon production API (`https://api.hieupb.io.vn/api/v1/market/prices/{symbol}/history/full`).
- **Target fields:** `symbol`, `trading_date`, `open_price`, `high_price`, `low_price`, `close_price`, `adjusted_close_price`, `volume`, `total_trade_value`, `source`.
- **Target symbols & date range:** `FPT`, `HPG`, `SSI` from `2024-01-01` to `2026-09-18` (extended evaluation window for statistical distributions).
- **Findings:**
  - Exactly 673 daily sessions returned per symbol (`2024-01-02` to `2026-09-18`).
  - **Freshness:** Upstream data is verified current to `2026-09-18` across all three symbols.
  - **Actual trading value:** Populated on 78/673 (11.6%) for FPT, 81/673 (12.0%) for HPG, and 61/673 (9.1%) for SSI. All other rows reliably fall back to `ESTIMATED_TP_X_VOLUME`.
  - **Classification:** Strictly confirmed as `OHLCV_PROXY`.
  - **Scope:** Sufficient historical depth (673 sessions > 200 bars) to fully evaluate all horizons including T03, T05, T10, T20, T50, and T200.

## Audit integrity and stop

- No production feature or ingestion code was changed.
- No database was mutated and no provider/account write operation was invoked.
- No secret value appears in this artifact or the capability matrix.
- Doraemon's tracked-secret scan passed.
- Findings are based on repository code, production read APIs, minimal live KBS/SSI probes and cited official/provider documentation.

**STOP:** The cross-repository market-data audit is complete. Production implementation requires a separate focused authorization.
