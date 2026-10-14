# ADR-EXEC-001 — One Canonical Execution Model (next-open)

**Status:** Accepted 2026-10-09. **Scope:** practice, lifecycle, backtest legacy/kernel, sweep, batch.
**Context:** Hai timing song song: legacy same-bar-close (`backtest_service:312,325-331`, `trade_lifecycle:251,337`) vs kernel next-open (`execution:127-130`), cộng intraday-SL/TP same-bar nội kernel (`execution:422-436,490-504,572-660`). Sweep luôn legacy (`strategy_lab:282-291`), batch kernel → cùng strategy khác PnL (GAP-EXEC-01).
**Decision:** Một kernel duy nhất: Signal[T] → Order QUEUED MARKET_NEXT_OPEN → Fill Open[T+1]; cut-off N-1 → UNFILLED_AT_CUTOFF; SL/TP intraday cũng next-open (không ngoại lệ ngầm; nếu giữ ngoại lệ phải doc + flag). Sizing trên fill-estimate. Legacy giữ tối đa 1 release sau parity, gắn banner deprecated, rồi xóa.
**Alternatives:** (a) Giữ dual + flag per-run — loại: nhân đôi divergence, user không biết vì sao lệch. (b) Chuẩn hóa về same-close — loại: kém đúng tài chính (không mua được giá đóng sau khi thấy tín hiệu).
**Consequences:** Thay đổi PnL lịch sử backtest cũ (phải ghi chú); sweep/batch parity; cần golden cases + migration banner.
**Compat/Migration:** Legacy route giữ + `deprecated:true`; parity test sweep==batch; sau xanh mới xóa file legacy.
**Verification:** Parity test cùng input; 10-20 golden hand-calc (FINANCIAL_GOLDEN_CASES.md); UAT Battle vs Backtest-single cùng kỳ vọng cùng PnL trong epsilon phí.
