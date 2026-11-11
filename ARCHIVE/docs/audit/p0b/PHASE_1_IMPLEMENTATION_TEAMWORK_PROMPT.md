# NHIỆM VỤ: NÂNG CẤP & SỬA LỖI TOÀN DIỆN SUMI V3 (PHASE 1 - TEAMWORK MODE)

Bạn là Trưởng nhóm Điều phối Kỹ thuật (Lead Technical Orchestrator) cùng đội ngũ Subagents trong chế độ Teamwork của Antigravity.
Nhiệm vụ của nhóm là thực hiện **Toàn bộ Phase 1: Cải tổ, Sửa lỗi và Tái cấu trúc Sumi V3** dựa trên kết quả kiểm thử thực nghiệm tại `test-results/p0b/PHASE_0B_FINAL_AUDIT_REPORT.md` và ma trận `test-results/p0b/feature_audit_matrix_p0b.csv`.

---

## 1. NGUYÊN TẮC BẤT KHẢ XÂM PHẠM (GUARDRAILS)
1. **BẢO VỆ DATABASE GỐC**: Không làm biến đổi `backend/sumi.db`. Mọi kiểm thử tự động và UAT phải chạy trên database cô lập (`test-results/p0b/audit.db` hoặc database tạm).
2. **KỶ LUẬT VIẾT CODE ĐƠN TỔ CHỨC (SINGLE WRITER PER BATCH)**: Để tránh conflict code giữa các subagents, nhóm phải thực hiện tuần tự theo từng BATCH rõ ràng:
   - Batch 1: Backend Data Normalization & CafeF 1000x Sizing.
   - Batch 2: Strategy Tester Bugfixes (Flow BB crash, Batch form block, Rule Builder action).
   - Batch 3: Menu UI/UX Grouping (Manual vs Auto) & Global Settings.
   - Batch 4: Verification Gate (Unit tests, Build, Playwright UAT & Evidence).
3. **KHÔNG GIẢM NHẸ TIÊU CHÍ KIỂM THỬ**: Tất cả các lỗi P0 và P1 đã phát hiện phải được sửa tận gốc rễ và được chứng minh bằng test tự động + ảnh chụp màn hình trình duyệt 1440x1000.

---

## 2. PHÂN CÔNG VAI TRÒ TEAMWORK (SUBAGENTS)

Hãy phân bổ và điều phối các vai trò chuyên môn sau:

- **Lead Orchestrator (Điều phối chính)**:
  - Giám sát tiến độ từng Batch, kiểm soát git diff, điều phối luồng làm việc giữa các subagents, chạy kiểm thử tổng hợp cuối cùng và lập Báo cáo Bàn giao Phase 1.
- **Subagent 1: Data & Backend Specialist (Chuyên viên Backend & Dữ liệu)**:
  - Phụ trách **Batch 1**: Xử lý logic nạp CafeF và chuẩn hóa đơn vị giá 1.000x.
  - Sửa `backend/app/services/cafef_importer.py` và `backend/app/services/import_workflow_service.py`:
    + Tự động nhân 1.000 cho giá cổ phiếu (equities) từ CafeF để chuyển đổi sang VNĐ thực tế (ví dụ: FPT 61.9 -> 61.900đ). Giữ nguyên điểm số cho chỉ số thị trường (VNINDEX, HNX-INDEX, VN30...).
    + Tự động sắp xếp tăng dần theo thời gian (`sort_values(by=['symbol', 'timestamp'], ascending=[True, True])`) nếu file đầu vào bị đảo ngược thời gian (sửa lỗi DF-01).
    + Cho phép bỏ qua (skip với cảnh báo) các dòng dị biệt (ngày nghỉ cuối tuần, giá âm) thay vì khóa cứng cả file (sửa lỗi DF-02).
  - Bổ sung unit test trong `backend/app/tests/test_cafef_importer.py` để bảo vệ logic mới.
