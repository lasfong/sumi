# HỆ THỐNG CHIẾN LƯỢC GIAO DỊCH CỤ THỂ THEO PHƯƠNG PHÁP QMV
**(`02_QMV_TRADING_STRATEGIES_PLAYBOOK.md`)**

> **Nguyên tắc Kiến trúc Chiến lược & Chống Nhìn Trước Tương Lai (`100% Causal`):**
> Tài liệu này chuyển hóa toàn bộ học thuyết gốc của QMV thành **08 Chiến lược Giao dịch Hoàn chỉnh**, phủ kín mọi chế độ thị trường từ Tích lũy nền, Bắt đáy hoảng loạn có kiểm soát, Đánh sóng trung hạn, Đánh đột phá, Tiếp diễn sóng, Xoay vòng `T+2.5` trong Sideway, Luân chuyển 9 Quả Bóng / Sóng Trà đá đến Hệ thống Bán chủ động & Rebalancing Kho 6 Phần.
>
> Mỗi chiến lược được thiết kế song song trên **2 lớp**:
> 1. **Lớp 1 — Playbook Thực Chiến Cho Trader:** Điều kiện bối cảnh, phân lớp cổ phiếu (`DT1/DT2/DT3/Trà đá`), Checklist trước giờ giao dịch, chia vốn `1/3 – 1/3 – 1/3`, cắt lỗ/cấp cứu, chốt lời, lịch kiểm định `T+` và điều kiện hủy kịch bản (`Invalidation`).
> 2. **Lớp 2 — Đặc Tả Định Lượng Nhân Quả (`100% Causal Backtest Logic`):** Mọi biểu thức tại phiên đóng cửa `t` **chỉ sử dụng dữ liệu lịch sử $\tau \le t$** (tuyệt đối không dùng đỉnh/đáy tương lai $t+1..t+k$), tuân thủ nghiêm vi cấu trúc TTCK Việt Nam:
>    - **Lô giao dịch:** Làm tròn xuống bội số `100` cổ phiếu ($\lfloor Q / 100 \rfloor \times 100$).
>    - **Chu kỳ thanh toán `T+2` (chiều về `T+2.5`):** Cổ phiếu mua tại phiên $t_0$ bị khóa bán trong phiên $t_0$ và $t_0 + 1$, chỉ được phép đặt lệnh bán từ phiên **$t \ge t_0 + 2$**!
>    - **Biên độ & Khóa thanh khoản Trần/Sàn:** `HOSE ±7%`, `HNX ±10%`, `UPCOM ±15%` (tham chiếu UPCOM ngày $t+1$ xấp xỉ giá bình quân gia quyền $\approx \frac{H_t + L_t + 2C_t}{4}$). Không thể khớp lệnh Mua khi **Trần cứng mất thanh khoản** ($H_t = L_t$ và $C_t \ge P_{\text{ceil}, t} - \epsilon$); không thể khớp lệnh Bán/Stop-loss khi **Sàn cứng mất thanh khoản** ($H_t = L_t$ và $C_t \le P_{\text{floor}, t} + \epsilon$, lệnh bán phải dời sang phiên đầu tiên thoát sàn cứng!).
>    - **Chi phí ma sát chuẩn:** Phí mua $= 0.15\%$; Phí bán + Thuế TNCN $= 0.15\% + 0.10\% = 0.25\%$ (+ trượt giá `slippage` mặc định `0.10% – 0.20%` tùy thanh khoản `DT1/DT2/DT3`).

---

## QUY ƯỚC KÝ HIỆU TOÁN HỌC DÙNG CHUNG TẠI PHIÊN `t` (CAUSAL PRIMITIVES)

Để tránh lặp lại và đảm bảo lập trình viên Rust/TypeScript cài đặt thống nhất trên toàn bộ 8 chiến lược:
- Chuỗi nến cổ phiếu: $(O_t, H_t, L_t, C_t, V_t)$; chuỗi chỉ số thị trường chung (`VNINDEX`): $(O^M_t, H^M_t, L^M_t, C^M_t, V^M_t)$.
- Biên độ nến $R_t = H_t - L_t$; Thân nến $B_t = |C_t - O_t|$; Tỷ lệ thân $r^{\text{body}}_t = \frac{B_t}{\max(R_t, \epsilon)}$; Vị trí đáy thân $p^{\text{low}}_t = \frac{\min(O_t, C_t) - L_t}{\max(R_t, \epsilon)}$; Vị trí đỉnh thân $p^{\text{high}}_t = \frac{\max(O_t, C_t) - L_t}{\max(R_t, \epsilon)}$.
- Khóa trần cứng: $\text{CeilLock}_t \iff (H_t = L_t) \land (C_t > 1.065 \, C_{t-1})$; Khóa sàn cứng: $\text{FloorLock}_t \iff (H_t = L_t) \land (C_t < 0.935 \, C_{t-1})$ *(điều chỉnh hệ số tương ứng `1.095/0.905` cho HNX và `1.14/0.86` cho UPCOM)*.
- Bộ lọc chống "vẽ nến ảo ATO 100 cổ": Một cây nến xanh dài chỉ được công nhận thực chất (`ValidBullBar_t`) nếu không phải phiên mở cửa giá sàn rồi giao dịch thanh khoản thấp quanh tham chiếu: $\neg\big(O_t \le 0.936 \, C_{t-1} \land C_t \le 1.01 \, C_{t-1} \land V_t < 0.8 \, \text{SMA}_5(V)_t\big)$.
- Đường trung bình khối lượng chuẩn QMV: $\overline{V}_{5,t} = \text{SMA}(V, 5)_t$; Thanh khoản cơ sở: $\overline{V}_{20,t} = \text{SMA}(V, 20)_t \ge 50{,}000$ cp/phiên (tối thiểu $\ge 10{,}000$ với Trà đá ở nền).
- Pivot Đáy/Đỉnh Nhân quả bậc $m$ (ví dụ $m = 3$, xác nhận trễ đúng $m$ phiên tại thời điểm $t$, không nhìn trước tương lai!):
  $$\text{PivotLow}_m(t - m) \iff L_{t-m} = \min_{\tau \in [t-2m,\, t]} L_\tau, \qquad \text{PivotHigh}_m(t - m) \iff H_{t-m} = \max_{\tau \in [t-2m,\, t]} H_\tau$$

---

## CHIẾN LƯỢC S1: MUA TÍCH LŨY NỀN & HỖ TRỢ CẠN CUNG TẠI ĐIỂM CHẠM 2–3 CHIA 3 PHẦN
**(`QMV Support & Test-Cung Core Swing` — Nguồn gốc: `KT01T2`, `DM01T4`, `DM01_BUỔI_4`, `HP4 v2_1, v3_2`)**

### Lớp 1 — Playbook Thực Chiến Cho Trader
1. **Bối cảnh Vĩ mô / VN-Index / Ngành cho phép đánh:**
   - Vĩ mô không ở Pha 4 Thắt chặt; `VN-Index` trả lời câu hỏi *"Thị trường có xấu không?"* = **Không xấu** (không nằm trong cấu trúc `LH + LL` dưới `SMA50` dốc xuống).
   - Cổ phiếu đang ở **`Hộp 0` (nền tích lũy gốc)** hoặc **`Hộp 2` (nhịp chỉnh đầu tiên `1 -> 2` trong sóng tăng, lùi `<= 1/3` đến `1/2` nhịp tăng trước)**; hạn chế áp dụng ở `Hộp 4` nếu biên lợi nhuận lên đỉnh cũ hẹp.
2. **Loại cổ phiếu phù hợp & Cấm kỵ:**
   - **Tối ưu nhất:** Nhóm **`DT2` (Midcap 3–4 sao, thanh khoản tốt)** và **`DT1`** thuộc các ngành đang/chuẩn bị có Key Driver (`NN1`, `NN2`).
   - **Cấm áp dụng:** Cổ phiếu vi phạm cổng "Không được chết", cổ phiếu đang rơi `Giá giảm + Vol tăng` (*Ba con quạ đen*), hoặc cổ phiếu vừa gãy phân phối đỉnh với `Volume` khổng lồ chưa tích lũy đủ tối thiểu `1 tháng (20 phiên)`.
3. **Checklist Điều kiện Cần & Đủ trước giờ đặt lệnh:**
   - [x] **Vùng Hỗ trợ ngang rõ ràng:** Trùng đáy gần nhất hoặc đỉnh cũ vừa vượt có thay đổi cơ bản (`Kháng cự -> Hỗ trợ`).
   - [x] **Quá trình giảm về hỗ trợ "Bình thường":** `Giá giảm + Khối lượng giảm dần` ($\overline{V}_{5,t}$ dốc xuống về ngang nền thấp trước sóng), biên độ giảm co hẹp dần (`< 1%–1.5%/phiên`), sau đó **`Giá dừng giảm + Lượng dừng giảm`**.
   - [x] **Bài toán `Reward / Risk > 1`:** Khoảng cách từ giá mua lên vùng kháng cự gần nhất lớn hơn khoảng cách xuống râu thấp nhất vùng hỗ trợ (lý tưởng `RR >= 1.5`).
   - [x] **Chỉ báo ủng hộ:** `RSI(14)` hoặc `CCI(14)` về vùng quá bán / tương đương đáy hỗ trợ trước (hoặc tạo Phân kỳ dương), giá chui từ ngoài biên dưới `BB(15, 2)` vào trong; nếu có Webapp QMV: `BB Status = Duy trì / Khỏe` và `DMZ(03, 05)` uốn lên từ `< 20..30`.
4. **Kịch bản Giải ngân `1/3 – 1/3 – 1/3` (Khớp lệnh lúc `14:15–14:30` hoặc `ATC`):**
   - **Phần 1 (`1/3` vốn mã — Thăm dò):** Chỉ vào tại **Điểm chạm 1** nếu cổ phiếu đang ở `Hộp 2` của một Uptrend khỏe và xuất hiện nến rút chân `Spring` trên khung `30m/1D` *(nếu bối cảnh chung Sideway yếu thì bỏ qua Chạm 1, để Phần 1 sang Điểm chạm 2!)*.
   - **Phần 2 (`1/3` vốn mã — Chủ lực "Test cung"):** Giải ngân tại **Điểm chạm số 2 (hoặc số 3)** khi giá test lại vùng hỗ trợ không thủng đáy Chạm 1 (hoặc nhúng `Spring` rút chân đóng cửa trên hỗ trợ) + xuất hiện **Nến tăng / Cụm ghép 2–3 nến đóng cửa ở `1/3 Trên` + `Volume` nhích vượt `SMA(Vol, 5)`**.
   - **Phần 3 (`1/3` vốn dự phòng):** Giải ngân nốt khi giá bứt lên vượt đỉnh nhịp nảy giữa hai đáy (xác nhận mẫu hình 2 Đáy / Hộp đáy nhỏ) mà vẫn đảm bảo `RR > 1`.
