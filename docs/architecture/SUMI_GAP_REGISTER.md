# SUMI GAP REGISTER — V3 (verified 2026-10-09)

**Quy tắc ưu tiên:** 1 Data corruption → 2 Financial correctness → 3 Lookahead → 4 Determinism → 5 Contract mismatch → 6 User-blocking UX → 7 Reliability → 8 Maintainability → 9 Performance → 10 Cosmetic.
**Trạng thái:** Open trừ khi ghi Fixed. Verify: CONFIRMED = còn tồn tại trên code hiện tại; PARTIALLY = một phần; CHANGED = hình thức đổi nhưng lõi còn.

## P0 — Blocker (data / tiền / lookahead / kẹt user)

| ID | Area | Current | Expected | Evidence | User/Data/Lookahead | Direction | Dependency | Verification |
|---|---|---|---|---|---|---|---|---|
| GAP-DB-01 | DB | `DATABASE_URL=sqlite:///./sumi.db` tương đối CWD; file ma `sumi.db` 0-byte ở root tồn tại song song `backend/sumi.db` 1.4MB | Absolute path neo theo file; guard không tạo DB ở CWD lạ; doc chạy từ `backend/` | `config.py:10`; `ls` root 0B Sep 1 vs backend 1499136B Oct 6; `alembic/env.py:37` tái sinh ma (NEW-DB-03) | User: nhầm DB. Data: CÓ | Absolute `Path(__file__)` + startup assert + xóa ma sau backup | Không | Test khởi động từ CWD lạ không tạo file; UAT temp-DB hash prod unchanged |
| GAP-DB-02 | DB | Mọi write-path import/sync/weekly/rollback `commit/flush` trần, không `rollback`; `get_db` chỉ `close` | `try/commit/except-rollback` mọi write; `get_db` rollback-on-error | `import_workflow:53,101,214,280,328,379,405,469`; `sync_workflow:252,321,372,446,476,550`; `dependencies:3-10`; `weekly:182` flush | Data: CÓ (transaction mở) | Bọc transaction + test lỗi giữa flush→commit | GAP-DB-01 | Test inject lỗi derive vẫn atomic; session không kẹt PendingRollback |
| GAP-DB-03 | DB | SQLite FK vô hiệu runtime (không pragma) | `PRAGMA foreign_keys=ON` qua connect listener | `db.py:1-8`; grep pragma trắng | Data: CÓ (mồ côi/cascade giả) | Event listener + test FK reject/cascade | GAP-DB-02 | Test xóa parent bị chặn / cascade drawing đúng |
| GAP-DB-04 | Data | Stub SSI/vnstock `md5+sin` không HTTP nhưng `execute_sync` INSERT `Candle source=sync:id` + derive weekly, không cờ/quarantine; live `sync_runs=0` | Chặn execute stub trừ `SUMI_ALLOW_SYNTHETIC_PROVIDERS=1` + quarantine + banner "dữ liệu giả" | `ssi_provider:95,99`; `vnstock:89,92`; `sync_workflow:394-407`; grep synthetic rỗng | Data: CÓ (nhiễm prod) | `is_synthetic` + guard + tách dataset | GAP-DB-02 | Execute stub mặc định bị từ chối; bật cờ mới ghi quarantine |
| GAP-EXEC-01 | Execution | Dual timing: legacy same-bar-close vs kernel next-open (+intraday-same nội kernel NEW-EXEC-01); sweep luôn legacy, batch kernel | Một kernel next-open (xem ADR-EXEC-001); xóa legacy sau parity | `backtest_service:312,325-331` + `trade_lifecycle:251,337` vs `execution:127-130,422-436,490-504,572-660`; `strategy_lab:282-291` không phases | User: cùng strategy khác PnL. Data: KHÔNG (logic) | Thống nhất kernel; SL/TP intraday cũng next-open hoặc doc ngoại lệ | GAP-DB-02 | Parity test sweep==batch cùng input; golden cases tay |
| GAP-REPLAY-01 | Replay | Raw `get_decisions/orders/trades` `.all()` vượt cursor | Filter `candle_index<=current_index` hoặc deprecate trong replay; chỉ dùng `practice-state` | `api/decisions:40-42,64-70` vs `practice_workflow:101-104` đúng | Lookahead: CÓ | Predicate + test regression | Không | Test tại index N: raw không trả activity >N |
| GAP-REPLAY-02 | Replay/FE | `IndicatorEngine.union+reindex` mở index tương lai; API gắn `is_projected` nhưng vẫn trả; FE 0 hit filter | Server drop/cắt `<=max_visible_ts` theo policy; FE phân biệt warming/mapping/server/projected/transport | `indicator_engine:340-349`; `api/replay:152-171` append mọi row; grep FE trắng | Lookahead: CÓ | Cắt server + `MappingResult` FE | GAP-EXEC-01 (policy chung) | Test Ichimoku +26 không trả/vẽ tương lai |
| GAP-FE-01 | FE/WS | WS prefix-cache trúng mọi TF + append `'D'/'split'` + `current_index+1` lạc quan; nav `Promise.all` không token, step không guard | Key đủ TF; index từ server; disable step khi pending + token/hủy | `Controller:118,141-156,253-285,294-304,597` | Lookahead/determinism: CÓ | Key đủ + guard + token | GAP-REPLAY-01 | Step nhanh 20 lần không lệch TF/index; test race |
| GAP-WS-01 | WS | Không reconnect/backoff/heartbeat/online-offline; BE không validate speed/JSON, không frame error/completed; send fire-and-forget; speed default vênh + restart mất tick | Backoff + ping/pong + validate/clamp + frame chuẩn + `set_speed`/echo + check send return | `useWebSocket:21-63`; `ws_replay:66-113`; `Controller:78,432-435` | User: dead playback câm | Xem ADR-WS-001 | GAP-FE-01 | Chaos matrix: restart backend giữa play tự nối, không busy-loop/cursor lệch |
| GAP-FE-02 | FE | Session switch chỉ dọn validation, không dọn candles/practice/journal, không cancelAll; double-clear; WS cũ đóng nhưng cache cũ nằm | removeQueries theo session cũ + `cancelAll` + đóng WS + single-clear | `useSessionSelection:62-65`; `Controller:110-114,171` | Lookahead (thấy session cũ): CÓ | Invalidate/cancel checklist | GAP-FE-01 | Switch session không flash dữ liệu cũ (test) |
| GAP-API-01 | Contracts | `BANKRUPT` mất ở FE; `CUSTOM_PRICE` vênh 3 đầu | Đồng bộ enum + validator + type một hướng | `enums:10-16,40-44`; `decision_schema:30-31`; `types/replay:52`, `types/trade:15` | User: 422/render sai | Mở validator hoặc bỏ thừa + thêm type | Không | Contract snapshot test |
| GAP-API-02 | Contracts | BB envelope lệch toàn phần: BE series `{flow_method,methodology_version,as_of,...points[{date,horizons...}]}` / chart `{...data}` vs FE `{timeframe,methodology,calculation_as_of,summary,points[{value,b basis...}]}`; GET không response_model | Một envelope chuẩn + FE map `horizons[H].bb_value`; test snapshot | `bb/contracts:150-183`; `chart_adapter:16-46`; `api/bb:40-80`; `bbApi:49-56`; `BBViewer:74-83` | User: N/A câm | Chuẩn hóa + response_model | Không | Contract test BB 2 formats |
| GAP-U-01 | UX | SELL/CLOSE xám vì T+2 chỉ trong `title` tooltip; `PositionPanel` không đọc `available_quantity` | Dòng inline `T+1 · về sau 1 phiên` + cột Khả dụng/Chờ về | `TradeControls:163-166,179-180`; `PositionPanel` không available | User kẹt tưởng freeze: CÓ | Inline text + đếm T | GAP-EXEC-01 (settlement wording) | UAT: BUY xong thấy lý do inline không cần hover |
| GAP-U-02 | UX | SL/TP sai → `sizingCalc null` panel vụt tắt không chữ; SHORT công thức LONG-only đảo dấu; sync drawing chỉ BUY | Alert inline + preview % live + công thức SHORT + sync mọi direction | `TradeControls:94-101,214-215,232-233,260-267,269,335`; `PositionPanel:21-23,57-73`; `practiceDomain:79-82` | User tưởng chập chờn + tính sai: CÓ | Validate hiển thị + nhánh SHORT | Không | UAT SL/TP/SHORT/sync matrix |
| GAP-U-03 | UX | Builder `handleExecuteRule/SwitchToBattle` ghi `sumi_pending_strategy_rule` + click tab nhưng `StrategyLabPage` zero `getItem`/listener → rule biến mất; banner "sẵn sàng thực thi" sai | Consume + prefill Tester hoặc đổi nút Copy YAML trung thực | `StrategyRuleBuilder:136-233`; grep consumer 0 hit; `StrategyLab:639-655` restore bỏ strategy/cash/benchmark | User mất dữ liệu tưởng bug: CÓ | Prefill hoặc đổi nút | Không | Critical test BUILDER→BATTLE |
| GAP-U-04 | UX | Undo/redo 32px tương phản thấp, title không shortcut, modal thiếu Ctrl+Z/Y, không toast/count; `PendingOrders` không nút hủy LIMIT | Hint + modal + toast + hủy LIMIT | `DrawingToolbar:184-229`; `KeyboardShortcutsModal:13-44` (file ở `components/replay/`); `PendingOrders:15-48` | User mất vẽ/lệnh không cứu: CÓ | Discoverability + cancel | Không | UAT undo/redo/cancel |
| GAP-U-05 | UX/Router | `/backtest` tồn tại (`App:57`) nhưng Sidebar không item; không link với StrategyLab; không chú thích dùng khi nào | Thêm vào Lab hoặc redirect/merge + ghi chú Backtest-single vs Battle/Sweep | `App:57`; `Sidebar:65-140`; `Backtest:95-317` | User lạc: CÓ | Quyết định giữ/merge (Target §6) | GAP-EXEC-01 | Điều hướng thấy được + chú thích |

