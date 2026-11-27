# THIẾT KẾ BỘ CHỈ BÁO KỸ THUẬT ĐỘC QUYỀN THEO PHƯƠNG PHÁP QMV — CUSTOM INDICATORS SPECIFICATION
**(`03_QMV_CUSTOM_INDICATORS_SPECIFICATION.md`)**

> **Mục tiêu Thiết kế Kiến trúc:**
> Các nền tảng biểu đồ đại trà (như TradingView mặc định hay KLineChart nguyên bản) chỉ cung cấp các chỉ báo sách giáo khoa đơn lẻ (`SMA`, `RSI`, `MACD`, `BB`, `Ichimoku` chuẩn Nhật) hoàn toàn **thiếu vắng những kỹ thuật thực chiến cốt lõi nhất của hệ thống QMV** như: *Ghép nến đa phiên theo quy tắc `1/3` & `50%`, nhận diện nến Trần/Sàn cứng Việt Nam, VPA theo `MA(5)` gắn với "Tiết cung nổ vol = 100% xả", Vùng giao dịch gọt râu hoảng loạn + đếm `3 Điểm Chạm`, Bối cảnh lớn `0–8` & 5 pha `Cáo–Thỏ`, tự động chuyển chế độ Xu hướng vs Dao động, tái tạo Hộp đen `TMC / DSPI / MPICP / DMZ03` có chống bẫy chuẩn hóa 50 phiên, và Ma trận luân chuyển `9 Quả Bóng` cảnh báo "Đổi gác"*.
>
> Tài liệu này đặc tả **07 Bộ Chỉ báo Độc quyền QMV (`Custom QMV Indicators`)**, đảm bảo **100% Nhân quả (`Causal` tại nến đóng cửa `t`)**, sẵn sàng để cài đặt hiệu năng cao trong `crates/sumi-core` (Rust) và hiển thị trực quan trên `KLineChart` (TypeScript Canvas) của workstation **Sumi**.

---

## TỔNG QUAN DANH MỤC 07 CUSTOM QMV INDICATORS

| Mã Định Danh (`ID`) | Tên Chỉ Báo Độc Quyền QMV | Vị Trí Hiển Thị (`KLineChart`) | Nguồn Dữ Liệu Đầu Vào | Bài Giảng QMV Gốc Làm Cơ Sở |
| :--- | :--- | :---: | :--- | :--- |
| **`QMV_MERGED_VPA`** | Ghép Nến Đa Phiên Thích Ứng & Thanh Áp Lực Cung–Cầu QMV | `Overlay` trên Nến + `Sub-pane` Volume | `OHLCV` đơn mã (`1D` / `30m` / `15m`) | `HP4 v1_4, v2_2, v3_1, v3_2`, `KT01T1, KT01T3`, `wa_10_02_2026` |
| **`QMV_ZONE_3TOUCH`** | Vùng Giao Dịch Mua/Bán, `3 Điểm Chạm`, Hộp 3 Độ Nghiêng & Thước `RR` | `Overlay` trên Nến | `OHLCV` đơn mã (`1D` / `1H` / `30m`) | `KT01T2, KT01T3`, `DM01T4`, `HP4 v1_2, v2_4, v3_4, v3_5` |
| **`QMV_SMART_MONEY_CYCLE`** | Radar 5 Pha Trò Chơi `Cáo – Thỏ` & Định Vị Bối Cảnh Lớn `0 – 8` | `Overlay` nhãn sóng + `Sub-pane` băng 5 pha | `OHLCV` đơn mã + `Weekly EMA200` | `HP1 v1..v3`, `KT01T1, KT01T2, KT01T4`, `HP4 v3_5`, `wa_10_02_2026` |
| **`QMV_REGIME_RIBBON_DIV`** | Dải Chuyển Chế Độ Xu Hướng–Dao Động & Máy Quét 4 Phân Kỳ Nhân Quả | `Overlay` (`BB15 + Ichi QMV`) + `Sub-pane` (`RSI/CCI/MACD`) | `OHLCV` đơn mã | `HP4 v1_3, v2_3, v3_3`, `KT01T3`, `V7 Guide`, `HP5 v3_4` |
| **`QMV_BLACKBOX_PROXY`** | Bộ Tái Tạo Hộp Đen Cổ Phiếu (`TMC/BB-Status`, `DSPI`, `MPICP`, `DMZ03..20`, `0–5★`) | `Multi Sub-pane` (3 ngăn đồng bộ Blackbox V7) | `OHLCV` đơn mã (+ `VNINDEX`) | `V5/V7 Guide`, `Chọn CP Fintech 17/07/2025`, `Fintech Buổi 1 & 2` |
| **`QMV_MARKET_BREADTH_BB`** | Hộp Đen Độ Rộng Thị Trường / Ngành (`MBZ`, `BBZ-M/S/EM` & `Sentiment`) | `Sub-pane` Vĩ mô / Dashboard | Cross-sectional 400+ mã thanh khoản | `V5/V7 Guide`, `Fintech Buổi 2`, `Yumeko Postmortem Regime Journal` |
| **`QMV_9BALL_ROTATION`** | Radar Ma Trận Luân Chuyển `9 Quả Bóng` (`NN1..3 × DT1..3`) & Cảnh Báo Đổi Gác | `Sub-pane` Heatmap / Dock Widget + Cảnh báo | Cross-sectional 400+ mã + Map Ngành ICB | `HP5 v1_3, v3_3, v3_4`, `KT01T4`, `DM01T2`, `VM01T3` |

---

## INDICATOR 1: `QMV_MERGED_VPA` — GHÉP NẾN ĐA PHIÊN THÍCH ỨNG & THANH ÁP LỰC CUNG–CẦU QMV

### 1.1. Cơ sở Bài giảng QMV Gốc
- **Nguồn trích dẫn:** `HP4 v2_2_2_kythuatghepnen_audio.mp3.md`, `v3_2_bg + hoidap + thaoluan`, `KT01T1`, `KT01T3`, `HP4 v1_5, v3_1_bg`, `wa_10_02_2026_ky_thuat_bat_day_an_toan_audio.mp3.md`.
- **Vấn đề giải quyết:** Phần mềm thông thường nhận diện mẫu nến theo tên cứng nhắc, đọc nến tràn lan ở cả vùng lưng chừng vô nghĩa, nhận nhầm vạch ngang **Trần cứng / Sàn cứng (`O = H = L = C`)** trên sàn VN thành nến `Doji` lưỡng lự, bị lừa bởi trò khớp 100 cổ giá sàn đầu phiên ATO vẽ nến xanh ảo, và dùng `MA(20)` cho Volume thay vì chuẩn **`MA(5)` (1 tuần)** kèm chuỗi tiến trình `T+3` của QMV.

### 1.2. Công thức Toán học & Mã giả Từng Bước (`100% Causal Pseudocode`)