5. **Cắt lỗ, Cấp cứu Vị thế, Chốt lời & Kiểm định `T+`:**
   - **`Stop-Loss` gốc:** Đặt tại `Lowest Low` (râu nến thấp nhất) của toàn bộ vùng hỗ trợ.
   - **Phác đồ Cấp cứu:** Nếu đã vào `2/3` mà đóng nến đỏ dài xuyên thủng `Stop-Loss` kèm `Vol` bán tăng $\rightarrow$ **bán cắt ngay `1/2` vị thế (`= 1/3` vốn)** để thu hồi `2/3` tiền mặt; nếu phần `1/3` còn lại không hồi lại vào nền sau `T+1` thì đóng nốt 100%.
   - **Chốt lời & Chặn lãi:** Mua vì Hỗ trợ $\rightarrow$ lên chạm **Vùng Kháng cự ngang kế tiếp** chốt trước `1/3` (hoặc `1/2` ở điểm chạm đỉnh thứ 2 trên `30m`); phần còn lại nâng `Trailing Stop` lên mốc **`2/3` quãng tăng** để gồng nếu vượt đỉnh.
   - **Lịch `T+` & `Invalidation`:** Sau khi giải ngân Phần 2 (điểm uốn chính), nếu qua **`T+3` đến `T+4` (4 phiên)** giá không bật lên khỏi vùng nền hỗ trợ mà lình xình kiệt cầu $\rightarrow$ kích hoạt **`Time-Stop` thoát vị thế hòa vốn/lỗ nhẹ**!

### Lớp 2 — Công thức Định lượng Hóa 100% Nhân quả (`Quant / Backtest Logic`)
Tại thời điểm đóng nến ngày $t$ (khớp lệnh tại $C_t$ nếu $\neg \text{CeilLock}_t$):
1. **Market Gate (`VN-Index` không Downtrend):**
   $$G^{\text{mkt}}_t \iff \big(C^M_t > \text{SMA}_{50}(C^M)_t\big) \;\lor\; \big(\text{SMA}_{50}(C^M)_t \ge \text{SMA}_{50}(C^M)_{t-5}\big) \;\lor\; \big(C^M_t > \text{SMA}_{20}(C^M)_t\big)$$
2. **Xác định Vùng Hỗ trợ Ngang Gần nhất $S_{\text{zone}}(t) = [S_{\text{low}}, S_{\text{high}}]$ và Kháng cự Kế tiếp $R_{\text{next}}(t)$:**
   - Quét lùi cửa sổ $[t - 60,\, t - 5]$ tìm các điểm đáy nhân quả $\text{PivotLow}_3(\tau)$ đã xác nhận ($\tau \le t - 3$). Chọn cụm đáy gần mức giá hiện tại nhất sao cho $S_{\text{low}} = \min_{\tau \in \mathcal{C}} L_\tau$ và $S_{\text{high}} = \max_{\tau \in \mathcal{C}} \min(O_\tau, C_\tau)$ (hoặc biên độ khoan dung $S_{\text{high}} = S_{\text{low}} \times 1.025$).
   - $R_{\text{next}}(t) = \min\big\{ H_\tau : \tau \in [t-90, t-3],\; \text{PivotHigh}_3(\tau),\; H_\tau > 1.04 \, C_t \big\}$ (nếu đang gần đỉnh cao 90 phiên thì lấy $R_{\text{next}}(t) = S_{\text{low}} + \Delta P_{\text{prior\_leg}}$).
3. **Điều kiện VPA "Giảm bình thường & Cạn cung tại Hỗ trợ" + Đếm Điểm Chạm $\ge 2$:**
   - Tồn tại ít nhất 1 lần chạm trước đó $\tau_1 \in [t - 25,\, t - 3]$ có $L_{\tau_1} \in [0.99 \, S_{\text{low}},\, 1.015 \, S_{\text{high}}]$ sau đó nảy lên $> S_{\text{high}} \times 1.015$, và hiện tại trong $[t - 2,\, t]$ giá quay lại kiểm định lần thứ 2 hoặc thứ 3: $\min_{j \in [t-2, t]} L_j \in [0.985 \, S_{\text{low}},\, 1.015 \, S_{\text{high}}]$ với $C_t \ge S_{\text{low}}$.
   - Quá trình lùi cạn cung: $\min_{j \in [t-3, t-1]} \overline{V}_{5, j} < 0.85 \, \overline{V}_{20, t}$ VÀ không có phiên xả bất thường trong 3 phiên trước ($\forall j \in [t-3, t-1]: \neg(C_j < C_{j-1} - 0.03 \, C_{j-1} \land V_j > 1.3 \, \overline{V}_{20, j})$).
4. **Tín hiệu Kích hoạt (`Trigger_S1(t)`) & Tỷ lệ `RR`:**
   - Nến đơn hoặc Nến ghép $K \in \{1, 2, 3\}$ phiên kết thúc tại $t$ tạo **Spring / Hammer ở `1/3 Trên`** hoặc **Nến Xanh vượt `50%` thân nến đỏ liền trước**:
     $$\text{CandleTrig}_t \iff \Big(p^{\text{low}}_{\text{merge}(K), t} \ge \frac{2}{3} \;\land\; C_t > C_{t-1}\Big) \;\lor\; \Big(C_t > O_t \;\land\; r^{\text{body}}_t \ge 0.5 \;\land\; C_t > \frac{O_{t-1} + C_{t-1}}{2} \;\land\; \text{ValidBullBar}_t\Big)$$
   - Cầu vào + Chỉ báo uốn + `RR > 1.2`:
     $$V_t \ge \max\big(V_{t-1},\; 0.95 \, \overline{V}_{5, t}\big) \;\land\; \big(\text{RSI}_{14,t} > \text{RSI}_{14,t-1} \land \text{CCI}_{14,t} > \text{CCI}_{14,t-1}\big) \;\land\; \frac{R_{\text{next}}(t) - C_t}{C_t - 0.995 \, S_{\text{low}}} > 1.2$$
   - *(Tùy chọn bật thêm QMV Proxy Gate: $\text{TMC}_t \ge \text{TMA50}_t \lor \Delta \text{TMC}_t > 0$, và $\text{DMZ}_{03,t}, \text{DMZ}_{05,t}$ vừa uốn lên từ $< 30$).*
5. **Luật Đi Vốn & Đóng Vị Thế Nhân Quả:**
   - **Tranche 1 (`1/3` ngân sách mã, mặc định `10% NAV / 3 = 3.33% NAV`):** Khớp tại $C_t$ khi thỏa Chạm 1 trong Uptrend ($C_t > \text{SMA}_{50,t} > \text{SMA}_{200,t}$) hoặc tại Chạm 2 đầu tiên.
   - **Tranche 2 (`1/3`):** Khớp khi thỏa trọn vẹn `Trigger_S1(t)` tại Điểm chạm $\ge 2$.
   - **Tranche 3 (`1/3`):** Khớp khi $C_t > \max_{\tau \in [\tau_1, t-1]} H_\tau$ (vượt đỉnh nhịp nảy giữa 2 đáy) với $V_t \ge 1.2 \, \overline{V}_{5,t}$ và $\text{RR}_t > 1.0$.
   - **Cấp cứu & Stop-Loss (chỉ áp dụng cho các lô đã đủ $t \ge t_{\text{buy}} + 2$ và $\neg \text{FloorLock}_t$):**
     - Nếu đang giữ $\ge 2$ tranches và $C_t < S_{\text{low}}$ kèm $r^{\text{body}}_t \ge 0.5 \land C_t < O_t$: Bán ngay `1/2` tổng khối lượng khả dụng tại $C_t$; nếu $C_{t+1} < S_{\text{low}}$, bán nốt phần còn lại.
     - **Time-Stop (`Setup Expiry`):** Tại $t = t_{\text{tranche2}} + 4$, nếu $C_t \le 1.01 \, P_{\text{entry,2}}$ và $H_{[t-3..t]} < P_{\text{entry,2}} + 0.3(R_{\text{next}} - P_{\text{entry,2}})$ $\rightarrow$ Đóng 100% vị thế.
     - **Take-Profit & Trailing Stop:** Khi $H_t \ge 0.99 \, R_{\text{next}}$, nếu xuất hiện nến `Upthrust` ($p^{\text{high}}_t \le 1/3$) hoặc $C_t < O_t$ kèm $V_t > 1.25 \, \overline{V}_{5,t}$ $\rightarrow$ Bán `1/2`; phần còn lại giữ theo chặn lãi động $C_t < \text{TrailingStop}_{1D}(t) = L_{\text{base}} + \frac{2}{3}(H_{\text{peak}, t} - L_{\text{base}})$.

---

## CHIẾN LƯỢC S2: KỸ THUẬT BẮT ĐÁY AN TOÀN ĐA KHUNG (`WEEKLY EMA200` + `ĐÁY 3 PHÂN KỲ MACD` + `"KILL NẾN ĐỎ"`)
**(`Safe Institutional Bottom-Fishing` — Nguồn gốc: `wa_10_02_2026_ky_thuat_bat_day_an_toan_audio.mp3.md`, `KT01T2`, `Postmortem Cổng 7`)**

### Lớp 1 — Playbook Thực Chiến Cho Trader
1. **Bối cảnh & Triết lý (`Thiên thời 30% + Địa lợi 30% + Nhân hòa 30% = 90%`):**
   - Dùng khi thị trường hoặc nhóm ngành vừa trải qua một đợt bán tháo chiết khấu sâu (`-25%` đến `-35%+` từ đỉnh), đám đông bi quan cực độ. Nếu *Thiên thời* (thanh khoản OMO/tín dụng) chưa thuận ("nước chảy ngược"), bắt buộc **hạ kỳ vọng xuống đánh nhịp hồi về kháng cự/đỉnh cũ gần nhất**, không mơ sóng x2!
2. **Loại cổ phiếu BẮT BUỘC (`Địa lợi` — Nghiêm cấm áp dụng cho hàng rác!):**
   - **Chỉ áp dụng cho cổ phiếu cơ bản tốt đầu ngành (`DT1`, `DT2` chất lượng 4–5 sao, rổ `QMV30 / QMV60` như `MBB, VCB, TCB, DGC, VCI, SSI, MWG`)** có doanh thu/LN không suy thoái 4 quý liên tiếp (loại `DBC` pha suy thoái), `CFO > 0` đồng pha lợi nhuận (loại `NVL, PDR, KBC`), `M-Score < -1.78`.
3. **6 Trường hợp TUYỆT ĐỐI CẤM Bắt Đáy ("Bắt dao rơi tử vong cao"):**
   - (1) Đang rơi với nến đỏ dài *Ba con quạ đen* + `Vol` bán tăng dần; (2) Giá còn lơ lửng quá xa phía trên `EMA(200)` khung Tuần; (3) Mới chỉ là **nhịp giảm đầu tiên (Đáy 1)**; (4) Đường `MACD` ngày đang cắm thẳng xuống tạo đáy thấp hơn cùng giá; (5) Bắt xanh/tím giữa phiên trước `14:15`; (6) Bắt đáy sự kiện xấu bất ngờ mà định ôm dài hạn.
