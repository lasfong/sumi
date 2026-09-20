# DEV Implementation Plan

Thứ tự triển khai đề xuất. DEV không nên bắt đầu từ việc vẽ oscillator; phải bắt đầu từ inventory/audit data rồi mới kích hoạt đúng computation mode.

# 15. DEV implementation plan

| **Phase**                         | **Goal**                                                                                    | **Required deliverable**                           | **Exit criterion**                                                          |
|-----------------------------------|---------------------------------------------------------------------------------------------|----------------------------------------------------|-----------------------------------------------------------------------------|
| R0 - Source inventory             | Inventory every existing CTCK/CafeF/public endpoint and stored table.                       | source_catalog.md + sample payloads.               | All candidate fields have source/semantic/date coverage.                    |
| R1 - Capability audit             | Determine true-flow feasibility by source/exchange/history.                                 | data_capability_matrix.csv + DQ report.            | Mode decision possible for each data segment.                               |
| R2 - TRUE_FLOW research prototype | Implement deterministic trade classification and daily Buy/Sell aggregation where possible. | Research notebook/module + metrics.                | Classification coverage/accuracy meets documented threshold or is rejected. |
| R3 - OHLCV_PROXY prototype        | Implement fallback with actual matched value if available.                                  | Proxy research module + benchmark against MFI/CMF. | Deterministic output, bounded, no look-ahead.                               |
| R4 - Multi-horizon + market       | Add rolling horizons, point-in-time universe, market aggregation and breadth.               | Daily symbol + market datasets.                    | Identity/coverage tests pass.                                               |
| R5 - Visual evaluation            | Plot BB03/05/20/50/200 and annotate candidate events.                                       | Research dashboard/charts.                         | Behavior is interpretable; thresholds documented statistically.             |
| R6 - Freeze V1 spec               | Freeze methodology, parameters, schema and labels.                                          | versioned specification + golden fixtures.         | All acceptance tests pass; data claims match mode.                          |
