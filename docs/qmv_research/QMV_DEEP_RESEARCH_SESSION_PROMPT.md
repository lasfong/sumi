# PROMPT KHỞI TẠO PHIÊN NGHIÊN CỨU CHUYÊN SÂU PHƯƠNG PHÁP QMV, CHỈ BÁO RIÊNG & CHIẾN LƯỢC BACKTEST

Sao chép toàn bộ nội dung trong khối dưới đây để bắt đầu một **Session mới** (gợi ý: có thể gõ kèm `/goal` hoặc `/boost` ở đầu phiên để Agent đọc vét cạn toàn bộ kho bài giảng mà không rút gọn sớm):

---

````markdown
Bạn đóng vai **Principal Quantitative Researcher, Chief Market Technician & QMV Trading System Architect**.

Mục tiêu tối thượng của phiên làm việc này là **nghiên cứu sâu đến tận gốc rễ toàn bộ kho tri thức và bài giảng các khóa học QMV** trong hệ thống **Yumeko** (`/Users/mizuhara/workspace/yumeko`), đặt **trọng tâm cốt lõi vào Phân Tích Kỹ Thuật (PTKT), Phân Tích Giá – Khối Lượng – Dòng Tiền và Thực Thi Danh Mục**, từ đó:
1. Đúc kết **Phương pháp Giao dịch chuẩn chỉnh 100% theo đúng tinh thần và bài giảng gốc của hệ thống QMV** (không pha tạp lý thuyết PTKT sách vở đại trà nếu QMV dạy khác).
2. Xây dựng **Bộ Chiến lược Giao dịch QMV cụ thể** (từ điều kiện bối cảnh, vùng mua/bán, chia vốn giải ngân, cắt lỗ, chốt lời, đánh giá `T+` đến điều kiện hủy kịch bản).
3. Thiết kế **Đặc tả Toán học cho Bộ Chỉ báo Độc quyền (Custom QMV Indicators)** và **Đặc tả Luật Định lượng Nhân quả (Backtestable Strategy Specs)** để tôi triển khai tiếp lên workstation phân tích kỹ thuật & backtest **Sumi** (`/Users/mizuhara/workspace/sumi`).

---

## 1. NGUYÊN TẮC NGHIÊN CỨU BẮT BUỘC (KHÔNG ĐỌC LƯỚT!)

1. **Đọc trực tiếp Bài giảng Gốc (`Slide PDF.md` + `Audio Transcript .mp3.md`), KHÔNG chỉ đọc bản tóm tắt Wiki:**
   - Các tệp trong `docs/qmv_method/` và `knowledge_factory/05_curated_wiki/` của Yumeko trước đây được biên tập theo hướng an toàn cho AI Advisor (*"không biến tín hiệu thành lệnh hành động"*), nên nhiều kỹ thuật thực chiến sắc bén nhất của giảng viên QMV nằm ở **bài giảng âm thanh gốc và phần Hỏi–Đáp (`hoidap` / `thaoluan`)** trong `docs/research/qmvcourse/`.
   - **Lưu ý kỹ thuật cực kỳ quan trọng khi đọc file `.mp3.md`:** Nhiều file transcript âm thanh trong `qmvcourse/` được lưu trên **1 dòng rất dài (50KB – 200KB+)**. Tuyệt đối không để công cụ đọc file cắt cụt ở 46KB đầu rồi bỏ qua phần còn lại — bắt buộc đọc nối tiếp bằng `ByteOffset` hoặc dùng script Python / chia nhiều `research` subagents (`run_workflow` / `invoke_subagent`) đọc song song từng học phần để **bao phủ trọn vẹn 100% nội dung bài giảng**.
