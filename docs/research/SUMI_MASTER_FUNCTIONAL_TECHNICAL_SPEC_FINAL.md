# SUMI — MASTER FUNCTIONAL & TECHNICAL SPECIFICATION

**Document type:** Master Functional + Technical Specification / Codex Handoff Baseline  
**Project:** Sumi — Vietnam Stock Market Quantitative Analysis & Backtest Platform  
**Scope:** Price, Volume, Money Flow, Signal Library, Strategy Rule Composition, Automated Multi-Ticker/Multi-Phase Backtesting  
**Status:** FINAL HANDOFF BASELINE — implementation-ready after mandatory codebase/data audit  
**Language:** Vietnamese with stable English technical identifiers  
**Master Spec Version:** 1.0-final  
**Companion Spec:** `SUMI Money Flow Blackbox V1 — Research Summary, Data Strategy, Algorithm Specification & DEV Validation Plan`

---

## 0. MỤC ĐÍCH TÀI LIỆU

Tài liệu này là đặc tả tổng quát để đội phát triển/Codex triển khai bài toán nâng cấp Sumi từ một hệ thống indicator/backtest cơ bản thành một nền tảng phân tích định lượng và kiểm thử chiến lược có cấu trúc, deterministic, giải thích được, dễ cấu hình và phù hợp với thị trường chứng khoán Việt Nam.

Tài liệu này **không thay thế** tài liệu nghiên cứu/đặc tả chuyên sâu của **Money Flow Blackbox (BB)**. Hai bộ tài liệu phải được bàn giao cùng nhau cho DEV.

### 0.1. Quy tắc bắt buộc về tài liệu Blackbox

DEV/Codex **PHẢI đọc kỹ toàn bộ bộ tài liệu Money Flow Blackbox trước khi implement bất kỳ chức năng nào liên quan tới dòng tiền**.

Trong phạm vi Money Flow/Blackbox:

- **BB Specification là tài liệu chuyên ngành authoritative** cho công thức, data semantics, `flow_method`, multi-horizon, market aggregation, breadth và giới hạn của dữ liệu.
- Tài liệu Master này chỉ quy định **vai trò, interface, integration, dependency và boundary** của BB trong toàn hệ thống.
- Nếu có khác biệt giữa Master Spec và BB Spec về chi tiết thuật toán Money Flow, **BB Spec được ưu tiên**.
- Không được tự ý thay thế core BB bằng RSI/MFI/CMF/OBV hoặc một “composite money flow score” nếu BB Spec không quy định như vậy.
- Mọi kết luận về khả năng chạy `EXECUTED_ORDER_FLOW`, `OHLCV_PROXY` hoặc hybrid theo thời kỳ phải chờ **data audit thực tế trên codebase/data source hiện có**.

---

# 0A. CÁCH ĐỌC SPEC & QUY ƯỚC REQUIREMENT

Tài liệu này dùng requirement IDs để DEV/Codex có thể trace từ yêu cầu → code → test → acceptance result.

| Prefix | Ý nghĩa |
|---|---|
| `FR-*` | Functional Requirement |
| `NFR-*` | Non-Functional Requirement |
| `SIG-*` | Signal/feature contract |
| `BBI-*` | Money Flow Blackbox integration requirement |
| `DSL-*` | Strategy DSL requirement |
| `BT-*` | Backtest/execution requirement |
| `MET-*` | Metrics/reporting requirement |
| `DATA-*` | Data/data-quality requirement |
| `API-*` | API/domain contract |
| `UI-*` | User-interface/configuration requirement |
| `TEST-*` | Test/acceptance requirement |

Requirement keywords:

- **MUST**: bắt buộc cho V1 production.
- **SHOULD**: mặc định nên thực hiện; chỉ bỏ khi audit cho thấy lý do kỹ thuật rõ ràng.
- **MAY**: optional.
- **DEFERRED**: không nằm trong V1; không được Codex tự mở rộng.
- **PROHIBITED**: không được implement theo cách mô tả.

### 0A.1. Quy tắc chống tự diễn giải

Nếu requirement chưa đủ dữ liệu để chốt implementation, DEV/Codex phải tạo một **Decision Required** item với:

```text
requirement_id
question
available_evidence
options
recommended_option
impact
owner
status
```

Không được âm thầm chọn threshold, data source, historical rule hoặc trading assumption chưa được specification/audit xác nhận.

### 0A.2. Thứ tự đọc bắt buộc

```text
1. Master Spec này — đọc toàn bộ
2. Money Flow Blackbox V1 Research & DEV Spec — đọc toàn bộ
3. Audit repository / database / external data adapters hiện có
4. Tạo Codebase Map + Data Capability Matrix
5. Tạo Implementation Plan mapping requirement IDs -> files/tests
6. Chỉ sau đó mới sửa code
```

---

# 1. PRODUCT VISION

Sumi tập trung vào ba trục phân tích cốt lõi:

1. **GIÁ — PRICE**  
   Giá đang ở trạng thái nào? Tăng, giảm, đi ngang, điều chỉnh, phục hồi, breakout hay breakdown? Cấu trúc candle và trend có khỏe không?

2. **KHỐI LƯỢNG — VOLUME**  
   Mức độ giao dịch có bất thường không? Biến động giá có được volume xác nhận không? Có dấu hiệu climax, strong demand, weak demand, upthrust, shakeout hay không?

3. **DÒNG TIỀN — MONEY FLOW / BLACKBOX**  
   Áp lực giao dịch chủ động đang nghiêng về bên mua hay bên bán ở các horizon khác nhau? Dòng tiền có đồng thuận ngắn/trung/dài hạn không? Áp lực đó lan rộng toàn thị trường hay chỉ tập trung vào vài mã?

Ba trục này phải được **tách biệt ở tầng primitive**, sau đó Signal Engine mới kết hợp 2–3 nguồn xác nhận khi cần.

### 1.1. Triết lý thiết kế

Sumi phải tuân thủ các nguyên tắc sau:

- **Simple first:** ưu tiên indicator/phương pháp phổ biến, dễ hiểu và dễ kiểm chứng.
- **Không dùng “indicator lạ” chỉ vì có thư viện hỗ trợ.**
- **Không tạo nồi trộn 10–20 indicator** để sinh một tín hiệu khó giải thích.
- Một composite signal thông thường nên dựa trên **2–3 thành phần có ý nghĩa khác nhau**.
- Tất cả tín hiệu phải **deterministic** và **không look-ahead**.
- Các tham số hợp lý phải **configurable qua UI**, không hard-code.
- Mỗi signal nên chỉ expose khoảng **2–5 tham số dễ hiểu**.
- Signal phải trả được **reason/evidence**, không chỉ boolean.
- Backend là **single source of truth** cho indicator, signal, execution và metrics.
- Backtest phải mô phỏng quy tắc thị trường Việt Nam và giữ reproducibility.

---

# 2. BỐI CẢNH CODEBASE HIỆN TẠI

## 2.1. Tech stack

- Python 3.13
- FastAPI
- SQLAlchemy
- Pydantic v2
- Uvicorn
- pandas
- numpy
- pandas-ta
- SQLite + Alembic
- React 19 + TypeScript
- TanStack Query
- lightweight-charts

## 2.2. Data hiện có

Bảng `candles` hiện có OHLCV 1D cho khoảng 3.734 mã/chỉ số với lịch sử từ khoảng năm 2000 tới giữa 2026.

Schema tối thiểu:

```text
timestamp
symbol
open
high
low
close
volume
adjustment_type
```

Các nguồn dữ liệu khác có thể tồn tại trong codebase/API hiện tại nhưng **phải audit thực tế**, đặc biệt cho Money Flow BB:

```text
executed trades
trade price
trade volume
aggressor side
bid / ask
foreign buy/sell
proprietary buy/sell
intraday order/trade data
```

## 2.3. Indicator hiện có

Tối thiểu gồm:

```text
SMA
EMA
MACD
RSI
Bollinger Bands
ATR
ADX
Ichimoku
Stochastic
Volume SMA
Parabolic SAR
SuperTrend
CCI
MFI
Keltner Channels
Relative Strength vs VNINDEX
```

## 2.4. Rule Engine hiện có

AST-based safe evaluator, không dùng `eval()`, với các toán tử dạng:

```text
gt
gte
lt
lte
eq
cross_up
cross_down
between
rising
falling
all
any
not
```

Thiết kế mới phải **tận dụng lại** nền tảng này thay vì tạo một DSL hoàn toàn mới nếu không cần thiết.

---

# 2A. FUNCTIONAL REQUIREMENTS CẤP HỆ THỐNG

| ID | Requirement | Priority |
|---|---|---|
| `FR-CORE-001` | Sumi MUST phân tích ba trục độc lập: Price, Volume, Money Flow; không blend primitive semantics. | MUST |
| `FR-CORE-002` | Indicator/feature calculation MUST nằm ở backend và deterministic với cùng input/config/version. | MUST |
| `FR-CORE-003` | Signal Engine MUST tạo atomic/composite signals từ primitive đã tính; không được tự thực hiện trade execution. | MUST |
| `FR-CORE-004` | Strategy Rule Evaluator MUST kết hợp indicator/signal bằng safe AST; không dùng unsafe `eval`. | MUST |
| `FR-CORE-005` | Backtest MUST tiêu thụ precomputed signals và không tái hiện logic indicator/signal bên trong execution loop. | MUST |
| `FR-CORE-006` | User MUST có thể cấu hình các parameter được expose mà không sửa source code. | MUST |
| `FR-CORE-007` | Signal quan trọng MUST trả reason/evidence đủ để UI/debug giải thích tại sao signal active. | MUST |
| `FR-CORE-008` | Batch backtest MUST chạy danh sách ticker tùy ý trên nhiều phase/date ranges trong một run. | MUST |
| `FR-CORE-009` | Mỗi ticker trong Batch Single-Asset mode MUST được cấp capital độc lập theo config. | MUST |
| `FR-CORE-010` | Kết quả MUST tạo đúng 9 metrics benchmark đã định nghĩa và hỗ trợ `N/A` edge cases. | MUST |
| `FR-CORE-011` | Toàn bộ causal signal/backtest MUST không look-ahead. | MUST |
| `FR-CORE-012` | Money Flow subsystem MUST tuân thủ companion BB Spec và expose flow method/data quality. | MUST |
| `FR-CORE-013` | Price/Volume/Backtest workstreams MUST có thể phát triển độc lập trong khi BB data audit chưa hoàn tất. | MUST |
| `FR-CORE-014` | Một historical run MUST có metadata đủ để reproduce lại calculation/execution semantics. | MUST |

## 2B. NON-FUNCTIONAL REQUIREMENTS