```python
# INPUT tại mỗi phiên t: chuỗi OHLCV[0..t], sàn giao dịch exchange in {"HOSE", "HNX", "UPCOM"},
#                        cờ in_special_zone[t] (nhận từ Indicator 2 & 4: chạm S/R, Trendline, BB ngoài, OB/OS/Div, Vol đột biến).

def compute_qmv_merged_vpa_at_bar(t, O, H, L, C, V, exchange, in_special_zone, params):
    eps = 1e-6
    limit_pct = 0.07 if exchange == "HOSE" else (0.10 if exchange == "HNX" else 0.15)
    ref_prev = C[t-1] if exchange != "UPCOM" else (H[t-1] + L[t-1] + 2.0 * C[t-1]) / 4.0

    # BƯỚC 1: Nhận diện Khóa Trần/Sàn cứng & Lọc nhiễu ATO 100 cổ vẽ nến ảo (HP4 v1_4, v1_5, v3_2)
    is_flat = (H[t] - L[t]) <= eps
    ceil_lock = is_flat and (C[t] >= ref_prev * (1.0 + limit_pct - 0.004))
    floor_lock = is_flat and (C[t] <= ref_prev * (1.0 - limit_pct + 0.004))

    vol_ma5 = SMA(V, params.vol_ma_fast, t)      # Mặc định = 5 phiên (1 tuần giao dịch)
    vol_ma20 = SMA(V, params.vol_ma_base, t)     # Mặc định = 20 phiên (nền tham chiếu)

    paint_bull_open_trap = (
        (O[t] <= ref_prev * (1.0 - limit_pct + 0.005))
        and (C[t] <= ref_prev * 1.01)
        and (V[t] < 0.85 * vol_ma5)
    )
    paint_bear_open_trap = (
        (O[t] >= ref_prev * (1.0 + limit_pct - 0.005))
        and (C[t] >= ref_prev * 0.99)
        and (V[t] > 1.25 * vol_ma5)              # Phân phối trên đường về bằng khớp trần đầu phiên (HP4 v1_5)
    )

    # BƯỚC 2: Hàm giải phẫu 1 cây nến bất kỳ (đơn hoặc ghép) theo Quy tắc 1/3 & 50% của QMV
    def classify_qmv_bar(o_val, h_val, l_val, c_val, is_ceil, is_floor):
        if is_ceil:
            return ("SOLID_CEIL_CONT", 1.0, "TOP_THIRD")      # Nến đặc tiếp diễn tăng cực mạnh (CẤM gọi là Doji!)
        if is_floor:
            return ("SOLID_FLOOR_CONT", 1.0, "BOT_THIRD")     # Nến đặc tiếp diễn giảm cực mạnh
        rng = max(h_val - l_val, eps)
        body = abs(c_val - o_val)
        body_ratio = body / rng
        body_bot_pos = (min(o_val, c_val) - l_val) / rng
        body_top_pos = (max(o_val, c_val) - l_val) / rng

        if body_bot_pos >= 2.0 / 3.0:
            zone_13 = "TOP_THIRD"                             # Phe Mua thắng (bất kể thân xanh hay đỏ!)
        elif body_top_pos <= 1.0 / 3.0:
            zone_13 = "BOT_THIRD"                             # Phe Bán thắng (bất kể thân xanh hay đỏ!)
        else:
            zone_13 = "MID_THIRD"                             # Lưỡng lự cân bằng

        if body_ratio <= params.small_body_max:               # <= 1/3 (0.333)
            bar_kind = "DOJI" if body_ratio <= 0.08 else "HAMMER_PIN"
        elif body_ratio >= params.large_body_min:             # >= 1/2 (0.50)
            bar_kind = "MARUBOZU_BULL" if c_val > o_val else "MARUBOZU_BEAR"
        else:
            bar_kind = "NORMAL_IGNORE"                        # 1/3 < ratio < 1/2 -> Bỏ qua không suy diễn!
        return (bar_kind, body_ratio, zone_13)

    single_kind, single_ratio, single_pos = classify_qmv_bar(O[t], H[t], L[t], C[t], ceil_lock, floor_lock)

    # BƯỚC 3: Ghép Nến Thích Ứng K = 2 -> K = 3 CHỈ TẠI VÙNG ĐẶC BIỆT (HP4 v2_2_2, v3_2_hoidap)
    merged_k = 1
    merged_candle = (O[t], H[t], L[t], C[t], single_kind, single_pos)
    if in_special_zone and t >= 2 and not (ceil_lock or floor_lock):
        # Ưu tiên ghép 2 nến khi 2 nến ngược màu hoặc nến đơn chưa rõ nghĩa
        if (C[t] - O[t]) * (C[t-1] - O[t-1]) < 0 or single_kind in ("NORMAL_IGNORE", "DOJI", "HAMMER_PIN"):
            o2, c2, h2, l2 = O[t-1], C[t], max(H[t-1], H[t]), min(L[t-1], L[t])
            k2_kind, _, k2_pos = classify_qmv_bar(o2, h2, l2, c2, False, False)
            merged_k = 2
            merged_candle = (o2, h2, l2, c2, k2_kind, k2_pos)
            # Nếu ghép 2 nến vẫn ra MID_THIRD lưỡng lự (VD: Harami) -> ghép 3 nến (chu kỳ T+2.5)
            if k2_pos == "MID_THIRD" or k2_kind == "NORMAL_IGNORE":
                o3, c3, h3, l3 = O[t-2], C[t], max(H[t-2], h2), min(L[t-2], l2)
                k3_kind, _, k3_pos = classify_qmv_bar(o3, h3, l3, c3, False, False)
                merged_k = 3
                merged_candle = (o3, h3, l3, c3, k3_kind, k3_pos)

    # Kiểm tra Trục 50% thân nến mạnh đầu cụm & Cờ "Kill Nến Đỏ / Kill Nến Xanh" (wa_10_02_2026)
    pierce_50_bull = (C[t-1] < O[t-1]) and (r_body(t-1) >= 0.5) and (C[t] > 0.5 * (O[t-1] + C[t-1])) and (C[t] > O[t])
    star3_50_bull = (t >= 2) and (C[t-2] < O[t-2]) and (r_body(t-2) >= 0.5) and (r_body(t-1) <= 0.35) and (C[t] > 0.5 * (O[t-2] + C[t-2]))
    kill_red_bar = (C[t-1] < O[t-1] and C[t] > H[t-1] and not paint_bull_open_trap)
    kill_green_bar = (C[t-1] > O[t-1] and V[t-1] >= 1.25 * vol_ma5 and C[t] < L[t-1])

    # BƯỚC 4: Máy Trạng Thái VPA Chuỗi >= 3 Phiên quanh SMA(Vol, 5) (KT01T1, HP4 v3_1)
    vol_spike = V[t] >= params.vol_spike_mult * vol_ma5       # Mặc định >= 1.25 * SMA5(V)
    vol_climax = V[t] >= 1.80 * vol_ma5 and V[t] >= 1.50 * vol_ma20
    prior_tiet_cung = all(C[j] >= C[j-1] and V[j] <= 0.95 * SMA(V, 5, j) for j in range(t-3, t))
    gain_20d = (C[t] - min(L[t-20:t+1])) / max(min(L[t-20:t+1]), eps)
    gain_60d = (H[t] - min(L[t-60:t+1])) / max(min(L[t-60:t+1]), eps)

    if prior_tiet_cung and vol_climax and (C[t] <= C[t-1] or merged_candle[5] == "BOT_THIRD"):
        vpa_state = "CLIMAX_AFTER_TIET_CUNG"                  # ~100% XẢ HÀNG CÂY THÔNG -> CHẠY NGAY!
    elif C[t] > C[t-1] and V[t] < 0.90 * vol_ma5 and C[t] > SMA(C, 20, t):
        vpa_state = "TIET_CUNG_2A2"                           # Giữ chặt, khóa cung tốt
    elif sum(1 for j in range(t-4, t+1) if V[j] > SMA(V, 5, j)) >= 4 and abs(C[t] - C[t-4]) / C[t-4] <= 0.015 and gain_60d >= 0.25:
        vpa_state = "STALLING_HIGH_VOL"                       # "Đạp ga mà xe không chạy" -> Hạ tỷ trọng!
    elif vol_spike and C[t] < O[t] and 0.05 <= gain_60d <= 0.12:
        vpa_state = "SHAKEOUT_STAGE_D"                        # Rũ hàng cuối cùng khi mới nhú 5-10% khỏi nền gom
    elif C[t] < C[t-1] * 0.98 and vol_spike and C[t] < SMA(C, 20, t):
        vpa_state = "DIST_ON_WAY_DOWN"                        # Giá giảm + Vol tăng bất thường -> CẤM BẮT ĐÁY!
    elif abs(C[t] - C[t-1]) / C[t-1] <= 0.01 and V[t] <= 0.65 * vol_ma20 and V[t] <= V[t-1]:
        vpa_state = "CAN_CUNG_EXHAUST"                        # Cạn kiệt người bán tại đáy
    elif C[t] > C[t-1] and V[t] >= 0.95 * vol_ma5 and not vol_climax:
        vpa_state = "HEALTHY_ABSORPTION_2A1"                  # Tăng bình thường lành mạnh
    elif C[t] < C[t-1] and V[t] < vol_ma5:
        vpa_state = "NORMAL_PULLBACK_2B2"                     # Giảm bình thường thiếu cầu
    else:
        vpa_state = "NEUTRAL"

    # Nhận diện SPRING ("Đạp kéo") & UPTHRUST ("Kéo xả") + Kiểm tra Phủ định Upthrust bằng Nến Xanh thứ 3 (case TNH)
    is_spring = in_special_zone and (L[t] < min(L[t-5:t])) and (merged_candle[5] == "TOP_THIRD" or C[t] > O[t] and r_body(t) >= 0.55) and (V[t] >= V[t-1])
    is_upthrust = in_special_zone and (H[t] > max(H[t-5:t])) and (merged_candle[5] == "BOT_THIRD") and vol_spike
    upthrust_negated = (t >= 2) and is_upthrust_at(t-1) and (C[t] > max(H[t-2], H[t-1])) and (merged_candle[5] == "TOP_THIRD" or C[t] > O[t-2])

    return {
        "merged_k": merged_k, "merged_ohlc": merged_candle[:4], "merged_kind": merged_candle[4], "merged_pos_13": merged_candle[5],
        "ceil_lock": ceil_lock, "floor_lock": floor_lock, "paint_open_trap": paint_bull_open_trap or paint_bear_open_trap,
        "pierce_50_bull": pierce_50_bull or star3_50_bull, "kill_red": kill_red_bar, "kill_green": kill_green_bar,
        "vol_ma5": vol_ma5, "vpa_state": vpa_state, "is_spring": is_spring, "is_upthrust": is_upthrust, "upthrust_negated": upthrust_negated
    }
```

### 1.3. Tham số Tinh chỉnh (`Tunable Parameters`)
- `vol_ma_fast = 5` (chuẩn 1 tuần QMV; cho phép đổi `3` hoặc `10`); `vol_ma_base = 20`.
- `small_body_max = 0.333` (`1/3`); `large_body_min = 0.50` (`1/2`); `vol_spike_mult = 1.25` (`+25%` vượt `MA5`).
- `max_merge_bars = 3` (khớp chu kỳ `T+2.5`; tùy chọn bật `show_ghost_merged_box = true`).

### 1.4. Hiển thị Trực quan trên `KLineChart`
- **Trên khung Giá (`Overlay`):**
  - Khi `merged_k in {2, 3}` tại Vùng đặc biệt, vẽ một **khung viền nến ghép bán trong suốt (`Ghost Merged Candle`)** bao quanh $K$ cây nến con: viền xanh ngọc `#00E676` nếu `TOP_THIRD` / `MARUBOZU_BULL`, viền đỏ cam `#FF5252` nếu `BOT_THIRD` / `MARUBOZU_BEAR`, viền vàng `#FFD54F` nếu `MID_THIRD`.
  - Gắn nhãn pill trực tiếp trên đỉnh/đáy nến: `SPRING` (`#00C853`), `UT / KÉO XẢ` (`#D50000`), `UT HỦY (TĂNG TIẾP)` (`#2979FF`), `KILL ĐỎ` (`#00BFA5`), `KILL XANH!` (`#FF1744`), `TRẦN CỨNG` (`#AA00FF`), `SÀN CỨNG` (`#00B0FF`), `BẪY ATO!` (`#FF9100`).
