# TARGET ARCHITECTURE V3 — Evolutionary (2026-10-09)

**Nguyên tắc:** tiến hóa, không rewrite. Giữ: FastAPI + SQLite local-first, `IndicatorEngine` backend-duy-nhất (pandas-ta), LWC v5, custom primitive drawing 7-kind, import preview→accept + weekly-local, URL `?session=` làm session authority. Sửa: chuẩn hóa execution, replay integrity, FE/WS state, contracts, UX-blocking. Cấm thêm feature mới trước Phase 0–1 (xem Review §4.1).

## 1. Ownership answers (quyết định khóa)

| Câu hỏi | Quyết định | Lý do / Ghi chú |
|---|---|---|
| Zustand giữ hay bỏ? | **Thu còn `sessionId` rồi xóa.** URL là authority duy nhất; Query cache mirror server, không tự tăng index. Xóa double-clear, single-clear duy nhất. | GAP-FE-02/FE-06. Tránh 4 nguồn sự thật. |
| Cursor authority? | **Backend `replay_sessions.current_index` duy nhất.** FE mirror từ server response/WS echo, không `current_index+1` lạc quan. | GAP-FE-01. Mọi nav phải chờ server index. |
| WS merge thế nào? | **Merge theo key đủ `(sessionId, timeframe, adjusted/unadjusted)` + server echo `current_index` + version/seq.** Prefix `['candles',id]` hiện tại bị cấm. Tách cache D/1W, validate canonical timeframe. | GAP-FE-01/WS-01. |
| Timeframe trong key? | **Bắt buộc.** Mọi query/mutation/cache WS đều key đủ TF + adjustment. Giá trị Canonical: `1D/1W`, `unadjusted/adjusted`. | Truth matrix #14. |
| Switch session invalidate? | **Checklist:** `cancelAll` + `removeQueries(session cũ)` + đóng WS cũ + single-clear + mở WS mới + prefetch session mới. Không flash dữ liệu cũ. | GAP-FE-02. |
| Cần ChartKernel facade? | **Có — mỏng.** Một `ChartKernel` facade bọc `createChart`, series, pane, primitive attach; cấm gọi LWC trực tiếp ngoài facade. Không abstract quá dày để thay lib. | GAP-FE-03. Giữ provider-boundary theo ADR-001. |
| Legacy drawing? | **Xóa 3 file dead** (`SumiDrawingAdapter`, `DrawingToolRegistry`, `WorkspacePersistence`), giữ/cách ly `workspaceTypes` cho tới khi type mới thay thế. Chỉ 7-kind primitive tồn tại. | GAP-FE-04. |
| `/backtest` orphan? | **Merge vào Lab làm tab `single`**, giữ route redirect `/backtest → /strategy-lab?tab=single` + chú thích Single vs Battle/Sweep/Multiphase. Không để 2 entry ẩn. | GAP-U-05. |

## 2. Canonical execution sequence (duy nhất)

```
Signal[T] (indicator prefix ≤ cursor, params versioned)
 → Order{QUEUED, type: MARKET_NEXT_OPEN | LIMIT | SL | TP, requested_price/qty, T}
 → Fill{price = Open[T+1] cho MARKET; cut-off: nếu T+1 > N-1 → UNFILLED_AT_CUTOFF; SL/TP intraday cũng next-open (ngoại lệ phải doc)}
 → Position{lots=100, side LONG/SHORT đối xứng, entry net of fee+slippage}
 → Settlement{calendar VN T+1.5 duy nhất qua SettlementEngine; available/pending tách bạch}
 → Exit{SL/TP/LIMIT/MANUAL/CUTOFF, price theo cùng rule fill}
 → Trade{realized net = gross − fee_buy − fee_sell − tax, không truth gross}
 → Metrics{winner: pnl_net>0 (breakeven=not-winner), PF=GP/GL (GL=0 → +inf, hiển thị N/A + tooltip), sample gate N≥30 cảnh báo <30, Sharpe một ngữ nghĩa daily-log, MaxDD close-based}
```

Tham số duy nhất từ `MarketProfile`: `{fee_buy=0.15%, fee_sell=0.15%, tax=0.1% sell, lot=100, slippage default 0, settlement=T+1.5 calendar, sizing_basis=fill_estimate}`. Mọi engine (practice, lifecycle, kernel, batch, sweep) đọc cùng profile. Xem ADR-EXEC-001, ADR-SETTLE-001.

