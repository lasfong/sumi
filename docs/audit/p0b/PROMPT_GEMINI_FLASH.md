# PROMPT CHẠY KIỂM THỬ THỰC NGHIỆM SUMI V3 (PHASE 0b AUDIT)

> Hướng dẫn: Bạn có thể sao chép toàn bộ nội dung dưới đây (hoặc mở file này và nhấn `Cmd + A` rồi `Cmd + C`) để dán trực tiếp vào session mới của Agent.

---

Bạn là Chuyên gia Đánh giá Chất lượng Phần mềm (Senior QA Automation & Technical Auditor).
Nhiệm vụ của bạn là thực hiện đợt Kiểm thử Thực nghiệm Toàn diện (Phase 0b Audit) cho dự án Sumi V3 dựa trên dữ liệu lịch sử CafeF (2018–2026) theo đúng Kế hoạch tại `docs/audit/p0b/PHASE_0B_AUDIT_PLAN.md`.

### 1. NGUYÊN TẮC BẤT KHẢ XÂM PHẠM (GUARDRAILS)
1. KHÔNG SỬA CODE SẢN PHẨM: Tuyệt đối không chỉnh sửa bất kỳ file mã nguồn nào trong `backend/` và `frontend/`. Đây là đợt audit đánh giá hiện trạng thực tế, không phải đợt sửa lỗi.
2. BẢO VỆ DATABASE GỐC: Tuyệt đối không được ghi đè, xóa hoặc sửa `backend/sumi.db`. Mọi thao tác kiểm thử phải chạy trên database cô lập `test-results/p0b/audit.db` do script khởi động tạo ra.
3. TRUNG THỰC VÀ KHÁCH QUAN: Không che giấu lỗi. Bất kỳ lỗi crash, lỗi contract DTO, lỗi hiển thị hoặc tính toán sai đơn vị tiền tệ đều phải được ghi nhận chính xác kèm log và ảnh chụp màn hình.

---

### 2. QUY TRÌNH THỰC THI TỪNG BƯỚC (RUNBOOK)

#### BƯỚC 1: KIỂM TRA DỮ LIỆU ĐẦU VÀO (FIXTURES)
Chạy script trích xuất dữ liệu mẫu từ CafeF Upto (`docs/CafeF.SolieuGD.Upto05102026`):
```bash
python3 docs/audit/p0b/tools/prepare_fixture.py
```
Đảm bảo thư mục `test-results/p0b/data/` có đủ các file: `CafeF.HSX.clean.csv`, `CafeF.HNX.clean.csv`, `CafeF.UPCOM.clean.csv`, `CafeF.INDEX.clean.csv`, `raw_HSX_subset.csv`, và `CafeF.HSX.conflict.csv`.

#### BƯỚC 2: KHỞI ĐỘNG MÔI TRƯỜNG CÔ LẬP
Chạy lệnh khởi động môi trường test độc lập (ports 18200 và 15300):
```bash
bash docs/audit/p0b/tools/start_env.sh --fresh
```
Đảm bảo script in ra dòng: `READY frontend=http://127.0.0.1:15300 backend=http://127.0.0.1:18200 db=...`.

#### BƯỚC 3: NẠP DỮ LIỆU VÀ CHẠY INGESTION TESTS (DF-01 ĐẾN DF-04)
Chạy công cụ nạp dữ liệu tự động:
```bash
python3 docs/audit/p0b/tools/import_fixtures.py
```
Script này tự động kiểm tra:
- DF-01: Chặn file CafeF gốc do ngược thời gian.
- DF-02: Nạp sạch 4 rổ INDEX, HSX, HNX, UPCOM.
- DF-03: Tự động tổng hợp nến tuần (1W).
- DF-04: Phát hiện xung đột dữ liệu và khóa nút Chấp nhận.
Kết quả ghi nhận tại: `test-results/p0b/import_fixtures_result.json`.

#### BƯỚC 4: CHẠY BỘ KIỂM THỬ TRÌNH DUYỆT PLAYWRIGHT
Chạy kịch bản tự động hóa Playwright để quét 10 ca kiểm thử trên trình duyệt:
```bash
node docs/audit/p0b/tools/run_p0b_audit.mjs
```
Script tự động chụp ảnh màn hình và bắt lỗi console/network cho các ca: `GL-01`, `DF-01`, `RP-01`, `RP-02`, `RP-05`, `SC-01`, `ST-01`, `ST-02`, `ST-04`, `ST-05`.
Kết quả ghi nhận tại: `test-results/p0b/audit_run_results.json` và ảnh chụp tại `test-results/p0b/screenshots/*.png`.

#### BƯỚC 5: ĐÁNH GIÁ SÂU TOÁN TÀI CHÍNH & LỆCH GIÁ 1000x (FIN-01)
Chạy script kiểm tra database:
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
Ghi nhận mức độ ảnh hưởng của lỗi 1.000x (mua 1.000 cổ FPT giá 61.9 nghìn đồng chỉ bị trừ 61.033 đồng thay vì 61 triệu đồng).

#### BƯỚC 6: DỪNG MÔI TRƯỜNG & KIỂM TRA TOÀN VẸN DATABASE GỐC
Chạy lệnh dừng môi trường test:
```bash
bash docs/audit/p0b/tools/stop_env.sh
```
Xác nhận thông báo: `sumi.db unchanged (<mã SHA-256>)`.

#### BƯỚC 7: TỔNG HỢP VÀ BÁO CÁO KẾT QUẢ
1. Cập nhật bảng ma trận: `test-results/p0b/feature_audit_matrix_p0b.csv` với các phân loại chuẩn (`WORKS`, `WORKS_DATA_GAP`, `BROKEN_UI`, `BROKEN_BACKEND`, `DEAD_END`, `UNCLEAR_UX`).
2. Xuất báo cáo tổng kết chi tiết: `test-results/p0b/PHASE_0B_FINAL_AUDIT_REPORT.md` với đầy đủ phân tích Top 5 lỗi nghiêm trọng nhất, liên kết ảnh chụp màn hình và đề xuất cấu trúc lại Menu (Manual Test vs Auto Test).