- **Trên ngăn Khối lượng (`Sub-pane`):** Vẽ đường `SMA(Volume, 5)` nét liền vàng `#FFC107` nổi bật; tô màu cột Volume theo `vpa_state` (Xanh dương `#29B6F6` = `TIET_CUNG`, Tím chớp đỏ `#D500F9` = `CLIMAX_AFTER_TIET_CUNG`, Cam đậm `#FF6D00` = `STALLING_HIGH_VOL`, Xám nhạt `#78909C` = `CAN_CUNG`).

### 1.5. Kết hợp vào Bộ lọc & Chiến lược
- Cung cấp trực tiếp các cờ nguyên tử (`is_spring`, `is_upthrust`, `kill_red`, `kill_green`, `vpa_state`, `ceil_lock`, `floor_lock`) cho mọi bộ lọc Screener và làm cò súng `Trigger` / `Exit` cho Chiến lược **S1, S2, S4, S7, S8**.

---

## INDICATOR 2: `QMV_ZONE_3TOUCH` — TỰ ĐỘNG VẼ VÙNG GIAO DỊCH MUA/BÁN, `3 ĐIỂM CHẠM`, HỘP 3 ĐỘ NGHIÊNG & THƯỚC `RR`

### 2.1. Cơ sở Bài giảng QMV Gốc
- **Nguồn trích dẫn:** `KT01T2` (Hỗ trợ ngang là Chính, gọt bỏ râu hoảng loạn, đếm 3 điểm chạm, `RR > 1`), `KT01T3` (Kháng cự gần nhất, đo tịnh tiến `1/3 – 1/2 – 2/3`, chặn lãi `2/3` sóng), `HP4 v2_4, v3_4, v3_5` (36 mẫu hình quy về 3 độ nghiêng `< 45°`, độ lùi `<= 50%`, tia `Ray` nối chạm `1 -> 2` đón chạm `3`, Đỉnh Tháng trùng Đỉnh Quý, nến Breakout `2/3` thân trên hộp).

### 2.2. Công thức Toán học & Mã giả Từng Bước (`100% Causal Pseudocode`)

```python
def compute_qmv_zone_3touch_at_bar(t, O, H, L, C, V, vpa_flags, params):
    # BƯỚC 1: Gọt bỏ râu hoảng loạn / phiên khóa sàn mất thanh khoản khi lấy mốc neo (KT01T2 & KT01T3 Audio)
    H_norm, L_norm = [], []
    for i in range(max(0, t - params.lookback_bars), t + 1):
        is_panic_bar = vpa_flags[i]["floor_lock"] or (
            (C[i] - O[i]) / max(H[i] - L[i], 1e-6) > -0.33
            and (min(O[i], C[i]) - L[i]) > 2.2 * ATR(14, i)
            and C[i-1] < 0.95 * C[max(0, i-4)]
        )
        # Nếu là râu hoảng loạn nhất thời -> chốt vẽ theo thân/đuôi bình thường thay vì đầu mút râu cực đoan
        L_norm.append(min(O[i], C[i]) if is_panic_bar else L[i])
        H_norm.append(H[i])

    # BƯỚC 2: Gom cụm Hỗ trợ / Kháng cự Ngang Gần nhất có trọng số Khối lượng & Cờ "Đỉnh Tháng == Đỉnh Quý"
    # Chỉ dùng các Pivot nhân quả xác nhận tại tau <= t - m (m = params.pivot_left_right = 3)
    pivots_low = find_causal_pivots_low(L_norm, end_idx=t - params.pivot_m, window=params.pivot_m)
    pivots_high = find_causal_pivots_high(H_norm, end_idx=t - params.pivot_m, window=params.pivot_m)

    # Gom các pivot nằm trong dải dung sai zone_width_pct (mặc định 1.8% - 2.5%) thành các Vùng (Zone)
    support_zones = cluster_horizontal_zones(pivots_low, V, C[t], side="BELOW", tol=params.zone_tol_pct)
    resist_zones = cluster_horizontal_zones(pivots_high, V, C[t], side="ABOVE", tol=params.zone_tol_pct)

    # Đánh dấu Kháng cự Kim Cương khi Đỉnh 20 phiên (1 tháng) trùng khít Đỉnh 60 phiên (1 quý) (HP4 v3_5_thaoluan)
    hi_20 = max(H[t-20:t])
    hi_60 = max(H[t-60:t])
    month_eq_quarter_res = abs(hi_20 - hi_60) / max(hi_60, 1e-6) <= 0.008

    # BƯỚC 3: Đếm Nhân quả Điểm Chạm 1 - 2 - 3 trên Vùng Hỗ trợ & Kháng cự gần nhất
    active_sup = support_zones[0] if support_zones else None   # [S_low, S_high, raw_lowest_wick]
    active_res = resist_zones[0] if resist_zones else None     # [R_low, R_high]

    sup_touches = count_distinct_zone_touches(L, H, C, active_sup, start=t-45, end=t, min_sep=3)
    res_touches = count_distinct_zone_touches(H, L, C, active_res, start=t-45, end=t, min_sep=3)
    # sup_touches = 1 -> "CHẠM 1 (DỄ FAIL)"; = 2 -> "CHẠM 2 (TEST CUNG - MUA CHÍNH)"; >= 3 -> "CHẠM 3 (CHÍN MUỒI)"

    # BƯỚC 4: Bộ nhận diện Hộp / Phễu 3 Độ Nghiêng (36 Mẫu hình QMV) bằng Tia nối 2 điểm đảo chiều gần nhất
    pattern_box = None
    if len(pivots_high) >= 2 and len(pivots_low) >= 2:
        h1, h2 = pivots_high[-2], pivots_high[-1]              # (idx, price)
        l1, l2 = pivots_low[-2], pivots_low[-1]
        slope_top_pct = (h2.price - h1.price) / max(h1.price * (h2.idx - h1.idx), 1e-6)
        slope_bot_pct = (l2.price - l1.price) / max(l1.price * (l2.idx - l1.idx), 1e-6)
        avg_slope = 0.5 * (slope_top_pct + slope_bot_pct)
        atr_pct = ATR(14, t) / C[t]

        # Kiểm tra 2 bộ lọc hình học cứng: Góc dốc thoải (< 45 độ ~ |avg_slope| < 0.8 * atr_pct) & Độ lùi <= 50%
        prior_leg = measure_prior_impulse_leg(H, L, min(h1.idx, l1.idx))
        retrace_depth = (max(h1.price, h2.price) - min(l1.price, l2.price)) / max(prior_leg.height, 1e-6)

        if abs(avg_slope) < 0.8 * atr_pct and retrace_depth <= 0.50:
            vol_slope = linreg_slope(V[min(h1.idx, l1.idx):t+1])
            if avg_slope > +0.15 * atr_pct:
                box_type = "SLOPE_UP_BEARISH"                  # Quy luật 1: Nghiêng lên (+ Vol tăng) -> GIẢM GIÁ!
            elif avg_slope < -0.15 * atr_pct:
                box_type = "SLOPE_DOWN_BULLISH"                # Quy luật 2: Nghiêng xuống (+ Vol cạn) -> TĂNG GIÁ!
            else:
                # Quy luật 3: Nằm ngang -> Đếm số lần nghỉ trong sóng lớn (nhịp 1-2 tiếp diễn, nhịp >= 3 đảo chiều)
                pause_num = count_prior_consolidation_boxes(H, L, t, lookback=120)
                box_type = "FLAT_CONT_1_2" if pause_num <= 2 else "FLAT_REV_3_TOP_BOT"

            upper_ray_t = h2.price + (h2.price - h1.price) / (h2.idx - h1.idx) * (t - h2.idx)
            lower_ray_t = l2.price + (l2.price - l1.price) / (l2.idx - l1.idx) * (t - l2.idx)
            # Kiểm tra quy tắc Breakout 2/3 thân nến nằm trên biên kháng cự hộp (HP4 v3_5_thaoluan)
            valid_breakout_23 = (
                C[t] > upper_ray_t and C[t] > O[t]
                and (C[t] - upper_ray_t) / max(C[t] - O[t], 1e-6) >= (2.0 / 3.0)
                and V[t] >= 1.30 * SMA(V, 5, t-1)
            )
            pattern_box = {
                "type": box_type, "upper_t": upper_ray_t, "lower_t": lower_ray_t,
                "retrace_depth": retrace_depth, "vol_expanding": vol_slope > 0, "breakout_23": valid_breakout_23
            }

    # BƯỚC 5: Phóng chiếu Mục tiêu Tịnh tiến (1/3 - 1/2 - 2/3 - 100%), Chặn lãi 2/3 Sóng & Tỷ lệ RR trực tiếp
    stop_loss = active_sup.raw_lowest_wick if active_sup else (C[t] - 2.0 * ATR(14, t))
    target_1 = active_res.low if active_res else (C[t] + prior_leg.height * (2.0 / 3.0))
    rr_ratio = (target_1 - C[t]) / max(C[t] - stop_loss, 1e-6)
    cur_leg_low, cur_leg_high = current_swing_bounds(H, L, t)
    trailing_stop_23 = cur_leg_low + (2.0 / 3.0) * (cur_leg_high - cur_leg_low)

    return {
        "active_sup": active_sup, "active_res": active_res, "month_eq_quarter_res": month_eq_quarter_res,
        "sup_touches": sup_touches, "res_touches": res_touches, "pattern_box": pattern_box,
        "stop_loss": stop_loss, "target_1": target_1, "rr_ratio": rr_ratio, "trailing_stop_23": trailing_stop_23
    }
```