- **Subagent 2: Strategy Lab & Frontend Specialist (Chuyên viên Strategy Tester)**:
  - Phụ trách **Batch 2**: Sửa các lỗi cốt lõi trên Strategy Tester.
  - Sửa lỗi crash màn hình trắng `toFixed` (ST-05):
    + Trong `frontend/src/components/chart/TechnicalFlowBBViewer.tsx`: Đọc an toàn dữ liệu từ `latestPoint.horizons` (ví dụ `const activeData = latestPoint.horizons?.[activeHorizons[0]] || Object.values(latestPoint.horizons || {})[0];`), sử dụng optional chaining `activeData?.value?.toFixed(2) ?? 'N/A'`.
  - Sửa lỗi chặn submit form Multi-Phase Batch (ST-02):
    + Trong `frontend/src/components/strategy/MultiPhaseBatchPanel.tsx`: Sửa ô nhập vốn `batch-initial-cash` thành `step="1000000"` (hoặc bỏ thuộc tính `step` hạn chế) để vốn mặc định 100.000.000đ submit trơn tru mà không vi phạm HTML5 `stepMismatch`.
  - Hoàn thiện ngõ cụt Rule Builder (ST-04):
    + Trong `frontend/src/components/strategy/StrategyRuleBuilder.tsx`: Bổ sung nút bấm hành động "🚀 Chạy Kiểm Định Với Quy Tắc Này" điều hướng sang tab Battle hoặc khởi chạy kiểm định với biểu thức AST vừa tạo.
- **Subagent 3: UI/UX & Navigation Specialist (Chuyên viên Giao diện & Menu)**:
  - Phụ trách **Batch 3**: Tái cấu trúc Menu và hoàn thiện cài đặt toàn cục.
  - Tái cấu trúc thanh điều hướng trong `frontend/src/components/layout/Sidebar.tsx`:
    + Chia 11 mục menu phẳng thành 2 nhóm nghiệp vụ rõ ràng:
      * **NHÓM 1: LUYỆN TẬP THỦ CÔNG (MANUAL PRACTICE)**:
        - Trading Lab (`/replay` - Core)
        - Nhật Ký Giao Dịch (`/journal`)
        - Phân Tích Hiệu Suất (`/analytics`)
      * **NHÓM 2: KIỂM ĐỊNH TỰ ĐỘNG & NGHIÊN CỨU (AUTO TEST & LAB)**:
        - Strategy Tester (`/strategy-lab` - V3)
        - Bộ Quét Tín Hiệu (`/scanner`)
        - Thư Viện Tín Hiệu (Signal Catalog)
        - Soạn Quy Tắc (Rule Builder)
      * **TIỆN ÍCH HỆ THỐNG**:
        - Nạp Dữ Liệu (`/import`)
        - Cài Đặt (Sửa nút Settings chết: gắn modal/drawer hiển thị thông tin cấu hình hệ thống và phím tắt).
    + Sửa Badge phiên bản: Đổi từ hardcoded `v2.0` thành `v3.0.0`.
- **Subagent 4: Quality Assurance & Verification Specialist (Chuyên viên Kiểm định & Nghiệm thu)**:
  - Phụ trách **Batch 4**: Chạy toàn bộ cổng kiểm tra kỹ thuật và trình duyệt.
  - Chạy backend test: `.venv/bin/pytest backend/app/tests`.
  - Chạy frontend test, lint và build: `npm test`, `npm run lint`, `npm run build` trong `frontend/`.
  - Khởi động môi trường cô lập (`start_env.sh --fresh`), nạp dữ liệu CafeF thật và chạy bộ kiểm thử Playwright cập nhật để chụp lại toàn bộ 10 ảnh màn hình bằng chứng chứng minh mọi lỗi đã được giải quyết.

---

## 3. KẾ HOẠCH THỰC THI CHI TIẾT (RUNBOOK)

