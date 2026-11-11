# SUMI Money Flow Blackbox V1 — Data-Ready Handoff v3

## Mục tiêu

Bộ tài liệu này chuyển Money Flow Blackbox sang kiến trúc **build-now, plug-data-later**:

- DEV dựng ngay BB engine, database schema, adapters, SUMI-420 aggregation và API output.
- Core không phụ thuộc một vendor cụ thể.
- Khi tìm được historical Active Buy/Sell data: backfill database và chạy ngay.
- Nếu chưa có historical True Flow: dùng `OHLCV_PROXY` cho lịch sử.
- Nếu có realtime Active Buy/Sell từ hôm nay: bắt đầu tự collect ngay để xây history riêng.
- “Market” của Sumi V1 được định nghĩa là **SUMI-420**, khoảng 420 mã trọng yếu, ít thay đổi.

## Quyết định chính

1. Target tốt nhất: `ACTIVE_TRADE_FLOW_VALUE`.
2. Chấp nhận tốt: `ACTIVE_TRADE_FLOW_VOLUME`.
3. Khi chỉ có volume: `ACTIVE_VOLUME_VALUE_ESTIMATE` cho aggregate thị trường.
4. Historical fallback: `OHLCV_PROXY`.
5. Mọi output phải có `flow_method`, `source_id`, `method_version`, `quality`.
6. `T03/T05/T10/T20/T50/T200` là rolling horizons, không reset block, không phải EMA ribbon.

## Thứ tự đọc

1. `00_EXECUTIVE_DECISION.md`
2. `01_TARGET_ARCHITECTURE.md`
3. `02_DATA_REQUIREMENTS_AND_CONTRACTS.md`
4. `03_DATA_SOURCE_RESEARCH.md`
5. `04_COLLECTION_AND_RPA_STRATEGY.md`
6. `05_DATABASE_AND_PIPELINE_SPEC.md`
7. `06_BB_ENGINE_SPEC.md`
8. `07_FALLBACK_AND_METHOD_BOUNDARIES.md`
9. `08_SUMI420_MARKET_SPEC.md`
10. `09_DORAEMON_AUDIT_PROMPT.md`
11. `10_DEV_IMPLEMENTATION_PLAN.md`
12. `11_VALIDATION_AND_ACCEPTANCE.md`
13. `12_REFERENCES.md`

`MASTER_SPEC.md` là bản hợp nhất.
