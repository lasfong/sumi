# TOÀN THƯ PHÂN TÍCH KỸ THUẬT & DÒNG TIỀN NGUYÊN BẢN HỆ THỐNG QMV
**(`01_QMV_TECHNICAL_ANALYSIS_COMPLETE_DOCTRINE.md`)**

> **Tuyên ngôn Học thuật & Phạm vi Tài liệu:**
> Tài liệu này đúc kết **100% nguyên bản** phương pháp Phân tích Kỹ thuật (PTKT), Phân tích Giá – Khối lượng – Dòng tiền (VPA/Blackbox) và Quản trị Thực thi Danh mục của **Hệ thống QMV** (GS. Quách Mạnh Hào, cùng đội ngũ giảng viên/trợ giảng Đức Lê, Dũng, Nhật, Trường Đào, Puka), dựa trên việc đọc vét cạn toàn bộ kho bài giảng gốc (`Slide PDF` + `Audio Transcript .mp3` + `Hỏi–Đáp / Thảo luận` + `Hướng dẫn V5/V7` + `Hậu kiểm thực nghiệm T+20`) trong hệ thống Yumeko.
> - **Không pha tạp sách vở đại trà:** Mọi chỗ QMV dạy khác với giáo trình PTKT phương Tây (như bản chất người quyết định khối lượng, nến trần/sàn không râu, không dùng cắt lỗ cố định `-7%`, vùng mua `EMA200` tuần, hay lý do hệ thống QMV không thiết kế `Bearish Signal 4`) đều được ghi nhận trung thực theo đúng lập luận kinh tế học và thực chiến TTCK Việt Nam của QMV.
> - **Minh bạch ranh giới dữ liệu:** Tách bạch rõ ràng giữa **(A) Quy tắc tính thuần túy từ `OHLCV + Khối lượng + VN-Index/Ngành`** (có thể lập trình chính xác 100% và kiểm định Backtest 10 năm lịch sử) và **(B) Hệ biến số độc quyền của Webapp QMV Fintech (`Blackbox`)** kèm cơ chế toán học gốc do giảng viên tiết lộ.

---

## MỤC LỤC TỔNG QUAN
1. **Chương 1:** Triết lý Gốc về Giá, Khối lượng, Trò chơi Dòng tiền (`Cáo – Chó săn – Thỏ`), Mô hình `9 Quả Bóng` & Cổng Lọc Vĩ mô – Chất lượng Doanh nghiệp (`HP1 + KT01T1 + HP5 + VM01 + DN01`)
2. **Chương 2:** Quy trình 6 Bước PTKT, 7 Bước Thực hành & Khung Ra Quyết Định Top-Down Đa Khung Thời Gian (`HP4 v2_6, v3_6 + KT01T4 + Thực Hành Ra Quyết Định`)
3. **Chương 3:** Nghệ thuật Đọc Nến, Kỹ thuật Ghép Nến & Phân tích Giá – Lượng (`VPA` Kiểu QMV — `KT01T1, KT01T3 + HP4 v1_4, v1_5, v2_1, v2_2, v3_1, v3_2 + wa_10_02_2026`)
4. **Chương 4:** Cấu trúc Thị trường: Hỗ trợ/Kháng cự, Quy tắc `3 Điểm Chạm`, 36 Mẫu hình Giá, Khoảng Trống Giá (`Gaps`), Đột phá (`Breakout`), Lý thuyết Dow & Sóng Elliott Thực dụng (`KT01T2, KT01T3 + HP4 v1_1, v1_2, v2_4, v2_5, v3_4, v3_5`)
5. **Chương 5:** Hệ thống Chỉ báo Kỹ thuật trong QMV & Giải mã Toàn diện Hộp Đen QMV Fintech (`HP4 v1_3, v2_3, v3_3 + KT01 + V5/V7 Guide + Fintech Buổi 1 & 2`)
6. **Chương 6:** Đi vốn Chia 3 Phần, Quản trị Rủi ro, Chặn Lãi, Rebalancing, Kỷ luật Đánh giá `T+` & 15 Sai lầm Thực chiến Kinh điển (`HP5 + KT01T2..T4 + DM01T4 + wa_10_02_2026 + Postmortem`)

---

## CHƯƠNG 1: TRIẾT LÝ GỐC VỀ GIÁ, KHỐI LƯỢNG, TRÒ CHƠI DÒNG TIỀN & MÔ HÌNH 9 QUẢ BÓNG

### 1.1. Phương trình Hình thành Giá & Bản chất Kinh tế học của TTCK Việt Nam
*(Nguồn: `KT01T1_-_Phân_tích_cung_cầu_và_trò_chơi.pdf.md`, `PTKT_Buổi_1_KT01T1_02072025_audio.mp3.md`, `DM01_BUỔI_4_THỰC_THI_DANH_MỤC_26122025_audio.mp3.md`, `v3_062023_taichinhhanhvi_baigiang_audio.mp3.md`)*

1. **Phương trình 3 nhân tố quyết định Thị giá (xếp theo đúng thứ tự trọng số tác động):**
   $$\text{Thị giá Cổ phiếu } (P_t) = f\Big(\underbrace{\text{Nguồn tiền}}_{\text{Số 1: Quyết định xu hướng}},\; \underbrace{\text{Định giá}}_{\text{Số 2: Tài sản đảm bảo dài hạn}},\; \underbrace{\text{Tâm lý}}_{\text{Số 3: Dao động ngắn hạn}}\Big)$$
   - **Khác biệt cốt lõi với kinh tế học vi mô sách vở:** Trong sách giáo khoa, giá thay đổi làm lượng cầu/cung trượt *dọc theo* đường cầu/cung cố định — trên TTCK Việt Nam, hiện tượng trượt dọc này chỉ tương ứng với **3 bước giá hiển thị trên bảng điện tử trong 1 phiên bình thường**, hoàn toàn không tạo ra xu hướng (`KT01T1 Audio`). Xu hướng tăng/giảm thực sự bắt buộc phải do **cả đường Cầu hoặc cả đường Cung dịch chuyển sang phải/trái** bởi 3 nhân tố ngoài giá:
     - **Nguồn tiền (Chính sách tiền tệ, lãi suất, tín dụng, thanh khoản liên ngân hàng):** Quyết định số 1, tạo ra xu hướng mạnh và kéo dài nhất (*"Cầu chỉ tăng khi có tiền"* — `hoc_phan_4_gioithieutongquanhp4_audio.mp3.md`).
     - **Định giá (KQKD thực chất & lãi suất chiết khấu $r$):** Tác động chậm theo Quý/Năm, đóng vai trò **"Tài sản đảm bảo / Bảo hiểm"** khi thị trường yếu hoặc điều chỉnh (`KT01T4 Audio`).
     - **Tâm lý đám đông:** Tác động cực ngắn nhưng phi tuyến tính. Khác với Định luật III Newton (lực và phản lực bằng nhau), phản ứng tâm lý trên TTCK luôn bị phóng đại thành **Quá lên (Overshoot / Hưng phấn)** hoặc **Quá xuống (Undershoot / Sợ hãi)** xoay quanh trục **Trung bình / Hợp lý** — đây là gốc rễ sinh ra **Nguyên lý Con số 3** xuyên suốt toàn bộ hệ thống QMV (`KT01T1 Slide` L333).
   - **Hệ quả thực chiến:** Nếu giá bứt ra khỏi vùng cân bằng mà *không có sự thay đổi về Nguồn tiền hay Định giá (chỉ thuần túy do Tâm lý)*, sau khi cảm xúc qua đi, **giá chắc chắn quay trở về vùng cân bằng cũ** (`KT01T1 Audio`).

2. **Thông tin bất cân xứng (Akerlof – Spence – Stiglitz) & Triết lý kiếm tiền từ "Sự vô lý":**
   - Dựa trên lý thuyết *Thị trường Quả chanh (The Market for Lemons)*: Do **Lựa chọn đối nghịch (Adverse Selection)** và **Rủi ro đạo đức (Moral Hazard)**, các doanh nghiệp tồi/"giả vờ tốt" luôn có động lực chi tiền PR, vẽ bánh dự án và in giấy phát hành mạnh nhất; trong khi doanh nghiệp "tốt thật" tuân thủ nghiêm **Thuyết Trật tự Phân hạng Vốn (Pecking Order Theory)**: *(1) Lợi nhuận giữ lại $\rightarrow$ (2) Vay nợ ngân hàng (khi $\text{ROE} > \text{lãi vay}$) $\rightarrow$ (3) Hạn chế tối đa phát hành cổ phiếu mới pha loãng* (`KT01T1 Slide` L47–166, `v3_062023_thongtinkhongcanxung_audio.mp3.md`).
   - **Định vị bản thân là "Thỏ thông minh":** Luôn mặc định mình là bên thiếu thông tin gốc; mọi tin tức miễn phí đến tai mình đều là **tín hiệu thứ cấp do ai đó cố tình phát ra (*Signaling*)** theo cấu trúc hình tháp đa cấp (*người phím tin bắt buộc phải vào hàng trước bạn*).
   - **Chân lý phân tách `Trading` và `Investing` (`KT01T1 Audio`):**
     > *"Hợp lý sẽ đến trong dài hạn, nhưng vô lý luôn tồn tại trong ngắn hạn. Và chúng ta kiếm được tiền vượt trội trong ngắn hạn chủ yếu là từ SỰ VÔ LÝ chứ không phải từ sự hợp lý!"*
     - **Giao dịch ngắn hạn (`Trading`, $< 1$ năm — Trò chơi tổng bằng 0):** Chọn theo **Dòng tiền & Sự vô lý đang được đám đông đồng thuận**, rút dứt khoát trước khi sự vô lý sụp đổ. **Tuyệt đối không lướt sóng cổ phiếu quá hợp lý/nặng mông (`DT1` như VNM)** vì biên độ hẹp không bõ phí và rủi ro `T+`.
     - **Đầu tư dài hạn (`Investing` — Trò chơi tổng khác 0 nhờ thặng dư lợi nhuận & cổ tức):** Chọn **Doanh nghiệp tốt thật (4–5 sao) dưới Định giá**. **Tuyệt đối cấm ôm dài hạn cổ phiếu thời trang / đầu cơ / giả vờ tốt!**

---

### 1.2. Cơ chế Vận hành Trò chơi Dòng tiền `Cáo – Chó săn – Thỏ` & 5 Pha Làm Giá
*(Nguồn: `tro_choi_dong_tien_tcdt_1..3_audio.mp3.md`, `version_3_062023_trochoidongtien_audio.mp3.md` + `hoidap`, `KT01T1 Slide & Audio`)*

1. **Ba vai chơi & Ràng buộc Vốn sống còn của Cáo:**
   - **Cáo (Chủ doanh nghiệp / Nhà tạo lập / Đội cầm trịch):** Mọi cổ phiếu trên sàn muốn có sóng đều phải có người cầm trịch điều tiết thanh khoản. Một chiến dịch bài bản cấu kết đồng bộ giữa *Chủ DN + Ngân hàng + CTCK (cấp kho/margin và soi luồng lệnh để biết rõ nhỏ lẻ còn hàng hay đã bán sạch!) + Media/KOLs* (`KT01T1 Audio`).
   - **Điểm yếu định lượng được của Cáo — Ngưỡng Call Margin:** Cáo gom hàng phần lớn bằng **tiền vay thế chấp chính cổ phiếu tại nhiều CTCK** (`ttkcx_3_tongket_audio.mp3.md`). Do đó, ở vùng gom hàng, Cáo bị khóa bởi một **mức giá chặn dưới (ngưỡng Call Margin)** — nếu để thủng sâu, Cáo phải nộp thêm tiền mặt làm vỡ kế hoạch vốn. Đây là cơ sở kinh tế giải thích vì sao vùng nền gom có hỗ trợ cứng rất đáng tin cậy!
   - **Chó săn (Đệ của Cáo / Brokers lớn / KOLs / Chủ room / Báo cáo phân tích thứ cấp):** Được vào hàng sớm ở chân sóng (Điểm 1), làm nhiệm vụ phát tán câu chuyện Cơ bản (FA) kèm một **"Giá mục tiêu (Target Price)"** thật xa để neo tâm lý (*Anchoring*) cho Thỏ. *(Lưu ý: Đội lái luôn dùng FA bánh vẽ để viết bài PR, còn dùng PTKT để vẽ nến/mẫu hình điều khiển hành vi — `v2_06_2022_trochoidongtien_hoidap_audio.mp3.md`).*
   - **Hai định lý Lý thuyết Trò chơi (Cân bằng Nash) chi phối toàn bộ đồ thị PTKT:**
     - *Định lý 1 (Ma trận Giấu tin ở Pha Gom):* Chiến lược trội khi gom là tuyệt đối giấu tin $\Rightarrow$ **Ngay khoảnh khắc tin đồn đầu tiên rò rỉ ra diễn đàn/room = Quá trình gom hàng của Cáo ĐÃ HOÀN TẤT 100%** (`KT01T1 Slide` L395–418).
     - *Định lý 2 (Ma trận Bán sớm ở Pha Đẩy):* Vì sợ người khác xả trước gần Giá mục tiêu, chiến lược trội là bán sớm $\Rightarrow$ **(a) Cáo không bao giờ kéo 1 mạch lên đỉnh** (nếu kéo thẳng từ 50 lên 100, Thỏ chốt non hết dọc đường không còn ai đổ vỏ ở 100), nên bắt buộc phải chủ động đè tạo **2 lần điều chỉnh (3 nhịp tăng)** với độ lùi mỗi lần **$\le 50\%$** đà tăng trước đó để giữ cấu trúc đáy cao dần lôi kéo lớp Thỏ mới; và **(b) Cáo luôn xả hàng ở `1/2` đến `2/3` quãng đường hướng tới Giá mục tiêu** (hô 80–100 từ nền 30–50 thì vùng 50–60 hoặc 70–80 tại lần chỉnh thứ 2 đã xả!) (`v3_trochoidongtien_audio.mp3.md`, `v3_taichinhhanhvi_hoidap_audio.mp3.md`).

2. **Bảng Giải phẫu Định lượng 5 Pha của Cáo trên Đồ thị & Bảng điện:**

