# Sprint 0 — Environment Setup

> **Mục tiêu:** Có môi trường dev chạy được test/build trên máy local.  
> **Thời gian:** 0.5–1 ngày  
> **Phụ thuộc:** Không  
> **Người thực hiện:** Mọi dev trong team  

> [!IMPORTANT]
> Sprint này là bắt buộc trước khi làm bất kỳ sprint nào khác. Nếu môi trường không chạy được, mọi task sau đều vô nghĩa.

---

## S0-T1: Kiểm tra Runtime

**Mục tiêu:** Xác nhận máy đã cài đầy đủ Node.js và Python với phiên bản đúng.

**Tại sao cần làm:** Dự án yêu cầu Node.js 18+ cho frontend (React/Vite) và Python 3.10+ cho backend (FastAPI/SQLAlchemy). Nếu thiếu hoặc sai version, mọi bước sau sẽ fail.

### Hướng dẫn từng bước

**Bước 1:** Mở Terminal (PowerShell hoặc CMD) và chạy:

```bash
node -v
```

**Kết quả mong đợi:** `v18.x.x` hoặc `v20.x.x` hoặc cao hơn.

**Bước 2:** Kiểm tra npm:

```bash
npm -v
```

**Kết quả mong đợi:** `9.x.x` hoặc `10.x.x` hoặc cao hơn.

**Bước 3:** Kiểm tra Python:

```bash
python --version
```

**Kết quả mong đợi:** `Python 3.10.x` hoặc `3.11.x` hoặc `3.12.x`.

> [!TIP]
> Trên Windows, nếu `python` không chạy, thử:
> ```bash
> py --version
> ```
> Nếu `py` chạy được thì dùng `py` thay cho `python` ở tất cả lệnh sau.

**Bước 4:** Nếu thiếu Node.js → tải tại https://nodejs.org (chọn LTS).  
Nếu thiếu Python → tải tại https://python.org (nhớ tick "Add Python to PATH" khi cài).

### Definition of Done (DOD)

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Node.js ≥ 18 | `node -v` hiện v18+ |
| npm chạy được | `npm -v` hiện version |
| Python ≥ 3.10 | `python --version` hoặc `py --version` hiện 3.10+ |

**Estimate:** 15–30 phút (nếu cần cài thêm: 1 giờ)

### Lưu ý cho dev junior

- **KHÔNG** cài Python từ Microsoft Store — nó có thể gây xung đột PATH.
- Sau khi cài, **đóng rồi mở lại Terminal** để PATH cập nhật.
- Nếu có nhiều version Python, dùng `py -3.11` để chỉ định version cụ thể.

---

## S0-T2: Tạo Backend Virtual Environment

**Mục tiêu:** Tạo môi trường Python ảo (venv) riêng cho dự án, cài đặt tất cả dependencies.

**Tại sao cần làm:** Venv giúp cách ly các thư viện Python của dự án này khỏi các dự án khác trên máy. Không dùng venv = tự rước rắc rối version conflict.

### Hướng dẫn từng bước

**Bước 1:** Mở Terminal, di chuyển vào thư mục backend:

```bash
cd E:\Workspace\sumi\backend
```

**Bước 2:** Tạo virtual environment:

```bash
python -m venv .venv
```

> [!TIP]
> Lệnh này tạo thư mục `.venv` chứa Python riêng cho dự án. Nếu dùng `py` thì: `py -m venv .venv`

**Bước 3:** Kích hoạt venv:

```bash
# Windows PowerShell:
.\.venv\Scripts\Activate.ps1

# Windows CMD:
.\.venv\Scripts\activate.bat
```

**Kết quả mong đợi:** Prompt hiện `(.venv)` ở đầu dòng, ví dụ:
```
(.venv) PS E:\Workspace\sumi\backend>
```

> [!WARNING]
> Nếu PowerShell báo lỗi "execution of scripts is disabled", chạy lệnh sau rồi thử lại:
> ```bash
> Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
> ```

**Bước 4:** Upgrade pip:

```bash
python -m pip install --upgrade pip
```

**Bước 5:** Cài đặt dependencies:

```bash
python -m pip install -r requirements.txt
```

**Bước 6:** Kiểm tra cài đặt thành công:

```bash
python -m pytest --version
```

**Kết quả mong đợi:** `pytest 8.x.x` hoặc tương tự.

```bash
python -c "import fastapi, sqlalchemy, pandas; print('OK - All imports successful')"
```

**Kết quả mong đợi:** `OK - All imports successful`

### Definition of Done (DOD)

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Venv tạo thành công | Thư mục `.venv` tồn tại trong `backend/` |
| Venv activated | Prompt hiện `(.venv)` |
| Dependencies cài xong | `pip install -r requirements.txt` không lỗi |
| pytest chạy được | `python -m pytest --version` hiện version |
| Core imports OK | `python -c "import fastapi, sqlalchemy, pandas; print('OK')"` không lỗi |