### BƯỚC 1: TRIỂN KHAI BATCH 1 (BACKEND & CAFEF 1000x SIZING)
1. Cập nhật `backend/app/services/cafef_importer.py`:
   - Chuẩn hóa giá: Nếu `symbol` không thuộc nhóm chỉ số (`INDEX`, `VNINDEX`, `HNX-INDEX`, `VN30`), nhân các cột `open`, `high`, `low`, `close` với `1000`.
   - Chuẩn hóa thứ tự thời gian: `df.sort_values(by=['symbol', 'timestamp'], ascending=[True, True], inplace=True)`.
   - Bỏ qua các dòng lỗi giá <= 0 hoặc ngày không hợp lệ.
2. Chạy test xác thực: `.venv/bin/pytest backend/app/tests/test_cafef_importer.py`.

### BƯỚC 2: TRIỂN KHAI BATCH 2 (STRATEGY TESTER FIXES)
1. Cập nhật `frontend/src/components/chart/TechnicalFlowBBViewer.tsx`:
   - Truy cập an toàn `latestPoint.horizons` để triệt tiêu lỗi `toFixed`.
2. Cập nhật `frontend/src/components/strategy/MultiPhaseBatchPanel.tsx`:
   - Sửa `step="1000000"` trên ô nhập vốn để form submit bình thường.
3. Cập nhật `frontend/src/components/strategy/StrategyRuleBuilder.tsx`:
   - Bổ sung nút hành động liên kết trực tiếp sang kiểm định chiến lược.
4. Chạy test xác thực frontend: `cd frontend && npm test`.

### BƯỚC 3: TRIỂN KHAI BATCH 3 (TÁI CẤU TRÚC MENU SIDEBAR & SETTINGS)
1. Cập nhật `frontend/src/components/layout/Sidebar.tsx` và `Sidebar.css`:
   - Bổ sung nhóm phân cấp: "MANUAL PRACTICE" và "AUTO TEST & LAB".
   - Kết nối sự kiện `onClick` cho nút Settings (mở Settings modal hoặc thông tin phiên bản).
   - Đổi version badge thành `v3.0.0`.
2. Kiểm tra giao diện hiển thị trên trình duyệt.

### BƯỚC 4: TRIỂN KHAI BATCH 4 (KIỂM ĐỊNH TOÀN DIỆN & BÀN GIAO)
1. Chạy toàn bộ test suites:
   - Backend: `.venv/bin/pytest backend/`
   - Frontend: `npm test && npm run build`
2. Khởi động môi trường cô lập: `bash docs/audit/p0b/tools/start_env.sh --fresh`.
3. Nạp dữ liệu CafeF: `python3 docs/audit/p0b/tools/import_fixtures.py`.
4. Chạy Playwright UAT: `node docs/audit/p0b/tools/run_p0b_audit.mjs`.
5. Dừng môi trường: `bash docs/audit/p0b/tools/stop_env.sh` (xác thực SHA-256 của `sumi.db`).
6. Cập nhật ma trận `feature_audit_matrix_p0b.csv` (chuyển trạng thái các lỗi từ FAIL thành PASS) và lập báo cáo `docs/audit/PHASE_1_COMPLETION_REPORT.md`.

---

## 4. TIÊU CHÍ HOÀN THÀNH (DEFINITION OF DONE)
1. 100% unit tests và build pass không có lỗi.
2. File CafeF gốc tự động được nạp thành công, giá cổ phiếu hiển thị bằng VNĐ thực tế (FPT ~61.900đ), lệnh Mua 100 cổ phiếu trừ đúng ~6.190.000đ vốn.
3. Tab Technical Flow BB hiển thị dữ liệu bình thường, không crash màn hình trắng.
4. Tab Multi-Phase Batch submit form thành công ngay lần bấm đầu tiên.
5. Menu sidebar được phân chia 2 nhóm rõ ràng, nút Settings hoạt động.
6. Ảnh chụp màn hình kiểm định mới được lưu đầy đủ vào `test-results/p1/screenshots/`.
7. `backend/sumi.db` gốc được kiểm chứng giữ nguyên vẹn.