4. **Checklist `4 Điều kiện CẦN + 1 Điều kiện ĐỦ` (`Nhân hòa`):**
   - [x] **ĐK Cần 1 (Vị trí Giá trị Tổ chức):** Giá đã chiết khấu $\ge 20\%–30\%$ từ đỉnh quý/năm và lùi về **tiệm cận sát ($\le +6\%$) hoặc nhúng xuyên xuống dưới đường `EMA(200)` khung Tuần** (hoặc `P/B` ngân hàng tiến sát mỏ neo `1.0x – 1.2x`).
   - [x] **ĐK Cần 2 (Đủ 2–3 Nhịp Giảm):** Đã hình thành **Đáy số 2 hoặc chuẩn nhất là Đáy số 3** (`Đáy 1 -> Hồi -> Đáy 2 sâu hơn -> Hồi cản cũ -> Đáy 3 thấp hơn Đáy 2 một chút`) kèm quá trình cạn cung trước điểm nổ.
   - [x] **ĐK Cần 3 (Phân kỳ Dương `MACD` Khung Ngày rõ rệt):** Giá tạo Đáy sau thấp hơn Đáy trước ($L_3 \le L_2$), nhưng đường **`MACD(12,26,9)` và `MACD Histogram` tạo Đáy sau CAO HƠN Đáy trước rõ ràng**!
   - [x] **ĐK Cần 4 (`Blackbox` Chụm Đáy Uốn Lên):** Các đường chu kỳ tiền `DM03, DM05, DM10, DM20` cùng chụm sát vùng thấp và đồng loạt uốn ngược lên (`TMC` ngừng rơi thẳng đứng, chuyển sang đi ngang).
   - [x] **ĐK ĐỦ (`Trigger` lúc `14:15–14:30` hoặc `ATC`):** Xuất hiện một trong ba cò súng nến xác nhận kèm **`Volume` tăng cao hơn các nến giảm liền trước**:
     1. **`Bullish Engulfing`** (thân & biên độ nến xanh trùm trọn nến đỏ trước);
     2. **Cụm 3 nến đáy** (`Đỏ mạnh -> Nến chững Hammer/Doji -> Nến 3 Xanh đóng cửa > 50% thân Nến 1` — tuyệt đối không mua sớm ở Nến 2!);
     3. **Quy tắc `"Kill Nến Đỏ"`** (nếu hôm trước mới là `Piercing Line` lửng lơ thì chờ nến xanh hôm nay đóng cửa vượt hẳn `High` của cây nến đỏ trước đó).
5. **Đi Vốn `1/3 – 1/3 – 1/3` & Quy trình Cấp cứu Bắt Nhầm Đáy Giả:**
   - **Vào `1/3` đầu tiên** ngay cuối phiên có ĐK Đủ; **giữ nguyên** khi bật tăng và chỉ vào **`1/3` tiếp theo** khi giá vượt cản ngắn hạn rồi **retest thành công** với cụm nến tăng mới.
   - **Nếu lỡ bắt sai ở Đáy giả** (MACD mới nhíu nhẹ chưa phân kỳ chuẩn, giá sập tiếp): Nếu vào thêm `1/3` ở nhịp nảy dưới thì ngay khi hồi chạm giá vốn lệnh 1 gặp nến đảo chiều giảm (`Bearish Engulfing / Dark Cloud / Kill nến xanh`) **phải thoát sạch hòa vốn ngay**! Nếu không kịp thoát ở đỉnh hồi mà giá đóng cửa **thủng đáy cây nến hỗ trợ**, lập tức **cắt `1/2` vị thế (`= bán 1/3`)**, giữ lại `2/3` sức mua tiền mặt chờ Đáy 3 thật!

### Lớp 2 — Công thức Định lượng Hóa 100% Nhân quả (`Quant / Backtest Logic`)
1. **Tính `Weekly EMA(200)` nhân quả tại ngày $t$:**
   - Gộp chuỗi ngày thành nến Tuần kết thúc vào Thứ Sáu gần nhất $\le t$ (hoặc cập nhật lũy tiến bao gồm phiên $t$ cho tuần hiện hành): $E^{\text{wk}}_{200, t} = \text{EMA}_{200}(C^{\text{weekly}})_t$.
   - Điều kiện chiết khấu & vùng bệ đỡ tổ chức:
     $$\text{Drawdown}_{120, t} = \frac{\max_{\tau \in [t-120, t]} H_\tau - L_t}{\max_{\tau \in [t-120, t]} H_\tau} \ge 0.20 \quad \land \quad \frac{L_t}{E^{\text{wk}}_{200, t}} \in [0.82,\; 1.06]$$
2. **Đếm $\ge 2$ Nhịp giảm (Chuẩn 3 Đáy) & Phân kỳ Dương `Daily MACD(12, 26, 9)`:**
   - Tìm chuỗi đáy nhân quả trong cửa sổ $[t - 75,\, t - 1]$: Tồn tại ít nhất đáy trước $\tau_a \in [t - 65,\, t - 8]$ (đã xác nhận $\text{PivotLow}_3(\tau_a)$, tốt nhất trước $\tau_a$ đã có thêm $\tau_0 < \tau_a$ để $\tau_{\text{curr}}$ là Đáy số 3) và đáy hiện tại $\tau_b \in [t - 4,\, t]$ sao cho:
     $$L_{\tau_b} \le 1.01 \, L_{\tau_a}, \qquad \text{MACD}_{\tau_b} > \text{MACD}_{\tau_a} + 0.05 \, |\text{MACD}_{\tau_a}|, \qquad \text{Hist}_t > \text{Hist}_{t-1}$$
3. **Điều kiện Đủ — Cò súng Nến `"Kill Nến Đỏ"` / `Engulfing` / `3-Bar Star` + Cầu Tổ chức:**
   - Định nghĩa cây nến đỏ tham chiếu gần nhất $r \in \{t-1, t-2\}$ có $C_r < O_r$:
     - $\text{Engulf}_t \iff (C_{t-1} < O_{t-1}) \land (C_t > O_{t-1}) \land (O_t \le C_{t-1}) \land (H_t \ge H_{t-1})$;
     - $\text{Star3}_t \iff (C_{t-2} < O_{t-2}) \land (r^{\text{body}}_{t-2} \ge 0.5) \land (r^{\text{body}}_{t-1} \le 0.35) \land \Big(C_t > \frac{O_{t-2} + C_{t-2}}{2}\Big) \land (C_t > O_t)$;
     - $\text{KillRed}_t \iff (C_r < O_r) \land (C_t > H_r) \land \text{ValidBullBar}_t$.
   - Kích hoạt mua Phần 1 (`1/3`):
     $$\text{Entry\_S2}_1(t) \iff \text{QualityGate} \;\land\; \text{WeeklyZone}_t \;\land\; \text{MACDDiv}_t \;\land\; (\text{Engulf}_t \lor \text{Star3}_t \lor \text{KillRed}_t) \;\land\; \Big(V_t > \max_{j \in [t-3, t-1]: C_j < O_j} V_j\Big)$$
4. **Gia tăng, Cấp cứu & Chốt lời Nhân quả:**
   - **Phần 2 (`1/3`):** Khi giá vượt đỉnh trung gian $R_{\text{mid}} = \max_{\tau \in [\tau_a, \tau_b]} H_\tau$ tại $t_k > t_{\text{buy1}}$, sau đó tại $t > t_k + 1$ lùi test $L_t \in [0.985 \, R_{\text{mid}}, 1.02 \, R_{\text{mid}}]$ không thủng và đóng nến xanh $C_t > H_{t-1}$ ("Kill nến đỏ chỉnh").
   - **Stop-Loss & Cắt `1/2`:** Mốc đáy bảo vệ $S_{\text{bot}} = \min_{j \in [\tau_b - 1, t_{\text{buy1}}]} L_j$. Nếu $t \ge t_{\text{buy}} + 2$ và $C_t < S_{\text{bot}}$: nếu đang giữ $\ge 2$ phần thì bán ngay `1/2` tại $C_t$ và đóng nốt nếu $C_{t+1} < S_{\text{bot}}$; nếu chỉ giữ `1/3` và $C_t < 0.985 \, S_{\text{bot}}$ kèm $V_t > \overline{V}_{5,t}$ thì đóng 100%.
   - **Target:** Nếu `VN-Index` dưới `SMA50` ("Thiên thời nghịch"), chốt `1/2` ngay tại kháng cự đỉnh cũ gần nhất $R_{\text{mid}}$ và `1/2` tại `SMA50` cổ phiếu; nếu `VN-Index` thuận (`> SMA50`), gồng theo `TrailingStop` `2/3` sóng.

---

## CHIẾN LƯỢC S3: ĐÁNH THEO XU HƯỚNG TRUNG HẠN & BẪY GẤU ICHIMOKU / BOLLINGER
**(`QMV Medium-Term Trend & Ichimoku Bear-Trap` — Nguồn gốc: `HP4 v1_3, v2_3_2, v3_3_bg`, `V7 Guide`, `KT01T2/T3`)**

### Lớp 1 — Playbook Thực Chiến Cho Trader
1. **Bối cảnh & Cổ phiếu phù hợp:**
   - Dành cho giai đoạn thị trường và ngành **Trending rõ nét** (Pha 1 Nới lỏng `3/3` hoặc Pha 2 Mở rộng `2/3`), nơi giảng viên QMV chứng minh các hệ thống bám trung bình động cho hiệu suất thắng áp đảo nhất và không bị rụng hàng non bởi chỉ báo quá mua.
   - **Cổ phiếu phù hợp:** `DT2` dẫn dắt và `DT1` tăng trưởng (như `HPG, FPT` pha tăng, `MWG, TCB, MBB, SSI, VCI`), `BB Status = Tiền khỏe` (`TMC > TMA20 > TMA50`, `TMA50` dốc lên). **Cấm dùng** khi cổ phiếu đi ngang trong mây hẹp hoặc `MA20, MA50, MA200` xoắn bện phẳng lì!
2. **Hai Điểm Vào Chuẩn QMV trong Sóng Trung Hạn:**
   - **Setup 3A (Chuyển pha Xuyên Mây + Golden Cross):** Giá tạo điểm uốn chui vào đám mây rồi **đóng cửa vượt hẳn lên bờ trên đám mây `Ichimoku QMV (6, 18, 36, 18)`**, đồng thời `SMA(20) > SMA(50)` (hoặc `SMA(50)` cắt lên `SMA(200)`), mây tương lai 18 phiên mở rộng màu xanh và `Chikou` thoát thoáng khỏi mây.
   - **Setup 3B (Đặc sản Bẫy Giảm `Ichimoku Bear-Trap` & Rũ Quá Bán trong Uptrend — Tỷ lệ thắng cực cao!):** Cổ phiếu đang trong Uptrend trung hạn vững chắc (**Giá nằm trên đám mây Kumo** và `SMA(50)` dốc lên), gặp nhịp rũ bỏ ngắn hạn đạp giá **xuyên xuống dưới đường cơ sở `Kijun(18)`** (hoặc rớt chớp nhoáng chạm biên dưới `BB(15, 2)` / `RSI` lùi về `40–50`), sau đó lực cầu kéo bật **đóng cửa vượt ngược lên trên `Tenkan(6)` và `Kijun(18)`**!