| ID | Requirement | Acceptance intent |
|---|---|---|
| `NFR-PERF-001` | Compute indicators/signals once per symbol/data snapshot, slice many phases. | Không recalc full feature stack cho từng phase. |
| `NFR-PERF-002` | 30–50 daily tickers × 3 phases SHOULD chạy ở mức interactive trên máy local sau warm cache. | Benchmark thực tế phải được DEV ghi lại; không hard-code SLA trước audit máy/data. |
| `NFR-DET-001` | Same input snapshot + same config/version MUST produce same output. | Golden/regression test. |
| `NFR-OBS-001` | Calculation/backtest error MUST chứa requirement/module/context đủ debug. | Structured logging. |
| `NFR-COMP-001` | Thiết kế MUST tương thích Python 3.13/Windows của project. | CI/local installation pass. |
| `NFR-MAINT-001` | Signal implementation MUST modular; không có một `calculate_all_signals()` monolith chứa toàn bộ business logic. | Code review. |
| `NFR-UX-001` | Ordinary end-user signal config SHOULD giới hạn khoảng 2–5 tham số/signal. | UI review. |
| `NFR-DQ-001` | Missing/insufficient data MUST propagate thành explicit quality/null state, không silently biến thành zero/false khi semantics khác. | Data-quality tests. |

---

# 3. KIẾN TRÚC LOGIC TỔNG THỂ

```text
Market/Data Sources
      │
      ▼
MarketDataLoader
      │
      ├──────────────► Money Flow Raw Data Adapter
      │                         │
      ▼                         ▼
IndicatorEngine           MoneyFlowBlackbox
      │                         │
      └──────────┬──────────────┘
                 ▼
            Feature Matrix
                 │
                 ▼
             SignalEngine
       Atomic + Composite Signals
                 │
                 ▼
        StrategyRuleEvaluator
                 │
                 ▼
       BacktestExecutionEngine
                 │
                 ▼
          Trade Ledger / PnL
                 │
                 ▼
          MetricsAggregator
                 │
                 ▼
      API / Tables / Charts / Export
```

### 3.1. Trách nhiệm từng tầng

**IndicatorEngine**  
Chỉ tính indicator và primitive numeric series.

**MoneyFlowBlackbox**  
Chỉ tính các primitive/series dòng tiền theo BB Spec. Không phát BUY/SELL trực tiếp.

**SignalEngine**  
Biến primitive thành atomic/composite signal, regime, score, event và explanation.

**StrategyRuleEvaluator**  
Kết hợp các signal/indicator bằng rule do user cấu hình.

**BacktestExecutionEngine**  
Không được tự tính indicator hoặc tự “suy luận signal”. Chỉ tiêu thụ entry/exit arrays và mô phỏng giao dịch.

---

# 3A. V1 DECISION MATRIX — KEEP / DEFER / EXCLUDE

Bảng này là guardrail để Codex không tự mở rộng scope.

| Ý tưởng | V1 Decision | Implementation interpretation |
|---|---|---|
| Bullish/Bearish Pattern | **KEEP** | Aggregate một tập candle pattern phổ biến, explainable. |
| Bullish/Bearish Technical Signal | **KEEP** | Trigger động từ MACD/RSI/EMA/swing; không trộn quá nhiều. |
| Khối lượng đột biến | **KEEP** | Relative Volume vs prior rolling baseline. |
| Uptrend/Downtrend | **KEEP** | EMA alignment + slope; confirmations optional. |
| Sideways | **KEEP** | ADX/BBW/EMA slope theo simple rule preset. |
| Correction/Recovery | **KEEP** | Contextual regime transitions với rule đơn giản. |
| New High/New Low | **KEEP** | Causal rolling breakout. |
| Cầu mạnh/Cầu yếu | **KEEP** | Price-volume technical inference; không gọi là observed true demand. |
| Kéo xả | **KEEP — SIMPLIFIED** | Upthrust/failed breakout/high-volume rejection; không build complex distribution engine. |
| Đạp kéo / Spring | **KEEP** | Support breach + reclaim + wick + relative volume. |
| Uốn hỗ trợ/Uốn kháng cự dạng curvature | **EXCLUDE V1** | Không polynomial/curve fitting. Có thể quay lại như simple composite với spec riêng. |
| Cầu mạnh tại hỗ trợ/Cung mạnh tại kháng cự | **KEEP** | Composite của support/resistance + VSA atomic signal. |
| Good/Bad Candle Structure | **KEEP** | Transparent score. |
| Favorable/Unfavorable Indicators | **KEEP** | Technical Health Score từ 3–4 nhóm, tránh double counting. |
| Ichimoku tốt/xấu | **KEEP** | Multi-factor score, displacement causal. |
| 4 loại divergence | **KEEP** | Confirmed causal pivots; RSI/MACD Histogram/Stochastic. |
| BB T03/T05/T10/T20/T50/T200 | **KEEP via BB Spec** | Money Flow subsystem authoritative. |
| BB turn/confluence events | **RESEARCH / PARAMETERIZED** | Không freeze 20/30/70/80 hoặc tolerance trước BB evaluation. |
| Tổ chức/nhỏ lẻ suy từ OHLCV | **PROHIBITED** | Chỉ dùng khi có investor-class data riêng. |
| Foreign/proprietary buy/sell | **KEEP WHEN DATA EXISTS** | Channel riêng; không blend vào core BB. |
| Value/Growth portfolio classification | **DEFERRED** | Cần fundamental data; ngoài Price/Volume/Money Flow V1. |
| Policy cycle: easing/transition/tightening | **DEFERRED / separate macro module** | Không suy ra từ technical indicators. |
| Machine-learning signal generation | **EXCLUDE V1** | Trái simple-first/reproducibility goal hiện tại. |
| Full shared-cash portfolio optimizer | **DEFERRED** | V1 là Batch Single-Asset Backtest. |

---

# 4. PHẠM VI SIGNAL LIBRARY V1

Signal Library V1 ưu tiên các signal có logic đơn giản, giải thích được và configurable.

## 4.1. Group A — Price/Candle Pattern & Technical Trigger

### 4.1.1. Bullish Pattern

**Status:** MUST HAVE  
**Output:** boolean + reason list

Aggregate từ một tập pattern phổ biến, không cần quá nhiều.

Recommended V1:

```text
bullish_engulfing
hammer
bullish_pinbar
morning_star
piercing_line
tweezer_bottom
inside_bar_breakout_up
```

`pattern.bullish_any = OR(selected_patterns)`

User có thể bật/tắt pattern qua config.

### 4.1.2. Bearish Pattern

Recommended V1:

```text
bearish_engulfing
shooting_star
bearish_pinbar
evening_star
dark_cloud_cover
tweezer_top
inside_bar_breakdown
```

`pattern.bearish_any = OR(selected_patterns)`

### 4.1.3. Bullish Signal

**Status:** MUST HAVE

Không phải candle pattern. Là nhóm trigger động từ indicator.

Recommended candidate primitives:

```text
MACD signal cross up
MACD zero cross up
RSI cross configurable level
EMA cross / EMA alignment transition
SuperTrend flip up (optional)
Bollinger squeeze breakout up (optional)
Break confirmed swing high
```

Default composite nên dùng threshold count hoặc rule rõ ràng, ví dụ `at_least_2` trên tập user chọn, thay vì OR tất cả.

### 4.1.4. Bearish Signal

Đối xứng Bullish Signal.

### 4.1.5. Volume Spike

**Status:** MUST HAVE

Primitive cơ bản:

```text
relative_volume = volume / rolling_mean(previous_volume, N)
volume_spike = relative_volume >= multiplier
```

Default gợi ý:

```text
period = 20
multiplier = 2.0
```

Optional filters:

```text
require_bullish_candle
require_bearish_candle
min_range_atr
```

---

# 5. GROUP B — PRICE TREND / REGIME

## 5.1. Uptrend

**Status:** MUST HAVE

Simple default:

```text
close > EMA_fast > EMA_slow
EMA_fast rising
```

Optional confirmation:

```text
ADX >= threshold
confirmed HH/HL structure
```

Default proposal:

```text
EMA_fast = 20
EMA_slow = 50
require_fast_slope = true
require_adx = false by default
```

Không bắt buộc nhồi ADX + swing + nhiều điều kiện cùng lúc.

## 5.2. Downtrend

Đối xứng:

```text
close < EMA_fast < EMA_slow
EMA_fast falling
```

## 5.3. Sideways

**Status:** MUST HAVE

Simple default nên kết hợp tối đa 2 điều kiện:

```text
ADX < threshold
AND/OR
Bollinger Band Width thấp
AND/OR
EMA slope gần 0
```

UI cho phép chọn rule mode.

## 5.4. Correction / Pullback

**Status:** MUST HAVE

Definition V1:

```text
previous/dominant regime = UPTREND
current drawdown from recent high within configured range
trend structure chưa bị phá hoàn toàn
```

Default có thể dùng:

```text
lookback_high = 20
min_drawdown_pct = 2
max_drawdown_pct = 12
support_reference = EMA50
```

Volume diminishing là confirmation optional, không phải bắt buộc.

## 5.5. Recovery

**Status:** MUST HAVE

Definition V1:

```text
recent regime contained DOWNTREND
AND
close recovers above short EMA
AND
short EMA starts rising OR short swing resistance is broken
```

RSI cross 45/50 có thể là optional confirmation.

Recovery không đồng nghĩa Uptrend.

## 5.6. New High / New Low

**Status:** MUST HAVE

Configurable horizons:

```text
20
50
252
custom
```

Causal calculation bắt buộc dùng rolling previous bars, không để current bar nằm trong reference window.

---

# 6. GROUP C — SUPPLY / DEMAND / SIMPLE VSA

Các signal nhóm này là **technical inference từ price + volume**, không được mô tả như quan sát trực tiếp “cung/cầu thật”.

## 6.1. Strong Demand

**Status:** MUST HAVE

Simple interpretation:

```text
up bar
wide range relative to ATR
close near high
relative volume high
```

Expose tối đa:

```text
volume_multiplier
min_range_atr
min_close_location
```

## 6.2. Weak Demand / No Demand

**Status:** MUST HAVE

Simple interpretation:

```text
price attempts to rise
range narrow
volume low relative to baseline
```

Optional compare with prior 1–2 bars.

## 6.3. Pull-and-Distribute / Kéo xả

**Status:** V1 SIMPLIFIED ONLY

Không xây một hệ thống distribution phức tạp.

V1 chỉ giữ một số atomic event dễ giải thích:

```text
upthrust
failed_breakout_high_volume
high_volume_rejection
```

Recommended default `upthrust`:

```text
intrabar/close attempts above prior resistance
close returns below resistance
upper wick large
relative volume high
```

`keo_xa` nếu cần chỉ là alias/composite của các atomic event này.

## 6.4. Shakeout / Spring / Đạp kéo

