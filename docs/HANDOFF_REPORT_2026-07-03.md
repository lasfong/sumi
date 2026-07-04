# Sumi Handoff Report

Ngay lap: 2026-07-03  
Repo: `https://github.com/lasfong/sumi.git`  
Workspace: `/Users/mizuhara/workspace/sumi`  
Base branch luc tiep nhan: `master`  
Branch dang thuc thi: `codex/lightweight-charts-v5-spike`  
Commit da kiem tra: `3c6b4ca70e8c3aa7e2d3bc62c779ee84e79b4840`  
Tag hien tai: `v2.0.0-rc1`

> Cach doc: muc `0.x` la cac checkpoint moi nhat va la ket luan canonical tai
> ngay 2026-07-04. Cac muc `1-14` la audit snapshot luc tiep nhan; mot so blocker
> trong snapshot da duoc xu ly va duoc ghi lai o muc `0.x`.

## Cap Nhat Sau Dot Thuc Thi 2026-07-04

### 0.1. Pham Vi Da Lam Sau Report Ban Dau

Sau khi review report ban dau va yeu cau chuan hoa chart UI theo TradingView Lightweight Charts v5, da thuc hien theo huong "khong dap toan bo project". Pham vi thay doi tap trung vao nhung diem dang can sua nhat de dua san pham ve trang thai co the dung va kiem thu:

- Nang `lightweight-charts` len v5.2.0.
- Refactor chart/replay thanh cac module co ranh gioi ro hon:
  - `ChartWorkspace`
  - `PaneManager`
  - `SeriesManager`
  - `IndicatorRenderRegistry`
  - `DrawingToolRegistry`
  - `SumiDrawingAdapter`
  - `WorkspacePersistence`
- Dung official pane API cua Lightweight Charts v5 thay vi gia lap pane bang nhieu chart tach roi.
- Giu backend indicator API la source of truth; frontend chi render du lieu indicator da tinh tu backend.
- Tach pane dung ky vong:
  - Price pane: candlestick, MA, EMA, Ichimoku, Bollinger Bands, PSAR, drawings.
  - Pane rieng: Volume, RSI, MACD, CCI.
- Them adapter/persistence cho drawing va indicator workspace.
- Sua backend indicator CCI bang cong thuc canonical de tranh bug trong dependency `pandas-ta`.
- Tao seed demo deterministic de test duoc app offline/local ma khong phu thuoc market-data service ben ngoai.
- Chay UAT bang browser tren cac man hinh chinh va sua bug phat hien trong qua trinh click that.

Nhan dinh sau khi thuc thi: van khong nen dap toan bo project. Phan chart/replay cu can thay mau kha nhieu, nhung viec thay mau co chon loc da kha thanh cong. Nen tiep tuc audit domain/accounting thay vi rewrite ca frontend/backend.

### 0.2. Trang Thai Kien Truc Sau Refactor Chart v5

Chart hien tai khong con la mot component React don lon lam qua nhieu viec. `CandleChart.tsx` dong vai tro facade/entry cho workspace moi, trong do:

- `ChartWorkspace` quan ly lifecycle chart, input candles/indicators/drawings va noi cac manager lai voi nhau.
- `PaneManager` tao va quan ly pane theo official Lightweight Charts v5 API.
- `SeriesManager` tao candlestick/volume/indicator series bang API v5 `addSeries(...)`.
- `IndicatorRenderRegistry` quyet dinh indicator nao overlay len price pane va indicator nao di pane rieng.
- `DrawingToolRegistry` quan ly tool drawing o muc ung dung.
- `SumiDrawingAdapter` chuan hoa serialize/deserialize drawing de persistence khong phu thuoc truc tiep vao third-party drawing library.
- `WorkspacePersistence` luu workspace local theo session, gom indicator workspace va drawing state can restore sau reload.

Community libraries duoc xem nhu nguon tham khao/adapter target, khong clone vao source tree va khong copy code lon vao repo. Huong nay phu hop voi yeu cau:

- `deepentropy/lightweight-charts-indicators`: tham khao cho cach mo rong indicator/RSI rieng pane.
- `deepentropy/lightweight-charts-drawing`: tham khao cho drawing extension.
- `difurious/lightweight-charts-line-tools-core`: tham khao drawing tools v5+.

Quyet dinh dung adapter la dung: neu sau nay doi thu vien drawing/indicator, core Sumi khong bi khoa chat vao mot implementation ben ngoai.

### 0.3. Cac Bug/Rui Ro Da Phat Hien Va Da Sua

1. Chart pane cu khong dung official v5 pane API.
   - Da thay bang `PaneManager`/`SeriesManager` tren Lightweight Charts v5.

2. Indicator warmup/null bi render sai thanh `0`.
   - Nguyen nhan: frontend convert `null` bang `Number(null)`.
   - Da sua: registry bo qua gia tri `null`/empty thay vi ve diem 0 gia.

3. CCI tinh sai do bug o dependency `pandas-ta`.
   - Phat hien cong thuc upstream co van de thu tu phep tinh.
   - Da override `_compute_cci` trong backend theo cong thuc canonical.
   - Da them regression test.

4. Replay layout bi tran ngang, controls co the bi day khoi viewport.
   - Da sua layout `App.tsx` va `ReplayPage.tsx` de responsive hon.

5. `EquityChart` dung API v4 cu cua Lightweight Charts.
   - Da migrate sang v5 `addSeries(AreaSeries/HistogramSeries)`.

6. Test e2e/screenshot backend co side effect khi pytest collection.
   - Da chuyen e2e thanh optional skip neu khong co `SUMI_E2E=1`.
   - Screenshot helper khong con tu chay trong pytest collection.

7. Moi truong Python dependency lech phien ban.
   - `pandas-ta==0.4.71b0` yeu cau Python >= 3.12.
   - Da dung Python 3.12 trong `.venv` va cap nhat README/requirements.

8. Du lieu demo ban dau chi co `FPT`, trong khi UI mac dinh dung `FPT, SSI, VCI` va benchmark `VNINDEX`.
   - Da mo rong `backend/scripts/seed_demo.py` seed 4 symbol: `FPT`, `SSI`, `VCI`, `VNINDEX`.
   - Ket qua seed local: 2.080 daily candles.

9. Backtest/Scanner mo trang len chua chon strategy, user bam Run khong co ket qua hoac bi alert.
   - Da them default derived strategy filename: strategy dau tien trong danh sach duoc chon san.
   - Khong dung setState trong effect de tranh vi pham lint React.

### 0.4. UAT Bang Browser Da Chay

Da test bang browser local voi backend `http://127.0.0.1:8000` va frontend `http://localhost:5173`.

Replay / Trading Lab:

- Mo session replay voi du lieu seed.
- Chart render duoc candlestick, volume va panes.
- Them EMA/RSI/MACD/CCI:
  - EMA overlay tren price pane.
  - RSI pane rieng.
  - MACD pane rieng gom line/signal/histogram.
  - CCI pane rieng, gia tri sau fix hien dung vung dao dong thay vi bi zero/null.