**Estimate:** 15–30 phút

### Lưu ý cho dev junior

- **LUÔN** activate venv trước khi làm việc với backend. Nếu quên, các lệnh `pytest`, `uvicorn` sẽ dùng Python hệ thống và có thể fail.
- Kiểm tra bằng cách chạy `where python` — nó phải trỏ vào `.venv\Scripts\python.exe`, không phải `C:\Python311\python.exe`.
- **KHÔNG** commit thư mục `.venv` lên git (đã có trong `.gitignore`).

---

## S0-T3: Cài Frontend Dependencies

**Mục tiêu:** Cài đặt node_modules cho frontend và ghi lại lỗi build hiện tại.

**Tại sao cần làm:** Cần biết chính xác frontend đang fail ở đâu trước khi sửa (Sprint 1).

### Hướng dẫn từng bước

**Bước 1:** Mở Terminal mới, di chuyển vào frontend:

```bash
cd E:\Workspace\sumi\frontend
```

**Bước 2:** Cài dependencies:

```bash
npm install
```

**Kết quả mong đợi:** Cài xong, có thể có warnings nhưng không error.

**Bước 3:** Thử build để ghi lại lỗi hiện tại:

```bash
npm run build
```

**Kết quả mong đợi:** Build sẽ FAIL. **Đây là expected.** Việc cần làm là copy toàn bộ output lỗi và lưu lại.

**Bước 4:** Lưu output lỗi vào file để tham khảo:

```bash
npm run build 2>&1 | Out-File -FilePath build_errors.txt
```

> [!IMPORTANT]
> Đọc file `build_errors.txt` và ghi chú lại:
> - Tổng số lỗi TypeScript
> - Danh sách file nào bị lỗi
> - Loại lỗi (type mismatch, missing prop, unused import, etc.)

### Definition of Done (DOD)

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| npm install thành công | Thư mục `node_modules` tồn tại |
| Đã chạy build | Có output từ `npm run build` |
| Lỗi được ghi lại | Có file/note liệt kê lỗi hiện tại |

**Estimate:** 15–30 phút

### Lưu ý cho dev junior

- `npm install` có thể mất vài phút tùy mạng.
- Nếu bị lỗi `ERESOLVE`, thử `npm install --legacy-peer-deps`.
- **KHÔNG** sửa bất kỳ lỗi nào trong bước này. Chỉ ghi lại. Sửa ở Sprint 1.

---

## S0-T4: Kiểm Tra Trạng Thái Git

**Mục tiêu:** Hiểu rõ trạng thái hiện tại của repository.

**Tại sao cần làm:** Cần biết đang ở branch nào, có file nào chưa commit, commit gần nhất là gì, trước khi bắt đầu sửa code.

### Hướng dẫn từng bước

**Bước 1:** Ở thư mục gốc dự án:

```bash
cd E:\Workspace\sumi
```

**Bước 2:** Kiểm tra trạng thái:

```bash
git status --short
```

**Ghi lại:** File nào modified (`M`), file nào untracked (`??`), file nào staged (`A`).

**Bước 3:** Xem 5 commit gần nhất:

```bash
git log --oneline -5
```

**Bước 4:** Kiểm tra branch hiện tại:

```bash
git branch
```

**Bước 5:** Nếu có file dirty mà không phải của mình, **KHÔNG** sửa. Báo lại cho team lead.

### Definition of Done (DOD)

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Biết branch hiện tại | `git branch` đã chạy |
| Biết file dirty | `git status --short` đã chạy |
| Biết commit history | `git log --oneline -5` đã chạy |

**Estimate:** 10 phút

### Lưu ý cho dev junior

- **KHÔNG** chạy `git clean` hay `git reset --hard` nếu không biết rõ đang làm gì.
- Nếu thấy file `backend/sumi.db` hiện trong git status → đó là runtime artifact, không nên commit.
- Nếu thấy `node_modules` hoặc `__pycache__` → kiểm tra `.gitignore` đã bao gồm chưa.

---

## 🏁 Definition of Done — Sprint 0 Tổng Thể

| # | Tiêu chí | Lệnh kiểm tra | Kết quả |
|---|----------|---------------|---------|
| 1 | Node.js ≥ 18 | `node -v` | v18+ |
| 2 | Python ≥ 3.10 | `python --version` | 3.10+ |
| 3 | Backend venv hoạt động | `python -m pytest --version` (trong venv) | Hiện version |
| 4 | Backend imports OK | `python -c "import fastapi, sqlalchemy, pandas; print('OK')"` | OK |
| 5 | Frontend deps cài xong | `ls frontend/node_modules` có file | Có thư mục |
| 6 | Build errors ghi lại | Có note/file liệt kê lỗi | Có |
| 7 | Git state rõ ràng | `git status --short` đã chạy | Có output |

> [!CAUTION]
> **Chỉ chuyển sang Sprint 1 khi TẤT CẢ 7 tiêu chí trên đều đạt.**