**Status:** MUST HAVE

Simple interpretation:

```text
price breaches prior support
then closes back above support
lower wick significant
relative volume elevated
```

Đây là signal đáng ưu tiên.

## 6.5. Uốn hỗ trợ / Uốn kháng cự

**Status:** EXCLUDED FROM V1 AS GEOMETRIC CURVATURE

Không dùng polynomial fitting/curvature trong V1 vì trái nguyên tắc simple-first.

Nếu product vẫn cần nhãn “uốn hỗ trợ/uốn kháng cự”, chỉ được triển khai sau này dưới dạng **simple composite**, ví dụ:

```text
near_support AND momentum_recovery
near_resistance AND momentum_weakening
```

và phải có spec riêng.

## 6.6. Strong Demand at Support

**Status:** MUST HAVE

Không tạo thuật toán mới:

```text
near_support AND (strong_demand OR spring)
```

## 6.7. Strong Supply at Resistance

**Status:** MUST HAVE

```text
near_resistance AND (upthrust OR strong_bearish_rejection)
```

---

# 7. GROUP D — TRADING SYSTEM / TECHNICAL HEALTH

## 7.1. Good / Bad Candlestick Structure

**Status:** MUST HAVE

Nên dùng score đơn giản từ:

```text
body ratio
close location within candle
upper/lower wick balance
direction
```

Output:

```text
candle_structure_score: -100..+100
good_candle: bool
bad_candle: bool
```

Default threshold gợi ý:

```text
good >= +40
bad <= -40
```

UI chỉ cần expose threshold và optional weighting preset; không expose quá nhiều weight nếu không cần.

## 7.2. Favorable / Unfavorable Indicators

**Status:** MUST HAVE, nhưng phải đơn giản

Tên kỹ thuật khuyến nghị:

```text
Technical Health Score
```

V1 chỉ nên dùng 3–4 nhóm độc lập:

```text
Trend: EMA alignment/slope
Momentum: RSI or CCI
Momentum transition: MACD
Participation/confirmation: Volume or Money Flow confirmation
```

Không double count nhiều oscillator cùng loại.

Ví dụ user preset có thể chọn:

```text
EMA + RSI + MACD
EMA + CCI + Volume
EMA + RSI + BB Confirmation
```

Output:

```text
technical_health_score: -100..+100
favorable: bool
unfavorable: bool
reason[]
```

Không tối ưu weights theo lợi nhuận historical để tránh overfitting.

## 7.3. Ichimoku Bullish / Bearish

**Status:** MUST HAVE

Sử dụng các thành phần phổ biến:

```text
price relative to visible Kumo
Tenkan vs Kijun
Chikou clearance
future Kumo orientation
Kijun slope optional
```

Output nên có:

```text
ichimoku_score
ichimoku_bullish
ichimoku_bearish
reason[]
```

Bắt buộc kiểm soát displacement đúng để không leak future data.

## 7.4. Four-way Divergence

**Status:** MUST HAVE

Supported oscillators V1:

```text
RSI
MACD Histogram
Stochastic
```

Types:

```text
regular_bullish
hidden_bullish
regular_bearish
hidden_bearish
```

### 7.4.1. Pivot requirement

Phải dùng confirmed pivot causal.

Một pivot tại bar `p` chỉ được biết sau `right_bars` phiên.

Data model nên lưu:

```text
pivot_at
confirmed_at
```

Signal chỉ được emit tại `confirmed_at` hoặc muộn hơn.

### 7.4.2. Default parameters

Gợi ý:

```text
left_bars = 3
right_bars = 3
min_separation = 5
max_separation = 60
```

Các parameter này configurable nhưng UI phải giữ đơn giản.

---

# 8. MONEY FLOW BLACKBOX — INTEGRATION CONTRACT

> **Authoritative companion:** `SUMI Money Flow Blackbox V1 — Research Summary, Data Strategy, Algorithm Specification & DEV Validation Plan`. Master Spec không được dùng để override công thức/data semantics trong companion spec. BB Spec hiện ở trạng thái **DESIGN BASELINE FOR RESEARCH IMPLEMENTATION**, vì vậy Data Audit Gate là bắt buộc trước production freeze.

## 8.1. Vai trò

Money Flow BB là subsystem độc lập phụ trách trục **DÒNG TIỀN**.

Signal Engine **không được tái tạo lại logic BB**.

## 8.2. Flow modes / measurement methods

Master architecture phải support các production/research modes do BB Spec định nghĩa:

```text
EXECUTED_ORDER_FLOW      # direct documented aggressor side
CLASSIFIED_ORDER_FLOW    # trades + quotes, classifier validated
TICK_TEST_ESTIMATE       # research only until validation threshold passes
OHLCV_PROXY              # technical fallback, not true order flow
```

Ý nghĩa và công thức chi tiết theo BB Spec.

### 8.2.1. EXECUTED_ORDER_FLOW

Ưu tiên khi có dữ liệu buyer-initiated/seller-initiated đáng tin cậy.

### 8.2.2. CLASSIFIED_ORDER_FLOW

Dùng khi không có `aggressor_side` trực tiếp nhưng có trade + synchronized quote data và classifier đã được validation theo BB Spec. Phải publish classification coverage/quality metadata.

### 8.2.3. TICK_TEST_ESTIMATE

Chỉ phục vụ research nếu chỉ có trade ticks. Không expose như production TRUE_FLOW cho tới khi vượt validation gate được ghi nhận trong BB Spec.

### 8.2.4. OHLCV_PROXY

Fallback khi lịch sử chỉ có OHLCV / matched trading value.

UI/API phải thể hiện khác biệt rõ ràng.

**Không splice** các method khác nhau thành một time series duy nhất như thể chúng đồng nhất.

## 8.3. Horizons

Canonical BB horizons theo BB Spec:

```text
BB03
BB05
BB10
BB20
BB50
BB200
```

Default chart có thể ẩn `BB10`, nhưng calculation/schema phải support đủ cả 6 horizons.

Các horizon là **rolling ratio**, không reset, không đồng nhất với EMA nhanh/chậm.

## 8.4. Whole-Market BB

Không average các BB từng cổ phiếu.

Phải aggregate primitive buy/sell value toàn universe trước, sau đó mới tính Market BB.

## 8.5. Breadth

Whole-Market output phải có flow breadth, ví dụ:

```text
positive_count_breadth_H
neutral_count_breadth_H
negative_count_breadth_H
positive_value_breadth_H
neutral_value_breadth_H
negative_value_breadth_H
```

Breadth là output độc lập với Market BB.

## 8.6. Không suy diễn investor class

Không được suy ra:

```text
institutional
retail
foreign
proprietary
```

chỉ từ BB horizon, candle, volume spike hoặc order-flow tổng.

Nếu có classification data riêng, lưu thành channel riêng.

## 8.7. Data audit blocker

Trước khi DEV chốt mode production, phải audit:

```text
raw executed trades?
aggressor_side?
bid/ask?
foreign buy/sell?
proprietary data?
historical coverage?
timezone/session semantics?
```

Kết quả audit phải tạo thành tài liệu/issue rõ ràng:

```text
TRUE_FLOW_AVAILABLE
CLASSIFIED_FLOW_AVAILABLE
RESEARCH_ONLY_TICK_ESTIMATE
PROXY_ONLY
HYBRID_BY_DATE
```

---

# 9. BLACKBOX-DERIVED EVENTS TRONG SIGNAL ENGINE

Signal Engine có thể tiêu thụ output BB để tạo event, nhưng không sửa core formula.

Examples:

```text
flow_turn_up_20
flow_turn_up_30
flow_turn_down_70
flow_turn_down_80
flow_confluence_up
flow_confluence_down
short_flow_recovery
short_flow_weakening
```

Threshold 20/30/70/80 **không phải hard-coded truth**; chúng phải configurable và chỉ được finalize theo BB Spec/research.

`50` giữ nghĩa balance nếu BB Spec quy định như vậy.

Các state như:

```text
money_weak
money_balanced
money_strong
money_saturated
inflow_signal
inflow_trend
outflow_signal
outflow_trend
```

chỉ triển khai khi BB Spec đã chốt rule chính xác.

---

# 10. SUPPORT / RESISTANCE PRIMITIVES

Để phục vụ Spring, Upthrust, Strong Demand at Support và Strong Supply at Resistance, V1 chỉ cần support/resistance đơn giản.

Allowed V1 sources:

```text
rolling N-bar high/low
confirmed swing high/low
EMA20/EMA50/EMA200
Kijun (optional)
```

Không cần clustering phức tạp, market profile, volume profile hoặc ML.

`near_support` / `near_resistance` nên normalize bằng ATR hoặc percentage tolerance.

---

# 11. SIGNAL REGISTRY

## 11.1. Mục tiêu

Mọi signal phải được đăng ký qua registry thay vì hard-code khắp codebase.

Example conceptual schema:

```python
SignalSpec(
    name="vsa.spring",
    category="VSA",
    version=1,
    output_type="bool",
    dependencies=["high", "low", "close", "volume", "atr14"],
    warmup=21,
    parameters={...},
)
```

## 11.2. Required metadata

Mỗi SignalSpec nên có:

```text
name
label_vi
category
version
description
output_type
dependencies
default_parameters
parameter_schema
warmup_bars
causal_delay_bars
reason_schema
status: active/experimental/deprecated
```

## 11.3. Stable namespaces

Recommended:

```text
pattern.*
technical_signal.*
volume.*
regime.*
vsa.*
support_resistance.*
structure.*
technical_health.*
ichimoku.*
divergence.*
flow.*
```

Internal safe AST names có thể map `.` thành `__`.

---

# 12. CONFIGURABILITY & UI PRINCIPLES

## 12.1. Nguyên tắc

Mỗi signal nên expose **2–5 tham số**.

Nếu cần >5–7 tham số mới dùng được, signal đó phải được xem xét lại hoặc chuyển sang advanced/experimental.

## 12.2. Ví dụ UI config

### Volume Spike

```text
Period               20
Multiplier           2.0
Require Bull Candle  false
```

### Uptrend

```text
Fast EMA             20
Slow EMA             50
Require EMA Rising   true
Require ADX           false
ADX Threshold        20
```

### Strong Demand

```text
Volume Multiplier    1.5
Min Range ATR        1.2
Min Close Location   0.70
```

### New High

```text
Lookback             20
Price Source          Close / High
Breakout Buffer ATR   0.0
```

## 12.3. Presets

UI nên có:

```text
Default
Conservative
Aggressive
Custom
```

nhưng backend vẫn lưu concrete parameter values để reproducible.

---

# 13. SIGNAL OUTPUT CONTRACT

Không chỉ trả boolean.

Recommended result shape:

```json
{
  "name": "vsa.strong_demand",
  "value": true,
  "score": 78.0,
  "version": 1,
  "params_hash": "...",
  "reasons": [
    "relative_volume=1.82 >= 1.50",
    "close_location=0.84 >= 0.70",
    "range_atr=1.43 >= 1.20"
  ]
}
```

Score optional cho signal binary thuần túy.

Frontend phải có khả năng hiển thị `reason` để debug/giải thích.

---

# 14. STRATEGY RULE COMPOSITION

## 14.1. Nguyên tắc

Strategy không viết Python trực tiếp.

User tạo rule bằng UI/YAML/JSON.

Example:

```yaml
entry:
  all:
    - pattern__bullish_any
    - technical_signal__bullish_any
    - volume__spike

exit:
  any:
    - technical_signal__bearish_any
    - vsa__upthrust

risk:
  stop_loss_pct: 7
  trailing_stop_pct: 10
```

## 14.2. Không mở rộng AST quá mức

Ưu tiên precompute signal Series rồi để AST chỉ kết hợp.

Không cho arbitrary function call hoặc unsafe eval.

## 14.3. Future extension

Có thể thêm safe operator sau nếu thật sự cần:

```text
at_least_n
within_last_n
count_true
```

nhưng chỉ sau khi có acceptance tests.

---

# 15. AUTOMATED BACKTEST — FUNCTIONAL REQUIREMENTS

## 15.1. User workflow

User phải có thể:

1. Chọn strategy.
2. Chọn danh sách ticker tùy ý.
3. Chọn một hoặc nhiều phase/date range.
4. Chọn capital per ticker.
5. Chọn fee/tax/slippage profile.
6. Chạy batch backtest.
7. Xem bảng kết quả theo từng phase.
8. Drill down từng ticker/trade.
9. Export kết quả.

## 15.2. Batch single-asset mode

Bảng mẫu yêu cầu mỗi ticker được test độc lập với cùng allocated capital.

Đây là:

```text
Batch Single-Asset Backtest
```

không phải shared-cash portfolio backtest.

Portfolio engine có thể phát triển sau.

## 15.3. Multi-phase

Example phases:

```text
Phase 1: 2021-03-20 → 2026-04-16
Phase 2: 2025-09-01 → 2026-04-16
Phase 3: 2026-03-01 → 2026-04-16
```

Indicator/signal không được tính lại từ đầu riêng cho từng phase nếu cùng run.

---

# 16. BACKTEST PIPELINE

```text
1. Resolve selected tickers/phases
2. Determine earliest required date - max warmup
3. Batch-load market data once
4. Compute indicators once per ticker
5. Compute BB features once per ticker/method where available
6. Compute signals once per ticker
7. Compile entry/exit boolean arrays once
8. Slice per phase
9. Run execution state machine
10. Build Trade[]
11. Aggregate metrics
12. Render/report/export
```

---

# 17. PERFORMANCE DESIGN

## 17.1. Compute once, slice many

Không làm:

```text
phase1 -> recalc everything
phase2 -> recalc everything
phase3 -> recalc everything
```

## 17.2. Batch DB loading

Recommended index:

```sql
CREATE INDEX IF NOT EXISTS idx_candles_symbol_ts_adj
ON candles(symbol, timestamp, adjustment_type);
```

Load nhiều ticker trong một query/batch khi hợp lý.

## 17.3. Vectorization

Pandas/NumPy đủ cho:

```text
EMA
RSI
MACD
rolling min/max
ATR
volume baseline
candle ratios
most pattern masks
```

Numba phù hợp cho:

```text
confirmed pivot scan
divergence pairing
stateful execution loop
trailing stop
settlement state
```

Không Numba hóa mọi thứ vô điều kiện.

## 17.4. Feature cache

Recommended cache key:

```text
symbol
timeframe
adjustment_type
last_data_timestamp
indicator_spec_hash
signal_spec_hash
flow_method
bb_spec_hash
```

Cache có thể dùng Parquet/columnar local cache.

---

# 18. NO LOOK-AHEAD CONTRACT

Đây là non-negotiable.

## 18.1. Indicator

Mọi rolling reference phải dùng dữ liệu đã available tại thời điểm bar hiện tại.

## 18.2. New High/Low

Reference window không chứa current bar.

## 18.3. Pivot/divergence

Pivot xảy ra tại `p`, signal chỉ được biết tại `p + right_bars`.

## 18.4. Daily signal execution

Nếu signal phụ thuộc close ngày T:

```text
signal known: after close T
entry earliest: next valid session/order event
```

Không fill ở close T trừ khi có intraday model chứng minh order có thể gửi trước close.

## 18.5. Regression test

Bắt buộc có truncation test:

```text
signal(t) calculated on full dataset
==
signal(t) calculated using data truncated at t
```

cho mọi causal column không có intentional confirmation delay.

---

# 19. VIETNAM MARKET EXECUTION RULES

Market rules phải nằm trong module riêng, không rải hard-code trong backtest loop.

## 19.1. Long-only

V1 mặc định long-only.

## 19.2. Settlement

Không biểu diễn đơn giản bằng floating `2.5 days`.

Model phải có settlement rule theo trading session/calendar.

Daily 1D data không thể biết chính xác high/low xảy ra sáng hay chiều ngày cổ phiếu về.

Vì vậy engine phải có documented conservative rule cho settlement-day exit nếu chỉ có daily OHLC.

## 19.3. Trading calendar

Không dùng calendar-day arithmetic.

Phải support:

```text
weekend
market holidays
special non-settlement days
```

## 19.4. Price bands / tick size / lot size

Không hard-code duy nhất ba con số vào execution loop.

Có `MarketRuleProvider` theo:

```text
exchange
date
security_status
```

Round lot default strategy có thể là 100, nhưng system design không nên giả định odd-lot không tồn tại.

## 19.5. Locked limit handling

Daily OHLC không chứng minh có thanh khoản tại trần/sàn.

Conservative fill policy cần support:

```text
limit_locked_up -> buy may be unfilled
limit_locked_down -> sell may be unfilled
```

---

# 20. PRICE STREAM VS ANALYSIS STREAM

Production design nên phân biệt:

```text
Analysis Price Stream
Execution Price Stream
```

Indicator/trend dài hạn nên có khả năng dùng adjusted series để tránh corporate action tạo false crash.

Execution simulation phải dùng tradable historical price semantics phù hợp.

Nếu chưa có corporate-action ledger đầy đủ, phải document giới hạn rõ ràng.

---

# 21. TRADE & POSITION MODEL

Recommended domain models:

```text
OrderIntent
Fill
Position
Trade
CashLedger
SettlementLot
BacktestRun
BacktestResult
```

Trade tối thiểu lưu:

```text
symbol
entry_signal_date
entry_fill_date
entry_price
quantity
entry_fee
exit_signal_date
exit_fill_date
exit_price
exit_fee
sell_tax
pnl
return_pct
bars_held
exit_reason
```

---

# 22. REQUIRED PERFORMANCE METRICS

Exact output columns:

```text
Ticker
Net Profit
% Net Profit
# Trades
Avg % Profit/Loss
Avg Bars Held
% of Winners
W. Avg % Profit
L. Avg % Loss
```

## 22.1. Trade PnL

```text
entry_cost = qty * buy_price + entry_fee
exit_net = qty * sell_price - exit_fee - sell_tax
pnl = exit_net - entry_cost
return_pct = 100 * pnl / entry_cost
```

## 22.2. Net Profit

Sum closed-trade PnL.

## 22.3. % Net Profit

```text
100 * NetProfit / InitialAllocatedCapital
```

## 22.4. # Trades

Số closed round-trip trades.

## 22.5. Avg % Profit/Loss

Mean `return_pct` của closed trades.

## 22.6. Avg Bars Held

Default:

```text
exit_bar_index - entry_bar_index
```

## 22.7. % of Winners

Theo yêu cầu hiện tại:

```text
winner = return_pct >= 0
```

Nếu sau này đổi convention `>0`, phải version metric spec.

## 22.8. W. Avg % Profit

Mean winner returns; `N/A` nếu không có winner.

## 22.9. L. Avg % Loss

Mean loser returns; `N/A` nếu không có loser. Giữ dấu âm.

## 22.10. Rounding

Không round intermediate calculation.

Chỉ format ở reporting layer, default 1 decimal nếu cần match bảng mẫu.

---

# 23. EDGE CASES BẮT BUỘC

Engine phải định nghĩa/test:

```text
0 trades
0 winners
0 losers
break-even trade
position open at phase end
entry too close to phase end to settle
missing bars
halt/suspension
holiday across settlement
locked ceiling/floor
corporate-action gap
insufficient warmup
NaN indicator periods
symbol listed after phase start
symbol delisted before phase end
```

---

# 24. PHASE BOUNDARY POLICY

Phải configurable và versioned.

Recommended benchmark mode:

- Không mở position mới nếu không thể legally exit theo settlement rule trước phase boundary, hoặc
- Cho phép mở nhưng mark unresolved/open-position metrics riêng.

Nếu `force_liquidate_at_phase_end = true`, phải document rằng đây là reporting assumption và không vi phạm settlement.

Không được âm thầm close một position chưa sellable.

---

# 25. OUTPUT TABLE REQUIREMENT

Renderer phải sinh đúng schema:

```markdown
| Ticker | Net Profit | % Net Profit | # Trades | Avg % Profit/Loss | Avg Bars Held | % of Winners | W. Avg % Profit | L. Avg % Loss |
```

Rules:

```text
missing metric -> N/A
loss -> preserve negative sign
numbers -> consistent decimal formatting
Ticker -> stable sort or configured ranking
```

Phase heading phải chứa date range và optional regime label.

---

# 26. WHOLE-MARKET / INDEX ANALYSIS

## 26.1. Price analysis

VNINDEX/VN30 có thể được chạy qua Price/Trend Indicator Engine tương tự một symbol nếu schema phù hợp.

## 26.2. Money Flow analysis

Whole-Market BB **không được thay thế bằng Volume của VNINDEX** nếu BB Spec yêu cầu aggregate member flows.

Phải support universe definitions:

```text
HOSE
HNX
UPCoM
VN30 constituents by date
custom universe
all eligible stocks
```

Universe phải point-in-time để tránh survivorship bias nếu lịch sử constituent data có sẵn.

---

# 27. DATA QUALITY & AUDIT

## 27.1. General candle audit

Check:

```text
duplicate bars
missing dates
negative/zero prices
negative volume
high < max(open, close)
low > min(open, close)
timezone/date alignment
adjusted/unadjusted consistency
```

## 27.2. Blackbox audit

Phải là task riêng trước production BB.

Audit output tối thiểu:

```text
Source name
Fields available
Granularity
Historical start/end
Aggressor side availability
Bid/ask availability
Foreign/proprietary availability
Known gaps
License/usage constraints
Recommended flow_method by date
```