### 2.3. Tham số Tinh chỉnh
- `lookback_bars = 120` (6 tháng tìm cản gần nhất); `pivot_m = 3`; `zone_tol_pct = 0.02` (`2.0%`); `clip_panic_wicks = true`.

### 2.4. Hiển thị Trực quan trên `KLineChart`
- Vẽ **Dải Hỗ trợ ngang** màu xanh lá mờ `rgba(0, 200, 83, 0.14)` (kèm đường đứt nét đỏ mảnh dưới cùng ghi `SL (Râu thấp nhất): XX.X`) và **Dải Kháng cự ngang** màu đỏ san hô mờ `rgba(255, 82, 82, 0.14)` (nếu trùng `Đỉnh Tháng = Đỉnh Quý` thì vẽ viền vàng kim cương `KC THÁNG=QUÝ`).
- Trên mỗi điểm chạm hiển thị vòng tròn đánh số **`①`**, **`② TEST CUNG`**, **`③ ĐIỂM NỔ`**; vẽ 2 tia `Ray` đóng khung mẫu hình (tô nền xanh nhạt nếu `SLOPE_DOWN_BULLISH`, cam cảnh báo nếu `SLOPE_UP_BEARISH`).
- Vẽ đường bậc thang cam đứt nét **`Chặn lãi 2/3 Sóng`** bám dưới giá khi đang tăng, và góc phải trên hiển thị thẻ **`RR = 1.85x (ĐẠT > 1)`** (xanh nếu `> 1`, xám đỏ nếu `<= 1`).

### 2.5. Kết hợp vào Bộ lọc & Chiến lược
- Cho phép lọc ngay trên Screener: *"Cổ phiếu đang ở Vùng Hỗ trợ Chạm 2–3 có `RR > 1.5`"*, *"Phễu giảm cạn cung chạm 3"*, *"Nến Breakout chuẩn `2/3` vượt Đỉnh Tháng=Quý"*, hoặc *"Cảnh báo Kênh bò lên dễ gãy"*.

---

## INDICATOR 3: `QMV_SMART_MONEY_CYCLE` — RADAR 5 PHA TRÒ CHƠI `CÁO – THỎ` & ĐỊNH VỊ BỐI CẢNH LỚN `0 – 8`

### 3.1. Cơ sở Bài giảng QMV Gốc
- **Nguồn trích dẫn:** `HP1 v1..v3` (5 pha của Cáo: Gom 1–3 tháng $\ge 50\%$ float, Spring & Điểm 1, Đẩy $\ge 50\%$ có 2 lần chỉnh $\le 50\%$, Xả 2 bước hoặc Cây thông đường về 40–50%, Đè về Điểm 3 $>$ Điểm 1), `KT01T2 & KT01T4` (Sơ đồ tâm lý `0-1-2-3-4-5-6-7-8`, chỉnh `<= 1/3` khỏe vs `>= 50%` khó vượt đỉnh), `wa_10_02_2026` (`Weekly EMA200`).

### 3.2. Công thức Toán học & Mã giả Từng Bước (`100% Causal Pseudocode`)

```python
def compute_qmv_smart_money_cycle_at_bar(t, O, H, L, C, V, zone_info, vpa_info, weekly_ema200, params):
    # BƯỚC 1: Đo đặc trưng nền tích lũy 1-3 tháng (20..65 phiên) & biên độ sóng từ nền gốc Vùng 0
    base_win = params.accum_window                             # Mặc định 45 phiên (~2 tháng)
    hi_base, lo_base = max(H[t-base_win:t]), min(L[t-base_win:t])
    med_base = median(C[t-base_win:t])
    tightness = (hi_base - lo_base) / max(med_base, 1e-6)
    quiet_vol_ratio = median(V[t-base_win:t-5]) / max(SMA(V, 60, t-5), 1e-6)
    late_accum_vol_pickup = (SMA(V, 5, t) > 1.25 * median(V[t-base_win:t-5])) and (abs(C[t] - med_base) / med_base <= 0.06)

    # Tìm mốc Vùng 0 (chân nền gom gần nhất trước sóng hiện tại)
    box0_anchor = locate_causal_box0_anchor(H, L, C, V, t, max_lookback=180)
    wave_gain_from_0 = (C[t] - box0_anchor.price) / max(box0_anchor.price, 1e-6)
    peak_since_0 = max(H[box0_anchor.idx:t+1])
    drawdown_from_peak = (peak_since_0 - C[t]) / max(peak_since_0, 1e-6)

    # BƯỚC 2: Đếm số nhịp đẩy & hộp điều chỉnh kể từ Vùng 0 -> Ánh xạ sang Sơ đồ 0-1-2-3-4-5-6-7-8
    swings = extract_causal_zigzag_legs(H, L, C, start=box0_anchor.idx, end=t, min_leg_pct=params.min_leg_pct)
    up_legs = [s for s in swings if s.dir == +1]
    down_legs = [s for s in swings if s.dir == -1]

    if tightness <= params.max_base_range and wave_gain_from_0 < 0.08:
        cycle_node = 0                                         # Hộp 0: Nền tích lũy Sợ hãi / Cân bằng gốc
    elif C[t] >= box0_anchor.price and drawdown_from_peak <= 0.50:
        # Đang ở sườn trái 0 -> 5
        n_pauses = len(down_legs)
        in_pullback = (swings[-1].dir == -1) if swings else False
        if n_pauses == 0:
            cycle_node = 1                                     # Đang đi nhịp đầu 0 -> 1
        elif n_pauses == 1:
            cycle_node = 2 if in_pullback else 3               # Hộp 2 (Vùng mua đẹp nhất!) hoặc đang kéo Sóng 3 (2 -> 3)
        else:
            cycle_node = 4 if in_pullback else 5               # Hộp 4 (Rủi ro cao hơn) hoặc Vùng 5 Hưng phấn cuối chu kỳ!
    else:
        # Đã gãy quá 50% từ đỉnh hoặc tạo Lower Low sau Bước 5 -> Sườn phải 5 -> 6 -> 7 -> 8
        right_bounces = count_bounces_after_peak(H, L, C, peak_idx=argmax_since_0, end=t)
        if right_bounces == 0:
            cycle_node = 6                                     # Nhịp rơi đầu tiên 5 -> 6 (Sóng A)
        elif swings[-1].dir == +1:
            cycle_node = 7                                     # Nhịp hồi Bulltrap Sóng B -> VÙNG "BÁN NỐT"!
        else:
            cycle_node = 8                                     # Sóng C rũ bỏ toàn diện -> chờ tái lập Vùng 0 mới

    # Đánh giá sức khỏe nhịp chỉnh hiện tại theo thước QMV (<= 1/3 vs > 50% vs > 2/3)
    last_retrace = down_legs[-1].retrace_of_prev if down_legs else 0.0
    pullback_health = (
        "HEALTHY_LE_1_3" if last_retrace <= 0.36 else
        "WEAK_1_3_TO_1_2" if last_retrace <= 0.50 else
        "BROKEN_GT_50PCT" if last_retrace <= 0.67 else "TREND_DEAD_GT_2_3"
    )

    # BƯỚC 3: Phân loại 5 Pha Trò Chơi Cáo - Thỏ (HP1)
    if cycle_node == 0 and tightness <= params.max_base_range and quiet_vol_ratio <= 0.90:
        fox_phase = "PHA_1_GOM_CUOI" if late_accum_vol_pickup else "PHA_1_GOM_TINH_LANG"
    elif cycle_node in (0, 1) and (vpa_info["is_spring"] or (0.05 <= wave_gain_from_0 <= 0.20 and V[t] >= 1.3 * SMA(V, 20, t))):
        fox_phase = "PHA_2_KHOI_DONG_DIEM_1"                   # Vùng Thỏ thông minh lên tàu (mới tăng 5-20%)!
    elif cycle_node in (2, 3, 4) and pullback_health in ("HEALTHY_LE_1_3", "WEAK_1_3_TO_1_2"):
        fox_phase = "PHA_3_DAY_GIA_VA_NGHI"
    elif (wave_gain_from_0 >= 0.35 and vpa_info["vpa_state"] in ("STALLING_HIGH_VOL", "CLIMAX_AFTER_TIET_CUNG")) or (
        0.35 <= drawdown_from_peak <= 0.55 and V[t] >= 2.0 * SMA(V, 20, t) and peak_since_0 / box0_anchor.price >= 2.0
    ):
        # Bao gồm cả Phân phối 2 bước trên đỉnh VÀ Biến thể "Cây thông xả đường về" sau khi rơi 40-50%!
        fox_phase = "PHA_4_CAO_XA_HANG"
    elif cycle_node in (6, 7, 8):
        fox_phase = "PHA_5_DE_GIA_MARKDOWN"
    else:
        fox_phase = "CHUYEN_TIEP"

    dist_to_wk_ema200 = (C[t] - weekly_ema200) / max(weekly_ema200, 1e-6)
    return {
        "cycle_node": cycle_node, "fox_phase": fox_phase, "wave_gain_from_0": wave_gain_from_0,
        "pullback_health": pullback_health, "dist_to_wk_ema200": dist_to_wk_ema200
    }
```

### 3.3. Tham số Tinh chỉnh
- `accum_window = 45` (`20..65` phiên); `max_base_range = 0.18` (`18%`); `min_leg_pct = 0.08` (`8%` cho cổ phiếu thường, `12%` cho `DT3`).

