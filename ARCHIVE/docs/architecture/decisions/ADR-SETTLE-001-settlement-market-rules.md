# ADR-SETTLE-001 — Settlement & Market Rules (T+1.5 calendar + MarketProfile)

**Status:** Accepted 2026-10-09.
**Context:** Hai truth T+2: calendar `settlement.py:42-95` vs bar-count (`backtest:346`, `practice:201-226`, `lifecycle:284-321` + fallback kernel `233-234,330`); slippage 3 truth (`broker:31` 0.001 vs kernel/batch 0.0 vs lifecycle absent); wording `T+2` vs `T+2.5/T+2` (GAP-EXEC-02/03, NEW-UX-01).
**Decision:** `SettlementEngine` calendar VN duy nhất (T+1.5: tiền về sau 1.5 phiên, hiển thị `T+1 · về sau 1 phiên`); một `MarketProfile` `{fee_buy 0.15%, fee_sell 0.15%, tax 0.1% sell, lot 100, slippage 0, settlement calendar, sizing_basis fill_estimate}`; mọi engine đọc cùng profile; một dòng assumptions mọi UI.
**Alternatives:** Bar-count — loại: sai ngày nghỉ/lễ, lệch tiền thực.
**Consequences:** SELL/CLOSE availability đổi ở biên ngày nghỉ; cần holiday-calendar VN (GAP-DB-11).
**Verification:** Golden settlement qua cuối tuần/lễ; UAT inline khả dụng/không khả dụng (GAP-U-01).