2. **Chế độ Read-Only với Yumeko & Chưa vội sửa code Sumi:**
   - Tuyệt đối không sửa đổi bất kỳ file nào trong `/Users/mizuhara/workspace/yumeko`.
   - Trong phiên này, tập trung **100% công sức vào Nghiên cứu Sâu & Thiết kế Đặc tả Định lượng**, lưu bộ tài liệu đầu ra tại `/Users/mizuhara/workspace/sumi/docs/qmv_research/`. Chỉ đọc mã nguồn `/Users/mizuhara/workspace/sumi` ở bước cuối để đối chiếu khoảng cách (`Gap Analysis`) giữa tri thức gốc QMV và những gì Sumi hiện đang có.
3. **Trung thực tri thức & Ranh giới dữ liệu minh bạch:**
   - Mọi quy tắc, mẫu hình, công thức hay con số đưa ra **phải kèm trích dẫn nguồn bài giảng QMV cụ thể** (tên file bài giảng/chuyên đề).
   - Tách bạch rõ ràng: **(a) Dữ liệu OHLCV + Khối lượng + VNINDEX thuần túy** (có thể tính chính xác 100% thành Custom Indicator & chạy Backtest lịch sử 10 năm trên Sumi) và **(b) Dữ liệu độc quyền của Webapp QMV Fintech** (`MB`, `BB`, `TM`, `XH Cầu`, `DM/DS/EM`, `TMC`, `DSPI`, `MPICP` — nêu rõ ý nghĩa gốc và thiết kế công thức xấp xỉ `OHLCV Proxy` tốt nhất nếu cần đưa vào Backtest).
4. **Chống nhìn trước tương lai (`100% Causal`) & Khớp vi cấu trúc TTCK Việt Nam:**
   - Mọi công thức Custom Indicator và Chiến lược Backtest phải tính toán nhân quả tại thời điểm đóng nến `t` (không dùng đỉnh/đáy tương lai chưa xác nhận), và tính đến đặc thù Việt Nam (`HOSE ±7%`, `HNX ±10%`, `UPCOM ±15%`, khóa trần/sàn mất thanh khoản, lô 100, chu kỳ hàng về `T+2`, phí/thuế).
5. Toàn bộ báo cáo và đặc tả viết bằng **tiếng Việt** sâu sắc, mạch lạc, định lượng rõ ràng.

---

## 2. BẢN ĐỒ KHO TRI THỨC QMV BẮT BUỘC PHẢI ĐỌC KỸ TRONG YUMEKO

Hãy tổ chức các `research` subagents đọc có hệ thống toàn bộ các nguồn sau trong `/Users/mizuhara/workspace/yumeko` (có thể kết hợp thêm MCP `yumeko-readonly` như `memory_search_method` để tra cứu chéo):

### Khối A — TRỌNG TÂM SỐ 1: Toàn bộ Học phần 4 & Chuyên đề Phân Tích Kỹ Thuật (`PTKT` & `KT01`)
1. **Chuyên đề PTKT Mới Nhất `KT01` (4 Slide + 4 Audio bài giảng tháng 07/2025 — BẮT BUỘC ĐỌC TRỌN VẸN 100%):**
   - `/Users/mizuhara/workspace/yumeko/docs/research/qmvcourse/qmv_chuyende/KT01T1_-_Phân_tích_cung_cầu_và_trò_chơi.pdf.md` & `PTKT_Buổi_1_KT01T1_-_Phân_tích_cung_cầu_và_trò_chơi_02072025_audio.mp3.md`
   - `/Users/mizuhara/workspace/yumeko/docs/research/qmvcourse/qmv_chuyende/KT01T2_-_Xác_định_vùng_giao_dịch_-_Mua.pdf.md` & `PTKT_Buổi_2_KT01T2_-_Xác_định_vùng_giao_dịch_-_Mua_04072025_audio.mp3.md`
   - `/Users/mizuhara/workspace/yumeko/docs/research/qmvcourse/qmv_chuyende/KT01T3_-_Xác_định_vùng_giao_dịch_-_Bán.pdf.md` & `PTKT_Buổi_3_KT01T3_-_Xác_định_vùng_giao_dịch_-_Bán_09072025_audio.mp3.md`
   - `/Users/mizuhara/workspace/yumeko/docs/research/qmvcourse/qmv_chuyende/KT01T4_-_Xây_dựng_danh_mục_giao_dịch.pdf.md` & `PTKT_Buổi_4_KT01T4_-_Xây_dựng_danh_mục_giao_dịch_11072025_audio.mp3.md`
   - `/Users/mizuhara/workspace/yumeko/docs/research/qmvcourse/qmv_chuyende/Thực_Hành_Ra_Quyết_Định_Với_QMV.md`