- Next/Prev hoat dong, panes dong bo theo replay index.
- Tao drawing horizontal line, reload lai van persist.
- Reload lai workspace van restore indicator/drawing theo session.
- Luong Scanner Signal -> Replay tao session moi va hien panel scanner signal dung.
- Khong thay console error trong cac flow da test.

Backtest Engine:

- Mo page tu trang thai mac dinh.
- Strategy dau tien duoc chon san sau fix.
- Bam Run Backtest chay thanh cong voi `FPT`.
- Tao result session, metrics va equity chart render.
- Khong console error.

Strategy Lab:

- Mo page voi default symbols `FPT, SSI, VCI`.
- Select All hai strategy.
- Compare Strategies chay thanh cong.
- Ket qua:
  - `MACD RSI Momentum`: SUCCEEDED.
  - `MA Crossover Simple`: SUCCEEDED.
- Khong console error.

Signal Scanner:

- Mo page voi default `FPT, SSI, VCI`, benchmark `VNINDEX`.
- Sau fix, strategy dau tien duoc chon san.
- Bam Run Scanner chay thanh cong.
- Ket qua mac dinh voi `MACD RSI Momentum`: 3 signals.
- Chon `MA Crossover Simple`: 9 signals.
- History scan duoc luu.
- Bam Replay tren signal tao session replay moi thanh cong.
- Khong console error.

Analytics:

- Mo `/analytics` voi session replay gan nhat.
- Page render metrics empty state hop ly vi session chua co closed trades.
- Khong console error.

Journal:

- Mo `/journal` voi session replay gan nhat.
- Page render empty journal state hop ly.
- Khong console error.

Data Feeds / Import:

- Mo `/import`.
- Form upload render, nut import disabled khi chua chon file.
- Khong console error.

### 0.5. Gate Tu Dong Da Chay

Frontend:

- `npm run lint`: pass.
- `npm test`: pass, 9 test files, 18 tests.
- `npm run build`: pass.

Backend:

- `../.venv/bin/python -m pytest -q`: pass, 48 passed, 1 skipped.

Can luu y:

- Frontend test co warning Node 26 ve localStorage experimental; da co polyfill trong test setup, warning khong lam fail test.
- Backend pytest co `StarletteDeprecationWarning` ve `httpx`/`TestClient`; day la dependency warning, chua phai functional bug.
- Browser e2e tu dong van optional, chua bat buoc trong CI neu chua cai Playwright va set `SUMI_E2E=1`.

### 0.6. Danh Gia Sau Khi Da Sua

Trang thai hien tai tot hon nhieu so voi report ban dau:

- Khong con la tinh trang chart v4/v5 lap lung.
- Replay chart da co kien truc co the mo rong.
- Indicator panes da dung official LWC v5 pane API.
- Backend van giu vai tro source of truth cho indicator.
- UAT browser cac flow chinh da pass tren data local.
- Test/lint/build backend/frontend da pass.

Tuy nhien, day chua nen duoc coi la "production trading system" da audit het nghiep vu. Cac rui ro con lai:

- Accounting/broker lifecycle can audit sau hon bang scenario co lenh mua/ban, pending order, T+2, fee/tax, partial/edge cases.
- Analytics hien render dung empty/result state, nhung can reconcile so lieu voi expected ledger bang test business-level.
- `pandas-ta` beta dependency van la rui ro, du da pin version va override CCI.
- Browser e2e nen duoc dua vao CI that su neu day la san pham can regression UI nghiem tuc.
- App van theo huong local-first, SQLite/no auth; neu muc tieu la multi-user SaaS thi can design lai tang auth/storage/deployment.

Ket luan moi sau dot thuc thi:

> Khong nen dap toan bo project. Nen giu codebase, tiep tuc refactor co chon loc. Phan chart/replay da duoc nang len nen tang Lightweight Charts v5 dung hon va da qua UAT browser. Sprint tiep theo nen tap trung audit broker/accounting/analytics bang acceptance scenario cu the, khong nen quay lai rewrite frontend/chart tu dau.

### 0.7. Cap Nhat Tiep Theo: Audit Broker/Accounting 2026-07-04

Sau phan chart/browser UAT, da tiep tuc audit vung rui ro cao nhat con lai: `TradeLifecycleService`, date-range replay/backtest/scanner va cac test accounting hien co.

Ket qua audit:

- Xac nhan co hai duong mo phong ton tai:
  - `TradeLifecycleService`: dang la duong user/API/backtest/replay that su di qua.
  - `BrokerSimulation`: engine event rieng, co test rieng nhung khong phai duong user-facing chinh.
- Uu tien sua `TradeLifecycleService` vi day la core anh can tin khi user dat lenh trong Trading Lab va khi Backtest chay qua replay session.

Bug nghiep vu da phat hien va da sua:

1. BUY market khong kiem tra suc mua.
   - Truoc day user co the dat BUY vuot `current_cash`, lam cash am phi ly trong app long-only.
   - Da them check `net_amount = price * quantity + buy_fee`.
   - Neu thieu tien, service tra HTTP 400 `Cannot buy: insufficient cash...`.
   - Pending limit BUY khi khop ma thieu tien se bi mark `rejected`.

2. Rejected trade decision co the de lai transaction dang do.
   - Da them rollback khi `_execute_buy/_execute_sell/_create_limit_order` raise HTTPException trong `process_decision`.
   - Tranh viec decision bi flush dang do vao DB session sau reject.

3. CLOSE/CUT_LOSS/TAKE_PROFIT khi khong co open position van co the thanh cong va log nhu trade executed.
   - Da sua thanh HTTP 400 `Cannot close: no open position`.
   - Khong tao order/trade sai.

4. Date range 1 ngay co the khong lay duoc candle do so sanh `DateTime <= Date`.
   - Da them helper `app/utils/date_range.py`.
   - Dung `timestamp >= start_at(start_date)` va `timestamp < end_before(end_date)`.
   - Ap dung cho:
     - Replay create/get/next candle queries.
     - Backtest candle load va benchmark regime range.
     - Scanner candle load.
     - Analytics VNINDEX benchmark curve.

Test moi/tang cuong:

- `test_buy_rejects_when_cash_is_insufficient`.
- `test_close_without_open_position_is_rejected`.
- `test_partial_reduce_keeps_trade_open_and_updates_cash`.
- Test cash insufficiency dung end_date cung ngay voi start_date de khoa bug inclusive end-date.

Gate sau audit:

- Backend: `../.venv/bin/python -m pytest -q` pass, 51 passed, 1 skipped.
- Frontend: `npm run lint` pass.
- Frontend: `npm test` pass, 9 files, 18 tests.
- Frontend: `npm run build` pass.

Danh gia sau audit broker/accounting:

