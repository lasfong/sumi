# Kế Hoạch Đánh Giá Chi Tiết Tính Năng Sumi V3 (Phase 0b Audit Plan)

> **Mục tiêu**: Đánh giá thực nghiệm chi tiết từng trang, từng nút bấm, luồng nghiệp vụ trên trình duyệt và API với bộ dữ liệu lịch sử CafeF (2018–2026), phân loại chính xác tính năng nào HOẠT ĐỘNG, tính năng nào LỖI/CHƯA HOÀN THIỆN, phát hiện lỗi ẩn (đặc biệt là quy đổi đơn vị giá CafeF), ghi lại bằng chứng screenshot và log.  
> **Nguyên tắc**: Tuyệt đối không can thiệp code sản phẩm, không sửa `backend/sumi.db`. Sử dụng môi trường audit cô lập (`test-results/p0b/audit.db`, cổng 18200 / 15300).

---

## 1. Môi Trường & Dữ Liệu Kiểm Thử

### 1.1 Bộ dữ liệu kiểm thử (Fixtures)
Bộ dữ liệu đã được trích xuất sẵn từ `docs/CafeF.SolieuGD.Upto05102026` vào thư mục `test-results/p0b/data/`:
1. `raw_HSX_subset.csv`: 10 mã HOSE giữ nguyên thứ tự gốc của CafeF (mới nhất xếp trước) để kiểm tra cơ chế phân loại của bộ nạp.
2. `CafeF.HSX.clean.csv`: 10 mã HOSE (FPT, SSI, HPG, VCB, MWG, VNM, TCB, MBB, ACB, VCI) từ 2018–2026 đã xếp tăng dần thời gian và chuẩn hóa OHLC.
3. `CafeF.HNX.clean.csv`: 2 mã HNX (SHS, PVS).
4. `CafeF.UPCOM.clean.csv`: 2 mã UPCOM (ACV, VEA).
5. `CafeF.INDEX.clean.csv`: 2 chỉ số (VNINDEX, HNX-INDEX).
6. `CafeF.HSX.conflict.csv`: Biến thể chứa 1 dòng giá thay đổi của FPT để test xung đột dữ liệu.

### 1.2 Khởi động và dừng môi trường
- Khởi động môi trường cô lập:
  ```bash
  bash docs/audit/p0b/tools/start_env.sh --fresh
  ```
  *(Tự động tạo `test-results/p0b/audit.db` mới, chạy alembic upgrade head, khởi chạy backend port 18200 và frontend port 15300)*
- Dừng môi trường:
  ```bash
  bash docs/audit/p0b/tools/stop_env.sh
  ```

---

## 2. Ma Trận 7 Miền Đánh Giá (Test Matrix)

### Miền 1: Data Feeds & Quản Lý Dữ Liệu (Import Flow)
- **DF-01 (Độ chấp nhận file CafeF gốc)**: Upload `raw_HSX_subset.csv`.
  - *Kỳ vọng/Hiện thực*: Hệ thống báo lỗi ngược thứ tự thời gian (`out_of_order_count > 0`, `can_accept = False`). Ghi nhận đây là hạn chế UX (cần tự động đảo chiều file thay vì bắt người dùng tự sửa).
- **DF-02 (Import nạp chuẩn)**: Nạp lần lượt `CafeF.INDEX.clean.csv`, `CafeF.HSX.clean.csv`, `CafeF.HNX.clean.csv`, `CafeF.UPCOM.clean.csv`.
  - *Kiểm tra*: Preview hiển thị đúng số dòng `parsed`, 0 dòng `rejected`. Bấm `Accept` lưu thành công vào DB.
- **DF-03 (Tự động tổng hợp nến tuần 1W)**:
  - *Kiểm tra*: Bảng `candles` có nến 1W tương ứng cho các mã vừa nạp.
- **DF-04 (Kiểm duyệt xung đột & Rollback)**:
  - Nạp `CafeF.HSX.conflict.csv` -> Preview phải phát hiện `conflicting_count > 0` và khóa nút `Chấp nhận`.
  - Test Rollback một đợt nạp từ lịch sử -> Dữ liệu của đợt nạp đó phải biến mất khỏi bảng nến.

### Miền 2: Đơn Vị Giá & Toán Học Tài Chính (Financial Sizing Sanity)
- **FIN-01 (Khớp đơn vị Nghìn Đồng vs Đồng)**:
  - Giá FPT từ CafeF lưu là `61.9` (nghĩa là 61.900đ).
  - Mở Replay FPT, vào form Mua 100 cổ phiếu.
  - *Kiểm tra*: Tiền trừ khỏi tài khoản là `6.190.000đ` hay chỉ là `6.190đ`? Phí giao dịch (0.15%) và thuế (0.1%) tính trên cơ sở nào?
  - *Đánh giá*: Xác định mức độ ảnh hưởng của lỗi sai lệch đơn vị giá 1.000x trên toàn bộ hệ thống.

### Miền 3: Trading Lab (Replay & Luyện Tập Thủ Công)
- **RP-01 (Khởi tạo phiên)**:
  - Tạo phiên FPT (2023-01-01 đến 2024-12-31).
  - Tạo phiên chế độ Ẩn mã (`BLIND_SYMBOL`), Ẩn ngày (`BLIND_DATE`), Ngẫu nhiên (`RANDOM`).
- **RP-02 (Điều khiển tua nến)**:
  - Bấm Next bar (`ArrowRight`), Next 5 bars, Auto-play. Kiểm tra nến xuất hiện mượt mà, không bị toast "End of data".