3. **Đi Vốn, Nắm Giữ ("Let It Run") & Quy tắc Chốt Lời Không Bán Non:**
   - Giải ngân **2 nhịp** (`HP4 v1_3` & `HP5`): `1/3` khi chớm chui mây / rút chân tại mây hỗ trợ; `2/3` khi xác nhận vượt hẳn bờ trên mây (Setup 3A) hoặc vượt ngược `Tenkan/Kijun` xác nhận Bear-Trap (Setup 3B). *(Nếu tài khoản tiền thịt đã có lãi bền vững trong Pha 1 Nới lỏng, đây là chiến lược duy nhất cho phép bồi thêm `Margin` trên chính mã mạnh này!).*
   - **Thiết quân luật Giữ Hàng:** Chừng nào giá còn bám biên trên `BB(15, 2)` hoặc `RSI > 70` mà chưa gãy cấu trúc $\rightarrow$ **TUYỆT ĐỐI HOÃN BÁN (`Let it run`)**!
   - **Điểm Bán nhất quán theo đúng công cụ vào lệnh:**
     - *Bán `1/3` cảnh báo:* Giá vọt ra ngoài biên trên `BB(15, 2)` rồi **chui ngược vào trong dải lần thứ 2** kèm phân kỳ âm `RSI + CCI`.
     - *Bán `2/3` thoát sóng trung hạn:* Giá đóng cửa gãy thủng `Kijun(18)` mà sau 2 phiên không lấy lại được, HOẶC đóng cửa xuyên thủng bờ dưới đám mây Kumo / `SMA(50)` kèm `TMC` cắt xuống `TMA20`!

### Lớp 2 — Công thức Định lượng Hóa 100% Nhân quả (`Quant / Backtest Logic`)
1. **Tính Hệ thống `Ichimoku QMV (Tenkan=6, Kijun=18, SenkouB=36, Disp=18)` nhân quả tại $t$:**
   - $T_{6,t} = \frac{\max_{[t-5, t]} H + \min_{[t-5, t]} L}{2}$; $\quad K_{18,t} = \frac{\max_{[t-17, t]} H + \min_{[t-17, t]} L}{2}$; $\quad SB^{\text{raw}}_{36,t} = \frac{\max_{[t-35, t]} H + \min_{[t-35, t]} L}{2}$.
   - **Đám mây hiện tại tại vị trí nến $t$** (được phóng chiếu từ 18 phiên trước — 100% nhân quả!):
     $$\text{SpanA}^{\text{now}}_t = \frac{T_{6, t-18} + K_{18, t-18}}{2}, \qquad \text{SpanB}^{\text{now}}_t = SB^{\text{raw}}_{36, t-18}$$
     $$\text{CloudTop}_t = \max(\text{SpanA}^{\text{now}}_t, \text{SpanB}^{\text{now}}_t), \qquad \text{CloudBot}_t = \min(\text{SpanA}^{\text{now}}_t, \text{SpanB}^{\text{now}}_t)$$
   - **Đám mây tương lai đầu mút** (tính từ giá trị ngay tại $t$ — 100% nhân quả!):
     $$\text{SpanA}^{\text{fut}}_t = \frac{T_{6,t} + K_{18,t}}{2}, \qquad \text{SpanB}^{\text{fut}}_t = SB^{\text{raw}}_{36,t}, \qquad \text{BullCloudFut}_t \iff \text{SpanA}^{\text{fut}}_t > \text{SpanB}^{\text{fut}}_t \;\land\; (\text{SpanA}^{\text{fut}}_t - \text{SpanB}^{\text{fut}}_t) > (\text{SpanA}^{\text{fut}}_{t-3} - \text{SpanB}^{\text{fut}}_{t-3})$$
   - **Chikou thoáng mây quá khứ (lùi 18 phiên):** $C_t > \max(H_{t-18},\; \text{CloudTop}_{t-18})$.
2. **Tín hiệu Mở Vị Thế Nhân Quả:**
   - **Setup 3A (Break Cloud):**
     $$\text{Trig\_S3A}_t \iff G^{\text{mkt}}_t \;\land\; (C_{t-1} \le \text{CloudTop}_{t-1}) \;\land\; (C_t > \text{CloudTop}_t) \;\land\; (T_{6,t} \ge K_{18,t}) \;\land\; \text{BullCloudFut}_t \;\land\; (C_t > H_{t-18}) \;\land\; (\text{SMA}_{50,t} > \text{SMA}_{50,t-5}) \;\land\; (V_t \ge 1.2 \, \overline{V}_{5,t})$$
   - **Setup 3B (Ichimoku Bear-Trap trong Uptrend):**
     $$\text{Trig\_S3B}_t \iff G^{\text{mkt}}_t \;\land\; (C_t > \text{CloudTop}_t) \;\land\; (\text{SMA}_{50,t} > \text{SMA}_{200,t}) \;\land\; (\text{SMA}_{50,t} > \text{SMA}_{50,t-10}) \;\land\; \Big(\exists j \in [t-4, t-1]: C_j < K_{18,j} \;\land\; L_j \ge 0.99 \, \text{CloudBot}_j\Big) \;\land\; \big(C_t > \max(T_{6,t}, K_{18,t})\big) \;\land\; (C_t > O_t)$$
3. **Luật Giữ & Thoát Vị Thế Nhân Quả:**
   - **Cấm bán non:** Bỏ qua mọi tín hiệu `RSI > 70` chừng nào $C_t \ge T_{6,t}$.
   - **Chốt `1/3` (BB(15, 2) Re-entry lần 2):** Xảy ra khi trong 15 phiên qua đã có 2 nhịp tách biệt mà $H_\tau > \text{BBUpper}_{15,2}(\tau)$ rồi đóng cửa $C_\tau < \text{BBUpper}_{15,2}(\tau)$ kèm $\text{RSI}_{14,t} < \text{RSI}_{14,\text{peak1}}$.
   - **Thoát toàn bộ (`Exit_S3`):** Khi $t \ge t_{\text{buy}} + 2$ và thỏa:
     $$\big(C_t < \text{CloudBot}_t\big) \;\lor\; \big(C_t < \text{SMA}_{50,t} \land C_{t-1} < \text{SMA}_{50,t-1}\big) \;\lor\; \big(C_t < K_{18,t} \land T_{6,t} < K_{18,t} \land V_t > 1.25 \, \overline{V}_{5,t}\big)$$

---

## CHIẾN LƯỢC S4: ĐÁNH ĐỘT PHÁ VÙNG TẮC NGHẼN / TAM GIÁC / PHỄU GIẢM CÓ KIỂM ĐỊNH `T+3 TIME-STOP`
**(`QMV Validated Breakout & Retest Momentum` — Nguồn gốc: `HP4 v2_4, v2_5, v3_5`, `KT01T2`, `Fintech V5/V7`)**

### Lớp 1 — Playbook Thực Chiến Cho Trader
1. **Bối cảnh & Loại cổ phiếu:**
   - **Bắt buộc thị trường chung ủng hộ** (`VN-Index` đang Uptrend hoặc `MBZ + BBZ-M` lan tỏa dương, `BB Status = Tiền khỏe`). Tuyệt đối hạn chế mua Breakout (`Bullish Signal 4`) khi vĩ mô bão hòa/tiền ít!
   - **Vị trí Bối cảnh lớn:** Ưu tiên tuyệt đối Breakout khỏi **`Hộp 0` (vào bước `0 -> 1`)** hoặc **`Hộp 2` (vào bước sóng 3 mạnh nhất `2 -> 3`, như `BSR, VCI, TCB, KDH, C4G`)**; cực kỳ thận trọng ở `Hộp 4`!
   - **Cổ phiếu phù hợp:** `DT2`, `DT1` có câu chuyện KQKD/ngành, hoặc cổ phiếu có tiền sử `Vùng tắc nghẽn + Thanh khoản nhỏ` nay báo `Biến động hơn trước`.
2. **Checklist 7 Điều kiện Sắt đá trước khi bấm lệnh:**
   - [x] **Trước vùng nén có xu hướng tăng dẫn đường** (không phải hộp hẹp nằm lọt giữa biên sideway lớn vô hướng).
   - [x] **Hình thái mẫu hình tin cậy nhất QMV:** Tam giác hướng lên, Tam giác cân (Phễu đóng ngang), Hộp chữ nhật ngang, hoặc Phễu giảm / Tay cầm hướng xuống (`< 45°`, độ lùi `<= 50%`, lý tưởng `<= 1/3`), ưu tiên tại **Điểm chạm số 3**.
   - [x] **Nén cạn cung TRƯỚC phiên nổ:** Trong ít nhất 5–10 phiên cuối vùng nén, biên độ hẹp (`BB Squeeze` / `ATR` thu hẹp) và **`Volume` thấp** (không duy trì vol cao phân phối trước đó!).
   - [x] **Cấu trúc Nến Đột phá `2/3 – 1/3` (hoặc `Breakaway Gap`):** Cây nến xanh dài có **$\ge 2/3$ thân nến nằm vượt hẳn lên trên cạnh kháng cự hộp** + **`Volume` nổ đột biến ($\ge 1.3–1.5 \times \text{SMA}_5(V)$)**.
   - [x] **Tỷ lệ `Reward / Risk > 1`** (Target đo bằng tịnh tiến nhịp tăng trước đặt vào đáy Điểm chạm 3; Stop-loss đặt tại cận dưới hộp hoặc đáy cây nến Breakout).
3. **Hai Phương thức Giải ngân (`Chia 3 Phần` chống bẫy Breakout lần 1 hỏng):**
   - **Lần 1 (`1/3` vốn):** Giải ngân tại phiên nổ Breakout đạt chuẩn `2/3` thân nến vượt hộp + nổ `Vol` (hoặc nếu thận trọng bỏ qua lần phá thứ 1 khi nến chưa đủ `2/3` trên cản).
   - **Lần 2 & 3 (`2/3` vốn — Điểm mua `Momentum / Retest` an toàn nhất của QMV):** Không mua đuổi nếu `T+1, T+2` giá phóng vọt thẳng đứng làm hỏng `RR`! Chỉ giải ngân `2/3` còn lại khi sau phiên nổ `2–5 phiên`, giá có nhịp **chững lại / kéo lùi (`pullback`) test lại cạnh kháng cự vừa vượt (nay là hỗ trợ mới) với `Volume` giảm dần** mà không làm `"Kill nến xanh breakout"`, rồi bật tăng trở lại vượt đỉnh nhịp test!
