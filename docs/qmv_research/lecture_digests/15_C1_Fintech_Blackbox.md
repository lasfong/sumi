# BÁO CÁO NGHIÊN CỨU CHUYÊN SÂU HỆ THỐNG QMV FINTECH & HỘP ĐEN BLACKBOX (KHỐI C1)

**Phạm vi tài liệu đã đọc trọn vẹn 100% từng byte (không cắt cụt):**
1. **`[V5]`**: `/Users/mizuhara/workspace/yumeko/docs/research/QMV_SYSTEM_GUIDE_V5.md` (80,293 bytes / 2,019 dòng — bản 22/12/2025).
2. **`[Audio_17072025]`**: `/Users/mizuhara/workspace/yumeko/docs/research/qmvcourse/qmv_chuyende/Chọn_cổ_phiếu_Fintech_QMV_17072025.mp3.md` (149,754 bytes).
3. **`[Audio_B1_21052026]`**: `/Users/mizuhara/workspace/yumeko/docs/research/qmvcourse/qmv_others/fintech_buoi_1_21052026_audio.mp3.md` (114,479 bytes).
4. **`[Audio_B2_25062026]`**: `/Users/mizuhara/workspace/yumeko/docs/research/qmvcourse/qmv_others/Fintech_buổi_2_25062026_audio.mp3.md` (183,409 bytes).
*(Đối chiếu bổ trợ thêm các dòng định danh mã biến mới năm 2026 từ bản hướng dẫn mà giảng viên mở trực tiếp trên màn hình trong Buổi 2: `HƯỚNG_DẪN_SỬ_DỤNG_HỆ_THỐNG_THÔNG_TIN_QMV_-_V7.pdf.md` - ký hiệu **`[V7_ref]`**).*

---

## PHẦN 1: GIẢI MÃ BẢN CHẤT TOÁN HỌC & Ý NGHĨA VẬN HÀNH CỦA TOÀN BỘ BIẾN ĐỘC QUYỀN QMV FINTECH

### 1.0. Tiên đề Gốc của Hộp Đen QMV Blackbox (`[V5]`, `[Audio_17072025]`, `[Audio_B2_25062026]`)
- **Mô hình "Cái hộp khép kín"**: Giả định thị trường của một cổ phiếu / ngành / toàn thị trường là một chiếc hộp đóng kín:
  - **Người mua** bỏ **Tiền (`Money`: `DM`, `TM`)** vào hộp $\rightarrow$ đẩy giá tăng.
  - **Người bán** bỏ **Cổ phiếu (`Stock/Shares`: `DS`, `TS`)** vào hộp $\rightarrow$ đẩy giá giảm (`[V5]` tr.14, `[Audio_17072025]`).
- **Tiên đề Giới hạn Nguồn lực & Nguyên lý Contrarian (Phản biện thái cực)**:
  - Trong một khoảng thời gian nhất định, tổng lượng tiền và lượng cổ phiếu tham gia cuộc chơi tương đối ổn định/hữu hạn (`[Audio_B2_25062026]`).
  - Khi tiền bị rút ra quá nhiều, hộp chạm ngưỡng tối thiểu để duy trì cuộc chơi gọi là vùng **"Duy trì"** $\rightarrow$ kích hoạt hành vi **bơm tiền trở lại** (cơ hội mua).
  - Khi tiền đổ vào quá nhiều trong ngắn hạn, nguồn lực cạn dần chạm giới hạn gọi là vùng **"Bão hòa"** $\rightarrow$ kích hoạt hành vi **rút tiền ra** (cơ hội bán) (`[V5]` tr.14).
  - Toàn bộ công cụ QMV vận hành theo nguyên lý **Contrarian**: chỉ báo càng chụm lại ở thái cực thấp (`<20`, `<30`) và uốn lên thì phản ứng đảo chiều tăng càng mạnh; càng chụm ở thái cực cao (`>70`, `>80`) và uốn xuống thì nguy cơ sụt giảm càng lớn (`[Audio_B2_25062026]`).
- **Tiên đề Bất đối xứng Cung - Cầu**: Trong thuật toán lõi, QMV **ưu tiên Cầu (Tiền) hơn Cung (Cổ phiếu)** với lập luận rằng sự thay đổi giá do lực Cầu dẫn dắt tạo ra cấu trúc xu hướng bền vững hơn so với sự thay đổi do Cung (`[V5]` tr.14).

---

### 1.1. CẤP THỊ TRƯỜNG & NGÀNH

#### (1) `MB` (`MBA`, `MBZ`) — Nhóm chỉ số "Cơ hội hấp dẫn" (`[V5]` tr.14-19, `[Audio_B2_25062026]`)
- **Bản chất toán học**:
  - Đo lường **tỷ lệ phần trăm (%) số cổ phiếu có thể giao dịch trên toàn thị trường (hoặc trong ngành)** đang nằm ở vùng giá hấp dẫn hay đắt đỏ **so với lịch sử quá khứ của chính từng cổ phiếu đó** (`[Audio_B2_25062026]`).
  - Lọc nhiễu đầu vào: Chỉ tính các cổ phiếu đủ thanh khoản giao dịch (ngưỡng trung bình 5 phiên / 1 tháng $\ge 10.000$ cổ phiếu/phiên để loại bỏ các mã chỉ khớp 1.000–2.000 CP gây méo chỉ số — `[Audio_B2_25062026]`).
  - Cắt theo 6 chu kỳ hành vi nhà đầu tư điển hình: **`03`** (T+3 / T+2.5), **`05`** (1 tuần), **`10`** (2 tuần), **`20`** (1 tháng), **`50`** (1 quý), **`200`** (1 năm) (`[V5]` tr.14, `[Audio_B2_25062026]`).
  - **Phân biệt `MBA` vs `MBZ`**:
    - **`MBA`** (`A` = *Actual / Anchor từ kỳ gốc*): Số liệu gốc tính xuyên suốt từ kỳ gốc, cho biết vị thế thật của cơ hội trên toàn khung thời gian rộng (ví dụ so sánh với đáy sâu nhất nửa năm / nhiều năm trước) (`[V5]` tr.14, 19; `[Audio_B2_25062026]`).
    - **`MBZ`** (`Z` = *Chuẩn hóa lăn 50 phiên*): Chuẩn hóa giá trị `MB` về thang `0–100` trong lát cắt **50 phiên giao dịch gần nhất (1 quý)** (`[V5]` tr.14, 19; `[Audio_B2_25062026]`).
- **Thang đo & Ý nghĩa vận hành**:
  - **Lưu ý chiều biến thiên**: Khi thị trường rơi sâu (nhiều cổ phiếu giảm quá đà trở nên "xấu quá mức" so với lịch sử), đường `MB` đi **XUỐNG** vùng thấp `<20`; theo nguyên lý contrarian, xấu kiệt cùng buộc tiền phải quay lại nên `<20` được gọi là vùng **"Hấp dẫn"** (`[Audio_B2_25062026]`, `[V5]` tr.15):
    - `< 20`: **Vùng Hấp dẫn** (tương ứng vùng tiền Duy trì).
    - `20 – 30` (`< 30`): **Vùng Quan sát**.
    - `30 – 70`: **Vùng Bình thường** (nếu đường chỉ số uốn trong vùng này chỉ hàm ý xu hướng trước đó chững lại / tạo sideways, **tuyệt đối không coi là cơ hội mua hay nguy cơ bán** — `[V5]` tr.15).
    - `70 – 80` (`> 70`): **Vùng Cảnh báo**.
    - `> 80`: **Vùng Khó khăn** (tương ứng vùng tiền Bão hòa).
  - **Quy tắc kích hoạt tín hiệu (Phải có Điểm Uốn + Hội tụ đa chu kỳ)**:
    - Tự thân việc nằm dưới 20 chưa phải điểm mua mà phải đợi **UỐN LÊN** từ `<20` ("Cơ hội hấp dẫn") hoặc `<30` ("Cơ hội quan sát"); chiều ngược lại **UỐN XUỐNG** từ `>80` ("Nguy cơ khó khăn") hoặc `>70` ("Nguy cơ cảnh báo") (`[V5]` tr.15).
    - **Cơ hội T+**: CẢ HAI đường `MBZ03` VÀ `MBZ05` cùng uốn lên từ vùng `<20` (nếu chỉ 1 trong 2 đường uốn thì chưa đủ điều kiện xác nhận cơ hội, chỉ đáng quan sát — đặc biệt nhạy ở chiều bán) (`[V5]` tr.15, `[Audio_B2_25062026]`).
    - **Cơ hội T++**: Có Cơ hội T+ (`03, 05`) + đường `10` cùng uốn lên (`[V5]` tr.15).
    - **Cơ hội theo sóng (Swing)**: Có Cơ hội T++ + các đường dài (`20`, `50`) cùng chụm lại ở vùng thấp và uốn lên (`[V5]` tr.15, `[Audio_B2_25062026]`).
  - **Vai trò trong hệ thống**: `MB` trả lời câu hỏi số 1 *"Cơ hội về giá đã hấp dẫn chưa?"* — đóng vai trò **ĐIỀU KIỆN CẦN** (phải xuất hiện trước hoặc đồng thời với dòng tiền `BB`) (`[V5]` tr.14-16, `[Audio_B2_25062026]`).

#### (2) `BB` / `BBA-M,S,EM` / `BBZ-M,S,EM` — Dòng tiền lan tỏa (Breadth Blackbox) (`[V5]` tr.14-19, `[Audio_B2_25062026]`)
- **Bản chất toán học**:
  - Khác với `DM` (cộng gộp tổng tiền coi cả thị trường như 1 cổ phiếu duy nhất), **`BB` đo ĐỘ LAN TỎA DIỆN RỘNG (Market/Sector Breadth)** — tỷ lệ % các cổ phiếu trên thị trường / trong ngành đang thực sự có dòng tiền chảy vào hay bị bán cổ phiếu ra đuổi theo cơ hội (`[V5]` tr.16, 19; `[Audio_B2_25062026]`).
  - **Giải mã ma trận hậu tố**:
    - `BBA` vs `BBZ`: `BBA` = số liệu tính từ kỳ gốc (khung toàn thời gian, dùng để kiểm tra tránh bẫy chuẩn hóa khi dòng tiền dao động ở vùng cao mà `BBZ` vẫn báo thấp); `BBZ` = chuẩn hóa `0–100` trong **50 phiên gần nhất** (`[V5]` tr.19, `[Audio_B2_25062026]`).
    - **`M`** (*Money*): Độ lan tỏa của **Dòng tiền vào** các cổ phiếu (`BBA-M`, `BBZ-M`).
    - **`S`** (*Stock/Shares*): Độ lan tỏa của **Cổ phiếu bán ra** trên diện rộng (`BBA-S`, `BBZ-S`).
    - **`EM`** (*Excess Money*): **Chênh lệch lan tỏa giữa Tiền vào và Cổ phiếu bán ra** (`M - S`) (`BBA-EM`, `BBZ-EM`) (`[V5]` tr.19).
- **Ý nghĩa vận hành & Quy tắc đồng thuận với `MB`**:
  - Trả lời câu hỏi số 2: *"Cơ hội đã rẻ rồi (`MB`), vậy dòng tiền có đuổi theo trên diện rộng không?"* — đóng vai trò **ĐIỀU KIỆN ĐỦ** (`[V5]` tr.16, `[Audio_B2_25062026]`).
  - Các đường cùng hướng lên = *"Tiền vào lan tỏa"*; cùng hướng xuống = *"Tiền ra lan rộng"* (`[V5]` tr.16).
  - **Tổ hợp xác nhận chắc chắn "Có cơ hội giao dịch T+"**: Cả `MBZ 03, 05` VÀ `BBZ-M 03, 05` đều chụm và tăng lên từ vùng `<20` (`MB` thường uốn lên trước 1 nhịp hoặc cùng ngày với `BB`, ví dụ thầy chỉ trên chart Buổi 2: ngày 09/06 `MBZ` uốn lên ở đáy, ngày 10/06 `BBZ-M` uốn lên theo $\rightarrow$ xác lập nhịp T+ thành công; ngược lại ngày 24-25/06 `MBZ` uốn lên nhưng `BBZ-M` không uốn đồng thuận $\rightarrow$ giá rẻ nhưng tiền chưa vào, đánh T+ rất khó ăn!) (`[V5]` tr.16, `[Audio_B2_25062026]`).
  - `BBZ-S` và `BBZ-EM` dùng làm chỉ báo bổ trợ để kiểm định lực tiền lan tỏa vào là mạnh hay yếu tương đối so với áp lực cung diện rộng (`[V5]` tr.16).

