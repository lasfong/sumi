# Prompt B10 — Replay ledger filter (GAP-REPLAY-01, GAP-EXEC-08)

Đọc: TARGET §3, DEV_BATCHES B10, `api/decisions.py`, orders/trades routes, `practice-state`.
Nhiệm vụ: predicate `candle_index<=current_index` hoặc deprecate raw trong replay; clamp `total_bars/earliest_release_date`.
Verify: tại index N raw không trả activity >N; UAT replay nav.
