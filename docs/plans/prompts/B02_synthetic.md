# Prompt B02 — Synthetic deny + quarantine (GAP-DB-04)

Đọc: ADR-PROV-001, DEV_BATCHES B02, providers ssi/vnstock, `sync_workflow.execute`.
Nhiệm vụ: default-deny execute synthetic trừ `SUMI_ALLOW_SYNTHETIC_PROVIDERS=1`; `is_synthetic` + quarantine dataset + banner "dữ liệu giả"; credentials về Settings.
Cấm chạm prod candles/sync_runs. Verify: stub mặc định 403; bật cờ mới ghi quarantine.