| Pha vận động | Hành động Giá & Thời gian | Dấu vết Khối lượng (VPA) & Dòng tiền Chủ động | Vi cấu trúc Bảng điện 3 Bước giá & Tin tức |
| :--- | :--- | :--- | :--- |
| **Pha 1: Gom hàng (`Accumulation` — Vùng 0)** | - Đi ngang **tối thiểu $\ge 1$ tháng, chuẩn `1 – 3 tháng`** (chu kỳ lớn `6 – 12 tháng`), gom kiểm soát **$\ge 50\%$ free-float**.<br>- **Biên độ hẹp** (không cho Thỏ lướt ăn vài %); không thủng ngưỡng vay Margin; nhú tăng sớm là bị đè cụt đầu. | - Đầu/giữa pha: **Vol thấp, đều**, cạn kiệt.<br>- Cuối pha gom: Giá đi ngang chưa tăng nhưng **Vol nhích tăng cao hơn bình thường**; Cáo mua chủ động hoặc bán đè ròng mà giá không giảm (`SBT, ABS, HNG`). | - **Dọa bán để gom:** Kê cục `Ask` khổng lồ ở bước giá 2–3, để bước 1 mỏng; bên `Bid` lèo tèo nhưng treo **lệnh chặn mua ẩn** sâu dưới 3 bước hiển thị.<br>- Thỏ đặt bán chen bước 1 $\rightarrow$ bị lệnh **Mua chủ động** đớp gọn!<br>- **Tin tức:** Im ắng tuyệt đối. |
| **Pha 2: Test cung / Rũ bỏ & Khởi động (`Spring` & Điểm 1)** | - Đạp thủng nền giả ép Thỏ lì lợm cắt lỗ rồi kéo rút chân bật ngược lên (**Nến `Spring`**).<br>- Bứt phá khỏi biên trên nền 1–3 tháng mở sóng (mới tăng **`5% – 20%`** từ nền gom là vùng vào RẤT TỐT cho Thỏ thông minh!). | - Tại phiên `Spring` hoặc phiên nổ Điểm 1: Khối lượng & lệnh **Mua chủ động của Cáo nổ vọt đột biến** phát tín hiệu triệu tập dòng tiền.<br>- Có thể quay tay mồi cầu ảo ở đầu sóng. | - **Tin xấu ngược ở đáy:** Cố tình tung tin xấu hoặc **Lãnh đạo / Cổ đông nội bộ đăng ký BÁN ngay vùng đáy** để dọa gom mẻ cuối!<br>- Tại Điểm 1: Chó săn tung tin đồn đầu tiên, cổ phiếu nhảy vọt thứ hạng Social. |
| **Pha 3: Đẩy giá & Nhịp nghỉ (`Markup` — Quy mô sóng $\ge 50\%–100\%$)** | - Huấn luyện **thói quen giao dịch trong phiên** (VD: sáng chỉnh chiều kéo).<br>- Có **2 hộp nghỉ/chỉnh** (`1->2` sau khi tăng 10–20%, và `3->4`), mức lùi chuẩn **$\le 1/3$** (tối đa **$\le 50\%$** đoạn tăng trước). | - Đường lên: `Giá tăng + Vol tăng đều` hoặc **Tiết cung** (`Giá tăng + Vol thấp`).<br>- Nhịp chỉnh 1: Vol tăng nhưng Cáo bán chủ động rất ít (chỉ Thỏ chốt non) $\rightarrow$ chỉnh nhanh vài phiên rồi kéo tiếp nhịp 2! | - Chiến dịch PR nở rộ (Báo chí, KOLs, Báo cáo mua CTCK nâng Target Price liên tục). |
| **Pha 4: Phân phối / Xả hàng (`Distribution` — Điểm 2)** | - Giá đã tăng **`30% – 50%+`** (đạt `1/2 – 2/3` Target) hoặc đang ở **nhịp tăng thứ 3**.<br>- *Cách 1 (2 bước trên đỉnh):* Nổ vol tạo đỉnh 1 $\rightarrow$ sau 1–2 phiên kéo hồi **Bulltrap đỉnh 2** xả nốt.<br>- *Cách 2 (Cây thông xả đường về):* Kéo cô đặc `10 -> 100`, thả rơi `40–50%` về `50–60` dụ bắt đáy "rẻ một nửa" để xả thật (lãi 5–6x)! | - **DẤU HIỆU KHÔNG THỂ CHE GIẤU:** **Khối lượng bùng nổ cực đại** tại vùng đỉnh (hoặc vùng bắt đáy 50–60 của cây thông) nhưng **Giá chững lại không tăng thêm / tạo nến `Upthrust`**.<br>- Cáo **không bao giờ quay tay vol ảo ở đỉnh**; Cáo chia nhỏ lệnh bán giả làm Chó/Thỏ trong khi Thỏ mua chủ động đạt cực đại. | - **Giả mua để xả:** Kê cục `Bid` khổng lồ lộ liễu ở bước dưới, `Ask` để mỏng tang; Thỏ đua mua lên bị táng thẳng, Thỏ kê đè cùng bước giá thì Cáo **hủy/rút lệnh mua mồi** rồi xả thẳng!<br>- **Tin tốt dồn dập nhất:** KQKD kỷ lục, **Lãnh đạo đăng ký MUA ở đỉnh** (tín hiệu giả lùa gà!). |
| **Pha 5: Đè giá & Reset vòng mới (`Markdown` — Điểm 3)** | - Thủng hỗ trợ sau Bulltrap; **sau khi gãy với Vol lớn không bao giờ lên lại trong 1 tuần** (mất `1 – 3 tháng`).<br>- Tạo nền mới ở **Điểm 3 > Điểm 1** để diễn kịch *"đồng hành dài hạn cùng doanh nghiệp"*. | - Đầu pha rơi có thể còn xả trên đường xuống (`Giá giảm + Vol cao` — cấm bắt!).<br>- Cuối pha rơi chuyển về bình thường: **`Giá giảm + Vol giảm cạn kiệt`** về ngang nền cũ trước sóng. | - Chó săn im bặt; Thỏ chửi bới khắp diễn đàn $\rightarrow$ Khi nào **Thỏ ngừng chửi, chán nản bỏ đi hết** là vòng mới bắt đầu! |

3. **Ma trận Hành động PTKT với Tin Tốt / Tin Xấu theo Vị thế Giá – Lượng (`v3_062023_thongtinkhongcanxung_audio.mp3.md`, `DM01_BUỔI_4`):**
   Cửa sổ sự kiện (*Event Window*) kéo dài **7–10 phiên** quanh ngày ra tin (tổng 2–4 tuần). Nội dung tin tốt/xấu trên báo **vô nghĩa nếu tách rời vị trí giá trước đó**:
   - **Tin Tốt ở nền tích lũy (1–3 tháng) hoặc mới chớm tăng $< 10–15\%$, Vol chưa nổ quá đà:** Tín hiệu khởi động sóng ("tin có đạo đức") hoặc thị trường phản ứng trễ $\rightarrow$ **MUA / GIỮ**.
   - **Tin Tốt ở nền nhưng giá trơ lì không tăng suốt nhiều phiên:** Nếu mua thuần vì kỳ vọng tin đó $\rightarrow$ lý do mua đã sai/hết hạn $\rightarrow$ **THOÁT RA**.
   - **Tin Tốt dồn dập (KQKD đột biến, chia cổ tức, tăng vốn, Lãnh đạo đăng ký MUA) sau khi giá đã tăng $30–50\%+$ kèm Vol nổ lớn:** Phản ứng thái quá ("Tin ra là bán"; với phát hành thêm: **tuyệt đối không lăn chốt** tránh chôn vốn & pha loãng ROE) $\rightarrow$ **CẤM MUA ĐUỔI; CHỐT LỜI NGAY** (chậm thì thoát ở nhịp Bulltrap thứ 2 sau 1–2 phiên).
   - **Tin Xấu / Lãnh đạo đăng ký BÁN sau chuỗi giảm sâu / tại nền đáy:** Nếu là nội bộ đăng ký bán ở đáy $\rightarrow$ đòn tâm lý gom vét ngược (Tích cực!). Nếu là cú sốc sự kiện bất ngờ $\rightarrow$ **cấm bán tháo sàn phiên hoảng loạn đầu tiên**; chờ 2–3 phiên hấp thụ `T+` có điểm uốn để cơ cấu ở nhịp nảy tâm lý (nếu nhảy vào bắt đáy sự kiện thì **`T+2` hàng về bắt buộc thoát sạch 100%** vì sau cú nảy đầu là Giai đoạn 3 ngấm tin định giá lại kéo dài nhiều tháng — `DM01_BUỔI_4` case `VIC/VHM/DGC`).
   - **Tin Xấu trọng yếu ở vùng giá cao mà giá chưa giảm ngay:** Cáo đang gồng đỡ tạm thời $\rightarrow$ **CƠ HỘI VÀNG BÁN SỚM GIÁ CAO**.

---

### 1.3. Mô hình `9 Quả Bóng` (`3 Nhóm Ngành × 3 Nhóm Dòng Tiền`) & Quy luật Luân chuyển
*(Nguồn: `HP5 v1_3, v2_3, v2_4, v3_3, v3_4`, `KT01T4 Slide & Audio`, `DM01T2`, `Chọn_cổ_phiếu_Fintech_QMV_17072025.mp3.md`)*

1. **Cấu trúc 2 tầng Luân chuyển độc lập ("Độ nhạy Vĩ mô" vs "Bình thông nhau Khẩu vị Rủi ro"):**
   - **Tầng ngang — 3 Nhóm Ngành Chu kỳ (dựa trên Đòn bẩy Tài chính `DFL` và Đòn bẩy Hoạt động `DOL`):**
     - **Nhóm ngành 1 (`NN1` — Nhạy sớm nhất ở Pha Tích lũy & Đầu Nới lỏng, `DFL` cao):** Trọng tâm tứ trụ **Ngân hàng** *(chạy nhúc nhích đầu tiên làm chỉ báo mở sóng)* $\rightarrow$ **Chứng khoán** $\rightarrow$ **Bất động sản** $\rightarrow$ **Xây dựng & VLXD** *(bản mở rộng 6 ngành gồm thêm Thực phẩm & Hóa chất)*.
     - **Nhóm ngành 2 (`NN2` — Chạy chủ lực ở Pha Mở rộng kinh tế, `DOL` cao):** **Bán lẻ**, Hàng tiêu dùng, Thực phẩm & Đồ uống, **Hóa chất**, **Tài nguyên cơ bản (Thép)**, **Sản phẩm & DV Công nghiệp / KCN**, **Dầu khí** *(giao thoa Công nghệ, Ô tô, Du lịch)*.
     - **Nhóm ngành 3 (`NN3` — Phòng thủ / Cuối chu kỳ Bão hòa):** **Dịch vụ hạ tầng (Điện, Nước, Khí - `GAS`)**, Viễn thông, Truyền thông, **Y tế**, **Bảo hiểm**.
     - **7 Bộ / Cụm ngành tương đồng luôn đi đôi tại VN (`KT01T4`):** (1) `Bank – Chứng – BĐS`; (2) `Bank ↔ Chứng`; (3) `BĐS → XD & VLXD`; (4) `Thực phẩm ↔ Bán lẻ ↔ Tiêu dùng ↔ Hàng không/Ô tô`; (5) `Công nghệ ↔ Truyền thông ↔ Viễn thông`; (6) `DVCN → Tài nguyên → Hóa chất ↔ Dầu khí`; (7) `Hạ tầng – Bảo hiểm – Y tế`.
   - **Tầng dọc — 3 Nhóm Dòng tiền Nội ngành (`DT1 -> DT2 -> DT3`) + `Trà đá / DT0 / MM`:**

| Tiêu chí định lượng | `DT1` (An toàn / Đầu ngành) | `DT2` (Trung bình / Tối ưu Swing) | `DT3` (Rủi ro cao / Đầu cơ) | Nhóm đặc thù: `Trà đá` (`< 10.000đ`), `DT0` & `MM` (Hàng lái) |
| :--- | :--- | :--- | :--- | :--- |
| **Vốn hóa & Đại diện** | **Big Cap / Bluechip** (mặc nhiên rổ `VN30`). | **Midcap** có nền tảng tốt (`CEO, DIG, BSI, FTS...`). | **Small Cap / Penny**. | Toàn sàn ~1.700 mã chỉ có ~390–400 mã giao dịch chuẩn; >1.000 mã là **Trà đá** (`<10k`) hoặc **`DT0`** (tắt thanh khoản). **`MM`** = mã gắn tên tuổi đội lái, đánh kiểu cây thông bất chấp ngành khi tiền dồi dào. |
| **KLGD trung bình** | Luôn duy trì **Lớn**, ra vào vốn lớn dễ dàng. | **Vừa / Khá**, thanh khoản tốt. | **Nhỏ / rất nhỏ** ở nền; chỉ nổ đột biến khi vào sóng. | Ở vùng đáy `0` vol cực nhỏ hoặc tự nhiên tăng vọt mà giá chưa chạy. |
| **Hệ số Beta ($\beta$)** (`KT01T4`) | **$1.0 \le \beta < 1.5$** (thấp/vừa). | **$1.5 \le \beta \le 2.0$ (đến $2.5$)**. | **$\beta > 2.0$ (hoặc $> 2.5$)**. | $\beta$ biến thiên cực gắt (từ nén chặt `< 0.5` vọt lên `> 2.5` khi mở sóng). |
| **Định giá `P/E, P/B` & Cổ tức** | `Trung bình – Thấp` so với ngành; chia cổ tức đều. | `Thấp & Rất thấp` so với ngành; có lãi tốt. | `Cao / Quá cao` hoặc **`P/E` Âm** (thua lỗ), không cổ tức. | Ưu tiên Trà đá Loại 1 (*Distressed thật*: `P/B << 1`, hoạt động thật bị lãi vay ăn mòn $\rightarrow$ hưởng *Double Whammy* khi hạ lãi suất!). |
| **Chất lượng DN (Dupont)** | **4 – 5 sao** (`ROE > WACC/LS vay`). | **3 sao**. | **1 – 2 sao**. | Tiêu chí duy nhất: **"KHÔNG ĐƯỢC CHẾT"** (không lỗ 2 năm liên tiếp). |
| **Đặc tính Vận động Giá & Chiến lược** | Nặng nề, biên hẹp nhưng Sharpe cao $\rightarrow$ **Dành cho Investing & làm chỉ báo đầu sóng; CẤM lướt T+ liên tục!** | **Thanh thoát, chạy mạnh và bền sóng nhất** $\rightarrow$ **Mặc định ưu tiên số 1 cho Swing Trading!** | Tăng sốc `30–40%/tuần` nhưng chỉnh gắt `-20..30%` $\rightarrow$ **Chỉ đánh ngắn `T+` / Rebalancing vòng, cấm kẹp thành cổ đông dài hạn!** | Gom lâu ở Vùng `0`, khi đánh lên trong mùa tiền nhiều thường đi đủ 5 bước **x2, x3, x4 lần** (từ 2–3k về mệnh giá 10k); hết tiền lại về Trà đá! |

2. **Quy luật Luân chuyển Đan xen (`Overlapping Rotation`) & 2 Tín hiệu Đỉnh Chu Kỳ:**
   - **Không nối đuôi tuần tự mà ĐAN XEN:** `1.1` khởi phát nền đáy $\rightarrow$ `1.2` $\rightarrow$ khi `1.2` và `1.3` đang chạy sôi động thì **`2.1` đã bắt đầu khởi động đan xen** $\rightarrow$ `2.2` $\rightarrow$ khi `2.2` và chớm `2.3` đang chạy thì **`3.1` khởi động đan xen** $\rightarrow$ `3.2` $\rightarrow$ nổ cuối cùng ở **`3.3`**. *(Trong nội bộ Giai đoạn 1 kéo dài ~1 năm, `1.1` và `1.3` thường "đổi gác" rũ bỏ khoảng **3 lần** trước khi bứt phá thật — `HP5 v3_4_hoidap`).*
   - **Điều kiện lan tỏa:** Nếu vĩ mô **tiền ít / thắt chặt**, dòng tiền đứt gãy chỉ quẩn quanh ở `DT1` (chớm `DT2`), hộp `DT3`/Trà đá bị bỏ đói hoàn toàn.
   - **Tín hiệu Cảnh báo Đỉnh #1 (Warning):** Khi **tất cả các nhóm rủi ro nhất (`DT3` / Trà đá ở khắp mọi nhóm ngành) đồng loạt chạy bốc đầu / tím trần** (vùng bóng `7–8–9` / Bước 5). Chưa đoán đỉnh bán non ngay khi đang chạy, nhưng bật cảnh giác tối đa!
   - **Tín hiệu Điều chỉnh Thực sự #2 — Hiện tượng "ĐỔI GÁC" (`Correction Trigger`):** Khi **`DT3` / Trà đá đột ngột chững lại, mất đà tăng** VÀ dòng tiền rút về **mua đỡ giá ở nhóm an toàn `DT1` (đặc biệt khi cả 3 quả bóng `1.1, 2.1, 3.1` đồng loạt giữ giá tốt hơn thị trường chung đang yếu)**.
     > **Thiết quân luật cho Nhà đầu tư Cá nhân (`HP5 v3_4_baigiang`):** Tổ chức/Quỹ mua `DT1` và ngành phòng thủ cuối chu kỳ chỉ vì điều lệ ép giữ cổ phiếu (giảm ít hơn Index đã là thắng). **Nhà đầu tư cá nhân thấy 3 quả bóng `DT1` xuất hiện cuối chu kỳ TUYỆT ĐỐI KHÔNG MUA PHÒNG THỦ mà phải RÚT SẠCH VỀ 100% TIỀN MẶT!**
   - **Bất đối xứng Sóng 5 $\rightarrow$ Sóng A $\rightarrow$ Sóng C (`HP4 v1_1`):** Khi mới gãy từ đỉnh Sóng 5 sang Sóng A, **`DT1` và `DT2` giảm rất nhanh trước**, trong khi **`DT3` / hàng lái vẫn được níu kéo giữ giá** làm đám đông chủ quan; nhưng sang **Sóng C**, toàn bộ các nhóm (kể cả `DT3`) sụp đổ thảm khốc cùng lúc!

---

### 1.4. Cổng Lọc Thượng Nguồn (`Macro Regime Gate` & `Corporate Quality / Anti-Trap Gate`)
*(Nguồn: `DM01T2`, `DM01_BUỔI_2_19122025_audio.mp3.md`, `VM01T2..T4`, `DN01T1..T4`, `v4_1_doanhnghieptrada_audio.mp3.md`, `tradaviahe_mscore_audio.mp3.md`, `tra_da_via_he_26022026_chu_ky_nganh_bds`)*

