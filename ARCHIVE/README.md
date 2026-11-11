# Sumi — Vietnam Technical Analysis & Strategy Lab (V3)

> **Phát triển bằng Antigravity:** bắt đầu tại [Autonomous DEV program](docs/dev-program/README.md). Một brief khởi chạy, checkpoint trên đĩa, implement–review–fix nội bộ; không chạy lại chuỗi prompt Phase 1.

Sumi là nền tảng phân tích kỹ thuật và kiểm định chiến lược chuyên sâu dành cho thị trường chứng khoán Việt Nam (dữ liệu CafeF từ 2010 đến 2026), hoạt động theo triết lý **Local-First** (100% dữ liệu lưu trữ và xử lý nội bộ trên máy người dùng, không phụ thuộc internet và không gửi dữ liệu ra bên ngoài).

---

## 🎯 2 Trụ Cột Cốt Lõi (V3 Core Pillars)

### 1. Trading Lab — Thực Hành Phân Tích Kỹ Thuật (PTKT) Thủ Công
- **Tua nến lịch sử Bar-by-Bar**: Dữ liệu nến được kiểm soát chặt chẽ từ backend, cam kết **tuyệt đối không lộ nến tương lai** (no future leak).
- **Position Tool & Live R:R**: Thiết lập lệnh PTKT trực quan (Entry, Stop Loss, Take Profit) với tính năng tính nhanh SL (-3%, -5%, -7%) và TP (1.5R, 2.0R, 2.5R, 3.0R) kèm tỷ lệ Risk : Reward hiển thị thời gian thực.
- **Đường giá trực quan trên Chart**: Tự động vẽ 3 đường đứt nét trên biểu đồ nến (Entry xanh dương, SL đỏ, TP xanh lục) kèm marker vị thế.
- **Tự động chốt lời / cắt lỗ (+R / -1R)**: Khi tua nến chạm TP hoặc SL, hệ thống tự động thanh lý vị thế và ghi nhận kết quả Win (+XR) hoặc Loss (-1.0R).
- **Practice Scoreboard**: Bảng điểm hiệu suất thời gian thực theo dõi: Tổng số lệnh, Tỷ lệ thắng (Win Rate %), Lợi nhuận ròng theo R (Net R), và Tỷ lệ R:R trung bình.
- **Reset Luyện tập 1-Click**: Nút làm mới bảng thống kê nhanh chóng để bắt đầu chuỗi setup mới mà không cần tạo lại phiên.
- **Giải phóng rào cản**: Đã loại bỏ hoàn toàn ràng buộc vốn ảo VND và quy tắc khóa T+2 để phục vụ tối đa cho việc luyện tập phản xạ đọc chart.

### 2. Strategy Tester — Kiểm Định Chiến Lược Tự Động (1-Click Battle)
- **1-Click Strategy Battle**: Trả lời câu hỏi *"Với cổ phiếu X, chiến lược chỉ báo nào phù hợp nhất và có tỷ lệ thắng cao nhất trong lịch sử?"*.
- **3 Chiến lược chỉ báo cốt lõi sẵn có**:
  1. `MACD + RSI Momentum`: MACD cắt lên Signal & RSI > 50.
  2. `Ichimoku Cloud Breakout`: Nến vượt mây Kumo & Tenkan > Kijun.
  3. `EMA Trend Following`: Giao cắt đường xu hướng EMA 20 và EMA 50.
- **Preset tiện lợi**: Gợi ý nhanh các mã cổ phiếu hàng đầu (`FPT`, `SSI`, `HPG`, `VCI`, `TCB`, `VNINDEX`) và các khung thời gian tiêu biểu (`2010–2026`, `5 Năm 2021–2026`, `3 Năm 2023–2026`, `2020–2022 Sóng lớn`).
- **Bảng so sánh đối đầu**: Thống kê chi tiết Tổng số lệnh, Win Rate %, Lợi nhuận ròng (% Net Return), Max Drawdown %, Star Rating (1–5 ⭐) và huy hiệu Khuyến nghị kỹ thuật.
- **Biểu đồ Multi-Strategy Equity Curve (%)**: Đồ thị SVG trực quan so sánh đường cong tăng trưởng lợi nhuận của tất cả chiến lược trên cùng một trục thời gian kèm crosshair tooltip.

---