### 3.4. Hiển thị Trực quan trên `KLineChart`
- **Overlay:** Đánh số trực tiếp các mốc **`[0]`**, **`[1]`**, **`[2 - HỘP MUA TỐT NHẤT]`**, **`[3]`**, **`[4 - HỘP CAO]`**, **`[5 - HƯNG PHẤN / BÁN MẠNH]`**, **`[6]`**, **`[7 - BULLTRAP BÁN NỐT]`**, **`[8]`** trên các đầu mút sóng đã xác nhận nhân quả, kèm đường kẻ xanh dương đậm **`Weekly EMA200 (Sàn Quỹ Lớn)`**.
- **Sub-pane Băng Trạng Thái (`Ribbon Strip` cao 36px):** Tô dải màu liên tục theo 5 pha Cáo–Thỏ: Xanh lam nhạt (`Pha 1 Gom`) $\rightarrow$ Xanh ngọc lục bảo (`Pha 2 Điểm 1`) $\rightarrow$ Xanh lá (`Pha 3 Đẩy`) $\rightarrow$ Đỏ tươi cảnh báo (`Pha 4 Cáo Xả / Xả Đường Về`) $\rightarrow$ Xám tối (`Pha 5 Đè giá`).

### 3.5. Kết hợp vào Bộ lọc & Chiến lược
- Cổng định vị chiến lược tự động: Chiến lược **S1/S4/S5** ưu tiên `cycle_node in {0, 2}`; **S7B (Trà đá)** lọc `fox_phase in {"PHA_1_GOM_CUOI", "PHA_2_KHOI_DONG_DIEM_1"}`; **S2 (Bắt đáy)** lọc `cycle_node == 8` + `dist_to_wk_ema200 <= +0.06`; **S8A (Bán quyết liệt)** kích hoạt ở `cycle_node in {5, 7}` hoặc `fox_phase == "PHA_4_CAO_XA_HANG"`.

---

## INDICATOR 4: `QMV_REGIME_RIBBON_DIV` — DẢI CHUYỂN CHẾ ĐỘ XU HƯỚNG–DAO ĐỘNG & MÁY QUÉT 4 PHÂN KỲ NHÂN QUẢ

### 4.1. Cơ sở Bài giảng QMV Gốc
- **Nguồn trích dẫn:** `HP4 v1_3, v2_3_1..4, v3_3_bg + hd`, `KT01T3` (`BB(15, 2)` chui vào lần 2, `CCI` kèm đường `MA` của `CCI`), `V7 Guide` (`Ichimoku (18, 6, 18, 36)` + Bear-Trap / Bull-Trap), `HP5 v3_4` (`Close > High[lowest_bar]` / `Close < Low[highest_bar]`).

### 4.2. Công thức Toán học & Mã giả Từng Bước (`100% Causal Pseudocode`)

```python
def compute_qmv_regime_ribbon_div_at_bar(t, O, H, L, C, V, params):
    # BƯỚC 1: Tính bộ Overlay chuẩn QMV: SMA(20, 50, 200), BB(15, 2.0) & Ichimoku QMV (6, 18, 36, 18)
    sma20, sma50, sma200 = SMA(C, 20, t), SMA(C, 50, t), SMA(C, 200, t)
    bb_mid, bb_std = SMA(C, params.bb_period, t), STD(C, params.bb_period, t)   # Mặc định bb_period = 15 (KT01T3)
    bb_up, bb_lo = bb_mid + 2.0 * bb_std, bb_mid - 2.0 * bb_std
    bb_width = (bb_up - bb_lo) / max(bb_mid, 1e-6)
    bb_squeeze = bb_width <= percentile(bb_width_series[t-120:t+1], 15)         # Thắt nút cổ chai 15% lịch sử mã

    ichi = compute_ichimoku_qmv(H, L, C, t, tenkan=6, kijun=18, senkou_b=36, disp=18)
    # Phát hiện Bẫy giá Ichimoku đặc sản QMV (HP4 v2_3_2, v3_3_bg)
    ichi_bear_trap = (
        C[t] > ichi.cloud_top_now and sma50 > sma200
        and any(C[j] < ichi.kijun[j] and L[j] >= 0.99 * ichi.cloud_bot_now for j in range(t-4, t))
        and C[t] > max(ichi.tenkan[t], ichi.kijun[t])
    )
    ichi_bull_trap = (
        C[t] < ichi.cloud_bot_now and sma50 < sma200
        and any(C[j] > ichi.kijun[j] for j in range(t-4, t))
        and C[t] < ichi.tenkan[t]
    )

    # Đếm tín hiệu Giá từ ngoài biên BB(15, 2) chui ngược vào trong dải (Lần 1 vs Lần 2 - KT01T3)
    bb_upper_reentry_2nd = count_band_reentries(H, C, bb_up_series, start=t-18, end=t, side="UPPER") >= 2
    bb_lower_reentry_2nd = count_band_reentries(L, C, bb_lo_series, start=t-18, end=t, side="LOWER") >= 2

    # BƯỚC 2: Bộ Chuyển Chế Độ Tự Động TRENDING vs SIDEWAYS & Thiết Quân Luật Bất Đối Xứng (HP4 v2_3_1, v3_3_bg)
    if C[t] > ichi.cloud_top_now and sma20 > sma50 and (sma50 > sma50_prev5) and not bb_squeeze:
        regime = "TRENDING_UP"
        # Trong Uptrend: KHÓA bán non vì quá mua; BẬT nhạy mua nhanh khi rũ quá bán!
        active_engine = "OVERLAY_MA_ICHI_PRIMARY | OSC_OVERSOLD_BUY_ONLY"
    elif C[t] < ichi.cloud_bot_now and sma20 < sma50 and (sma50 < sma50_prev5) and not bb_squeeze:
        regime = "TRENDING_DOWN"
        # Trong Downtrend: KHÓA bắt dao rơi quá bán; BẬT bán thoát nhanh khi hồi quá mua!
        active_engine = "DEFENSIVE_CASH | OSC_OVERBOUGHT_SELL_ONLY"
    else:
        regime = "SIDEWAYS_BOX"
        # Trong Sideways: TẮT giao cắt MA (tránh whipsaw T+); BẬT RSI/CCI + Biên Hộp S/R!
        active_engine = "OSC_AND_SR_BOX_PRIMARY | DISABLE_MA_CROSS"

    # BƯỚC 3: Tính RSI(14), CCI(14) kèm SMA(CCI, 5) & MACD(12, 26, 9) -> Quét 4 Loại Phân Kỳ Nhân Quả
    rsi14 = RSI(C, 14, t)
    cci14 = CCI(H, L, C, params.cci_period, t)                 # Mặc định 14 (cho phép 10 hoặc 20)
    cci_ma5 = SMA_series(cci14_series, 5, t)                   # Đường MA gắn trên CCI để bắt điểm uốn (KT01T3)
    macd_line, sig_line, hist = MACD(C, 12, 26, 9, t)

    # Điều kiện cần chống "Overthinking đoán phân kỳ sớm": Giá đã lình xình >= 2 phiên & chỉ báo tạo điểm uốn!
    osc_inflect_up = (rsi14 > rsi14_prev and cci14 > cci14_prev and cci14 > cci_ma5)
    osc_inflect_dn = (rsi14 < rsi14_prev and cci14 < cci14_prev and cci14 < cci_ma5)

    # Quét 4 loại phân kỳ nối theo High/Low (HP4 v1_3, v3_3_hd) + Cò súng giá nhân quả (HP5 v3_4)
    div_signals = scan_4_causal_divergences(
        H, L, C, rsi_series, cci_series, macd_series, t,
        osc_inflect_up=osc_inflect_up, osc_inflect_dn=osc_inflect_dn,
        require_dual_confirm=params.require_rsi_and_cci        # Confirm đồng thời RSI + CCI
    )
    # Mỗi phần tử trong div_signals gồm:
    # - kind in {"REG_BULL", "HID_BULL", "REG_BEAR", "HID_BEAR"}
    # - status in {"WARNING_LINH_XINH", "CONFIRMED_TRIGGER"}
    #   trong đó CONFIRMED_TRIGGER chỉ bật khi C[t] > H[idx_lowest_low] (Bull) hoặc C[t] < L[idx_highest_high] (Bear)!

    return {
        "regime": regime, "active_engine": active_engine, "bb15": (bb_up, bb_mid, bb_lo, bb_squeeze),
        "bb_reentry2": (bb_lower_reentry_2nd, bb_upper_reentry_2nd),
        "ichi_qmv": ichi, "ichi_traps": (ichi_bear_trap, ichi_bull_trap),
        "rsi14": rsi14, "cci14": cci14, "cci_ma5": cci_ma5, "macd": (macd_line, sig_line, hist),
        "divergences": div_signals
    }
```

### 4.3. Tham số Tinh chỉnh
- `bb_period = 15` (tùy chọn `20`); `ichi_params = (6, 18, 36, 18)` (tùy chọn `(3, 6, 18, 36)` cho T+ nhanh hoặc `(9, 26, 52, 26)` cổ điển); `cci_period = 14`; `require_rsi_and_cci = true`.

### 4.4. Hiển thị Trực quan trên `KLineChart`
- **Overlay:** Đám mây `Ichimoku (6, 18, 36, 18)` thanh thoát + dải `BB(15, 2)` nét mảnh; khi `regime == "SIDEWAYS_BOX"`, làm mờ các đường MA xuống độ đậm `20%` để nhắc trader không đánh giao cắt MA! Gắn huy hiệu `BEAR-TRAP (MUA!)` / `BULL-TRAP (THOÁT!)` và `BB CHUI VÀO LẦN 2`.
- **Sub-pane:** Hiển thị đồng thời `RSI(14)` và `CCI(14)` + `SMA(CCI, 5)` (chuẩn hóa chung khung nhìn hoặc 2 tầng gọn), tự động kẻ đoạn thẳng nối 2 đỉnh/đáy phân kỳ: nét đứt vàng khi mới ở trạng thái `WARNING_LINH_XINH` và chuyển sang **nét liền xanh/đỏ kèm mũi tên kích hoạt** ngay tại cây nến thỏa `CONFIRMED_TRIGGER`!