1. **Quy tắc Định lượng Phân bổ Tổng Vốn (`3/3 – 2/3 – 1/3 – 0/3`) theo Chu kỳ Chính sách Tiền tệ:**
   - **Pha 1 — Bắt đầu Nới lỏng** *(Hạ LS điều hành / hoặc Nới lỏng cấp độ 2 đầu năm bơm room tín dụng `> 15%` lên `17–20%`, CPI `< 4.0%`, OMO bơm ròng)*: **`3/3` Cổ phiếu (`100%`) – `0/3` Tiền mặt**; cho phép dùng **Margin gia tăng trên vị thế tiền thịt đang thắng**.
   - **Pha 2 — Đầu/Giữa Bão hòa Đỉnh** *(LS điều hành đi ngang ở đáy không giảm thêm, tín dụng `~14–15%`, CPI `~4.0%`)*: **`2/3` Cổ phiếu (`~67%`) – `1/3` Tiền mặt (`~33%`)**; **cắt bỏ hoàn toàn Margin**, xoay trọng tâm sang `NN2`.
   - **Pha 3 — Cuối Bão hòa $\rightarrow$ Chuyển tiếp** *(Hết room tín dụng cuối quý/cuối năm — "tiền nhiều không nhiều thêm", OMO hút ròng / LS liên ngân hàng vượt trần tái cấp vốn `4.5%`)*: **`1/3` Cổ phiếu (`~33%`) – `2/3` Tiền mặt (`~67%`)**; phòng thủ cao độ.
   - **Pha 4 — Chính thức Thắt chặt** *(LS chính sách TĂNG trở lại, CPI `> 4.5%`, bán dự trữ ngoại hối)*: **`0/3` Cổ phiếu (`0%`) – `3/3` Tiền mặt (`100%`)** (gửi tiết kiệm, vàng, ngoại tệ; đóng cổng 100% với Trà đá).
   - **5 Biến số Tiền tệ báo hiệu đảo chiều SỚM NHẤT (trước cả LS chính sách):** (1) NHNN nâng LS OMO / phát hành Tín phiếu hút tiền *(kiểm tra sau **7 ngày đáo hạn** xem có hút cuốn chiếu + LS liên ngân hàng có vọt tăng không; nếu hút tín phiếu mà LS liên ngân hàng vẫn thấp và CPI `< 4%` thì chỉ là rung lắc cơ hội gom hỗ trợ!)*; (2) **LS huy động Thị trường 1 và LS Liên ngân hàng luôn tăng TRƯỚC LS chính sách**; (3) Tỷ giá USD/VND ép NHNN **bán Ngoại tệ** rút tiền dự trữ NHTM; (4) Room tín dụng kịch trần theo quý (`25%/quý` năm 2026) hoặc mùa nộp thuế tháng 3; (5) Tiền gửi không kỳ hạn **`M1` sụt giảm** chảy sang kỳ hạn dài `M2`.

2. **Cổng Lọc Chất lượng Doanh nghiệp (`Hard Anti-Delisting Gate` & `Beneish M-Score`) để không sập bẫy PTKT trên cổ phiếu rác:**
   - **5 Điều kiện Cứng "KHÔNG ĐƯỢC CHẾT" (Vi phạm 1 điều kiện = Chặn đứng `REJECT` mọi tín hiệu PTKT):**
     1. Đã **lỗ ròng 2 năm liên tiếp** (đặc biệt đang ở Q3–Q4 năm thứ 3 trước ngưỡng hủy niêm yết bắt buộc 3 năm của HOSE).
     2. **Lỗ lũy kế $>$ Vốn điều lệ thực góp**.
     3. **Vốn chủ sở hữu $< 0$ (Âm)**.
     4. Bị nhắc nhở **chậm nộp BCTC / diện hạn chế hoặc đình chỉ giao dịch / ý kiến kiểm toán ngoại trừ trọng yếu**.
     5. **ROE âm liên tục 5 năm** (doanh nghiệp hủy diệt vốn cổ đông).
   - **Bộ lọc Gian lận Kế toán Beneish `M-Score` (tính trên BCTC Năm $t$ vs $t-1$ — `tradaviahe_mscore_audio.mp3.md`):**
     - **`M-Score < -2.22` (Vùng Xanh):** An toàn kế toán $\rightarrow$ Pass cho cả Investing & Trading.
     - **`-2.22 <= M-Score <= -1.78` (Vùng Xám):** Kiểm tra chéo `TATA < 0` (`OCF > Lợi nhuận`) và Piotroski F-Score.
     - **`M-Score > -1.78` (Vùng Đỏ — Nguy cơ xào nấu BCTC cực cao):** Soi ngay 2 biến chí mạng là **`DSRI > 1.20`** (phải thu phình nhanh hơn doanh thu $>20\%$ do xuất hóa đơn khống sân sau) và **`TATA > 0`** ($\frac{\text{LN} - \text{OCF}}{\text{Tổng TS}} > 0$, mang trọng số cực đại $+4.679$ — lãi giấy không có tiền thật!), cùng `AQI > 1` (vốn hóa chi phí treo như `HAG`), `DEPI > 1` (kéo dài khấu hao), hoặc thủ thuật mua nhích sở hữu lên đúng **`51.01%`** để hợp nhất doanh thu ảo (`HHS`). Nếu không giải thích được bằng M&A thật $\rightarrow$ **Loại thẳng trong 5 giây cho Investing; nếu đánh sóng đầu cơ phải khóa cứng ở chế độ Trading ngắn hạn `T+`!**
   - **Bộ lọc Phá "Bẫy Giá trị `P/B` Thấp" riêng cho Ngành Bất động sản (`26022026_bds`):** Gạch bỏ dự án treo $>5$ năm và lãi vay vốn hóa vào tồn kho; yêu cầu `EBIT / Lãi vay > 1.0`, `Vòng quay tồn kho` hợp lý (tránh loại mất 25–50 năm mới bán hết kho như `PDR 0.04`, `NVL 0.02`), `Nợ vay / EBITDA` thấp, và `Người mua trả tiền trước > 50% Tồn kho` kèm `CFO > 0`.

---

## CHƯƠNG 2: QUY TRÌNH 6 BƯỚC PTKT, 7 BƯỚC THỰC HÀNH & KHUNG RA QUYẾT ĐỊNH TOP-DOWN

*(Nguồn: `HP4 v2_6_1_1..1_5`, `v3_6_bg + hoidap + thaoluan`, `KT01T4`, `Thực_Hành_Ra_Quyết_Định_Với_QMV.md`, `DM01_BUỔI_4`)*

### 2.1. Khung Ra Quyết Định Top-Down 5 Tầng Chuẩn QMV
Mọi quyết định giao dịch đều phải đi từ rừng xuống cây, tuyệt đối không mở chart cổ phiếu riêng lẻ lên vẽ ngay:

```
[Tầng 1: XU THẾ LỚN - Vĩ mô & Tiền tệ] -> Định tỷ trọng Cổ phiếu/Tiền (3/3, 2/3, 1/3, 0/3) & Key Drivers
        │
        ▼
[Tầng 2: THỊ TRƯỜNG CHUNG - VN-Index] -> "Thị trường có xấu (Downtrend) không?" + Định giá VNI + Bối cảnh lớn (0..8)
        │  (Nếu VNI Downtrend rõ -> ĐỨNG NGOÀI; Nếu VNI không xấu -> Cho phép Trading theo phân hóa!)
        ▼
[Tầng 3: NGÀNH & NHÓM DÒNG TIỀN]     -> Chọn Ngành hút tiền (NN1/NN2/NN3) × Lớp rủi ro (DT1 -> DT2 -> DT3 / Trà đá)
        │
        ▼
[Tầng 4: THẨM ĐỊNH CỔ PHIẾU 3 CHÂN]   -> (1) Cơ bản/Sao/Định giá ("Không chết" / Tài sản đảm bảo)
        │                                 + (2) Khối ngoại & Cáo-Chó-Thỏ + (3) Blackbox (TMC, DSPI, MPICP, Hấp thụ T+)
        ▼
[Tầng 5: 6 BƯỚC PTKT & ĐI LỆNH 2 PHA] -> Khung 1D xác định VÙNG (Zone + RR > 1) -> Khung 30m/15m chốt THỜI ĐIỂM (Trigger)
```

- **Phân biệt sắc bén "Xu thế lớn" vs "Bối cảnh lớn" (`KT01T4`):**
  - **Xu thế lớn** = Chu kỳ chính sách tiền tệ (tiền nhiều hay tiền ít).
  - **Bối cảnh lớn** = Vị trí trên sơ đồ tâm lý `0 – 1 – 2 – 3 – 4 – 5 – 6 – 7 – 8` của thị trường hoặc của từng cổ phiếu. Khi *Xu thế lớn* vẫn nhiều tiền nhưng *Bối cảnh lớn* của VN-Index đã ở Bước `5` (trên định giá), thị trường sẽ điều chỉnh luân phiên theo ngành: mã ở Bước `5` phải bán, nhưng mã ở Bước `0->1` hoặc `1->2` chỉnh lại là cơ hội mua!

---

### 2.2. Quy trình 6 Bước PTKT Chuẩn (`HP4 v2_6_1_4, v3_6_bg`) & Nâng cấp 7 Bước Thực hành (`KT01T4`)
1. **Bước 1 — Xác định Xu hướng & Vị trí Bối cảnh lớn (`Trend & 0–8 Cycle`):**
   - Dùng bộ 4 đường `SMA(20)` (1 tháng), `SMA(50)` (1 quý), `SMA(100)` (6 tháng), `SMA(200)` (1 năm) trên khung `1D`, kết hợp cấu trúc Đỉnh – Đáy (`HH/HL` vs `LH/LL`), Đường xu hướng nối 2 đáy/đỉnh gần nhất, và đếm nhịp trên sơ đồ `0-1-2-3-4-5` (đã qua mấy lần điều chỉnh lưỡng lự?).
2. **Bước 2 — Xác định Ngưỡng Hỗ trợ, Kháng cự & Vùng Giao dịch Quan trọng (`Trading Zones`):**
   - Tìm vùng đỉnh/đáy ngang gần nhất có khối lượng tích tụ lớn để biết: *(a) Tăng thì lên tới đâu nghỉ/chốt (Reward)* và *(b) Sai thì rơi về đâu đỡ/cắt (Risk)*. Khi vượt đỉnh lịch sử hoặc thủng đáy cũ, dùng thước đo tịnh tiến nhịp trước + quy tắc chia phần **`1/3 – 1/2 – 2/3 – 100%`** (hoặc Fibonacci đã gọt bỏ râu hoảng loạn).
3. **Bước 3 — Kiểm tra Mẫu hình Kỹ thuật (`36 Chart Patterns` & `Quy tắc 3 lần`):**
   - Nhận diện hình thái hộp/kênh/tam giác/phễu lưỡng lự theo độ nghiêng (`< 45°`, độ lùi `<= 50%`), kiểm tra xem đây là lần tích lũy thứ 1, thứ 2 hay đã là lần thứ 3 cuối sóng, và đếm **Điểm chạm số 1 – 2 – 3**.
4. **Bước 4 — Kiểm tra các Chỉ số Kỹ thuật (`Indicators`):**
   - Đọc tối giản `Bollinger Bands` (`MA20` hoặc `MA15`) + `RSI(14)` + `CCI(14/10/20)` (hoặc `MACD`, `Ichimoku` tùy sở trường cố định) theo 4 yếu tố: *(1) Hướng đi & Trendline kẻ trực tiếp trên chỉ báo, (2) Vùng điểm Quá mua/Bình thường/Quá bán đặt thuận theo xu thế lớn, (3) Điểm uốn thực sự, (4) Phân kỳ Thường / Ẩn*.
5. **Bước 5 — Phân tích Giá và Lượng (`VPA` theo Chuỗi $\ge 3$ phiên & `MA5 Volume`):**
   - Tuyệt đối không bỏ qua! Đọc tương quan dịch chuyển Cung – Cầu tối thiểu 1 vòng `T+3` quanh đường `SMA(Volume, 5)` để kiểm chứng lực hấp thụ, tiết cung, cạn cung hay kéo xả phân phối ("đạp ga xe không chạy").
6. **Bước 6 — Phân tích Đồ thị Nến & Khoảng trống giá (`Candlestick Merging & Gaps`) tại Khu vực Hiện tại:**
   - Chốt câu hỏi **"KHI NÀO (Timing)"** bằng Nến đặc biệt (`Spring`, `Upthrust`, `Marubozu`, cụm ghép 2–3 nến) ngay tại vùng Hỗ trợ/Kháng cự.
7. **Bước 7 (Bổ sung hoàn thiện trong `Thực_Hành_Ra_Quyết_Định_Với_QMV.md` & `DM01_BUỔI_4`) — Lập Kế hoạch 3 Mốc (`Mua – Target – Stoploss` với `RR > 1`), Đi Vốn Chia 3 Phần & Quản trị sau Lệnh:**
   - Ấn định rõ Vùng mua (không mua đuổi quá `MaxBuy`), `Target 1/2`, `StopLoss`, giải ngân `1/3 – 1/3 – 1/3` và kiểm định theo cửa sổ `T+`.

---

### 2.3. Thứ tự Trọng số Ưu tiên Công cụ & Phối hợp Đa Khung Thời Gian Đặc thù Việt Nam
1. **Thứ tự quyền lực khi các công cụ mâu thuẫn nhau (`v2_6_1_2`, `hoc_phan_4_gioithieu`, `KT01T2`):**
   $$\text{Vĩ mô \& VN-Index} > \text{Hỗ trợ/Kháng cự NGANG} > \text{VPA (Giá – Lượng chuỗi T+3)} > \text{Nến tại Vùng đặc biệt} > \text{Phân kỳ Chỉ báo} > \text{Mẫu hình / Elliott / Fibo}$$
   - **Cấm lấy bình quân nhiều chỉ báo:** Nếu `RSI` và `CCI` báo lệch nhau, phải tra lại lịch sử quá khứ của chính cổ phiếu đó (thói quen đội tạo lập của mã đó) xem công cụ nào có tỷ lệ đúng cao hơn để chọn tin công cụ đó (`v1_6_ontapnguoimoi_audio.mp3.md`).

2. **Hệ thống Đa khung Thời gian khớp Vi cấu trúc TTCK Việt Nam:**
   - **Khung Tuần (`1W`) & Tháng (`1M`):** Không dùng để bấm lệnh ngắn hạn vì độ trễ lớn; chỉ dùng cho **3 nhiệm vụ chiến lược**: *(1) Xác định sàn giá trị tổ chức `Weekly EMA(200)` khi bắt đáy cổ phiếu cơ bản (`wa_10_02_2026`)*; *(2) Soi nguy cơ Phân kỳ âm khung Tuần/Tháng khi giá tiến sát đỉnh năm cũ (`VIP`, `FRT`)*; *(3) Đếm 3 nhịp tăng tuần để tránh mua đuổi đỉnh tuần (`PNJ`)*. Trên khung `1D`, dùng luôn `MA200 / MA100 / MA50 / MA20` để đại diện trực quan cho Năm / 6 Tháng / Quý / Tháng.
   - **Khung Ngày (`1D` — Trục trung tâm):** Khớp tự nhiên với tâm lý "ngủ 1 đêm bình tâm lại" và chu kỳ thanh toán `T+2.5`. Dùng để xác định **Vùng Giao Dịch (`Zone`)** và ra quyết định chiến lược sau khi **đóng nến kết phiên** (tránh bẫy tín hiệu nhảy múa theo râu nến giữa phiên — `Audio_B2_25062026`).
   - **Khung Nội ngày (`1H / 30m / 15m` — Trục kích hoạt `Trigger`):**
     - Chỉ mở **sau khi** khung `1D` đã đi vào Vùng Mua / Vùng Bán / Vùng Stop-loss (tuyệt đối cấm lấy phân kỳ `30m` để đoán đỉnh khi khung `1D` chưa chạm cản!).
     - **Khung `30m`:** Đếm **3 điểm chạm nhỏ** tại hỗ trợ/kháng cự, soi phân kỳ sớm và vùng lình xình nâng chặn lãi. *(Lưu ý: Cây nến `30m` cuối ngày có thân dẹt không bóng nhưng Volume vọt cao là do cơ chế khớp định kỳ **ATC**, KHÔNG phải nến phân phối bất thường — `HP4 v1_4` case `CTG`; và khi soi Volume nội ngày của thị trường chung, phải mở **`VN30` khung `M30`** thay vì `VN-Index` để tránh méo do vốn hóa nhóm Vin — `wa_22_01_2026`).*
     - **Khung `15m`:** Vì phiên **ATO (15 phút)** và **ATC (15 phút)** đều dài đúng 15 phút, khung `15m` giúp các cột Volume đồng nhất đơn vị thời gian, cực chuẩn khi lướt **Rebalancing T0** (`HP4 v3_6_thaoluan`).
     - **Khung giờ vàng bấm nút Mua mới (`wa_10_02_2026`):** Chỉ giải ngân vào **10–15 phút cuối phiên liên tục (`14:15 – 14:30`) hoặc đặt trong phiên `ATC`** để khóa chặt hình thái nến ngày, triệt tiêu bẫy kéo xanh giả buổi sáng rồi đạp đỏ buổi chiều!