2. **Học phần 4 — Phân Tích Kỹ Thuật & Dòng Tiền (`49` file qua 3 phiên bản `v1`, `v2`, `v3` — kho báu kỹ thuật thực chiến):**
   Thư mục: `/Users/mizuhara/workspace/yumeko/docs/research/qmvcourse/qmv_hp_4_phan_tich_ky_thuat_dong_tien/`
   - **Giá & Lượng (`VPA`), Hỗ trợ/Kháng cự, Đường xu hướng:** `v1_5_*`, `v2_1_*` (4 bài), `v2_6_on_luyen_1_6_onluyenvpa*`, `v3_1_*` (Bài giảng + Hỏi đáp + Thảo luận).
   - **Đồ thị Nến & Kỹ thuật Ghép Nến:** `v1_4_*`, `v2_2_*` (đặc biệt `v2_2_phan_tich_do_thi_nen_2_kythuatghepnen*`), `v3_2_*` (Bài giảng + Hỏi đáp + Thảo luận).
   - **Các Chỉ số Kỹ thuật (`MA`, `Bollinger Bands`, `Ichimoku`, Xu hướng vs Dao động):** `v1_3_*`, `v2_3_*` (4 bài), `v3_3_*` (Bài giảng + Hỏi đáp).
   - **Mẫu hình Kỹ thuật:** `v1_2_*`, `v2_4_*` (4 bài), `v3_4_*` (Bài giảng + Hỏi đáp + Thảo luận).
   - **Khoảng trống giá (`Gaps`), Breakout, Lý thuyết Dow & Sóng Elliott:** `v1_1_*`, `v2_5_*` (4 bài), `v3_5_*` (2 bài).
   - **Tổng hợp Xây dựng Chiến lược Giao dịch & 6 Bước PTKT:** `v2_6_on_luyen_1_1..1_5*` (đặc biệt `xaydungclgd` & `ungdung6buocptkt`), `v3_6_tong_hop_van_dung_*` (Bài giảng + Hỏi đáp + Thảo luận).

### Khối B — Trò Chơi Dòng Tiền (`HP1`), Thực Thi Danh Mục (`HP5`, `DM01`) & Bối Cảnh Chu Kỳ (`HP2/VM01`, `HP3/DN01`)
1. **Học phần 1 — Trò chơi dòng tiền (`Cáo – Thỏ – Chó săn`, Tài chính hành vi, Thông tin bất cân xứng):**
   - `/Users/mizuhara/workspace/yumeko/docs/research/qmvcourse/qmv_hp_1_tro_choi_dong_tien/` (đặc biệt các bài `tro_choi_dong_tien_*` và `version_3_062023_*`).
2. **Học phần 5 & Chuyên đề `DM01` — Phân nhóm Dòng tiền (`DT1/DT2/DT3`), Xây dựng Chiến lược & Thực thi Danh mục:**
   - `/Users/mizuhara/workspace/yumeko/docs/research/qmvcourse/qmv_hp_5_xay_dung_danh_muc/` (đặc biệt `version_1_3`, `version_1_4`, `version_2_72022_4`, `version_3_42023_3_luachonnhomdongtien*`, `version_3_42023_4_xaydungchienluocgiaodich*`).
   - `/Users/mizuhara/workspace/yumeko/docs/research/qmvcourse/qmv_chuyende/DM01T1..T4*.md` (Slide + Audio `12/2025`, đặc biệt `DM01T2` và `DM01T4_-_Thực_thi_danh_mục`).