- Rủi ro P0/P1 trong user-facing trade lifecycle da giam dang ke.
- Cac scenario T+2, fee/tax, full close, multi round-trip, partial reduce, insufficient cash, close-empty da co regression tests.
- Van con technical debt: `BrokerSimulation` va `TradeLifecycleService` la hai abstraction mo phong song song. Chua can rewrite ngay, nhung sprint tiep theo nen quyet dinh hop nhat hoac loai bo mot duong de tranh drift nghiep vu.
- Analytics da co test tong quat va equity curve; tuy nhien neu sau nay co multi-symbol position trong cung mot session, can audit lai pricing holdings vi model hien tai chu yeu thiet ke quanh session mot symbol.

### 0.8. Cap Nhat Tiep Theo: Boundary BrokerSimulation 2026-07-04

Da tiep tuc xu ly technical debt "hai duong broker song song".

Ket luan boundary:

- Duong user-facing hien tai la `TradeLifecycleService`.
  - API decisions dung service nay.
  - Replay pending orders dung service nay.
  - Backtest service cung di qua service nay.
- `BrokerSimulation` trong `app/domain/engine/broker.py` hien la event-driven engine module rieng, duoc test boi `test_engine.py`, nhung khong nam tren luong API/frontend hien tai.

Quyet dinh ky thuat:

- Chua rewrite hay hop nhat lon ngay, vi de lam dung can design lai research engine va replay broker chung mot contract.
- Giam drift truoc bang cach:
  - Tao `app/domain/market_rules.py` gom fee/tax constants dung chung.
  - `TradeLifecycleService` va `BrokerSimulation` cung dung constants nay.
  - Them `MarketConstraints.calculate_t_plus_2_ready(...)` vi `EnginePosition` da goi method nay nhung truoc do chua co.

Bug trong engine experimental da sua:

- BUY bi reject vi thieu tien nhung order van nam trong `active_orders`.
  - Da sua de remove rejected order khoi active list.
  - Them regression test.

Test moi cho engine:

- `test_rejected_buy_does_not_remain_active_when_cash_is_insufficient`.
- `test_engine_sell_before_t_plus_2_is_rejected_then_allowed`.

Gate rieng:

- `../.venv/bin/python -m pytest app/tests/test_engine.py -q`: 5 passed.
- Full backend gate sau thay doi nay: `../.venv/bin/python -m pytest -q` pass, 53 passed, 1 skipped.
- Frontend gate van xanh: `npm run lint`, `npm test`, `npm run build`.

Danh gia:

- Drift fee/T+2 giua hai duong da giam.
- Van nen coi `TradeLifecycleService` la canonical broker path cho san pham hien tai.
- Neu muon nang cap nghiem tuc, sprint sau nen tao `BrokerPort`/`ExecutionPolicy` chung, roi cho replay/backtest/engine cung dung mot accounting core. Day la refactor co chon loc, khong phai rewrite toan bo.

### 0.9. Cap Nhat Tiep Theo: Product Completion Checkpoint 2026-07-04

Sau feedback rang "done ky thuat" khong du de ban giao cho khach hang, da chuyen sang product-completion mode va them acceptance baseline ro rang trong:

- `docs/PRODUCT_COMPLETION_PLAN_2026-07-04.md`

Thay doi san pham/engineering moi:

1. Them canonical accounting module:
   - `app/domain/accounting.py`
   - Tinh buy/sell gross, fee, tax, net amount.
   - Tinh net PnL va PnL percent.

2. `TradeLifecycleService` dung accounting module:
   - BUY, SELL, normal close va force liquidation dung chung logic fee/tax.
   - Force liquidation gan `trade_id` cho sell execution va tinh net PnL co fee/tax thay vi gross PnL tho.

3. Them accounting regression tests:
   - `test_accounting.py`
   - Force liquidation net PnL test trong `test_trade_lifecycle.py`.

4. Chay product browser smoke va phat hien bug that:
   - Strategy Lab comparison dung `Promise.all` goi nhieu backtest song song.
   - Tren SQLite local-first, viec nay gay `sqlite3.OperationalError: database is locked`.
   - Da sua Strategy Lab comparison chay backtest tuan tu.

5. Browser smoke sau fix:
   - `npm run smoke:browser`: pass.
   - Flow smoke gom Replay trade, T+2 reject, close sau settlement, Backtest, Strategy Lab, Scanner -> Replay, Analytics.

Gate moi nhat:

- Backend: `../.venv/bin/python -m pytest -q` pass, 55 passed, 1 skipped.
- Frontend: `npm run lint` pass.
- Frontend: `npm test` pass, 9 files, 18 tests.
- Frontend: `npm run build` pass.
- Product browser smoke: `npm run smoke:browser` pass khi backend/frontend local dang chay va demo data da seed.

Danh gia product-readiness sau checkpoint nay:

- Co the coi day la mot release-candidate baseline de review noi bo.
- Chua nen goi la production SaaS, nhung da vuot qua muc "done ky thuat".
- Cac workflow cot loi da co gate lap lai duoc.
- Viec can lam tiep neu tiep tuc hardening: dua browser smoke vao CI/local release checklist mac dinh va tiep tuc mo rong acceptance matrix cho analytics/accounting edge cases.

### 0.10. Ke Hoach Phat Trien Sau Review 2026-07-04

Roadmap chi tiet da duoc cap nhat tai:

- `docs/PRODUCT_COMPLETION_PLAN_2026-07-04.md`

Ket luan danh gia hien tai:

- Sumi la release-candidate baseline cho san pham local-first/daily-candle, khong
  con la prototype rong.
- Chart v5, replay, backtest, scanner va Strategy Lab da co luong chay that va
  browser smoke.
- Chua du bang chung de goi la stable release: analytics chua duoc doi soat voi
  known ledger hand-calculated; browser smoke chua la gate CI bat buoc; working
  tree chua duoc dong goi thanh commit/release artifact sach.
- Khong nen dap toan bo. Can harden theo thu tu domain correctness truoc, UX va
  release engineering sau.

Thu tu ke hoach bat buoc:

1. M0 - Freeze baseline: review diff, chay lai gate, commit va tao review artifact.
2. M1 - Accounting/ledger: khoa invariant cash, position, fee, tax, settlement va PnL.
3. M2 - Analytics reconciliation: doi soat equity/drawdown/benchmark/metrics voi ledger mau.
4. M3 - Data integrity: no-future-leak, import validation va deterministic golden tests.
5. M4 - UX completion: day du loading/empty/error/recovery va UAT desktop/mobile.
6. M5 - Release engineering: clean install, migration, seed, portable verification va CI smoke.
7. M6 - Go/no-go: khong con P0/P1 va co release evidence lap lai duoc.

Uoc luong con lai cho mot ky su full-stack co kinh nghiem: 14-24 engineering
days, chua gom thoi gian sua defect moi phat hien khi doi soat. Day la uoc luong
hardening, khong phai cam ket lich; moi milestone chi duoc dong khi acceptance
criteria cua milestone do pass.

Danh gia rewrite:

- Chart/replay UI khong con la ung vien rewrite sau migration Lightweight Charts v5.
- Broker/ledger hoac backtest orchestration chi nen thay the co gioi han neu
  acceptance tests chung minh chi phi sua cao hon thay moi.