---

## CHƯƠNG 3: NGHỆ THUẬT ĐỌC NẾN, KỸ THUẬT GHÉP NẾN & PHÂN TÍCH GIÁ – LƯỢNG (`VPA` KIỂU QMV)

### 3.1. Hệ Tiên đề Phân tích Giá – Lượng (`VPA`) Nguyên bản QMV
*(Nguồn: `KT01T1 Slide & Audio`, `HP4 v1_5, v2_1_1..4, v2_6_1_6, v3_1_bg + hoidap + thaoluan`, `hoc_phan_4_gioithieutongquanhp4_audio.mp3.md`)*

1. **Định luật Bất biến số 1 — "Ai là người quyết định Khối lượng khớp lệnh?":**
   - Bảng điện chỉ cho thấy điểm cân bằng đã khớp $(P_t, V_t)$. Nhưng bản chất vi mô đằng sau là:
     - **Trong Xu hướng GIÁ TĂNG (`Cầu > Cung`):** Giá tăng do Bên Mua đẩy, nhưng **KHỐI LƯỢNG KHỚP LỆNH thực tế do BÊN BÁN (CUNG) quyết định!** *(Chứng minh: Phiên tăng trần trắng bên bán, cầu chất hàng triệu cổ nhưng không ai bán thì `Vol = 0`; bên bán nhả ra bao nhiêu mới khớp bấy nhiêu).* **Hệ quả đập tan ngộ nhận đám đông:** Giá đang ở vùng cao mà nổ thanh khoản kỷ lục KHÔNG phải "tiền mới vào mạnh", mà là **bên cầm hàng đang xả ra cực mạnh**!
     - **Trong Xu hướng GIÁ GIẢM (`Cung > Cầu`):** Giá giảm do Bên Bán đè, nhưng **KHỐI LƯỢNG KHỚP LỆNH thực tế do BÊN MUA (CẦU) quyết định!** *(Chứng minh: Phiên sàn trắng bên mua, bên mua chịu hứng bao nhiêu mới khớp bấy nhiêu).* **Hệ quả:** Trong đà giảm, `Vol` giảm dần là bình thường do cầu đứng ngoài; hễ `Vol` ngừng giảm và tăng vọt lên tại hỗ trợ kèm giá chững lại $\Rightarrow$ **Bên Mua lớn đã nhập cuộc gom hàng!**

2. **Định luật số 2 — "Bình thường vs Bất thường" & "Điều kiện Cần (Lượng) vs Điều kiện Đủ (Giá)":**
   - **2 Trạng thái Bình thường duy nhất:** *(a)* Xu thế tăng bình thường = **`Giá tăng + Lượng tăng đều`** (bám trên `SMA(Vol, 5)` dốc lên — bên mua hấp thụ tốt lượng chốt lời rải rác); *(b)* Xu thế giảm bình thường = **`Giá giảm + Lượng giảm dần`** (cầu thờ ơ, áp lực cung cũng vơi dần).
   - **Quy tắc hành động (`KT01T1 Audio`):** Chỉ chuẩn bị vào/ra lệnh khi cổ phiếu đang vận động **Bình thường** rồi xuất hiện điểm chuyển pha; **tuyệt đối cấm bắt đáy khi đang Bất thường chiều giảm (`Giá giảm sâu + Lượng tăng cao` = đang xả hàng trên đường xuống, như `HAH, VCB, TCB, FPT`)** và **cấm đoán đỉnh bán non khi đang `Tiết cung` (`Giá tăng + Lượng thấp`) chưa nổ vol**!
   - **Lượng là Điều kiện CẦN (Cảnh báo sớm); Giá là Điều kiện ĐỦ (Phán quyết cuối cùng):**
     - Chiều lên: Xuất hiện phiên `Vol` đột biến ($\ge 1.25 \times \text{SMA}(V, 5)$, hoặc ở đỉnh lớn hơn hẳn cây `Vol` bắt đáy nhịp trước) là đèn vàng cảnh báo cung tuôn ra. **NHƯNG** nếu đóng cửa giá vẫn tăng khỏe (như `GMD` trong `KT01T1`) thì cầu vẫn thắng $\rightarrow$ giữ tiếp! Chỉ khi trong **`T+1 .. T+3` sau cây nổ Vol mà GIÁ DỪNG TĂNG / LÌNH XÌNH ("đạp ga xe không chạy" 3–6 phiên với Vol nằm trên MA5) hoặc QUAY ĐẦU GIẢM** thì mới xác nhận đỉnh phân phối!
     - Chiều xuống: `Vol` giảm cạn kiệt về ngang nền thấp trước sóng rồi ngừng giảm là điều kiện cần; **GIÁ DỪNG GIẢM và bật tăng kèm `Vol` nhích vượt `SMA(V, 5)`** mới là điều kiện đủ. *(Nếu gặp phiên giảm mạnh nổ `Vol` bắt đáy hoảng loạn, chưa mua ngay phiên đó mà bắt buộc chờ kiểm định ngày **`T+3 / T+2.5`** xem hàng về giá có thủng tiếp không!).*

3. **Giải phẫu Chuỗi `A – B – C – D`, Hiện tượng `Tiết cung` & 3 Thủ thuật Phân phối Trá hình tại VN:**
   - **Chuỗi vận động làm giá:** `Giai đoạn A` (Cầu tăng đều: `Giá ↑ + Vol ↑ đều`) $\rightarrow$ `Giai đoạn B` (**Tiết cung** `2A2`: đường Cầu dịch phải + đường Cung dịch trái do tạo lập khóa cung hoặc tin nội bộ thâu tóm như `HAH, STB, VPB, HCM`, làm **`Giá phi mạnh + Vol giảm/đi ngang thấp`**) $\rightarrow$ `Giai đoạn C` (Phân phối: sau khi đã tăng **$\ge 40\%–50\%$**, nổ `Vol` khổng lồ + giá chững/giảm).
   - **Phân biệt `Giai đoạn D` (Rũ hàng) vs `Giai đoạn C` (Phân phối):** Nếu cổ phiếu vừa thoát nền gom 1–3 tháng, **mới chỉ tăng `5% – 10%`** mà nổ `Vol` đỏ giảm điểm $\Rightarrow$ Khả năng cao là **Giai đoạn D (Rũ hàng cuối cùng)** để ép Thỏ yếu bóng vía nhả hàng. Nếu đã tăng `>= 40%–50%` mới nổ `Vol` chững giá $\Rightarrow$ Chắc chắn là **Phân phối `C`**!
   - **Quy tắc thoát hiểm số 1 sau `Tiết cung` (`hoc_phan_4_gioithieutongquanhp4_audio.mp3.md`):** Cổ phiếu đi lên bằng *Tiết cung* không kiểm chứng được độ dày cầu thực, nên khi bất ngờ xuất hiện **1 phiên nổ `Volume` khổng lồ**, xác suất đội lái xả hàng sập thành **"Cây thông Noel" đạt gần như `100%` $\rightarrow$ Bắt buộc thoát sạch ngay lập tức**, không chờ `T+3` như cổ phiếu tăng kèm vol đều!
   - **3 Thủ thuật Phân phối Trá hình của Đội lái VN (`HP4 v1_5`):**
     1. *Xả vào các phiên nến Đỏ:* Giả vờ điều chỉnh đỏ nhẹ ở vùng đỉnh để dụ nhỏ lẻ tưởng rẻ nhảy vào hứng, nhận diện qua `Vol` vùng đỉnh duy trì cao liên tục trên `MA5`.
     2. *Khớp 10–100 cổ phiếu giá Sàn ngay ATO ở vùng đỉnh:* Ghim `Open = Giá Sàn` để cả ngày giao dịch quanh tham chiếu vẫn vẽ ra **cây nến Xanh thân cực dài giả hiệu Breakout** dụ nhỏ lẻ đua đỉnh!
     3. *Khớp 10–100 cổ phiếu giá Trần ngay ATO trên đường rơi:* Ghim `Open = Giá Trần` rồi xả suốt phiên tạo nến đỏ dài `Vol` lớn phân phối nốt trên đường về.

---

### 3.2. Giải phẫu Định lượng Nến Đơn & Kỹ thuật Ghép Nến Độc Quyền QMV
*(Nguồn: `HP4 v1_4, v2_2_1..4, v3_2_bg + hoidap + thaoluan`, `KT01T3`, `wa_10_02_2026`)*

1. **Nguyên tắc Tối thượng: "Chỉ đọc 2 Loại Nến Đặc Biệt tại 5 Vùng Đặc Biệt" (Hiệu lực dự báo `1 – 3` nến):**
   - **5 Vùng Đặc Biệt hợp lệ:** (1) Hỗ trợ / Kháng cự ngang; (2) Chạm Đường xu hướng (`Trendline`); (3) Vùng nổ `Volume` đột biến; (4) Vùng chỉ báo Quá mua / Quá bán / Phân kỳ; (5) Điểm chạm Mẫu hình kỹ thuật (hoặc sau chuỗi tăng/giảm rõ rệt tối thiểu `3–4 phiên` đủ 1 vòng `T+`). Nến đứng lơ lửng giữa đường $\rightarrow$ **bỏ qua 100%!**
   - **Định lượng Tỷ lệ Thân nến / Tổng biên độ** ($R_{\text{body}} = \frac{|C - O|}{H - L}$):
     - **Ngoại lệ Vi cấu trúc VN (`H == L`, tức $O = H = L = C$):** Nến ngang không bóng do **Tăng trần cứng hoặc Giảm sàn cứng mất thanh khoản** ($\pm 7\%$ HOSE, $\pm 10\%$ HNX, $\pm 15\%$ UPCOM) **TUYỆT ĐỐI KHÔNG PHẢI DOJI LƯỠNG LỰ**, mà là **Nến Đặc Tiếp Diễn Cực Mạnh** (biên độ thực là không xác định do bị chặn trần/sàn)!
     - **Loại Đặc Biệt 1 — Nến Lưỡng lự / Đảo chiều Tâm lý ($R_{\text{body}} \le \frac{1}{3} \approx 33.3\%$ trên nền $H - L$ dài):** Gồm `Doji` ($R_{\text{body}} \approx 0$, tin cậy đảo chiều cao nhất) và `Nhóm Búa / Pin bar` ($R_{\text{body}} \le 1/3$). Biến động thuần do tâm lý, không có cơ bản mới $\rightarrow$ dễ đảo chiều.
     - **Loại Bình thường ($\frac{1}{3} < R_{\text{body}} < \frac{1}{2}$):** Không phân tích riêng lẻ. *(Lưu ý: Trong uptrend khỏe, spread nến xanh rộng hơn nến đỏ; trong downtrend, spread nến đỏ rộng hơn nến xanh).*
     - **Loại Đặc Biệt 2 — Nến Chắc chắn / Cương quyết ($R_{\text{body}} \ge \frac{1}{2} = 50\%$, tiến tới $\ge 85\%$ ở `Marubozu`):** Màu sắc quyết định trực tiếp phe thắng; nếu xuất hiện khi phá cản ngang hoặc ghép từ 3 nến ngày liên tiếp thì phản ánh **thay đổi Căn bản (cơ bản/thông tin mới)**, biến Kháng cự thành Hỗ trợ cứng (nhưng nếu phiên sau không tiếp diễn mà tạo 2 nến lưỡng lự ngay cản thì cả cụm là kéo xả!).
   - **Quy tắc Vị trí `1/3 Trên – 1/3 Giữa – 1/3 Dưới` (Khi $R_{\text{body}} \le 1/3$, VỊ TRÍ quan trọng hơn MÀU SẮC!):**
     - Chia đoạn $[L, H]$ thành 3 phần bằng nhau:
       - **Thân nằm gọn ở `1/3 Trên`** ($\min(O, C) \ge L + \frac{2}{3}(H - L)$, bóng dưới $\ge 67\%$): **NẾN TĂNG (Phe Mua thắng)** — dù thân màu đỏ vẫn là Nến Tăng đảo chiều chuẩn xác tại hỗ trợ (case `HCM, HPG`)!
       - **Thân nằm ở `1/3 Giữa`**: **NẾN LƯỠNG LỰ cân bằng** — cần phiên kế tiếp xác nhận.
       - **Thân nằm gọn ở `1/3 Dưới`** ($\max(O, C) \le L + \frac{1}{3}(H - L)$, bóng trên $\ge 67\%$): **NẾN GIẢM (Phe Bán thắng)** — dù thân màu xanh vẫn là cảnh báo đảo chiều giảm tại kháng cự!

2. **Công thức Toán học Ghép Nến Độc Quyền QMV (Xóa bỏ việc học vẹt 1.000 mẫu nến):**
   Khi tại Vùng đặc biệt xuất hiện cụm $K$ nến đan xen chưa rõ nghĩa ($K = 2$ ưu tiên cao nhất; nếu ghép 2 nến vẫn ra Nến Lưỡng lự kiểu `Harami` thì ghép $K = 3$ ứng với chu kỳ `T+2.5`; ngoại lệ ghép $K = 4..N$ khi có 2 Doji giống hệt nhau đứng giữa hoặc gom gọn cả cụm phân phối/tích lũy ngang):
   $$O_{\text{ghép}} = O_{t-K+1}, \qquad C_{\text{ghép}} = C_t, \qquad H_{\text{ghép}} = \max_{i \in [t-K+1,\, t]} H_i, \qquad L_{\text{ghép}} = \min_{i \in [t-K+1,\, t]} L_i$$
   - **Quy tắc Trục `50%` & Thuật ngữ `"Kill Nến Đỏ / Kill Nến Xanh"` (`KT01T3`, `HP4 v1_4`, `wa_10_02_2026`):**
     - Trong cụm 2–3 nến đảo chiều (`Morning Star`, `Piercing Line` vs `Evening Star`, `Dark Cloud Cover`), cây nến quyết định đóng cửa **vượt quá `50%` thân cây nến mạnh đầu tiên** ($C_t > \frac{O_1 + C_1}{2}$ ở đáy, hoặc $C_t < \frac{O_1 + C_1}{2}$ ở đỉnh) là ranh giới lật thế cờ (nếu đẩy thẳng lên vùng `1/3` đầu mút thì thuyết phục tuyệt đối!).
     - **`"Kill Nến Đỏ"`:** Khi một cây nến đỏ dài (hoặc nến đỏ phá vỡ hỗ trợ) bị cây nến xanh phía sau bật lên **đóng cửa vượt hẳn lên trên đỉnh `High` của cây nến đỏ đó** ($C_t > H_{\text{red}}$) $\Rightarrow$ Cây nến đỏ chính thức bị vô hiệu hóa hoàn toàn (phe bán tháo đã cạn sạch)! Ngược lại, **`"Kill Nến Xanh"`** ($C_t < L_{\text{green}}$ của cây nến nổ vol/breakout) xác nhận cú nổ thất bại, bắt buộc thoát vị thế ngay!
   - **Bảng biến đổi nhanh các tổ hợp sách vở qua phép Ghép Nến QMV:**
     - `Bullish Engulfing` (2 nến) / `Morning Star` (3 nến) / `Piercing Line sâu` (2 nến) $\xrightarrow{\text{Ghép}}$ **1 Nến Búa Tăng (`Hammer / Spring`)** thân nhỏ ở `1/3 Trên`.
     - `Hai búa ngược màu sát nhau ở đáy (Tweezer Bottom)` $\xrightarrow{\text{Ghép}}$ **1 `Dragonfly Doji`** (cực mạnh!).
     - `Bearish Engulfing` (2 nến) / `Evening Star` (3 nến) / `Dark Cloud Cover` (2 nến) $\xrightarrow{\text{Ghép}}$ **1 Nến Búa Ngược (`Shooting Star / Upthrust`)** thân nhỏ ở `1/3 Dưới`.
     - `Hai búa ngược ở đỉnh (Tweezer Top)` $\xrightarrow{\text{Ghép}}$ **1 `Gravestone Doji`** (báo động đỏ!).
     - `Three White Soldiers` / `Three Black Crows` (3 nến cùng màu dài dần) $\xrightarrow{\text{Ghép}}$ **1 `Marubozu` khổng lồ 3 ngày** = Thay đổi cơ bản thực chất (như `VCB, TCB` dính *Ba con quạ đen* kèm vol tăng dần $\rightarrow$ cấm tuyệt đối bắt đáy!).

