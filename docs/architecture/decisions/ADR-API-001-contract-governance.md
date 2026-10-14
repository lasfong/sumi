# ADR-API-001 — Contract Governance (snapshot + envelope + no silent fallback)

**Status:** Accepted 2026-10-09.
**Context:** 15 mismatches (BANKRUPT, CUSTOM_PRICE, D/split, BB envelope, required/optional đảo, TradePlan 7 thiếu, checklist, signal max2/forbid/params, Candle.source, catalog đôi, 422 array→`[object Object]`, `detail=str(e)`, dead importCafef/data_feeds) (GAP-API-01..11).
**Decision:** Một envelope chuẩn mỗi domain + `response_model` mọi GET; snapshot test Pydantic↔TS (openapi typegen, version pin); error envelope `{code,message,field_errors[]}`, cấm `str(e)`; FE normalize 422; một route catalog; một `signalsClient` pin version; xóa dead.
**Alternatives:** Sửa FE cho vừa BE từng chỗ — loại: drift tái diễn.
**Verification:** Contract snapshot CI đỏ khi lệch; UAT không `[object Object]`; BB 2 formats unify.
