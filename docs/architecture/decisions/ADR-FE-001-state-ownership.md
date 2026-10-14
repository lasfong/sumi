# ADR-FE-001 — Frontend State Ownership (URL + Query mirror, drop Zustand)

**Status:** Accepted 2026-10-09.
**Context:** 4 nguồn: useState + TanStack Query (no defaults, stale phân tán) + Zustand nửa-chết (candles/positions/trades dead, partialize sessionId, double-clear) + URL `?session=` canonicalize; WS prefix-cache sai TF + optimistic index; nav race; switch flash cũ (GAP-FE-01/02/06, GAP-FE-09).
**Decision:** URL là authority session duy nhất; Query mirror server (không tự tăng index); Zustand thu còn sessionId rồi xóa; WS merge key đủ `(session,T F,adjustment)` + echo index; nav token/hủy + disable step khi pending; switch checklist cancelAll+removeQueries+closeWS+single-clear.
**Alternatives:** Giữ Zustand làm store chính — loại: đã nửa-chết, trùng Query, double-clear.
**Verification:** Step nhanh 20 lần không lệch; switch không flash; test race/token.