3. **Hệ thống Giao dịch Chủ lực `Spring` ("Đạp kéo") & `Upthrust` ("Kéo xả"):**
   - **Nến `Spring` ("Nến mùa xuân / Đè để mua"):** Xuất hiện **tại Hỗ trợ**; trong phiên đạp xuyên thủng hỗ trợ ($L_t < S_{\text{zone}}$) rũ bỏ Thỏ hoảng loạn rồi kéo ngược đóng cửa **trên hỗ trợ** ($C_t \ge S_{\text{zone}}$) với thân ở `1/3 Trên` (hoặc nến xanh đặc dài mở cửa dưới hỗ trợ kéo lên cao, hoặc ghép từ 2 nến) + `Volume` ngừng giảm và nhích tăng/đột biến. *Xác nhận:* Phiên $t+1$ đóng nến xanh tăng; *Hủy (Fail):* Phiên $t+1$ đóng nến đỏ thủng dưới $L_t$.
   - **Nến `Upthrust` ("Kéo xả"):** Xuất hiện **tại Kháng cự** sau nhịp tăng; trong phiên kéo vọt qua kháng cự ($H_t > R_{\text{zone}}$) dụ FOMO rồi xả ép đóng cửa **tụt lại dưới kháng cự** ($C_t \le R_{\text{zone}}$) với thân ở `1/3 Dưới` + `Volume` lớn/đột biến. *Xác nhận:* Phiên $t+1$ là nến đỏ giảm; *Phủ định Upthrust (tránh bẫy bán non — case `TNH` `HP4 v3_2_thaoluan`):* Nếu phiên kế tiếp là nến Xanh mạnh vượt đỉnh làm phép **ghép 3 nến biến thành 1 Nến Tăng**, tín hiệu Upthrust bị xóa bỏ và xu hướng tăng tiếp diễn!

---

## CHƯƠNG 4: CẤU TRÚC THỊ TRƯỜNG, `3 ĐIỂM CHẠM`, 36 MẪU HÌNH GIÁ, GAPS, BREAKOUT, DOW & SÓNG ELLIOTT

*(Nguồn: `KT01T2`, `KT01T3`, `HP4 v1_1, v1_2, v2_4_1..4, v2_5_1..4, v3_4_bg + hoidap + thaoluan, v3_5_bg + thaoluan`)*

### 4.1. Vùng Giao Dịch (`Trading Zone`), Đường Xu Hướng & Sơ đồ Bối cảnh lớn `0 – 8`
1. **Hỗ trợ / Kháng cự Ngang là CHÍNH — Cách vẽ không bị nhiễu:**
   - Hỗ trợ/Kháng cự không bao giờ là 1 mức giá lẻ tẻ mà là **một Vùng / Dải giá (`Zone`)** quanh **đỉnh/đáy gần nhất trước đó** (không lấy quá xa khi cơ bản đã đổi), ưu tiên nơi có **khối lượng lịch sử tích tụ lớn**, các cây nến rút chân phản ứng mạnh, hoặc nơi **Đỉnh Tháng trùng khít Đỉnh Quý liền trước** (`VCI, TCB, KDH, VND` trong `HP4 v3_5_thaoluan`). *(Mẹo vẽ chính xác khi râu nến `1D` nhiễu: hạ xuống khung `1H` và kẻ qua giá đóng cửa các nến `1H` — `v3_4_thaoluan`).*
   - **Bí quyết gọt bỏ râu hoảng loạn khi vẽ Trendline & Fibonacci (`KT01T2 & KT01T3 Audio`):** Sau các cú sốc thông tin (như thuế quan tháng 4/2025 hay thiên nga đen), **bỏ qua các phiên rơi sàn tiết cung / đầu mút râu nến hoảng loạn cực đoan**, chỉ đặt mốc neo từ cây nến mà **thanh khoản và tâm lý trở lại trạng thái bình thường**!
   - **Kỷ luật thép:** Đã kẻ đường Hỗ trợ / Kháng cự / Trendline mà bị xuyên thủng thì **tuyệt đối cấm nắn, dịch chuyển hay vẽ lại** để tự lừa mình gồng lỗ (`HP4 v1_5`).
   - **Quy trình 2 bước xác nhận Đảo chiều Xu hướng theo Dow–QMV (`HP4 v2_5_4, v3_5_thaoluan`):**
     - *Bước 1 (Gãy Trendline tăng):* Chỉ mới báo hiệu **gia tốc tăng nóng đã suy yếu / chuyển sang Sideway** (tin ngắn hạn hết lực), chưa phải Downtrend! Phản xạ: kẻ ngay Trendline giảm phụ nối 2 đỉnh và hạ tỷ trọng phòng thủ.
     - *Bước 2 (Phá vỡ Đáy liền trước tạo `Lower Low` / Khóa kép Neckline ngang + Trendline chéo):* Chính thức xác nhận đảo chiều sang Xu hướng Giảm!

2. **Sơ đồ Bối cảnh Lớn `0 – 1 – 2 – 3 – 4 – 5 – 6 – 7 – 8` & Quy tắc Đàn hồi `1/3 – 1/2 – 2/3`:**
   - QMV chắt lọc tinh thần của Dow, Wyckoff và Elliott thành mô hình 3 trạng thái tâm lý (*Sợ hãi – Bình thường – Hưng phấn*) gồm **3 nhịp đẩy tăng (`0->1`, `2->3`, `4->5`) xen kẽ 2 hộp lưỡng lự nghỉ (`1->2`, `3->4`)** và **3 nhịp sườn phải (`5->6`, `6->7`, `7->8`)**:
     - **Các Vùng Mua (`0, 2, 4` — rủi ro tăng dần):** `Hộp 0` = nền tích lũy gốc (an toàn giá vốn nhưng chờ lâu); **`Hộp 2` (chỉnh lần 1 — Sóng 2)** = *"Nhịp chỉnh đầu tiên không đáng lo"*, vùng mua chuẩn mực có xác suất thắng và biên lợi nhuận Sóng 3 (`1.5x – 2x` Sóng 1) tốt nhất; **`Hộp 4` (chỉnh lần 2 — Sóng 4, thường nông hơn Sóng 2)** = vẫn mua được nhưng đã ở vùng cao, bắt buộc kiểm tra chặt `Reward/Risk > 1`; `Mốc 6` = chỉ dành cho lướt nảy kỹ thuật `T+3`.
     - **Các Vùng Bán (`1, 3, 5, 7`):** `Vùng 1` = bán dè dặt (hoặc giữ nguyên nếu mới thoát nền gom dài); `Vùng 3` = bán cơ cấu/rebalancing bình thường; **`Vùng 5` = bán mạnh / quyết liệt thoát hàng**; **`Vùng 7` (đỉnh Sóng B Bulltrap sườn phải) = vùng "bán nốt" toàn bộ hàng kẹp**!
   - **Thước đo Độ sâu Điều chỉnh thay thế Fibonacci lẻ (`KT01T3, KT01T4, DM01_BUỔI_4`):**
     - Chỉnh **$\le \frac{1}{3}$ (`~33% – 38.2%`)** đoạn tăng trước (hoặc đi ngang đủ thời gian không cần giảm giá): **Xu hướng tăng cực khỏe**, xác suất vượt đỉnh đi tiếp rất cao!
     - Chỉnh **$>\frac{1}{3}$ đến $\frac{1}{2}$ (`50%`)**: Xu hướng tăng yếu đi; chạm ngưỡng cân bằng **`50%`** thì **rất khó bật vượt đỉnh cũ ngay** mà phải tích lũy lại lâu (`case VSC`).
     - Chỉnh **$> 50\%$**: Cán cân tâm lý đã nghiêng sang Sợ hãi $\rightarrow$ **hủy mọi kịch bản mẫu hình tích lũy tăng!**
     - Chỉnh **$>\frac{2}{3}$ (`~66.7%`)**: Xu hướng tăng trước đó **chính thức bị xóa bỏ hoàn toàn**.

---

### 4.2. Quy tắc `3 Điểm Chạm` (`Rule of 3`) cho Cả Mua Hỗ Trợ lẫn Mua Breakout
*(Nguồn: `KT01T2`, `KT01T3`, `DM01_BUỔI_4`, `HP4 v1_2, v3_4_bg`)*

- **Nguyên lý gốc:** Tâm lý đám đông cần thử thách 2 lần để hấp thụ cung và thẩm thấu thông tin, nên **Lần chạm / phá thứ 1 thường yếu và dễ hỏng (`Fail`)**, đặc biệt **lần Breakout thứ 1 có xác suất hỏng (`False Breakout / Bulltrap`) còn CAO HƠN cả lần chạm Hỗ trợ thứ 1**!
- **Cách đếm & Chọn cặp điểm chạm thực thi (Zoom vào khung `30m` hoặc `1D`):**
  - **Điểm chạm 1:** Bỏ qua hoặc chỉ lót vị thế thăm dò nhỏ nếu sợ lỡ tàu trong uptrend mạnh.
  - **Điểm chạm 2 (`"Test cung"` — Trung tâm giải ngân/chốt lời chính):** Vì ngoài thực tế không phải lúc nào cũng có điểm chạm 3, **Điểm chạm số 2 luôn là trọng tâm hành động chính**.
  - **Điểm chạm 3:** Điểm xác nhận chín muồi nhất để bứt phá khỏi hộp/mẫu hình hoặc hoàn tất tạo đáy rũ bỏ (`Spring` / Đáy 3).
  - **Quy tắc thích nghi theo Xu thế lớn (`DM01_BUỔI_4`):**
    - *Hộp nằm trong Xu thế Tăng lớn (như Hộp 2):* Nhiều người chực mua nên giá hay chạy luôn từ chạm 2 $\Rightarrow$ Đánh cặp **Điểm chạm `1 – 2`**.
    - *Hộp nằm trong Xu thế Giảm lớn / Tạo đáy sau rơi sâu:* Gần như không bao giờ lên chữ V từ chạm 1 $\Rightarrow$ Bỏ qua hoàn toàn lần 1, chỉ đánh cặp **Điểm chạm `2 – 3`**!

---

### 4.3. Hệ Quy chiếu Tối giản `36 Mẫu Hình Giá` về 3 Quy luật Độ nghiêng
*(Nguồn: `HP4 v1_2, v2_4_1..4, v3_4_bg + hoidap + thaoluan`)*

Điều kiện cần để một vùng lưỡng lự là Mẫu hình hợp lệ: **(1) Mỗi biên trên/dưới có tối thiểu `2 điểm chạm`** (dùng công cụ *Tia / Ray* nối điểm 1 $\rightarrow$ 2 phóng sang phải để đón Điểm chạm 3); **(2) Góc nghiêng thoải `< 45°`**; **(3) Độ lùi mẫu hình `<= 50%`** đoạn xu hướng liền trước; **(4) Thời gian hình thành tối ưu ngắn hạn `2 – 3 tuần` (`10–20 phiên`)**, trung hạn `1 – 3 tháng` (`20–60 phiên`). Toàn bộ 36 mẫu hình quy về **3 Quy luật bất biến**:

| Quy luật Độ nghiêng | Các Mẫu hình đại diện (trong bộ 36 mẫu) | Cấu trúc Khối lượng chuẩn | Kết cục Xác suất cao & Lưu ý Thực chiến QMV |
| :--- | :--- | :--- | :--- |
| **1. Nghiêng LÊN $\Rightarrow$ GIẢM GIÁ** *(Bẫy Bull-trap thị giác số 1 lừa nhỏ lẻ!)* | Kênh song song lên / Cờ giảm (`Mẫu 7–8`), Phễu đóng lên / Nêm tăng (`Mẫu 15–16`), Phễu mở lên (`Mẫu 27–28`). | **Khối lượng TĂNG DẦN** khi bò lên + nổ Vol lớn / Upthrust / Phân kỳ âm ở chạm 3. | **Gãy cạnh dưới GIẢM MẠNH** bất kể trước đó tăng hay giảm (nếu trước đó đang giảm sẵn thì rơi tiếp càng khốc liệt — `CTG, BSR, C4G`). *Mẹo Kevin (`v3_4_hoidap`):* Bán chủ động ngay tại hỗ trợ chạm 3 khi chưa lỗ nếu đỉnh liền trước có phân kỳ âm + vol xả lớn! |
| **2. Nghiêng XUỐNG $\Rightarrow$ TĂNG GIÁ** *(Á quân độ tin cậy tiếp diễn tại VN)* | Kênh song song xuống / Cờ tăng (`Mẫu 5–6`), Phễu đóng xuống / Nêm giảm (`Mẫu 13–14`), Phễu mở xuống (`Mẫu 25–26`), **Tay cầm của Cốc tay cầm** (`C4G`). | **Khối lượng THẤP & GIẢM DẦN** (cạn cung) trong phễu $\rightarrow$ **TĂNG VỌT** khi phá cạnh trên ở chạm 3. | **Bứt phá TĂNG GIÁ** bất kể trước đó tăng hay giảm (miễn lùi `<= 50%`, lý tưởng `~1/3` đoạn tăng trước). *(Với Cốc tay cầm: miệng phải bắt buộc chạm lại đỉnh trái trước khi rũ làm quai hướng xuống; điểm vượt miệng cốc chính là Điểm chạm số 3!).* |
| **3. Nằm NGANG $\Rightarrow$ Tiếp diễn ở Nhịp `1–2`, Đảo chiều ở Nhịp `3`** | **Quán quân độ tin cậy:** Tam giác hướng lên (`17–20`) & Tam giác cân/Phễu đóng ngang (`9–12`); Hộp chữ nhật (`1–4`); Tam giác giảm (`21–24`); Phễu mở ngang (`29–32`, *ít tin cậy nhất do $\sigma$ mở rộng*); **2 Đỉnh/Đáy (`33–34`) & Vai–Đầu–Vai (`35–36`)**. | - *Tiếp diễn (nhịp 1–2):* Vol đều/giảm trong hộp, nổ mạnh + Breakaway Gap khi vượt biên.<br>- *Đỉnh 2Đ / VĐV xuôi:* Vol toàn vùng đỉnh **duy trì RẤT CAO** (đỉnh 2 nổ vol xanh mà không vượt đỉnh 1 là Cáo xả!).<br>- *Đáy 2Đ / VĐV ngược:* Vol vùng đáy **cạn kiệt** (biên giảm co về `< 1%/phiên` như `TCB`), nổ vol khi vượt Neckline. | - Mới tăng 10–20% (nhịp 1) $\rightarrow$ **Tiếp diễn xu hướng cũ**.<br>- Cuối sóng dài (nhịp dao động thứ 3) $\rightarrow$ **Đảo chiều**: Phần *Đầu* của VĐV xuôi chính là cú **`Upthrust`** thất bại; phần *Đầu* của VĐV ngược chính là cú **`Spring`** rũ đáy; xác nhận gãy 2 Đỉnh bằng **Khóa kép (Neckline ngang + Trendline chéo)**.<br>- **Đo Target mẫu hình:** Copy chiều dài đoạn tăng/giảm ngay trước mẫu hình đặt vào **đáy/đỉnh Điểm chạm số 3** chiếu lên/xuống! |

---

### 4.4. Bốn Loại Khoảng Trống Giá (`Gaps`) & 7 Điều kiện Sắt đá của `Breakout Thật`
*(Nguồn: `HP4 v1_1, v2_5_1..3, v3_5_bg + thaoluan`, `KT01T2`)*