### 4.5. Kết hợp vào Bộ lọc & Chiến lược
- Lọc tự động các tổ hợp vàng của thầy Hào: *"BB Squeeze + Phân kỳ dương kép RSI&CCI + MACD cắt lên"*, *"Ichimoku Bear-Trap trong Uptrend"*, *"Phân kỳ dương ẩn RSI > 50"*.

---

## INDICATOR 5: `QMV_BLACKBOX_PROXY` — BỘ TÁI TẠO HỘP ĐEN DÒNG TIỀN CỔ PHIẾU TỪ `OHLCV`

### 5.1. Cơ sở Bài giảng QMV Gốc & Ranh giới Trung thực
- **Nguồn trích dẫn:** `QMV_SYSTEM_GUIDE_V5.md`, `HƯỚNG_DẪN_V7.pdf.md`, `Chọn_cổ_phiếu_Fintech_QMV_17072025.mp3.md`, `fintech_buoi_1_21052026_audio.mp3.md`, `Fintech_buổi_2_25062026_audio.mp3.md`.
- **Minh bạch tri thức:** Trên Webapp QMV Fintech, hộp đen tính từ dữ liệu lệnh/khớp chi tiết nội bộ. Tuy nhiên, dựa trên toàn bộ các quy luật cơ học do chính GS. Quách Mạnh Hào tiết lộ trong 4 tài liệu gốc (*hai đơn vị Tiền vs Cổ phiếu khác nhau; bất đối xứng ưu tiên Cầu hơn Cung; `TMC` kèm `TMA20/TMA50` quyết định 4 pha `BB Status`; `SP = TM_chuẩn - TS_chuẩn`; `DSPI [-1,+1]` chu kỳ 20 phiên; `MPICP [0,100]` đo tiền nhỏ lẻ so với tiền lớn; cắt lát 6 chu kỳ `03..200` chuẩn hóa 50 phiên kèm 2 bẫy chuẩn hóa; `0–5 sao + Tốt hơn`; không có `Bearish Signal 4`*), bộ công thức `OHLCV Proxy` dưới đây tái tạo trung thành tối đa cấu trúc động lực học của QMV trên lịch sử 10 năm `OHLCV`.

### 5.2. Công thức Toán học & Mã giả Từng Bước (`100% Causal Pseudocode`)

```python
def compute_qmv_blackbox_stock_proxy_series(O, H, L, C, V, anchor_idx, params):
    eps = 1e-9
    N = len(C)
    TMC, TSC = [0.0] * N, [0.0] * N

    for t in range(1, N):
        # BƯỚC 1: Phân tách VSA Bất đối xứng Tiền vào hộp (dM_t) vs Cổ phiếu bán vào hộp (dS_t)
        tr_t = max(H[t], C[t-1]) - min(L[t], C[t-1]) + eps
        clv_t = ((C[t] - min(L[t], C[t-1])) - (max(H[t], C[t-1]) - C[t])) / tr_t   # [-1, +1]
        ret_t = (C[t] - C[t-1]) / max(C[t-1], eps)
        typ_p = (H[t] + L[t] + 2.0 * C[t]) / 4.0
        val_t = typ_p * V[t]                                                        # Đơn vị: Giá trị Tiền (VNĐ)
        val_ma20 = SMA_val(typ_p_series, V, 20, t)
        vol_ma20 = SMA(V, 20, t)

        # Tiên đề ưu tiên Cầu (alpha_demand = 1.15 > 1.0): Giá tăng đóng cửa cao -> dM dương mạnh;
        # Khối lượng nổ to nhưng râu trên dài (H - C lớn) -> dS vọt nhanh hơn dM làm tốc độ SP âm!
        demand_drive = 0.65 * clv_t + 0.35 * tanh(ret_t / 0.035)
        alpha = params.demand_asymmetry if demand_drive >= 0 else 1.0               # Mặc định alpha = 1.15
        dM_t = val_t * alpha * demand_drive

        upper_wick_pen = (H[t] - C[t]) / tr_t if V[t] > 1.2 * vol_ma20 else 0.0
        supply_drive = 0.55 * (-clv_t) + 0.45 * upper_wick_pen + 0.25 * max(0.0, V[t] / max(vol_ma20, eps) - 1.0) * (1.0 if C[t] <= C[t-1] else 0.3)
        dS_t = V[t] * supply_drive                                                  # Đơn vị: Lượng Cổ phiếu

        if t >= anchor_idx:                                                         # Ngày gốc mặc định 20/03/2024
            TMC[t] = TMC[t-1] + dM_t
            TSC[t] = TSC[t-1] + dS_t

    # BƯỚC 2: Tính tại phiên t các biến trạng thái từ chuỗi lũy kế [0..t] (100% Nhân quả!)
    tma20 = SMA_arr(TMC, 20, t)
    tma50 = SMA_arr(TMC, 50, t)

    # Chu trình 4 pha BB Status & Ngày giao cắt BB Predict (V5 tr.26, Fintech B1 & B2)
    if TMC[t] > tma20 and tma20 > tma50 and TMC[t] >= TMC[t-3]:
        bb_status = "TIEN_KHOE"                                                     # Pha 2: Mua gia tăng / Giữ
    elif TMC[t] >= tma20 and tma20 <= tma50 and (tma50 - tma20) < (SMA_arr(TMC, 50, t-3) - SMA_arr(TMC, 20, t-3)):
        bb_status = "TIEN_DUY_TRI"                                                  # Pha 1: Ưu tiên Mua lần đầu!
    elif TMC[t] <= tma20 and tma20 >= tma50:
        bb_status = "TIEN_BAO_HOA"                                                  # Pha 3: Bán lần đầu / Chốt tỉa
    else:
        bb_status = "TIEN_YEU"                                                      # Pha 4: Bán gia tăng / Đứng ngoài

    # Chuẩn hóa TM_Chuẩn & TS_Chuẩn [0, 1] từ anchor_idx đến t -> Tốc độ SP = TM_Chuẩn - TS_Chuẩn
    tm_norm = minmax_scale_causal(TMC, anchor_idx, t)
    ts_norm = minmax_scale_causal(TSC, anchor_idx, t)
    sp_cum = tm_norm - ts_norm

    # BƯỚC 3: Cắt lát 6 Chu kỳ k in {3, 5, 10, 20, 50, 200}, Chuẩn hóa 50 phiên (Z) & Chống 2 Bẫy Chuẩn Hóa!
    DM, DS, DMZ, DSZ, EM = {}, {}, {}, {}, {}
    for k in (3, 5, 10, 20, 50, 200):
        DM[k] = TMC[t] - TMC[max(0, t - k)]
        DS[k] = TSC[t] - TSC[max(0, t - k)]
        # Chuẩn hóa Min-Max lăn trong cửa sổ 50 phiên gần nhất [t-49 .. t] -> thang [0, 100]
        dm_win = [TMC[j] - TMC[max(0, j - k)] for j in range(max(0, t - 49), t + 1)]
        ds_win = [TSC[j] - TSC[max(0, j - k)] for j in range(max(0, t - 49), t + 1)]
        DMZ[k] = 100.0 * (DM[k] - min(dm_win)) / max(max(dm_win) - min(dm_win), eps)
        DSZ[k] = 100.0 * (DS[k] - min(ds_win)) / max(max(ds_win) - min(ds_win), eps)
        EM[k] = (DM[k] / max(val_ma20 * k, eps)) - (DS[k] / max(vol_ma20 * k, eps)) # EM03 > 0: Hấp thụ hết hàng T+2.5 về!

    # Kiểm tra 2 Bẫy Chuẩn Hóa 50 phiên (Fintech Buổi 2 - CỰC KỲ QUAN TRỌNG!):
    dead_amplitude_trap = (max(dm_win_k3) - min(dm_win_k3)) < 0.20 * val_ma20       # Bẫy 1: Biên độ mặc định chết phẳng lì
    high_anchor_trap = min(dm_win_k20) > +0.50 * val_ma20 * 5                       # Bẫy 2: Neo tít trên đỉnh dương cao
    norm_trap_flag = dead_amplitude_trap or high_anchor_trap

    # Tín hiệu Uốn lên 20/30 theo cấp độ vòng tiền T+, T++, SW (chỉ hợp lệ khi KHÔNG dính Bẫy chuẩn hóa!)
    uon_len_t_plus = (not norm_trap_flag) and (DMZ[3] > DMZ_prev[3] and DMZ_prev[3] < 25) and (DMZ[5] > DMZ_prev[5] and DMZ_prev[5] < 30)
    uon_len_swing = uon_len_t_plus and (DMZ[10] > DMZ_prev[10] and DMZ_prev[10] < 30) and (DMZ[20] > DMZ_prev[20] and DMZ_prev[20] < 35)
    dao_chieu_tang_t_plus = (dM_series[t] > 0) and (dM_series[t-1] < 0) and (dM_series[t-2] < 0) and (EM[3] > 0)

    # BƯỚC 4: DSPI [-1, +1] (+ MA5, MA20) -> 4 Pha XH Cầu (20 phiên) & MPICP [0, 100] (+ MA5, MA20)
    raw_dsp = EMA_step(EM[20], 3, t)
    dspi = clip(raw_dsp / max(max_abs_window(raw_dsp_series, t, 20), eps), -1.0, +1.0)
    dspi_ma5, dspi_ma20 = SMA_arr(dspi_series, 5, t), SMA_arr(dspi_series, 20, t)
    xh_cau = (
        "CAU_DUY_TRI" if dspi < 0 and dspi > dspi_prev else
        "CAU_KHOE"    if dspi >= 0 and dspi >= dspi_ma5 else
        "CAU_BAO_HOA" if dspi > 0.4 and dspi < dspi_prev else "CAU_YEU"
    )

    # MPICP [0, 100] quanh 50: Xấp xỉ hưng phấn/hoảng loạn của dòng tiền Nhỏ lẻ (Thỏ) so với Tay to
    retail_fomo = 0.55 * Stochastic_K(H, L, C, 14, t) + 0.45 * RSI(C, 10, t)
    inst_dampener = -10.0 if (V[t] > 1.6 * vol_ma20 and abs(C[t] - C[t-1]) / C[t-1] < 0.015 and C[t] < SMA(C, 20, t)) else 0.0
    mpicp = clip(EMA_val(retail_fomo + inst_dampener, 3, t), 0.0, 100.0)
    mpicp_ma5, mpicp_ma20 = SMA_arr(mpicp_series, 5, t), SMA_arr(mpicp_series, 20, t)

    # BƯỚC 5: Chấm Điểm Phù Hợp Giao Dịch (0 - 5 Sao) + Đạo hàm bậc 1 (Tốt hơn / Không đổi / Kém hơn)
    price_trend = "UP" if (C[t] > SMA(C, 20, t) > SMA(C, 50, t) and SMA(C, 50, t) >= SMA(C, 50, t-5)) else (
                  "DOWN" if (C[t] < SMA(C, 20, t) < SMA(C, 50, t) and SMA(C, 50, t) < SMA(C, 50, t-5)) else "SIDEWAYS")
    raw_score = (
        (2.0 if price_trend == "UP" else (1.0 if price_trend == "SIDEWAYS" and C[t] > SMA(C, 20, t) else 0.0))
        + (1.5 if bb_status == "TIEN_KHOE" else (1.0 if bb_status == "TIEN_DUY_TRI" else 0.0))
        + (1.0 if xh_cau in ("CAU_KHOE", "CAU_DUY_TRI") and dspi > dspi_ma5 else 0.0)
        + (0.5 if 25 <= mpicp <= 78 and mpicp > mpicp_prev else 0.0)
    )
    stars = int(round(clip(raw_score, 0.0, 5.0)))
    star_delta = "TOT_HON" if raw_score > raw_score_prev + 0.25 else ("KEM_HON" if raw_score < raw_score_prev - 0.25 else "KHONG_DOI")

    # Tín hiệu tổng hợp Bullish Signal 1..4 & Bearish Signal 1..3 (TUYỆT ĐỐI KHÔNG CÓ BEARISH SIGNAL 4!)
    bull_sig, bear_sig = classify_qmv_action_signals(t, price_trend, bb_status, xh_cau, mpicp, uon_len_t_plus)
    return {
        "TMC": TMC[t], "TMA20": tma20, "TMA50": tma50, "bb_status": bb_status, "SP": sp_cum,
        "DSPI": (dspi, dspi_ma5, dspi_ma20, xh_cau), "MPICP": (mpicp, mpicp_ma5, mpicp_ma20),
        "DM": DM, "DMZ": DMZ, "EM03": EM[3], "norm_trap_flag": norm_trap_flag,
        "uon_len_t_plus": uon_len_t_plus, "uon_len_swing": uon_len_swing, "dao_chieu_tang_t_plus": dao_chieu_tang_t_plus,
        "price_trend": price_trend, "stars": stars, "star_delta": star_delta, "bull_sig": bull_sig, "bear_sig": bear_sig
    }
```