3. **Bối cảnh Vĩ mô/Ngành (`VM01`) & Lọc Doanh nghiệp/Trà đá (`DN01`) đủ để làm Cổng Lọc (`Regime & Quality Gates`) cho PTKT:**
   - `VM01T2..T4` (Chu kỳ chính sách tiền tệ & Luân chuyển ngành), `DN01T1..T4` (Nhận diện công ty "trà đá" / M-Score để tránh bẫy phân tích kỹ thuật trên cổ phiếu rác làm giá).

### Khối C — Hệ Thống QMV Fintech (`V4 / V5 / V7`), Case Studies Thực Chiến 2025–2026 & Memory Thực Nghiệm
1. **Tài liệu Hệ thống Fintech & Chọn cổ phiếu:**
   - `/Users/mizuhara/workspace/yumeko/docs/research/qmvcourse/qmv_others/HƯỚNG_DẪN_SỬ_DỤNG_HỆ_THỐNG_THÔNG_TIN_QMV_-_V7.pdf.md`
   - `/Users/mizuhara/workspace/yumeko/docs/research/QMV_SYSTEM_GUIDE_V5.md`
   - `/Users/mizuhara/workspace/yumeko/docs/research/qmvcourse/qmv_chuyende/Chọn_cổ_phiếu_Fintech_QMV_17072025.mp3.md`
   - `/Users/mizuhara/workspace/yumeko/docs/research/qmvcourse/qmv_others/fintech_buoi_1_21052026_audio.mp3.md` & `Fintech_buổi_2_25062026_audio.mp3.md`
2. **Các Buổi Chuyên Đề Thực Chiến & Bắt Đáy / Phân Tích Mã Cụ Thể (`qmv_others`):**
   - `wa_10_02_2026_ky_thuat_bat_day_an_toan_audio.mp3.md` (**Kỹ thuật bắt đáy an toàn!**)
   - `wa_22_01_2026_vci_ssi_audio.mp3.md`, `tra_da_via_he_cung_qmv_04032026_tcb_audio.mp3.md`, `tra_da_via_he_cung_qmv_26022026_chu_ky_nganh_bds_2026_audio.mp3.md`, `wa_16_01_26_nganh_ban_le_chu_ky_but_pha_audio.mp3.md`, `tradaviahe_mscore_audio.mp3.md`.
3. **Kho Tinh Chế, Sổ tay Vận hành & Bài học Postmortem trong Yumeko (đối chiếu kiểm chứng):**
   - `/Users/mizuhara/workspace/yumeko/QMV_System_Reference.md`
   - `/Users/mizuhara/workspace/yumeko/docs/qmv_method/` (`QMV_COURSE_KNOWLEDGE_INVENTORY_2026-07-06.md`, `QMV_TRADING_SYSTEM_RULEBOOK_2026-07-06.md`, `QMV_BUOC3_SO_TAY_VAN_HANH_HE_THONG_QMV_CHO_TRADER_2026-07-06.md`, `QMV_TRADING_CHECKLISTS_AND_PLAYBOOK_2026-07-06.md`)
   - `/Users/mizuhara/workspace/yumeko/knowledge_factory/03_working_notes/KT01/`, `04_curated_memory/KT01_memory.jsonl`, `05_curated_wiki/{frameworks,insights}/KT01-*`, và đặc biệt các ghi chú đối chiếu chi tiết từng file tại `knowledge_factory/07_rebuild_candidates/QMV_COURSE_HP4_2026-05-28/` (`HP4_FILE_LEVEL_NOTES.md`, `HP4_HP1_TIMING_CROSS_CHECK.md`) & `QMV_COURSE_HP5_2026-05-28/HP5_HP1_HP4_RISK_SIZING_CROSS_CHECK.md`.
   - `/Users/mizuhara/workspace/yumeko/memory/` (`archive_2026_05/PATTERN_BANK.md`, `patterns/`, `lessons/lesson_postmortem_*.json`, `regime_journal/`).

