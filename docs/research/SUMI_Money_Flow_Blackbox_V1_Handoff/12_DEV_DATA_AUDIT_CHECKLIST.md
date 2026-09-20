# DEV Data Audit Checklist

Dùng checklist này trong phase R0/R1. Mỗi nguồn dữ liệu phải có bằng chứng/documentation hoặc sample thực tế; không suy đoán từ tên field.

## 1. Source inventory

- [ ] Liệt kê API/source đang dùng: CTCK, CafeF, exchange/public endpoint, cache nội bộ.
- [ ] Ghi exchange: HOSE / HNX / UPCoM.
- [ ] Ghi earliest/latest date và số symbol.
- [ ] Ghi update frequency/latency.
- [ ] Lưu 1–3 sample response thô cho mỗi endpoint.

## 2. Executed flow capability

- [ ] Dòng dữ liệu là **matched executions** hay order placement/statistics?
- [ ] Có `trade_price`?
- [ ] Có `trade_volume` / `trade_value`?
- [ ] Có timestamp đủ chi tiết?
- [ ] Có `aggressor_side` BUY/SELL?
- [ ] Định nghĩa BUY/SELL là aggressor hay chỉ phía đặt lệnh?
- [ ] ATO/ATC được biểu diễn thế nào?
- [ ] Put-through/thỏa thuận có tách riêng không?

## 3. Quote capability nếu phải classify

- [ ] Có best bid/best ask lịch sử?
- [ ] Quote timestamp có thể align với trade timestamp?
- [ ] Có sequence/order để xử lý cùng timestamp?
- [ ] Có quote stale/gap đáng kể?
- [ ] Tỷ lệ trade inside spread là bao nhiêu?
- [ ] Có labeled sample để đo classification accuracy?

## 4. Daily fallback capability

- [ ] OHLCV đầy đủ?
- [ ] Actual matched trading value có không?
- [ ] Nếu chỉ có volume, xác định công thức estimated value.
- [ ] Corporate action/adjusted series có nhất quán?
- [ ] Missing row khác no-trade session được nhận diện?

## 5. Universe integrity

- [ ] listing_date
- [ ] delisting_date
- [ ] exchange history
- [ ] security_type
- [ ] suspension/trading status
- [ ] index membership effective_from/effective_to nếu cần index scope

## 6. Classified investor flows

- [ ] foreign_buy_value / foreign_sell_value
- [ ] proprietary_buy_value / proprietary_sell_value
- [ ] semantics documented
- [ ] không suy institutional/retail nếu source không có classification

## 7. Gate result

Cho từng `source × exchange × date_range`, chọn đúng một:

- [ ] `TRUE_FLOW`
- [ ] `TRUE_FLOW_ESTIMATED`
- [ ] `OHLCV_PROXY`
- [ ] `UNAVAILABLE`

Ghi kèm `semantic_confidence`, `coverage_ratio`, `missingness`, `known_limitations`.