- Multi-user SaaS, realtime/intraday va real broker integration nam ngoai release
  hien tai va can mot chuong trinh kien truc rieng neu tro thanh muc tieu san pham.

## 1. Tom Tat Dieu Hanh

Sumi la mot ung dung web local-first cho viec luyen tap giao dich va nghien cuu phan tich ky thuat chung khoan Viet Nam. San pham co hai nhanh chinh:

1. Manual Replay Trading Lab: tua lai du lieu lich su tung nen, che dau tuong lai, dat lenh gia lap, ghi journal va xem hieu qua giao dich.
2. Automated Backtest / Research Lab: chay chien luoc khai bao an toan tren du lieu lich su, so sanh theo symbol, giai doan, regime, scanner va Strategy Lab.

Danh gia tong quan: day khong phai la prototype trong trang thai rong. Repo da co kha nhieu module that: FastAPI backend, SQLAlchemy models, Alembic migrations, React frontend, chart, store, API clients, test backend/frontend, docs V2 va release checklist. Tuy nhien, neu muc tieu mong muon la mot san pham trading replay/backtest dung chuan nghiep va dang tin cay, code hien tai van co rui ro kien truc va do chinh xac nghiep vu dang ke.

Khuyen nghi tai thoi diem tiep nhan: khong nen "dap het lam lai" ngay lap tuc.
Audit/fix ban dau da duoc thuc hien va ket qua xac nhan huong refactor co chon loc
la kha thi. Ke hoach canonical tiep theo khong con la uoc luong 1-2 sprint ban
dau; xem muc `0.10` va `docs/PRODUCT_COMPLETION_PLAN_2026-07-04.md`.

Ly do khong nen dap het ngay:

- San pham da co tap tai lieu V2 kha ro, day la tai san co gia tri.
- Da co schema, migration, API, UI va test suite cho cac luong chinh.
- Nhieu P0 cu da duoc xu ly tren code: bo `eval()` truc tiep, them `execution.trade_id`, contract analytics da dong bo hon.

Ly do khong nen tiep tuc them feature nua:

- Kien truc domain chua that su sach: co hai duong broker/accounting song song.
- Backtest dang di qua `ReplaySession` va `TradeLifecycleService`, nhung cach lam nay con mang tinh "tan dung" hon la mot research engine dung nghia.
- Chat luong nghiep vu can kiem chung lai bang du lieu that: T+2, pending order, PnL, equity curve, no-future-leak, indicator warmup.
- Tai thoi diem tiep nhan, moi truong sau clone chua san sang chay gate. Blocker
  nay da duoc xu ly trong workspace hien tai; README da ghi Python 3.12,
  dependency, seed va smoke commands. Clean-checkout reproducibility van la gate
  cua Milestone 5.

## 2. Muc Tieu San Pham Theo Tai Lieu

Tai lieu canonical nam o:

- `docs/INDEX.md`
- `docs/PRODUCT_STRATEGY_V2.md`
- `docs/SPEC_V2.md`
- `docs/MANUAL_REPLAY_SPEC.md`
- `docs/BACKTEST_ENGINE_SPEC.md`
- `docs/ACCEPTANCE_CRITERIA_V2.md`
- `docs/ROADMAP_TO_COMPLETION.md`
- `docs/PROGRESS_V2.md`
- `docs/RELEASE_CHECKLIST_V2.md`
- `docs/DECISIONS.md`

Thesis san pham: Sumi khong chi la mot backtester, ma la "TradingView-like manual replay + disciplined trading journal + technical-analysis research engine + Vietnam market rules + local data ownership".

MVP bat buoc theo docs:

- Import du lieu CafeF.
- Tao replay session theo symbol/timeframe/date range.
- Backend chi tra ve candles da lo dien, tuyet doi khong leak future.
- Indicator trong replay phai tinh theo session/current_index, khong tinh tren toan bo du lieu roi cat o frontend.
- Mo phong giao dich Viet Nam: khong short, phi mua/ban, thue ban, T+2, limit order, bien do tran/san.
- Journal luu setup, reason, confidence, stop, target, mistake, review.
- Analytics tinh PnL, win rate, expectancy, profit factor, R-multiple, drawdown.
- Backtest dung strategy declarative an toan, khong dung Python `eval()` tren input API.
- Gate: backend tests, frontend lint/test/build, Alembic migration.

Trang thai docs moi nhat:

- `docs/PROGRESS_V2.md` ghi V2 da hoan thanh release hardening.
- `docs/RELEASE_CHECKLIST_V2.md` ghi gate gan nhat ngay 2026-06-30: backend pytest pass, Alembic pass, frontend lint/test/build pass, browser smoke pass.
- `docs/PRODUCT_STRATEGY_V2.md` van co mot doan "Current Repo Reality" cu, noi nhieu blocker con ton tai. Khi doi chieu voi code, nhieu blocker trong do da duoc sua, nen tai lieu nay co mot phan bi lech thoi diem.

## 3. Stack Ky Thuat

Backend:

- Python
- FastAPI
- SQLAlchemy ORM
- SQLite mac dinh
- Alembic migration
- Pydantic / pydantic-settings
- pandas / pandas-ta
- pytest
- Mot so connector future/POC: yfinance, vnstock, Binance/Yahoo feeds

Frontend:

- React
- TypeScript
- Vite
- TanStack Query
- Zustand
- Lightweight Charts
- lucide-react
- Vitest / Testing Library
- Playwright cho browser smoke

Database:

- SQLite la duong chinh cho V2.
- `docker-compose.yml` co TimescaleDB/Postgres, nhung docs ghi day la future/optional. Khong nen xem la dependency bat buoc cua V2.

## 4. Ban Do Thu Muc

Root:

- `backend/`: API, services, models, migrations, tests.
- `frontend/`: React app.
- `docs/`: spec, decisions, progress, release checklist, archived pre-v2 docs.
- `scripts/`: release/browser verification scripts.
- `docker-compose.yml`: TimescaleDB optional.

Backend quan trong:

- `backend/app/main.py`: tao FastAPI app, CORS, include routers.
- `backend/app/config.py`: settings, DB URL, DEBUG, AUTO_CREATE_TABLES, CORS origins.
- `backend/app/db.py`: SQLAlchemy engine/session/base.
- `backend/app/models/`: SQLAlchemy models.
- `backend/app/schemas/`: Pydantic response/request schemas.
- `backend/app/api/`: FastAPI route modules.
- `backend/app/services/`: business services.
- `backend/app/domain/`: engine, strategy, indicator, regime, data feed.
- `backend/alembic/versions/`: migrations.
- `backend/app/tests/`: backend tests.

Frontend quan trong:

- `frontend/src/App.tsx`: route shell va lazy pages.
- `frontend/src/pages/`: Import, Replay, Backtest, Scanner, StrategyLab, Analytics, Journal.
- `frontend/src/api/`: API clients theo domain.
- `frontend/src/components/chart/`: Candle chart, drawing, indicators.
- `frontend/src/components/replay/`: session setup, order controls, positions, journal.
- `frontend/src/store/replayStore.ts`: Zustand persisted replay session id.
- `frontend/src/types/`: TypeScript contracts.