4. **Cắt lỗ, Chốt lời & Kỷ luật Thép `T+3 Time-Stop`:**
   - **`T+3 Time-Stop` (Quy tắc Thầy Hào `HP4 v2_5_2`):** Đúng ngày **`T+3` hàng về tài khoản**, nếu cổ phiếu **không tăng tiếp so với giá phiên Breakout** (lình xình đi ngang hoặc tụt nhẹ dù chưa chạm Stop-loss dưới hộp) $\rightarrow$ **BÁN DỨT KHOÁT NGAY LẬP TỨC** (đột phá thật là phải chạy, không chạy là sai!).
   - **`Invalidation` cứng:** Giá đóng cửa xuyên thủng ngược vào trong hộp hoặc thủng đáy `Low` của cây nến Breakout (`"Kill nến xanh"`).
   - **Chốt lời:** Chia 3 mốc theo đoạn tăng tịnh tiến (`1/3 – 2/3 – 100%` `Measured Move`), chốt dần khi gặp kháng cự hoặc nâng `Trailing Stop` `2/3` sóng.

### Lớp 2 — Công thức Định lượng Hóa 100% Nhân quả (`Quant / Backtest Logic`)
1. **Nhận diện Vùng Tắc Nghẽn / Hộp / Tam giác Cạn Cung trong $[t - W,\, t - 1]$ ($W \in [12, 40]$):**
   - Tồn tại nhịp tăng dẫn đường trước hộp: $\frac{\max_{\tau \in [t-W-20, t-W]} H_\tau}{\min_{\tau \in [t-W-45, t-W-15]} L_\tau} \ge 1.12$ (hoặc đang ở nền tích lũy lớn $W \in [30, 60]$ có `SMA50 >= SMA200 * 0.97`).
   - Biên hộp kháng cự trên $B_{\text{top}} = \max_{\tau \in [t-W, t-1]} \max(O_\tau, C_\tau)$ (kèm đỉnh râu $H_{\text{box}} = \max_{\tau \in [t-W, t-1]} H_\tau$) và biên dưới $B_{\text{bot}} = \min_{\tau \in [t-W, t-1]} L_\tau$ thỏa mãn:
     - Độ hẹp & Độ lùi hợp lệ: $\frac{H_{\text{box}} - B_{\text{bot}}}{B_{\text{top}}} \le 0.12$ VÀ mức lùi so với nhịp tăng trước $\le 0.50$.
     - Đủ tối thiểu 2 lần chạm cạnh trên trước $t$ và **Nén cạn cung 5 phiên sát nổ**: $\text{SMA}_5(V)_{t-1} \le 0.85 \, \overline{V}_{20, t-1}$ VÀ $\text{ATR}_{10, t-1} < \text{ATR}_{30, t-1}$.
2. **Điều kiện Phiên Đột phá Chuẩn (`BreakoutBar(t)` — Quy tắc `2/3` thân nến hoặc `Breakaway Gap`):**
   $$\text{Break23}_t \iff (C_t > H_{\text{box}}) \;\land\; (C_t > O_t) \;\land\; (r^{\text{body}}_t \ge 0.55) \;\land\; \Big(\frac{C_t - B_{\text{top}}}{C_t - O_t} \ge \frac{2}{3}\Big) \;\land\; \text{ValidBullBar}_t$$
   $$\text{BreakGap}_t \iff (L_t > H_{\text{box}}) \;\land\; (C_t \ge O_t)$$
   $$\text{Trig\_S4\_Break}(t) \iff (C^M_t > \text{SMA}_{20}(C^M)_t) \;\land\; (\text{Break23}_t \lor \text{BreakGap}_t) \;\land\; (V_t \ge 1.35 \, \overline{V}_{5, t-1}) \;\land\; \Big(\frac{\Delta P_{\text{prior}}}{C_t - \max(B_{\text{bot}}, L_t \times 0.99)} > 1.2\Big)$$
   - Khi thỏa `Trig_S4_Break(t)`: Mua **Tranche 1 (`1/3`)** tại $C_t$ (ghi nhớ $t_{\text{brk}} = t$, $L_{\text{brk}} = L_t$, $C_{\text{brk}} = C_t$).
3. **Điểm Mua Chủ Lực `Retest / Momentum` (`2/3` vốn còn lại tại $t \in [t_{\text{brk}} + 2,\, t_{\text{brk}} + 8]$):**
   - Trong $[t_{\text{brk}} + 1,\, t - 1]$ có ít nhất 1 phiên chững/lùi test gần đỉnh hộp: $\min_j L_j \le 1.025 \, H_{\text{box}}$, **không phiên nào đóng cửa thủng $B_{\text{top}}$ hoặc thủng $L_{\text{brk}}$**, khối lượng nhịp chỉnh giảm ($\min_j V_j < 0.75 \, V_{t_{\text{brk}}}$), và tại $t$ giá bật xanh vượt đỉnh phiên chỉnh liền trước ($C_t > H_{t-1} \land C_t > O_t \land \text{RR}_t > 1.0$) $\rightarrow$ Mua tiếp **`2/3` vốn**.
4. **Luật `T+3 Time-Stop` & `Kill Nến Xanh` Nhân Quả:**
   - **Tại đúng $t = t_{\text{brk}} + 3$:** Nếu $C_t \le C_{\text{brk}}$ (sau 3 ngày hàng về giá không cao hơn giá đóng cửa phiên nổ!) $\rightarrow$ **Đóng 100% vị thế ngay tại $C_t$**!
   - **Mọi phiên $t \ge t_{\text{buy}} + 2$:** Nếu $C_t < \max(B_{\text{top}} \times 0.99,\; L_{\text{brk}})$ ("Kill nến xanh breakout") $\rightarrow$ **Đóng 100% vị thế**!
   - **Target:** $T_{\text{meas}} = B_{\text{bot, touch3}} + \Delta P_{\text{prior}}$; chốt `1/3` tại $\frac{2}{3} \Delta P_{\text{prior}}$, giữ `2/3` theo `TrailingStop` `2/3` sóng.

---

## CHIẾN LƯỢC S5: ĐÁNH NHỊP TIẾP DIỄN SÓNG TĂNG QUA PHÂN KỲ DƯƠNG ẨN & HỘP CHỈNH NÔNG `<= 1/3`
**(`Hidden Bullish Divergence & Shallow Pullback` — Nguồn gốc: `HP4 v3_3_bg + hd, v3_6_thaoluan [Case VIB, SSI]`, `KT01T3/T4 [Case HPG]`)**

### Lớp 1 — Playbook Thực Chiến Cho Trader
1. **Bối cảnh & Ý nghĩa Kinh tế học QMV:**
   - Dành cho những nhà đầu tư **bị lỡ chân sóng đầu (`0 -> 1`)** của các cổ phiếu mạnh dẫn dắt (`DT1`, `DT2` có `MA20 > MA50`, `BB Status = Tiền khỏe`) và muốn lên tàu an toàn ở đáy **`Hộp 2` (đón Sóng 3)** hoặc đáy **`Hộp 4` (ăn nốt Sóng 5 ngắn)**.
   - Bản chất cung cầu: Trong nhịp điều chỉnh của cổ phiếu khỏe, chỉ báo `RSI/CCI` hoặc `Stochastic` lùi sâu hơn đáy chỉ báo trước (phản ánh đám đông chốt lời ngắn hạn ép chỉ báo rũ sạch quá mua), **NHƯNG đáy giá lại giữ cao hơn hẳn đáy trước** (lùi nông `<= 1/3` đến tối đa `1/2` nhịp tăng trước, và đặc biệt đẹp khi `RSI(14)` vẫn trụ trên vùng `45–50`!). *"Bán rũ mạnh trên chỉ báo mà giá không xuống sâu $\rightarrow$ cầu đỡ cực khỏe, chắc chắn tăng tiếp!"*
2. **Checklist Điều kiện Cần & Đủ:**
   - [x] **Xu hướng gốc là Tăng:** `Close > SMA(50)`, `SMA(20) > SMA(50)`, `SMA(50)` dốc lên.
   - [x] **Độ sâu nhịp chỉnh lành mạnh:** Lùi **$\le 1/3$ (`~38.2%`)** (tối đa `< 50%`) biên độ nhịp tăng liền trước, khối lượng sụt giảm dần dưới `SMA(Vol, 5)`.
   - [x] **Phân kỳ Dương Ẩn (`Hidden Bullish Divergence`):** Đáy giá hiện tại **cao hơn** đáy giá liền trước ($L_{\text{curr}} > L_{\text{prev}}$), trong khi đáy của **`RSI(14)` và/hoặc `CCI(14)` thấp hơn (hoặc ngang bằng)** đáy chỉ báo tương ứng trước đó.
   - [x] **Cò súng Kích hoạt (`Trigger`):** Chỉ báo bẻ góc uốn lên (phá vỡ trendline giảm ngắn của chỉ báo) VÀ giá xuất hiện nến rút chân / nến xanh **đóng cửa vượt lên trên mức `High` của cây nến có đáy thấp nhất trong nhịp chỉnh hiện tại** ($C_t > H_{t_{\text{pullback\_low}}}$)!
3. **Đi Vốn, Quản trị Kỳ vọng & Cắt lỗ:**
   - Ở **`Hộp 2`** (nhịp chỉnh đầu tiên): Giải ngân đủ `2/3` tại phiên Trigger + `1/3` khi vượt đỉnh cũ Sóng 1, kỳ vọng Target hội tụ `Measured Move = 100%` nhịp 1 (`Fibo Extension 1.0 – 1.618`, như `VIB` lên `54`).
   - Ở **`Hộp 4`** (nhịp chỉnh thứ 2 — giảng viên lưu ý dư địa Phân kỳ dương ẩn ở cuối sóng ngắn hơn!): Chỉ giải ngân tối đa **`2/3` vốn dự kiến**, chốt lời chủ động ngay khi chạm đỉnh cũ hoặc đạt `61.8% – 100%` Sóng 1!
   - **`Stop-Loss` & `Invalidation`:** Đặt ngay dưới đáy nhịp chỉnh $L_{\text{curr}}$ (hoặc nếu nhịp chỉnh ăn lẹm **quá `50%` đoạn tăng trước** thì hủy kịch bản ngay lập tức!). Áp dụng `Time-Stop` **4 phiên** nếu sau Trigger giá không đi lên.

### Lớp 2 — Công thức Định lượng Hóa 100% Nhân quả (`Quant / Backtest Logic`)
1. **Cấu trúc Uptrend & Hộp Chỉnh Nông tại $t$:**
   - $C_t > \text{SMA}_{50,t} \;\land\; \text{SMA}_{20,t} > \text{SMA}_{50,t} \;\land\; \text{SMA}_{50,t} > \text{SMA}_{50,t-5}$.
   - Gọi $\tau_1 \in [t - 55,\, t - 15]$ là đáy khởi phát nhịp tăng liền trước ($\text{PivotLow}_3(\tau_1)$), $\tau_{\text{pk}} \in [\tau_1 + 5,\, t - 3]$ là đỉnh nhịp tăng đó ($H_{\tau_{\text{pk}}} = \max_{[\tau_1, t-2]} H$, với $\frac{H_{\tau_{\text{pk}}} - L_{\tau_1}}{L_{\tau_1}} \ge 0.12$), và $\tau_2 = \arg\min_{j \in [\tau_{\text{pk}}+1,\, t]} L_j$ là đáy thấp nhất của nhịp chỉnh hiện tại ($\tau_2 \in [t - 4,\, t - 1]$):
     $$\text{RetraceRatio}_t = \frac{H_{\tau_{\text{pk}}} - L_{\tau_2}}{H_{\tau_{\text{pk}}} - L_{\tau_1}} \in [0.15,\; 0.48] \quad (\text{ưu tiên } \le 0.382) \;\land\; L_{\tau_2} > L_{\tau_1} \times 1.03$$