## P1 — Nên sửa sớm (đúng nhưng giòn / drift / lạc)

| ID | Area | Current → Expected | Evidence |
|---|---|---|---|
| GAP-DB-05 | DB | `alembic/env` thiếu 8 models → import `app.models` đầy đủ | `env:14-23` vs `models/__init__:1-39` |
| GAP-DB-06 | DB | FK replay-graph + items/mutations + candles thiếu index/composite → thêm (giữ UQ) | models decision/order/execution/position/trade/journal/event + `import_run:43-45,54-58` + `candle:6-12` |
| GAP-DB-07 | DB | `decisions.checklist VARCHAR` vs `journal JSON` → thống nhất JSON (NEW-DB-01; journal đã khớp live) | `decision:29` vs `journal_entry:22` + live DDL |
| GAP-DB-08 | Data | Float compare tuyệt đối → `isclose` epsilon | `classifier:244,252,260` |
| GAP-DB-09 | Data | Naive/aware trộn → một chuẩn `Asia/Ho_Chi_Minh` hoặc naive toàn bộ | `import:188`, `weekly:94` naive vs `accepted_at/rolled_back/sync:417` aware |
| GAP-DB-10 | DB | `a2d639ecd5d5` add-column trần → guard `_has_column` (các migration giữa đã có) | `a2d639ecd5d5` vs `20260629_0004` pattern |
| GAP-DB-11 | Data | Weekend/holiday cứng → holiday-calendar VN | `weekly` Monday + `classifier` weekday |
| GAP-DB-12 | Data | Credentials `os.getenv` rải rác → gom `Settings` + masking/log-guard | `sync_workflow:47`, `doraemon:34-35` |
| GAP-DB-13 | Data | `DataFeedService` bypass + comment TimescaleDB sai → deprecate/route qua workflow | `data_service:32,63-73` |
| GAP-DB-14 | Data | Preview commit ngay cả khi thiếu `--confirm` (NEW-DB-02) → preview trên temp/tách commit hoặc cảnh báo rõ | `seed_cafef` + `import_workflow:53,101,214` |
| GAP-EXEC-02 | Execution | Slippage 3 truth → một `MarketProfile` | `broker:31` 0.001 vs kernel/batch 0.0 vs lifecycle absent |
| GAP-EXEC-03 | Execution | T+2 2 truth → `SettlementEngine` duy nhất | `settlement:42-95` vs bar-count `backtest:346`, `practice:201-226`, `lifecycle:284-321` (+fallback kernel `233-234,330`) |
| GAP-EXEC-04 | Execution | Realized gross vs net → net everywhere | `practice:191` gross vs `:261` net vs kernel net vs `lifecycle:345-346` gross chunk |
| GAP-EXEC-05 | Execution | Winner/PF/sample 3 vênh → một spec (breakeven? PF-inf vs N/A? gate N?) | `metrics:168,185-192` vs `analytics:19,195-196,233-240` vs `strategy_lab:487` |
| GAP-EXEC-06 | Execution | Sharpe chỉ analytics + 2 truth nội bộ (NEW-EXEC-02) → bổ sung vào metrics/batch + một ngữ nghĩa | `analytics:158-180` vs `268-297`; batch trắng sharpe |
| GAP-EXEC-07 | Execution | Sizing basis mismatch (close signal vs open fill) → sizing trên fill estimate/requested | `backtest:319-321,559-570` vs `execution:420` |
| GAP-EXEC-08 | Execution | `total_bars/earliest_release_date` lộ forward → clamp về cursor | `practice:224-226,304` |
| GAP-API-03 | Contracts | `CreateSessionMode` thiếu `backtest` → mở hoặc guard FE | `replay_schema:42-47` vs enums vs `types/replay:53` |
| GAP-API-04 | Contracts | Analytics đảo required/optional 2 chiều → đồng bộ + snapshot | `analytics_schema:55,79` vs `types/analytics:68,92` |
| GAP-API-05 | Contracts | `TradePlanResult` thiếu 7 field → bổ sung FE | `trade_planning:16-26` vs `types/decision:decision` |
| GAP-API-06 | Contracts | Journal `checklist_snapshot` mất ở FE; decision FE hẹp `string` → đồng bộ `dict\|str` | `journal_schema:17`, `decision_schema:25` vs `types/journal`, `types/decision` |
| GAP-API-07 | Contracts | Signal `max2+forbid+trùng+params` không ở FE; params/output_type hẹp ở FE (NEW-API-01) → validate FE + nới type | `signal_schema:33-61,71-84` vs `types/signals` |
| GAP-API-08 | Contracts | Catalog hai route (`/api/import/catalog` dead) → giữ một | `import_data:113` vs `symbols:17` vs `importApi:134` |
| GAP-API-09 | Contracts | 422 array thành `[object Object]`; `detail=str(e)` lộ nội bộ; single ErrorBoundary; QueryClient retry 3 | `client:12-20`, `Controller:33-36`, `DataSyncPanel:16-19`; `replay:142`, `indicators:90`, `analytics:14`, `bb:68,106`, `universe:28,40,42`; `App:8,51` |
| GAP-API-10 | Contracts | `importCafefData` gọi route luôn-400; `data_feeds` dead vi phạm weekly/timezone + deps nặng → xóa/cách ly | `importApi:143-148` vs `import_data:119-137`; `data_feeds/*` + `data_service` |
| GAP-API-11 | Contracts | `Candle.source` optional↔required; decision checklist hẹp (NEW-API-03); BB GET envelope (NEW-API-02) | `candle_schema:14` vs `types/candle:11` |
| GAP-FE-03 | FE | Không ChartKernel facade (2 createChart, calls rải rác); `SumiPrimitive...` đã type-only (NEW-FE-01, tốt) | `CandleChart:2,81`; `SeriesManager:1-11`; `EquityChart:2,26`; `PositionLine:1` |
| GAP-FE-04 | FE | Legacy drawing 3 file dead nhưng `workspaceTypes` còn sống → xóa 3 file + giữ/cách ly types | `SumiDrawingAdapter`, `DrawingToolRegistry`, `WorkspacePersistence` 0 import ngoài test; `workspaceTypes` import bởi CandleChart/Series/Pane/Registry |
| GAP-FE-05 | FE | God 1366 dòng (764+602) + 6 hidden outputs + remount rail + order-controls fixed | Tách Header/ChartPanel/RightPanel + hooks; debug sau DEV flag |
| GAP-FE-06 | FE | Zustand nửa chết + double-clear | Thu còn sessionId rồi xóa; single-clear |
| GAP-FE-07 | FE | RenderRegistry ~15 `return []` câm → `MappingResult` + surface | `IndicatorRenderRegistry` |
| GAP-FE-08 | FE | Hai signal client chung transport nhưng vênh version/payload → một `signalsClient` + pin version | `SignalInspector:199` vs `Explanation:66-84` (đã chung `signalsApi:19-24`, NEW-FE-03) |
| GAP-FE-09 | FE | QueryClient trần + staleTime phân tán; snapshot JSON vào DOM mỗi render; project duplicate; indicator storm burst | `App:8`; `CandleChart:46-136`; `Provider:205`; `Provider ~31-50/~244-265`; `Controller:477-562` |
| GAP-FE-10 | FE | `useWebSocket` không backoff + log prod + state re-render (thuộc WS nhưng tách để fix FE) | `useWebSocket:1-70` |
| GAP-U-06 | UX | `?tab=multiphase/flow` không nav active (catalog/builder đã có branch) | `Sidebar:52-63` |
| GAP-U-07 | UX | `warming` trần không tooltip/progress/badge | `IndicatorPaneChrome:14-15`; `ChartLegendOverlay:199-201` |
| GAP-U-08 | UX | Signal dồn trong tab trade đẩy BUY/SELL xuống fold | `ReplayWorkspace:678-706` |
| GAP-U-09 | UX | Reorder `fixed right:340 opacity:0.15 fontSize:8` + trôi khi resize (NEW-UX-03) | `ReplayWorkspace:533-571`; `IndicatorManager:27` |
| GAP-U-10 | UX | Mobile chỉ Replay có banner (off-by-one 1179 vs 1180); Lab/Scanner/Backtest/Journal không banner; không công bố min-width | `TradeControls:21-23,48-49,162`; `Workspace:135-138`; tables `overflowX:auto` |
| GAP-U-11 | UX | Assumptions chỉ Sweep đầy đủ; Battle thiếu; Backtest format khác; Trade dialog không dòng assumptions; wording `T+2` vs `T+2.5/T+2` (NEW-UX-01) | `StrategyLab:1572-1574` vs `1401-1406`; `Backtest:295-306`; `TradeControls:111-116`; `MultiPhase:725` |
| GAP-U-12 | UX | Journal 4 tên 2 khái niệm; Workspace không link `/journal`/`/analytics`; CLOSE chia sẻ tooltip SELL (NEW-UX-02) | `PracticeRail:30-34`; `JournalPage:77-127`; `TradeControls:180` |
| GAP-U-13 | UX | Empty VI/EN lẫn + thiếu CTA/stepper; Space 2 nghĩa; speed 2 nguồn lệch; magnet trùng; persistence tooltip mờ; version hardcode | Nhiều file nhỏ (P2 nhưng gom để khỏi quên) |

## P2 — Vệ sinh/perf (khi chạm file)

- Vectorize `apply/iterrows`; weekly query `BETWEEN` thay vì load full + loop O(W²); bulk inserts + preload symbols; paginate preview items; helper `to_date_str`/`formatVietnamese`; `detect_kind()` duy nhất; thống nhất threading `connect_args`; xóa dead helpers; `nextCandle` params thống nhất; `ProviderRegistry` thread-safe.

**Traceability:** mỗi GAP map tới acceptance (R/I/D/T/G) trong `docs/testing/ACCEPTANCE_TRACEABILITY_MATRIX.md`; mỗi batch trong `docs/plans/DEV_BATCHES.md` phải dẫn GAP IDs nó đóng.