---

## 3. BỘ 4 TÀI LIỆU ĐẦU RA CẦN XUẤT (LƯU TẠI `/Users/mizuhara/workspace/sumi/docs/qmv_research/`)

Sau khi đọc kỹ toàn bộ kho học liệu trên, hãy biên soạn **4 tài liệu chuyên sâu, chi tiết đến mức cầm vào là giao dịch và lập trình được ngay**:

### Tài liệu 1: `01_QMV_TECHNICAL_ANALYSIS_COMPLETE_DOCTRINE.md`
**("Toàn Thư Phân Tích Kỹ Thuật & Dòng Tiền Nguyên Bản Hệ Thống QMV")**
- Đúc kết đầy đủ, có chiều sâu và có trích dẫn nguồn bài giảng cho từng mảnh ghép PTKT của QMV:
  1. **Triết lý gốc về Giá, Khối lượng & Trò chơi Dòng tiền (`HP1 + KT01T1`):** Mô hình `Cáo – Thỏ – Chó săn`, 4 pha vận động của nhà tạo lập (Gom – Đẩy – Xả – Đè), đặc tính hành vi giá-lượng khác biệt giữa `DT1`, `DT2`, `DT3` và cổ phiếu `Trà đá`.
  2. **Quy trình 6 Bước PTKT & Khung Ra Quyết Định Top-Down của QMV (`HP4 v2_6 + Thực Hành Ra Quyết Định`):** Trình tự chuẩn từ nhìn Thị trường chung → Ngành → Nhóm dòng tiền → Đa khung thời gian (`Tháng / Tuần / Ngày / Intraday`) → Vùng giao dịch → Tín hiệu hành động.
  3. **Nghệ thuật Đọc Nến, Ghép Nến & Phân tích Giá – Lượng (`VPA` kiểu QMV — `HP4 v2_1, v2_2, v3_1, v3_2`):** Toàn bộ quy tắc **Ghép nến** của QMV, cách đọc thân nến/bóng nến đặt trong tương quan khối lượng và vị trí (tại nền tích lũy vs tại đỉnh hưng phấn vs tại hỗ trợ/kháng cự), dấu hiệu cạn cung, giữ cung, rũ bỏ (`shakeout`), kéo xả (`upthrust`).
  4. **Cấu trúc Thị trường: Hỗ trợ/Kháng cự, Quy tắc `3 Điểm Chạm`, Đường Xu hướng, Vùng Tắc Nghẽn, Khoảng Trống Giá (`Gaps`), Lý thuyết Dow & Sóng Elliott thực dụng (`HP4 v2_4, v2_5 + KT01T2, KT01T3`):** Cách QMV vẽ và xác nhận vùng mua/vùng bán chuẩn xác, phân biệt Breakout thật vs Bulltrap/Beartrap, các mẫu hình giá mà giảng viên QMV tin dùng nhất vs các mẫu hình hay lừa nhỏ lẻ.
  5. **Hệ thống Chỉ báo Kỹ thuật trong QMV (`HP4 v2_3, v3_3 + V7 Guide`):** Cách QMV phân nhóm chỉ báo (Xu hướng vs Dao động), bộ tham số gốc mà QMV giảng dạy cho `MA`, `Bollinger Bands`, `Ichimoku`, `MACD`, `RSI`, `CCI`..., khi nào chỉ báo xu hướng đúng/sai, khi nào chỉ báo dao động bị "liệt" trong sóng mạnh, và cách kết hợp với hệ thống biến số QMV Fintech (`MB`, `BB`, `TM`, `XH Cầu`, `DM/DS/EM`, `TMC`, `DSPI`, `MPICP`).
  6. **Đi vốn, Chia 3 Phần, Quản trị Rủi ro & Kỷ luật Đánh giá `T+` (`HP5 + KT01T4 + DM01T4`):** Phân biệt *"Lý do mua (Thesis)"* và *"Tín hiệu mua (Trigger)"*, quy tắc giải ngân `3 phần`, đặt cắt lỗ ban đầu & dời điểm chặn lãi, quy trình kiểm định lại lệnh khi hàng về `T+2 / T+3 / T+5`, và tổng hợp **Top các sai lầm thực chiến phổ biến nhất của học viên** từ các buổi Hỏi–Đáp.