---

# 28. EXCLUDED / DEFERRED FROM V1

Để giữ đơn giản, V1 không ưu tiên:

```text
machine learning signal generation
neural networks
complex polynomial curvature signals
institution/retail inference from OHLCV
large proprietary indicator zoo
volume profile / market profile unless separately specified
order-book microstructure signals if no reliable data
full portfolio optimizer
short selling
options/derivatives execution
parameter optimization for max historical profit
```

---

# 29. VERSIONING & REPRODUCIBILITY

Mỗi run phải lưu tối thiểu:

```text
strategy_version
indicator_spec_version
signal_spec_version
bb_spec_version
flow_method
market_rule_version
dataset_snapshot or last-data hash
adjustment_mode
initial_capital
commission_rate
sell_tax_rate
slippage_model
execution_model
settlement_model
phase definitions
parameter JSON
```

Không được sửa silently semantics của signal version đã dùng trong run cũ.

---

# 30. TEST STRATEGY

## 30.1. Unit tests

Cho từng indicator/signal:

```text
known synthetic pattern -> expected result
boundary threshold tests
NaN/warmup tests
parameter validation
```

## 30.2. No-lookahead tests

Bắt buộc cho:

```text
pivot
divergence
Ichimoku displacement
new high/low
rolling support/resistance
BB events
```

## 30.3. Execution tests

Bắt buộc:

```text
next-bar execution
T+ settlement
weekend/holiday
locked limit
stop loss before/after sellable
trailing stop
phase-end
fees/taxes
board-lot sizing
```

## 30.4. Metrics tests

Bắt buộc:

```text
0 trades
all winners
all losers
break-even winner convention
N/A formatting
rounding only at render stage
```

## 30.5. Golden/regression tests

Chọn một số ticker/date range cố định, lưu expected signals/trades/metrics và dùng để phát hiện regression.

---

# 31. IMPLEMENTATION PHASING

## Phase 0 — Discovery & Data Audit

- Audit codebase hiện tại.
- Audit actual data sources.
- Audit BB data availability.
- Map existing IndicatorEngine/BacktestService/RuleEvaluator.
- Không code lại các module đã có nếu có thể extend.

## Phase 1 — Core Feature Foundation

Implement/refactor:

```text
CandleFeatures
Volume relative metrics
Support/Resistance simple primitives
Confirmed pivots
SignalRegistry
Reason/evidence output
```

## Phase 2 — Core Signal V1

Implement:

```text
Bullish/Bearish Pattern
Bullish/Bearish Technical Signal
Volume Spike
Uptrend/Downtrend/Sideways
Correction/Recovery
New High/Low
Strong Demand/Weak Demand
Spring
Upthrust
Good/Bad Candle Structure
Ichimoku Good/Bad
Four-way Divergence
```

## Phase 3 — Blackbox Integration

Sau khi đọc BB Spec và hoàn tất data audit:

```text
MoneyFlowBlackbox adapter
flow_method handling
BB horizons
market aggregation
breadth
BB-derived events
```

## Phase 4 — Batch Backtest Upgrade

```text
precompute feature matrix
multi-ticker
multi-phase
settlement/calendar module
execution state machine
metrics/reporting
```

## Phase 5 — Caching / Performance / UX

```text
feature cache
Numba hotspots
preset UI
signal explanation UI
backtest compare/export
```

---

# 32. ACCEPTANCE CRITERIA — SYSTEM LEVEL

Sumi V1 được coi là đạt yêu cầu khi:

1. Một user không cần sửa code có thể cấu hình các signal phổ biến từ UI.
2. Signal result deterministic và reproducible.
3. Mỗi signal quan trọng có explanation/reason.
4. Không có future leak trong automated truncation tests.
5. Divergence chỉ phát sau pivot confirmation.
6. BB output luôn gắn `flow_method` rõ ràng.
7. Không blend True Flow và Proxy Flow như cùng series.
8. Whole-Market BB aggregate đúng theo BB Spec và có Breadth.
9. Strategy rule dùng AST safe evaluator, không unsafe eval.
10. 30–50 tickers × 3 phases có thể chạy trong thời gian tương tác hợp lý trên máy local sau khi feature computation/cache được tối ưu.
11. Backtest tôn trọng settlement/calendar policy đã document.
12. 9 metrics khớp specification và edge cases.
13. Kết quả có thể export/re-run với cùng version/config và cho cùng kết quả.

---

# 33. DELIVERABLES DEV/CODEX PHẢI TẠO

Tối thiểu:

```text
1. Codebase audit report
2. BB data audit report
3. SignalRegistry implementation
4. Signal modules + tests
5. MoneyFlowBlackbox integration adapter
6. Market calendar/settlement module
7. Batch backtest runner
8. Metrics module
9. API schemas/endpoints
10. UI configuration forms
11. Result tables/detail views
12. Migration files if persistence schema changes
13. Golden regression fixtures
14. Developer documentation
```

---

# 34. CODEX HANDOFF RULES

Khi giao tài liệu này cho Codex/DEV, phải kèm:

1. **Toàn bộ repository Sumi hiện tại.**
2. **Master Spec này.**
3. **Toàn bộ tài liệu nghiên cứu/Specification Money Flow Blackbox.**
4. Nếu có, sample output/table/reference screenshots.

Codex phải làm theo thứ tự:

```text
READ -> AUDIT -> MAP -> PLAN -> IMPLEMENT -> TEST
```

Không được bắt đầu bằng việc tạo hàng loạt file mới trước khi audit codebase.

### 34.1. BB-specific handoff requirement

Trước khi sửa bất cứ file nào liên quan tới Money Flow:

- Đọc toàn bộ BB research/spec.
- Tóm tắt lại bằng technical checklist.
- Xác định data mode khả dụng thực tế.
- Nếu data không đủ True Flow, không được giả lập `EXECUTED_ORDER_FLOW` từ OHLCV.
- Nếu dùng proxy, phải ghi `OHLCV_PROXY` xuyên suốt domain model/API/UI.

---

# 35. OPEN ISSUES / DECISIONS PENDING

Các phần sau **chưa được tự ý chốt trong code** nếu BB/data audit hoặc product owner chưa xác nhận:

```text
Exact BB thresholds 20/30/70/80
Exact definition of BB “uốn”
Confluence tolerance
Historical availability of true buyer/seller initiated flow
Hybrid date boundary between true flow and proxy flow
Exact broker fee preset(s)
Exact historical market-rule source and special-day handling
Corporate action treatment in execution stream
Phase-end liquidation policy default
```

Mọi open issue phải được đưa vào implementation checklist thay vì ngầm chọn một giá trị.

---

# 36. FINAL DESIGN SUMMARY

Sumi không cần trở thành một hệ thống chứa hàng trăm indicator.

Kiến trúc mục tiêu là:

```text
~10 familiar technical primitives
        +
small set of transparent candle/volume primitives
        +
Money Flow Blackbox as an independent subsystem
        ↓
Atomic Signals
        ↓
Simple Composite Signals (2–3 confirmations)
        ↓
Strategy DSL
        ↓
Causal Vietnam-aware Backtest
        ↓
Auditable Metrics
```

Triết lý xuyên suốt:

> **Đơn giản hơn nhưng kiểm chứng được tốt hơn.**

> **Không coi proxy là dữ liệu thật.**

> **Không để Signal Engine, Blackbox và Backtest trộn trách nhiệm.**

> **Mọi signal phải giải thích được và mọi backtest phải tái lập được.**

---

# APPENDIX A — RECOMMENDED V1 SIGNAL CATALOG

```text
PATTERN
- bullish_engulfing
- hammer
- bullish_pinbar
- morning_star
- piercing_line
- tweezer_bottom
- inside_bar_breakout_up
- bearish_engulfing
- shooting_star
- bearish_pinbar
- evening_star
- dark_cloud_cover
- tweezer_top
- inside_bar_breakdown
- bullish_any
- bearish_any

TECHNICAL_SIGNAL
- macd_cross_up/down
- macd_zero_up/down
- rsi_level_cross_up/down
- ema_cross_up/down
- swing_break_up/down
- bullish_any
- bearish_any

VOLUME
- relative_volume
- volume_spike
- volume_climax_up
- volume_climax_down

REGIME
- uptrend
- downtrend
- sideways
- correction
- recovery
- new_high
- new_low

VSA
- strong_demand
- weak_demand
- upthrust
- failed_breakout_high_volume
- spring
- strong_demand_at_support
- strong_supply_at_resistance

STRUCTURE
- candle_structure_score
- good_candle
- bad_candle
- technical_health_score
- favorable
- unfavorable

ICHIMOKU
- ichimoku_score
- bullish
- bearish

DIVERGENCE
- rsi_regular_bullish
- rsi_hidden_bullish
- rsi_regular_bearish
- rsi_hidden_bearish
- macdh_regular_bullish
- macdh_hidden_bullish
- macdh_regular_bearish
- macdh_hidden_bearish
- stochastic_regular_bullish
- stochastic_hidden_bullish
- stochastic_regular_bearish
- stochastic_hidden_bearish

FLOW
- BB03
- BB05
- BB20
- BB50
- BB200
- market_BB*
- breadth_*
- BB-derived events per BB Spec
```

---

# APPENDIX B — RECOMMENDED CORE INDICATORS

Ưu tiên giữ tập nhỏ:

```text
EMA
RSI
CCI
MACD
ATR
ADX
Bollinger Bands
Stochastic
Ichimoku
Volume SMA / Relative Volume
Confirmed Swing High/Low
```

MFI có thể giữ như technical indicator tham khảo nhưng **không mặc định là core Money Flow Blackbox**.

---

# APPENDIX C — DOCUMENT PRECEDENCE

Khi hai bộ tài liệu được giao cùng nhau:

```text
Master Functional & Technical Spec
    ├─ authoritative về system scope, integration, module boundaries, backtest, signal architecture
    │
    └─ references
         Money Flow Blackbox Research / Specification
             └─ authoritative về BB mathematics, flow semantics, data modes, horizons, market aggregation, breadth
```

Nếu conflict:

```text
Money Flow topic -> BB Spec wins
System integration/backtest topic -> Master Spec wins
```

# APPENDIX D — IMPLEMENTATION-GRADE SIGNAL CONTRACTS

Appendix này khóa contract tối thiểu cho V1. Các công thức là baseline đơn giản; nếu codebase hiện có một implementation tương đương nhưng semantics khớp và đã test, DEV SHOULD reuse/refactor thay vì viết lại.

## D.0. Shared Candle/Volume Primitives

Các primitive dưới đây SHOULD được tính một lần và reuse:

```text
range            = max(high - low, epsilon)
body             = abs(close - open)
upper_wick       = high - max(open, close)
lower_wick       = min(open, close) - low
body_ratio       = body / range
upper_wick_ratio = upper_wick / range
lower_wick_ratio = lower_wick / range
close_location   = (close - low) / range           # 0..1
atr_ratio        = range / ATR14_previous_or_causal
relative_volume  = volume / SMA(volume.shift(1), N)
```

Rules:

- `epsilon` chỉ để tránh divide-by-zero; candle zero-range phải có deterministic result.
- Relative-volume baseline MUST exclude current bar.
- ATR/reference series MUST obey causal availability.
- Shared primitives SHOULD được materialize trong feature frame để tránh mỗi signal tính lại.

### D.0.1. Common output contract

Mỗi signal output SHOULD support:

```text
value: bool | float | enum | null
score: float | null
reasons: list[str]
version: int/string
params_hash: string
warmup_complete: bool
data_quality: GOOD | DEGRADED | INSUFFICIENT | MISSING
available_at: timestamp/date
```

`available_at` đặc biệt quan trọng cho confirmed pivot/divergence.

---

## D.1. Pattern contracts

### `SIG-PAT-001` Bullish Engulfing

**Inputs:** O/H/L/C  
**Default formula:**

```text
prev_bearish = close[-1] < open[-1]
curr_bullish = close > open
body_engulf  = open <= close[-1] AND close >= open[-1]
quality      = body_ratio >= 0.55 AND close_location >= 0.70
signal       = prev_bearish AND curr_bullish AND body_engulf AND quality
```

**UI parameters:** `min_body_ratio`, `min_close_location`, `require_quality`.  
**Look-ahead:** none.  
**Tests:** exact engulf boundary; zero body previous bar; gap; NaN warmup.

### `SIG-PAT-002` Bearish Engulfing

Symmetric to `SIG-PAT-001`.

### `SIG-PAT-003` Hammer / Bullish Pinbar

Baseline:

```text
lower_wick >= 2.0 * max(body, epsilon)
upper_wick_ratio <= 0.25
close_location >= 0.65
body_ratio <= 0.50
```

Optional context: `close < EMA20 OR return_N < negative_threshold`. Context MUST be configurable; no future data.

### `SIG-PAT-004` Shooting Star / Bearish Pinbar

Symmetric wick/close-location logic.

### `SIG-PAT-005` Morning Star

Three-bar baseline:

```text
bar t-2 bearish AND body_ratio[t-2] >= 0.55
body[t-1] <= 0.40 * body[t-2]
bar t bullish
close[t] >= midpoint(open[t-2], close[t-2])
close_location[t] >= 0.65
```

Gap MUST NOT be mandatory by default for Vietnam-market V1.

### `SIG-PAT-006` Evening Star

Symmetric to Morning Star.

### `SIG-PAT-007` Piercing Line / Dark Cloud Cover

Use prior-body midpoint recovery/rejection with configurable close-location quality filter.

### `SIG-PAT-008` Tweezer Bottom/Top

Default equality tolerance:

```text
abs(reference_low_or_high[t] - [t-1]) <= tolerance_atr * ATR14
```

Default `tolerance_atr = 0.15`.

### `SIG-PAT-009` Inside Bar Breakout

```text
inside[t-1] = high[t-1] < high[t-2] AND low[t-1] > low[t-2]
up_break[t] = close[t] > high[t-2] + breakout_buffer_atr * ATR14[t]
down_break[t] = close[t] < low[t-2] - breakout_buffer_atr * ATR14[t]
```

Default `breakout_buffer_atr = 0.05`.

### `SIG-PAT-010` Bullish/Bearish Any

Aggregate selected pattern booleans; return reason list containing exact active child patterns. Default semantic is `ANY`, not a new pattern algorithm.

---

## D.2. Technical trigger contracts

### `SIG-TECH-001` MACD Signal Cross

```text
up   = macd[t-1] <= signal[t-1] AND macd[t] > signal[t]
down = macd[t-1] >= signal[t-1] AND macd[t] < signal[t]
```

### `SIG-TECH-002` MACD Zero Cross

```text
up   = macd[t-1] <= 0 AND macd[t] > 0
down = macd[t-1] >= 0 AND macd[t] < 0
```

### `SIG-TECH-003` RSI Level Cross

Configurable `level` (common presets 30/50/70, but no forced trading semantics):

```text
cross_up   = rsi[t-1] <= level AND rsi[t] > level
cross_down = rsi[t-1] >= level AND rsi[t] < level
```

### `SIG-TECH-004` EMA Cross / Alignment Transition

Support `fast_period`, `slow_period`; cross event distinct from persistent alignment state.

### `SIG-TECH-005` Swing Break

Break only a **confirmed** swing reference. Breakout buffer configurable as ATR/percent.

### `SIG-TECH-006` Bullish/Bearish Composite Trigger

Config:

```text
children: selected technical triggers
minimum_confirmations: default 2
```

Do not default to OR-all. Reasons MUST identify active children/count.

---

## D.3. Volume contracts

### `SIG-VOL-001` Relative Volume

```text
baseline_N[t] = mean(volume[t-N : t])     # prior N bars, excludes current
relative_volume[t] = volume[t] / baseline_N[t]
```

Default `N=20`.

### `SIG-VOL-002` Volume Spike

```text
relative_volume >= multiplier
```

Default `multiplier=2.0`. Optional bullish/bearish candle filter and minimum range/ATR.

### `SIG-VOL-003` Volume Climax Up/Down

Baseline candidate:

```text
relative_volume >= 2.0
range / ATR14 >= 1.5
close_location >= 0.75   # up climax
close_location <= 0.25   # down climax
```

This is a technical label, not evidence of investor identity.

---

## D.4. Regime contracts

### `SIG-REG-001` Uptrend

Default minimal rule:

```text
close > EMA_fast > EMA_slow
EMA_fast[t] > EMA_fast[t-slope_lookback]
```

Defaults: `fast=20`, `slow=50`, `slope_lookback=5`.  
Optional confirmations: `ADX >= threshold`, HH/HL confirmed structure. Do not enable all by default.

### `SIG-REG-002` Downtrend

Symmetric.

### `SIG-REG-003` Sideways

V1 SHOULD expose a preset rather than many raw switches. Default candidate:

```text
ADX14 < 18
AND BollingerBandwidth <= rolling_percentile_threshold
```

Alternative simple preset may use EMA slope near zero. Final preset values MAY be tuned statistically for robustness but MUST NOT be selected solely by P&L optimization.

### `SIG-REG-004` Correction / Pullback

Required context:

```text
recent/dominant uptrend == true
recent_high = max(high of prior lookback)
drawdown_pct within [min,max]
trend_not_broken = close >= selected_support_reference - tolerance
```

Defaults candidate: `lookback=20`, `min=2%`, `max=12%`, `support=EMA50`. Diminishing volume is optional confirmation.

### `SIG-REG-005` Recovery

Required context: recent downtrend. Default simple confirmation uses at most two of:

```text
close > EMA20
EMA20 rising
break confirmed short swing high
RSI cross configured recovery level
```

Recovery MUST remain distinct from Uptrend.

### `SIG-REG-006` New High / New Low

```text
prior_high_N = max(high.shift(1), N)
new_high = selected_price > prior_high_N + buffer
```

Analogous for low. Defaults: common presets 20/50/252; `buffer=0` or small ATR buffer. Current bar MUST NOT be part of reference window.

---

## D.5. Supply/Demand / VSA contracts

### `SIG-VSA-001` Strong Demand

Default baseline:

```text
close > close[-1]
range / ATR14 >= 1.2   # configurable; conservative preset may use 1.4
body_ratio >= 0.50
close_location >= 0.70
relative_volume >= 1.5
```

Expose 3–4 understandable parameters only.

### `SIG-VSA-002` Weak Demand / No Demand

Baseline:

```text
close >= close[-1]
range / ATR14 <= 0.70
relative_volume <= 0.80
```

Optional: current volume < prior one/two bars.

### `SIG-VSA-003` Upthrust / Simplified Kéo Xả

Reference resistance MUST use only prior/confirmed data.

```text
high > resistance + breach_buffer
close < resistance
upper_wick_ratio >= 0.35
close_location <= 0.45
relative_volume >= 1.5
```

`kéo_xả` MAY aggregate `upthrust`, `failed_breakout_high_volume`, `high_volume_rejection`; it MUST NOT imply proven distribution by institutions.

### `SIG-VSA-004` Spring / Shakeout / Đạp Kéo

```text
low < support - breach_buffer
close > support
lower_wick_ratio >= 0.35
close_location >= 0.70
relative_volume >= 1.30
```

Default support lookback candidate `20`; tolerance/buffer ATR-based.

### `SIG-VSA-005` Strong Demand at Support

```text
near_support AND (strong_demand OR spring)
```

### `SIG-VSA-006` Strong Supply at Resistance

```text
near_resistance AND (upthrust OR strong_bearish_rejection)
```

---

## D.6. Support/Resistance contract

### `SIG-SR-001` Near Support / Resistance

Allowed sources V1:

```text
rolling N-bar low/high excluding current reference bar
confirmed swing low/high
EMA20/50/200
Kijun (optional)
```

Distance:

```text
abs(price - level) <= tolerance_atr * ATR
```

or configurable percent mode. Default SHOULD use ATR normalization. If multiple sources are enabled, reasons MUST state which source matched.

---

## D.7. Candle structure contract

### `SIG-STR-001` Candle Structure Score

Baseline transparent score:

```text
direction = +1 if close>open, -1 if close<open, else 0
close_score = 2*close_location - 1
wick_balance = (lower_wick - upper_wick) / range
raw = 0.45*direction*body_ratio + 0.35*close_score + 0.20*wick_balance
score = 100 * clip(raw, -1, +1)
```

Default:

```text
good_candle = score >= +40
bad_candle  = score <= -40
```

Weights SHOULD be fixed as named presets in ordinary UI; custom weights MAY be advanced-only. No P&L weight fitting in V1.

---

## D.8. Technical Health Score contract

### `SIG-HEALTH-001` Technical Health

Goal: summarize **different information families**, not average every available indicator.

Recommended V1 default preset uses four components:

```text
Trend      : EMA alignment/slope
Momentum   : RSI OR CCI   # choose one by preset
Transition : MACD histogram/signal state
Participation: Relative Volume OR BB flow confirmation if available
```

Each component maps to `[-1,+1]`, then weighted average → `[-100,+100]`.

Rules:

- RSI and CCI SHOULD NOT both receive large weights in the same default preset because information overlaps.
- Volume and BB MUST remain identifiable reasons; BB absence must not silently become negative score.
- Default thresholds candidate: `favorable >= +35`, `unfavorable <= -35`.
- Exact weight preset MUST be versioned and displayed in metadata.

---

## D.9. Ichimoku contract

### `SIG-ICHI-001` Ichimoku Score