## 5. Kien Truc Hien Tai

### 5.1 Kien Truc Tong Quan

Luong du lieu thuc te:

```text
CafeF CSV/TXT/ZIP
-> CafeFImporter
-> symbols + candles
-> ReplaySession
-> ReplayService.get_candles(session_id)
-> UI chart chi nhan visible candles
-> DecisionCreate
-> TradeLifecycleService
-> orders + executions + positions + trades
-> AnalyticsService
-> Analytics UI
```

Luong backtest/scanner:

```text
Strategy YAML / config
-> strategy_loader + StrategyConfig
-> BacktestService / ScannerService
-> indicator calculation
-> rule evaluator
-> backtest creates virtual ReplaySession(mode="backtest")
-> TradeLifecycleService.process_decision()
-> trades/executions
-> AnalyticsService
-> result slices
```

Kien truc mong muon trong docs:

```text
Market Data
-> Indicator Engine
-> Signal/Pattern Engine
-> Broker/Execution Engine
-> Trade Ledger
-> Analytics Engine
-> UI Reports
```

Thuc te gan dung y tuong nay, nhung chua that su tach domain engine sach. Mot so logic quan trong van nam trong service thao tac DB truc tiep.

### 5.2 API Surface

Route chinh:

- `GET /api/health`
- `POST /api/import/cafef`
- `POST /api/import/benchmark`
- `GET /api/symbols`
- `POST /api/replay/sessions`
- `GET /api/replay/sessions`
- `GET /api/replay/sessions/{id}`
- `GET /api/replay/sessions/{id}/candles`
- `POST /api/replay/sessions/{id}/next`
- `POST /api/replay/sessions/{id}/previous`
- `GET/PUT /api/replay/sessions/{id}/drawings`
- `GET /api/replay/sessions/{id}/indicators`
- `POST /api/replay/sessions/{id}/decisions`
- `GET /api/replay/sessions/{id}/decisions`
- `GET /api/replay/sessions/{id}/position`
- `GET /api/replay/sessions/{id}/orders`
- `GET /api/replay/sessions/{id}/trades`
- `GET /api/replay/sessions/{id}/analytics`
- `GET /api/indicators/registry`
- `GET /api/indicators/{symbol}`
- `POST /api/backtest/run`
- `GET /api/backtest/strategies`
- `POST /api/scanner/run`
- `GET /api/scanner/runs`
- `GET /api/scanner/runs/{id}`
- `POST /api/scanner/replay-session`
- `POST /api/strategy-lab/sweep`
- `POST/GET /api/strategy-lab/runs`
- `GET/DELETE /api/strategy-lab/runs/{id}`
- `WS /api/ws/replay/{session_id}`

### 5.3 Data Model

Core tables:

- `symbols`: symbol metadata, exchange, sector, industry.
- `candles`: OHLCV by symbol/timeframe/timestamp/adjustment_type.
- `replay_sessions`: session state, current_index, cash, mode, source signal.
- `decisions`: manual/backtest decisions.
- `orders`: order side/type/price/quantity/status.
- `executions`: filled executions, includes `trade_id`.
- `positions`: current/closed positions.
- `trades`: lifecycle-level trade record.
- `journal_entries`: notes/reviews.
- `event_logs`: system activity.
- `drawing_states`: persisted drawing JSON.
- `strategy_lab_runs`: saved strategy lab outputs.
- `scanner_runs`: saved scanner outputs.

Nhan xet data model:

- Data model da du de chay MVP replay/backtest.
- `executions.trade_id` la sua doi dung huong de giai quyet PnL nhieu round-trip.
- Chua co cac bang professional v1 nhu `indicator_definitions`, `signal_definitions`, `market_regimes`, `backtest_runs`, `backtest_trades`, `scan_results`, `sector_memberships` nhu trong `SPEC_V2.md`. Hien tai scanner/backtest run history luu payload JSON thay vi model normalized.
- Viec luu `source_payload`, scanner/history/result bang JSON giup ra nhanh, nhung se kho query/analytics ve sau.

## 6. Cong Nang Hien Co

### 6.1 Import Du Lieu CafeF

Module: `backend/app/services/cafef_importer.py`

Ho tro:

- CSV/TXT.
- ZIP gom CSV/TXT.
- Mapping column CafeF ve `symbol`, `timestamp`, `open`, `high`, `low`, `close`, `volume`.
- Parse ngay CafeF format `YYYYMMDD`.
- Validate data quality: missing, gia <= 0, volume am, high < low, OHLC ngoai range.
- Detect exchange tu filename: HOSE/HSX, HNX, UPCOM.
- Upsert `symbols`.
- Upsert `candles` bang SQLite conflict update.

Rui ro:

- Import dang phu thuoc header/format du lieu CafeF; can test bang file that cua user.
- Validation row-by-row co the cham voi dataset lon.
- Duplicate warning duoc tao, sau do duplicate duoc drop keep last. Can dam bao nguoi dung hieu duplicate nao bi ghi de.

### 6.2 Manual Replay

Module:

- `backend/app/services/replay_service.py`
- `backend/app/api/replay.py`
- `frontend/src/pages/ReplayPage.tsx`
- `frontend/src/components/chart/CandleChart.tsx`
- `frontend/src/components/replay/*`

Ho tro:

- Tao replay session theo symbol/timeframe/adjustment/date range/initial cash.
- `current_index` bat dau 0.
- `get_candles()` chi query `limit = current_index + 1`.
- Next/previous candle.
- Multi-timeframe query voi target timeframe, nhung van cat theo current timestamp.
- Session list/resume.
- Drawing persistence.
- WebSocket replay playback.
- Scanner-to-replay launch co source metadata.

Diem dung:

- No-future-leak candles duoc enforce o backend.
- Indicator endpoint cua replay dung `ReplayService.get_candles()`, nen ve nguyen tac khong leak future.

Rui ro:

- Previous/rewind khong xoa decisions cu. Test co xac nhan decisions van giu lai khi rewind. Ve san pham, day co the dung neu muon audit, nhung UI/UX phai noi ro vi user co the thay decision o candle tuong lai so voi chart hien tai.
- `next_candle()` commit session.current_index truoc, sau do mark-to-market, pending orders, bankruptcy. Can test ky transaction consistency neu pending order fail giua chung.
- Khi session completed, logic van goi mark-to-market/pending matching. Can UAT xem hanh vi end-of-session co dung ky vong.

### 6.3 Trading Simulation

Module chinh: `backend/app/services/trade_lifecycle_service.py`

Ho tro:

- Actions: BUY, ADD, SELL, REDUCE, CLOSE, CUT_LOSS, TAKE_PROFIT, HOLD, SKIP.
- Order types: market-at-close va limit.
- Fee/tax:
  - Buy fee: 0.15%
  - Sell fee: 0.15%
  - Sell tax: 0.1%