### Tài liệu 2: `02_QMV_TRADING_STRATEGIES_PLAYBOOK.md`
**("Hệ Thống Chiến Lược Giao Dịch Cụ Thể Theo Phương Pháp QMV")**
- Xây dựng bộ **Chiến lược Giao dịch Cụ thể (từ 6 đến 10 chiến lược hoàn chỉnh)** phủ kín các bối cảnh mà QMV dạy:
  - Ví dụ: *(1) Mua tích lũy nền / Hỗ trợ cứng cạn cung chia 3 phần (`KT01T2`), (2) Bắt đáy an toàn khi hoảng loạn / Phân kỳ có kiểm soát rủi ro (`wa_10_02_2026` + `KT01`), (3) Đánh theo xu hướng bám MA / Bollinger / Mây Ichimoku cho sóng trung hạn, (4) Đánh Breakout thoát vùng tắc nghẽn / Breakaway Gap có xác nhận dòng tiền, (5) Đánh nhịp luân chuyển dòng tiền `T+` trong vùng biên độ (`Swing/Sideway`), (6) Chiến lược Bán chủ động tại vùng kháng cự & Bán phòng vệ khi gãy cấu trúc (`KT01T3`), (7) Chiến lược Rebalancing trên cổ phiếu sẵn có trong danh mục (`DM01T4`)...*
- Với **mỗi chiến lược**, trình bày chặt chẽ theo 2 lớp song song:
  1. **Playbook Thực Chiến Cho Trader:** Bối cảnh Vĩ mô/VNINDEX/Ngành cho phép đánh, Loại cổ phiếu phù hợp (`DT1 / DT2 / DT3` — cấm áp dụng cho nhóm nào), Checklist điều kiện cần & đủ trước giờ đặt lệnh, Điểm mua Phần 1 / Phần 2 / Phần 3, Công thức Cắt lỗ, Mục tiêu Chốt lời từng phần, Lịch kiểm tra `T+`, Điều kiện Hủy kịch bản (`Invalidation`).
  2. **Công thức Định lượng Hóa 100% Nhân quả (`Quant/Backtest Logic`):** Biểu thức toán học tường minh tại nến `t` (chỉ dùng dữ liệu lịch sử `<= t`) để máy tính có thể quét tự động và chạy Backtest khách quan trên 10 năm lịch sử!

