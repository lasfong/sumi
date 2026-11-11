# Sumi Documentation Index

Tài liệu dự án **Sumi V3** (Technical Analysis & Strategy Lab for Vietnam Stock Market).

---

## Tài Liệu Hiện Hành

| File | Mục đích |
|---|---|
| [V3_FINAL_HANDOFF_REPORT_2026-09-12.md](file:///e:/Workspace/sumi/docs/V3_FINAL_HANDOFF_REPORT_2026-09-12.md) | Báo cáo bàn giao chính thức: kiến trúc hai trụ cột, 31 kịch bản E2E, bằng chứng kiểm định 100%, hướng dẫn vận hành |
| [PRODUCT_ACCEPTANCE_CRITERIA_V3.md](file:///e:/Workspace/sumi/docs/PRODUCT_ACCEPTANCE_CRITERIA_V3.md) | Tiêu chuẩn nghiệm thu định lượng V3 (G, R, I, D, T) — hợp đồng chất lượng bắt buộc |
| [ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md](file:///e:/Workspace/sumi/docs/ARCHITECTURE_DECISION_001_REPLAY_UI_REBUILD.md) | ADR-001: Tái thiết kế Replay UI, ranh giới Canvas/Provider/State |
| [ARCHITECTURE_DECISION_002_MARKET_DATA_PROVIDER.md](file:///e:/Workspace/sumi/docs/ARCHITECTURE_DECISION_002_MARKET_DATA_PROVIDER.md) | ADR-002: Đánh giá Market Data Provider, thiết kế Provider Boundary Adapter |

## Gói Phát Hành (`docs/release/`)

| File | Mục đích |
|---|---|
| [RELEASE_NOTES_v3.0.0.md](file:///e:/Workspace/sumi/docs/release/RELEASE_NOTES_v3.0.0.md) | Ghi chú phát hành v3.0.0, ma trận nghiệm thu, giới hạn hệ thống |
| [ACCESSIBILITY_AND_KEYBOARD.md](file:///e:/Workspace/sumi/docs/release/ACCESSIBILITY_AND_KEYBOARD.md) | Đặc tả phím tắt và khả năng điều hướng bàn phím |
| [PLATFORMS_AND_PRIVACY.md](file:///e:/Workspace/sumi/docs/release/PLATFORMS_AND_PRIVACY.md) | Yêu cầu hệ thống và cam kết bảo mật 100% Local-first |
| [BACKUP_AND_RECOVERY.md](file:///e:/Workspace/sumi/docs/release/BACKUP_AND_RECOVERY.md) | Quy trình sao lưu, phục hồi cơ sở dữ liệu và xuất dữ liệu |

## Schema & Quyết Định Kỹ Thuật (`docs/decision-packs/`)

| File | Mục đích |
|---|---|
| [BATCH_0_DRAWING_PROVIDER_DECISION.md](file:///e:/Workspace/sumi/docs/decision-packs/BATCH_0_DRAWING_PROVIDER_DECISION.md) | Quyết định thiết kế drawing engine tự xây, TypeScript interface `DrawingProvider` |
| [sumi-drawing-document-v1.schema.json](file:///e:/Workspace/sumi/docs/decision-packs/sumi-drawing-document-v1.schema.json) | JSON Schema chuẩn hóa cho đối tượng vẽ — đang được kiểm tra trong test suite |

## Nghiên Cứu (`docs/research/`)

Thư mục nghiên cứu chứa các tài liệu đặc tả kỹ thuật chuyên sâu và hồ sơ thiết kế tính năng mới.

## Bất Biến Kỹ Thuật

- **Không bao giờ** làm lộ nến tương lai — API replay chỉ trả dữ liệu đến `current_index`.
- **Không bao giờ** thay đổi `backend/sumi.db` trong test tự động — dùng database tạm.
- **Tính toán chỉ báo authoritative** từ backend `IndicatorEngine`.
- **100% Local-first** — không telemetry, không gửi dữ liệu ra bên ngoài.