1. **4 Loại Gap dưới cơ chế ATO ngắt quãng của Việt Nam:**
   - **`Common Gap` (Trong hộp đi ngang, Vol bình thường, không tin mới):** Xác suất bị lấp (`Fill Gap`) rất cao $\rightarrow$ **Cấm mua đuổi**, chờ lấp gap.
   - **`Breakaway Gap` (Phá vỡ mẫu hình Tam giác / Phễu giảm / Hộp ngang tại điểm chạm 3 + Vol nổ đột biến):** Tâm lý định giá lại trên nền cơ bản tốt $\rightarrow$ **Không bị lấp**, trở thành **Hỗ trợ cứng**, cho phép mua.
   - **`Runaway / Continuation Gap` (Dọc thân sóng mạnh / mới thoát đáy có tin tốt + Vol tăng mạnh nhưng nhỏ hơn Exhaustion Gap):** Giữ vững sau `T+3`, làm hỗ trợ cứng tiếp diễn xu hướng.
   - **`Exhaustion Gap` (Cuối sóng tăng nóng $\ge 50\%–100\%$ hoặc sau 2 lần nghỉ + Vol ĐỘT BIẾN CỰC ĐẠI + không tin mới):** Bị lấp xuyên thủng ngay trong **`T+1 .. T+3`** (ai đua phiên gap hàng về chắc chắn lỗ!), cô lập vùng đỉnh thành kháng cự cứng $\rightarrow$ **Chốt lời / Thoát khẩn cấp!**

2. **7 Điều kiện xác nhận `Breakout Thật` (Chống xác suất hỏng số đông của Breakout lần 1):**
   1. **Thuận xu hướng VN-Index & Cấu trúc MA mở rộng:** `MA20 > MA50 > MA200` và các đường MA phải đang **dốc lên mở rộng** (cảnh giác nếu `MA20 > MA50` nhưng đang uốn cong chúc xuống!). Không mua hộp hẹp nằm lọt thỏm giữa một biên sideway lớn không có sóng tăng dẫn đường.
   2. **Nền nén chặt + Khối lượng TRƯỚC Breakout phải THẤP:** Nếu trước phiên nổ mà `Volume` duy trì cao suốt nhiều phiên thì rất dễ là phân phối kéo dài và phiên nổ hôm nay là cú xả cuối cùng! *(Thời gian chạy kỳ vọng tỷ lệ thuận với thời gian nén: nén 20 phiên chạy ~20 phiên; nén 6 tháng chạy ~6 tháng).*
   3. **Phá ngưỡng Kháng cự thực sự quan trọng** (đỉnh lịch sử Vol lớn, hoặc Đỉnh Tháng trùng Đỉnh Quý, hoặc Điểm chạm số 3 của Tam giác/Phễu giảm).
   4. **Quy tắc Hình học Nến Đột phá `2/3 – 1/3` (`HP4 v3_5_thaoluan` — Case `TCB`):** Cây nến xanh thân dài đột phá chuẩn phải có **tối thiểu `2/3` thân nến nằm vượt hẳn lên phía trên vùng sideway** (tối đa `1/3` thân nằm lại trong hộp). Nếu `2/3` thân vẫn chìm trong hộp và chỉ nhú `1/3` lên trên cản $\rightarrow$ chưa mua ngay phiên đầu, chờ phiên sau xác nhận!
   5. **Khối lượng phiên Breakout tăng đột biến** ($\ge 1.25–1.5 \times \text{SMA}(V, 5)$) để hấp thụ cung kẹp đỉnh cũ *(riêng breakout cản chéo giảm ở đáy chỉ cần `Vol` ngừng giảm sau chuỗi cạn cung)*.
   6. **Bài toán `Reward / Risk > 1` bắt buộc:** $\frac{\text{Target} - \text{Entry}}{\text{Entry} - \text{StopLoss}} > 1$.
   7. **Kiểm định `Retest` & Kỷ luật thép `T+3 Time-Stop` (`HP4 v2_5_2`):** Sau phiên nổ, giá thường lình xình hoặc retest lại cạnh hộp vừa vượt (với `Vol` giảm dần dạng phễu giảm nhỏ) trong `2–3 phiên` mà **không đóng cửa thủng ngược vào hộp / thủng đáy cây nến breakout** ("Kill nến xanh"). Đặc biệt khi cược Breakout ở vùng cao (Hộp 4): **Đúng ngày `T+3` hàng về tài khoản mà giá không tăng tiếp như kỳ vọng $\rightarrow$ BÁN NGAY LẬP TỨC kể cả hòa vốn hay lỗ nhẹ!** *(Và cảnh giác Bulltrap trong Downtrend: nếu vừa rơi mạnh mà nảy hồi kèm `Vol` vọt tăng quá nhanh là hàng kẹp tranh xả thoát thân = Bulltrap!).*

---

## CHƯƠNG 5: HỆ THỐNG CHỈ BÁO KỸ THUẬT TRONG QMV & GIẢI MÃ TOÀN DIỆN HỘP ĐEN QMV FINTECH

### 5.1. Phân loại Chỉ báo, Thiết quân luật Chế độ Thị trường & Bộ Tham số Chuẩn QMV
*(Nguồn: `HP4 v1_3, v2_3_1..4, v3_3_bg + hd`, `KT01T2, KT01T3`, `V7 Guide`, `Fintech Buổi 1 & 2`, `wa_10_02_2026`)*

1. **Cách phân loại riêng của QMV & Thiết quân luật Bất đối xứng theo Xu hướng:**
   - **Nhóm 1 — Trung bình động (`Overlay`, không chuẩn hóa: `MA, Bollinger Bands, Ichimoku`):** Chậm hơn, dùng để **Xác nhận & Giữ hàng** khi thị trường **Có xu hướng (`Trending`)**; bị nhiễu cắt qua cắt lại liên tục (`whipsaw` chết vì `T+`) khi thị trường **`Sideways`**.
   - **Nhóm 2 — Sức mạnh xu hướng (`Sub-pane`, chuẩn hóa thống kê về 3 vùng tâm lý *Quá mua – Trung tâm – Quá bán*: `RSI, CCI, Stochastic, Williams %R, CMF, MFI`):** Nhạy hơn (dẫn dắt / cảnh báo sớm). **Tuyệt đối cấm dùng ngược xu hướng lớn:**
     - **Trong `Uptrend` mạnh:** Chỉ báo vào **Quá mua (`RSI > 70`, `CCI > +100`)** sẽ neo lì cả tháng ("bị liệt" chiều bán) $\rightarrow$ **TRÌ HOÃN BÁN (`Let it run`)**, chỉ bán khi cắt thủng khỏi vùng quá mua hoặc gãy trendline của chính chỉ báo! Ngược lại, chỉ báo rớt chớp nhoáng về **Quá bán** (nhất là trên `1D` hoặc `1H`) $\rightarrow$ **HÀNH ĐỘNG MUA NHANH!**
     - **Trong `Downtrend` mạnh:** Chỉ báo rơi vào **Quá bán (`RSI < 30`, `CCI < -100`)** sẽ nằm bẹp dí dưới đáy $\rightarrow$ **HOÃN MUA tối đa (cấm bắt dao rơi)**! Ngược lại, chỉ báo hồi chạm **Quá mua** $\rightarrow$ **BÁN THOÁT HÀNG NGAY LẬP TỨC!**
     - **Trong `Sideways`:** Tắt tín hiệu giao cắt `MA`, chuyển sang đánh biên `Hỗ trợ/Kháng cự ngang + RSI/CCI + Rebalancing`.
   - **Nhóm 3 — Lưỡng tính (`MACD`):** Cấu tạo từ hiệu trung bình động nhưng chuẩn hóa xoay quanh trục `0` (cắt lên trên trục `0` cực khỏe; đọc thêm Histogram và Phân kỳ).

2. **Bảng Tham số Gốc & Quy tắc Độc chiêu của từng Chỉ báo qua các thế hệ học liệu QMV:**

| Chỉ báo | Bộ Tham số Chuẩn trong QMV | Quy tắc Độc chiêu Nguyên bản QMV (100% tính được từ `OHLCV`) |
| :--- | :--- | :--- |
| **`MA` (`SMA` & `EMA`)** | - Khung `1D`: **`SMA(5)`** (1 tuần), **`SMA(20)`** (1 tháng), **`SMA(50)`** (1 quý), **`SMA(100)`** (6 tháng), **`SMA(200)`** (1 năm); hoặc hệ `9 – 20 – 50` (`HP5`).<br>- Khung `1W`: **`EMA(200)`** (`wa_10_02_2026`).<br>- Khối lượng: **`SMA(Volume, 5)`**. | - *Kỷ luật nhất quán:* Mua vì cắt lên `MA(k)` nào thì bắt buộc bán theo đúng `MA(k)` đó, cấm đổi sang MA dài hơn để bao biện gồng lỗ (`HP4 v1_3`).<br>- *Góc nhìn độc quyền `Weekly EMA200`:* Với cổ phiếu cơ bản tốt (`DT1/DT2` như `MBB, DGC, VCB`), vùng tiệm cận hoặc nhúng xuyên nhẹ dưới `EMA(200)` khung Tuần sau 3 nhịp giảm **KHÔNG phải điểm bán downtrend sách vở mà là bệ đỡ gom hàng cùng giá vốn quỹ lớn**! |
| **`Bollinger Bands`** | - Giáo trình `HP4`: **`BB(20, 2.0)`**.<br>- Biểu đồ cá nhân thầy Hào (`KT01T3`): **`BB(15, 2.0)`** kèm `SMA(15)`. | - *Overreaction:* Giá vọt ra ngoài biên trên/dưới BB và bám biên $\rightarrow$ xu hướng cực mạnh tiếp diễn (*cấm chặn tàu*).<br>- *Tín hiệu Đảo chiều:* Giá từ **bên ngoài biên BB chui ngược trở lại vào bên trong dải BB** (đặc biệt ở **lần thứ 2** chui vào trong BB thì độ tin cậy đảo chiều cực cao — `KT01T3`).<br>- *`BB Squeeze` (Thắt nút cổ chai so với lịch sử chính mã đó):* Báo hiệu sắp nổ biên độ lớn; hướng nổ phải xác nhận bằng **Phân kỳ `RSI/CCI` + `MACD` cắt lên**. |
| **`Ichimoku Kinko Hyo`** | - Giáo trình `HP4`: Chuẩn **`(9, 26, 52, 26)`**.<br>- **Độc quyền QMV Fintech `V7` (`V7 Guide` tr.7, 29 & `Fintech B1–B2`):** **`(18, 6, 18, 36)`** *(tức `Tenkan = 6`, `Kijun = 18`, `Senkou B = 36`, `Dịch chuyển = 18`; biến thể ngắn `3 – 6 – 18 – 36`)* nhanh gấp ~1.5x chuẩn Nhật để khớp nhịp tháng/quý VN! | - *Giải ngân 2 nhịp xuyên mây:* Vào thăm dò khi giá tạo điểm uốn chui từ dưới vào trong mây; gia tăng khi giá vượt hẳn bờ trên đám mây + mây tương lai mở rộng xanh + `Chikou` rời xa mây.<br>- *Phát hiện **Bear-Trap** (trong Uptrend):* Giá đang **trên mây**, bị đạp thủng xuống dưới `Kijun` rồi vòng lên **vượt ngược lên trên `Tenkan` / `Kijun`** $\rightarrow$ bẫy giảm giả, mua mạnh!<br>- *Phát hiện **Bull-Trap** (trong Downtrend):* Giá đang **dưới mây**, hồi vượt lên trên `Kijun` rồi **rơi thủng ngược xuống dưới `Tenkan`** $\rightarrow$ bẫy tăng giả, thoát ngay! |
| **`RSI` & `CCI`** *(Combo tủ của thầy Hào)* | - `RSI(14)` (trục `50`, biên `30 – 70`).<br>- `CCI(14)` (linh hoạt `10` hoặc `20` theo lịch sử mã; trục `0`, biên `-100 .. +100`), **gắn thêm đường `SMA` ngay trên `CCI`** (`KT01T3`). | - Kẻ trực tiếp **Trendline và Kháng cự/Hỗ trợ ngang lên chính đường `RSI` & `CCI`** (`Điểm 1`: phá trendline của chỉ báo; `Điểm 2`: cắt qua mốc `30`/`70` hoặc cắt đường `MA` của `CCI`).<br>- Khi **cả `RSI` VÀ `CCI` đồng thời phân kỳ** (confirm lẫn nhau), xác suất đảo chiều tăng vọt. |
| **`MACD`, `Stochastic`, `CMF`** | - `MACD(12, 26, 9)` (trục `0`).<br>- `Stochastic(K=15, D=3, Smooth=3)` (`HP5`).<br>- `CMF(20)` (trục `0`). | - `MACD` là chỉ báo số 1 để bắt **Phân kỳ dương Đáy 3** trên khung `1D` (`wa_10_02_2026`). Chốt lời theo độ cao `MACD`: cắt xuống gần `0` chỉ chốt nhẹ 1 phần; cắt xuống ở vùng cực đại lịch sử (`Stoch > 80`) $\rightarrow$ thoát toàn bộ.<br>- `CMF` đối chiếu đỉnh: Index vượt đỉnh mà `CMF` thấp dần = tiền không vào thêm, chỉ kéo trụ. |

3. **Ma trận 4 Loại Phân Kỳ (`Divergence`) & Cò súng Xác nhận Nhân quả 100%:**
   - **Quy tắc nối đỉnh/đáy (`HP4 v1_3`):** Ưu tiên nối theo **`High` (ở đỉnh)** và **`Low` (ở đáy)** thay vì `Close`, với 2 đỉnh/đáy không cách quá xa nhau:
     1. *Phân kỳ Dương Thường (`Regular Bullish` — Đảo chiều Tăng ở đáy):* $L_{\text{sau}} \le L_{\text{trước}}$ NHƯNG $\text{Ind}_{\text{sau}} > \text{Ind}_{\text{trước}}$ (hoặc $L_{\text{sau}} < L_{\text{trước}}$ và $\text{Ind}_{\text{sau}} = \text{Ind}_{\text{trước}}$).
     2. *Phân kỳ Dương Ẩn (`Hidden Bullish` — Tiếp diễn Tăng ở đáy chỉnh):* $L_{\text{sau}} > L_{\text{trước}}$ NHƯNG $\text{Ind}_{\text{sau}} \le \text{Ind}_{\text{trước}}$ (cực đẹp nếu `RSI` vẫn giữ $\ge 50$ như case `VIB`; lưu ý dư địa thường là nhịp cuối ngắn hơn phân kỳ thường).
     3. *Phân kỳ Âm Thường (`Regular Bearish` — Đảo chiều Giảm ở đỉnh):* $H_{\text{sau}} \ge H_{\text{trước}}$ NHƯNG $\text{Ind}_{\text{sau}} < \text{Ind}_{\text{trước}}$.
     4. *Phân kỳ Âm Ẩn (`Hidden Bearish` — Tiếp diễn Giảm ở đỉnh hồi):* $H_{\text{sau}} < H_{\text{trước}}$ NHƯNG $\text{Ind}_{\text{sau}} \ge \text{Ind}_{\text{trước}}$.
   - **Điều kiện kích hoạt Nhân quả (Cấm đoán phân kỳ khi giá đang phi thẳng đứng!):**
     - *Điều kiện cần:* Giá phải chững lại **"lình xình" tối thiểu `2–3 phiên`** cắt ra khỏi trendline ngắn hạn + chỉ báo thực sự bẻ góc tạo **Điểm uốn** (cắt đường `MA` của chính chỉ báo).
     - *Cò súng Giá định lượng (`HP5 v3_4_baigiang`):*
       - **Mua Phân kỳ Dương:** Kích hoạt tại phiên `t` khi giá đóng cửa vượt lên trên mức `High` của cây nến có đáy thấp nhất vùng phân kỳ: $C_t > H_{t_{\text{lowest\_low}}}$.
       - **Bán Phân kỳ Âm:** Kích hoạt tại phiên `t` khi giá đóng cửa thủng xuống dưới mức `Low` của cây nến có đỉnh cao nhất vùng phân kỳ: $C_t < L_{t_{\text{highest\_high}}}$.

---

