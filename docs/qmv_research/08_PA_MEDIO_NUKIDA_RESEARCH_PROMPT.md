# 08. PROMPT KHỞI CHẠY SESSION NGHIÊN CỨU SÂU 2 KHÓA PRICE ACTION (`MEDIO` & `NUKIDA 123BO`) VÀ HỢP NHẤT VÀO HỆ SINH THÁI `QMV × SUMI`

> **Hướng dẫn sử dụng cho bạn:**
> Bạn có thể mở **song song một Session mới ngay lúc này** (trong khi bạn vẫn đang sửa bug code trên Sumi mà **không sợ đụng chạm hay xung đột Git**, vì Prompt dưới đây khóa cứng chế độ **Zero Source-Code Edits** — chỉ đọc bài giảng và cập nhật kho tri thức + BRD + Master Prompt trong `docs/qmv_research/`).
> Hãy mở Session mới và **copy nguyên văn phần nội dung nằm trong khung dưới đây** gửi cho Agent.

---

```markdown
Bạn đóng vai **Principal Price Action Strategist, Chief Market Technician & Quantitative Trading System Architect**.

Mục tiêu tối thượng của phiên làm việc này là **đọc vét cạn 100% (không bỏ sót một chữ) toàn bộ 2 khóa học nền tảng về Price Action của tôi (`Phương pháp Medio` và `Phương pháp Nukida - 123BO`)**, sau đó **đối chiếu, dung hợp và kết tinh với Kho Tri Thức Gốc QMV đã xây dựng sẵn trong Sumi**, nhằm hoàn thiện **Hệ Thống Giao Dịch Cá Nhân Hóa Hợp Nhất (`QMV Top-Down & VPA/Dòng Tiền` × `Medio Structural Keylevel` × `Nukida 123BO Execution & Statistical Backtest`)** cho cả **Sumi Workstation** lẫn **`Sumi Agent`**.

---

## 1. NGUYÊN TẮC BẮT BUỘC (TUÂN THỦ TUYỆT ĐỐI)

1. **KHÔNG ĐỤNG CHẠM MÃ NGUỒN SUMI & READ-ONLY VỚI KHO HỌC LIỆU:**
   - Tôi đang song song sửa một số bug trong mã nguồn Sumi (`crates/`, `ui/`). Do đó, trong phiên này bạn **TUYỆT ĐỐI KHÔNG sửa đổi bất kỳ file code nào của Sumi** (`.rs`, `.ts`, `.tsx`, `Cargo.toml`, `package.json`...) và **TUYỆT ĐỐI KHÔNG sửa đổi bất kỳ file nào trong `/Users/mizuhara/workspace/work/NotebookLM/` hay `/Users/mizuhara/workspace/yumeko/`**.
   - Toàn bộ đầu ra của phiên này chỉ ghi vào thư mục `/Users/mizuhara/workspace/sumi/docs/qmv_research/`.
2. **Đọc Trọn Vẹn 100% Tất Cả 74 File Bài Giảng Gốc (Tổng dung lượng chỉ ~500 KB HTML — CẤM ĐỌC LƯỚT!):**
   - Hai khóa học nằm tại `/Users/mizuhara/workspace/work/NotebookLM/`:
     - Khóa 1 (`48` file `.html` trong `Sources/`, ~280 KB): `Trading 1_ Phương pháp Medio (Price Action)`
     - Khóa 2 (`26` file `.html` trong `Sources/`, ~220 KB): `Trading 2_ Phương pháp Nukida (123bo)`
   - **Lưu ý kỹ thuật cực kỳ quan trọng trên macOS (APFS NFD Unicode):** Tên thư mục và tên file tiếng Việt trong `work/NotebookLM/` đang ở chuẩn **Unicode tổ hợp NFD** (`Phương pháp...`). Hãy dùng Python `os.listdir("/Users/mizuhara/workspace/work/NotebookLM")` / `os.walk` kết hợp bóc sạch thẻ HTML (giữ nguyên cấu trúc tiêu đề `h1..h6`, `ul/ol/li`, đoạn văn) để trích xuất và đọc **100% nguyên văn từng câu chữ của cả 48 bài Medio + 26 bài Nukida** (kể cả 2 file tổng hợp Slide/FAQ `00_Tai_Lieu_Tong_Hop_*` và các bài thực hành/bonus). Có thể chia 2–4 `research` subagents đọc song song và lưu bản ghi chép chi tiết từng cụm bài vào `/Users/mizuhara/workspace/sumi/docs/qmv_research/pa_digests/`.
3. **Đọc Ngữ Cảnh Kho Tri Thức QMV & Thiết Kế Sumi Đã Có Sẵn Trước Khi Dung Hợp:**
   - Đọc đầy đủ bộ tài liệu hiện có tại `/Users/mizuhara/workspace/sumi/docs/qmv_research/`:
     - `00_QMV_KNOWLEDGE_MASTER_INDEX.md`
     - `01_QMV_TECHNICAL_ANALYSIS_COMPLETE_DOCTRINE.md`
     - `02_QMV_TRADING_STRATEGIES_PLAYBOOK.md`
     - `03_QMV_CUSTOM_INDICATORS_SPECIFICATION.md`
     - `05_SUMI_QMV_WORKSTATION_BRD.md`
     - `06_NEXT_SESSION_IMPLEMENTATION_PROMPT.md`
     - `07_SUMI_AGENT_ARCHITECTURE_AND_SPEC.md`
4. **Trung Thực Học Thuật, Tôn Trọng Vi Cấu Trúc Thị Trường & 100% Nhân Quả (`Causal <= t`):**
   - Đầu tiên phải đúc kết **nguyên bản 100%** tư duy gốc của **Medio** và **Nukida (`123BO`)** kèm trích dẫn rõ tên bài học.
   - Sau đó, khi dung hợp với **QMV** để đưa lên **Sumi**, phải phân tích sắc bén sự khác biệt vi cấu trúc giữa **Forex/Crypto/Vàng** (đánh 2 chiều Long/Short, `T+0`, đòn bẩy cao, ít dùng Volume, đặt SL sát vài pip) với:
     - **(a) Cổ phiếu cơ sở Việt Nam (`HOSE ±7% / HNX ±10% / UPCOM ±15%`, chỉ đánh Long, kẹt thanh toán `T+2.5`, nhiễu râu nến `ATO/ATC`, có Volume & Dòng tiền tổ chức cực rõ)** $\rightarrow$ cách kết hợp `Keylevel Medio` + `123BO Nukida` với `VPA SMA5` & `Chia vốn 1/3 – 1/3 – 1/3` của QMV sao cho không bị quét SL oan mà tối ưu điểm bóp cò!
     - **(b) Hợp đồng tương lai `VN30F1M` (`HNX:VN30F1!`) & Crypto/FX trên Sumi** (vốn Sumi đã hỗ trợ sẵn `T+0` 2 chiều Long/Short) $\rightarrow$ nơi các setup `Medio` & `Nukida 123BO` nguyên bản phát huy 100% sức mạnh!
   - Mọi công thức toán học cho Custom Indicator (`Keylevel`, `BOS/CHoCH`, `123BO`) và Backtest Strategy phải **100% nhân quả tại nến đóng `t`** (tuyệt đối không dùng đỉnh/đáy tương lai chưa xác nhận!).

---

## 2. CÁC CHỦ ĐỀ TRỌNG TÂM BẮT BUỘC BÓC TÁCH SÂU TỪ MEDIO & NUKIDA

### A. Từ Khóa Học `Medio (Price Action)` (`48` bài):
1. **Triết lý tối giản của Medio, Quản lý vốn & Cách kiếm tiền với tài khoản nhỏ** (`Bài 00, 01, 15, 16`).
2. **Cách thị trường chuyển động (`Sóng đẩy vs. Sóng hồi`), Momentum, Trendline & Fibonacci kiểu Medio** (`Bài 03, 05, 06, 07`).
3. **Đọc Nến theo Bản chất Cung–Cầu Áp lực (không học vẹt tên nến)** (`Bài 04.2, 04.3`).
4. **Trái tim của Medio — Cấu trúc Thị trường & Xác định `Keylevel Quan Trọng` (`Important Keylevel`) + Trọn bộ `8 Bài Lọc Nhiễu Keylevel Quan Trọng` (bao gồm cả lọc nhiễu `GAP`, phá vỡ thật vs phá vỡ giả)** (`Bài 02.01..02.03` và `Bài 08.01..08.16`).
5. **Quản lý Stoploss Thông minh (dời SL theo cấu trúc swing, không dời bừa bãi)** (`Bài 08.15`).
6. **Hệ thống Điểm vào lệnh MasterClass & Chuẩn Medio:**
   - Vào lệnh khi xác nhận **Xu hướng hồi kết thúc** & Mô hình giá tiếp diễn (`Bài 09`).
   - Cách xác nhận **Xu hướng đảo chiều** thực sự (`Bài 10`).
   - Vào lệnh **Có Hợp lưu (`Confluence`)** vs **Không Hợp lưu** & **Ngược xu hướng (`Counter-Trend`)** (`Bài 11, 12`).
   - Quy trình phối hợp **Đa khung thời gian (`Multi-Timeframe`) chuẩn Medio** (`Bài 13..13.4`).

### B. Từ Khóa Học `Nukida Pro Training (123BO)` (`26` bài):
1. **Cách đọc thấu Cấu trúc Thị trường kiểu Nukida & Phân biệt sống còn giữa `"Vùng Mua Bán"` với `"Vùng Bò Gấu (Bull/Bear Zone)"`** (`Bài 01..05, 25`).
2. **Cấu trúc Mẫu hình `1-2-3 Breakout (123BO)` Nguyên bản:** Cách đếm nhịp `1 – 2 – 3`, điểm vào lệnh chuẩn và nguyên tắc đặt Dừng lỗ (`SL`) / Chốt lời (`TP`) (`Bài 06, 07, 19`).
3. **Hệ thống Bộ lọc Xác suất Cao của Nukida:**
   - **Trọn bộ `Các Yếu Tố Bên Trong (Internal Factors 1, 2, 3)`** (`Bài 08..11`) — giải mã cấu trúc bên trong của chính mô hình `123BO`.
   - **Trọn bộ `Các Yếu Tố Bên Ngoài (External Factors 1, 2, 3)`** (`Bài 13..16`) — bối cảnh sóng lớn, vị trí vùng, không gian chạy giá (`Room to Left` / cản đối diện).
4. **Kỹ thuật Thực thi Linh hoạt:** **Điểm vào lệnh Sớm (`Early Entry`)** vs **Điểm vào lệnh Muộn (`Limit / Retest Entry`)**, và danh mục **Các kịch bản `"Gặp là vào"`** (`Bài 12, 17, 18, 20`).
5. **Phương pháp Thống kê, Xây dựng Bộ Quy Tắc Cá Nhân & Thực hành Backtest Thực tế của Nukida** (`Bài 00, 21, 22, 23, 24`) — đặc biệt chú ý cách Nukida thống kê các biến số `Bên trong / Bên ngoài` trên bảng thống kê backtest, vì đây là chất liệu vàng để nâng cấp **`QMV/PA Trading Dojo`** và khả năng tự thống kê của **`Sumi Agent`**!

---

## 3. BỘ TÀI LIỆU ĐẦU RA CẦN TẠO MỚI & CẬP NHẬT TRONG `/Users/mizuhara/workspace/sumi/docs/qmv_research/`

Hãy tạo mới các tài liệu chuyên sâu sau (viết bằng tiếng Việt sắc sảo, định lượng rõ ràng) và cập nhật đồng bộ bộ hồ sơ kiến trúc của Sumi:

1. **Lưu trữ Ghi chép Chi tiết Bài giảng Gốc (`pa_digests/`):**
   - Lưu các bản bóc tách đầy đủ của Medio và Nukida vào `/Users/mizuhara/workspace/sumi/docs/qmv_research/pa_digests/` để bảo toàn 100% chi tiết ví dụ và FAQ cho các session sau.
2. **Tạo mới `09_MEDIO_NUKIDA_COMPLETE_PRICE_ACTION_DOCTRINE.md`:**
   - *"Toàn Thư Price Action Nguyên Bản: Phương Pháp Medio (`Keylevel & Cấu Trúc`) & Phương Pháp Nukida (`123BO, Yếu Tố Nội/Ngoại & Thống Kê Thực Chiến`)"* — trình bày đầy đủ, có hệ thống 100% kiến thức gốc của cả 2 khóa học.
3. **Tạo mới `10_UNIFIED_QMV_PA_TRADING_SYSTEM_AND_SPECS.md`:**
   - **Phần 1 — Ma trận Dung Hợp Tư Duy (`QMV × Medio × Nukida`):**
     - Chỉ rõ cách ghép 3 mảnh thành **1 Quy trình Giao dịch Cá nhân Thống nhất**: dùng `QMV` làm Bộ lọc Thượng nguồn (Vĩ mô, 9 Quả Bóng, `DT1/2/3`, Không chết/M-Score, Dòng tiền `TMC` & `VPA SMA5`), dùng `Medio` để xác định chính xác **`Keylevel Quan Trọng` đã lọc nhiễu & thời điểm Kết thúc sóng hồi**, và dùng `Nukida 123BO` (kèm kiểm tra `Yếu tố Bên trong + Bên ngoài`) làm **Bộ kích hoạt Điểm vào Sớm / Điểm vào Limit đa khung (`1D -> 1H/30m/15m`)**!
     - Bảng đối chiếu & giải quyết các điểm xung đột (VD: Cách kết hợp `Trailing SL theo Keylevel Medio` với `Chặn lãi 2/3 sóng QMV`; cách điều chỉnh `SL sát sau nhịp 2 của 123BO` dưới luật `T+2.5` cơ sở vs áp dụng trực tiếp 2 chiều cho `VN30F1M` phái sinh).
   - **Phần 2 — Đặc tả Toán học Nhân quả 100% (`Causal <= t`) cho Chỉ báo & Chiến lược Price Action Mới trên Sumi:**
     - Thiết kế công thức & pseudocode nhân quả cho các Custom Indicators bổ sung:
       1. `PA_MEDIO_KEYLEVEL_STRUCTURE`: Tự động nhận diện Cấu trúc Sóng đẩy/Sóng hồi, đánh dấu `Keylevel Quan Trọng` (đỉnh/đáy dẫn tới phá vỡ cấu trúc thật `BOS`), áp dụng các bộ lọc nhiễu của Medio (bao gồm xử lý `GAP`), và báo hiệu `CHoCH / Kết thúc sóng hồi`.
       2. `PA_NUKIDA_123BO_ENGINE`: Tự động phát hiện mẫu hình `1-2-3 Breakout` (cả chiều Thuận xu hướng lẫn Đảo chiều tại `Vùng Mua Bán`), chấm điểm tự động các `Yếu tố Bên trong` & `Yếu tố Bên ngoài`, vẽ sẵn mốc `Early Entry`, `Limit Entry`, `SL` chuẩn Nukida và đo khoảng trống tới cản đối diện (`R:R`).
     - Thiết kế các **Chiến lược Hợp nhất (`S9: QMV Flow + Medio Keylevel Pullback End`, `S10: QMV VPA + Nukida 123BO Confluence`, và `S11: Pure Medio/123BO Two-Way cho VN30F1M Phái sinh`)** sẵn sàng cho Backtest Engine của Sumi!
4. **Cập nhật Đồng bộ Không Ghi Đè Mất Nội Dung Cũ cho Bộ Hồ sơ Kiến trúc Sumi (`00`, `05`, `06`, `07`):**
   - Cập nhật `00_QMV_KNOWLEDGE_MASTER_INDEX.md` để bổ sung toàn bộ kho tri thức `Medio & Nukida` (`09`, `10` và `pa_digests/`).
   - Cập nhật `05_SUMI_QMV_WORKSTATION_BRD.md`: Bổ sung 2 chỉ báo Price Action mới (`PA_MEDIO_KEYLEVEL_STRUCTURE`, `PA_NUKIDA_123BO_ENGINE`), bổ sung Playbook `S9..S11`, và tích hợp **Bảng Thống Kê Yếu Tố Nội/Ngoại kiểu Nukida** vào `Trading Dojo` & `Journal`!
   - Cập nhật `07_SUMI_AGENT_ARCHITECTURE_AND_SPEC.md`: Trang bị thêm `Canonical Skills` về `Medio Keylevel` và `Nukida 123BO` cho `Sumi Agent`, để khi tôi hỏi về bất kỳ mã cổ phiếu hay hợp đồng `VN30F1M` nào, `Sumi Agent` biết phối hợp nhuần nhuyễn cả **Dòng tiền/VPA QMV + Keylevel Medio + 123BO Nukida** đúng với phong cách giao dịch gốc của tôi!
   - Cập nhật `06_NEXT_SESSION_IMPLEMENTATION_PROMPT.md` để phiên triển khai code sau này tự động nạp và hiện thực hóa trọn vẹn cả **QMV + Medio + Nukida (`123BO`) + `Sumi Agent`**.
```
