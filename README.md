# VN Replay Trading Lab (Sumi)

Sumi là một Web-app Local-first được thiết kế dành riêng cho Nhà đầu tư cá nhân tại Việt Nam. Nó cho phép bạn nạp dữ liệu lịch sử từ CafeF và luyện tập ra quyết định giao dịch (Mua/Bán) trên biểu đồ nến mà không sợ lộ trước tương lai (No Future Leak). Quan trọng hơn, Sumi tích hợp sẵn một hệ thống **Trading Journal** (Nhật ký giao dịch) và **Analytics** (Phân tích hiệu suất) giúp bạn hiểu rõ bản thân đang sai ở đâu và setup nào đem lại lợi nhuận cao nhất.

## Tính năng nổi bật

1. **Replay Engine không gian lận**: Dữ liệu tương lai bị cắt đứt hoàn toàn ở tầng Backend, đảm bảo bạn không thể "nhìn trộm" nến ngày mai.
2. **Trading Journal Tích hợp**: Mỗi quyết định Buy/Sell đều yêu cầu (tuỳ chọn) bạn nhập Lý do, Loại Setup (Breakout, Pullback...) và Độ tự tin (1-5 sao).
3. **Analytics Dashboard**: Tự động tính toán Win Rate, Profit Factor, R-Multiple, Expectancy, Average Win/Loss.
4. **Phím tắt nhanh (Shortcuts)**: Giao dịch không cần dùng chuột (`Space` để Next, `B` để Buy, `S` để Sell, `C` để Close).
5. **Giao diện Dark Mode**: Tích hợp TradingView Lightweight Charts tối ưu và đẹp mắt.

## Hướng dẫn cài đặt (Installation)

Sumi chạy hoàn toàn trên máy tính cá nhân của bạn (Local-first). Database sử dụng SQLite nên không cần cài đặt thêm CSDL phức tạp.

### Yêu cầu
- Python 3.10+
- Node.js 18+

### 1. Cài đặt Backend
Mở Terminal / Command Prompt và chạy các lệnh sau:

```bash
cd backend
python -m venv venv

# Windows
.\venv\Scripts\activate
# Mac/Linux
source venv/bin/activate

pip install -r requirements.txt
```

Khởi chạy Server Backend:
```bash
uvicorn app.main:app --reload --port 8000
```

### 2. Cài đặt Frontend
Mở một Terminal mới:

```bash
cd frontend
npm install
```

Khởi chạy Server Frontend:
```bash
npm run dev
```

Mở trình duyệt tại địa chỉ `http://localhost:5173` để sử dụng.

---

## Hướng dẫn sử dụng

### Bước 1: Nạp dữ liệu (Import Data)
1. Tải dữ liệu lịch sử chứng khoán (File CSV nến) từ [CafeF](https://cafef.vn/du-lieu/du-lieu-download.chn).
2. Xả nén thư mục tải về (Ví dụ: `CafeF.SolieuGD...`) vào thư mục `data/raw/cafef_sample/` của dự án.
3. Chạy lệnh siêu tốc để nạp dữ liệu vào Database:
```bash
cd backend
python scripts/import_batch.py ../data/raw/cafef_sample
```

### Bước 2: Bắt đầu luyện tập
1. Vào mục **Replay Lab** trên giao diện web.
2. Gõ mã chứng khoán (Ví dụ: `VNINDEX`, `FPT`) và bấm `Start Session`.
3. Bấm `Space` (Phím cách) để tiến từng ngày.
4. Khi thấy cơ hội, bấm `B` (Buy) hoặc `S` (Sell).
5. Một hộp thoại sẽ hiện ra yêu cầu bạn điền **Lý do (Reason)** và chấm điểm **Độ tự tin**. Bấm Confirm để khớp lệnh.
6. Khi muốn chốt lời/cắt lỗ, bấm `C` (Close).

### Bước 3: Đánh giá bản thân
1. Chuyển sang mục **Analytics**.
2. Nhập Session ID (Số ID sẽ hiện trên thanh tiêu đề lúc bạn Replay) và xem các báo cáo tổng quan về năng lực giao dịch của bạn trong phiên đó.

---

## Kiến trúc công nghệ (Tech Stack)
- **Backend:** FastAPI, SQLAlchemy, SQLite, Pandas (Fast calculation)
- **Frontend:** React, TypeScript, Vite, TanStack Query, Zustand, TradingView Lightweight Charts

*Dự án tuân thủ nghiêm ngặt nguyên tắc Domain-Driven Design (DDD) tách biệt hoàn toàn Decision, Order, Execution, Position và Trade.*
