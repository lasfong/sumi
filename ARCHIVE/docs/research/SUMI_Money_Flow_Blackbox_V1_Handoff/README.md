# SUMI Money Flow Blackbox V1 — DEV Handoff Pack

## Mục tiêu

Bộ tài liệu này là handoff từ session nghiên cứu Money Flow Blackbox sang DEV/Codex. Phạm vi **chỉ** là Money Flow Blackbox; không định nghĩa toàn bộ Signal Engine hoặc Backtest Engine.

## Quyết định cốt lõi

1. **Ưu tiên TRUE_FLOW**: buyer-initiated vs seller-initiated **executed trading value**.
2. Nếu không có aggressor side nhưng có trade + quote đủ chất lượng, có thể dùng `TRUE_FLOW_ESTIMATED` sau khi validation classification.
3. Nếu public data không đáp ứng, dùng **OHLCV_PROXY** và phải ghi rõ là technical proxy; không được gọi là tiền vào/ra thật.
4. Multi-horizon dùng rolling `T03/T05/T10/T20/T50/T200`, không reset block.
5. Whole-market phải aggregate value/flow trước rồi mới tính ratio; không average score từng mã.
6. Flow Breadth và Activity là dimension riêng, không nhồi vào BB score.
7. Không suy institutional/retail từ horizon, trade size, OHLCV, MFI/CMF hoặc volume spike.

## Thứ tự đọc dành cho DEV

1. `01_RESEARCH_CONCLUSIONS.md` — hiểu bài toán và lý do thiết kế.
2. `02_ARCHITECTURE_DECISION.md` — quyết định mode và invariant.
3. `03_DATA_AUDIT_AND_SOURCE_STRATEGY.md` — **làm trước khi code core**.
4. Nếu Data Gate pass TRUE_FLOW: đọc `04_TRUE_FLOW_SPEC.md`.
5. Nếu Data Gate fail: đọc `05_OHLCV_PROXY_SPEC.md`.
6. `06_MARKET_AGGREGATION_AND_BREADTH.md` — market/universe logic.
7. `08_INTEGRATION_SCHEMA.md` — contract dữ liệu đầu ra.
8. `09_VALIDATION_AND_ACCEPTANCE.md` — QA/research validation.
9. `07_UI_STATES_AND_USER_WORKFLOW.md` — chỉ sau khi primitive/horizon đã đúng.
10. `10_DEV_IMPLEMENTATION_PLAN.md` — roadmap thực thi.
11. `11_REFERENCES.md` — nguồn nghiên cứu.

`MASTER_SPEC.md` là bản đầy đủ và là **source of truth** khi có mâu thuẫn giữa các file tách nhỏ.

## Data Gate — quyết định mode

```text
Executed trades + trustworthy aggressor_side?
    YES -> TRUE_FLOW
    NO  -> Trades + synchronized bid/ask, classification validated?
              YES -> TRUE_FLOW_ESTIMATED
              NO  -> OHLCV / matched trading value available?
                        YES -> OHLCV_PROXY
                        NO  -> NO-GO for BB
```

## Công thức TRUE_FLOW chuẩn

```text
BUY_H  = rolling_sum(buyer_initiated_value, H)
SELL_H = rolling_sum(seller_initiated_value, H)

OIB_H = (BUY_H - SELL_H) / (BUY_H + SELL_H)
BB_H  = 50 * (1 + OIB_H)
      = 100 * BUY_H / (BUY_H + SELL_H)
```

`50` = aggressive buy/sell balance. `>50` = buyer-initiated dominance. `<50` = seller-initiated dominance.

## Deliverable đầu tiên DEV phải tạo

Trước implementation core, tạo **Data Capability Report** cho từng source / exchange / date range, tối thiểu gồm:

- executed trade hay chỉ order statistics;
- aggressor side có/không và semantics;
- trade timestamp precision;
- bid/ask quote availability + synchronization;
- matched value vs put-through separation;
- historical depth và missingness;
- listing/delisting/trading-status metadata;
- foreign/proprietary fields nếu có;
- phần trăm executed value classify được;
- mode đề xuất cho từng khoảng lịch sử: `TRUE_FLOW`, `TRUE_FLOW_ESTIMATED`, `OHLCV_PROXY`, hoặc `UNAVAILABLE`.

## Nguyên tắc không được vi phạm

- Proxy không được đổi tên thành actual money inflow/outflow.
- Không splice TRUE_FLOW và OHLCV_PROXY thành một series không có nhãn.
- Không dùng current universe để backfill lịch sử.
- Không optimize measurement design bằng backtest P&L.
- Không biến BB thành composite MFI + CMF + OBV + ADL.