2. **Phân Kỳ Dương Ẩn trên `RSI(14)` / `CCI(14)` + Cò súng `Close > High[lowest_bar]`:**
   $$\text{HiddenDiv}_t \iff \Big(\min_{j \in [\tau_2-1, \tau_2+1]} \text{RSI}_{14,j} \le \min_{j \in [\tau_1-1, \tau_1+1]} \text{RSI}_{14,j} \;\land\; \text{RSI}_{14,\tau_2} \ge 42\Big) \;\lor\; \Big(\min_{j \in [\tau_2-1, \tau_2+1]} \text{CCI}_{14,j} < \min_{j \in [\tau_1-1, \tau_1+1]} \text{CCI}_{14,j}\Big)$$
   $$\text{Trig\_S5}(t) \iff G^{\text{mkt}}_t \;\land\; \text{HiddenDiv}_t \;\land\; \big(C_t > H_{\tau_2}\big) \;\land\; (C_t > O_t) \;\land\; (\text{RSI}_{14,t} > \text{RSI}_{14,t-1}) \;\land\; (\text{CCI}_{14,t} > \text{CCI}_{14,t-1})$$
3. **Quản trị Vị thế:**
   - **Stop-Loss:** $C_t < L_{\tau_2}$ hoặc mức lùi vượt quá $50\%$ đoạn $[L_{\tau_1}, H_{\tau_{\text{pk}}}]$.
   - **Time-Stop:** $t = t_{\text{buy}} + 4$ mà $C_t \le P_{\text{entry}}$.
   - **Target:** Chốt `1/2` tại đỉnh cũ $H_{\tau_{\text{pk}}}$ nếu là Hộp 4 (đã có $\ge 2$ nhịp tăng trong 90 phiên), hoặc giữ hướng tới $T_{\text{ext}} = L_{\tau_2} + (H_{\tau_{\text{pk}}} - L_{\tau_1})$ kèm `TrailingStop` `2/3` sóng nếu là Hộp 2.

---

## CHIẾN LƯỢC S6: ĐÁNH XOAY VÒNG NHỊP NGẮN `T+03` TRONG THỊ TRƯỜNG SIDEWAY & HẤP THỤ HÀNG VỀ
**(`QMV T+2.5 Cycle Inflection Swing` — Nguồn gốc: `Fintech Buổi 1 & 2`, `Chọn CP Fintech 17/07/2025`, `V5/V7 Guide`)**

### Lớp 1 — Playbook Thực Chiến Cho Trader
1. **Bối cảnh Thị trường cho phép đánh `T+`:**
   - Dành riêng cho môi trường **Thị trường Đi ngang / Phân hóa (`Sideways`)**, khi đánh xu hướng dài không hiệu quả nhưng dòng tiền vẫn luân chuyển ngắn hạn.
   - **Kiểm tra điều kiện cần cấp Thị trường (`Fintech Buổi 2`):**
     1. Biên độ dao động của dòng tiền `DM03` (bản Mặc định) đủ rộng (không bị phẳng lì bóp nghẹt thanh khoản);
     2. Cả `MBZ(03, 05)` VÀ `BBZ-M(03, 05)` cùng uốn lên từ vùng thấp `< 20..30` (hoặc `EM03 > 0`).
2. **Loại cổ phiếu & Nghịch lý Chọn Sao:**
   - Chọn cổ phiếu thanh khoản cao thuộc `DT2` (hoặc `DT1` biên rộng như Chứng khoán `SSI, VCI, VND, SHS`), có `TMA50` đi ngang hoặc hướng lên (tránh mã `TMC` đang cắm đầu rơi thẳng đứng!).
   - **Áp dụng nghịch lý Contrarian:** Ưu tiên mã đang **`1 – 3 sao` + xuất hiện nhãn `Tốt hơn`** (cấu trúc giá hôm nay vừa cải thiện so với hôm qua), **tránh mua mã đã `5 sao + Không đổi/Kém hơn`** ở cuối nhịp T+!
   - **Cấm kỵ tuyệt đối:** Trong thị trường Sideway, thấy mã báo `Tiền vào 3 phiên liên tiếp` thì **CẤM nhảy vào mua đuổi ở phiên thứ 3** (vì phiên 4 hàng T+ về sẽ bị chốt úp ngay) — chỉ đưa vào Watchlist chờ chỉnh xong mới bắt!
3. **Tổ hợp Tín hiệu Bắt Đúng Chân Vòng `T+03` Mới:**
   - Sau 2–3 phiên điều chỉnh nhẹ đủ một chu kỳ hàng `T+2.5` về tài khoản (hoặc 2.5 ngày sau một cây nổ Vol trước đó), hàng về **được hấp thụ hết không bị bán tháo**:
     - `Đảo chiều tăng hôm nay` + `Đảo chiều tăng T+` ($dM_t > 0$ sau 2 phiên $dM_{t-1} < 0, dM_{t-2} < 0$);
     - `DMZ03` và `DMZ05` cùng chụm dưới `< 20..30` và uốn lên (**kiểm tra chéo bản Mặc định không bị bẫy biên độ chết 1% hay bẫy neo đỉnh!**);
     - `Tốc độ chu kỳ 03` (`SP03 / EM03 > 0` — cầu thực sự khỏe hơn cung cổ phiếu bán ra trong vòng 3 phiên);
     - `DSPI` và `MPICP` đang từ vùng thấp nhích lên (chưa chạm trần hưng phấn `MPICP > 80`).
4. **Đi Vốn & Chốt Lời Nhanh Theo Chu Kỳ `T+3 .. T+5`:**
   - Vì biên lợi nhuận nhịp `T+` trong Sideway mỏng (`+5%` đến `+12%`), giải ngân gọn **2 phần** (`1/2` tại phiên đảo chiều `T+`, `1/2` nếu phiên hôm sau xác nhận cầu tiếp diễn và giá chưa vượt `MaxBuy`), tổng tỷ trọng mỗi mã $\le 10\%$ NAV.
   - **Chốt lời dứt khoát ngay khi hàng về `T+2.5 .. T+5`** nếu thấy: `DMZ(03, 05)` chụm trên `> 70..80` uốn xuống + `MPICP` vọt chạm đỉnh cao (`> 75–80`) + chạm kháng cự hộp ngang (như case `VIC` 25/06/2026: chốt lời T+ ra trước, đợi 2–3 phiên `DMZ03` rơi nhanh về `< 30` lại vòng vào làm ván mới!).

### Lớp 2 — Công thức Định lượng Hóa 100% Nhân quả (`Quant / Backtest Logic`)
Sử dụng bộ công thức `QMV Blackbox OHLCV Proxy` (đặc tả chi tiết tại Tài liệu `03`):
1. **Điều kiện Không Bị Bẫy Chuẩn Hóa 50 Phiên & Xu hướng Tiền Nền:**
   - Biên độ dòng tiền 3 phiên thực tế đủ rộng: $\max_{\tau \in [t-49, t]} |\text{DM}_{03}(\tau)| \ge 1.5 \times \overline{Val}_{20, t} \times 0.15$ VÀ không bị neo đỉnh ($\text{DM}_{20}(t) < 0.85 \max_{[t-49, t]} \text{DM}_{20}$).
   - Nền tiền lớn không rút tháo: $\text{TMA50}_t \ge \text{TMA50}_{t-5}$.
2. **Tín hiệu Khởi Phát Vòng `T+03` Mới (`Trig_S6(t)`):**
   - Đảo chiều dòng tiền đúng nhịp `T+2.5`: $dM_t > 0 \;\land\; dM_{t-1} < 0 \;\land\; dM_{t-2} < 0$.
   - Hội tụ chu kỳ ngắn uốn lên từ vùng Duy trì `< 30` + Tốc độ `EM03 > 0`:
     $$\big(\text{DMZ}_{03, t-1} < 30 \land \text{DMZ}_{05, t-1} < 35\big) \;\land\; \big(\text{DMZ}_{03, t} > \text{DMZ}_{03, t-1} \land \text{DMZ}_{05, t} > \text{DMZ}_{05, t-1}\big) \;\land\; \big(\text{EM}_{03, t} > 0\big)$$
   - `MPICP` chưa hưng phấn ($\text{MPICP}_t < 60 \land \text{MPICP}_t > \text{MPICP}_{t-1}$), điểm sao `1 <= Stars_t <= 3` với `Stars_t > Stars_{t-1}` (`Tốt hơn`), và giá còn cách kháng cự 20 phiên tối thiểu $+5\%$ ($\frac{\max_{[t-20, t-1]} H - C_t}{C_t} \ge 0.05$).
3. **Luật Chốt lời & Thoát Nhịp `T+` (từ $t \ge t_{\text{buy}} + 2$):**
   - **Chốt lời Thái cực trên:** $\big(\text{DMZ}_{03, t} > 75 \land \text{DMZ}_{03, t} < \text{DMZ}_{03, t-1}\big) \;\lor\; (\text{MPICP}_t \ge 80) \;\lor\; (H_t \ge 0.99 \max_{[t-25, t_{\text{buy}}]} H)$ $\rightarrow$ Đóng 100% vị thế thu tiền quay vòng!
   - **Stop-Loss & Time-Stop ngắn:** Đóng 100% nếu $C_t < \min_{j \in [t_{\text{buy}}-2, t_{\text{buy}}]} L_j$ (thủng đáy cụm đảo chiều) HOẶC giữ quá $t = t_{\text{buy}} + 5$ mà lợi nhuận $< +1.5\%$ và $\text{EM}_{03, t} < 0$.

---

## CHIẾN LƯỢC S7: ĐÁNH LUÂN CHUYỂN 9 QUẢ BÓNG `DT1 -> DT2 -> DT3` & SĂN SÓNG "TRÀ ĐÁ" QUÉT NGƯỢC BƯỚC `0 -> 1`
**(`9-Ball Risk Rotation & Distressed Penny Surge` — Nguồn gốc: `KT01T4`, `HP5 v3_3, v3_4`, `DN01 / Trà đá`, `Fintech Buổi 1`)**

### Lớp 1 — Playbook Thực Chiến Cho Trader
1. **Bối cảnh Vĩ mô Tiên quyết (Cổng Khóa Sống Còn):**
   - **CHỈ MỞ KHÓA** khi vĩ mô ở **Môi trường Tiền nhiều / Nới lỏng** (Pha 1 hoặc giữa Pha 2; `Sentiment Gauge` hướng lên — tiền chảy vào tài sản rủi ro mạnh hơn tài sản an toàn) VÀ các cổ phiếu lớn **`DT1` đầu ngành (`Ngân hàng, Chứng khoán, BĐS`) đã chạy mở sóng trước đó**!
   - **ĐÓNG CỔNG 100%** khi vĩ mô Thắt chặt hoặc khi xuất hiện tín hiệu **"Đổi gác" (`DT3` chững lại + tiền quay về giữ giá ở 3 quả bóng `DT1`)**!