- T+2 sell constraint.
- No shorting / insufficient position rejection.
- Pending limit order.
- Price band HOSE/HNX/UPCOM.
- Position average price.
- Trade record open/close.
- Execution co `trade_id`.
- Force liquidation khi equity <= 0.

Diem dung:

- Test co cover T+1 rejected, T+2 allowed, full lifecycle fee/tax, multi-round-trip PnL, limit order above ceiling rejected, limit pending/fill.

Rui ro thiet ke:

- `TradeLifecycleService` vua tao decision/order/execution/position/trade, vua enforce rule, vua commit DB. Day la service lon, kho test isolated va kho transaction orchestration.
- Chi quan ly mot open position theo `session_id` va status open, khong filter symbol day du o mot so doan. V2 moi session chu yeu mot symbol nen tam chap nhan, nhung se vo neu mot session multi-symbol.
- T+2 duoc tinh bang `Decision.candle_index <= current_index - 2`, day la "2 candle/session bars", khong phai lich giao dich/settlement phuc tap hon. Voi daily candles thi tam dung cho MVP.
- Gia khop market la close hien tai. Neu mong muon market-next-open/ATC/ATO dung thuc te hon thi can chinh lai design.
- Co mot `BrokerSimulation` rieng trong `backend/app/domain/engine/broker.py` voi event dispatcher, slippage, commission va portfolio. No co test rieng nhung khong phai duong chinh cua replay/backtest hien tai. Day la dau hieu kien truc dang tach doi.

### 6.4 Indicator Engine

Module: `backend/app/domain/engine/indicator_engine.py`

Ho tro registry:

- SMA
- EMA
- MACD
- RSI
- Bollinger Bands
- ATR
- ADX
- Ichimoku
- Stochastic
- Volume SMA
- PSAR
- SuperTrend
- CCI
- MFI
- Keltner Channels

Diem dung:

- Indicator co id/label/category/pane/method/params/aliases/description.
- Parameter validation co min/max.
- Unknown params/unknown indicator bi reject.
- Replay indicator endpoint dung visible candles.

Rui ro:

- BacktestService lai co `_compute_indicators()` rieng cho SMA/EMA/RSI/MACD, khong dung chung `IndicatorEngine`. Day la duplication va co nguy co replay/backtest tinh indicator khac nhau.
- `pandas-ta` dependency co the la nguon cai dat loi tren mot so moi truong Python moi/cu.

### 6.5 Analytics

Module:

- `backend/app/services/analytics_service.py`
- `backend/app/schemas/analytics_schema.py`
- `frontend/src/types/analytics.ts`
- `frontend/src/pages/AnalyticsPage.tsx`
- `frontend/src/components/analytics/EquityChart.tsx`

Ho tro:

- Total trades.
- Win rate.
- Total net PnL.
- Average win/loss.
- Profit factor.
- Expectancy.
- Largest win/loss.
- R multiple / average R.
- Max drawdown amount/pct.
- Drawdown periods.
- Sharpe.
- Sortino.
- SQN.
- Setup performance.
- Symbol performance.
- Mistake performance.
- Benchmark curve VNINDEX.
- Trade distribution.
- Outlier impact.

Diem dung:

- Equity curve contract da theo V2: `timestamp`, `equity`, `cash`, `holdings_value`, `drawdown`, `drawdown_pct`.
- Frontend type da match backend schema.

Rui ro:

- Neu khong co closed trade, `get_analytics()` return response toi thieu, khong build equity curve. Neu user dang co open position, analytics co the khong hien equity/drawdown nhu mong muon.
- Equity curve holdings value dung close price cua candle session cho tat ca holdings. V2 session mot symbol thi ok; multi-symbol se sai.
- Benchmark curve dung field `time/value`, khac equity point. Frontend dang xu ly duoc, nhung contract khong hoan toan dong nhat.
- Metrics tai chinh nhu Sharpe/Sortino/SQN dang o muc simplified, can audit neu dung de ra quyet dinh nghien cuu that.

### 6.6 Backtest

Module:

- `backend/app/services/backtest_service.py`
- `backend/app/domain/strategy/*`
- `backend/app/api/backtest.py`
- `frontend/src/pages/BacktestPage.tsx`

Ho tro:

- Run single-symbol va multi-symbol.
- Load strategy tu YAML/config.
- Sample strategies: MA crossover, MACD RSI momentum.
- Rule syntax dang co hai mode:
  - Condition string an toan qua AST whitelist.
  - Declarative DSL: `gt`, `gte`, `lt`, `lte`, `eq`, `all`, `any`, `not`, `cross_up`, `cross_down`, `between`, `rising`, `falling`.
- Reject malicious expression nhu `__import__('os')`.
- No-data returns failed status.
- Multi-symbol summary.
- Result slices by symbol, period, regime.

Diem dung:

- Khong con thay `eval()` truc tiep trong backtest service.
- Error backtest tra ve status failed/error_code/message thay vi chi print traceback.
- Dung `TradeLifecycleService.process_decision()` de chia se mot phan accounting voi manual replay.

Rui ro lon:

- Backtest tao `ReplaySession(mode="backtest")` va ghi vao DB that. Neu user chay nhieu backtest, database co the day session/trade/execution "ao". Can co lifecycle/cleanup/persisted run model ro hon.
- `_compute_indicators()` trong backtest bi tach khoi IndicatorEngine. Indicator replay va backtest co the khac cong thuc/warmup.
- Backtest loop bat dau tu index 1, khong co warmup policy ro rang ngoai viec NaN -> None -> false.
- Entry/exit order market-at-close tai close hien tai co the gay assumption khac voi backtest ky vong.
- Chua co position sizing/risk engine day du. Stop loss/take profit trong schema co nhung chua thay dung day du trong loop.
- Chua co forced liquidation/end-of-test close position ro rang. Neu con open trade cuoi ky, analytics chi tinh closed trades.

### 6.7 Scanner

Module:

- `backend/app/services/scanner_service.py`
- `backend/app/api/scanner.py`
- `frontend/src/pages/ScannerPage.tsx`

Ho tro:

- Chay entry rules cua strategy tren danh sach symbols.
- Tinh indicators bang backtest service.
- Gan regime theo benchmark/VNINDEX.
- Save scanner run vao DB.
- Tao replay session tu signal scanner voi lookback/forward window.

Rui ro:

- Scanner chi bat entry signals, chua phai full signal taxonomy/pattern engine nhu docs dai han.
- Result luu JSON payload, nhanh nhung kho query/aggregate.
- Chat luong scanner phu thuoc backtest indicator/rule duplication.

### 6.8 Strategy Lab

Module:

- `backend/app/services/strategy_lab_service.py`
- `backend/app/api/strategy_lab.py`
- `frontend/src/pages/StrategyLabPage.tsx`

Ho tro:

- Strategy comparison.
- Parameter sweep.
- Persist saved runs vao DB.
- Frontend con co localStorage history.

Rui ro:

- Ket qua lab luu payload JSON, chua co normalized table cho backtest_runs/trades/slices.
- Sweep path/value co the can validation manh hon neu mo rong cho user.

### 6.9 Frontend UX

Pages:

- Import Page: upload CafeF.
- Replay Page: chart, session setup/resume, indicators, drawings, trade controls, positions, pending orders, journal.
- Backtest Page: run strategy.
- Scanner Page: run scanner va open replay tu signal.
- Strategy Lab Page: compare/sweep/save runs.
- Analytics Page: session analytics.
- Journal Page: session journal entries.

Diem dung:

- Frontend da co actual app, khong phai landing page.
- API clients tach theo domain.
- Zustand persist `sessionId` giup resume session.
- ErrorBoundary co test.

Rui ro:

- UI co kha nhieu inline style va logic trong page components lon, dac biet ReplayPage/StrategyLabPage.
- `App.tsx` co trang root welcome don gian, khong redirect vao luong chinh.
- Can browser UAT bang data that de biet "chua hoat dong dung mong muon" nam o UX, nghiep vu hay data.

## 7. Kiem Thu Va Gate

Backend tests tim thay:

- `test_engine.py`
- `test_strategy_lab.py`
- `test_analytics_advanced.py`
- `test_scanner.py`
- `test_trade_lifecycle.py`
- `test_api_integration.py`
- `test_analytics.py`
- `test_replay_no_future_leak.py`
- `test_backtest.py`
- `test_regime_classifier.py`
- `test_cafef_importer.py`
- `test_indicators.py`
- `test_strategy_lab_history.py`

Frontend tests tim thay:

- `frontend/src/utils/date.test.ts`
- `frontend/src/components/common/ErrorBoundary.test.tsx`
- `frontend/src/components/replay/__tests__/SessionSetup.test.tsx`
- `frontend/src/pages/__tests__/StrategyLabPage.test.tsx`
- `frontend/src/pages/__tests__/BacktestPage.test.tsx`
- `frontend/src/pages/__tests__/AnalyticsPage.test.tsx`
- `frontend/src/pages/__tests__/ScannerPage.test.tsx`

Release gate script:

- `scripts/verify-v2.ps1`
- Chay backend pytest.
- Chay Alembic upgrade.
- Chay frontend lint/test/build.
- Tuy chon BrowserSmoke.

Trang thai moi truong khi audit:

- Repo da clone thanh cong.
- Local `master` bang `origin/master`.
- Chua co `backend/.venv`.
- Chua co `frontend/node_modules`.
- Python system la 3.9.6.
- Thu chay `python3 -m pytest backend/app/tests -q` fail vi `No module named pytest`.
- Do do chua xac nhan duoc gate hien tai tren may nay.

Ket luan test:

- Ve mat codebase, test coverage co chu y den cac rui ro dung: no-future-leak, T+2, PnL, security rule, indicators, scanner, strategy lab.
- Ve mat ban giao, chua the noi "dang pass" neu khong dung dung moi truong `.venv`/npm dependencies va chay gate.

## 8. Cau Hinh Va Van Hanh

Config backend:

- `DATABASE_URL`: default `sqlite:///./sumi.db`
- `DEBUG`: default `False`
- `AUTO_CREATE_TABLES`: default `False`
- `CORS_ALLOWED_ORIGINS`: default `http://localhost:5173,http://127.0.0.1:5173`

Nhan xet:

- Config default da an toan hon so voi blocker cu.
- `AUTO_CREATE_TABLES=False` nghia la can Alembic migration de tao DB, dung cho release.
- README huong dan Windows PowerShell:
  - Backend: `.\.venv\Scripts\python.exe -m uvicorn app.main:app --reload --port 8000`
  - Frontend: `npm.cmd install`, `npm.cmd run dev`
- Tren macOS/Linux can chinh lenh tuong ung.

## 9. Rui Ro Chinh

### R1. Domain core chua du sach de mo rong

`TradeLifecycleService` la duong chinh, nhung `BrokerSimulation` ton tai song song. Neu tiep tuc phat trien backtest/replay phuc tap, kha nang drift logic cao.

Tac dong:

- Manual replay va automated backtest co the tinh khac nhau.
- Fix T+2/fee/order mot noi co the khong anh huong noi kia.
- Kho test nhung rule tai chinh phuc tap.

Khuyen nghi:

- Chon mot duong broker/accounting duy nhat.
- Tot nhat la viet domain ledger/broker thuong Python, pure function/class, DB service chi la adapter.

### R2. Backtest dang tan dung ReplaySession qua nhieu

Backtest tao session ao va ghi DB theo luong manual replay. Cach nay giup share accounting nhanh, nhung khong phai model research chuan.

Tac dong:

- DB bi lan manual session va backtest session.
- Analytics chi tinh closed trades, open trade cuoi ky co the bi bo qua.
- Kho luu/replay lai backtest run mot cach structured.

Khuyen nghi:

- Neu giu code hien tai, them cleanup/persistence policy ro cho `mode="backtest"`.
- Neu rewrite core, tao `backtest_runs`, `backtest_trades`, `equity_points`, `result_slices` rieng.

### R3. Indicator engine bi duplicate

Replay/API indicator dung `IndicatorEngine`, backtest dung `_compute_indicators()` rieng.

Tac dong:

- Cung mot indicator co the cho ket qua khac giua Replay va Backtest.
- Kho them indicator moi mot lan dung o moi cho.

Khuyen nghi:

- BacktestService phai dung chung IndicatorEngine hoac mot batch engine chung.
- Define warmup behavior ro.

### R4. Analytics co kha nang sai neu vuot ngoai one-symbol session

Equity curve tinh holdings value bang close price cua candle session cho tat ca holdings. V2 one-symbol thi ok, multi-symbol thi sai.

Tac dong:

- Neu backtest multi-symbol duoc combine analytics trong tuong lai, equity/drawdown sai.

Khuyen nghi:

- Giai doan ngan han: document "session/backtest analytics la per-symbol".
- Giai doan sau: equity engine can price lookup theo symbol/timestamp.

### R5. UI/UX chua duoc xac minh voi du lieu that

Docs noi browser smoke da pass, nhung tren moi truong hien tai chua chay duoc. User bao "chua hoat dong dung mong muon", co the loi nam o luong UX hon la code core.

Khuyen nghi:

- Lam mot UAT script voi file CafeF that va expected behavior cu the.
- Ghi lai bug theo luong: import, tao session, replay, indicator, buy/sell, analytics.

### R6. Docs lech version

`PRODUCT_STRATEGY_V2.md` con ghi blocker cu, trong khi progress/release checklist noi complete.

Tac dong:

- Nguoi moi nhan ban giao de hieu sai trang thai.

Khuyen nghi:

- Cap nhat docs: tach "historical blocker" va "current known issues".

## 10. Khoang Cach So Voi Mong Muon San Pham Chuyen Nghiep

Neu mong muon la mot cong cu ca nhan de luyen replay va test nhanh strategy, code hien tai co nen tang kha tot.

