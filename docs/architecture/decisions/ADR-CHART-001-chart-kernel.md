# ADR-CHART-001 — ChartKernel Thin Facade (keep LWC v5)

**Status:** Accepted 2026-10-09.
**Context:** 2 `createChart`, calls rải rác, `any/never`, snapshot DOM; god 1366 dòng; RenderRegistry `return []` câm; legacy drawing 3 file dead nhưng workspaceTypes còn sống (GAP-FE-03/04/05/07/09).
**Decision:** Một `ChartKernel` facade mỏng (create/series/pane/primitive attach/destroy), cấm gọi LWC trực tiếp ngoài facade; tách god thành Header/ChartPanel/RightPanel/Dock + hooks; RenderRegistry trả `MappingResult`; xóa 3 file legacy drawing; debug sau DEV flag.
**Alternatives:** Full abstraction thay lib — loại: vi phạm ADR-001 provider gate, tốn kém vô ích.
**Verification:** Không còn `createChart` ngoài facade (grep); UAT pan/zoom/resize/advance/reload giữ drawings; mapping-missing surface có UI.