2. **Hai Nhánh Chiến thuật Thực chiến của Thầy Hào:**
   - **Nhánh 7A — Đón Luân chuyển Nội ngành & Liên ngành (`DT1 -> DT2` & `7 Cụm ngành đi đôi`):**
     - Khi thấy tiền vào mạnh một ngành trong `1–2 phiên` đầu mà nhóm `DT1` đã vọt xa nền $\rightarrow$ lập tức chọn cổ phiếu **`DT2` (Midcap $\beta \in [1.5, 2.5]$, định giá thấp so với ngành)** trong chính ngành đó, hoặc đón `DT1/DT2` của ngành cặp đôi đi sau (`Bank chạy -> múc Chứng`; `BĐS chạy -> múc XD&VLXD`; `Hóa chất chạy -> múc Dầu khí`).
   - **Nhánh 7B — Săn Sóng "Trà Đá" (`Thị giá < 10.000đ` / `DT3`) bằng Kỹ thuật "Quét Ngược Bước `0 -> 1`":**
     - **Lọc Sinh Tồn ("Không được chết"):** Không lỗ 2 năm liên tiếp, lỗ lũy kế $<$ vốn góp, VCSH $> 0$, không bị hạn chế giao dịch; ưu tiên **Trà đá Loại 1** (`P/B < 1.0`, có doanh thu kinh doanh thật từng bị lãi vay ăn mòn nay hưởng lợi kép lãi suất giảm, như case `MST`).
     - **Không mò kim đáy bể chờ ở vùng chết thanh khoản:** Chấp nhận **bỏ qua `10% – 20%` đầu tiên**! Dùng bộ lọc tìm mã có tiền sử **`Vùng tắc nghẽn` kéo dài ($\ge 2–3$ tháng) + `Thanh khoản nhỏ`** nay bất ngờ nổ **`Biến động hơn trước` + `Khối lượng đột biến` + `%SP > 0`** (`Cáo` phát lệnh đánh lên).
     - **Soi ngược Bối cảnh lớn:** Nếu cú nổ vừa qua mới chỉ là **Bước `0 -> 1`** (mới tăng `7% – 20%` từ nền tích lũy nhiều tháng) $\rightarrow$ chờ nhịp nghỉ/chỉnh đầu tiên **`1 -> 2`** (hoặc gom rải nếu đang đặt chân ở cuối Vùng `0` có `Vol` gom lớn mà giá chưa tăng) để ăn chuỗi sóng `x2, x3` hướng về mệnh giá `10.000đ`! Nếu mở ra thấy đã ở Bước `3, 4, 5` trên cao $\rightarrow$ bỏ qua hoặc chỉ đánh lướt ngắn.
3. **Quản trị Tỷ trọng, Chốt lời Chống "Cây thông" & Thoát Đỉnh Chu Kỳ:**
   - **Giới hạn tổng vốn cho toàn bộ nhánh `DT3` / Trà đá:** Tối đa **$\le 1/3$ tổng danh mục** (mỗi mã $\le 5\%–8\%$ NAV), **không dùng tiền vay nóng ngắn hạn**.
   - **Trị bệnh tham đỉnh trên hàng lái (`HP1 v3_hoidap`):** Chia 3 phần chốt lời chủ động dọc đường lên (chốt `1/3` đầu tiên từ mức lãi `+15%..+20%`, chốt `1/3` ở nhịp tăng thứ 2, giữ `1/3` cuối chặn lãi sát).
   - **Thoát khẩn cấp 100% không ngoái đầu:** Ngay khi **(1) Nổ `Volume` cực đại sau chuỗi Tiết cung / trần liên tiếp**, HOẶC **(2) Toàn bộ `DT3`/Trà đá trên toàn sàn đồng loạt tím trần rồi chững lại và tiền đổi gác về `DT1`**!

### Lớp 2 — Công thức Định lượng Hóa 100% Nhân quả (`Quant / Backtest Logic`)
1. **Cổng Kích hoạt Khẩu vị Rủi ro (`RiskOnRegime(t)` từ rổ 400+ mã toàn sàn):**
   - Phân rổ vũ trụ cổ phiếu theo vốn hóa/thanh khoản/thị giá lăn 60 phiên thành `Basket_DT1`, `Basket_DT2`, `Basket_DT3` (gồm Trà đá $C < 10{,}000\text{đ}$ đã pass `HardAntiDelistingGate`).
   - Điều kiện mở sóng luân chuyển & chưa "Đổi gác":
     $$\text{DT1\_Led}_t \iff \text{Ret}_{20}(\text{Basket\_DT1})_t > +0.04 \;\land\; C^M_t > \text{SMA}_{50}(C^M)_t$$
     $$\text{GuardChange}_t \iff \big(\text{Ret}_5(\text{Basket\_DT3})_t < -0.01\big) \;\land\; \big(\text{Ret}_5(\text{Basket\_DT3})_{t-5} > +0.06\big) \;\land\; \big(\text{Ret}_5(\text{Basket\_DT1})_t > \text{Ret}_5(\text{Basket\_DT3})_t + 0.025\big)$$
     $$\text{RiskOnRegime}_t \iff \text{DT1\_Led}_t \;\land\; \neg \text{GuardChange}_t$$
2. **Tín hiệu Quét Ngược Chân Sóng Bước `0 -> 1 -> 2` (`Trig_S7B(t)`):**
   - **Tiền sử Nền Gom Dài (`Box 0` trong $[t - 75,\, t - 12]$):** Biên độ giá 60 phiên trước pha nổ nằm trong dải hẹp $\frac{\max_{[t-75, t-12]} H - \min_{[t-75, t-12]} L}{\text{Median}_{[t-75, t-12]} C} \le 0.22$ VÀ chưa tăng quá xa từ nền gốc: $\frac{C_t - P_{\text{base60}}}{P_{\text{base60}}} \in [0.06,\; 0.24]$ (bắt đúng vùng mới nhú `6% – 24%`, loại bỏ mã đã chạy `> 30%–50%`!).
   - **Tín hiệu Nổ Khởi Động Bước `0 -> 1` đã xuất hiện trong 10 phiên qua:** $\exists k \in [t-10,\, t-2]: (C_k > 1.04 \, C_{k-1}) \land (V_k > 2.0 \, \overline{V}_{20, k-1}) \land (\text{SP}_k > 0)$.
   - **Điểm Vào tại Nhịp Chỉnh / Hấp Thụ `1 -> 2` hiện tại ($t$):** Mức lùi từ đỉnh nhịp 1 $\le \frac{1}{3}$ đoạn $(H_{\text{wave1}} - P_{\text{base60}})$, hàng `T+` về không làm thủng nửa thân cây nổ $k$, và tại $t$ cầu đẩy tiếp ($C_t > H_{t-1} \land V_t > \overline{V}_{5,t}$).
3. **Thoát Khẩn Cấp Chống Cây Thông (`Exit_S7(t)`):**
   - Bán 100% ngay tại $C_t$ (hoặc phiên mở sàn kế tiếp nếu dính `FloorLock`) khi $t \ge t_{\text{buy}} + 2$ và xảy ra **một trong ba** tín hiệu:
     1. **Nổ Vol Xả sau Tiết cung / Kéo dốc:** $\frac{C_{t-1}}{P_{\text{entry}}} \ge 1.25 \;\land\; V_t > 2.2 \, \overline{V}_{5, t-1} \;\land\; \big(C_t \le C_{t-1} \lor p^{\text{high}}_t \le 0.4\big)$;
     2. **Cảnh báo Đổi Gác toàn thị trường:** $\text{GuardChange}_t = \text{True}$;
     3. **Vi phạm Chặn lãi động `2/3` Sóng** hoặc thủng nền hỗ trợ gần nhất.

---

## CHIẾN LƯỢC S8: HỆ THỐNG BÁN CHỦ ĐỘNG ĐA TẦNG & REBALANCING KHO 6 PHẦN TRÊN CỔ PHIẾU SẴN CÓ
**(`Active Distribution Exit & 6-Part Inventory Rebalancing` — Nguồn gốc: `KT01T3`, `KT01T4`, `DM01T4`, `DM01_BUỔI_4`, `HP4 v1_6, v3_4`)**

### Lớp 1 — Playbook Thực Chiến Cho Trader
1. **Hai Mục tiêu Tách Biệt (`Thoát hẳn Bảo vệ Thành quả` vs `Rebalancing Hạ Giá Vốn`):**
   - **Chế độ 8A — Bán Thoát Hẳn Vị Thế (`Full / Scale-Out Exit`):** Áp dụng cho mọi vị thế `Trading`, hoặc vị thế `Investing` khi cổ phiếu đã đi hết **Bước `5` của Bối cảnh lớn** (`Thị giá >> Định giá` + `BB Status = Bão hòa / Tiền yếu` + cảnh báo đỉnh chu kỳ).
   - **Chế độ 8B — `Rebalancing` Kho Hàng Dài Hạn (`6-Part Inventory`):** Chỉ áp dụng trên cổ phiếu **Doanh nghiệp tốt (`4–5 sao`)** đang trong chu kỳ hưởng lợi, `BB Status = Duy trì / Khỏe`, và còn nằm ở nửa dưới/giữa Bối cảnh lớn (`Hộp 1, 3` hoặc kênh đi ngang trung hạn).
2. **Quy trình Bán Chủ Động 3 Nhịp tại Vùng Kháng Cự (`KT01T3` — *"Mua chậm rãi, Bán dứt khoát sớm"*):**
   - Xác định trước **Vùng Bán**: Đỉnh cũ gần nhất, hoặc mốc đo tịnh tiến `1/3 – 1/2 – 2/3 – 100%` + cận trên `BB(15, 2)`.
   - Khi giá tiến vào Vùng Bán, mở khung **`30m` + `1D`** kiểm tra:
     - *Nếu vượt cản bằng nến xanh đặc dài khỏe + Giá lượng bình thường (`VNI` vượt 1442):* **Không bán chặn máy móc, để lãi chạy!**
     - *Nếu xuất hiện dấu hiệu suy yếu:* **(1) Nến `Upthrust` (Kéo xả râu trên dài) / `Evening Star` / `Exhaustion Gap` kèm `Vol` cao**; **(2) `"Đạp ga xe không chạy"` (3–6 phiên đi ngang trên đỉnh với `Vol > SMA5` mà giá không tăng)**; **(3) Phân kỳ âm đồng thời trên `RSI + CCI` (cắt xuống `MA` của `CCI`) + Giá chui từ ngoài biên trên `BB(15, 2)` vào trong lần 2 + `Blackbox` chụm `100%` uốn xuống**:
       - **Lần 1 (`1/3` lượng hàng):** Bán thăm dò ở đỉnh chạm 1 (nếu đã ở Bước `5` như `GAS` thì bán dứt khoát ngay từ chạm 1!).
       - **Lần 2 (`1/3` — Mục tiêu bán chính):** Bán tại **Điểm chạm đỉnh số 2** có `Upthrust + Vol lớn`.
       - **Lần 3 (`1/3` cuối):** Bán sạch khi gãy cận dưới vùng dao động đỉnh / gãy `Trailing Stop` `2/3` sóng (hoặc ở vùng `7` sườn phải nếu lỡ kẹp).
   - **Luật Cấm Bán Muộn Ngay Hỗ Trợ (`KT01T3`):** Nếu không kịp bán trên đỉnh mà giá đã rơi sát một vùng Hỗ trợ bên dưới và thấy **`Giá dừng giảm + Lượng dừng giảm`** (`RSI/CCI` xoắn lại) $\rightarrow$ **DỪNG BÁN NGAY LẬP TỨC**, chờ nhịp nảy `bull-trap` mới bán!
