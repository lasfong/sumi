# NHIỆM VỤ: AUDIT TOÀN DIỆN TÍNH NĂNG SUMI V3 (PHASE 0b - TEAMWORK MODE)

Bạn là Trưởng nhóm Điều phối (Lead Orchestrator) cùng đội ngũ Subagents trong chế độ Teamwork của Antigravity.
Nhiệm vụ của nhóm là thực hiện **Kiểm thử Thực nghiệm Toàn diện (Phase 0b Audit)** cho dự án Sumi V3 trên dữ liệu lịch sử CafeF (2018–2026), xác định chính xác tính năng nào HOẠT ĐỘNG, tính năng nào LỖI, phát hiện lỗi ẩn (đặc biệt là lỗi lệch 1.000x đơn vị giá CafeF), lưu trữ bằng chứng screenshot và lập báo cáo chi tiết.

Kế hoạch chi tiết của đợt audit nằm tại: `docs/audit/p0b/PHASE_0B_AUDIT_PLAN.md`.

---

## 1. NGUYÊN TẮC BẤT KHẢ XÂM PHẠM (GUARDRAILS)
1. **KHÔNG SỬA CODE SẢN PHẨM**: Tuyệt đối không chỉnh sửa code trong `backend/` và `frontend/`. Đây là đợt audit đánh giá hiện trạng thực tế, không phải đợt fix bug.
2. **BẢO VỆ DATABASE GỐC**: Tuyệt đối không chạm vào `backend/sumi.db`. Mọi kiểm thử phải chạy trên môi trường cô lập do script khởi động tạo ra (`test-results/p0b/audit.db`, ports 18200 / 15300).
3. **TRUNG THỰC VÀ KHÁCH QUAN**: Bất kỳ lỗi crash, lỗi lệch contract API hay sai số tài chính đều phải được ghi nhận chính xác kèm log và ảnh chụp màn hình.

---

## 2. PHÂN CÔNG VAI TRÒ TEAMWORK (SUBAGENTS)

Hãy phân bổ hoặc kích hoạt các vai trò sau để xử lý song song và độc lập:

- **Lead Orchestrator (Điều phối chính)**:
  - Giám sát toàn bộ tiến trình, điều phối các subagents, quản lý khởi động/dừng môi trường cô lập, đối chiếu với `docs/audit/p0b/PHASE_0B_AUDIT_PLAN.md` và tổng hợp báo cáo cuối cùng.
- **Subagent 1: Data & Pipeline Specialist (Chuyên viên Dữ liệu)**:
  - Chịu trách nhiệm kiểm tra bộ dữ liệu CafeF Upto (`docs/CafeF.SolieuGD.Upto05102026`).
  - Chạy công cụ trích xuất dữ liệu: `python3 docs/audit/p0b/tools/prepare_fixture.py`.
  - Chạy công cụ nạp dữ liệu: `python3 docs/audit/p0b/tools/import_fixtures.py`.
  - Đánh giá và ghi nhận bằng chứng cho các ca: `DF-01` (bị từ chối do ngược thời gian), `DF-02` (nạp nến sạch INDEX, HSX, HNX, UPCOM), `DF-03` (tổng hợp nến tuần 1W), `DF-04` (chặn xung đột).
- **Subagent 2: Browser UAT & Automation Specialist (Chuyên viên Kiểm thử Giao diện)**:
  - Chạy kịch bản tự động hóa Playwright: `node docs/audit/p0b/tools/run_p0b_audit.mjs`.
  - Kiểm tra 10 ảnh chụp màn hình trong `test-results/p0b/screenshots/` (1440x1000).
  - Thu thập lỗi console, network request lỗi của các trang:
    - `GL-01`: Menu phẳng 11 mục, nút Settings ở sidebar là nút chết (dead button).
    - `RP-01`, `RP-02`, `RP-05`: Replay chart, tua nến, luật T+2, chặn lô lẻ.
    - `SC-01`: Bộ quét tín hiệu kỹ thuật Signal Scanner.
    - `ST-01`, `ST-02`, `ST-04`, `ST-05`: Giao diện Strategy Tester (đối đầu 3 chiến lược, lỗi submit Multi-Phase, ngõ cụt Rule Builder, crash màn hình trắng `toFixed` ở tab Technical Flow BB).