Compute causal Tenkan/Kijun/raw spans using standard periods configured by IndicatorEngine. Visible historical cloud MUST honor displacement.

Default bullish factors:

```text
1. close > visible_cloud_top
2. Tenkan > Kijun
3. Chikou clearance bullish using causal comparison semantics
4. raw Span A > raw Span B (future-cloud orientation known at t)
5. Kijun slope positive (optional factor/preset)
```

Default production preset SHOULD use 4 core factors and treat Kijun slope as optional unless existing project semantics already include it.

Output:

```text
ichimoku_score: integer count / normalized score
ichimoku_bullish
ichimoku_bearish
reasons[]
```

Test MUST demonstrate no future-cloud leak.

---

## D.10. Divergence contract

### `SIG-DIV-001` Confirmed Pivot

Default research/production baseline:

```text
left_bars=3
right_bars=3
```

A price pivot occurring at `p` is only available at `p + right_bars`.

Persist/emit:

```text
pivot_at
confirmed_at
pivot_type
pivot_price
```

### `SIG-DIV-002..005` Four-way divergence

Pair consecutive compatible confirmed pivots satisfying configurable separation:

```text
min_separation=5
max_separation=60
```

Definitions:

```text
Regular Bullish: price LL + oscillator HL
Hidden Bullish : price HL + oscillator LL
Regular Bearish: price HH + oscillator LH
Hidden Bearish : price LH + oscillator HH
```

Supported oscillators V1: RSI, MACD Histogram, Stochastic.

Rules:

- Signal availability = max confirmation time of all required pivots.
- Hidden divergence SHOULD require matching trend context preset.
- MACD Histogram comparison SHOULD normalize scale when needed; exact normalization implementation/version must be deterministic.
- Never backdate signal to `pivot_at` in backtest.

---

## D.11. Money Flow event integration contract

### `BBI-SIG-001`

Signal Engine MAY consume BB outputs such as:

```text
BB03/05/10/20/50/200
flow_method
data_quality
direction
regime
breadth
```

and parameterized events defined by BB Spec.

### `BBI-SIG-002`

`TURN_UP(20/30)`, `TURN_DOWN(70/80)`, confluence and regime events MUST follow BB companion spec. Master MUST NOT freeze thresholds/tolerances before BB research evaluation.

### `BBI-SIG-003`

A strategy using BB MUST be able to require a permitted `flow_method`/minimum `data_quality` to avoid silently mixing semantic modes between tickers/periods.

---

# APPENDIX E — API, DOMAIN & PERSISTENCE CONTRACTS

## E.1. Signal Definition API

Conceptual Pydantic model:

```python
class SignalDefinition(BaseModel):
    name: str
    version: str
    category: str
    label_vi: str
    description: str
    output_type: Literal['bool','float','enum']
    parameters_schema: dict
    default_parameters: dict
    dependencies: list[str]
    warmup_bars: int
    causal_delay_bars: int
    status: Literal['ACTIVE','EXPERIMENTAL','DEPRECATED']
```

`GET /api/signals/registry` SHOULD return registry metadata for dynamic UI generation.

## E.2. Signal Calculation Request

Conceptual request:

```json
{
  "symbols": ["NKG", "SSI"],
  "timeframe": "1D",
  "start": "2025-01-01",
  "end": "2026-04-16",
  "signals": [
    {"name": "volume.spike", "params": {"period": 20, "multiplier": 2.0}},
    {"name": "regime.uptrend", "params": {"fast_ema": 20, "slow_ema": 50}}
  ],
  "include_reasons": true
}
```

Actual route naming MAY adapt to current API conventions after codebase audit.

## E.3. Strategy configuration contract

```json
{
  "strategy_id": "...",
  "strategy_version": 1,
  "entry": {
    "all": [
      "pattern__bullish_any",
      "technical_signal__bullish_any",
      "volume__spike"
    ]
  },
  "exit": {
    "any": [
      "technical_signal__bearish_any",
      "vsa__upthrust"
    ]
  },
  "risk": {
    "stop_loss_pct": 7.0,
    "trailing_stop_pct": 10.0
  }
}
```

Persist **concrete resolved parameter values**, not only preset name.

## E.4. Backtest Request Contract

```json
{
  "strategy_version_id": "...",
  "symbols": ["NKG", "NLG", "NVL", "SSI", "STB"],
  "phases": [
    {"id": "full_cycle", "start": "2021-03-20", "end": "2026-04-16", "label": "Toàn chu kỳ"},
    {"id": "sideways", "start": "2025-09-01", "end": "2026-04-16", "label": "Đi ngang"},
    {"id": "hard", "start": "2026-03-01", "end": "2026-04-16", "label": "Khó khăn"}
  ],
  "capital_per_symbol": 100000000,
  "execution_profile": "VN_EQUITY_DAILY_V1",
  "fee_profile": "...",
  "force_liquidate_at_phase_end": false
}
```

Validation MUST reject duplicate/invalid phases, unknown symbols according to current project policy, non-positive capital, incompatible timeframe/execution profile.

## E.5. Backtest Result Contract

Top-level:

```text
run_id
run_metadata
phase_results[]
warnings[]
data_quality_summary
```

Each phase result:

```text
phase_id
start/end
rows[]        # exact 9 benchmark metrics per ticker
aggregate_diagnostics optional
```

Each row MUST preserve raw numeric values; markdown/UI formatting happens in presentation layer.

## E.6. Trade ledger contract

A closed trade MUST contain enough data to reproduce metrics:

```text
trade_id
symbol
entry_signal_at
entry_order_at
entry_fill_at
entry_price
quantity
entry_fee
settlement_available_at
exit_signal_at
exit_order_at
exit_fill_at
exit_price
exit_fee
sell_tax
pnl
return_pct
bars_held
exit_reason
```

Open/unresolved positions at phase end MUST not masquerade as closed trades.

## E.7. Money Flow integration payload

Do not redefine BB math here. Integration requires at minimum the companion schema fields:

```text
methodology_version
scope_type / scope_id
calculation_date
horizon
flow_method
value_source
bb_score
oib_raw
buy_value/sell_value when applicable
classification_coverage
warmup_complete
data_quality
direction/regime
breadth fields for aggregate scopes
```

API/UI MUST preserve method boundaries.

## E.8. Persistence policy

Prefer storing:

```text
strategy definitions/versions
backtest run metadata
trade ledger/results
optional computed-feature cache metadata
BB methodology/data-quality metadata when persisted
```

Do not immediately persist every transient signal boolean for all 3,734 symbols unless profiling/use case demonstrates value. Feature cache may remain columnar files keyed by spec/data hash.

---

# APPENDIX F — UI FUNCTIONAL CONTRACT

## F.1. Signal Catalog UI

`UI-SIG-001` User MUST be able to browse signal by category: Pattern, Technical Signal, Volume, Regime, VSA, Structure, Ichimoku, Divergence, Flow.

`UI-SIG-002` Each signal card/form MUST show:

```text
Vietnamese label
short meaning
parameters + defaults
current version
experimental badge if applicable
```

`UI-SIG-003` Default view SHOULD hide advanced/research parameters.

## F.2. Explanation UI

`UI-EXP-001` For a selected symbol/date, UI SHOULD display active signal + score + reasons.

Example:

```text
Cầu mạnh ✓   Score 78
- Relative volume 1.82 >= 1.50
- Close location 0.84 >= 0.70
- Range/ATR 1.43 >= 1.20
```

## F.3. BB UI

BB UI MUST follow companion spec labels and quality/method badges. It MUST NOT label `OHLCV_PROXY` as actual capital inflow/outflow.

Default chart SHOULD expose BB03/05/20/50/200 with BB10 optionally toggled, while backend supports all canonical horizons.

## F.4. Strategy Builder

User SHOULD construct rules from registered fields/operators rather than typing arbitrary Python. UI validates incompatible parameter types before submit.

## F.5. Backtest UI

At minimum:

```text
Strategy selector/version
Ticker multi-select
Phase editor
Capital per ticker
Fee/tax/slippage profile
Run button
Progress/result state
Phase tabs/tables
Ticker trade drill-down
Export
Warnings/data-quality panel
```

Backtest warnings MUST surface assumptions such as proxy flow, insufficient warmup, unresolved open positions, missing data, conservative settlement handling.

---

# APPENDIX G — DATA CONTRACT & AUDIT CHECKLIST

## G.1. Candle data requirements

`DATA-CANDLE-001` Unique key SHOULD be at least `(symbol, timestamp, adjustment_type/timeframe if applicable)`.

`DATA-CANDLE-002` Validate:

```text
open/high/low/close finite and positive where instrument semantics require
high >= max(open, close, low)
low <= min(open, close, high)
volume >= 0
no duplicate active bars
ordered trading sessions
```

`DATA-CANDLE-003` Missing session is not automatically a zero-volume bar; use exchange calendar/security status to distinguish suspension/no trading/missing vendor record.

## G.2. Corporate actions

Audit whether adjusted/unadjusted histories are internally consistent. Document which stream feeds indicator calculations and which feeds execution. If historical execution cannot be reconstructed correctly around corporate actions, flag affected runs/data quality rather than silently fabricating fills.

## G.3. BB Data Audit Gate

Master requires the companion BB deliverables:

```text
source_catalog.md
sample payloads
field semantic notes
data_capability_matrix.csv
coverage/missingness/latency report
recommended flow_method by source x exchange x date range
```

No production TRUE_FLOW implementation decision before this gate.

## G.4. Point-in-time universe

Whole-market/breadth calculations REQUIRE symbol lifecycle/eligibility policy. Do not backfill current survivors into historical breadth. If lifecycle data is unavailable, market historical output must carry explicit limitation/quality state.

---

# APPENDIX H — BACKTEST EXECUTION CONTRACT

## H.1. Signal-to-order timing

`BT-TIME-001` Daily signal depending on close of session T is available only after that calculation cutoff. Default earliest fill is next valid execution session/event, normally T+1 open for daily model.

`BT-TIME-002` A strategy MAY use open-known information differently only if a separate execution model explicitly specifies availability timing.

## H.2. Position sizing

V1 Batch Single-Asset mode:

```text
allocated_capital = configured capital per symbol
buy_price_effective = execution price including slippage model
quantity = floor(available_cash / estimated_total_unit_cost / lot_size) * lot_size
```

Exact fee-aware sizing MUST avoid spending more cash than allocated after fees.

## H.3. Settlement

Settlement MUST use trading-session calendar, not `timedelta(days=N)`.

For 1D data, if shares become sellable during settlement-day PM but intraday high/low ordering is unknowable, the execution profile MUST document conservative treatment. A safe baseline is:

```text
before sellable session: no exit
settlement-day PM availability: close-based exit may be allowed if consistent with model
full daily high/low stop logic: only from a session where whole tradable bar is sellable
```

