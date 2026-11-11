# ADR-WS-001 — WebSocket Reliability (BE state machine, FE remote)

**Status:** Accepted 2026-10-09.
**Context:** Không reconnect/backoff/heartbeat/validate; speed default vênh (500 vs 1000), restart mất tick; send fire-and-forget; không frame error/completed (GAP-WS-01, GAP-FE-01/10).
**Decision:** BE là máy trạng thái (validate JSON, clamp speed, `set_speed`/echo `playback_state`, frame `{type,seq,current_index,payload}`, frames `error`/`completed`, check send return, ping/pong). FE là remote (backoff reconnect, online/offline, token/hủy, disable step pending, resume từ echo).
**Alternatives:** FE tự chạy tick — loại: divergence khi mất kết nối (dead playback).
**Verification:** Chaos matrix (WEBSOCKET_TEST_MATRIX.md): restart BE giữa play tự nối, không busy-loop/cursor lệch.