3. **Quy trình Khép Vòng `Rebalancing` (Chế độ 8B — Kỷ luật "Đúng hay Sai đều phải Mua lại"):**
   - Chia danh mục mã `Core` thành `3 phần cổ phiếu + 3 phần tiền = Kho 6 phần` (hoặc giữ `70% Core` + đảo `30% Satellite` chia 3 lần `10% – 10% – 10%`):
     - Bán ra $\Delta Q$ (ví dụ `1/3` hoặc `1/2` kho) tại Vùng Kháng cự khi có `Bearish Signal 1, 2` + `Upthrust` / `MPICP > 80`.
     - **Kịch bản A (Đoán đúng — Giá chỉnh về Hỗ trợ dưới):** Kiên nhẫn chờ **Điểm chạm 2–3 + Điểm uốn cạn cung** (trên `1D` hoặc `15m/30m`) để mua lại **đúng $\Delta Q$ cổ phiếu** $\rightarrow$ tổng cổ phiếu giữ nguyên, giá vốn giảm sâu!
     - **Kịch bản B (Đoán sai — Giá vượt kháng cự tăng luôn):** Tuyệt đối không bỏ luôn mã tốt! Chờ giá xác nhận vượt hẳn cản rồi **lùi nhẹ / đi ngang retest trên nền kháng cự cũ (nay thành hỗ trợ mới) tạo điểm uốn tăng tiếp (`Bullish Signal 3`)** $\rightarrow$ **Mua cover lại đủ $\Delta Q$ cổ phiếu** (chấp nhận mất một đoạn nhỏ coi như phí bảo hiểm, giữ trọn vị thế cho đại sóng!).

### Lớp 2 — Công thức Định lượng Hóa 100% Nhân quả (`Quant / Backtest Logic`)
1. **Tín hiệu Phân Phối Chủ Động tại Kháng Cự (`DistSignal(t)`):**
   - Vị trí vùng cao: $H_t \ge 0.985 \, R_{\text{zone}}(t)$ HOẶC $\frac{H_t - L_{\text{base60}}}{L_{\text{base60}}} \ge 0.35$ HOẶC $H_{t-1} > \text{BBUpper}_{15,2}(t-1)$.
   - Ba cụm tín hiệu cảnh báo đỉnh nhân quả:
     - **Cụm 1 (`Upthrust / Exhaustion`):** $\big(H_t > R_{\text{zone}} \land C_t \le R_{\text{zone}} \land p^{\text{high}}_t \le \frac{1}{3} \land V_t \ge 1.25 \, \overline{V}_{5,t}\big)$ HOẶC lấp `Exhaustion Gap` trong `T+1..T+3`;
     - **Cụm 2 (`Stalling High Vol` — "Đạp ga xe không chạy"):** Trong 5 phiên $[t-4, t]$, có $\ge 4$ phiên $V_j > \overline{V}_{5,j}$ NHƯNG $\frac{|C_t - C_{t-4}|}{C_{t-4}} \le 0.015$ và $C_t \le \max_{[t-4, t-1]} C$;
     - **Cụm 3 (`Dual Bearish Divergence + BB(15,2) Re-entry`):** Đỉnh giá $H_{\text{curr}} \ge 0.99 \, H_{\text{prev}}$ nhưng $\text{RSI}_{14,\text{curr}} < \text{RSI}_{14,\text{prev}} - 3 \;\land\; \text{CCI}_{14,\text{curr}} < \text{CCI}_{14,\text{prev}} - 20 \;\land\; \text{CCI}_{14,t} < \text{SMA}_5(\text{CCI}_{14})_t \;\land\; C_t < \text{BBUpper}_{15,2}(t)$.
2. **Bộ Chặn "Cấm Bán Đúng Hỗ Trợ Đang Dừng Giảm" (`NoPanicAtSupport(t)`):**
   $$\text{AtSupportStopping}_t \iff \big(L_t \le 1.02 \, S_{\text{major}}(t)\big) \;\land\; \big(|C_t - C_{t-1}| \le 0.008 \, C_{t-1}\big) \;\land\; \big(V_t \le V_{t-1} \le V_{t-2}\big) \;\land\; \neg(C_t < 0.985 \, S_{\text{major}}(t))$$
   *(Nếu $\text{AtSupportStopping}_t = \text{True}$, hoãn lệnh bán đuổi `Momentum Sell`, chờ nảy hồi $\ge +2\%–3\%$ hoặc thủng hẳn $0.985 \, S_{\text{major}}$ mới bán!).*
3. **Máy Trạng Thái Nhân Quả cho `Rebalancing` Kho $Q_{\text{target}}$ (Chế độ 8B):**
   - **Trạng thái `FULL` ($Q_t = Q_{\text{target}}$):** Khi `DistSignal(t)` kích hoạt tại kháng cự $R_{\text{zone}}$ (với `TMA50` còn dốc lên, chưa phải Bước 5 phân phối chu kỳ) $\rightarrow$ Bán $\Delta Q = \lfloor \frac{1}{3} Q_{\text{target}} / 100 \rfloor \times 100$ (hoặc $\frac{1}{2} Q_{\text{target}}$ tại chạm đỉnh 2), lưu $P_{\text{sold}} = C_t, R_{\text{sold}} = R_{\text{zone}}$, chuyển sang trạng thái `TRIMMED`.
   - **Trạng thái `TRIMMED` ($Q_t = Q_{\text{target}} - \Delta Q$):**
     - *Nhánh Mua lại Giá thấp (Đoán đúng):* Khi $C_t \le 0.94 \, P_{\text{sold}}$ (hoặc chạm hỗ trợ hộp $S_{\text{zone}}$) VÀ thỏa điều kiện điểm uốn `CandleTrig_t` của Chiến lược S1 $\rightarrow$ Mua lại đúng $+\Delta Q$ cổ phiếu, quay về `FULL` (hạ giá vốn thành công!).
     - *Nhánh Mua Cover Giá cao (Đoán sai — Kỷ luật bắt buộc):* Nếu giá không chỉnh mà bứt phá đóng cửa $C_{t_b} > 1.02 \, R_{\text{sold}}$ kèm $V_{t_b} > 1.25 \, \overline{V}_{5, t_b}$, sau đó tại $t \in [t_b + 2,\, t_b + 8]$ giữ vững $L_{[t_b+1..t]} \ge 0.99 \, R_{\text{sold}}$ và bật xanh $C_t > H_{t-1}$ $\rightarrow$ **Mua cover lại đúng $+\Delta Q$ cổ phiếu** tại $C_t$, quay về `FULL`!
     - *Nhánh Chuyển sang Thoát Hẳn:* Nếu trong lúc `TRIMMED`, bối cảnh lớn gãy ($\text{TMC}_t < \text{TMA20}_t < \text{TMA50}_t$ và $C_t < \text{SMA}_{50,t}$ sau khi xuyên thủng $S_{\text{zone}}$) $\rightarrow$ Hủy mua lại, bán nốt phần còn lại theo quy tắc bảo vệ danh mục!

---

## BẢNG MA TRẬN ĐIỀU PHỐI 8 CHIẾN LƯỢC THEO CHẾ ĐỘ THỊ TRƯỜNG (`STRATEGY ROUTING MATRIX`)

| Chế độ Vĩ mô & `VN-Index` | Nhóm Cổ phiếu Cho phép | Chiến lược Chủ đạo được Bật (`ON`) | Chiến lược Bị Khóa Cứng (`OFF`) | Tổng Tỷ trọng Cổ phiếu / Tiền mặt |
| :--- | :--- | :--- | :--- | :---: |
| **1. Đầu & Giữa Nới lỏng + `VNI` Uptrend** | `DT1 -> DT2 -> DT3` & `Trà đá Loại 1` | **S1** (Hỗ trợ Hộp 2), **S3** (Trend MA/Ichi), **S4** (Breakout), **S5** (Phân kỳ ẩn), **S7** (9 Bóng & Trà đá), **S8B** (Rebalancing) | Tắt chế độ bắt đáy hoảng loạn **S2** (vì cổ phiếu không về `EMA200` tuần) | **`3/3` (`100%` CP + cho phép Margin trên S3)** |
| **2. Bão hòa Đỉnh / Tăng Phân hóa (`VNI` Bước 4–5)** | Chọn lọc `DT2` ngành có Driver, `Trà đá` mới ở `0->1`, `NN2/NN3` | **S1** (chỉ Hộp 0–2), **S5** (chốt nhanh), **S6** (Xoay vòng `T+03`), **S7B** (chỉ mã mới `0->1`), **S8A & S8B** | **KHÓA S4 (Cấm mua Breakout vùng cao)**; khóa mua đuổi phiên 3 | **`2/3` $\rightarrow$ `1/3` CP (`33%–67%` Tiền, `0` Margin)** |
| **3. Đi ngang Hẹp / Hậu Điều chỉnh (`VNI` Sideway)** | `DT2`, `DT1` biên rộng `1–3 sao + Tốt hơn` | **S1** (Biên dưới hộp chạm 2–3), **S6** (`T+03` Inflection), **S8B** (Kho 6 Phần `15m/30m`) | **KHÓA S3 (MA Whipsaw)**, **KHÓA S4**, **KHÓA S7 (Trà đá tắt thanh khoản)** | **`1/3` đến `2/3` CP** |
| **4. Cú Sốc Bán Tháo Hoảng Loạn (`Panic / Đáy 3`)** | **CHỈ `DT1, DT2` cơ bản 4–5★ (`QMV30/60`)** | **S2** (`Weekly EMA200 + Đáy 3 MACD + Kill Nến Đỏ`), **S8A** (Cơ cấu hàng yếu nhịp nảy) | **KHÓA S3, S4, S5, S7**; cấm tuyệt đối bắt đáy `DT3`/Trà đá/BĐS nợ xấu | **`1/3` $\rightarrow$ `2/3` CP (chia 3 phần nghiêm ngặt)** |
| **5. Chính thức Thắt chặt / Đổi Gác `DT3 -> DT1` cuối chu kỳ** | **Cổ phiếu "Tiền mặt"** | **S8A** (Thoát sạch 100% danh mục về Tiền mặt) | **KHÓA TOÀN BỘ S1..S7 & S8B** (Cấm mua cả cổ phiếu phòng thủ `DT1`!) | **`0/3` CP – `3/3` (`100%`) Tiền mặt** |