### 5.2. Giải mã Toàn diện Hộp Đen QMV Fintech (`Blackbox V5/V7`) & Ranh giới Dữ liệu
*(Nguồn: `QMV_SYSTEM_GUIDE_V5.md`, `HƯỚNG_DẪN_V7.pdf.md`, `Chọn_cổ_phiếu_Fintech_QMV_17072025.mp3.md`, `fintech_buoi_1_21052026_audio.mp3.md`, `Fintech_buổi_2_25062026_audio.mp3.md`)*

> **Tiên đề "Cái Hộp Khép Kín" của QMV Blackbox:** Mỗi cổ phiếu / ngành / toàn thị trường là một chiếc hộp đóng kín với nguồn lực hữu hạn: Bên mua bỏ **Tiền (`DM`, `TM`)** vào hộp đẩy giá lên; Bên bán bỏ **Cổ phiếu (`DS`, `TS`)** vào hộp đè giá xuống; thuật toán **ưu tiên Cầu (Tiền) hơn Cung (Cổ phiếu)**. Khi tiền rút cạn chạm ngưỡng **"Duy trì"** (`<20` trên thang chuẩn hóa 50 phiên) $\rightarrow$ kích hoạt bơm tiền quay lại (*Contrarian Buy*); khi tiền đổ vào quá mức chạm ngưỡng **"Bão hòa"** (`>80`) $\rightarrow$ kích hoạt rút tiền ra (*Contrarian Sell*).

| Tên Biến Độc Quyền QMV Fintech | Cấp độ & Thang đo | Bản chất Toán học Gốc (Từ lời giảng Thầy Hào & V5/V7) | Quy tắc Vận hành Thực chiến & Cảnh báo Bẫy | Khả năng Tái tạo từ `OHLCV` 10 năm |
| :--- | :--- | :--- | :--- | :--- |
| **`MB` (`MBA`, `MBZ`)** — *Cơ hội hấp dẫn* | Thị trường & Ngành (`0..100`, 6 chu kỳ `03, 05, 10, 20, 50, 200`) | Tỷ lệ % số cổ phiếu đủ thanh khoản ($\overline{V} \ge 10.000$ cp/phiên) đang ở vùng giá hấp dẫn/đắt đỏ so với lịch sử chính nó. `MBA` = gốc toàn thời gian; `MBZ` = chuẩn hóa `0–100` trong **50 phiên gần nhất**. | **ĐIỀU KIỆN CẦN:** `<20` = Hấp dẫn (Duy trì), `<30` = Quan sát, `30–70` = Bình thường (*uốn trong 30–70 chỉ là sideway, CẤM coi là tín hiệu!*), `>70` = Cảnh báo, `>80` = Khó khăn. **Cơ hội T+** đòi hỏi CẢ `MBZ03` VÀ `MBZ05` cùng uốn lên từ `<20` (`T++` thêm đường `10`; `Swing` thêm `20, 50`). | **Xấp xỉ rất sát (`High Fidelity`)** từ rổ 400+ mã thanh khoản trên sàn VN. |
| **`BB` (`BBA/BBZ - M, S, EM`)** — *Dòng tiền lan tỏa* | Thị trường & Ngành (`0..100`, 6 chu kỳ) | Tỷ lệ % cổ phiếu trên sàn/ngành đang có dòng tiền chảy vào (`M`), bị bán cổ phiếu ra (`S`), và chênh lệch lan tỏa ròng (`EM = M - S`). | **ĐIỀU KIỆN ĐỦ:** Xác nhận nhịp T+ chắc ăn khi **cả `MBZ(03,05)` VÀ `BBZ-M(03,05)` đồng thuận chụm dưới `<20` và uốn lên** (`MB` uốn mà `BBZ-M` không uốn $\rightarrow$ giá rẻ nhưng tiền chưa lan tỏa, đánh T+ cực khó ăn!). | **Xấp xỉ rất sát** từ Breadth dòng tiền 400+ mã. |
| **`TM-Chuẩn`, `TS-Chuẩn`, `Tốc độ SP` (`%SP`)** | Thị trường, Ngành & Cổ phiếu (`0..100` hoặc `0..1`) | Lũy kế tiền (`TM`) và cổ phiếu (`TS`) từ ngày gốc (`20/03/2024` trên V7, trước đó `20/03/2021`), chuẩn hóa cùng thang đo. **`SP = TM_Chuẩn - TS_Chuẩn`**; hàng ngày **`%SP = %DM - %DS`**. | Giải bài toán mù VSA: Khi giá tăng + vol tăng, nếu **`SP > 0` mở rộng** = Tiền mua áp đảo lượng cổ phiếu xả (giữ chặt!); nếu giá xanh nhưng **`SP < 0`** = Lượng cổ phiếu tuồn vào hộp lớn hơn tiền vào (xả ngầm!). Bảng Top chia Trái (độ lớn khi TT lên) vs Phải (nhảy bậc khi TT chỉnh). | **Xấp xỉ cơ học cao** bằng phân tách VSA `Price×Vol` (Cầu) vs `Vol` (Cung). |
| **`Sentiment Gauge`** — *Tâm lý thị trường* | Toàn thị trường | Tương quan dòng tiền chảy vào nhóm cổ phiếu **Rủi ro** so với nhóm **An toàn** (loại mã thanh khoản quá bé). | Điều tiết trực tiếp bảng *Định giá so sánh tâm lý*: Tâm lý chuyển Tiêu cực $\rightarrow$ Tích cực thì trên mức Bình thường vẫn giữ được tới mức Tích cực; chuyển Tích cực $\rightarrow$ Tiêu cực thì trên Bình thường đã phải Bán! | **Xấp xỉ chính xác** bằng tỷ số dòng tiền `DT2+DT3` so với `DT1`. |
| **`Cáo – Chó săn – Thỏ`** & **`Mua lên / Bán xuống`** | Cổ phiếu (Tick / Lệnh khớp) | **Độc quyền Tick-Rule QMV:** Chỉ cộng các lệnh khớp làm **dịch chuyển bước giá ($\Delta P_{\text{tick}} > 0$ = Mua đẩy lên; $\Delta P_{\text{tick}} < 0$ = Bán đạp xuống)**, chia 3 cỡ lệnh Lớn (Cáo) – Vừa (Chó săn) – Nhỏ (Thỏ). | Khác hoàn toàn Buy/Sell chủ động bảng điện thường (bỏ qua lệnh khớp ngang bước giá $\Delta P_{\text{tick}} = 0$). Sau 3 tuần im lìm tự nhiên cột mua Cáo–Chó–Thỏ vọt cao = có ý chí đẩy giá thật! | **Dữ liệu Tick độc quyền**; trên `OHLCV` ngày chỉ xấp xỉ được qua hướng thân nến $\times$ bậc `Vol/MA20`. |
| **`Price Trend`** & **`New High/Low`** | Cổ phiếu (Cửa sổ Quý `50–60 phiên`) | Lý thuyết là cấu trúc đỉnh/đáy quý; thuật toán chạy máy hiện tại dùng **hệ thống đường trung bình động 1 quý (`MA20/MA50`)** phân 3 nhãn `Tăng / Đi ngang / Giảm` + tiệm cận Đỉnh/Đáy `10, 20, 50, 60` phiên. | Đóng vai trò trụ cột số 1 của cấu trúc giá. | **Tính chính xác ~100%** từ `OHLCV`. |
| **`TMC` + `TMA20` + `TMA50` $\rightarrow$ `BB Status` & `BB Predict`** | Cổ phiếu | `TMC` = Tổng tiền lũy kế vào hộp cổ phiếu; `TMA20 = SMA(TMC, 20)`, `TMA50 = SMA(TMC, 50)`. Từ vị trí & giao cắt 3 đường này sinh ra 4 pha **`BB Status`**: `Duy trì (1) -> Tiền khỏe (2) -> Bão hòa (3) -> Tiền yếu (4)` và ngày giao cắt **`BB Predict`** (`Dấu hiệu / Đang / Xu hướng vào–ra`). | **KIM CHỈ NAM TỐI THƯỢNG:** Nếu `TMC < TMA20 < TMA50` dốc xuống (`Tiền yếu`), mọi tín hiệu cầu/phân kỳ bên dưới chỉ như hòn đá lăn dừng nghỉ tạm trên dốc, **cấm mua mới vội**! Trong TT khó, mã đang giữ chớm báo `Tiền dấu hiệu ra` là thoát ngay! | **Xấp xỉ rất sát** từ chuỗi `TMC` tái tạo trên `OHLCV`. |
| **`DSPI` (`[-1, +1]`, `MA5, MA20`) $\rightarrow$ `XH Cầu`** | Cổ phiếu (Chu kỳ `20 phiên` = 1 tháng) | Đo sức ép Cầu (tiền vào) tương đối so với Cung (cổ phiếu vào hộp) trong 20 phiên quanh trục cân bằng `0`. Sinh ra 4 pha **`XH Cầu`**: `Cầu duy trì -> Cầu khỏe -> Cầu bão hòa -> Cầu yếu`. | Con lắc 20 phiên xoay quanh đoàn tàu lớn `BB Status`: Tổ hợp đẹp nhất là **`Price Trend = Tăng` + `BB Status = Khỏe/Duy trì` + `XH Cầu` chuyển từ `Duy trì -> Khỏe` (`DSPI` uốn lên vượt `0`)**. | **Xấp xỉ rất sát** từ hiệu chuẩn hóa Cầu – Cung 20 phiên. |
| **`MPICP` (`[0, 100]`, `MA5, MA20`)** — *Sức ép tâm lý CP* | Cổ phiếu (Quanh trục `50`) | **Đo lượng TIỀN NHỎ (Nhỏ lẻ) đi vào so với TIỀN LỚN!** Nhỏ lẻ vào nhiều $\rightarrow$ tâm lý hưng phấn (`MPICP -> 80..100`); nhỏ lẻ bán tháo sạch $\rightarrow$ tâm lý bi quan (`MPICP -> 0..20`). | Lùi sâu `<< 50` rồi nhích lên = điều kiện cần tạo đáy; vọt sát đỉnh `80–100` = **ưu tiên chốt lời T+ ngay** (như case `VIC` 25/06/2026). Là 1 trong 3 lõi tính Phân kỳ QMV (`MPICP + RSI + CCI`). | **Cần phân lớp quy mô lệnh**; trên `OHLCV` xấp xỉ qua tổ hợp xung lượng ngắn hạn & dòng tiền phiên vol vừa/nhỏ. |
| **`DM03..200` & `DS03..200` (Mặc định vs Chuẩn hóa `Z` 50 phiên)** | Cổ phiếu & Thị trường | Cắt lát tiền/cổ phiếu theo 6 chu kỳ nắm giữ. **Chu kỳ `03` quan trọng nhất** vì khớp 100% quy chế hàng về **`T+2.5`** tại VN (`EM03 > 0` = hàng T+ về được hấp thụ hết, không bị bán tháo!). | **CẢNH BÁO 2 BẪY CHUẨN HÓA 50 PHIÊN:** *(1) Bẫy biên độ chết:* `DM` mặc định phẳng lì sát `0` nhưng `DMZ` vẫn kéo thành `0..100` (mua vào chỉ nhích 1%); *(2) Bẫy neo đỉnh:* `DM` mặc định neo tít trên cao nhưng đáy cục bộ vẫn báo `DMZ < 20`! **Bắt buộc soi chéo cả Mặc định lẫn Chuẩn hóa** (chỉ bật `03, 05, 10, 20` khi chuẩn hóa). | **Xấp xỉ rất sát** kèm bộ lọc chống bẫy biên độ chết. |
| **`Bullish Signal 1..4` vs `Bearish Signal 1..3`** + `MaxBuy, Target 1/2, StopLoss, MinSell` | Cổ phiếu (Tín hiệu hành động) | `Pattern` = thuần PTKT; `Signal` = `Pattern` + Blackbox + RR. Mua: `1` (Mua đáy), `2` (Mua hỗ trợ), `3` (Mua đuổi Momentum), `4` (Mua phá đỉnh Breakout). Bán: `1` (Bán đỉnh), `2` (Bán kháng cự), `3` (Bán đuổi). | **TẠI SAO KHÔNG CÓ `BEARISH SIGNAL 4` (Bán thủng hỗ trợ)?** QMV chủ đích triệt tiêu số 4 chiều bán vì backtest cho thấy báo bán breakdown làm nhỏ lẻ toàn bán đúng đáy rũ `Spring`! Thay vào đó dùng `StopLoss` định sẵn lúc mua (chỉ cắt khi bối cảnh xấu + nến đỏ dài xuyên thủng) và `MinSell` chặn bán tháo giá thấp. | **Tái tạo trọn vẹn 100% logic cấu trúc** trên Sumi. |
| **`Phù hợp Trading (0–5 sao)` + `Tốt hơn / Kém hơn`** | Cổ phiếu | Hội tụ 3 trụ (`Price Trend + BB Status + DSPI/MPICP`) ra `0–5 sao` + đạo hàm bậc 1 so với hôm qua (`Tốt hơn / Không đổi / Kém hơn`). | **Nghịch lý Contrarian cực đắt:** Trong TT Sideway/khó hoặc cuối sóng, đợi đủ `5 sao` mới mua là đu đúng ngọn sóng cuối; lọc **`0–2 sao (hoặc 2–3 sao) + Tốt hơn + Phân kỳ dương / Uốn lên 20`** mới bắt trúng chân sóng khởi phát có dư địa dày nhất! | **Tái tạo trọn vẹn 100%** từ 3 trụ cột. |

---

## CHƯƠNG 6: ĐI VỐN CHIA 3 PHẦN, QUẢN TRỊ RỦI RO, KỶ LUẬT `T+` & TOP SAI LẦM THỰC CHIẾN

*(Nguồn: `KT01T2, KT01T3, KT01T4`, `DM01T4`, `DM01_BUỔI_4`, `HP5 v3_4`, `wa_10_02_2026`, `wa_22_01_2026`, `HP4 v1_6, v2_6_1_6`, `memory/lessons/lesson_postmortem_*.json`)*

### 6.1. Phương trình Thực thi & Cấu trúc Quy mô Danh mục Chuẩn QMV
1. **Phương trình Thực thi Bất khả xâm phạm (`DM01_BUỔI_4`):**
   $$\text{Quyết định Thực thi (Execution)} = \underbrace{\text{Lý do Mua/Bán (Thesis — Điều kiện Cần)}}_{\text{Định giá rẻ / Chu kỳ ngành / Game sự kiện / Vùng Hỗ trợ–Kháng cự}} + \underbrace{\text{Tín hiệu Hành động (Trigger — Điều kiện Đủ)}}_{\text{Điểm uốn Nến + VPA + Phân kỳ + Blackbox uốn}}$$
   - **Kỷ luật đồng bộ nguyên nhân:** *"Vào vì lý do gì, Ra vì đúng lý do đó!"* Mua theo Hỗ trợ $\rightarrow$ chạm Kháng cự phải bán; mua vì Sự kiện cổ tức/tăng vốn $\rightarrow$ thoát trước ngày chốt quyền; mua vì Trading `T+` $\rightarrow$ gãy tín hiệu hoặc quá hạn `T+3` không chạy là thoát ngay, **tuyệt đối cấm chuyển giao dịch Trading thua lỗ thành "Đầu tư dài hạn bất đắc dĩ"!**

2. **Quy mô Watchlist, Giới hạn Tập trung & Kiểm soát Trùng kênh (`KT01T4`, `wa_10_02_2026`, `Postmortem Yumeko`):**
   - **Watchlist rộng (`9 – 20 mã`):** Thiết lập tối thiểu `3 ngành × 3 cổ phiếu/ngành` (hoặc 15–20 mã đại diện các cụm ngành luân chuyển) để cảm nhận dòng tiền xoay vòng mà không bị loạn giữa 1.700 mã.
   - **Giải ngân hẹp (`3 – 5 cổ phiếu`):** Cá nhân chỉ nắm giữ thực tế **3 đến 5 mã** có điểm vào đẹp nhất (vượt quá 20 mã lợi ích đa dạng hóa bằng 0).
   - **Giới hạn tỷ trọng đơn mã & Trùng kênh:** Tối đa **$\le 10\%$ NAV / 1 cổ phiếu** (trần cứng tuyệt đối không quá **$20\%$ NAV** trong trường hợp cực hiếm). Cấm gom 5–6 mã cùng trục nhạy cảm lãi suất (như bài học hậu kiểm tài khoản Personal ôm `66.9%` NAV ở trục Ngân hàng + Chứng khoán khiến rủi ro hệ thống phình đại).

