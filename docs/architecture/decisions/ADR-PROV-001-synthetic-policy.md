# ADR-PROV-001 — Provider & Synthetic Data Policy (quarantine by default-deny)

**Status:** Accepted 2026-10-09.
**Context:** Stub SSI/vnstock `md5+sin` không HTTP nhưng `execute_sync` ghi `Candle source=sync:id` thật + derive weekly, không cờ/quarantine; live `sync_runs=0`; secrets `os.getenv` rải rác; `DataFeedService` bypass (GAP-DB-04/12/13).
**Decision:** Default-deny: `execute_sync` với provider synthetic bị từ chối trừ `SUMI_ALLOW_SYNTHETIC_PROVIDERS=1`; khi bật: ghi quarantine dataset tách prod + banner "dữ liệu giả" + provenance `is_synthetic`; credentials gom `Settings` + masking; deprecate `DataFeedService`/data_feeds.
**Alternatives:** Cho phép stub ghi prod có flag — loại: vẫn nhiễm weekly/rollback/provenance.
**Verification:** Execute stub mặc định bị từ chối (test); bật cờ mới ghi quarantine; temp-DB hash prod unchanged.
