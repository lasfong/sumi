# Sumi - VN Replay Trading Lab

Đây là một local-first personal webapp để luyện phân tích kỹ thuật và ra quyết định giao dịch chứng khoán Việt Nam.

## Tính năng (First Slice)
- Import dữ liệu lịch sử từ CafeF.
- Xem biểu đồ nến chứng khoán Việt Nam.
- Replay từng cây nến trong quá khứ không rò rỉ dữ liệu tương lai.
- Ghi lại nhật ký quyết định giao dịch (Decision, Order, Trade).
- Phân tích hiệu suất cơ bản.

## Kiến trúc
- **Backend:** Python, FastAPI, SQLAlchemy, SQLite (local-first)
- **Frontend:** React, TypeScript, Vite, TradingView Lightweight Charts

## Cài đặt (Backend)
```bash
cd backend
python -m venv venv
source venv/Scripts/activate # Windows
pip install -r requirements.txt
uvicorn app.main:app --reload
```
