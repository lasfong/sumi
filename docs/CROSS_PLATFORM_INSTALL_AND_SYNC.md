# Sumi Supercharts v3.0 — Hướng Dẫn Cài Đặt Đa Nền Tảng & Đồng Bộ Đa Máy (`macOS ↔ Windows PC`)

Sumi v3.0 áp dụng kiến trúc đường dẫn chuẩn đa nền tảng tương thích hoàn toàn với **Yumeko v3.0**, không phụ thuộc đường dẫn cục bộ trên máy phát triển và chạy sạch ngay trên máy cài mới (`Clean-Slate`).

---

## 1. Chuẩn Hóa Thư Mục Cấu Hình & Dữ Liệu

| Thành phần | macOS | Windows 10/11 (x64) | Linux | Biến môi trường ghi đè |
|---|---|---|---|---|
| **Cấu hình (`config`)** | `~/.config/sumi/` | `%APPDATA%\sumi\` | `~/.config/sumi/` | `SUMI_CONFIG_DIR` |
| **Kho dữ liệu (`data`)** | `~/.local/share/sumi/` | `%LOCALAPPDATA%\sumi\data\` | `~/.local/share/sumi/` | `SUMI_DATA_DIR` |
| **CSDL Thị trường (nến)** | `<data>/sumi_market.db` | `<data>\sumi_market.db` | `<data>/sumi_market.db` | `SUMI_MARKET_DB` |
| **CSDL Workspace** | `<data>/sumi_workspace.db` | `<data>\sumi_workspace.db` | `<data>/sumi_workspace.db` | `SUMI_WORKSPACE_DB` |
| **CSDL Yumeko (Chỉ Đọc)** | `~/.local/share/yumeko/yumeko-app.sqlite` | `%LOCALAPPDATA%\yumeko\data\yumeko-app.sqlite` | `~/.local/share/yumeko/yumeko-app.sqlite` | `YUMEKO_DATA_DIR` |

> **Di trú tự động không phá hủy (macOS `v2.2 -> v3.0`):**
> Khi khởi chạy lần đầu trên máy Mac đã có dữ liệu v2.2 tại `~/Library/Application Support/com.sumi.supercharts/`, Sumi tự động sao lưu trực tuyến bằng **SQLite Online Backup API** (giữ nguyên toàn bộ WAL frames và `958.282+` nến), kiểm tra `PRAGMA integrity_check = ok`, ghi biên nhận SHA-256 tại `~/.local/share/sumi/.migration_v3_receipt.json`, và **giữ nguyên bản gốc không xóa**.

---

## 2. Đóng Gói & Cài Đặt

### A. Trên macOS (Apple Silicon / Intel)
```bash
# Cài đặt trực tiếp vào ~/Applications/Sumi Supercharts.app và đăng ký sumi://
bash scripts/build-macos-app.sh

# Kèm đóng gói file .dmg phân phối trong dist-macos/
bash scripts/build-macos-app.sh --dmg
```

### B. Trên Windows 10/11 (x64 — Yêu cầu Rust MSVC, Node.js 20+, Edge WebView2)
```powershell
# Đóng gói bản Portable + ZIP + cài vào %LOCALAPPDATA%\Programs\Sumi + Shortcut + đăng ký sumi://
powershell -ExecutionPolicy Bypass -File .\scripts\build-windows-app.ps1 `
  -InstallToLocalAppData `
  -CreateDesktopShortcut `
  -RegisterUriScheme

# Tùy chọn: tạo thêm bộ cài NSIS / MSI
powershell -ExecutionPolicy Bypass -File .\scripts\build-windows-app.ps1 -BuildInstaller
```

---

## 3. Clean-Slate Bootstrap (Máy Mới Hoàn Toàn)

Trên máy Windows/Mac cài mới hoàn toàn chưa có `sumi_market.db`:
1. Mở **Sumi Supercharts** → nhấp biểu tượng **Kho Dữ Liệu** (Database) trên thanh Top Header.
2. Nhấn **Danh mục ~1.600** để quét danh sách cổ phiếu `HOSE / HNX / UPCOM` trực tiếp từ TradingView vào SQLite.
3. Nhấn **Đồng bộ EOD VN** để tự động tải nến ngày mới nhất (có thanh tiến trình trực tiếp).
4. *(Tùy chọn)* Nếu có thư mục CSV lịch sử dài hạn 10 năm của CafeF, nhập đường dẫn thư mục (VD: `D:\CafeFData`), giữ bật **Kèm RAW (D/S)** để tự động phát hiện sự kiện cổ tức (`D`) và chia tách (`S`), rồi nhấn **Nạp thư mục CafeF vào SQLite**.

---

## 4. Đồng Bộ Workspace Đa Máy (`Mac ↔ Windows PC`)

### Cách 1 — Đồng bộ 2 chiều qua Thư mục Đám mây (Khuyên dùng ngay)
Hỗ trợ mọi dịch vụ đồng bộ tệp sẵn có: **iCloud Drive, OneDrive, Google Drive, Dropbox, hoặc Syncthing**.
1. Trên **cả 2 máy** (Mac & Windows), mở biểu tượng **Kho Dữ Liệu → tab "Đồng Bộ Đa Máy"**.
2. Trỏ cùng một thư mục đồng bộ (VD trên Mac: `~/Library/Mobile Documents/com~apple~CloudDocs/SumiSync`, trên Windows: `C:\Users\<Ten>\iCloudDrive\SumiSync` hoặc `C:\Users\<Ten>\OneDrive\SumiSync`) rồi bấm **Lưu**.
3. Nhấn **Đồng bộ 2 chiều ngay (LWW + SHA-256)**:
   - Sumi kiểm chứng tính toàn vẹn `SHA-256` của tệp `sumi_workspace_latest.sumi.json`.
   - Tự động tạo bản sao lưu xoay vòng (`5` bản `.pre_sync.bak`) trước khi gộp.
   - Gộp thông minh **Last-Write-Wins (LWW) + Tombstones** cho toàn bộ Watchlists, Drawings, Layouts, Indicators, Templates, Alerts, Replay Sessions và Nhật ký giao dịch.
   - **An toàn tuyệt đối:** Kho dữ liệu nến lớn `sumi_market.db` và tệp bí mật cảnh báo (`alerts_secrets.json`) **không bao giờ** bị đưa vào gói đồng bộ.

### Cách 2 — Xuất / Nhập Thủ Công `.sumi.json`
- Bấm **Xuất `.sumi.json`** để tạo gói snapshot có ký SHA-256 trong `<SUMI_DATA_DIR>/exports/`.
- Chuyển file sang máy còn lại và dùng ô **Nhập** để gộp an toàn (vẫn tự động tạo `.pre_sync.bak`).

### Cách 3 — Hạ Tầng Doraemi OAuth 2.1 + PKCE (Sẵn Sàng Kích Hoạt)
Sumi v3.0 đã cài sẵn lớp `DoraemiSyncTransport` (RFC 7636 PKCE `S256`). Khi bạn muốn đồng bộ trực tiếp lên server Doraemi chung với Yumeko, chỉ cần chạy file migration đề xuất tại `docs/architecture/DORAEMI_MIGRATION_0006_PROPOSAL.sql` trên máy chủ Doraemi.
