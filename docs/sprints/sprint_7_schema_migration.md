# Sprint 7 — Schema & Migration Cleanup

> **Mục tiêu:** Database quản lý chuyên nghiệp bằng Alembic. Exchange data đầy đủ. Docs cập nhật.  
> **Thời gian:** 2–3 ngày  
> **Phụ thuộc:** Sprint 5 + Sprint 6

---

## S7-T1: Ghi Nhận Quyết Định Schema

**Estimate:** 30 phút

### Mục tiêu

Ghi rõ vào `docs/DECISIONS.md` rằng MVP dùng SQLite theo PRODUCT_SPEC.

### File sửa

- `docs/DECISIONS.md`

### Nội dung thêm

```markdown
## Decision 7: Schema Scope for MVP
- **Decision**: MVP sử dụng SQLite schema theo `PRODUCT_SPEC.md` (tables: symbols, candles, replay_sessions, decisions, orders, executions, positions, trades, journal_entries, event_logs).
- **Reason**: Đây là hướng local-first đã chọn. PostgreSQL/TimescaleDB là scope tương lai (Phase F6 trong FUTURE_ROADMAP.md).
- **Implication**: Không thêm PostgreSQL-specific features (TimescaleDB hypertable, etc.) cho đến khi có quyết định mới. Schema phải tương thích SQLite.
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Decision ghi trong DECISIONS.md | Đọc file |
| Nội dung rõ ràng scope SQLite | Đọc file |

---

## S7-T2: Tạo Alembic Migration Đầu Tiên

**Estimate:** 2–3 giờ

### Vấn đề

`alembic/versions/` hiện trống. App dùng `create_all()` để tạo tables.

### Hướng dẫn từng bước

**Bước 1:** Kiểm tra alembic config:

```bash
cd backend
cat alembic.ini | grep sqlalchemy.url
```

Đảm bảo URL trỏ đúng vào database (SQLite path).

**Bước 2:** Kiểm tra `alembic/env.py` import `Base` đúng:

```python
# alembic/env.py
from app.models import Base
target_metadata = Base.metadata
```

> [!WARNING]
> Nếu `env.py` chưa import đúng Base, Alembic sẽ tạo migration rỗng.

**Bước 3:** Tạo migration:

```bash
cd backend
.\.venv\Scripts\activate
alembic revision --autogenerate -m "initial_schema"
```

**Bước 4:** Kiểm tra file migration vừa tạo trong `alembic/versions/`:
- Có CREATE TABLE cho tất cả models
- Có UNIQUE constraints
- Có indexes

```bash
# Xem file vừa tạo
cat alembic/versions/*_initial_schema.py
```

**Bước 5:** Test migration trên fresh database:

```bash
# Backup DB hiện tại
copy sumi.db sumi_backup.db

# Xóa DB để test fresh
del sumi.db

# Chạy migration
alembic upgrade head

# Verify tables được tạo
python -c "
import sqlite3
conn = sqlite3.connect('sumi.db')
cursor = conn.execute(\"SELECT name FROM sqlite_master WHERE type='table'\")
for row in cursor:
    print(row[0])
"
```

**Kết quả mong đợi:** Tất cả tables tồn tại (symbols, candles, replay_sessions, decisions, orders, executions, positions, trades, journal_entries, event_logs).

**Bước 6:** Khôi phục DB gốc:

```bash
del sumi.db
copy sumi_backup.db sumi.db
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Migration file tồn tại | `ls alembic/versions/` có file |
| Migration tạo đủ tables | Đọc file, có CREATE TABLE |
| `alembic upgrade head` chạy được | Exit code 0 |
| Fresh DB có đầy đủ tables | Python script verify |

### Sai lầm thường gặp

- ❌ Quên import models trong env.py → migration rỗng
- ❌ Xóa DB production trước khi backup → mất data
- ❌ Migration file có syntax error → cần kiểm tra kỹ

---

## S7-T3: Bỏ create_all Khỏi Production Startup

**Estimate:** 1 giờ

### File sửa

- `backend/app/main.py`

### Hướng dẫn

**Bước 1:** Tìm dòng `create_all`:

```python
# TRƯỚC (tìm và sửa):
Base.metadata.create_all(bind=engine)
```

**Bước 2:** Gắn điều kiện:

```python
# SAU:
import os

# Chỉ auto-create tables trong development mode
if os.getenv("AUTO_CREATE_TABLES", "false").lower() == "true":
    Base.metadata.create_all(bind=engine)
```

Hoặc đơn giản hơn cho local-first MVP:

```python
# backend/app/main.py
from app.config import settings

# Trong event startup:
@app.on_event("startup")
async def startup():
    if settings.DEBUG or settings.AUTO_CREATE_TABLES:
        Base.metadata.create_all(bind=engine)
```

**Bước 3:** Thêm setting trong config:

```python
# backend/app/config.py
class Settings:
    DEBUG: bool = True
    AUTO_CREATE_TABLES: bool = True  # Tạm giữ True cho local dev
```

**Bước 4:** Tạo script dev riêng (optional):

```python
# backend/scripts/init_db.py
"""Script để tạo tables thủ công khi cần."""
from app.db import engine
from app.models import Base

if __name__ == "__main__":
    Base.metadata.create_all(bind=engine)
    print("All tables created successfully.")
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| create_all có điều kiện | Đọc code |
| Default cho local dev vẫn hoạt động | App start OK |
| Có script init_db.py thay thế | File tồn tại |

---

## S7-T4: Populate Exchange Từ CafeF Filename

**Estimate:** 2–3 giờ

### Logic

CafeF files thường có tên chứa exchange:
```text
CafeF.HSX.01.01.2020-15.06.2026.csv  → HOSE
CafeF.HNX.01.01.2020-15.06.2026.csv  → HNX
CafeF.UPCOM.01.01.2020-15.06.2026.csv → UPCOM
```

### File sửa

- `backend/app/services/cafef_importer.py`

### Hướng dẫn

```python
def _detect_exchange_from_filename(filename: str) -> str:
    """
    Detect exchange from CafeF filename.
    
    CafeF.HSX... → HOSE
    CafeF.HNX... → HNX
    CafeF.UPCOM... → UPCOM
    """
    filename_upper = filename.upper()
    
    if "HSX" in filename_upper or "HOSE" in filename_upper:
        return "HOSE"
    elif "HNX" in filename_upper:
        return "HNX"
    elif "UPCOM" in filename_upper:
        return "UPCOM"
    else:
        return None  # Unknown


# Trong hàm import, khi tạo/cập nhật Symbol:
exchange = _detect_exchange_from_filename(filename)

symbol_record = db.query(Symbol).filter(Symbol.symbol == symbol_name).first()
if symbol_record:
    if exchange and not symbol_record.exchange:
        symbol_record.exchange = exchange
else:
    symbol_record = Symbol(
        symbol=symbol_name,
        exchange=exchange,
        # ...
    )
    db.add(symbol_record)
```

### Test

```python
def test_import_hsx_sets_exchange_hose():
    exchange = _detect_exchange_from_filename("CafeF.HSX.01.01.2020-15.06.2026.csv")
    assert exchange == "HOSE"

def test_import_hnx_sets_exchange_hnx():
    exchange = _detect_exchange_from_filename("CafeF.HNX.data.txt")
    assert exchange == "HNX"

def test_import_upcom_sets_exchange():
    exchange = _detect_exchange_from_filename("CafeF.UPCOM.zip")
    assert exchange == "UPCOM"

def test_unknown_filename():
    exchange = _detect_exchange_from_filename("random_data.csv")
    assert exchange is None
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Import HSX file → symbols.exchange = "HOSE" | Test pass |
| Import HNX file → "HNX" | Test pass |
| Import UPCOM file → "UPCOM" | Test pass |
| Unknown file → exchange None | Test pass |

---

## S7-T5: Cập Nhật README

**Estimate:** 1–2 giờ

### File sửa

- `README.md`

### Nội dung cần cập nhật

1. **Trạng thái hiện tại** — nêu rõ đang ở giai đoạn nào
2. **Cách chạy test** — commands cụ thể
3. **Giới hạn hiện tại** — liệt kê honest
4. **Cách import data** — steps rõ ràng
5. **Environment variables** — nếu có

### Template

```markdown
## Trạng thái hiện tại

Dự án đang trong giai đoạn hoàn thiện MVP core:
- ✅ Import CafeF CSV/TXT/ZIP
- ✅ Replay Engine (no-future-leak)
- ✅ Trade Lifecycle (Decision → Order → Execution → Position → Trade)
- ✅ T+2 settlement rule
- ✅ Fee/Tax calculation
- ✅ Basic Analytics
- 🟡 Limit Orders (mới implement)
- 🟡 Advanced Analytics (đang chuẩn hóa)
- ❌ Algorithmic Backtest Engine

## Cách chạy test

### Backend
\```bash
cd backend
.\.venv\Scripts\activate
python -m pytest -v app/tests
\```

### Frontend
\```bash
cd frontend
npm run build
\```

## Giới hạn hiện tại

- Database: SQLite (local-first, chưa hỗ trợ PostgreSQL)
- Chỉ hỗ trợ timeframe 1D
- Chưa có multi-user authentication
- Chưa có real-time market data
- Strategy backtest engine chưa hoàn thiện
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| README phản ánh đúng trạng thái | Đọc file |
| Có hướng dẫn chạy test | Đọc file |
| Có giới hạn hiện tại | Đọc file |
| Không tuyên bố quá mức | Đọc file |

---

## 🏁 Definition of Done — Sprint 7 Tổng Thể

| # | Tiêu chí | Cách kiểm tra |
|---|----------|---------------|
| 1 | Schema decision documented | Đọc DECISIONS.md |
| 2 | Alembic migration tồn tại | `ls alembic/versions/` |
| 3 | Fresh DB works via migration | `alembic upgrade head` test |
| 4 | create_all có điều kiện | Đọc main.py |
| 5 | Exchange populated from filename | Import test |
| 6 | README cập nhật | Đọc file |
| 7 | All backend tests pass | `pytest -q` exit 0 |
| 8 | Frontend build pass | `npm run build` exit 0 |