Exact historical Vietnam rule/source/date handling remains a MarketRuleProvider responsibility and MUST be versioned.

## H.4. Stop/Trailing precedence

Execution profile MUST define deterministic precedence when multiple exits are triggered on the same daily bar and intrabar order is unknowable. Options such as conservative worst-case fill are acceptable if documented/versioned. Do not silently choose the most profitable order.

## H.5. Limit locked

If daily bar indicates a completely locked price and no liquidity evidence exists, conservative profile SHOULD reject fills that require unavailable opposite liquidity. Implementation must be covered by golden fixtures.

## H.6. Phase end

The engine MUST distinguish:

```text
closed trades
open positions
positions unsellable due to settlement
forced liquidation assumption
```

`force_liquidate_at_phase_end=true` may only close if the execution model says the position is legally sellable; otherwise report unresolved position.

---

# APPENDIX I — METRIC CONTRACT & EXAMPLES

## I.1. Exact calculations

For each closed trade `i`:

```text
entry_gross_i = qty_i * entry_price_i
entry_cost_i  = entry_gross_i + entry_fee_i
exit_gross_i  = qty_i * exit_price_i
exit_net_i    = exit_gross_i - exit_fee_i - sell_tax_i
pnl_i         = exit_net_i - entry_cost_i
return_pct_i  = 100 * pnl_i / entry_cost_i
bars_held_i   = exit_bar_index_i - entry_bar_index_i
```

Then:

```text
Net Profit          = sum(pnl_i)
% Net Profit        = 100 * Net Profit / InitialAllocatedCapital
# Trades            = count(closed trades)
Avg % Profit/Loss   = mean(return_pct_i)
Avg Bars Held       = mean(bars_held_i)
% of Winners        = 100 * count(return_pct_i >= 0) / #Trades
W. Avg % Profit     = mean(return_pct_i where >= 0) else N/A
L. Avg % Loss       = mean(return_pct_i where < 0) else N/A
```

## I.2. Zero-trade semantics

Recommended output:

```text
Net Profit        0
% Net Profit      0
# Trades          0
Avg % P/L         N/A
Avg Bars Held     N/A
% of Winners      N/A
W. Avg % Profit   N/A
L. Avg % Loss     N/A
```

## I.3. Monetary unit

Core calculations SHOULD store base currency amount (VND/Decimal or declared numeric representation). `Net Profit` reporting may display thousand VND only via explicit formatter/unit metadata. Never change unit silently.

---

# APPENDIX J — TEST & ACCEPTANCE TRACEABILITY

## J.1. Core acceptance tests

| ID | Requirement | Acceptance |
|---|---|---|
| `TEST-SIG-001` | Signal determinism | Same input/config/version => identical signal frame. |
| `TEST-SIG-002` | Warmup | No signal claims before required history except explicitly partial-capable features. |
| `TEST-SIG-003` | Explanation | Active composite signal lists active child reasons. |
| `TEST-CAUSAL-001` | Future invariance | Appending/changing future rows cannot alter previously available causal outputs. |
| `TEST-CAUSAL-002` | Pivot delay | Pivot/divergence signal cannot appear before `confirmed_at`. |
| `TEST-CAUSAL-003` | New high/low | Current bar excluded from comparison reference. |
| `TEST-ICHI-001` | Ichimoku | Historical cloud/displacement fixture proves no future leak. |
| `TEST-BB-001` | BB integration | `flow_method`/quality preserved end-to-end; method boundary visible. |
| `TEST-BB-002` | BB source of truth | Master implementation delegates BB math to BB module/spec; no duplicate alternate formula in SignalEngine. |
| `TEST-DSL-001` | AST safety | Arbitrary calls/imports/attribute escapes rejected. |
| `TEST-BT-001` | Next-bar timing | Close-derived signal never fills same close in default daily model. |
| `TEST-BT-002` | Settlement | Position cannot exit before sellable trading session. |
| `TEST-BT-003` | Holiday | Settlement crosses weekend/holiday using trading calendar correctly. |
| `TEST-BT-004` | Limit lock | Conservative profile rejects impossible locked-limit fills. |
| `TEST-BT-005` | Phase end | Unsellable/open positions are not counted as closed trades. |
| `TEST-MET-001` | 9 metrics | Golden trade ledger produces exact expected metric values. |
| `TEST-MET-002` | N/A | No winner/loser/zero-trade cases format as specified. |
| `TEST-PERF-001` | Compute once | Instrumentation shows feature computation is not repeated for each phase in same run. |
| `TEST-REPRO-001` | Reproducibility | Stored run metadata can rerun same snapshot/config to same result within declared tolerance. |

## J.2. Companion BB tests

All BB acceptance tests `AT01..AT20` defined in the BB V1 Research & DEV Spec are mandatory for the BB workstream where applicable. Master test suite SHOULD reference/execute those tests rather than copy divergent variants.

---

# APPENDIX K — CODEBASE INTEGRATION PLAN

Actual file names MUST be confirmed by audit. Based on known codebase context, expected mapping is:

```text
backend/app/domain/engine/indicator_engine.py
    - retain existing indicator responsibility
    - expose/reuse calculated Series/DataFrames

backend/app/domain/signals/
    registry.py
    candle_features.py
    patterns.py
    technical.py
    volume.py
    regimes.py
    support_resistance.py
    vsa.py
    structure.py
    health.py
    ichimoku_signals.py
    divergence.py
    flow_events.py

backend/app/domain/money_flow/
    - integration boundary following BB Spec
    - exact files decided after BB data audit

backend/app/domain/backtest/
    models.py
    execution.py
    metrics.py
    batch_runner.py

backend/app/domain/market/
    calendar.py
    settlement.py
    rules.py

backend/app/services/backtest_service.py
    - orchestration/adaptation layer
    - move domain calculations out if currently monolithic
```

This is a **target map, not permission to blindly create all files**. DEV first maps existing repository and reuses equivalent modules.

## K.1. Migration principle

Prefer incremental migration:

```text
existing behavior + tests
        ↓
extract primitive/module
        ↓
add new signal contract
        ↓
wire registry/API
        ↓
backward-compatibility check
```

Do not perform a large rewrite and signal expansion simultaneously.

---

# APPENDIX L — DEFINITION OF DONE BY PHASE

## L.0. Phase 0 — Audit

Done only when:

```text
repository architecture mapped
current indicator/rule/backtest behavior documented
DB schema/indexes audited
sample datasets inspected
BB Data Audit Gate deliverables created or explicitly scheduled with owner
open decisions recorded
implementation plan maps requirement IDs -> code/tests
```

## L.1. Phase 1 — Feature Foundation

Done only when:

```text
shared candle/volume primitives implemented/tested
support/resistance primitive implemented/tested
confirmed pivot primitive implemented/tested
SignalRegistry serves metadata
reason/evidence contract works end-to-end for at least one signal
future-invariance harness exists
```

## L.2. Phase 2 — Core Signals

Done only when:

```text
all KEEP V1 non-BB signals implemented or explicitly waived by PO
unit/boundary/warmup tests pass
causal tests pass
UI/API can query definitions and resolved parameters
representative chart/date manual validation completed
```

## L.3. Phase 3 — BB Integration

Done only when:

```text
DEV has read companion BB Spec
data mode decision supported by audit evidence
BB module passes applicable AT01..AT20
flow_method/data_quality preserved through API/UI
Signal Engine consumes BB read-only outputs
no hidden TRUE_FLOW/PROXY splice
```

## L.4. Phase 4 — Backtest

Done only when:

```text
batch multi-ticker/multi-phase run works
feature computation reused across phases
settlement/calendar execution tests pass
9 metric golden tests pass
trade drill-down reconciles to summary
open-position/phase-end semantics visible
```

## L.5. Phase 5 — Performance & UX

Done only when:

```text
profiling identifies real hotspots
cache invalidation/version keys verified
Numba only applied where benchmark justifies it
interactive benchmark recorded on reference machine/data
signal config and explanation UI usable without code edits
exports preserve units/N/A/signs/version metadata
```

---

# APPENDIX M — IMPLEMENTATION HANDOFF CHECKLIST FOR CODEX

Before code changes, Codex/DEV MUST produce a short audit response containing:

```text
A. Existing modules that can be reused
B. Existing modules that conflict with this spec
C. Data tables/sources actually present
D. BB capability tier by source/date where known
E. Backtest timing/settlement behavior currently implemented
F. Gaps against requirement IDs
G. Proposed implementation order
H. Files expected to modify/create
I. Tests expected to add
J. Open decisions requiring Product Owner
```

Then implementation proceeds in small reviewable batches. Every batch SHOULD state which requirement IDs it closes.

---

# APPENDIX N — FINAL SOURCE-OF-TRUTH RULES

1. **Master Spec** is authoritative for product scope, Price/Volume signal architecture, Strategy DSL, integration, backtest, metrics, UI contracts and delivery process.
2. **Money Flow Blackbox V1 Research & DEV Spec** is authoritative for BB terminology, formulas, data tiers, mode selection, BB horizons, whole-market aggregation, breadth, BB events/research gates and BB acceptance tests.
3. Current codebase behavior is not automatically authoritative merely because it exists; conflicts must be surfaced during audit.
4. External library behavior is not authoritative for Sumi semantics unless explicitly adopted and regression-tested.
5. Defaults in this Master marked **candidate/gợi ý/research** are not license to optimize on P&L; freeze them only through documented design/validation decision.
6. Any material semantic change requires a version bump so historical backtests remain interpretable.

---

# FINAL HANDOFF STATEMENT

Bộ bàn giao tối thiểu cho DEV/Codex gồm **hai tài liệu**:

```text
1. SUMI_MASTER_FUNCTIONAL_TECHNICAL_SPEC_FINAL.md
2. SUMI Money Flow Blackbox V1 — Research & DEV Spec
```

Không cần một bộ tài liệu lớn hơn nếu hai tài liệu này được đọc đầy đủ và DEV thực hiện đúng quy trình audit-first.

Mục tiêu của Master không phải biến mọi ý tưởng thành code. Mục tiêu là khóa một hệ thống V1 có đặc tính:

```text
simple
causal
configurable
explainable
versioned
reproducible
Vietnam-market aware
```

và giữ ranh giới rõ giữa:

```text
PRICE        -> technical state of price
VOLUME       -> participation/activity confirmation
MONEY FLOW   -> BB subsystem with explicit measurement method
SIGNAL       -> deterministic interpretation of primitives
STRATEGY     -> user-configured composition
BACKTEST     -> execution simulation only
METRICS      -> closed-trade evaluation
```

**DEV/Codex không được bắt đầu bằng cách “tìm thêm indicator”. Bắt đầu bằng audit, reuse, contract và tests.**