### 5.3. Tham số Tinh chỉnh
- `anchor_date = "2024-03-20"` (theo bản cập nhật V7 tháng 06/2026; cho phép tự động lùi `max(0, t - 500)` khi chạy Backtest 10 năm 2016–2026!); `demand_asymmetry = 1.15`; `norm_window = 50` phiên; `display_dmz_horizons = [3, 5, 10, 20]` (ẩn đường `200` ở chế độ chuẩn hóa đúng lời dặn của thầy Hào).

### 5.4. Hiển thị Trực quan trên `KLineChart`
- Thiết kế **Chế độ Sub-pane 3 Tầng Đồng Bộ QMV V7** (hoặc chuyển đổi bằng nút Tab gọn ngay góc trái pane):
  1. *Ngăn 1 (`Xu hướng Tiền Lớn`):* Đường `TMC` (đen đậm), `TMA20` (vàng `#FFC107`), `TMA50` (xanh dương `#2196F3`) + nhãn trạng thái `BB Status` & chấm giao cắt `BB Predict`.
  2. *Ngăn 2 (`Con lắc Cung Cầu & Tâm lý`):* Đường `DSPI [-1..+1]` (cam `#FF9800` kèm `MA5/MA20`) và `MPICP [0..100]` (tím `#AB47BC` quanh trục `50`).
  3. *Ngăn 3 (`Đa chu kỳ DMZ 03, 05, 10, 20`):* Băng vùng `< 20` (xanh lá nhạt — Duy trì) và `> 80` (đỏ nhạt — Bão hòa); nếu `norm_trap_flag == True`, phủ cảnh báo sọc xám **`CẢNH BÁO BẪY CHUẨN HÓA (XEM BẢN MẶC ĐỊNH!)`**.
- Trên thanh Header cổ phiếu hiển thị huy hiệu sao động: **`★★☆☆☆ (2 Sao) • TỐT HƠN ▲ • Bullish Signal 1`**.

---

## INDICATOR 6: `QMV_MARKET_BREADTH_BB` — HỘP ĐEN ĐỘ RỘNG THỊ TRƯỜNG / NGÀNH & ĐỒNG HỒ TÂM LÝ

### 6.1. Cơ sở Bài giảng QMV Gốc
- **Nguồn trích dẫn:** `V5 Guide` (tr.2, 14–19), `V7 Guide` (Hình 1.1), `Fintech Buổi 2` (`MBZ` điều kiện cần + `BBZ-M` điều kiện đủ trên rổ cổ phiếu thanh khoản $\ge 10.000$ cp/phiên; `Sentiment Gauge` đo tiền Rủi ro vs An toàn), và `Yumeko Postmortem Regime Journal` (`2026-10-15`: cảnh báo bẫy hồi ngắn hạn `BBM05 = 59%` trên nền áp lực bán dài hạn `BBS200 = 88.7%`!).

### 6.2. Công thức Toán học & Mã giả Từng Bước (`100% Causal Pseudocode`)

```python
def compute_qmv_market_breadth_at_bar(t, universe_ohlcv, sector_filter=None):
    # BƯỚC 1: Lọc vũ trụ cổ phiếu đủ thanh khoản giao dịch tại ngày t (SMA20(V) >= 10,000 cp/phiên)
    eligible = [
        sym for sym in universe_ohlcv
        if (sector_filter is None or sym.sector == sector_filter) and SMA(sym.V, 20, t) >= 10_000
    ]
    M_cnt = max(len(eligible), 1)

    # BƯỚC 2: Tính MBA (Tỷ lệ % mã ở vùng giá hấp dẫn/thấp so với lịch sử chính nó) & BBA-M, BBA-S, BBA-EM
    MBA, BBA_M, BBA_S, BBA_EM = {}, {}, {}, {}
    for k in (3, 5, 10, 20, 50, 200):
        # Lưu ý chiều biến thiên chuẩn QMV (Fintech Buổi 2): Thị trường rơi sâu -> đường MB đi XUỐNG vùng < 20 ("Hấp dẫn")!
        # Do đó MBA_k đo tỷ lệ % cổ phiếu đang nằm trên ngưỡng phân vị giá lịch sử k phiên (rơi đồng loạt -> MBA tụt < 20)
        in_upper_zone = sum(1 for s in eligible if price_percentile_causal(s.C, t, max(k * 3, 50)) >= 50.0)
        money_in_cnt  = sum(1 for s in eligible if s.proxy_DM[k][t] > 0)
        stock_out_cnt = sum(1 for s in eligible if s.proxy_DS[k][t] > 0)

        MBA[k]    = 100.0 * in_upper_zone / M_cnt
        BBA_M[k]  = 100.0 * money_in_cnt / M_cnt
        BBA_S[k]  = 100.0 * stock_out_cnt / M_cnt
        BBA_EM[k] = BBA_M[k] - BBA_S[k]

    # Chuẩn hóa lăn 50 phiên thành MBZ[k] và BBZ_M[k], BBZ_S[k], BBZ_EM[k] trên thang [0, 100]
    MBZ   = {k: minmax_50bar(MBA_series[k], t) for k in (3, 5, 10, 20, 50, 200)}
    BBZ_M = {k: minmax_50bar(BBA_M_series[k], t) for k in (3, 5, 10, 20, 50, 200)}

    # Tổ hợp xác nhận chắc chắn "CƠ HỘI T+ TOÀN THỊ TRƯỜNG" (MBZ 03,05 VÀ BBZ-M 03,05 đồng thuận uốn lên từ < 20..25)
    market_t_plus_go = (
        MBZ[3] > MBZ_prev[3] and MBZ_prev[3] < 25 and MBZ[5] > MBZ_prev[5] and MBZ_prev[5] < 25
        and BBZ_M[3] > BBZ_M_prev[3] and BBZ_M_prev[3] < 25 and BBZ_M[5] > BBZ_M_prev[5] and BBZ_M_prev[5] < 30
    )

    # Cờ Cảnh báo Hậu kiểm Yumeko: "Bẫy Độ Rộng Ngắn Hạn trên Nền Bán Dài Hạn Áp Đảo" (regime_journal 2026-10-15)
    structural_bear_breadth_trap = (BBA_M[5] >= 50.0) and (BBA_S[200] >= 80.0) and (MBA[200] <= 25.0)

    # BƯỚC 3: Sentiment Gauge (Tỷ trọng tiền chảy vào Nhóm Rủi Ro DT2+DT3 so với Nhóm An Toàn DT1)
    flow_risk = sum(max(0.0, s.dM[t]) for s in eligible if s.money_tier in ("DT2", "DT3"))
    flow_safe = sum(max(0.0, s.dM[t]) for s in eligible if s.money_tier == "DT1")
    sentiment_gauge = EMA_val(100.0 * flow_risk / max(flow_risk + flow_safe, 1e-9), 10, t)

    return {
        "MBA": MBA, "MBZ": MBZ, "BBA_M": BBA_M, "BBA_S": BBA_S, "BBZ_M": BBZ_M,
        "market_t_plus_go": market_t_plus_go, "structural_bear_breadth_trap": structural_bear_breadth_trap,
        "sentiment_gauge": sentiment_gauge
    }
```