- **Subagent 3: Financial Math & Core Logic Auditor (Chuyên viên Toán Tài Chính & Logic Lõi)**:
  - Phân tích trực tiếp database kiểm thử `test-results/p0b/audit.db` (các bảng `orders`, `trades`, `positions`, `replay_sessions`).
  - Đánh giá ca `FIN-01`: Phân tích lỗi lệch 1.000 lần đơn vị giá CafeF (mua 1.000 cổ FPT giá 61.9 bị trừ 61 nghìn đồng thay vì 61 triệu đồng, ảnh hưởng tới tính toán lãi lỗ, vốn, thuế và phí).
  - Phân tích nguyên nhân gốc rễ (root cause) của crash `toFixed` (lệch DTO giữa backend và frontend).

---

## 3. QUY TRÌNH THỰC THI CHI TIẾT (RUNBOOK)

### Bước 1: Chuẩn bị Fixtures
Chạy script trích xuất dữ liệu mẫu từ CafeF Upto:
```bash
python3 docs/audit/p0b/tools/prepare_fixture.py
```
*Kết quả đầu ra*: Thư mục `test-results/p0b/data/` có đầy đủ các file clean và file test xung đột.

### Bước 2: Khởi động Môi trường Cô lập
Khởi chạy backend (cổng 18200) và frontend (cổng 15300) kết nối vào DB tạm `test-results/p0b/audit.db`:
```bash
bash docs/audit/p0b/tools/start_env.sh --fresh
```
*(Yêu cầu chạy ngoài sandbox nếu môi trường chặn bind localhost port. Đảm bảo in ra dòng `READY frontend=... backend=...`).*

### Bước 3: Nạp Dữ Liệu Kiểm Thử & Chạy Ingestion Tests
Chạy script nạp tự động qua API Sumi:
```bash
python3 docs/audit/p0b/tools/import_fixtures.py
```
*Ghi nhận kết quả tại*: `test-results/p0b/import_fixtures_result.json`.

### Bước 4: Chạy Kịch Bản Tự Động Hóa Browser Playwright
Chạy toàn bộ 10 ca kiểm thử trình duyệt:
```bash
node docs/audit/p0b/tools/run_p0b_audit.mjs
```
*Ghi nhận kết quả tại*: `test-results/p0b/audit_run_results.json` và ảnh chụp tại `test-results/p0b/screenshots/*.png`.

### Bước 5: Đánh Giá Toán Tài Chính & Lỗi Lõi (FIN-01, ST-05)
Chạy script kiểm tra database để trích xuất số liệu mua bán và số dư tài khoản:
```bash
python3 -c "
import sqlite3
con = sqlite3.connect('test-results/p0b/audit.db')
cur = con.cursor()
print('--- ORDERS ---')
for r in cur.execute('SELECT id, session_id, symbol, side, order_type, quantity, status FROM orders'): print(r)
print('--- POSITIONS ---')
for r in cur.execute('SELECT symbol, quantity, average_price, total_cost, realized_pnl FROM positions'): print(r)
print('--- SESSIONS ---')
for r in cur.execute('SELECT id, symbol, initial_cash, current_cash FROM replay_sessions'): print(r)
"
```
Ghi nhận chi tiết mức độ lệch vốn 1.000x.

### Bước 6: Dừng Môi Trường & Xác Thực Toàn Vẹn Database Gốc
Chạy script tắt môi trường kiểm thử:
```bash
bash docs/audit/p0b/tools/stop_env.sh
```
*Đảm bảo script in ra*: `sumi.db unchanged (<mã SHA-256>)`.

---

## 4. ĐẦU RA YÊU CẦU (DELIVERABLES)

Khi hoàn thành, nhóm phải tạo ra 2 tài liệu hoàn chỉnh:
1. **Bảng Ma Trận Tính Năng**: `test-results/p0b/feature_audit_matrix_p0b.csv` với các cột chuẩn:
   `id,group,page,feature,status_phase0,status_phase0b,classification,severity,root_cause,evidence`
2. **Báo Cáo Tổng Kết Chi Tiết**: `test-results/p0b/PHASE_0B_FINAL_AUDIT_REPORT.md` bao gồm:
   - Tổng quan tỷ lệ tính năng WORKS / BROKEN.
   - Phân tích Top 5 lỗi nghiêm trọng nhất (Lỗi đơn vị giá 1.000x CafeF, Crash toFixed ở Technical Flow BB, Chặn form submit ở Multi-Phase Batch, Ngõ cụt AST của Rule Builder, Hạn chế all-or-nothing của CafeF import).
   - Đánh giá kiến trúc UI/UX menu và đề xuất phân nhóm (Manual Test vs Auto Test).
   - Đường dẫn liên kết đến các ảnh chụp màn hình bằng chứng trong `test-results/p0b/screenshots/`.
