# 08 — SUMI-420 Market Specification

## Definition

“Toàn thị trường” trong Sumi V1 = `SUMI-420`.

Khoảng 420 mã trọng yếu, managed và version hóa.

## Market BB with value

Daily:

```text
MarketBuy_t  = Sum(active_buy_value_i)
MarketSell_t = Sum(active_sell_value_i)
```

Horizon:

```text
MarketBB_H = 100 * Sum_H(MarketBuy_t)
                  / Sum_H(MarketBuy_t + MarketSell_t)
```

Không average BB từng mã.

## Only active volume

Không cộng raw volumes trực tiếp qua cổ phiếu giá khác nhau.
Ước lượng value bằng representative price rồi aggregate.

## Breadth

```text
positive = count(BB_H > 50 + buffer)
negative = count(BB_H < 50 - buffer)
neutral  = remainder
```

Market BB = value dominance.
Breadth = participation.

## Coverage

```text
symbol_coverage
value_coverage
```

phải expose trong output.

## Universe version

Nếu thay mã, tạo version mới. Không cần reconstruct tất cả lịch sử thị trường Việt Nam.