### 6.3. Hiển thị & Tích hợp
- Hiển thị cặp đường đôi `MBZ(03, 05, 10, 20)` và `BBZ-M(03, 05, 10, 20)` kèm thanh ngang cảnh báo đỏ khi `structural_bear_breadth_trap == True` (*"BẪY HỒI NGẮN HẠN: BBS200 > 80% — CẤM BÌNH QUÂN GIÁ CỔ PHIẾU DƯỚI MA50!"*) và đồng hồ `Sentiment Gauge` (`Tiêu cực < 35 | Bình thường 35–65 | Hưng phấn > 65`).

---

## INDICATOR 7: `QMV_9BALL_ROTATION` — RADAR MA TRẬN LUÂN CHUYỂN `9 QUẢ BÓNG` & CẢNH BÁO "ĐỔI GÁC"

### 7.1. Cơ sở Bài giảng QMV Gốc
- **Nguồn trích dẫn:** `HP5 v1_3, v3_3, v3_4`, `KT01T4`, `DM01T2`, `VM01T3`, `Chọn CP Fintech 17/07/2025`.
- **Vấn đề giải quyết:** Không một công cụ kỹ thuật truyền thống nào trên thế giới thể hiện được cấu trúc **Ma trận 9 Quả Bóng (`3 Nhóm ngành × 3 Nhóm dòng tiền`)**, sự luân chuyển đan xen `1.1 -> 1.2 -> (1.3 + 2.1) -> ... -> 3.3`, **7 Cụm ngành đi đôi**, và đặc biệt là tín hiệu sinh tử **"Đổi gác" (`DT3` chững lại + tiền trú ẩn ngược về 3 quả bóng `1.1, 2.1, 3.1`)** báo hiệu nhà đầu tư cá nhân phải rút về `100%` tiền mặt!

### 7.2. Công thức Toán học & Mã giả Từng Bước (`100% Causal Pseudocode`)

```python
def compute_qmv_9ball_rotation_at_bar(t, universe_ohlcv, vnindex_close):
    # BƯỚC 1: Phân loại động Nhân quả mỗi cổ phiếu vào 3 Nhóm Ngành (NN1, NN2, NN3) x 3 Nhóm Dòng Tiền (DT1, DT2, DT3)
    # - NN1: Ngân hàng, Chứng khoán, BĐS, Xây dựng & VLXD
    # - NN2: Bán lẻ, Thực phẩm & ĐU, Hàng tiêu dùng, Hóa chất, Tài nguyên (Thép), SP & DV Công nghiệp, Dầu khí, Công nghệ
    # - NN3: Dịch vụ hạ tầng (Điện/Nước/Khí), Viễn thông, Truyền thông, Y tế, Bảo hiểm
    # - Phân tầng DT1 / DT2 / DT3 trong từng ngành theo phân vị lăn 60 phiên của (Vốn hóa / GTGD + Thị giá + Beta_60d):
    #   + DT1: VN30 hoặc Top 25% GTGD ngành & C >= 15,000đ & 0.8 <= Beta <= 1.55
    #   + DT3: Thị giá < 10,000đ (Trà đá) HOẶC Bottom 35% GTGD ngành HOẶC Beta > 2.1
    #   + DT2: Phần trung gian còn lại (Midcap thanh khoản tốt, 1.3 <= Beta <= 2.2)

    balls = {(g, d): [] for g in (1, 2, 3) for d in (1, 2, 3)}
    for s in universe_ohlcv:
        if s.passes_hard_anti_delisting_gate_at(t):
            balls[(s.sector_group, s.money_tier_at(t))].append(s)

    # BƯỚC 2: Tính Điểm Dòng Tiền Tương Đối & Tốc Độ %SP cho từng Quả Bóng (g, d) trong cửa sổ 5 & 20 phiên
    ball_metrics = {}
    mkt_ret5 = (vnindex_close[t] - vnindex_close[t-5]) / vnindex_close[t-5]
    for (g, d), stocks in balls.items():
        rel_ret5  = median([(s.C[t] - s.C[t-5]) / s.C[t-5] - mkt_ret5 for s in stocks]) if stocks else 0.0
        breadth_m = mean([1.0 if s.proxy_DM[5][t] > 0 else 0.0 for s in stocks]) * 100.0 if stocks else 0.0
        ceil_pct  = mean([1.0 if (s.C[t] / s.C[t-1] - 1.0) >= 0.062 else 0.0 for s in stocks]) * 100.0 if stocks else 0.0
        heat_score = clip(50.0 + 400.0 * rel_ret5 + 0.4 * (breadth_m - 50.0), 0.0, 100.0)
        ball_metrics[(g, d)] = {"heat": heat_score, "rel_ret5": rel_ret5, "breadth_m": breadth_m, "ceil_pct": ceil_pct}

    # BƯỚC 3: Nhận diện Pha Luân Chuyển Đan Xen, Cảnh Báo Đỉnh #1 (Trà đá đồng loạt trần) & Tín Hiệu #2 ("ĐỔI GÁC")
    dt3_heat_avg = mean([ball_metrics[(g, 3)]["heat"] for g in (1, 2, 3)])
    dt1_heat_avg = mean([ball_metrics[(g, 1)]["heat"] for g in (1, 2, 3)])
    dt3_ceil_avg = mean([ball_metrics[(g, 3)]["ceil_pct"] for g in (1, 2, 3)])

    # Cảnh báo Đỉnh #1: Tất cả các quả bóng DT3 (1.3, 2.3, 3.3) đồng loạt nóng rực / tím trần diện rộng!
    euphoria_peak_warning = all(ball_metrics[(g, 3)]["heat"] >= 68.0 for g in (1, 2, 3)) or (dt3_ceil_avg >= 18.0)

    # Tín hiệu Điều chỉnh Thực sự #2 ("ĐỔI GÁC"): Trước đó DT3 vừa nóng, nay DT3 suy yếu rơi nhanh trong khi cả 3 bóng DT1
    # (1.1, 2.1, 3.1) lại giữ giá vượt trội thị trường -> Tổ chức trú ẩn phòng thủ, CÁ NHÂN PHẢI CẦM 100% TIỀN MẶT!
    prior_dt3_hot = max(dt3_heat_avg_series[t-12:t]) >= 65.0
    dt3_stalling  = (dt3_heat_avg < dt3_heat_avg_prev3 - 8.0) and (dt3_heat_avg < 50.0)
    dt1_defensive = sum(1 for g in (1, 2, 3) if ball_metrics[(g, 1)]["rel_ret5"] > +0.01) >= 2
    guard_change_cash_alert = prior_dt3_hot and dt3_stalling and dt1_defensive

    # Gợi ý Quả bóng & Cụm ngành đi đôi đón đầu tiếp theo (Overlapping Lead-Lag)
    next_rotation_targets = infer_overlapping_next_balls_and_paired_sectors(ball_metrics)

    return {
        "ball_metrics": ball_metrics,
        "euphoria_peak_warning": euphoria_peak_warning,
        "guard_change_cash_alert": guard_change_cash_alert,
        "next_rotation_targets": next_rotation_targets
    }
```

### 7.3. Hiển thị Trực quan & Tích hợp Hệ thống
- Hiển thị **Ma trận Lưới `3 × 3` Quả Bóng Trực Quan** (kích thước vòng tròn tỷ lệ thuận quy mô hút tiền `NN1 > NN2 > NN3`, màu sắc chuyển từ Xám lạnh $\rightarrow$ Xanh ngọc khởi động $\rightarrow$ Vàng cam dẫn dắt $\rightarrow$ Đỏ tím quá nhiệt) ngay trong Dock/Sidebar của Sumi, kèm chuỗi đường cong lịch sử `DT1 vs DT2 vs DT3` ở `Sub-pane`.
- Khi `euphoria_peak_warning == True`: Hiển thị banner vàng **`CẢNH BÁO ĐỈNH SỐ 1: TOÀN BỘ DT3 / TRÀ ĐÁ ĐỒNG LOẠT CHẠY — SIẾT CHẶN LÃI!`**.
- Khi `guard_change_cash_alert == True`: Phát cảnh báo đỏ toàn hệ thống **`TÍN HIỆU ĐỔI GÁC (DT3 -> DT1): RÚT VỀ TIỀN MẶT — CẤM MUA PHÒNG THỦ!`** và tự động khóa mở lệnh mua mới của các Chiến lược **S1, S4, S7** trong Backtest Engine!