Sizing: tính trên **fill estimate (Open[T+1] est / requested LIMIT)**, không trên close signal. Ghi `sizing_basis` vào order để audit.

## 3. Replay integrity (server-authoritative)

- REST: `candles limit=current_index+1`; MTF `timestamp<=`; signals `observed<=cursor`; indicators **server drop projected** (`<=max_visible_ts`), không trả rows tương lai dù gắn flag.
- Read duy nhất trong replay: `practice-state` (đã filter). Raw `decisions/orders/trades` hoặc filter `candle_index<=current_index` hoặc deprecate trong replay context.
- Không trả `total_bars/earliest_release_date` vượt cursor; clamp hoặc bỏ.
- FE render `MappingResult`: `ok | warming (N/X + progress) | mapping-missing (surface) | server-error | projected-dropped | transport-error`. Cấm `return []` câm.
- `IndicatorEngine` vẫn backend-duy-nhất; `union+reindex` sửa thành cắt cursor; fail-closed all-or-nothing giữ nguyên.

## 4. Transaction & data safety boundary

- Mọi write-path: `try/commit/except-rollback`. `get_db` rollback-on-error + close.
- `PRAGMA foreign_keys=ON` qua connect listener.
- DB path absolute neo `Path(__file__)`; startup assert không tạo DB ở CWD lạ; xóa file ma root sau backup.
- Alembic `env.py` import full `app.models`; migration mới guard `_has_column`.
- Provider stub: `execute_sync` mặc định **từ chối**; chỉ chạy khi `SUMI_ALLOW_SYNTHETIC_PROVIDERS=1` + ghi quarantine dataset + banner "dữ liệu giả". Tách khỏi prod candles.
- Preview không commit khi thiếu `--confirm` (temp/tách commit).

## 5. WS reliability (BE state machine, FE remote)

- BE: validate JSON + clamp speed + `set_speed`/echo `playback_state` + frame chuẩn `{type, seq, current_index, payload}` + `error`/`completed` frames + check send return.
- FE: backoff reconnect + ping/pong + online/offline + token/hủy nav + disable step khi pending + echo index từ server.
- Speed một nguồn (BE), default thống nhất, restart không mất tick (resume từ echo).

## 6. Contracts governance

- Một envelope chuẩn mỗi domain; BB chuẩn hóa một envelope + `response_model` mọi GET.
- Pydantic ↔ TS snapshot test (openapi → typegen hoặc snapshot thủ công có version).
- Không `detail=str(e)`; error envelope `{code, message, field_errors[]}`; FE normalize 422 array, không `[object Object]`.
- Route catalog một (`/api/symbols/catalog`); xóa dead `/api/import/catalog`, `importCafefData` always-400, `data_feeds` cách ly.
- Signal version pin mọi nơi; một `signalsClient`.

## 7. FE structure target (sau tách god)

```
ReplayPage (composition) 
 ├─ ReplayHeader (session/symbol/date/index/OHLCV)
 ├─ ChartPanel (ChartKernel + PaneManager + SeriesManager + RenderRegistry→MappingResult)
 ├─ RightPanel (tabs: Trade | Signals | Indicators | Journal-link)
 ├─ ReplayDock (nav/play/speed/keyboard)
 └─ hooks: useReplaySession, useReplayNav(token), useReplayWS(merge-key), useIndicatorState, useDrawingState
```

Debug hidden outputs sau `import.meta.env.DEV` flag. QueryClient defaults: `retry:1, staleTime theo domain, gcTime 명시`. Không snapshot JSON vào DOM mỗi render.

## 8. Những gì KHÔNG đổi

- Không đổi DB engine (vẫn SQLite file), không đổi pandas-ta, không đổi LWC v5, không thêm chart/drawing dependency khi chưa có spike+license+boundary (AGENTS.md).
- Không đổi import checksum/provenance thiết kế; chỉ bọc transaction + guard.
- Không đổi local-first: no telemetry, no external send.

## 9. Migration/compat

- Legacy execution giữ song song tối đa 1 release sau parity test xanh, gắn banner `deprecated`, rồi xóa.
- Legacy drawing xóa ngay khi type mới thay `workspaceTypes` (không breaking vì 0 import ngoài type).
- `/backtest` redirect giữ bookmark cũ.
- Checklist_snapshot VARCHAR→JSON có migration backfill + dual-read 1 release.