## 💻 Khởi Động Nhanh (Quick Start)

### Cách 1: Khởi động 1-Click trên Windows (Khuyên dùng)
Chỉ cần nhấp đúp chuột vào file:
- **`start-sumi.bat`**: Tự động kích hoạt cả Backend (FastAPI cổng 8000) và Frontend (Vite cổng 5173), đồng thời tự động mở trình duyệt web.
- **`stop-sumi.bat`**: Dừng toàn bộ các tiến trình nền của Sumi một cách an toàn.

*(Hoặc sử dụng PowerShell: `.\scripts\start-sumi.ps1` và `.\scripts\stop-sumi.ps1`)*

### Cách 2: Khởi động Thủ công

**1. Backend (Python 3.12+):**
```powershell
cd backend
.\.venv\Scripts\python.exe -m uvicorn app.main:app --reload --port 8000
```
*(Nếu cần seed dữ liệu mẫu demo offline: `.\.venv\Scripts\python.exe scripts\seed_demo.py`)*

**2. Frontend (Node 20+):**
```powershell
cd frontend
npm install
npm run dev
```
Truy cập: `http://localhost:5173`

---

## 🛡️ Tiêu Chuẩn Nghiệm Thu & Kiểm Thử Toàn Diện (Verification Gates)

Dự án tuân thủ nghiêm ngặt quy chuẩn kỹ thuật `AGENTS.md`:

### 1. Kiểm thử kỹ thuật nhanh (Fast Gate)
```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\verify-v2.ps1
```
- **Backend Pytest**: 193/193 tests PASSED (0 failures).
- **Alembic Database Migration**: Head schema đồng bộ.
- **Frontend ESLint**: 0 errors, 0 warnings.
- **Frontend Vitest**: 31/31 test files, 193/193 tests PASSED (0 failures).
- **Frontend Production Build**: `tsc -b && vite build` hoàn tất sạch sẽ (<1 giây).

### 2. Kiểm thử tự động trên Browser thực tế (Playwright E2E UAT Gate)
```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\run-comprehensive-uat.ps1
```
- **Độ phân giải chuẩn**: `1440 × 1000`.
- **Phạm vi**: Bao phủ trọn vẹn 6 domain: Replay Engine, Trading Lab, Technical Indicators, Drawing Tools, Strategy Tester và System Modals.
- **Kết quả nghiệm thu**: **31/31 tests PASSED (100%)**, 0 lỗi Console/Runtime.
- **Bảo vệ dữ liệu**: Kiểm tra mã SHA-256 của `backend/sumi.db` trước và sau test đảm bảo database gốc hoàn toàn bất biến.

---

## 📂 Cấu Trúc Mã Nguồn (Directory Overview)

```
sumi/
├── backend/                  # FastAPI Application Core
│   ├── app/
│   │   ├── api/              # REST & WebSocket Endpoints
│   │   ├── domain/engine/    # IndicatorEngine & StrategyRuleEvaluator (Authoritative)
│   │   ├── models/           # SQLAlchemy Data Models
│   │   ├── services/         # ReplayService, TradeLifecycleService, BacktestService
│   │   └── tests/            # Pytest test suite (193 tests)
│   └── scripts/              # Data import & demo seed scripts
├── frontend/                 # React 19 + TypeScript + Vite Client
│   ├── src/
│   │   ├── components/chart/ # Lightweight Charts v4/v5 adapters, SeriesManager, DrawingToolbar
│   │   ├── components/replay/# Trading Lab, PracticeScoreboard, TradeControls, Modals
│   │   ├── components/strategy/# MultiStrategyEquityChart SVG component
│   │   ├── pages/            # ReplayPage, StrategyLabPage, ImportPage, AnalyticsPage
│   │   └── store/            # Zustand state stores
├── docs/                     # Canonical product documentation & ADRs
├── scripts/                  # Automated verification & launcher scripts
├── start-sumi.bat            # 1-Click Windows starter
└── stop-sumi.bat             # 1-Click Windows stopper
```

---

## 📄 Bản Quyền & Giấy Phép
Dự án được xây dựng và tối ưu hóa phục vụ cộng đồng nhà đầu tư và nhà giao dịch kỹ thuật theo tiêu chuẩn mã nguồn mở, độc lập và bảo vệ quyền riêng tư dữ liệu (Local-First).
