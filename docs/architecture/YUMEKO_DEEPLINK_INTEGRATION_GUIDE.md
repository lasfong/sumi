# Hướng Dẫn Tích Hợp Nút 1-Click *"Mở trên Sumi (`KT01`)"* từ Yumeko.app & `agy acp`

*(Theo Quyết định #2, repo `/Users/mizuhara/workspace/yumeko` được giữ **100% Read-Only** trong quá trình nâng cấp Sumi v3.0. Khi bạn sẵn sàng gắn nút mở trực tiếp từ `Yumeko.app`, chỉ cần thực hiện 3 bước ngắn dưới đây.)*

---

## 1. Deep-Link URI Scheme (`sumi://inspect`)

Sumi v3.0 đã đăng ký chuẩn URI `sumi://` trên cả **macOS** (`CFBundleURLTypes` trong `scripts/build-macos-app.sh`) và **Windows 10/11** (`HKCU\Software\Classes\sumi` trong `scripts/build-windows-app.ps1`) kèm cơ chế **Single-Instance Loopback IPC** (`<SUMI_DATA_DIR>/runtime.lock`, quyền `0600`, xác thực `X-Sumi-Nonce`):

```text
sumi://inspect?symbol=FPT&as_of=2026-11-26&tf=1D&tab=yumeko
```

| Tham số | Ý nghĩa | Ví dụ |
|---|---|---|
| `symbol` / `ticker` | Mã cổ phiếu (tự suy luận `HOSE/HNX/UPCOM` nếu không ghi sàn) | `FPT`, `HOSE:VCB`, `HNX:PVS` |
| `as_of` | Khóa ngày chốt Point-in-Time (ưu tiên chuỗi `dnse_real` + ghép lịch sử dài hạn minh bạch) | `2026-11-26` |
| `tf` | Khung thời gian mặc định khi mở biểu đồ | `1D`, `1W`, `4h`, `1h`, `15m` |
| `tab` | Mở thẳng tab tương ứng ở thanh bên phải | `yumeko`, `watchlist`, `alerts` |

**Bên phía `Yumeko.app` (khi bạn muốn thêm nút 1-click sau này):**
- Chỉ cần gọi `yumeko_types::open_url_in_system_browser(&format!("sumi://inspect?symbol={sym}&as_of={as_of}&tf=1D&tab=yumeko"))` từ một nút trong màn hình Báo cáo / Danh mục / Cơ hội.
- Nếu Sumi đang chạy, cửa sổ Sumi sẽ tự nổi lên trước và chuyển thẳng sang mã + ngày chốt `as_of` đó mà **không** mở trùng 2 cửa sổ.

---

## 2. Khai Báo `sumi-mcp` cho AI Agent (`agy acp` / Claude / Codex)

Sumi cung cấp máy chủ MCP JSON-RPC 2.0 chuẩn `protocolVersion = "2025-03-26"` qua cả binary nhẹ `sumi mcp` lẫn `sumi-desktop --mcp-serve`:

```json
{
  "mcpServers": {
    "sumi": {
      "command": "sumi",
      "args": ["mcp"]
    }
  }
}
```

5 công cụ MCP khả dụng:
1. `sumi_kt01_analyze_symbol` — Chạy đầy đủ cổng xác nhận `KT01` (`1W/1D/1H`, QMV Blackbox thật vs OHLCV Proxy, `SL/TP1/TP2` chuẩn hóa `VND/kVND`).
2. `sumi_mtf_screen` — Quét đa khung thời gian (`1W + 1D`) trên 123 tín hiệu QMV.
3. `sumi_run_backtest` — Kiểm định định lượng chiến lược QMV (`qmv_s1..qmv_s6`, `sma_ema`, `rsi_mean_rev`, `macd`, ...) theo luật `T+2` & trần/sàn VN.
4. `sumi_market_breadth_rrg` — Độ rộng thị trường & tọa độ xoay vòng 4 góc phần tư `RRG` theo ngày `as_of`.
5. `sumi_export_kt01_verdict` — Xuất gói `KT01GateVerdictV1` + `TechnicalEvidenceBundleV1` có ký `SHA-256` (`advisory_only = true`).

---

## 3. Đường Dẫn Trao Đổi Bằng Chứng (`Sumi -> Yumeko`)

Mỗi lần bấm **"Lưu Luận Điểm KT01"** trên Sumi (hoặc gọi tool MCP `sumi_export_kt01_verdict`), Sumi tự động ghi bộ tệp bất biến (`.json` + `.md`) tại:
- Kho gốc của Sumi: `<SUMI_DATA_DIR>/kt01_verdicts/<as_of>/<SYM>_<sha8>.json`
- Dropzone tự chọn cho Yumeko *(tuyệt đối không ghi vào `yumeko-app.sqlite`)*: `<YUMEKO_DATA_DIR>/sumi_evidence/<as_of>/<SYM>_<sha8>.json`
