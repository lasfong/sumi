# CONTRACT TEST STRATEGY (Pydantic <-> TS)

## Muc tieu
- Moi response GET co response_model; 1 envelope/domain; khong any/optional-abuse/silent-fallback.
- Phat hien drift truoc khi FE su dung.

## Co che
1. Export OpenAPI tu FastAPI (test) -> snapshot json versioned.
2. Typegen hoac snapshot TS types (types/replay, trade, candle, analytics, decision, journal, signals, bb) -> diff CI.
3. Rieng BB: snapshot 2 formats cu -> assert 1 envelope moi + map horizons[H].bb_value.
4. Error envelope: assert {code,message,field_errors[]}; cam detail=str(e); FE normalize 422 array (khong [object Object]).
5. Enum parity: BANKRUPT, CUSTOM_PRICE, SessionMode/CreateSessionMode(backtest), Candle.source, checklist dict|str, signal max2/forbid/params/output_type.
6. Dead routes: assert /api/import/catalog 404 hoac redirect; importCafefData removed; data_feeds isolated.
7. Version pin: signals calculate + bundle cung version; BB methodology_version hien thi.

## Gate
- Per-batch cham schemas/API/FE types: snapshot test xanh moi merge.
- Dual-read (VARCHAR/JSON) 1 release khi migrate checklist.
- UAT mau: gay 422 co y -> UI hien field error tieng Viet, khong crash.

## Trace
- GAP-API-01..11, GAP-FE-07/08 -> ACCEPTANCE G-01/G-03.