---

### 6.2. Công thức Đi Vốn `1/3 – 1/3 – 1/3`, Phác đồ Cấp cứu Vị thế Sai & Kho 6 Phần
1. **Quy tắc Giải ngân Chia 3 Phần bằng nhau (`1/3 – 1/3 – 1/3` — `DM01_BUỔI_4`, `KT01T2`, `wa_10_02_2026`):**
   - **Lần 1 (`1/3` vốn dự kiến cho mã đó):** Giải ngân thăm dò ("đặt chân") tại điểm uốn sớm (Điểm chạm 1 ở Hộp 2 trong uptrend, hoặc nến đảo chiều đầu tiên tại vùng hỗ trợ). Chấp nhận trước tâm lý giá còn có thể nhúng test lại.
   - **Lần 2 (`1/3` vốn — Trọng tâm chính):** Giải ngân tại **Điểm chạm số 2 ("Test cung") hoặc Điểm chạm số 3** khi có xác nhận `Nến tăng/Spring + Vol tăng + Chỉ báo/Blackbox uốn lên` (hoặc bồi thêm ngay nếu sau Lần 1 giá bứt phá xác nhận đáy luôn).
   - **Lần 3 (`1/3` vốn dự phòng bắt buộc giữ lại):**
     - *Khi ĐÚNG:* Giá bứt phá vượt cản / retest thành công xác nhận vào sóng $\rightarrow$ giải ngân nốt `1/3` cuối cùng để đủ `3/3` vị thế ăn trọn thân sóng *(nếu giá đã phóng vọt quá xa làm `RR < 1` thì không mua đuổi phần cuối mà giữ nguyên `2/3` ăn lãi!)*.
     - *Khi SAI (Phác đồ Cấp cứu Sinh tử khi đã cầm `2/3` mà giá thủng đáy hộp hỗ trợ — `DM01_BUỔI_4` & `wa_10_02_2026`):* **TUYỆT ĐỐI CẤM nhồi thêm phần `1/3` dự phòng để bình quân giá xuống!** Ngay khi đóng nến xuyên thủng đáy vùng hỗ trợ (hoặc khựng lại ở nhịp hồi kỹ thuật chạm giá vốn Lần 1 mà chưa có phân kỳ MACD), **BẮT BUỘC CẮT NGAY `1/2` VỊ THẾ ĐANG GIỮ (tức bán dứt khoát `1/3`, chỉ giữ lại tối đa `1/3`)!**
       - *Ưu việt toán học "Win–Win":* Nếu cắt xong cổ phiếu bất ngờ kéo ngược lên $\rightarrow$ `1/3` còn giữ sinh lãi kéo cả deal về hòa/lãi; nếu cổ phiếu sập sâu tiếp về Đáy 3 thật $\rightarrow$ tài khoản đã **khôi phục được `2/3` sức mua tiền mặt** để đàng hoàng làm lại từ đầu ở vùng Đáy 3 chuẩn xác!
   - *(Biến thể cho nhà đầu tư tích lũy mã `Core` dài hạn `70% giữ cố định / 30% lướt` như `VCI` trong `wa_22_01_2026`: Chia vốn thành **4 phần bằng nhau theo SỐ TIỀN VNĐ cố định** tại 4 tầng hỗ trợ tuần — không gấp thếp Martingale — giúp giá càng thấp tự động mua được nhiều cổ phiếu hơn).*

2. **Kỹ thuật `Rebalancing` (Hạ giá vốn trên cùng khối lượng) & Công thức `Kho 6 Phần` vượt rào `T+2.5`:**
   - **Điều kiện làm Rebalancing:** Chỉ thực hiện khi cổ phiếu cơ bản tốt đang ở nửa dưới/giữa Bối cảnh lớn (`Hộp 1, 3`) và `BB Status = Duy trì / Khỏe` (nếu đã lên Bước `5` + `BB Bão hòa/Yếu` + nổ Vol đỉnh thì **thoát hẳn hoặc Arbitrage đổi sang mã chân sóng `0->1`**, không mua lại!).
   - **Thao tác:** Đang nắm chuẩn $Q_0$ cổ phiếu $\rightarrow$ chạm kháng cự có tín hiệu chững/Upthrust thì bán ra `1/3` hoặc `1/2` $Q_0$.
     - *Kịch bản 1 (Giá chỉnh về hỗ trợ):* Chờ **Điểm uốn + Cạn cung (chạm 2–3)** mua lại **đúng số lượng cổ phiếu đã bán** $\rightarrow$ giữ nguyên $Q_0$, bỏ túi chênh lệch hạ giá vốn!
     - *Kịch bản 2 (Bán sai — Giá vượt đỉnh chạy tiếp):* Kỷ luật thép của thầy Hào (`KT01T3` L335, `DM01_BUỔI_4`): **"Đã là Rebalancing trên danh mục đầu tư, bán xong DÙ ĐÚNG HAY SAI VẪN BẮT BUỘC PHẢI MUA LẠI!"** Không đua ngay cây breakout đầu tiên, mà đợi nhịp **chững lại / retest trên nền kháng cự vừa vượt (nay là hỗ trợ mới)** tạo điểm uốn tăng tiếp (`Bullish Signal 3`) để cover đủ lại số cổ phiếu đã bán!
   - **Công thức `Kho 6 Phần` (`HP4 v1_6`):** Chia tài khoản lướt Rebalancing thành `50% Cổ phiếu (chia 3 phần) + 50% Tiền mặt (chia 3 phần) = 6 phần`; mỗi ngày chỉ dùng tối đa `1 phần` $\rightarrow$ ngày nào cũng có sẵn hàng khả dụng và tiền ứng để đánh `T0` trong phiên trên khung `15m / 30m` bất chấp luật `T+2.5`! *(Riêng trên sàn **UPCoM** biên độ $\pm 15\%$ tính tham chiếu ngày mai theo **bình quân gia quyền** hôm nay: bắt buộc đóng vòng Rebalancing ngay trong ngày!).*

---

### 6.3. Quy tắc Đặt `Stop-Loss`, Dời `Trailing Stop` & Lịch Kiểm định `T+`
1. **Đặt `Stop-Loss` theo Cấu trúc Vùng Hỗ trợ (Không dùng `%` cố định máy móc!):**
   - Thầy Hào bác bỏ cắt lỗ cứng nhắc `-5% / -7%` lơ lửng giữa đường:
     - *Mua tại Hỗ trợ:* `StopLoss` đặt tại **mức giá thấp nhất (`Lowest Low`) của cây nến có râu dưới dài nhất quanh toàn bộ vùng hỗ trợ đó** (`KT01T2`).
     - *Mua Đột phá / Momentum:* `StopLoss` đặt tại **cận dưới của hộp tích lũy** hoặc **đáy cây nến Breakout**.
   - **Điều kiện thực thi Cắt lỗ (Tránh bán đúng đáy rũ `Spring` — `V5 Guide` tr.7 & `KT01T3`):** Khi hàng về mà giá chạm vùng `StopLoss`, mở ngay khung `30m/15m` hỏi: *"Có cầu dừng giảm ở đây không?"* Nếu `Giá dừng giảm + Lượng dừng giảm` $\rightarrow$ **DỪNG BÁN** chờ nhịp nảy `bull-trap` để xử lý giá tốt hơn. Chỉ cắt dứt khoát ngay lập tức khi **bối cảnh chung xấu VÀ nến đỏ dài xuyên thủng hỗ trợ không dừng giảm**!

2. **Hai Kỹ thuật Dời Điểm Chặn Lãi (`Trailing Stop`) khi Vị thế đã Có Lãi (`KT01T3 Audio`):**
   - **Cách 1 (Trên khung `1D` — Quy tắc `2/3` Sóng):** Khi giá tăng lên nấc cao mới, bỏ `StopLoss` ở đáy cũ; đo chiều cao nhịp tăng hiện tại từ chân sóng ($L_{\text{base}}$) lên đỉnh cao nhất đã đạt ($H_{\text{peak}}$) và **dời điểm chặn lãi lên mốc `2/3` tính từ dưới lên (tức chỉ cho phép lùi tối đa `1/3` tính từ đỉnh xuống)**:
     $$\text{TrailingStop}_{1D}(t) = H_{\text{peak}}(t) - \frac{1}{3}\big(H_{\text{peak}}(t) - L_{\text{base}}\big) = L_{\text{base}} + \frac{2}{3}\big(H_{\text{peak}}(t) - L_{\text{base}}\big)$$
   - **Cách 2 (Trên khung `30m` hoặc dưới Cây nến Nổ Vol đỉnh):** Dời chặn lãi bám sát ngay dưới **cận dưới của từng hộp đi ngang lình xình mới nhất trên khung `30m`**, hoặc ngay dưới **đáy `Low` của cây nến nổ Volume lớn vùng cao** (`PNJ` trong `wa_16_01_26`).

3. **Lịch Kiểm định Chu kỳ `T+` & Quy tắc `Setup Expiry / Time-Stop`:**
   - **Cửa sổ canh lệnh `3 ngày` (`T0 -> T+1 -> T+2` — `DM01T4`):** Khi có `Bullish Signal` ở `T0` nhưng giá vọt quá `MaxBuy` hoặc chưa đủ điểm chạm, giữ mã trong diện canh **3 phiên liên tiếp** để bắt điểm trùng test lại hợp lý ở `T+1` hoặc `T+2`. *(Trong TT Sideway, thấy mã vào bảng `Tiền vào 3 phiên liên tiếp` thì **cấm mua đuổi ở phiên 3** kẻo dính xả hàng về ở phiên 4; đưa vào Watchlist chờ chỉnh xong đánh nhịp 2!)*
   - **Kiểm định `T+2` cho Giao dịch Sự kiện hoảng loạn:** Hàng về chiều `T+2` (`T+2.5`) **bắt buộc thoát sạch 100%** bất kể lãi/lỗ (`DM01_BUỔI_4`).
   - **Kiểm định `T+3` & `Setup Expiry / Time-Stop` (`2 – 4 phiên` — `HP4 v2_6_1_6` & `v2_5_2`):**
     > *"Đã giao dịch dựa trên Phân tích kỹ thuật là nó phải NHANH! Nếu sau `2–3 ngày` hoặc `3–4 ngày` (1 vòng hàng `T+3` về) mà cổ phiếu KHÔNG chạy đúng kịch bản mong đợi (cứ lình xình đi ngang dù chưa chạm ngưỡng cắt lỗ) $\Rightarrow$ Tín hiệu kỹ thuật đó đã HẾT HẠN SỬ DỤNG, BẮT BUỘC ĐÓNG VỊ THẾ THOÁT RA ĐỨNG NGOÀI NGAY!"*

---

### 6.4. Tổng hợp 15 Sai lầm Thực chiến Kinh điển nhất của Học viên (Từ Q&A Khóa học & Hậu kiểm `Postmortem T+20`)
1. **Mê mẩn mẫu hình cổ phiếu riêng lẻ mà phớt lờ VN-Index đang Downtrend** hoặc Cổng vĩ mô đang khóa (`HP4 v1_6, v2_6_1_4`).
2. **Ngộ nhận "Nổ Volume khổng lồ trên vùng đỉnh cao = Tiền lớn mới vào"** nên nhảy vào đua lệnh đúng ổ phân phối của Cáo (`KT01T1`, `HP4 v2_6_1_2`).
3. **Đọc VPA trên 1 phiên đơn lẻ** thay vì nhìn chuỗi tiến trình tối thiểu `T+3` quanh `SMA(Vol, 5)`, hoặc nhầm khái niệm *"xanh vỏ đỏ lòng"* của Index với hiện tượng *"Tiết cung"* rất tốt của cổ phiếu đơn lẻ (`HP4 v3_1_hoidap`).
4. **Bắt dao rơi / Bình quân giá xuống sớm khi cổ phiếu đang rơi `Giá giảm + Vol tăng` (*Ba con quạ đen*), còn nằm dưới `SMA20/SMA50` và chưa có Phân kỳ dương cạn cung** — được chứng minh đắt giá qua chuỗi 4 lệnh hậu kiểm liên tiếp của `FPT` (lỗ thêm `-10.68%` đến `-18.90%` trong 20 phiên) và `VTP` (`-13.37%`) trong `memory/lessons/lesson_postmortem_*.json`!
5. **Bị đánh lừa bởi "Bẫy Độ rộng Ngắn hạn hồi phục trên Nền Chu kỳ Dài bị phe bán khống chế"** (`BBM05 = 59%` nhưng `BBS200 = 88.7%` trong `regime_journal/2026-10-15.json`), ôm thụ động `hold_monitor` cổ phiếu gãy cấu trúc trung hạn khiến tài khoản bị "luộc ếch" cưa chân bàn.
6. **Mua đuổi khi nến kéo xanh/tím giữa phiên (trước `14:15` / `ATC`)** hoặc sập bẫy đội lái khớp 100 cổ giá sàn đầu phiên ATO vẽ nến xanh dài giả (`wa_10_02_2026`, `HP4 v1_4`).
7. **Sập bẫy giá tham chiếu bình quân gia quyền trên sàn `UPCoM`** (hoảng sợ bán đáy cuối phiên hoặc đua trần ảo quên mất ngày mai giá tham chiếu tự co về trung bình gia quyền — `HP4 v1_6`).
8. **Vi phạm `Setup Expiry` (`3–4 ngày` không chạy không chịu thoát) & Nhập nhằng biến lệnh `Trading T+` bị lỗ thành "cổ đông dài hạn"** trên cổ phiếu đầu cơ `DT3` / Trà đá (`HP5 v3_4`, `HP4 v2_6_1_6`).
9. **Hoảng loạn cắt lỗ đúng vùng Hỗ trợ cứng** khi khung `1H/30m` đã quá bán sâu, nổ vol bắt đáy và rút chân `Spring` (lúc gãy sớm trên cao thì tiếc không bán, về đúng đáy hỗ trợ lại đem bán tháo — `KT01T3`, `HP4 v1_6`).
10. **Đua lệnh Breakout lần thứ 1 ở Hộp cao (Bước 4–5) khi `RR <= 1`**, hoặc nhầm lẫn giữa *Mua theo đà (chờ retest sau break)* với *Mua đuổi mù quáng* (`KT01T2`, `HP4 v3_6_hoidap`).
11. **Bị lừa bởi Màu sắc thân nến & Đọc sai nến Trần/Sàn cứng:** Thấy thân đỏ nhỏ ở `1/3 Trên` lại sợ giảm (dù bản chất là Nến Tăng!), hoặc thấy vạch ngang `O=H=L=C` lúc sàn cứng lại tưởng Doji đảo chiều (`HP4 v3_2_thaoluan`).
12. **Dùng chỉ báo Dao động (`RSI/CCI`) ngược xu hướng lớn** (bán non ngay khi `RSI` vừa chớm `>70` ở đầu Uptrend làm mất sạch sóng 3, hoặc bắt đáy ngay khi `RSI` vừa `<30` trong Downtrend) và tự vẽ phân kỳ sớm (*Overthinking*) khi giá chưa lình xình và chỉ báo chưa tạo điểm uốn (`HP4 v3_3_bg`).
13. **Mắc bẫy Chuẩn hóa 50 phiên của Blackbox** (thấy `DMZ < 20` uốn lên là mua mà không kiểm tra bản Mặc định đang bị bóp biên độ chết `1%` hay đang neo lơ lửng trên đỉnh cao) và chờ cổ phiếu đủ `5 sao` mới mua ở cuối sóng thay vì bắt `1–2 sao + Tốt hơn` (`Fintech Buổi 1 & 2`).
14. **Mua cổ phiếu phòng thủ / `DT1` cuối chu kỳ khi thấy "Đổi gác" từ `DT3` về `DT1`**, không hiểu đó là hành vi bất đắc dĩ của quỹ lớn còn cá nhân phải rút về `100%` tiền mặt (`HP5 v3_4`).
15. **Đảo hàng sai nhịp `T+` ("Đứng núi này trông núi nọ") & Tập trung quá mức / Trùng kênh ngành** (bán mã nền tích lũy sắp nổ để nhảy sang mã đã tăng nóng đúng đỉnh `T+3`, hoặc dồn `>20%` NAV vào 1 mã và `>60%` NAV vào 1 cụm ngành — `HP4 v1_6`, `Postmortem Yumeko`).