Neu mong muon la cong cu nghien cuu/backtest dang tin cay nhu mot engine chuyen nghiep, can them:

- Ledger/accounting engine duy nhat, deterministic, test bang fixtures tai chinh.
- Backtest result model rieng, khong ghi lan vao replay sessions.
- Indicator computation chung cho replay/backtest/scanner.
- Data quality pipeline manh hon cho CafeF/VNINDEX/split-adjusted data.
- Market calendar/settlement model dung hon thay vi chi dem candle index.
- End-of-test position handling.
- Risk management: stop loss, take profit, sizing, exposure, max allocation.
- Export/reporting.
- Normalized runs/trades/slices de query.
- UI UAT theo workflow thuc te.

## 11. Danh Gia "Co Nen Dap Het Lam Lai Khong?"

### Phuong An A: Dap het lam lai

Chi nen chon neu:

- Muc tieu san pham da khac hoan toan voi V2 docs.
- User can engine chuyen nghiep, multi-symbol portfolio-level, data scale lon, reporting chuan.
- Team chap nhan mat 2-4 tuan dau de rebuild foundation truoc khi co UI dung duoc.
- Code hien tai sau khi chay gate/UAT fail tren nhieu luong core va sua tiep ton hon rebuild.

Loi ich:

- Co the thiet ke lai domain core dung ngay tu dau.
- Giam no ky thuat song song BrokerSimulation/TradeLifecycleService.
- Co schema backtest/research tot hon.

Bat loi:

- Mat toan bo tien do UI/API/test hien co neu lam qua tay.
- De lap lai loi cu neu khong giu spec/test.
- Khong can thiet neu van chi can local-first replay MVP.

### Phuong An B: Refactor co chon loc

Nen chon neu:

- Muon dua san pham ve dung mong muon nhanh nhat.
- Core problem nam o logic giao dich, analytics, UX, data import, khong phai toan bo stack.
- Chap nhan dong bang feature moi trong 1-2 sprint.

Loi ich:

- Giu duoc docs, frontend, API, tests, migrations.
- Tap trung sua dung cac luong cot loi.
- It rui ro hon rewrite toan phan.

Bat loi:

- Can ky luat cao de khong tiep tuc va chong no ky thuat.
- Mot so module lon co the van phai tach lai.

Khuyen nghi: chon Phuong An B truoc.

## 12. Ke Hoach Audit/Fix De Ra Quyet Dinh Cuoi

### Sprint 0: Dung Moi Truong Va Gate

Muc tieu:

- Tao `.venv`, cai backend requirements.
- `npm install`.
- Chay `scripts/verify-v2.ps1` hoac lenh tuong duong macOS.
- Chay browser smoke neu co backend/frontend running.

Output:

- Mot report pass/fail that.
- Log loi dependency/test/build neu co.

### Sprint 1: UAT Theo Luong That

Dung 1 dataset CafeF that va checklist:

1. Import du lieu.
2. Kiem tra symbols/exchange.
3. Tao replay session.
4. Kiem tra candles khong leak future.
5. Add EMA/RSI/MACD.
6. BUY.
7. SELL T+1 bi reject.
8. SELL T+2 thanh cong.
9. Limit order invalid bi reject.
10. Limit order valid pending/fill.
11. Drawings persist.
12. Journal save.
13. Analytics render va so sanh PnL bang tay.
14. Run MA Crossover backtest.
15. Run MACD RSI Momentum backtest.
16. Run scanner -> open replay from signal.

Output:

- Danh sach bug theo severity.
- Phan loai bug: data, backend domain, frontend UX, infra.

### Sprint 2: Fix Foundation

Neu bug tap trung vao domain:

- Hop nhat indicator engine.
- Tach broker/accounting core.
- Sua analytics open-position/equity.
- Lam ro backtest session lifecycle.

Neu bug tap trung vao UX:

- Sua ReplayPage controls/error states.
- Sua session setup/resume.
- Sua chart/indicator rendering.
- Sua analytics display.

Sau Sprint 2 moi quyet dinh:

- Neu core da on: tiep tuc refactor co chon loc.
- Neu core van sap/sai nhieu: rewrite domain core, giu frontend/docs/test lam khung.

## 13. Danh Sach File Nen Doc Truoc Khi Sua

Docs:

- `README.md`
- `docs/INDEX.md`
- `docs/SPEC_V2.md`
- `docs/MANUAL_REPLAY_SPEC.md`
- `docs/BACKTEST_ENGINE_SPEC.md`
- `docs/ACCEPTANCE_CRITERIA_V2.md`
- `docs/DECISIONS.md`
- `docs/RELEASE_CHECKLIST_V2.md`

Backend:

- `backend/app/main.py`
- `backend/app/config.py`
- `backend/app/models/*.py`
- `backend/app/services/replay_service.py`
- `backend/app/services/trade_lifecycle_service.py`
- `backend/app/services/analytics_service.py`
- `backend/app/services/backtest_service.py`
- `backend/app/services/scanner_service.py`
- `backend/app/domain/strategy/rule_evaluator.py`
- `backend/app/domain/engine/indicator_engine.py`
- `backend/app/domain/engine/broker.py`
- `backend/app/tests/test_replay_no_future_leak.py`
- `backend/app/tests/test_trade_lifecycle.py`
- `backend/app/tests/test_backtest.py`
- `backend/app/tests/test_analytics_advanced.py`

Frontend:

- `frontend/src/App.tsx`
- `frontend/src/pages/ReplayPage.tsx`
- `frontend/src/pages/BacktestPage.tsx`
- `frontend/src/pages/ScannerPage.tsx`
- `frontend/src/pages/StrategyLabPage.tsx`
- `frontend/src/pages/AnalyticsPage.tsx`
- `frontend/src/api/*.ts`
- `frontend/src/types/analytics.ts`
- `frontend/src/store/replayStore.ts`
- `frontend/src/components/chart/CandleChart.tsx`

## 14. Ket Luan

Sumi hien tai la mot release candidate co nhieu thanh phan da duoc xay dung, khong phai mot du an vo nghia can xoa trang ngay. Nhung no cung chua du chac de tiep tuc build feature tren do neu user dang thay "chua hoat dong dung mong muon".

Quyet dinh hop ly nhat:

1. Dung moi truong va chay gate that.
2. UAT bang du lieu that theo luong manual replay.
3. Dong bang feature moi.
4. Sua/tach lai domain foundation trong 1-2 sprint.
5. Chi rewrite neu sau khi audit, loi nam o thiet ke core va chi phi sua lon hon chi phi viet lai.

Neu phai dua ra mot cau ngan gon de bao cao:

> Khong nen dap het ngay. Nen coi day la mot V2 candidate co nen tang dung duoc nhung domain core chua du sach. Can audit gate + UAT that, sau do refactor co chon loc. Neu manual replay, broker/accounting va analytics van sai sau dot fix foundation, luc do moi nen rewrite core, khong rewrite toan bo frontend/docs/test.