### Tài liệu 3: `03_QMV_CUSTOM_INDICATORS_SPECIFICATION.md`
**("Thiết Kế Bộ Chỉ Báo Kỹ Thuật Độc Quyền Của Riêng Tôi Theo QMV — Custom Indicators Spec")**
- Từ những kỹ thuật tinh túy nhất trong bài giảng QMV mà các phần mềm biểu đồ thông thường (TradingView mặc định) **không có sẵn**, hãy thiết kế **5 – 8 Custom Indicators độc quyền** dành riêng cho phương pháp QMV.
- Gợi ý các hướng đào sâu từ bài giảng (bạn tự do sáng tạo/chỉnh sửa dựa trên những gì đọc được từ khóa học):
  - *Indicator 1:* **Ghép Nến Đa Phiên & Thanh Áp Lực Cung–Cầu QMV (`QMV Merged Candle & VPA Bar`)** — tự động hóa kỹ thuật ghép nến của Học phần 4 (`v2_2`) và phân loại nến Giá-Lượng (Nỗ lực vs Kết quả, Cạn cung, Giữ cung, Upthrust, Shakeout).
  - *Indicator 2:* **Tự Động Vẽ Vùng Giao Dịch Mua/Bán, `3 Điểm Chạm` & Hộp Tắc Nghẽn (`QMV Trading Zone & 3-Touch Structure`)** — tự động hóa bài giảng `KT01T2` & `KT01T3`.
  - *Indicator 3:* **Radar Pha Trò Chơi Dòng Tiền `Cáo – Thỏ` (`QMV Smart-Money Cycle: Gom / Đẩy / Phân Phối / Đè`)** — kết hợp cấu trúc nền giá, biên độ nến, khối lượng tương đối và chu kỳ dòng tiền.
  - *Indicator 4:* **Bộ Chỉ Báo Xu Hướng – Dao Động Hợp Lưu Chuẩn QMV (`QMV Trend-Oscillator Regime Ribbon`)** — giải quyết bài toán giảng viên dạy trong `HP4 v2_3`: tự động chuyển chế độ ưu tiên chỉ báo Xu hướng (`MA/BB/Ichimoku`) khi có sóng và chỉ báo Dao động (`RSI/CCI`) khi đi ngang trong hộp!
  - *Indicator 5:* **Bộ Đo Xấp Xỉ Các Biến Độc Quyền QMV Fintech Từ OHLCV + Ngành (`QMV Blackbox Proxy Suite`)** — phân tích sâu cách xấp xỉ tốt nhất cho `Xu hướng Giá`, `BB Status`, `XH Cầu`, `Sức mạnh Ngành / Luân chuyển DT1→DT2→DT3` từ dữ liệu nến của toàn bộ 475 mã trên sàn VN!
- Mỗi Indicator phải có: **Cơ sở bài giảng QMV → Công thức toán học từng bước (Causal Pseudocode) → Tham số tinh chỉnh → Cách hiển thị trực quan trên biểu đồ (Overlay trên nến hay Sub-pane, màu sắc, nhãn cảnh báo) → Cách kết hợp vào Bộ lọc & Chiến lược.**

### Tài liệu 4: `04_SUMI_GAP_ANALYSIS_AND_IMPLEMENTATION_ROADMAP.md`
**("Đối Chiếu Với Sumi Hiện Tại & Lộ Trình Nâng Cấp Indicator / Backtest Engine")**
- Ở bước cuối cùng, hãy đọc kiểm tra mã nguồn hiện tại của `/Users/mizuhara/workspace/sumi`:
  - `crates/sumi-core/src/{indicators,scanner,backtest}/`
  - `ui/src/chart/indicators/{catalog.ts,customIndicators.ts}`
  - `ui/src/components/dock/{StockScreenerTab.tsx,StrategyTesterTab.tsx}`
- Đối chiếu thẳng thắn và chi tiết với Tài liệu 1, 2, 3 vừa đúc kết:
  1. Những gì Sumi vừa làm ở bản QMV v1 đã đúng hay **đang hiểu chưa tới / lệch so với bài giảng gốc `HP4` & `KT01`**? (Ví dụ: Sumi mới chỉ khớp lệnh 1 cục 100% vốn trong Backtest, trong khi QMV dạy giải ngân **chia 3 phần** và kiểm định **`T+`**; hay bộ lọc `VSA` / `Ichimoku` / `Vùng mua KT01` hiện tại còn thiếu điều kiện tinh tế nào trong bài giảng âm thanh?).
  2. Bảng xếp hạng ưu tiên (Roadmap) các nâng cấp đáng giá nhất lên `sumi-core` (Rust) và `KLineChart` (TypeScript) để tôi duyệt triển khai ở phiên kế tiếp.

---

Hãy bắt đầu bằng việc lập kế hoạch đọc vét cạn toàn bộ kho bài giảng QMV trong `/Users/mizuhara/workspace/yumeko` theo từng cụm học phần (sử dụng song song các `research` subagents và đảm bảo không bị cắt dòng trên các file `.mp3.md` dài), sau đó tổng hợp thành 4 tài liệu nghiên cứu & thiết kế xuất sắc nhất.
````
