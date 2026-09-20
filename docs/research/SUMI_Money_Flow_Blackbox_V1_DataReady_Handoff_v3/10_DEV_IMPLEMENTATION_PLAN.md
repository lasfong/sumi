# 10 — DEV Implementation Plan

## Phase 0 — Contracts ngay
- enums `flow_method`;
- DB migrations;
- adapter interfaces;
- SUMI-420 tables;
- calculator contracts;
- source/method/version metadata.

Không phụ thuộc vendor.

## Phase 1 — OHLCV Proxy research
Dùng history hiện có tính BB03/05/10/20/50/200 và làm test harness.

## Phase 2 — Active Flow fixtures
CSV synthetic:

```text
date,symbol,active_buy_value,active_sell_value
```

để test full True Flow pipeline.

## Phase 3 — Adapter theo audit
Có thể là:

```text
VnstockDataActiveFlowAdapter
SSIStreamingTradeAdapter
FireAntDailyActiveFlowAdapter
CsvImportAdapter
UiPathDropFolderAdapter
```

## Phase 4 — Forward collector
Nếu chỉ có realtime:
- deploy sớm;
- raw persistence;
- EOD aggregate;
- quality report;
- alerts.

Mỗi ngày không collect là mất một ngày future history.

## Phase 5 — Historical backfill
Khi tìm được source, import + reconcile + rerun BB. Core không đổi.

## Phase 6 — SUMI-420 Market BB
Aggregate active values/estimates, breadth, coverage.

## Phase 7 — Empirical calibration
Không optimize P&L.
Nghiên cứu distribution, extremes, turns, confluence, Proxy-vs-True overlap.

## Framework Definition of Done

1. Add vendor bằng adapter không sửa BB formula.
2. Import CSV active flow là BB chạy.
3. Proxy coexist nhưng không conflated.
4. SUMI-420 chạy trên fixtures.
5. Mọi output trace source/method/version/quality.
6. Raw collector reprocess idempotently.