- **RP-03 (Chỉ báo & Multi-pane)**:
  - Bật EMA (20, 50), MACD, RSI, Ichimoku. Kiểm tra các đường vẽ hiển thị đúng pane và giá trị cập nhật từng nến.
- **RP-04 (Hệ thống Vẽ Chart & Persistence)**:
  - Vẽ Trendline, Horizontal Line, Fibonacci, Hộp R:R.
  - Refresh trang F5 -> Các hình vẽ có được khôi phục nguyên vẹn không?
- **RP-05 (Vòng đời Đặt Lệnh & Quản trị Rủi ro)**:
  - Đặt lệnh BUY MARKET, BUY LIMIT.
  - Đặt Stop Loss & Take Profit.
  - Kiểm tra ép lô chẵn 100 cổ phiếu (nhập 150 phải chặn).
  - Kiểm tra thanh toán T+2 (vừa mua xong bấm SELL ngay phải bị chặn/vô hiệu hóa).
- **RP-06 (Scoreboard & Debrief Modal)**:
  - Đóng lệnh -> Kiểm tra Practice Scoreboard cập nhật PnL, Winrate, R-multiple.
  - Kết thúc phiên -> Mở Session Debrief Modal, gắn tag lỗi tâm lý, xem báo cáo tổng kết.

### Miền 4: Journal & Analytics (Nhật Ký & Phân Tích Phiên)
- **JA-01 (Ghi nhật ký & Checklist)**:
  - Vào page Journal, chọn session vừa giao dịch. Thêm ghi chú, đánh giá checklist trước lệnh.
  - Xuất file CSV / JSON.
- **JA-02 (Phân tích chỉ số Analytics)**:
  - Vào page Analytics -> Biểu đồ vốn (Equity Curve), Drawdown, phân phối lệnh theo setup/thời gian.

### Miền 5: Signal Scanner & Explanation
- **SC-01 (Quét tín hiệu lịch sử)**:
  - Vào page Signal Scanner, chọn chiến lược EMA Trend Following hoặc MACD RSI Momentum, quét trên rổ FPT, SSI, HPG giai đoạn 2023–2024.
  - *Kiểm tra*: Kết quả quét ra danh sách các điểm kích hoạt tín hiệu.
- **SC-02 (Liên kết Scanner -> Replay)**:
  - Bấm vào một kết quả quét -> Có tự động mở Replay đúng mã và đúng nến phát tín hiệu đó không?

### Miền 6: Strategy Tester (Kiểm Định Tự Động)
- **ST-01 (Tab Battle & Sweep với dữ liệu thật)**:
  - Mở Strategy Tester -> Chọn 3 chiến lược đối đầu (EMA, MACD, Ichimoku).
  - Đặt ngày: In-Sample `2023-01-02` đến `2023-12-29`, Out-of-Sample `2024-01-02` đến `2024-12-27`.
  - Chạy so sánh -> Kiểm tra bảng kết quả, biểu đồ so sánh vốn, tiêu chí xếp hạng (>5 lệnh).
  - Chạy quét tham số (Sweep) cho `ema_fast` -> Kiểm tra bảng phương án.
- **ST-02 (Tab Multi-Phase Batch)**:
  - Kiểm tra lỗi chặn form do ô nhập vốn (`step="10000000"`, `min="1000000"`).
  - Chạy API trực tiếp và chụp ảnh lỗi form UI để xác nhận defect.
- **ST-03 (Tab Signal Catalog)**:
  - Tra cứu 72 tín hiệu, lọc theo nhóm (VSA, Ichimoku, Divergence...).
- **ST-04 (Tab Rule Builder)**:
  - Soạn quy tắc kết hợp: `health__favorable == True and bb__direction_rising == True`.
  - Bấm sao chép YAML -> Xác nhận thiếu nút "Chạy Backtest ngay" (Đánh giá giới hạn Dead-end).
- **ST-05 (Tab Money Flow BB)**:
  - Chụp ảnh màn hình crash (`Cannot read properties of undefined (reading 'toFixed')`).
  - Ghi nhận chi tiết lệch contract DTO giữa Backend và Frontend.

### Miền 7: Điều Hướng Toàn Cục & Cài Đặt (Global UX)
- **GL-01 (Menu & Badge)**:
  - Kiểm tra danh sách menu phẳng, badge v2.0 vs release v3.0.0.
  - Bấm nút Settings -> Xác nhận không có hành động.

---

## 3. Thang Đánh Giá & Đầu Ra Báo Cáo

Mỗi test case phải được ghi nhận vào `test-results/p0b/evidence.json` và cập nhật file ma trận `feature_audit_matrix.csv` theo các cột:
- `Test_ID`: DF-01, FIN-01, RP-01...
- `Feature_Name`: Tên tính năng cụ thể.
- `Execution_Status`: PASS / FAIL / BLOCKED.
- `Classification`: WORKS / WORKS_DATA_GAP / BROKEN_UI / BROKEN_BACKEND / DEAD_END / DUPLICATE / UNCLEAR_UX.
- `Severity`: P0 (chặn tính năng cốt lõi) / P1 (lỗi nghiệp vụ lớn) / P2 (lỗi UX/hiển thị) / P3 (thẩm mỹ/nhỏ).
- `Defect_Description`: Mô tả ngắn gọn nguyên nhân gốc rễ.
- `Evidence_File`: Đường dẫn ảnh chụp màn hình hoặc file json payload.