#### (3) `TM-Chuẩn`, `TS-Chuẩn`, `Tốc độ / SP (EM)` (`[V5]` tr.14-27, `[Audio_17072025]`, `[Audio_B1_21052026]`, `[Audio_B2_25062026]`)
- **Bản chất toán học**:
  - **`TM` (Total Money)** & **`TS` (Total Shares/Stock)**: Cộng dồn lũy kế hàng ngày tổng lượng tiền đưa vào hộp (`TM`) và tổng lượng cổ phiếu đưa vào hộp (`TS`) tính từ **ngày gốc**:
    - *Cập nhật mốc ngày gốc qua các phiên bản*: Trong `[V5]` (tr.18, 20) và `[Audio_17072025]`, ngày gốc đặt tại ngày thành lập QMV **20/03/2021**. Đến `[Audio_B2_25062026]`, giảng viên cho biết đã dời ngày gốc cộng dồn trên giao diện thị trường về **20/03/2024** vì để gốc 2021 đường tiền có độ dốc chúc xuống khó quan sát, dời sang 2024 thấy rõ xu hướng tiền và cổ phiếu tăng lên và dễ so sánh khoảng cách tương đối hơn.
  - **`TM-Chuẩn`** & **`TS-Chuẩn`**: Vì Tiền (VNĐ) và Cổ phiếu (khối lượng CP) là **hai đại lượng có đơn vị đo khác nhau**, QMV chuẩn hóa cả hai về cùng thang **`0–100`** (trên đồ thị Thị trường/Ngành — `[V5]` tr.18, 20) hoặc **`0–1`** (trên đồ thị Cổ phiếu — `[V5]` tr.38) để đặt chung lên một hệ trục (`[Audio_B2_25062026]`).
  - **`Tốc độ` / `SP` (Speed) / `EM` (Excess Money)**:
    - Công thức trực tiếp từ lời giảng (`[Audio_17072025]`):
      - Cấp tích lũy: **`Tốc độ (SP) = TM-Chuẩn − TS-Chuẩn`** (*"Chúng ta lấy tổng tiền mà chúng ta trừ đi cái tổng cổ phiếu thì chúng ta sẽ ra được cái tốc độ"*).
      - Cấp biến động hàng ngày (trong các bảng Top ngành / Top cổ phiếu): **`%SP = %DM − %DS`** (*"Phần trăm DM là daily money — phần trăm tiền vào so với ngày trước đó; phần trăm DS là daily stock/shares — lượng cổ phiếu đưa vào hộp hàng ngày; và phần trăm SP đơn giản là cái này trừ cái này"`*).
- **Ý nghĩa vận hành**:
  - Trả lời câu hỏi số 3 sau `MB` và `BB`: *"Tổng tiền thực tế bơm vào có NHIỀU không? Và bên Mua (Tiền) hay bên Bán (Cổ phiếu) đang cầm bóng chiếm ưu thế?"* (`[V5]` tr.14, 16; `[Audio_B2_25062026]`). Ví dụ: Có cơ hội T+ (`MB` + `BB` uốn lên) nhưng `TM` đi ngang nhỏ giọt hoặc cắm xuống $\rightarrow$ cơ hội yếu, phân hóa hẹp, không bền (`[V5]` tr.16, `[Audio_B2_25062026]`).
  - **Giải bài toán kinh điển của VSA**: Khi Giá tăng + Khối lượng tăng đột biến (hoặc Giá giảm + Lượng giảm), mắt thường không biết giá tăng nhanh hơn hay lượng xả ra nhanh hơn:
    - Nếu **`SP > 0` và lớn** (khoảng cách `TM-Chuẩn` nằm trên `TS-Chuẩn` mở rộng, ví dụ mã `VHG` `SP = 3` hay `NDN` có `SP` cao nhất ngành BĐS trong `[Audio_17072025]`): Lực tiền mua áp đảo lượng cổ phiếu chốt lời $\rightarrow$ xu hướng tăng khỏe, yên tâm nắm giữ!
    - Nếu tiền vẫn vào (`%DM > 0`, giá xanh) nhưng **`SP < 0` hoặc suy yếu** (như ví dụ `TVC` trong `[Audio_B2_25062026]` và một mã trà đá trong `[Audio_17072025]`): Lượng cổ phiếu tuồn vào hộp bán đang lớn hơn tốc độ tiền vào $\rightarrow$ cảnh báo áp lực xả ngầm, xác suất điều chỉnh cao!
  - **So sánh Ngành & Phân rã Nhóm Dòng tiền (`1, 2, 3, 0`) / Vốn hóa / MM / Trà đá**:
    - Chế độ **So sánh ngành** quy điểm xuất phát `= 0` tại đầu khung thời gian chọn (ví dụ 20 phiên = 1 tháng) cho mọi ngành và vẽ đường chuẩn Thị trường (màu vàng): ngành nằm trên đường vàng và dốc lên là ngành hút tiền vượt trội; theo backtest của thầy, một khi tiền vào ngành tạo xu hướng thì luôn kéo dài nhiều phiên chứ không chỉ 1 phiên (`[Audio_B2_25062026]`).
    - Trong từng ngành được chia tiếp thành **`Dòng tiền 1`** (đầu ngành/bluechip chạy đầu sóng như VHM, VIC), **`Dòng tiền 2`** (midcap đầu cơ thanh khoản cao nhiều người đu bám chạy nhịp 2 như CEO, DIG), **`Dòng tiền 3`** (nhỏ/penny chạy cuối sóng), và **`Dòng tiền 0`** (nhóm cổ phiếu bình thường gần như tắt thanh khoản bị lãng quên — `[Audio_17072025]`). Ngoài ra có tab riêng cho rổ **Cổ phiếu MM (có tạo lập)** và rổ **Cổ phiếu Trà đá** (`[V5]` tr.28-29).
    - Các bảng xếp hạng Top 5 ngành / Top 10 cổ phiếu chia 2 nửa **Trái vs Phải**: Bên trái xếp theo độ lớn `%DM`, `%DS` (giảm cung), `%SP`; bên phải xếp theo **Nâng bậc / Giảm bậc** (số bậc nhảy hạng đột biến). *Quy tắc nhìn*: Thị trường đang lên $\rightarrow$ nhìn bên trái (tiền vào nhiều); thị trường đang giảm/chỉnh $\rightarrow$ nhìn bên phải (nâng bậc đột biến / giảm cung mạnh để đón đầu); mã nào xuất hiện ở cả hai bên là cực khỏe (`[Audio_17072025]`, `[V5]` tr.17, 21).

#### (4) `Sentiment Gauge` — Biểu đồ Tâm lý thị trường trên Dashboard (`[V5]` tr.2, 34-35; `[Audio_17072025]`; `[Audio_B2_25062026]`; `[V7_ref]` Hình 1.1)
- **Bản chất toán học**:
  - Đặt tại trang **Dashboard Thị Trường** (`[V5]` tr.2). Đo **tương quan dòng tiền chảy vào nhóm cổ phiếu Rủi ro so với nhóm cổ phiếu An toàn** (khi tiền vào cổ phiếu rủi ro nhiều hơn cổ phiếu an toàn $\rightarrow$ đồ thị Tâm lý thị trường tăng; ngược lại tiền rút về cổ phiếu an toàn/phòng thủ hoặc rút khỏi nhóm rủi ro $\rightarrow$ đồ thị giảm — `[V7_ref]` tr.3).
  - Loại trừ nhiễu: Các mã thanh khoản quá nhỏ / trà đá khớp vài nghìn CP bị loại ra khỏi rổ tính tâm lý thị trường chung (`[Audio_B2_25062026]`).
- **Ý nghĩa vận hành**:
  - Dao động theo chu kỳ đủ dài (`> T+`) chứ không nhiễu từng ngày, dùng để đóng khung trạng thái tâm lý toàn thị trường đang ở pha **Tiêu cực / Bình thường / Tích cực**.
  - Là biến điều tiết ngoại sinh trực tiếp cho công cụ **Định giá so sánh tâm lý** của từng cổ phiếu (vì hệ thống không tự động gán nhãn tâm lý thị trường thay người dùng — `[V5]` tr.34-35):
    - Khi Tâm lý thị trường chuyển từ **Tiêu cực $\rightarrow$ Tích cực**: Giá dưới mức Bình thường là *Thấp giá (Mua)*; trên mức Bình thường đến Tích cực là *Hợp lý (Giữ/Gia tăng)*; vượt trên mức Tích cực mới là *Cao giá (Bán)*.
    - Khi Tâm lý thị trường chuyển từ **Tích cực $\rightarrow$ Tiêu cực**: Giá trên mức Bình thường đã bị coi là *Cao giá (Bán)*; phải xuống dưới mức Bình thường và dưới mức Tiêu cực mới là *Hợp lý / Thấp giá*!

#### (5) `Dòng tiền chủ động: Cáo - Chó săn/Sói - Thỏ` & Nút `Mua lên / Bán xuống` (`[V5]` tr.3, 31, 36; `[Audio_17072025]`; `[Audio_B1_21052026]`; `[Audio_B2_25062026]`)
- **Giải mã 3 đối tượng**: Trong `[V5]` (tr.31, 36) gọi là **Cáo - Sói - Thỏ**; trên giao diện thực chiến trong `[Audio_17072025]` và `[Audio_B2_25062026]` (và `[V7_ref]`) hiển thị là **Cáo - Chó săn - Thỏ**:
  - **Cáo**: Các lệnh chủ động quy mô **Lớn** (tay to / tạo lập).
  - **Chó săn / Sói**: Các lệnh chủ động quy mô **Vừa phải**.
  - **Thỏ**: Các lệnh chủ động quy mô **Nhỏ** (nhà đầu tư cá nhân nhỏ lẻ).
- **Bản chất toán học ĐỘC QUYỀN (Khác hoàn toàn "khớp lệnh chủ động Buy/Sell" trên bảng giá thông thường!)**:
  - Giảng viên giải thích tường minh trong `[Audio_17072025]` và `[Audio_B2_25062026]`:
    > *"Mua chủ động được thiết kế ở QMV theo nghĩa là nếu chúng tôi nhìn thấy cái giá nó khớp và **nó đẩy lên một bước [bước giá / tick]** thì đó mới được gọi là mua chủ động. Và khối lượng mua chủ động này **KHÔNG KHỚP** với khối lượng giao dịch bình thường hay khối lượng khớp lệnh bình thường, mà chỉ tập trung vào hành vi là liệu có hành vi **đẩy giá lên** hay hành vi **đẩy giá xuống** hay không!"*
  - Như vậy:
    - **Mua chủ động QMV (`Mua đẩy giá lên`)**: Chỉ cộng các lệnh khớp làm giá tăng lên ít nhất 1 bước giá ($\Delta P_{\text{tick}} > 0$).
    - **Bán chủ động QMV (`Bán đẩy giá xuống`)**: Chỉ cộng các lệnh khớp làm giá giảm xuống ít nhất 1 bước giá ($\Delta P_{\text{tick}} < 0$). Các lệnh khớp ngang bước giá ($\Delta P_{\text{tick}} = 0$) không tính là hành vi đẩy/đè giá dứt khoát!
- **Ý nghĩa vận hành**:
  - Biểu đồ gồm các **cột mua/bán ròng hàng ngày** của Cáo, Chó săn, Thỏ + **đường cộng dồn lũy kế tổng ròng cả 3 nhóm** theo thời gian (`[Audio_B2_25062026]`).
  - Nút lọc **`Mua lên`** / **`Bán xuống`** trên Monitor/Screener kích hoạt khi tổng mua bán chủ động ròng của cả 3 nhóm nghiêng hẳn về dương (nỗ lực đẩy giá lên dứt khoát) hoặc âm (nỗ lực đè giá xuống) (`[Audio_B1_21052026]`, `[V5]` tr.3).
  - Nhận diện gom/đánh lên: Một cổ phiếu suốt 3 tuần cột Cáo-Chó săn-Thỏ im lìm, tự nhiên những phiên gần đây sôi động vọt lên ở chiều mua (ví dụ `MST` trong `[Audio_17072025]`) $\rightarrow$ dấu hiệu chắc chắn có ý chí đẩy giá. Ngược lại nếu cả Cáo và Thỏ đều lèo tèo (ví dụ `NVL` trong `[Audio_B2_25062026]`) $\rightarrow$ giá biến động hẹp, chưa thể bứt phá ngay.

---

### 1.2. CẤP CỔ PHIẾU

#### (1) `Price Trend` (`Uptrend/Tăng`, `Downtrend/Giảm`, `Sideways/Đi ngang`) + `New High / New Low` (`[V5]` tr.3, 35; `[Audio_B1_21052026]`; `[Audio_B2_25062026]`)
- **Giải mã cơ chế toán học qua 2 phiên bản (chi tiết cực đắt giá!)**:
  - Trong **`[V5]` (tr.3, dòng 95-110)**: Ghi rõ quá trình chuyển đổi thuật toán:
    > *"Phiên bản đầu tiên sử dụng ước lượng đánh giá uptrend, downtrend, sideways. **Phiên bản hiện tại dựa trên nguyên tắc đường trung bình** do không có sự khác biệt lớn về kết quả cuối cùng giữa hai cách làm."*
    Khung tham chiếu chuẩn của `Price Trend` là **quý gần nhất (~50–60 phiên)**: `Uptrend` = Giá có xu hướng tăng trong quý gần nhất; `Downtrend` = Giá có xu hướng giảm trong quý gần nhất; `Sideways` = Giá có xu hướng dao động trong quý gần nhất.
  - Trong **`[Audio_B1_21052026]` & `[Audio_B2_25062026]`**: Khi giảng ý nghĩa kinh tế cho học viên, thầy diễn giải theo định nghĩa cấu trúc đỉnh-đáy gốc (bản 1): `Uptrend` = *đáy sau cao hơn đáy trước, đỉnh sau cao hơn đỉnh trước*; `Downtrend` = *đáy sau thấp hơn đáy trước, đỉnh sau thấp hơn đỉnh trước*; không thỏa mãn cả hai là `Sideways` (đi ngang).
  - **Tổng hợp bản chất**: Bản chất lý thuyết là cấu trúc đỉnh-đáy trong cửa sổ 1 quý (50–60 phiên), còn thuật toán chạy tự động hiện hành trên hệ thống dùng **hệ thống đường trung bình động của quý gần nhất** (quan hệ vị trí & độ dốc MA ngắn hạn so với MA20/MA50) làm xấp xỉ mượt hóa tương đương!
  - Đi kèm **`New Low / Đáy mới`** và **`New High / Đỉnh mới`** (hiển thị kèm con số kỳ hạn như `Đỉnh 10`, `Hỗ trợ 20`, `Đáy 50 / Đỉnh 50`, `Kháng cự 60`): Cho biết giá hôm nay đang tiệm cận vùng hỗ trợ/kháng cự hoặc lập đáy/đỉnh mới trong vòng 10, 20, 50 hay 60 phiên (`[V5]` tr.3, 35; `[Audio_17072025]`; `[Audio_B1_21052026]`; `[Audio_B2_25062026]`).

#### (2) `TMC` (kèm `TMA20`, `TMA50`) — Đường Xu hướng Tổng tiền Cổ phiếu (`[V5]` tr.37; `[Audio_B1_21052026]`; `[Audio_B2_25062026]`)
- **Bản chất toán học**:
  - **`TMC`** (đường màu đen trên đồ thị đầu tiên của Blackbox cổ phiếu): Tổng lượng tiền thực tế lũy kế cộng dồn hàng ngày đưa vào hộp của cổ phiếu đó tính từ thời điểm gốc (`[V5]` tr.37, `[Audio_B2_25062026]`).
  - **`TMA20`** (đường màu vàng) & **`TMA50`** (đường màu xanh): Là **đường trung bình động giản đơn 20 phiên (`MA20`) và 50 phiên (`MA50`) tính trực tiếp trên chuỗi `TMC`** (*"Tôi bổ sung cái đường TM20 và TM50 ở đây bản chất là tôi tính trung bình của cái đường này [TMC] thôi, để nhìn xu hướng theo Quý cho dễ"* — `[Audio_B2_25062026]`, `[V5]` tr.37).
- **Ý nghĩa vận hành ("Kim chỉ nam / Đích đến")**:
  - `TMC` có tính chu kỳ rất rõ (độ dài các nhịp cắt lên/cắt xuống đường vàng `TMA20` thường tương đối đều nhau theo thói quen của nhóm cổ đông đặc trưng ở cổ phiếu đó — `[Audio_B2_25062026]`).
  - **Quy tắc tối thượng**: `TMC` là kim chỉ nam chi phối mọi đồ thị bên dưới. Nếu `TMC` đang dốc xuống nằm dưới `TMA20 < TMA50` (tiền đang rút ra, ví dụ `NVL`, `D2D`, `DCM` trong `[Audio_B2_25062026]`), thì mọi tín hiệu tích cực của Cầu, Tâm lý hay Phân kỳ dương bên dưới **chỉ như hòn đá lăn từ đỉnh núi gặp mô đất dừng nghỉ tạm thời**, cơ hội rất yếu/ngắn, không mua mới vội! Ngược lại nếu `TMA50` (đường xanh) vẫn hướng lên (ví dụ `VIC` ngày 25/06/2026), nhịp `TMC` uốn xuống chỉ là nhịp nghỉ trong xu hướng tiền vào lớn $\rightarrow$ bán chốt T+ xong phải canh vòng lại mua ngay!

#### (3) `BB Status` (`Tiền khỏe / Bão hòa / Tiền yếu / Duy trì`) & `BB Predict` (`[V5]` tr.4, 7-8, 26, 35; `[Audio_B1_21052026]`; `[Audio_B2_25062026]`)
- **Bản chất toán học**:
  - `[V5]` (tr.26) và `[Audio_B1_21052026]`, `[Audio_B2_25062026]` xác nhận trực tiếp: **`BB Status` là trạng thái dự báo xu hướng vòng tiền lớn được ước lượng trực tiếp từ vị trí và sự giao cắt của bộ 3 đường `TMC` (đen), `TMA20` (vàng), `TMA50` (xanh)!**
  - Chu trình khép kín 4 pha của Vòng Tiền Lớn: **`Duy trì` (1) $\rightarrow$ `Tiền khỏe` (2) $\rightarrow$ `Bão hòa` (3) $\rightarrow$ `Tiền yếu` (4) $\rightarrow$ `Duy trì` (1)**:
    1. **`Tiền duy trì`**: Sau chuỗi giảm, tiền đã ít nhưng **không ít thêm / ra chậm lại và có xu hướng quay lại** (`TMC` đi ngang tạo nền đáy, bắt đầu cắt lên `TMA20`, hai đường vàng `TMA20` và xanh `TMA50` thu hẹp khoảng cách) $\rightarrow$ ủng hộ giá dừng giảm; nếu có tín hiệu mua thì phân tích theo hướng **Mua lần đầu**. *(Lưu ý `[V5]` tr.8: `Duy trì` được ưu tiên hơn `Bão hòa` vì hàm ý đảo chiều tiền vào!)*
    2. **`Tiền khỏe`**: Tiền đang đi vào mạnh (`TMC > TMA20 > TMA50`, dốc lên) $\rightarrow$ ủng hộ tăng giá; có tín hiệu mua thì phân tích theo hướng **Mua gia tăng**.
    3. **`Tiền bão hòa`**: Tiền đã vào nhiều nhưng **không nhiều thêm / bắt đầu chững hoặc bán ra sớm** (`TMC` uốn ngang/cắt xuống `TMA20` trên nền cao) $\rightarrow$ ủng hộ giá dừng tăng; có tín hiệu bán thì phân tích theo hướng **Bán lần đầu**.
    4. **`Tiền yếu`**: Tiền đang rút ra (`TMC < TMA20 < TMA50`, dốc xuống) $\rightarrow$ ủng hộ giảm giá; có tín hiệu bán thì phân tích theo hướng **Bán gia tăng**.
- **`BB Predict` (Dự báo chuyển pha của `BB Status`)**:
  - **Không phải tín hiệu Mua/Bán**, chỉ phát sinh đúng vào những ngày các đường `TMC`, `TMA20`, `TMA50` giao cắt hoặc hội tụ chuyển pha (`[Audio_B1_21052026]`, `[Audio_B2_25062026]`, `[V5]` tr.4, 35):
    - **Chiều vào**:
      - `Tiền dấu hiệu vào`: `TMC` bắt đầu uốn và cắt lên `TMA20` từ vùng yếu $\rightarrow$ tiền *có thể* chuyển từ `Tiền yếu` sang `Duy trì` (chỉ báo đúng 1 ngày cắt, ví dụ `EVF` ngày 18/05 trong `[Audio_B1_21052026]`).
      - `Tiền đang vào`: `TMC` giữ trên `TMA20` và kéo `TMA20` tiến sát lại `TMA50` $\rightarrow$ tiền *đang* vào pha `Duy trì`.
      - `Tiền xu hướng vào`: Các đường cắt xác nhận xu hướng tăng $\rightarrow$ tiền *đang chuyển từ `Duy trì` sang `Tiền khỏe`* (chắc chắn nhất nhưng trễ giống Ichimoku vì xu hướng đã chạy trước đó vài phiên — `[Audio_B1_21052026]`).
    - **Chiều ra**:
      - `Tiền dấu hiệu ra` (*`Tiền khỏe` $\rightarrow$ có thể sang `Bão hòa`*), `Tiền đang ra` (*đường `TMC` cắt xuống vào pha `Bão hòa`, như điểm cắt trên đồ thị `NVL` trong `[Audio_B2_25062026]`*), `Tiền xu hướng ra` (*chuyển từ `Bão hòa` sang `Tiền yếu`*). *Quy tắc khắc cốt ghi tâm của thầy trong `[Audio_B1_21052026]`: Trong thị trường khó khăn, cổ phiếu đang giữ mà chớm báo `Dấu hiệu ra` là phải thoát ngay lập tức!*

#### (4) `DSPI` (`-1..+1` quanh `0`, kèm `MA5`, `MA20`) & `XH Cầu` (Chu kỳ cung cầu 20 phiên xoay quanh `BB Status`) (`[V5]` tr.4, 8, 26, 35, 37; `[Audio_B1_21052026]`; `[Audio_B2_25062026]`; `[V7_ref]` Hình 5.12)
- **Bản chất toán học**:
  - **`DSPI`** (*Demand Supply Pressure Index*): Đo lường **sức ép Cầu tương đối so với Cung (mức độ tiền vào tương đối so với cổ phiếu bán ra vào hộp / tương quan Tổng tiền – Tiền vào theo chu kỳ)** trong khung thời gian ngắn hạn **1 tháng = 20 phiên gần nhất**, được chuẩn hóa vào khoảng **`[-1, +1]`** với **`0` là trục cân bằng trung tính** (`[V5]` tr.8, 26, 37; `[Audio_B2_25062026]`; `[V7_ref]` tr.31).
  - Kèm 2 đường trung bình động **`MA5`** và **`MA20`** tính trên `DSPI` để xác định xu hướng và điểm uốn (`[V7_ref]` tr.31).
  - Giải thích trực tiếp của thầy (`[Audio_B2_25062026]`):
    - `DSPI > 0`: Cầu thắng thế so với cung $\rightarrow$ **hàm ý có tiền mới vào cổ phiếu**.
    - `DSPI < 0`: Sức ép cung lớn hơn cầu $\rightarrow$ **không có tiền mới, tiền đang rút ra tương đối**.
- **Mối liên hệ với `XH Cầu` (`Chu kỳ cung cầu`) & `Dự báo Cầu`**:
  - `XH Cầu` chính là 4 pha của đường hình sin `DSPI` trong chu kỳ 20 phiên: **`Cầu duy trì` (1)** (*`DSPI` ở vùng âm thấp `<0` ngừng giảm và uốn lên*) $\rightarrow$ **`Cầu khỏe` (2)** (*`DSPI` hướng lên mạnh mẽ vượt `0`*) $\rightarrow$ **`Cầu bão hòa` (3)** (*`DSPI` ở vùng dương cao gần `+1` đạt đỉnh và bắt đầu uốn xuống*) $\rightarrow$ **`Cầu yếu` (4)** (*`DSPI` lao dốc xuống âm*). (Đi kèm `Dự báo Cầu`: `Dấu hiệu vào / Đang vào / Xu hướng vào / Ra` tương tự `BB Predict` — `[Audio_B1_21052026]`, `[V7_ref]` tr.7).
  - **Cơ chế phối hợp "Vòng quay nhỏ xoay quanh Xu hướng lớn" (`[V5]` tr.4, 8; `[Audio_B1_21052026]`)**:
    - `BB Status` (`TMC`) là đường cong lớn định hướng (đoàn tàu / đích đến); `XH Cầu` (`DSPI`) là đường hình sin màu cam (20 phiên) xoay quanh đường lớn (các nhà ga / bậc thang).
    - **Tổ hợp Trading hoàn hảo nhất**: `Price Trend = Tăng` + `BB Status = Tiền khỏe (hoặc Duy trì)` + `XH Cầu` chuyển từ `Cầu duy trì` sang `Cầu khỏe` (`DSPI` uốn lên từ vùng âm/thấp = đoàn tàu vào ga đón khách $\rightarrow$ điểm mua chuẩn, kỳ vọng sóng tăng kéo dài). Nếu `BB Status = Tiền khỏe` mà `XH Cầu = Bão hòa / Yếu` $\rightarrow$ chỉ là chỉnh ngắn hạn để mua lại.
    - Ngược lại, nếu `BB Status = Bão hòa / Tiền yếu` + `Price Trend = Đi ngang / Giảm`, dù `XH Cầu` chuyển sang `Cầu khỏe` thì nhịp hồi cũng rất ngắn/nhanh hoặc chỉ đơn giản là tạm chững chưa rơi ngay (`[V5]` tr.7-8, `[Audio_B2_25062026]`).

#### (5) `MPICP` (`0..100` quanh `50`, kèm `MA5`, `MA20`) — Sức ép tâm lý thị trường cấp Cổ phiếu (`[Audio_B1_21052026]`; `[Audio_B2_25062026]`; `[V7_ref]` Hình 5.12)
- **Định danh & Thang đo**:
  - Tên trên giao diện là **Sức ép thị trường / Sức ép tâm lý** của cổ phiếu (`MPICP` = *Market Pressure Index cho Cổ Phiếu*, đặt tên "Sức ép thị trường" để không nhầm với biểu đồ "Tâm lý thị trường" ngoài Dashboard — `[V7_ref]` tr.31).
  - Dao động trong thang **`0..100`** với **`50` là mức trung bình**, kèm 2 đường trung bình **`MA5`** và **`MA20`** để xác định xu hướng (`[V7_ref]` tr.31).
- **Bản chất toán học ĐỘC ĐÁO (Tiết lộ nguyên văn của thầy trong `[Audio_B2_25062026]`)**:
  > *"Đối với sức ép tâm lý thì tôi lại đo lường nó hơi khác một chút: nếu như cung và cầu [`DSPI`] tôi dựa chủ yếu vào việc tiền vào so với cổ phiếu vào hộp để tìm ra sức ép, thì cái **Sức ép thị trường bản chất nó là đồ thị tâm lý: tôi đo lường các LƯỢNG TIỀN NHỎ so với LƯỢNG TIỀN LỚN đi vào!** Là bởi vì tư duy của tôi là nếu có **nhiều nhỏ lẻ vào** có nghĩa rằng tâm lý nó tích cực; còn nếu **nhiều nhỏ lẻ mà ra** thì tâm lý nó tiêu cực, là bởi vì những ông nhỏ lẻ là những ông dễ phản ánh tâm lý nhất!"*
- **Ý nghĩa vận hành**:
  - Vận hành như một dao động hình sin song hành cùng `DSPI` quanh trục xu hướng `TMC`:
    - Khi `MPICP` lùi sâu về vùng cực thấp (`<< 50`, nhỏ lẻ hoảng loạn/chán nản bán sạch) rồi bắt đầu nhích lên $\rightarrow$ nhỏ lẻ nhạy bén bắt đầu quan tâm mua lại, kết hợp `DSPI` ở vùng thấp tạo **điều kiện cần** báo hiệu cổ phiếu đã nén cạn kiệt, có thể dừng giảm (`[Audio_B1_21052026]`, `[Audio_B2_25062026]`).
    - Khi `MPICP` vọt lên vùng cực cao (`>> 50` sát đỉnh 80–100, nhỏ lẻ hưng phấn FOMO mua mạnh): **Cứ lên thái cực trên là ưu tiên ra hàng cho chắc!** Ví dụ kinh điển thầy phân tích mã `VIC` ngày 25/06/2026 (`[Audio_B2_25062026]`): dù `TMA50` còn tăng và `DSPI` mới ở lưng chừng `>0`, nhưng `MPICP` đã chạm vùng đỉnh cao nhất + `DMZ` chụm cao uốn xuống $\rightarrow$ chốt lời T+ ra trước, đợi `MPICP` hạ về vùng thấp rồi vào lại!
  - Ngoài ra, `MPICP` còn là 1 trong 3 chỉ báo lõi (cùng `RSI` và `CCI`) được QMV dùng để tính tín hiệu **Phân kỳ dương / âm** (`[Audio_B1_21052026]`).

#### (6) `DM03/05/10/20/50/200` & `DS03/05/10/20/50/200` (Bản gốc Mặc định vs Bản Chuẩn hóa 50 phiên + `EM0x / SP0x`) (`[V5]` tr.14-18, 27, 34, 38-39; `[Audio_17072025]`; `[Audio_B1_21052026]`; `[Audio_B2_25062026]`)
- **Bản chất toán học**:
  - Cắt lát lượng tiền đưa vào hộp (`DM`) và lượng cổ phiếu đưa vào hộp (`DS`) theo 6 chu kỳ nắm giữ: **`03` (T+3 / T+2.5), `05` (1 tuần), `10` (2 tuần), `20` (1 tháng), `50` (1 quý), `200` (1 năm)**.
  - **Tại sao chu kỳ `03` là QUAN TRỌNG NHẤT & CHÍNH XÁC NHẤT?**: Thầy nhấn mạnh trong cả `[Audio_B1_21052026]` và `[Audio_B2_25062026]`: Các chu kỳ `05, 10, 20, 50, 200` chỉ là mô phỏng hình tượng (giả định có người giữ 1 tuần, 1 tháng...), nhưng **`03` mô phỏng sát thực 100% quy chế thanh toán T+2.5 của thị trường Việt Nam** — bất kỳ ai mua cổ phiếu chắc chắn phải đến T+2.5 hàng về mới có quyền quyết định bán hay giữ tiếp! Nhìn `EM03 / SP03` (tương quan tiền `DM03` trừ cổ phiếu `DS03`) sẽ biết chính xác mỗi lần hàng T+ về, đám đông trên thị trường đang bán tháo ra (`EM03 < 0`) hay hấp thụ bơm tiền giữ tiếp (`EM03 > 0`).
- **So sánh Bản Gốc (Mặc định) vs Bản Chuẩn hóa 50 phiên (`Z`) & CẢNH BÁO BẪY CHUẨN HÓA**:
  1. **Bản gốc (Mặc định - chưa chuẩn hóa, xoay quanh trục `0`)**:
     - Giữ nguyên biên độ thực tế trên toàn thời gian quan sát (với đường `200` là tính lăn 200 phiên).
     - `DM0x > 0` = thực sự **có tiền mới bơm vào hộp**; `DM0x < 0` = tiền đang rút ra (`[V5]` tr.18, 38; `[Audio_17072025]`). Đường dài (màu tím `50`/`200`) dùng để định hướng dòng tiền lớn (`[Audio_17072025]`).
     - `DS0x > 0` = cổ phiếu bán vào hộp nhiều hơn bình thường (`[V5]` tr.39).
  2. **Bản Chuẩn hóa (`Z` - thang `0..100` trong 50 phiên gần nhất)**:
     - Kéo giãn giá trị `DM0x`, `DS0x` trong cửa sổ **50 phiên gần nhất** về khoảng `0–100` (`[V5]` tr.14, 18, 38-39; `[Audio_B2_25062026]`).
     - Sinh ra tín hiệu **`Uốn lên 20 / 30`** và **`Uốn xuống 80 / 70`** trên bảng BB Tích cực/Tiêu cực & Bộ lọc:
       - Mặc định cột lọc `Uốn lên 20/30` trên bảng thống kê thiết kế cho **T+ (`DMZ03` và `DMZ05` cùng về `<20` hoặc `<30` rồi uốn lên)** (`[Audio_B2_25062026]`).
       - Trên bảng chi tiết cổ phiếu có gắn nhãn phạm vi vòng tiền: **`T+`** (chỉ `03` và `05` chụm uốn), **`T++`** (`03, 05, 10` chụm uốn), **`SW` (Swing)** (nhiều đường từ `03` đến `20, 50` cùng chụm lại ở `<20 / <30` và uốn lên $\rightarrow$ độ tin cậy đảo chiều cao nhất!) (`[V5]` tr.34, `[Audio_17072025]`).
       - Với `DS` chuẩn hóa: Từ `70, 80` uốn xuống = đỉnh điểm bán tháo đã qua; từ `20, 30` uốn lên = bắt đầu xả cổ phiếu ra (`[V5]` tr.39).
       - Khuyến nghị hiển thị: Khi bật Chuẩn hóa để đánh giá trading, **tốt nhất chỉ bật 4 chu kỳ `03, 05, 10, 20`** (`[V5]` tr.38-39), vì đường `200` khi nhét vào cửa sổ chuẩn hóa 50 phiên sẽ bị méo thị giác (như học viên hỏi trong `[Audio_B2_25062026]`: `DM200` mặc định đang ở trên cao trượt nhẹ xuống nhưng chuẩn hóa 50 phiên lại biến thành `<20`!).
     - **BẪY CHUẨN HÓA BẮT BUỘC PHẢI TRÁNH (`[Audio_B2_25062026]`)**:
       - *Bẫy 1 (Biên độ chết)*: Khi cổ phiếu/thị trường bị bóp nghẹt thanh khoản, `DM` mặc định chỉ lăn tăn li ti sát trục `0` suốt 50 phiên, phép chuẩn hóa 50 phiên VẪN phóng đại biên độ li ti đó thành `0` và `100`! Nếu thấy chuẩn hóa chạm `0` uốn lên mà nhảy vào mua nhưng bản mặc định đang phẳng lì $\rightarrow$ giá chỉ nhích từ mép dưới lên mép trên biên hẹp (1-2%), không có ăn!
       - *Bẫy 2 (Neo đỉnh)*: Nếu `DM` mặc định đang dao động tít trên vùng dương cao suốt 50 phiên, đáy của vùng cao đó vẫn hiển thị `<20` trên đồ thị chuẩn hóa $\rightarrow$ mua vào là dính nguy cơ sập mạnh!
       - **Quy tắc bắt buộc**: Luôn kiểm tra chéo **cả Mặc định lẫn Chuẩn hóa 50 phiên**!

#### (7) `Bullish Signal 1, 2, 3, 4` & `Bearish Signal 1, 2, 3` (kèm `Bullish / Bearish Pattern`) (`[V5]` tr.6-7, 34; `[Audio_17072025]`; `[Audio_B1_21052026]`; `[Audio_B2_25062026]`)
- **Phân biệt `Pattern` vs `Signal`**:
  - **`Bullish / Bearish Pattern`** (tên cũ: `Buy / Sell Pattern`): Cảnh báo sớm nhận diện hành động giá / đảo chiều khi thỏa mãn một hoặc nhiều hệ thống kỹ thuật phổ biến (Cụm 1–3 nến, Bộ 5 chỉ báo kỹ thuật, Ichimoku QMV, mẫu hình AI). Trên màn hình hội viên hiện chữ `Tích cực` / `Tiêu cực` (màn hình dev của thầy hiện mã số rule để theo dõi hiệu quả từng rule) (`[Audio_17072025]`, `[Audio_B1_21052026]`, `[V7_ref]` tr.34).
  - **`Bullish / Bearish Signal`** (tên cũ: `Buy / Sell Rule`): Cấp độ hành động cao nhất ("trái tim của Screener"), kết hợp `Pattern` với **các thuật toán độc quyền QMV về Dòng tiền Blackbox, Cung cầu, Mua bán chủ động và tỷ lệ Lợi nhuận kỳ vọng / Rủi ro kỳ vọng** (`[Audio_17072025]`, `[Audio_B1_21052026]`, `[V5]` tr.34).
- **Ý nghĩa từng cấp số `Bullish Signal 1, 2, 3, 4`**:
  - **`1` = Mua đáy**: Giá ở vùng đáy thấp / tại hoặc nhúng dưới hỗ trợ sâu rồi vòng lên (ví dụ `DPG` báo số 1 trong `[Audio_B1_21052026]`). Cần kiên nhẫn chờ giá lên; rất hợp mua đầu tư/gom khi định giá rẻ + `BB Status = Duy trì / Tiền khỏe`.
  - **`2` = Mua hỗ trợ**: Giá đã tạo vùng dao động nền nhỏ và bật lên vượt vùng hỗ trợ/dao động thấp $\rightarrow$ an toàn hơn số 1 nhưng biên lợi nhuận kỳ vọng nhỏ hơn.
  - **`3` = Mua đuổi (Momentum)**: Mua thuận đà khi giá đã lên vùng cao gần kháng cự.
  - **`4` = Mua phá đỉnh (Breakout)**: Mua khi giá bứt phá vượt đỉnh/kháng cự (rủi ro cao nhất nếu gặp thị trường yếu; chỉ dùng khi thị trường chung tốt + `BB Status = Tiền khỏe / Bão hòa` + đi kèm `Cầu mạnh` + `New High`) (`[V5]` tr.6-7, `[Audio_B1_21052026]`).
- **Ý nghĩa từng cấp số `Bearish Signal 1, 2, 3` & Lý do KHÔNG có số 4**:
  - **`1` = Bán đỉnh** (trên/tại đỉnh cao rơi xuống); **`2` = Bán kháng cự** (chạm kháng cự / rơi khỏi nền dao động trên cao); **`3` = Bán đuổi (Momentum)** (bán tiếp theo đà giảm — phải soi kỹ hỗ trợ mạnh bên dưới kẻo bán đúng đáy!).
  - **Tại sao không có `Bearish Signal 4` (Bán thủng hỗ trợ / Cắt lỗ tự động)?**: Giảng viên giải thích rõ trong `[Audio_B1_21052026]`, `[Audio_B2_25062026]` và `[V5]` (tr.6): QMV chủ đích **không thiết kế tín hiệu số 4 chiều bán** để tránh kích hoạt tâm lý hoảng loạn bán tháo máy móc ngay khi giá vừa chớm thủng hỗ trợ (rất dễ dính bẫy rũ hàng / Spring, và hệ thống đã dùng `MinSell` để hãm đà bán đuổi).
  - *Đánh giá theo bối cảnh*: Bán tin cậy nhất khi `BB Status = Bão hòa / Tiền yếu` + `Price Trend = Đi ngang / Giảm`. Nếu `BB Status = Duy trì / Tiền khỏe` + `Price Trend = Tăng`, thì `Bearish Signal` chỉ dùng để **Rebalancing (bán chốt một phần rồi canh mua lại)** (`[V5]` tr.7).

#### (8) `MaxBuy`, `Target 1`, `Target 2`, `StopLoss`, `MinSell` (`[V5]` tr.7, 34; `[Audio_17072025]`; `[Audio_B2_25062026]`)
- Là các mốc giá ước lượng **chỉ tính tại ngày phát sinh Signal (T0)**; hành động thực tế ở các ngày T+ bắt buộc phải cập nhật theo bối cảnh mới:
  - **`MaxBuy`** (khi có `Bullish Signal`): Mức giá tối đa gợi ý mua không vượt quá tại thời điểm quan sát, được tính tương đối dựa trên **các chỉ số trung bình và biên độ giá ngày hôm đó** (nên trong phiên khi giá High giật mạnh hoặc qua các ngày có tín hiệu mới thì `MaxBuy` sẽ dịch chuyển theo — `[Audio_B2_25062026]`). Nếu giá chạy vượt `MaxBuy`, không mua đuổi mà đợi chỉnh về quanh `MaxBuy`.
  - **`Target 1` & `Target 2`**: Các mốc chốt lời kỳ vọng thứ nhất và thứ hai, ước lượng từ **các vùng kháng cự gần nhất** kết hợp **đo chiều dài các bước sóng tăng trước đó (tịnh tiến mũi tên bước 1 = bước 3 / Fibonacci mở rộng 1.618)** (`[Audio_17072025]`, `[Audio_B2_25062026]`). Nếu bối cảnh xấu đi phải chủ động chốt sớm không chờ chạm Target (`[V5]` tr.7).
  - **`StopLoss`**: Đặt tại **vùng hỗ trợ khỏe gần nhất** tính ở ngày T0 (`[Audio_B2_25062026]`). **Cảnh báo sống còn (`[V5]` tr.7)**: Khi hàng về T+, chính mức `StopLoss` đó rất có thể lại là vùng hỗ trợ đỡ giá bật lên! Do đó **CHỈ cắt lỗ tại `StopLoss` khi bối cảnh chung xấu VÀ giá phá thủng bằng một phiên giảm mạnh (cây nến đỏ dài / `Cầu yếu`)**.
  - **`MinSell`** (khi có `Bearish Signal`): Mức giá gợi ý **bán không thấp hơn** tại ngày ra tín hiệu — hàm ý nếu hôm đó bán thì chỉ bán một phần từ vùng giá đó trở lên, không bán tháo bằng mọi giá. Tuy nhiên, nếu sang các phiên sau bối cảnh xấu thật sự và giá không hồi mà thủng sâu hơn, thì việc bán tiếp ở `MinSell` hoặc lùi mốc bán về hỗ trợ kế tiếp là điều bắt buộc phải làm (`[V5]` tr.7, `[Audio_B2_25062026]`).

#### (9) `Mức độ phù hợp giao dịch Trading (0 - 5 sao)` + `Tốt hơn / Không đổi / Kém hơn` — TẠI SAO `0-2 sao + Tốt hơn` LẠI NGON HƠN `5 sao` Ở CUỐI SÓNG? (`[V5]` tr.33; `[Audio_B1_21052026]`; `[Audio_B2_25062026]`)
- **Bản chất toán học**:
  - Đánh giá tổng hợp **độ đẹp/hoàn thiện của cấu trúc form tăng giá hiện tại** theo các khung thời gian chuẩn (Ngày, Tuần, Tháng, Quý) dựa trên sự hội tụ của 3 trụ cột: **`Price Trend` (Giá) + `BB Status` (Tiền) + `XH Cầu / DSPI & MPICP` (Cung cầu & Tâm lý)** (`[V5]` tr.33, `[Audio_B1_21052026]`, `[Audio_B2_25062026]`).
  - Thang điểm: `0 sao` (không có sao nào = form đang yếu, chưa đủ điều kiện xếp hạng tăng — `[Audio_17072025]`, `[Audio_B2_25062026]`), `1–2 sao` (form còn yếu / mới chớm chuyển biến), **`3 sao`** (ngưỡng cân bằng chuẩn dùng cho nút lọc nhanh `Danh mục Giao dịch` — báo hiệu quá trình tạo đáy có thể đã hoàn thành, form tương đối an toàn), `4–5 sao` (cả giá, tiền, cầu, tâm lý đều đã đồng thuận tăng đẹp).
  - Đi kèm biến đạo hàm bậc 1 theo ngày: **`Tốt hơn` / `Không đổi` / `Kém hơn`** — đo sự thay đổi của điểm cấu trúc form giá **hôm nay so với ngày hôm qua** (`[Audio_B1_21052026]`, `[Audio_B2_25062026]`).
- **Giải mã nghịch lý thực chiến: Tại sao `0–2 sao + Tốt hơn` lại ngon hơn `5 sao` ở cuối sóng / thị trường khó?**:
  - Giảng viên phân tích cực kỳ sâu sắc trong cả `[Audio_B1_21052026]` (ví dụ lọc `1 sao + Tốt hơn` bắt được `VJC` vừa nổ `Bullish Signal`) và `[Audio_B2_25062026]` (ví dụ `D2D` đang `0 sao + Tốt hơn + Phân kỳ dương + Uốn lên 20`):
    1. Số sao chỉ chụp ảnh **hiện trạng đã diễn ra**: để đạt **`4–5 sao`**, cổ phiếu bắt buộc phải tăng một đoạn khá dài để kéo cả Giá, Tiền, Cầu và Tâm lý cùng lên cao.
    2. Trong thị trường uptrend lớn nhiều tiền, 3–5 sao vẫn chạy tiếp tốt. **NHƯNG trong thị trường Sideways, khó khăn hoặc ở cuối một nhịp sóng ngắn**, nguyên lý Contrarian của QMV chỉ rõ: *"Cái gì quá đẹp, quá chắc ăn (`5 sao`, đặc biệt khi `Không đổi` hoặc `Kém hơn`) thì rất có thể là SỰ KẾT THÚC của sóng tăng chứ không phải sự bắt đầu!"* Đợi đủ 5 sao mới mua là mua đúng ngọn sóng cuối, biên lợi nhuận còn rất mỏng mà rủi ro chỉnh sát sườn!
    3. Ngược lại, một cổ phiếu đang **`0 sao`, `1 sao` hoặc `2 sao` nhưng xuất hiện chữ `Tốt hơn`** (hôm nay cấu trúc giá bắt đầu cải thiện so với hôm qua) + đi kèm **`Bullish Signal` / `Phân kỳ dương` / `Uốn lên 20`** chính là **điểm khởi phát sớm nhất từ vùng xấu kiệt cùng sang chu kỳ mới** $\rightarrow$ dư địa ăn dày hơn hẳn (dù xác suất thất bại cao hơn 3 sao nên phải đi vốn từng phần)! Điểm cân bằng hài hòa nhất giữa độ an toàn và lợi nhuận trong thị trường phân hóa là **`2–3 sao + Tốt hơn`** (`[Audio_B1_21052026]`, `[Audio_B2_25062026]`).

#### (10) `Vùng tắc nghẽn` (đi kèm `Biến động hơn trước`, `Biến động hơn thị trường`, `Thanh khoản nhỏ`) (`[V5]` tr.5, 35; `[Audio_B1_21052026]`; `[Audio_B2_25062026]`)
- **Bản chất toán học**:
  - **`Vùng tắc nghẽn`**: Thuật toán nhận diện giá cổ phiếu bị **nén chặt trong một biên độ dao động rất hẹp suốt một khoảng thời gian tương đối dài**, không có xu hướng (ví dụ `VPB` chỉ đi ngang trong biên 26–28 suốt thời gian dài — `[Audio_B1_21052026]`, `[Audio_B2_25062026]`).
  - **`Biến động hơn thị trường`**: Cổ phiếu có độ lệch chuẩn (standard deviation) / hệ số `Beta` cao hơn thị trường (`Beta > 1`) — dùng để chọn mã bốc hơn VN-Index khi thị trường vào sóng tăng (`[Audio_B1_21052026]`, `[Audio_B2_25062026]`).
  - **`Biến động hơn trước`**: So sánh độ biến động ngắn hạn hiện tại (giá & thanh khoản, phản ánh qua `Beta 30 ngày` và biên độ swing gần đây) với lịch sử tĩnh lặng trước đó của **chính cổ phiếu đó** (`[Audio_B1_21052026]`, `[Audio_B2_25062026]`).
- **Ý nghĩa vận hành**:
  - `Vùng tắc nghẽn` báo hiệu lò xo bị nén chuẩn bị bung mạnh: nếu tắc nghẽn ở vùng đáy thấp = **nghi vấn đang gom hàng tích lũy**; nếu tắc nghẽn ở vùng đỉnh cao = **nghi vấn đang phân phối** (`[Audio_B2_25062026]`).
  - Kết hợp `Thanh khoản nhỏ` + tiền sử `Vùng tắc nghẽn` + nay xuất hiện `Biến động hơn trước` là bộ chỉ báo số 1 để trả lời câu hỏi *"Cổ phiếu nào đang được gom xong và chuẩn bị đánh lên?"*, đặc biệt hiệu quả cho nhóm **Trà đá (`< 10.000đ`)** trong vĩ mô tiền nhiều (`[V5]` tr.5, `[Audio_B1_21052026]`).

#### (11) `Kéo xả` vs `Đạp kéo` (cùng nhóm `Cầu mạnh/yếu`, `Uốn HT/KC`, `Cầu HT / Cung KC`) (`[V5]` tr.4, 35; `[Audio_B1_21052026]`; `[Audio_B2_25062026]`)
- **Bản chất toán học (Nhận diện hành vi VSA / Wyckoff ngay trong phiên hôm nay)**:
  - **`Kéo xả`**: Trong phiên giá được kéo thốc lên cao rồi bị xả ép đóng cửa tụt xuống thấp (râu trên dài) — thầy khẳng định trực tiếp trong `[Audio_B1_21052026]` và `[Audio_B2_25062026]`: **mô phỏng chính xác nến `Upthrust` (UT)** trong Wyckoff!
  - **`Đạp kéo`**: Trong phiên giá bị đạp sâu xuống dưới rồi lực cầu kéo rút chân đóng cửa lên cao (ví dụ `DPM` trong `[Audio_B1_21052026]`) — thầy khẳng định trực tiếp trong `[Audio_B2_25062026]`: **mô phỏng chính xác nến `Spring` (rút chân)**!
  - **`Cầu mạnh` / `Cầu yếu` (trong ngày)**: Tổ hợp cấu trúc thân nến đơn (ví dụ nến xanh dài `Marubozu` = `Cầu mạnh`; nến đỏ đặc dài = `Cầu yếu`) kết hợp dòng tiền/cầu của riêng ngày hôm đó (`[Audio_B1_21052026]`).
  - **`Uốn hỗ trợ / Uốn kháng cự` vs `Cầu mạnh tại hỗ trợ / Cung mạnh tại kháng cự`**:
    - `Uốn HT / KC`: Giá chạm vùng hỗ trợ/kháng cự và tạo điểm uốn đảo chiều về mặt hình học giá.
    - `Cầu HT / Cung KC`: Kiểm tra xem tại điểm uốn hỗ trợ/kháng cự đó **khối lượng giao dịch / lực mua bán thực tế có bung ra lớn hay không**! Ví dụ thực chiến trên mã `VIC` (`[Audio_B2_25062026]`): `VIC` tích đỏ `Uốn kháng cự` nhưng ô `Cung KC` để trống vì khối lượng tại đỉnh đó nhỏ $\rightarrow$ giá chững lại chỉ do tâm lý e ngại vùng cao chứ không phải tay to xả hàng mạnh!

---

## PHẦN 2: TỔNG HỢP MỌI GỢI Ý CƠ CHẾ TÍNH TỪ GIÁ & KHỐI LƯỢNG $\rightarrow$ THIẾT KẾ `OHLCV PROXY` TRUNG THÀNH NHẤT VỚI QMV

Dựa trên toàn bộ các tiết lộ của thầy Hào xuyên suốt 4 tài liệu gốc, dưới đây là bản giải phẫu cơ chế ngầm và thiết kế công thức xấp xỉ **`OHLCV Proxy`** bám sát 100% triết lý QMV:

### 2.1. Các Manh Mối Toán Học Trực Tiếp Từ Lời Giảng Của Thầy
1. **Về `DM` (Tiền vào hộp) & `DS` (Cổ phiếu bán vào hộp)**:
   - Không phải `Volume` thô, cũng không gán nhị phân "phiên xanh = 100% tiền vào, phiên đỏ = 100% cổ phiếu ra" (`[Audio_17072025]`).
   - Khi giá tăng + khối lượng tăng: **Cả `DM` và `DS` đều tăng** (vì theo lý thuyết cung cầu QMV trong `[Audio_B1_21052026]` & `[Audio_B2_25062026]`, khối lượng khớp lệnh nhìn thấy trên bảng điện chính là lượng cổ phiếu người bán chấp nhận tung ra, còn việc giá vẫn đóng cửa tăng chứng tỏ lực tiền mua lớn hơn lượng cổ phiếu bán đó!).
   - Hai biến dùng **hai đơn vị gốc khác nhau**: `DM` mang bản chất **Giá trị Tiền (Price $\times$ Volume có trọng số cầu)**, còn `DS` mang bản chất **Lượng Cổ phiếu (Volume có trọng số cung)**, và thuật toán **ưu tiên Cầu hơn Cung** (`[V5]` tr.14, `[Audio_B2_25062026]`).
   - Khi giá tăng, lượng tăng nhưng giá tăng vượt trội so với mức tăng của lượng $\rightarrow$ `SP = TM_chuẩn - TS_chuẩn > 0` (hoặc `%SP = %DM - %DS > 0`); ngược lại khi lượng nổ quá lớn mà biên độ tăng giá hẹp / bị ép râu trên $\rightarrow$ `DS` vượt trội `DM`, làm `SP < 0` (`[Audio_17072025]`).
2. **Về Cắt Lát Chu Kỳ & Chuẩn Hóa**:
   - Lũy kế từ ngày gốc $t_0$ (`20/03/2024` hoặc đầu chuỗi): `TMC(t)` và `TSC(t)` là tổng cộng dồn hàng ngày.
   - `TMA20 = SMA(TMC, 20)` và `TMA50 = SMA(TMC, 50)` (`[Audio_B2_25062026]`).
   - Chu kỳ $k \in \{3, 5, 10, 20, 50, 200\}$ cắt lát dòng tiền/cổ phiếu lăn trong cửa sổ $k$ phiên; bản Chuẩn hóa `Z` dùng **Min-Max Scaling 50 phiên gần nhất** đưa về `[0, 100]` (`[V5]` tr.14, 18; `[Audio_B2_25062026]`).
3. **Về `DSPI` vs `MPICP` vs `Dòng tiền chủ động`**:
   - `DSPI` $\in [-1, +1]$ (cửa sổ 20 phiên, kèm `MA5, MA20`): Đo sức ép ròng `Tiền vào vs Cổ phiếu bán ra` (`DSPI > 0` khi có tiền mới ròng vào vượt cung).
   - `MPICP` $\in [0, 100]$ quanh `50` (kèm `MA5, MA20`): Đo **tỷ trọng dòng tiền nhỏ lẻ (Thỏ) mua/bán so với dòng tiền lớn (Cáo)** (`[Audio_B2_25062026]`).
   - `Dòng tiền chủ động (Cáo - Chó săn - Thỏ)`: Trên dữ liệu khớp lệnh tick, chỉ tính các lệnh làm dịch chuyển bước giá ($\Delta P_{\text{tick}} \neq 0$).
4. **Về Bộ Chỉ Báo Kỹ Thuật Đi Kèm**:
   - **Ichimoku QMV**: Tham số **`(6, 18, 36)`** (hoặc viết đủ 4 số `(18, 6, 18, 36)` / `3-6-18-36` — nhanh gấp ~1.5 lần chuẩn `9-26-52` để khớp nhịp tháng/quý Việt Nam — `[Audio_B1_21052026]`, `[Audio_B2_25062026]`, `[V7_ref]` tr.7, 29).
   - **Khối lượng đột biến**: $V_t \ge 1.25 \times \text{SMA}(V, 5)$ (vượt ~25% trung bình 5 phiên — `[Audio_B1_21052026]`).
   - **Bộ 5 chỉ báo (`Chỉ số đẹp/xấu`)**: `RSI`, `CCI`, `MACD`, `Chaikin Money Flow (CMF)`, và `Stochastic/Ultimate Oscillator` đồng loạt uốn lên từ vùng thấp (`<30`) hoặc uốn xuống từ vùng cao (`>70`) (`[Audio_B1_21052026]`, `[Audio_B2_25062026]`).
   - **Phân kỳ dương / âm (thường & ẩn)**: Đo bất đồng thuận đỉnh/đáy giữa Giá và bộ 3 **`MPICP` + `RSI` + `CCI`** (`[Audio_B1_21052026]`).

### 2.2. Đề Xuất Công Thức Xấp Xỉ `OHLCV Proxy` Trung Thành Nhất Với Triết Lý Gốc QMV
Khi chỉ có dữ liệu `OHLCV` hàng ngày $(O_t, H_t, L_t, C_t, V_t)$ (không có sổ lệnh tick-by-tick), ta có thể tái tạo đầy đủ hệ biến QMV với độ tương thích cơ học rất cao như sau:

1. **Phân tách trọng số Cầu ($w^D_t$) và Cung ($w^S_t$) trong từng cây nến (VSA Decomposition)**:
   - Xét biên độ thực của nến (True Range $TR_t = \max(H_t, C_{t-1}) - \min(L_t, C_{t-1}) + \epsilon$) và vị trí đóng cửa thực:
     $$\text{CLV}_t = \frac{(C_t - \min(L_t, C_{t-1})) - (\max(H_t, C_{t-1}) - C_t)}{TR_t} \in [-1, +1]$$
   - Tỷ lệ biến thiên giá chuẩn hóa theo biến động trung bình 20 phiên ($\sigma_{\Delta P, 20}$): $r^P_t = \frac{C_t - C_{t-1}}{C_{t-1}}$.
   - **Lượng tiền ròng đưa vào hộp ngày $t$ (`dM_t`)** (Ưu tiên Cầu qua hệ số bất đối xứng $\alpha > 1$, ví dụ $\alpha \approx 1.15$, nhân với Giá trị giao dịch $Val_t = P_{\text{typ}, t} \cdot V_t$):
     $$dM_t = Val_t \cdot \left( \alpha \cdot \text{sign\_smooth}(r^P_t, \text{CLV}_t) \right)$$
     Trong đó thành phần cầu dương khi giá tăng đóng cửa cao, âm khi giá giảm đóng cửa thấp; hoặc tách tường minh **Tiền bơm vào** $M^{\text{in}}_t = Val_t \cdot \left(\frac{1 + \text{CLV}_t}{2}\right)^{\gamma_D} \cdot (1 + k_P r^P_t)$ so với mức duy trì nền $\overline{Val}_{20}$ để có biến thiên ròng `dM_t`.
   - **Lượng cổ phiếu bán ròng đưa vào hộp ngày $t$ (`dS_t`)** (Đơn vị Khối lượng $V_t$ so với nền thanh khoản $\overline{V}_{20}$, tăng mạnh khi khối lượng phình to nhưng giá bị nén / râu trên dài):
     $$dS_t = V_t \cdot \left(\frac{1 - \text{CLV}_t}{2} + \beta \cdot \mathbb{I}(V_t > \overline{V}_{20})\frac{H_t - C_t}{TR_t}\right) - c_S \overline{V}_{20}$$
     *(Đảm bảo đúng tính chất thầy giảng: Khi giá tăng + vol tăng khỏe với nến đóng cao, cả $M^{\text{in}}_t$ và lượng cổ phiếu khớp đều tăng nhưng `%DM > %DS` nên `%SP > 0`; khi vol nổ đột biến tại kháng cự mà râu trên dài $H_t - C_t$ lớn, $dS_t$ vọt nhanh hơn $dM_t$ làm `SP < 0`!).*

2. **Tái tạo `TMC`, `TMA20`, `TMA50`, `TM-Chuẩn`, `TS-Chuẩn`, `SP`, `DM0x`, `DS0x`**:
   - $\text{TMC}_t = \sum_{\tau=t_0}^t dM_\tau$, $\quad \text{TSC}_t = \sum_{\tau=t_0}^t dS_\tau$ (với $t_0 = \text{2024-03-20}$).
   - $\text{TMA20}_t = \text{SMA}(\text{TMC}, 20)_t$, $\quad \text{TMA50}_t = \text{SMA}(\text{TMC}, 50)_t$.
   - $\text{TM\_Chuẩn}_t = \frac{\text{TMC}_t - \min_{t_0..t}(\text{TMC})}{\max_{t_0..t}(\text{TMC}) - \min_{t_0..t}(\text{TMC})}$, tương tự cho $\text{TS\_Chuẩn}_t$, và $\text{SP}_t = \text{TM\_Chuẩn}_t - \text{TS\_Chuẩn}_t$.
   - Cắt lát chu kỳ $k \in \{3, 5, 10, 20, 50, 200\}$:
     $$\text{DM}_k(t) = \text{TMC}_t - \text{TMC}_{t-k} \quad (\text{hoặc } \text{TMC}_t - \text{SMA}(\text{TMC}, k)_t), \qquad \text{DS}_k(t) = \text{TSC}_t - \text{TSC}_{t-k}$$
   - Chuẩn hóa 50 phiên (`Z`):
     $$\text{DMZ}_k(t) = 100 \times \frac{\text{DM}_k(t) - \min_{\tau \in [t-49, t]} \text{DM}_k(\tau)}{\max_{\tau \in [t-49, t]} \text{DM}_k(\tau) - \min_{\tau \in [t-49, t]} \text{DM}_k(\tau) + \epsilon}$$
   - Tín hiệu `Đảo chiều tăng T+`: $dM_t > 0$ VÀ $dM_{t-1} < 0$ VÀ $dM_{t-2} < 0$ (`[Audio_B2_25062026]`, `[V7_ref]` Hình 1.7).

3. **Tái tạo `DSPI [-1, +1]` & `MPICP [0, 100]`**:
   - **`DSPI_t`**: Chuẩn hóa sức ép ròng Cầu trừ Cung trong cửa sổ 20 phiên về $[-1, +1]$ (dùng hàm $\tanh$ hoặc chuẩn hóa theo cực trị tuyệt đối 20 phiên để giữ đúng điểm `0` là trung tính `dM = 0`):
     $$X^{\text{DSP}}_t = \text{EMA}_{3}\left(\frac{\text{DM}_{20}(t)}{\overline{Val}_{20}} - \frac{\text{DS}_{20}(t)}{\overline{V}_{20}}\right), \qquad \text{DSPI}_t = \text{clip}\left(\frac{X^{\text{DSP}}_t}{\max_{\tau \in [t-19, t]} |X^{\text{DSP}}_\tau| + \epsilon}, -1, +1\right)$$
     kèm $\text{MA5}(\text{DSPI})$ và $\text{MA20}(\text{DSPI})$.
   - **`MPICP_t` (Proxy hành vi Nhỏ lẻ vs Tay to từ OHLCV)**:
     - Nhỏ lẻ thường mua đuổi ở các phiên biến động giá xanh thanh khoản vừa/nhỏ hoặc FOMO khi giá kéo xa nền ngắn hạn, và bán tháo khi giá rơi đỏ; trong khi tay to thể hiện ở các phiên khối lượng lớn gấp nhiều lần trung bình. Có thể xấp xỉ dòng tiền nhỏ lẻ tương đối bằng sự kết hợp giữa xung lượng giá ngắn hạn (Stochastic/RSI 14) và dòng tiền ở các phiên không có tổ chức đỡ lớn ($V_t \le 1.25 \overline{V}_{20}$), chuẩn hóa mượt về $[0, 100]$ quanh trục `50`, kèm $\text{MA5}(\text{MPICP})$ và $\text{MA20}(\text{MPICP})$.

4. **Tái tạo `Price Trend`, `Kéo xả / Đạp kéo`, `Vùng tắc nghẽn`, `Cáo - Chó săn - Thỏ` Proxy**:
   - **`Price Trend` (Quý = 50 phiên)**: Kết hợp độ dốc $\text{SMA}(C, 20)$, $\text{SMA}(C, 50)$ và vị trí $C_t$: `Uptrend` khi $C_t > \text{SMA}_{20} > \text{SMA}_{50}$ và $\Delta \text{SMA}_{50} > 0$ (hoặc đỉnh/đáy swing 50 phiên tăng dần); `Downtrend` khi ngược lại; còn lại là `Sideways`.
   - **`Kéo xả` (Upthrust)**: $H_t > \max(H_{t-1..t-5})$ (hoặc biên trên lớn) NHƯNG $\frac{H_t - C_t}{H_t - L_t} > 0.6$ và $C_t \le O_t$ (hoặc đóng cửa nửa dưới nến kèm vol $\ge \overline{V}_{10}$).
   - **`Đạp kéo` (Spring)**: $L_t < \min(L_{t-1..t-5})$ (nhúng sâu) NHƯNG $\frac{C_t - L_t}{H_t - L_t} > 0.6$ (kéo đóng cửa cao nửa trên nến).
   - **`Vùng tắc nghẽn`**: Độ rộng kênh giá 20 phiên $\frac{\max_{20}(H) - \min_{20}(L)}{\text{SMA}_{20}(C)} < \theta_{\text{tight}}$ (ví dụ $< 8\%–10\%$) VÀ $\text{ATR}_{14}/\text{ATR}_{50} < 0.8$.
   - **`Mua lên / Bán xuống` (OHLCV Proxy)**: Chỉ kích hoạt khi thân nến dứt khoát $\frac{|C_t - O_t|}{H_t - L_t} > 0.5$, $\Delta C_t \neq 0$, và phân tầng quy mô qua tỷ số thanh khoản $V_t / \overline{V}_{20}$ (`> 1.5` = Cáo tham gia; `0.8–1.5` = Chó săn; `< 0.8` = Thỏ).

---

## PHẦN 3: CÁC CÔNG THỨC LỌC CỔ PHIẾU (SCREENER / MONITOR RECIPES) THỰC CHIẾN TRÊN MÀN HÌNH

### 3.1. Nguyên Tắc Bàn Thờ Trước Khi Bấm Lọc (`[V5]`, `[Audio_17072025]`, `[Audio_B1_21052026]`)
1. **Xóa bộ lọc mặc định ngay khi mở trang**: QMV Monitor mở lên luôn tích sẵn mặc định `Bullish Pattern` + `Khối lượng đột biến` $\rightarrow$ bắt buộc bấm **"Xóa điều kiện lọc"** trước khi thao tác (`[Audio_B1_21052026]`).
2. **Toán tử lọc là giao `AND` (Loại trừ)**: Tích càng nhiều ô cùng lúc thì tập kết quả càng bị siết chặt dễ bỏ sót cơ hội $\rightarrow$ chỉ chọn 2–4 tiêu chí cốt lõi kết hợp **1 nhóm Tình trạng (Xu hướng)** + **1 nhóm Tín hiệu (Hôm nay)** (`[V5]` tr.7-8).
3. **Khung tư duy Top-Down 3 bước trước khi lọc mã (`[Audio_17072025]`, `[Audio_B1_21052026]`, `[Audio_B2_25062026]`)**:
   - **Bước 1 (Xu thế vĩ mô & Định giá VN-Index)**: Nếu VN-Index nằm dưới giá trị hợp lý (ví dụ `< 1266` trong buổi `17/07/2025`) hoặc cuối chu kỳ thắt chặt thị trường giảm sâu $\rightarrow$ **Investing (ôm cả thị trường / lọc `Giá trị + Thắt chặt`)**. Nếu VN-Index đã vượt lên trên giá trị hợp lý (`> 1266`) hoặc thị trường đang tăng mạnh $\rightarrow$ **CẤM lọc đầu tư Giá trị**, chuyển 100% sang **Trading theo Luân chuyển Ngành & Key Driver** (Tín dụng cuối quý, KQKD đột biến, hoặc Khối ngoại nâng hạng)!
   - **Bước 2 (Bộ cửu chương luân chuyển ngành bất biến của thầy — `[Audio_17072025]`)**:
     - Bộ 1 (Trụ cột): `Ngân hàng` $\leftrightarrow$ `Chứng khoán` $\rightarrow$ `Bất động sản` $\rightarrow$ kéo theo `Xây dựng & Vật liệu xây dựng` $\rightarrow$ `Dịch vụ công nghiệp`.
     - Bộ 2 (Hàng hóa): `Sản phẩm & Dịch vụ công nghiệp` nhúc nhích $\rightarrow$ nghĩ ngay tới `Tài nguyên cơ bản` $\rightarrow$ `Hóa chất` động đậy $\rightarrow$ nghĩ ngay tới `Dầu khí`.
     - Bộ 3 (Tiêu dùng): `Thực phẩm đồ uống` $\leftrightarrow$ `Bán lẻ` $\leftrightarrow$ `Hàng tiêu dùng / Ô tô / Du lịch giải trí`.
     - Bộ 4 (Công nghệ): `Công nghệ` $\leftrightarrow$ `Truyền thông` $\leftrightarrow$ `Viễn thông`.
     - Luân chuyển nội ngành: Tiền vào Ngành $\rightarrow$ vào `Dòng tiền 1` (Bluechip dẫn dắt) $\rightarrow$ lan sang `Dòng tiền 2` (Midcap nhiều người đu bám: CEO, DIG...) $\rightarrow$ nổ mạnh cuối cùng ở `Dòng tiền 3` & `Trà đá`!
   - **Bước 3 (Liên hoàn 3 dòng Định giá theo giai đoạn — `[Audio_B2_25062026]`)**:
     - Thị trường giảm sâu $\rightarrow$ lọc **`Vùng giá đầu tư` (`Định giá dòng tiền`)** để gom tích lũy chờ sóng.
     - Tiền vừa xác nhận vào một ngành 2–3 phiên $\rightarrow$ lọc **`Thấp so với ngành` (`Định giá so sánh ngành` PE/PB)** để nhặt mã rẻ trong ngành đó.
     - Đã mua cổ phiếu vào form chạy $\rightarrow$ soi **`Định giá so sánh tâm lý`** (mốc Bình thường / Tích cực) để biết giữ đến giá nào thì chốt!

---

### 3.2. Sáu Công Thức Lọc Thực Chiến (Screener / Monitor Recipes) Giảng Viên Thao Tác Trực Tiếp

#### Công thức 1: Lọc Mua Đáy / Bắt Đảo Chiều Sớm Khi Thị Trường Khó (`[Audio_B1_21052026]`, `[Audio_B2_25062026]`, `[V5]` tr.7)
- **Tổ hợp A — Đón lõng trước khi uốn (Tránh mua muộn)**:
  - `Tình trạng Blackbox`: **`Dưới 30`** (các đường chu kỳ tiền ngắn đã rơi về vùng duy trì `<30`, chuẩn bị uốn).
  - `Hệ thống giao dịch`: **`Phân kỳ dương`** (trên `MPICP/RSI/CCI` — tín hiệu thầy đánh giá tỷ lệ thắng cao nhất giai đoạn thị trường giảm để tìm cổ phiếu đã "cắt cơn giảm", sóng hồi sẽ dài hơn `Phân kỳ dương ẩn`).
  - `Mức độ phù hợp giao dịch`: **`0 sao / 1 sao / 2 sao`** + **`Tốt hơn`** (bắt ngay chân sóng khi cấu trúc giá vừa chuyển biến cải thiện so với hôm qua).
- **Tổ hợp B — Xác nhận điểm uốn tại đáy**:
  - `Tín hiệu hôm nay`: **`Bullish Signal = 1`** (hoặc tự nới lỏng thành: `Bullish Pattern` + `Uốn lên 20 / 30` + `Đạp kéo` [Spring] / `Uốn hỗ trợ` + `Cầu mạnh tại hỗ trợ`).
- **Bộ lọc an toàn khi thực thi (Quy tắc 3 lần dừng giảm & Đáy 2 — `[Audio_B2_25062026]`)**:
  - Mở đồ thị `TMC` (Blackbox cổ phiếu) và đồ thị Giá: Nếu `TMC` vẫn đang cắm đầu rơi thẳng đứng (như `D2D`, `DCM`) và mới là **lần dừng giảm đầu tiên (Đáy 1)** $\rightarrow$ **Người mua mới TUYỆT ĐỐI đứng ngoài** (chỉ người kẹp sẵn hàng mới được mua lướt Rebalancing)! Chỉ giải ngân mua mới khi cổ phiếu đã có **2–3 lần nỗ lực dừng giảm**, `TMC` chuyển sang đi ngang dừng rút tiền, và giá test lại tạo **Đáy số 2 (hoặc Đáy số 4)** kèm nến rút chân (như `NVL`).

#### Công thức 2: Lọc Mua Tích Lũy / Săn Cổ Phiếu "Tay To Đang Gom" & Săn "Trà Đá" (`[V5]` tr.5, `[Audio_17072025]`, `[Audio_B1_21052026]`)
- **Bối cảnh kích hoạt**: Môi trường tiền nhiều / tín dụng bơm mạnh, nhóm lớn (VN30, Ngân hàng, Chứng khoán) đã chạy lên cao.
- **Thao tác 2 bước (Tránh lỗi tích chung `Tắc nghẽn` + `Biến động hơn trước` trong cùng 1 ngày!)**:
  1. **Bước 1 (Lập Watchlist nền nén)**: Chọn khoảng thời gian nhiều phiên trước đó, lọc **`Vùng tắc nghẽn`** + **`Thanh khoản nhỏ`** (hoặc `Thị giá: Trà đá < 10` / nhóm `Dòng tiền 0, 3`).
  2. **Bước 2 (Bắt tín hiệu nổ súng đánh lên)**: Hôm nay lọc lại trên nhóm đó (hoặc soi bảng `Top 10 KLGD đột biến`) tìm mã bắt đầu báo **`Biến động hơn trước`** + kiểm tra biểu đồ **`Cáo - Chó săn - Thỏ`** thấy cụm cột mua đẩy giá sôi động hẳn lên sau nhiều tuần ngủ yên + `%SP > 0` (như case study `MST`, `VHG`, `NDN` trong `[Audio_17072025]`).
  3. **Bước 3 (Khám sức khỏe chống hủy niêm yết cho Trà đá)**: Kiểm tra `Doanh nghiệp tốt` (`>= 3 sao` hoặc `Nỗ lực KQKD quý` cải thiện từ 3 lên 4 sao là quá đẹp; nếu `0 sao` thì nhìn BCTC xem có lỗ ròng 2 năm liên tiếp không và dùng prompt AI tiếng Việt hỏi xem mã đó có cảnh báo hủy niêm yết từ UBCK không — `[Audio_17072025]`). Đi vốn tối đa **1/3 danh mục** cho nhánh rủi ro/trà đá.

#### Công thức 3: Lọc Mua Tại Hỗ Trợ vs Mua Phá Đỉnh (Breakout) Trên Tab Kỹ Thuật (`[Audio_17072025]`, `[Audio_B1_21052026]`)
- **Nhánh Mua Hỗ Trợ (An toàn)**:
  - Tích **`Tiếp cận hỗ trợ`** (ví dụ hỗ trợ 20 phiên: ra 56 mã) $\rightarrow$ lọc hẹp tiếp bằng **`Phân kỳ dương`** HOẶC **`Khối lượng đột biến`**.
  - Mở chart soi **Bối cảnh lớn 5 bước sóng (`0-1-2-3-4-5`)** và **Quy tắc 3 điểm chạm** trên khung ngày / 30 phút (chỉ mua hoặc gia tăng ở **điểm chạm thứ 2 hoặc thứ 3**, không hành động ở điểm chạm đầu tiên!).
- **Nhánh Mua Phá Lên (Break-up / Breakout khi thị trường khỏe)**:
  - Tích **`Phá lên (Break-up)`** + **`Khối lượng đột biến`** + Mẫu hình dao động (ví dụ **`Tam giác hướng lên`**) hoặc kết hợp trên Monitor: **`Cầu mạnh` + `New High` + `Bullish Signal = 4`**.
  - Điều kiện bắt buộc: Bối cảnh lớn của cổ phiếu đang chuẩn bị bước vào **Bước sóng 3** (hoặc đầu Bước 5 với trà đá), `BB Status` là `Tiền khỏe`.

#### Công thức 4: Chiến Thuật Lọc `Tiền Vào 2 - 3 Phiên Liên Tiếp` (`[Audio_17072025]`, `[Audio_B1_21052026]`, `[Audio_B2_25062026]`)
- **Khi thị trường Uptrend / Tiền nhiều (`[Audio_17072025]`)**:
  - Mở bảng **`Tiền vào 2 phiên liên tiếp`** và **`Tiền vào 3 phiên liên tiếp`**.
  - **Mỏ vàng săn gom ngầm**: Tìm những mã **tiền vào hộp liên tiếp 2–3 phiên MÀ GIÁ CHƯA TĂNG** $\rightarrow$ bằng chứng tay to đang âm thầm gom hàng chưa đánh lộ diện!
  - Nếu muốn đánh lướt nhanh ngay trong phiên/ngắn hạn: Tích kết hợp **`Tiền vào hôm nay` + `Mua lên`** (vừa có tiền vào hộp vừa có lệnh Cáo/Chó săn đẩy bước giá lên dứt khoát) + lọc `Thanh khoản tốt` (`[Audio_B1_21052026]`).
- **Khi thị trường Sideways / Xoay vòng T+ nhanh (`[Audio_B2_25062026]`) — CẢNH BÁO ĐU ĐỈNH PHIÊN 4**:
  - Thầy dặn cực kỹ: Trong thị trường hẹp chỉ đánh T+, mã nào xuất hiện ở bảng **`Tiền vào 3 phiên liên tiếp`** chắc chắn là mã đáng chú ý (bình thường không ai ngó, nay tiền vào 3 phiên liền là bắt đầu có đội đánh theo quy tắc 3 nhịp).
  - **NHƯNG TUYỆT ĐỐI KHÔNG NHẢY VÀO MUA ĐUỔI NGAY Ở PHIÊN THỨ 3!** Vì đám đông đánh T+ nên đến phiên thứ 4 hàng về rất dễ bị chốt chỉnh ngay (*"nhảy vào mua ngay là tòi đấy!"*). Cách đánh chuẩn: **Đưa mã `Tiền vào 3 phiên` vào Watchlist, ngồi rình nhịp chỉnh đầu tiên kết thúc để múc nhịp đánh số 2!**

#### Công thức 5: Công Thức Đánh `T+03` Chuẩn Xác Nhất (`[Audio_17072025]`, `[Audio_B1_21052026]`, `[Audio_B2_25062026]`)
1. **Kiểm tra điều kiện thị trường cho phép đánh T+**:
   - Xem Blackbox Toàn thị trường: Biên độ dao động của `DM03` (bản mặc định) có đủ rộng không (nếu co hẹp sát 0 là T+ không đủ biên lợi nhuận!). Kiểm tra `MBZ 03, 05` và `BBZ-M 03, 05` có uốn lên từ `<20 / <30` không, và `EM03` có dương không (`[Audio_B2_25062026]`).
2. **Tổ hợp tiêu chí bắt đúng chân vòng T+ mới của cổ phiếu**:
   - **`Đảo chiều tăng hôm nay` + `Đảo chiều tăng T+`**: Tín hiệu cốt lõi nhất! Chứng minh rằng sau nhịp giảm đủ chu kỳ T+2.5 (hoặc sau cây nổ vol 2.5 ngày trước), lượng hàng T+ về tài khoản **được hấp thụ hết, người cầm hàng KHÔNG bán tháo mà dòng tiền đã đảo chiều bơm lại mở vòng T+ mới** (như case study `TIG` trong `[Audio_17072025]` và ví dụ thực hành trong `[Audio_B2_25062026]`).
   - **`Uốn lên 20` hoặc `Uốn lên 30` (nhãn `T+` hoặc `SW`)**: `DMZ03` và `DMZ05` cùng chụm dưới 20/30 uốn lên (nhớ liếc sang bản Mặc định xác nhận biên độ dao động thực của tiền không bị phẳng lì 1%!).
   - **`Tốc độ chu kỳ 03 (`SP03 / EM03`) > 0`**: Đảm bảo trong chu kỳ T+3, cầu thực sự khỏe hơn cung cổ phiếu bán ra (`[Audio_B1_21052026]`).
   - Cộng hưởng thêm **`Bullish Signal (1 hoặc 2)`** + `DSPI` & `MPICP` đang từ vùng thấp đi lên $\rightarrow$ xác suất thắng nhịp T+ cao nhất!
   - **Khi chốt lời T+**: Thấy `DMZ 03, 05` chụm trên `>70 / >80` uốn xuống + `MPICP` chạm đỉnh cao + `Bearish Signal` (như case study `VIC` ngày 25/06/2026) hoặc báo `Đảo chiều giảm T+` $\rightarrow$ bán ra ngay (nếu `TMA50` còn tăng thì đợi 1–2 phiên `DMZ03` rơi nhanh từ 70 về `<30` lại vòng vào mua tiếp!).

---

### 3.3. Kỹ Thuật Tra Cứu Lịch Sử Tín Hiệu T+ & Tránh Bẫy Tín Hiệu Trong Phiên vs Cuối Ngày

1. **Thủ thuật kiểm tra toàn bộ lịch sử tín hiệu quá khứ của 1 mã hoặc 1 ngành (`[Audio_17072025]`, `[Audio_B1_21052026]`, `[Audio_B2_25062026]`)**:
   - **Xem full lịch sử của 1 cổ phiếu bất kỳ**: Vào QMV Monitor $\rightarrow$ **Xóa hết mọi tick điều kiện lọc** $\rightarrow$ Gõ mã cổ phiếu (VD: `FPT`, `NVL`, `DCM`) $\rightarrow$ Chọn khoảng ngày `Từ ngày ... Đến ngày ...` $\rightarrow$ Bấm `Lọc cổ phiếu` $\rightarrow$ **BẮT BUỘC click vào tiêu đề cột `Ngày` để sắp xếp lại thứ tự thời gian** (do mặc định truy vấn trả về ngày có thể bị lộn xộn — `[Audio_B1_21052026]`). Cách này giúp kiểm tra quá khứ mã đó có hay bị tín hiệu nhiễu không, ngày ra tín hiệu trước đó có kèm `Phân kỳ dương` không (như lúc thầy soi lại lịch sử ngày 10/06 của `NVL` và ngày 18/06 của `DCM` trong `[Audio_B2_25062026]`).
   - **Mẹo quét lùi "Trong tuần này" khi ngành mục tiêu hôm nay trống tín hiệu (`[Audio_17072025]`)**: Khi dòng tiền vĩ mô ủng hộ một ngành (ví dụ ngành Chứng khoán) nhưng lọc ngày hôm nay ra `0` cổ phiếu $\rightarrow$ nghĩa là tín hiệu đã nổ cách đó vài phiên! Đổi bộ lọc thời gian sang **"Trong tuần này"** để lôi ra các mã vừa phát `Buy Rule + T+` cách đây 2–3 phiên (ví dụ thầy tìm ra `EVS` nổ tín hiệu ngày 15/07 khi đang đứng ở ngày 17/07) để kiểm tra phản ứng hàng T+ về và đón nhịp test lại!
   - **Đọc bảng lịch sử 6 phiên trên trang Cổ phiếu**: Nếu trong 6 phiên gần nhất trạng thái đổi liên tục từ `Buy Pattern` sang `Sell Pattern` rồi lại `Buy Pattern` $\rightarrow$ **cổ phiếu đang dao động ngang tạo nền tích lũy**, không mua đuổi phá nền sớm (`[Audio_17072025]`).

2. **Quy tắc Vàng tránh bẫy tín hiệu Trong Phiên (Real-time 30 phút) vs Kết Phiên (Cuối ngày) (`[Audio_17072025]`, `[Audio_B2_25062026]`)**:
   - Hệ thống QMV cập nhật dữ liệu **30 phút/lần** trong giờ giao dịch (`[Audio_17072025]`).
   - **Cơ chế gây bẫy trong phiên**: Trong phiên giao dịch, giá tạm thời có thể vọt lên mức cao nhất `High` (trước khi bị đạp cụt đầu về chiều thành nến `Kéo xả / Upthrust`). Vì `MaxBuy`, `Bullish Signal`, `Uốn lên` và `Top tiền vào` đều tính trên giá hiện hành và đường trung bình động, nên trong phiên các con số này **nhảy múa liên tục** theo râu nến (chưa kể hệ thống tính toán thuật toán nặng cho toàn thị trường cần thời gian chạy xong batch — `[Audio_B2_25062026]`).
   - **Chỉ thị dứt khoát của giảng viên**:
     - **Luôn ra quyết định dựa trên dữ liệu KẾT PHIÊN (Cuối ngày)** để có số liệu đóng cửa chuẩn xác nhất và có thời gian buổi tối bình tĩnh chạy hết Checklist 5 bước (Tài chính & Định giá $\rightarrow$ Khối ngoại & Cáo-Chó săn-Thỏ $\rightarrow$ Blackbox `TMC/DSPI/MPICP/DMx` $\rightarrow$ Kỹ thuật Bối cảnh lớn 5 bước sóng & 3 điểm chạm $\rightarrow$ Lập kế hoạch vùng Quản trị / Rebalancing / StopLoss).
     - **Giải tỏa tâm lý sợ lỡ tàu cuối phiên**: Học viên hỏi *"Cuối phiên các mã Top tiền vào đã tăng cao rồi khó vào thì làm sao?"* $\rightarrow$ Thầy trả lời (`[Audio_17072025]`): Nếu cổ phiếu thực sự bước vào form chuẩn ở **Bước sóng 2 hoặc Bước sóng 3** trong bối cảnh lớn 5 bước sóng, việc bỏ qua bước 0–1 hay mua chậm hơn 5–10% ở nhịp chỉnh/tích lũy kế tiếp là hoàn toàn bình thường và an toàn hơn gấp bội so với thói quen nhảy vào đua lệnh mù quáng theo tín hiệu chập chờn giữa phiên!