# BÁO CÁO NGHIÊN CỨU CHUYÊN SÂU HỆ THỐNG PTKT QMV – CỤM HP4-3: CÁC CHỈ SỐ KỸ THUẬT (MA, BOLLINGER BANDS, ICHIMOKU, XU HƯỚNG VS DAO ĐỘNG, PHÂN KỲ)

*(Báo cáo được đúc kết từ việc đọc trọn vẹn 100% từ byte đầu đến byte cuối toàn bộ 7 bài giảng gốc thuộc Học phần 4 – Chủ đề 3: `v1_3_chisokythuat_audio.mp3.md`, `v2_3_..._1_lythuyetve2nhomchiso_audio.mp3.md`, `v2_3_..._2_viduvema_bb_ichimoku_audio.mp3.md`, `v2_3_..._3_nhomchisoxuhuong_audio.mp3.md`, `v2_3_..._4_viduthutevatongket_audio.mp3.md`, `v3_3_..._bg_audio.mp3.md`, và `v3_3_..._hd_audio.mp3.md`).*

---

## PHẦN 1: CÁCH PHÂN LOẠI 2 NHÓM CHỈ BÁO CỐT LÕI CỦA QMV, NGUYÊN LÝ TOÁN HỌC & CHẾ ĐỘ THỊ TRƯỜNG (TRENDING VS SIDEWAYS)

### 1.1. Đặc điểm thuật ngữ riêng của QMV & Cách phân loại 2 nhóm cốt lõi (+ 1 tiểu nhóm Lưỡng tính)
> **Lưu ý cực kỳ quan trọng về thuật ngữ của giảng viên QMV**: Trong các giáo trình phương Tây, người ta thường gọi MA/BB/Ichimoku là *Trend Indicators* (Chỉ báo xu hướng) và RSI/CCI/Stochastic là *Oscillators* (Chỉ báo dao động). Tuy nhiên, xuyên suốt cả 3 phiên bản `v1_3`, `v2_3_1`, `v3_3_bg`, giảng viên QMV phân chia hàng ngàn chỉ số kỹ thuật thành **2 nhóm cốt lõi** (kèm **1 nhóm Lưỡng tính**) với tên gọi như sau:

1. **Nhóm 1 – Nhóm Chỉ số Trung bình động (Overlay / Không chuẩn hóa)** (`v1_3`, `v2_3_1`, `v3_3_bg`):
   - **Cách nhận diện trực quan**: Khi thêm vào đồ thị (trên Vietstock hoặc các nền tảng web), chỉ báo **nằm đè / gắn liền trực tiếp lên khung đồ thị giá**.
   - **Các đại diện giảng dạy**: **MA (Moving Average)**, **Bollinger Bands**, **Đám mây Ichimoku** (và xét về công thức gốc có thêm **MACD**).
   - **Ý nghĩa & Độ trễ**: Phản ánh trạng thái thị trường hiện tại so với quá khứ. Dù mọi chỉ số kỹ thuật đều chậm, nhóm Trung bình động **bị chậm nhiều hơn**, thiên về vai trò **Xác nhận (Confirm)** xu hướng đã và đang diễn ra (`v1_3`, `v2_3_1`, `v3_3_bg`).

2. **Nhóm 2 – Nhóm Chỉ số Sức mạnh xu hướng (hay gọi tắt là "Chỉ số xu hướng" / Chỉ số dẫn dắt – Tương đương nhóm Dao động & Động lượng đã chuẩn hóa)** (`v1_3`, `v2_3_1`, `v2_3_3`, `v3_3_bg`):
   - **Cách nhận diện trực quan**: Khi thêm vào đồ thị, chỉ báo **tách rời khỏi khung đồ thị giá**, nằm ở cửa sổ phụ riêng biệt bên dưới.
   - **Các đại diện thông dụng (7 chỉ báo trên slide giảng viên)**: **RSI**, **CCI**, **Stochastic**, **Williams %R**, **Momentum**, **CMF (Chaikin Money Flow)**, **MFI (Money Flow Index)** (`v1_3`, `v2_3_3`, `v3_3_bg`).
   - **Ý nghĩa & Độ trễ**: Đo lường tương quan sức mạnh bên mua – bên bán để đánh giá xu thế hiện tại có tiếp diễn hay sắp chững lại/đảo chiều. Ít trễ hơn nhóm Trung bình động nên được coi là **chỉ số dẫn dắt (leading)**, có giá trị dự báo/cảnh báo sớm hơn (`v1_3`, `v2_3_1`, `v3_3_bg`).

3. **Tiểu nhóm Lưỡng tính (Hybrid Indicators)** (`v1_3`, `v3_3_bg`):
   - **Đại diện điển hình**: **MACD** (ngoài ra trong `v3_3_bg`, thầy nhắc thêm **Stochastic** và **Williams %R** ở khía cạnh dùng giao cắt nhanh-chậm).
   - **Bản chất lưỡng tính**: MACD được cấu tạo từ hội tụ/phân kỳ các đường trung bình động (nhóm 1), nhưng lại được **chuẩn hóa xoay quanh trục `0`** và tách khỏi đồ thị giá (nhóm 2). Vì vậy có thể dùng theo cả 2 cách: giao cắt đường nhanh – đường chậm như MA, hoặc xét vị trí trên/dưới trục `0`, độ rộng cột bar (histogram) và đọc Phân kỳ như chỉ báo sức mạnh xu hướng (`v1_3`, `v3_3_bg`).

### 1.2. Nguyên lý toán học & Triết lý nền tảng giảng viên giải thích
- **Bản chất công thức Giá & Lượng + Lý do chậm mà vẫn dùng** (`v1_3`, `v2_3_1`, `v2_3_3`, `v3_3_bg`):
  - Tất cả hàng ngàn chỉ số kỹ thuật bản chất chỉ là **công thức toán học kết hợp trực tiếp giữa Giá và Lượng (Khối lượng)** trong quá khứ. Vì tính từ dữ liệu quá khứ nên **100% chỉ số kỹ thuật đều CHẬM (lagging)**.
  - Tại sao chậm mà vẫn dùng? Vì PTKT dựa trên giả định hành vi quá khứ sẽ lặp lại trong tương lai do 2 yếu tố cấu trúc:
    1. **Đội tạo lập thị trường (Market Maker / Đội lái)** (`v1_3`, `v2_3_1`, `v3_3_bg`): Ở TTCK Việt Nam, gần như cổ phiếu lớn hay nhỏ đều có tạo lập ("lái có tâm" tạo thanh khoản mua bán hai chiều, hoặc "lái tiêu cực" kéo xả lùa gà). Mỗi đội có thói quen điều tiết giá khác nhau.
    2. **Tệp "Khách hàng mục tiêu" (Nhà đầu tư mục tiêu) của từng cổ phiếu** (`v2_3_1`, `v3_3_bg`): Giống kinh doanh có khách hàng mục tiêu, cổ phiếu trả cổ tức tiền mặt đều đặn thu hút người lớn tuổi/hưu trí -> giao dịch điềm đạm, ít biến động, tạo các dải dao động rõ nét; ngược lại cổ phiếu công nghệ không chia cổ tức (chỉ đánh cược tăng trưởng lãi vốn - capital gain) thu hút giới trẻ ưa rủi ro -> tăng dựng đứng và sập rất gắt (`v2_3_1`).
- **Nguyên lý toán học: Không chuẩn hóa vs Chuẩn hóa (Normalization)** (`v2_3_3`, `v3_3_bg`):
  - **Không chuẩn hóa (Nhóm Trung bình động)**: Tính trung bình cộng trực tiếp của $N$ thanh nến gần nhất (MA), hoặc cộng/trừ $2\sigma$ độ lệch chuẩn thống kê (BB), hoặc trung bình nhiều chu kỳ dịch chuyển thời gian (Ichimoku). Giá trị chạy tự do theo mệnh giá cổ phiếu.
  - **Chuẩn hóa thống kê (Nhóm Sức mạnh xu hướng / Dao động)**: Trong `v3_3_bg` (và `v2_3_3`), giảng viên tiết lộ bản chất: *"Các đường chỉ số xu hướng này bản chất cũng xây dựng trên các đường trung bình, NHƯNG ĐƯỢC CHUẨN HÓA"*.
    - **Cơ chế tính toán tăng/giảm**: Phần lớn các chỉ báo này tách riêng những ngày giá tăng để tính lực đi lên và những ngày giá giảm để tính lực đi xuống -> Do đó chỉ báo đi lên = lực mua mạnh, chỉ báo đi xuống = lực bán mạnh (`v3_3_bg`).
    - **Cơ chế chuẩn hóa & Nguyên lý 3 phần QMV**: Phép chuẩn hóa ép giá trị chỉ báo dao động xoay quanh một **đường trung tâm / trung bình (`0` hoặc `50`)** và bị chặn bởi **cận trên – cận dưới** (`0` đến `1`, `-1` đến `+1`, `0` đến `100`...). Điều này khớp hoàn hảo với **Nguyên lý 3 phần** của hệ thống QMV: chia không gian chỉ báo thành **3 vùng tâm lý**: **(1) Vùng Quá mua (Mua nhiều quá) – (2) Vùng Trung tâm (Bình thường) – (3) Vùng Quá bán (Bán nhiều quá)** (`v3_3_bg`).

### 1.3. Ba tác dụng của chỉ báo kỹ thuật: Lý thuyết vs Thực chiến 20 năm của QMV (`v1_3`, `v2_3_1`, `v3_3_bg`)
- Sách giáo khoa nêu 3 tác dụng: **(1) Cảnh báo (Warning) – (2) Xác nhận (Confirmation) – (3) Dự đoán (Prediction)**.
- **Quan điểm thực chiến QMV**:
  - **Cảnh báo là quan trọng số 1**: Giống như **"đèn vàng giao thông"** (`v1_3`). Thấy cảnh báo không phải để nhắm mắt mua/bán ngay, mà để bật chế độ tập trung quan sát hành vi **Giá & Lượng (VPA)**, **Nến**, **Mẫu hình**, **Hỗ trợ/Kháng cự**.
  - **Xác nhận thường đã muộn**: Khi đạt điều kiện xác nhận (MA cắt nhau, giá vượt hẳn mây, chỉ báo cắt thoát vùng quá mua/quá bán) thì phần lớn sóng đầu đã qua (`v2_3_1`, `v3_3_bg`).
  - **Dự đoán cực kỳ hạn chế**: Mang nặng tính chủ quan, chỉ hình thành khi hội tụ đồng thời nhiều công cụ (`v1_3`, `v2_3_1`, `v3_3_bg`).

### 1.4. Khi nào Nhóm Trung bình động ĐÚNG / SAI? Khi nào Nhóm Dao động bị "LIỆT / SAI LIÊN TỤC"?
1. **Nhóm Trung bình động (MA, BB, Ichimoku)**:
   - **ĐÚNG (Hiệu quả tối đa)**: Khi thị trường hoặc cổ phiếu **có xu hướng rõ ràng (Trending)** kéo dài theo tháng, quý, năm. Giảng viên khẳng định qua backtest: trong giai đoạn thị trường uptrend mạnh kéo dài cả năm, mọi hệ thống giao dịch đơn giản dựa trên MA (cắt lên mua, nằm trên giữ, cắt xuống bán) gần như **100% đều thắng lớn** (`v1_3`, `v2_3_2`, `v3_3_bg`).
   - **SAI LIÊN TỤC / VÔ HIỆU**: Khi thị trường **Sideways (đi ngang)** hoặc biến động giật cục ngắn ngày. Đường giá cắt lên cắt xuống đường MA liên tục (whipsaw); đặc biệt do TTCK Việt Nam vướng chu kỳ thanh toán **T+3** (tại thời điểm bài giảng), mua lúc cắt lên thì hàng chưa về tài khoản giá đã cắt xuống gây lỗ, bán lúc cắt xuống xong giá lại giật ngược lên ngay (`v1_3`, `v2_3_2`).

2. **Nhóm Sức mạnh xu hướng / Dao động (RSI, CCI, Stochastic...) – Hiện tượng "Bị liệt / Sai liên tục" nếu dùng ngược xu hướng**:
   - **Sai lầm kinh điển của đám đông**: Đọc sách máy móc cứ thấy chỉ báo vào vùng Quá mua (>70) là bán, vào vùng Quá bán (<30) là mua!
   - **Giải thích của QMV về hiện tượng neo biên ("liệt") & Thiết quân luật *"Ủng hộ xu hướng, KHÔNG dùng ngược xu hướng"*** (`v1_3`, `v2_3_1`, `v2_3_3`, `v3_3_bg`):
     - **Trong UPTREND mạnh (Xu thế tăng)**:
       - Khi RSI/CCI đi vào vùng **Quá mua (Mua quá nhiều)**: Thực chất nó đang xác nhận xung lực mua cực mạnh ủng hộ xu hướng tăng, và nó có thể **neo lì ở vùng quá mua kéo dài cả tháng trời** (`v2_3_3`)! Nếu bán ngay khi chớm quá mua sẽ mất sạch sóng lớn (thậm chí nhiều trader chuyên nghiệp chỉ bắt đầu mua khi RSI vượt 70 đi vào vùng quá mua - `v3_3_bg`). **Quy tắc xử lý**: **Trì hoãn (delay) lệnh bán**, coi là cảnh báo để quan sát nhưng *"cứ để nó chạy (let it run)"* cho đến khi gãy trendline chỉ báo hoặc cắt rớt ngược khỏi vùng quá mua (`v1_3`, `v2_3_3`, `v3_3_bg`).
       - Ngược lại, trong Uptrend mạnh mà chỉ báo đột ngột rớt xuống vùng **Quá bán (Bán quá nhiều)**: Đóng vai trò cú rũ bỏ ngắn hạn -> **Tác dụng cực nhạy, phải HÀNH ĐỘNG NHANH: nhảy vào bắt đáy / mua sớm ngay!** (`v1_3`, `v2_3_1`, `v2_3_3`, `v3_3_bg`).
     - **Trong DOWNTREND mạnh (Xu thế giảm)**:
       - Khi RSI/CCI rơi vào vùng **Quá bán (Bán quá nhiều)**: Nó có thể **nằm bẹp dí dưới đáy quá bán suốt thời gian dài** trong khi giá vẫn rơi không phanh. Ai thấy quá bán nhảy vào mua ngay sẽ chết vì bắt dao rơi -> **Quy tắc xử lý**: **Thận trọng tối đa, nhởn nha chờ đợi, KHÔNG mua vội** (`v1_3`, `v2_3_3`, `v3_3_bg`).
       - Ngược lại, trong Downtrend mà chỉ báo nảy hồi lên chạm vùng **Quá mua (Mua quá nhiều)**: Lực hồi cạn kiệt trước xu thế giảm lớn -> **Tác dụng cực mạnh, phải HÀNH ĐỘNG NHANH: chạy đỉnh / bán thoát hàng ngay lập tức!** (`v1_3`, `v2_3_3`, `v3_3_bg`).

### 1.5. Cách nhận diện & Chuyển chế độ giữa Trending vs Sideways (`v1_3`, `v2_3_1`, `v2_3_2`, `v3_3_bg`, `v3_3_hd`)
- **Nhận diện trạng thái & điểm chuyển pha**:
  - **Đang Trending**: Cấu trúc đỉnh sau cao hơn đỉnh trước, đáy sau cao hơn đáy trước (nối các đáy tạo đường xu hướng dốc lên bền vững kéo dài hàng tháng/quý); giá nằm hẳn trên mây Ichimoku; dải Bollinger Bands mở rộng hoặc giá bám biên ngoài BB (`v1_3`, `v3_3_bg`, `v3_3_hd`).
  - **Chuyển từ Trending sang Sideways**: (1) Giá cắt thủng đường trendline tăng -> chấm dứt giai đoạn *"tăng chắc"*, bước vào giai đoạn lưỡng lự sau tăng (`v1_3`); (2) Dải Bollinger Bands đang mở rộng bắt đầu **co hẹp lại** (`v1_3`, `v3_3_hd`); (3) Giá đi vào nằm **bên trong đám mây Ichimoku** (`v2_3_2`, `v3_3_bg`); (4) Hình thành vùng kháng cự ngang – hỗ trợ ngang hoặc mẫu hình lưỡng lự (như phễu ngang mở rộng, chữ nhật...) (`v1_3`, `v2_3_1`).
  - **Chuyển từ Sideways sang Trending mới**: Dải Bollinger Bands **thắt nút cổ chai hẹp bất thường** so với quá khứ rồi bung mở ra kèm giá breakout qua biên kháng cự/hỗ trợ (đặc biệt tại **điểm chạm số 3** theo quy luật tâm lý 3 cấp độ rủi ro của con người trong `v1_3`), được xác nhận bởi MACD cắt lên hoặc Phân kỳ RSI/CCI (`v1_3`, `v3_3_bg`, `v3_3_hd`).
- **Chuyển đổi vũ khí giao dịch theo chế độ**:
  - **Khi Trending**: Dùng **Nhóm Trung bình động (MA, BB, Ichimoku)** làm kim chỉ nam giữ hàng ăn trọn sóng + dùng **Nhóm Sức mạnh xu hướng** thuần túy để gia tăng/bắt nhịp chỉnh thuận chiều xu hướng. (Lưu ý thêm từ `v3_3_bg`: Nếu cổ phiếu đang có **game / tin tức nóng / dòng tiền đầu cơ cuồn cuộn**, các cảnh báo kỹ thuật trở thành phụ, chỉ chỉnh nhẹ rồi đi tiếp; ngược lại cổ phiếu **không có tin tức gì đặc biệt** hoặc cổ phiếu **thanh khoản lớn, đại chúng, thậm chí "cổ phiếu trà đá"** nhiều nhỏ lẻ tham gia thì PTKT đọc tâm lý đám đông cực kỳ chuẩn xác - `v1_3`, `v3_3_bg`).
  - **Khi Sideways**: Tắt/bỏ qua tín hiệu giao cắt MA đơn thuần để tránh bẫy T+3; chuyển sang dùng **Nhóm Chỉ số sức mạnh xu hướng (RSI, CCI, Stochastic...)** kết hợp chặt chẽ với **Kháng cự ngang – Hỗ trợ ngang** và chiến thuật **Rebalance** (mua thấp bán cao đảo trên lượng cổ phiếu sẵn có trong danh mục) (`v1_3`, `v2_3_1`, `v3_3_bg`).

---

## PHẦN 2: BỘ THAM SỐ GỐC VÀ QUY TẮC SỬ DỤNG CỦA TỪNG CHỈ BÁO TRONG QMV (HP4-3)

> **Kiểm chứng thực tế từ 100% văn bản 7 bài giảng HP4-3**: Trong phạm vi HP4-3 (`v1_3`, `v2_3`, `v3_3`), giảng viên giảng dạy dựa trên các bộ tham số gốc phổ biến trên đồ thị Vietstock (nhằm tận dụng hiệu ứng tâm lý đám đông nhiều người cùng nhìn) kết hợp nguyên lý tự hiệu chỉnh tham số theo lịch sử từng mã (*Data Mining*), chứ **chưa sử dụng** các bộ tham số biến thể như Ichimoku `(6, 18, 36, 18)` hay `BB(15)` của các chuyên đề nâng cao về sau. Dưới đây là chi tiết từng chỉ báo:

### 2.1. Đường Trung bình động – MA (SMA / EMA / WMA)
- **Loại đường MA**: Giảng viên đề cập có `SMA` (trung bình giản đơn) và `EMA` (trung bình mượt), không nhắc `WMA`. Trên đồ thị giảng dạy, thầy dùng **MA thông thường (SMA)** cho đơn giản (`v3_3_bg`), đồng thời lưu ý bản thân đường Giá chính là `MA(1)` (`v1_3`).
- **Bộ chu kỳ tham số QMV sử dụng** (`v1_3`, `v2_3_2`, `v3_3_bg`):
  - Bộ 3 đường chuẩn trên mọi đồ thị bài giảng: **`MA(5)` (màu xanh) – `MA(50)` (màu đỏ) – `MA(200)` (màu vàng)**:
    - **`MA(5)`**: Đại diện **1 tuần** giao dịch (5 phiên) -> Xu thế ngắn hạn / lướt sóng T+. Giảng viên lưu ý tùy cổ phiếu có thể chỉnh thành **`9`**, **`10`** hoặc **`20`** (`v1_3`, `v2_3_2`).
    - **`MA(20)`**: Đại diện **1 tháng** giao dịch (20 phiên) -> Cũng chính là đường trục giữa của Bollinger Bands (`v2_3_2`, `v3_3_bg`). *(Trong `v2_3_2` thầy có nhắc thoáng qua con số 30 ngày và nói nhịu 50 ngày là 1 tháng, nhưng ở cả `v1_3` và `v3_3_bg` thầy chuẩn hóa rõ ràng: `5` = 1 tuần, `20` = 1 tháng, `50` = 1 quý, `200` = 1 năm).*
    - **`MA(50)`**: Đại diện **1 quý** giao dịch -> Chuẩn mực quốc tế cho xu thế **Trung hạn** (`v1_3`, `v2_3_2`, `v3_3_bg`).
    - **`MA(200)`**: Đại diện **1 năm** giao dịch -> Chuẩn mực quốc tế cho xu thế **Dài hạn** (`v1_3`, `v2_3_2`, `v3_3_bg`).
    - *(Các chu kỳ `15` và `100` không được sử dụng trong 7 bài giảng này).*
- **Quy tắc chọn Khung thời gian (Timeframe) kèm MA** (`v1_3`):
  - Dù phân tích ngắn hạn, trung hạn hay dài hạn cho TTCK Việt Nam, giảng viên khuyên **luôn dùng đồ thị Ngày (1D - Daily)** vì được số đông quan sát nhất (muốn nhìn trung/dài hạn thì thu phóng quãng thời gian rộng ra trên khung 1D cùng `MA50`, `MA200`, tốt hơn là bật khung Tuần/Tháng vì khung Tuần làm mất chi tiết diễn biến trong tuần).
  - Chỉ hạ xuống khung **15 phút hoặc 30 phút** khi cần soi điểm uốn bắt đáy trong phiên hoặc lướt T0 (`v1_3`).
- **Quy tắc giao dịch với MA** (`v1_3`, `v2_3_2`, `v3_3_bg`):
  1. **Định vị đa khung xu hướng**: Giá > `MA5` (tăng ngắn hạn); `MA5` > `MA50` hoặc Giá > `MA50` (tăng trung hạn 1 quý); `MA50` > `MA200` hoặc Giá > `MA200` (tăng dài hạn 1 năm).
  2. **Quy tắc Giao cắt & Nắm giữ**: Giá cắt lên `MA5` -> Tín hiệu Mua ngắn hạn; chừng nào giá còn nằm trên `MA5` thì **Giữ tiếp (*let it run*)**. Giá lừng khừng cắt thủng xuống `MA5` -> Cảnh báo Bán; chừng nào giá còn nằm dưới `MA5` thì **Chờ (đứng ngoài)** cho tới khi cắt ngược lên lại (`v1_3`, `v2_3_2`). Đường MA ngắn cắt lên MA dài (`MA5` cắt `MA50`, `MA50` cắt `MA200`) -> Xác nhận chuyển pha tăng trung hạn / dài hạn (`v1_3`, `v2_3_2`, `v3_3_bg`).
  3. **Hỗ trợ động & Kháng cự động**: Trong uptrend, mỗi nhịp chỉnh giá chạm `MA5` bật lên là điểm mua T+; nếu gãy `MA5` trong nhịp chỉnh lớn thì canh mua khi chạm hỗ trợ trung hạn `MA50` nảy lên (`v2_3_2`, `v3_3_bg`).
  4. **Kỷ luật bất biến về tham số (`v1_3`)**: Mua vì đường MA chu kỳ nào (ví dụ mua vì cắt lên `MA10`) thì bắt buộc phải bán theo đúng đường `MA10` đó; **tuyệt đối cấm tự ý đổi tham số** sang `MA20` hay `MA50` để bao biện cho việc gồng lỗ/níu kéo vị thế!

### 2.2. Dải Bollinger Bands (BB)
- **Bộ tham số gốc trong HP4-3** (`v1_3`, `v2_3_2`, `v3_3_bg`):
  - Đường trung tâm: **`MA(20)`** (trung bình 20 phiên).
  - Độ rộng biên trên & biên dưới: **`2` lần độ lệch chuẩn ($\pm 2\sigma$ / `2 Standard Deviations`)** của giá trong 20 phiên gần nhất.
  - **Giải nghĩa tài chính học (`v1_3`)**: Độ lệch chuẩn ($\sigma$) chính là thước đo **Rủi ro / Sự không chắc chắn (Uncertainty)** của thị trường.
- **Quy tắc sử dụng Bollinger Bands trong QMV**:
  1. **Đường giữa và 2 biên trong trạng thái bình thường**: Trục giữa `MA20` dùng như đường MA hỗ trợ/kháng cự và giao cắt; dải trên đóng vai trò vùng xuất hiện lực cung (kháng cự động), dải dưới là vùng xuất hiện lực cầu (hỗ trợ động) khi thị trường tăng/giảm ôn hòa (`v1_3`, `v2_3_2`, `v3_3_bg`).
  2. **Quy tắc "Phản ứng thái quá (Overreaction)" – Bám biên (Walking the Bands) & Vọt ra ngoài rồi chui ngược vào trong** (`v1_3`, `v2_3_2`, `v3_3_bg`):
     - **Bác bỏ sai lầm "chạm biên là đảo chiều"**: Chạm dải trên KHÔNG có nghĩa là phải bán, chạm dải dưới KHÔNG có nghĩa là phải mua (`v1_3`, `v3_3_bg`).
     - **Bám biên & Vọt ra ngoài biên**: Khi giá bám sát dải trên hoặc **vọt hẳn ra ngoài dải trên BB**, đó là trạng thái **Overreaction / FOMO (tăng cực mạnh)**. Quy tắc: **Chừng nào giá còn nằm ngoài (hoặc bám sát) dải trên BB -> Xu thế cực mạnh vẫn tiếp diễn -> CỨ ĐỂ NÓ CHẠY, cấm bán non!** Tương tự, khi giá thủng ra ngoài dải dưới BB và tiếp tục nằm ngoài -> đà rơi hoảng loạn còn tiếp diễn, để rơi tiếp, cấm bắt đáy (`v1_3`, `v2_3_2`, `v3_3_bg`).
     - **Tín hiệu hành động khi "Chui ngược trở lại vào trong dải"**: Chỉ khi giá **đã vọt ra ngoài dải BB rồi quay đầu chui ngược vào bên trong dải BB**, lúc đó mới báo hiệu đà tăng/giảm thái quá đã suy yếu! Ở biên trên chui vào -> cảnh báo suy yếu để canh chốt; ở biên dưới rơi ra ngoài rồi chui ngược vào trong -> đợt bán thái quá thất bại, tạo **điểm mua đảo chiều** (`v1_3`, `v2_3_2`, `v3_3_bg`). *(Lưu ý: Trong 7 bài HP4-3 giảng viên dạy quy tắc chuyển trạng thái từ ngoài chui vào trong, kết hợp nguyên tắc thuận xu thế lớn — uptrend thì rớt biên dưới mua nhanh, vượt biên trên hoãn bán — chứ chưa chia nhỏ thành quy tắc đếm chạm biên lần 1 vs lần 2).*
  3. **Hiện tượng Thắt nút cổ chai / Nén (BB Squeeze / Độ hẹp) vs Mở rộng** (`v1_3`, `v2_3_2`, `v3_3_bg`, `v3_3_hd`):
     - **Mở rộng**: Phản ánh xu hướng đang mạnh; chừng nào BB còn mở rộng thì xu hướng còn khỏe (`v3_3_hd`).
     - **Co hẹp bất thường (Squeeze / Press)**: Khi dải BB đang rộng mà thu hẹp lại -> hai phe mua bán cân bằng, lưỡng lự. Để biết đã đủ nén để nổ chưa, phải **đối chiếu với độ hẹp trong quá khứ của chính mã đó** (`v2_3_2`). Khi hẹp bất thường (nhất là đi ngang tích lũy lâu ở hỗ trợ) -> **Chắc chắn chuẩn bị có một cú bùng nổ lớn sắp xảy ra!**
     - **Cách xác định hướng nổ**: Bản thân BB hẹp **không tự nói được sẽ nổ lên hay nổ xuống** (`v1_3`, `v2_3_2`, `v3_3_bg`, `v3_3_hd`). Phải phối hợp với **Kháng cự/Hỗ trợ, Phân kỳ RSI/CCI và giao cắt MACD** (ví dụ: BB hẹp + Phân kỳ dương + MACD cắt lên = cược nổ lên - `v3_3_hd`). Vì sau khi nén chặt xu hướng bung ra thường kéo dài, nhà đầu tư có thể chờ giá chính thức breakout mở dải BB rồi vào lệnh vẫn kịp ăn trọn sóng (`v3_3_bg`).

### 2.3. Đám mây Ichimoku (Ichimoku Kinko Hyo)
- **Bộ tham số & Quan điểm của giảng viên trong HP4-3** (`v1_3`, `v2_3_1`, `v2_3_2`, `v3_3_bg`, `v3_3_hd`):
  - **Bộ tham số**: Trong cả `v1_3` và `v2_3_2`, giảng viên dùng **bộ tham số chuẩn mặc định trên Vietstock với chu kỳ gốc `26` phiên** (thầy nêu rõ con số **26 thanh nến / 26 phiên** lùi về quá khứ cho đường chậm Chikou và **26 phiên** chiếu về tương lai ~1 tháng cho đám mây Kumo; **không dùng** bộ `6, 18, 36, 18` trong học phần này).
  - **Triết lý giải ảo của giảng viên**: Thầy đưa Ichimoku vào bài giảng không phải để thần thánh hóa nó (cá nhân thầy không dùng vì rối mắt - `v3_3_hd`), mà để chứng minh rằng công cụ trông phức tạp nhất thực chất cũng chỉ là **5 đường trung bình động ghép lại**, vận hành đúng theo quy luật giao cắt và hỗ trợ/kháng cự như MA (`v1_3`, `v2_3_1`, `v2_3_2`, `v3_3_bg`).
- **Quy tắc đọc 5 thành phần & Đám mây Kumo** (`v1_3`, `v2_3_2`, `v2_3_4`, `v3_3_bg`):
  1. **Tenkan-sen (Đường chuyển đổi – màu xanh)** & **Kijun-sen (Đường cơ sở – màu đỏ/nâu)**:
     - Tenkan là đường trung bình ngắn (nhanh), Kijun là đường trung bình dài (chậm).
     - **Giao cắt**: Tenkan cắt lên Kijun = Mua; cắt xuống = Bán. Đặc biệt nếu Tenkan võng xuống chạm/tiệm cận Kijun nhưng **không cắt thủng mà đi trùng rồi bật lên** -> báo hiệu phe bán yếu, đà tăng tiếp diễn (`v1_3`).
     - **Vị trí điểm cắt so với mây**: Điểm giao cắt nằm trên mây và **càng xa đám mây thì xu thế tăng càng mạnh**; cắt sát đám mây thì xu thế yếu hơn (`v2_3_2`, `v3_3_bg`).
  2. **Senkou Span A (Dẫn dắt A) & Senkou Span B (Dẫn dắt B) – Đám mây Kumo tương lai 26 phiên**:
     - Senkou A nằm trên Senkou B -> **Mây Xanh** (kỳ vọng xu thế tăng); Senkou A cắt xuống dưới Senkou B -> **Mây Đỏ** (kỳ vọng xu thế giảm/xấu) (`v1_3`, `v2_3_2`, `v3_3_bg`).
     - **Cảnh báo chống mê tín mây tương lai 26 phiên (`v1_3`, `v2_3_2`)**: Mây tương lai 26 phiên chỉ là phép phóng chiếu từ công thức quá khứ (*"vẽ đường cho hươu chạy"*), hoàn toàn có thể đổi màu ngược lại chỉ sau vài phiên nếu giá đảo chiều mạnh (thầy chỉ rõ ví dụ đoạn xoắn đổi sang mây đỏ bị fail rồi lập tức xoắn xanh trở lại trên đồ thị `v1_3`).
     - **Độ dày/mỏng, độ dốc & sự thu hẹp của mây**: Mây dày và dốc lên thể hiện xu thế mạnh, vùng hỗ trợ/kháng cự dày; mây mỏng hoặc thu hẹp lại báo hiệu xu thế suy yếu/dễ đổi chiều; giá càng cách xa đám mây thì xu thế càng mạnh (`v1_3` phần Q&A bạn Dũng; `v2_3_4`).
  3. **Chikou Span (Đường chậm – lùi lại 26 phiên)**:
     - **Chikou càng rời xa đám mây (và đường giá) thì xu thế hiện tại càng mạnh và bền vững**; nếu Chikou tiến lại gần đám mây là dấu hiệu xu hướng suy yếu, không tốt (`v1_3`, `v2_3_2`, `v3_3_bg`).
  4. **Vị trí Giá vs Mây & Chiến thuật giải ngân Xuyên mây (`v1_3`, `v2_3_2`, `v3_3_bg`)**:
     - Giá **trên mây** = Uptrend (mây là hỗ trợ động); Giá **dưới mây** = Downtrend (mây là kháng cự động); Giá **trong mây** = Sideways.
     - **Cách chia tiền mua xuyên mây (`v1_3`)**: Khi giá đang dưới mây tạo điểm uốn vòng chui vào trong mây -> đà giảm chững lại (điểm bắt đáy sớm, rủi ro cao); khi giá **vượt hẳn lên khỏi bờ trên đám mây** -> điểm xác nhận an toàn (Confirm). Nhà đầu tư chuẩn mực **chia tiền mua 2 lần**: giải ngân thăm dò khi giá tạo điểm uốn chui vào mây và **mua gia tăng** khi giá vượt hẳn lên trên mây!
  5. **Kỹ thuật đặc sản của Ichimoku: Phát hiện Bẫy giá – Bear Trap & Bull Trap (`v2_3_2`, `v3_3_bg`)**:
     - **Phát hiện Bẫy Giảm (Bear Trap trong Uptrend)**: Giá đang nằm **trên đám mây** (xu thế gốc là tăng), bị đạp thủng xuống dưới **đường cơ sở (Kijun)** làm thị trường tưởng sập, NHƯNG sau đó giá vòng lên **vượt ngược lên trên đường chuyển đổi (Tenkan) / đường cơ sở** -> Xác nhận nhịp đạp vừa rồi chỉ là bẫy giảm, nhịp hồi là tăng thật chứ không phải bulltrap! *(Ghi chú văn bản: Trên slide gốc ở cả `v2_3_2` và `v3_3_bg` ghi: "Giá trên đám mây bị kéo xuống dưới đường cơ sở rồi lại vượt lên trên đường chuyển đổi"; ở `v3_3_bg` khi giảng miệng thầy có đoạn nói nhịu đảo tên hai đường ngắn/dài rồi tự đính chính).*
     - **Phát hiện Bẫy Tăng (Bull Trap trong Downtrend)**: Giá đang nằm **dưới đám mây** (xu thế gốc là giảm), hồi phục **vượt lên trên đường cơ sở (Kijun)** tưởng như đảo chiều, NHƯNG sau đó lại **rơi thủng ngược xuống dưới đường chuyển đổi (Tenkan)** -> Xác nhận là **Bull trap (bẫy tăng)**, xu thế giảm tiếp diễn (`v2_3_2`, `v3_3_bg`).

### 2.4. MACD, RSI, CCI, Stochastic, Williams %R, CMF/MFI & Triết lý tối giản chỉ báo
- **Kiểm chứng danh mục chỉ báo trong 7 bài HP4-3**:
  - Có giảng dạy/minh họa: **MACD, RSI, CCI, Stochastic, Williams %R, Momentum, CMF (Chaikin Money Flow), MFI (Money Flow Index)** và **Volume**.
  - **Không sử dụng trong 7 bài này**: `ADX/DMI`, `OBV`, và **không dạy cách đặt đường MA đè lên chỉ báo RSI/CCI** (thay vào đó QMV dạy **vẽ trực tiếp đường Trendline và đường Kháng cự/Hỗ trợ ngang lên chính RSI/CCI**!).
- **Quy tắc cụ thể từng chỉ báo**:
  1. **MACD** (`v3_3_bg`, `v3_3_hd`):
     - Tham số mặc định Vietstock; xoay quanh trục trung tâm **`0`**. Các cột bar nhỏ (**Histogram**) = Đường nhanh (xanh) trừ Đường chậm (đỏ).
     - Đường nhanh cắt lên đường chậm = Mua; cắt xuống = Bán. **Bộ lọc sức mạnh theo trục `0`**: Khi hai đường và Histogram nằm **trên trục `0`** là thị trường tăng giá -> điểm cắt lên nằm trên trục `0` cực khỏe và đáng tin cậy; nằm **dưới trục `0`** (cột bar đỏ) là thị trường giảm giá (`v3_3_bg`). Kết hợp thêm đọc Phân kỳ đỉnh/đáy MACD.
  2. **RSI (Relative Strength Index)** – *Chỉ báo chủ lực số 1 của giảng viên* (`v1_3`, `v2_3_3`, `v3_3_bg`, `v3_3_hd`):
     - Tham số mặc định (14 phiên). Đường trung tâm **`50`**, dải trung bình **`30 – 70`**, trên **`70`** là **Quá mua**, dưới **`30`** là **Quá bán**. Khi RSI nằm ở nửa trên vùng trung bình (**`50 – 70`**) là bên mua đang thắng thế lành mạnh (`v1_3`); nếu cả chu kỳ dài RSI không bao giờ tụt xuống `30` chứng tỏ cổ phiếu giữ uptrend bền bỉ (`v3_3_bg`).
     - **Kỹ thuật kẻ Trendline & Kháng cự/Hỗ trợ trực tiếp trên RSI (`v3_3_bg`)**:
       - *Quy tắc 2 điểm Bán trên RSI*: Nối các đáy RSI tạo trendline tăng. **Điểm bán 1**: RSI cắt thủng trendline tăng của chính nó; **Điểm bán 2**: RSI từ vùng >70 cắt rơi xuống dưới ngưỡng `70` trở lại vùng bình thường (có thể chia 3 phần bán dần).
       - *Quy tắc 2 điểm Mua trên RSI*: Nối các đỉnh RSI tạo trendline giảm. **Điểm mua 1**: RSI phá vỡ cắt lên trên trendline giảm; **Điểm mua 2**: RSI từ vùng <30 vòng lên cắt qua ngưỡng `30` vào vùng bình thường.
       - *Kháng cự/Hỗ trợ ngang trên RSI*: Kẻ đường ngang qua các đỉnh/đáy lịch sử của RSI; khi RSI tiến tới đỉnh cũ thường ngập ngừng, nếu phá qua dễ dàng chứng tỏ xung lực cực mạnh (`v3_3_bg`).
  3. **CCI (Commodity Channel Index)** – *Chỉ báo chủ lực số 2 của giảng viên* (`v1_3`, `v2_3_3`, `v3_3_bg`, `v3_3_hd`):
     - **Tham số**: Mặc định phổ biến **`14` ngày**, nhưng giảng viên chỉ rõ phải linh hoạt hiệu chỉnh (ví dụ **`10` ngày** hoặc **`20` ngày**) sao cho khớp với nhịp lịch sử của từng cổ phiếu (`v3_3_bg`).
     - **Ngưỡng vùng**: Xoay quanh đường trung tâm **`0`** với dải trung tâm bôi xám (chuẩn `-100` đến `+100`): vượt trên dải trung tâm là **Quá mua**, tụt dưới dải trung tâm là **Quá bán** (`v1_3`, `v2_3_3`).
     - **Đặc tính phối hợp RSI + CCI**: CCI phản ứng nhạy hơn với các pha bứt tốc/quá đà (nhiều lúc CCI đã vọt vào vùng Quá mua và vượt đỉnh lịch sử cũ báo hiệu xung lực cực đại trong khi RSI vẫn dưới 70 - `v1_3`, `v2_3_3`). Khi **cả RSI và CCI cùng xuất hiện phân kỳ** thì độ tin cậy cao vượt trội (`v3_3_hd`).
  4. **Stochastic & Williams %R** (`v2_3_3`, `v3_3_bg`, `v3_3_hd`):
     - Hai chỉ báo này vận động gần như giống hệt nhau (`v2_3_3`). Với Stochastic, đường nhanh (xanh) cắt lên đường chậm (đỏ) cho tín hiệu Mua **đáng tin cậy nhất khi điểm cắt nằm ở/sát vùng Quá bán bên dưới** (`v3_3_bg`).
  5. **CMF (Chaikin Money Flow), MFI & Momentum** (`v1_3`, `v2_3_3`, `v2_3_4`, `v3_3_bg`):
     - Ba chỉ báo này đi rất giống nhau, chỉ cần chọn 1 cái (thầy hay bật **CMF**).
     - **Cách đọc CMF**: Xoay quanh trục **`0`**. Trên `0` = dòng tiền vào dương (ở lại thị trường); dưới `0` = dòng tiền rút ra (`v1_3`, `v2_3_3`, `v2_3_4`). Phải so sánh độ cao của CMF hiện tại với đỉnh CMF quá khứ: nếu chỉ số giá vẫn vượt đỉnh mà CMF thấp dần/đi ngang -> tiền không vào thêm mà chỉ kéo trụ, cảnh báo rủi ro đảo chiều tiềm ẩn (`v1_3`, `v2_3_4`).
  6. **Nguyên tắc "Ít chỉ báo nhưng hiểu sâu" & Ba câu hỏi bắt buộc** (`v1_3`, `v2_3_3`, `v3_3_bg`, `v3_3_hd`):
     - Cả hệ thống chỉ cần biết ~6 chỉ số, và thực chiến chỉ nên chuyên sâu **2–3 chỉ số** nhóm xu hướng (combo chuẩn của thầy: **Bollinger Bands (gồm MA20) + RSI + CCI**).
     - Mọi lần liếc đồ thị chỉ số đều phải trả lời **3 câu hỏi**: (1) **Hướng đi** thời gian qua thế nào? (2) **Vùng điểm** hiện tại (quá mua / bình thường / quá bán) đặt trong xu thế lớn tăng hay giảm? (3) Có chạm **Kháng cự/Hỗ trợ của chỉ số** hay tạo **Phân kỳ / lệch độ dốc** với giá không?

---

## PHẦN 3: KỸ THUẬT ĐỌC PHÂN KỲ (DIVERGENCE) CHUẨN QMV

### 3.1. Bản chất Kinh tế học (Cung – Cầu) của Phân kỳ (`v3_3_bg`, `v3_3_hd`)
Giảng viên QMV giải thích bản chất tại sao Phân kỳ lại hiệu quả bằng **Quy luật Cung – Cầu** chứ không học vẹt hình vẽ:
- Vì chỉ báo sức mạnh xu hướng đi xuống mạnh phản ánh các phiên bán mạnh, đi lên mạnh phản ánh các phiên mua mạnh:
  - **Bản chất Phân kỳ Dương (ở Đáy – Canh Mua)**: Chỉ báo lao xuống sâu (*bán mạnh*), nhưng giá lại không giảm tương ứng -> Chứng tỏ bên mua hấp thụ, chống đỡ cực tốt. **Định lý QMV**: *"Cái gì bán mạnh mà giá không xuống thì khả năng cao nó phải đi lên!"* (`v3_3_bg`, `v3_3_hd`).
  - **Bản chất Phân kỳ Âm (ở Đỉnh – Canh Bán)**: Giá rướn lên đỉnh mới nhưng chỉ báo tụt xuống thấp hơn -> Sức mua thực chất đã hụt hơi (chỉ là nỗ lực cố kéo giá trụ) hoặc bị lực bán đè mạnh. **Định lý QMV**: *"Cái gì mua mạnh mà giá không lên nổi thì khả năng cao nó phải đi xuống!"* (`v1_3`, `v3_3_bg`, `v3_3_hd`).
- Ngay cả khi chưa ngược chiều hoàn toàn, chỉ cần **độ dốc (slope)** giữa đường Giá và đường Chỉ báo vênh nhau mạnh (ví dụ đáy giá dốc nhẹ/đi ngang mà đáy chỉ báo dốc đứng, hoặc ngược lại) đã là **sự bất bình thường** — mà trong PTKT, *"bình thường không phải cơ hội, bất bình thường mới là cơ hội"* (`v3_3_bg`, `v3_3_hd`).

### 3.2. Ma trận 4 loại Phân kỳ: Thường (Regular) vs Ẩn (Hidden) (`v3_3_bg`, `v3_3_hd`)
Giảng viên quy ước cực kỳ dễ nhớ: **Dương = Tìm ở ĐÁY (so các ĐÁY với nhau) để MUA**; **Âm = Tìm ở ĐỈNH (so các ĐỈNH với nhau) để BÁN** (`v3_3_hd`):

| Loại Phân kỳ | Vị trí so sánh | Hình thái Đường Giá | Hình thái Chỉ báo (RSI/CCI/MACD/Stoch) | Ý nghĩa xu hướng & Đặc điểm thực chiến QMV |
| :--- | :--- | :--- | :--- | :--- |
| **1. Phân kỳ Dương Thường** (*Regular Bullish*) | **So 2 Đáy** (trong nhịp giảm) | Đáy sau **thấp hơn** (hoặc ngang bằng) đáy trước | Đáy sau **cao hơn** (hoặc ngang bằng khi giá thấp hơn) đáy trước | Cảnh báo **ĐẢO CHIỀU** từ Giảm sang Tăng (`v1_3`, `v3_3_hd`). |
| **2. Phân kỳ Dương Ẩn** (*Hidden Bullish*) | **So 2 Đáy** (trong xu thế đã Tăng) | Đáy sau **cao hơn** đáy trước | Đáy sau **thấp hơn** (hoặc ngang bằng) đáy trước | Báo hiệu **TIẾP DIỄN** đà tăng (dành cho người vào muộn ăn nốt một nhịp). **Lưu ý QMV**: Vì xuất hiện khi giá đã tăng một quãng dài, biên độ ăn nhịp này **thường ngắn hơn** và có thể là nhịp tăng cuối trước chu kỳ giảm (`v3_3_bg`, `v3_3_hd`)! |
| **3. Phân kỳ Âm Thường** (*Regular Bearish*) | **So 2 Đỉnh** (trong nhịp tăng) | Đỉnh sau **cao hơn** (hoặc ngang bằng) đỉnh trước | Đỉnh sau **thấp hơn** đỉnh trước | Cảnh báo **ĐẢO CHIỀU** từ Tăng sang Giảm (`v1_3`, `v2_3_3`, `v3_3_bg`, `v3_3_hd`). |
| **4. Phân kỳ Âm Ẩn** (*Hidden Bearish*) | **So 2 Đỉnh** (trong xu thế Giảm) | Đỉnh sau **thấp hơn** đỉnh trước | Đỉnh sau **cao hơn** đỉnh trước | Báo hiệu **TIẾP DIỄN** xu thế giảm -> Canh bán thoát hàng (`v3_3_hd`). |

### 3.3. Quy tắc đếm & nối Đỉnh/Đáy trong Phân kỳ (`v1_3`, `v2_3_4`, `v3_3_bg`, `v3_3_hd`)
- **Cần mấy đỉnh/đáy?**: Cần tối thiểu **2 đỉnh** (phân kỳ âm) hoặc **2 đáy** (phân kỳ dương). Nếu so 2 đỉnh/đáy ngắn hạn sát nhau thì chỉ cảnh báo nhịp chỉnh/hồi ngắn hạn (`v2_3_4`); nếu so với đỉnh/đáy lớn tại vùng kháng cự/hỗ trợ trọng yếu trong quá khứ thì cảnh báo đảo chiều trung/dài hạn (`v1_3`, `v3_3_bg`). *(Kết hợp thêm quy luật tâm lý QMV ở `v1_3`: những biến cố phá vỡ/đảo chiều lớn nhất thường rơi vào **điểm chạm/đáy số 3**, tương ứng 3 lớp nhà đầu tư quá sợ hãi – trung bình – quá tự tin).*
- **Nối theo giá nào? (`v1_3` – Q&A Trường Tuấn)**: Ưu tiên nối theo **Giá Cao nhất (High) và Giá Thấp nhất (Low)** trong phiên thay vì Giá Đóng cửa, để phản ánh trọn vẹn cả các pha phản ứng thái quá trong ngày. Trường hợp 2 đỉnh/đáy của giá **ngang bằng nhau** mà chỉ báo dốc lên/xuống ngược chiều vẫn được tính là phân kỳ (`v1_3`, `v3_3_hd`).

### 3.4. Tại sao Phân kỳ chỉ là CẢNH BÁO chứ chưa phải lệnh Mua/Bán ngay? Điều kiện xác nhận Điểm uốn Phân kỳ!
1. **Bẫy "Kết luận phân kỳ khi chưa tạo đỉnh/đáy" (`v1_3`, `v3_3_bg`)**:
   - Giảng viên cảnh báo nghiêm khắc: Khi giá đang tăng dốc thẳng đứng, thấy RSI thấp hơn đỉnh cũ mà vội kết luận "Phân kỳ âm" để bán là **SAI**! Vì nếu giá cứ tăng tiếp kéo RSI vượt đỉnh cũ luôn thì phân kỳ bị xóa sổ hoàn toàn (`v3_3_bg`).
   - **Điều kiện cần để hình thành điểm uốn phân kỳ – Giá phải "Lình sình" (`v1_3`, `v3_3_bg`, `v3_3_hd`)**:
     - Giá phải chững lại, **lình sình lưỡng lự ít nhất vài ba phiên** (1 cây nến lưỡng lự chưa nói lên điều gì, phải 2–3 phiên trở lên mà chỉ báo không lên nổi mới bắt đầu nguy hiểm - `v1_3`).
     - **Cách xác định định lượng "giá lình sình" (`v3_3_bg`)**: Kẻ đường xu hướng (trendline) ngắn hạn của đường giá; chừng nào giá còn nằm trên trendline tăng thì chưa lình sình — giá phải **cắt lùi ra khỏi trendline tăng ngắn hạn** (bắt đầu đi ngang/trúc xuống) đồng thời chỉ báo tạo **điểm uốn** vòng xuống! (Muốn soi điểm uốn phân kỳ sớm hơn ở đáy trong phiên có thể mở đồ thị **30 phút** như ví dụ ngày 26/03 ở `v1_3`).
2. **Bốn điều kiện hội tụ xác nhận trước khi hành động (`v1_3`, `v2_3_4`, `v3_3_bg`, `v3_3_hd`)**:
   - Phân kỳ mới chỉ là "đèn vàng" nhắc theo dõi sát. Tuyệt đối không nhảy vào mua/bán ngay nếu các yếu tố khác bình thường (*"chưa có gì bất thường rõ ràng thì cứ để nó chạy"* - `v3_3_hd`). Chỉ hành động khi kiểm tra thêm:
     1. **Vùng Kháng cự / Hỗ trợ mạnh & Xu thế lớn**: Phân kỳ xuất hiện ngay tại Hỗ trợ/Kháng cự mạnh (`v3_3_hd`), và thuận xu thế lớn (ví dụ Phân kỳ dương xuất hiện trong nhịp chỉnh của Uptrend hoặc cuối pha Sideways sau Uptrend thì bắt đáy rất nhạy - `v1_3`).
     2. **Xác nhận từ Khối lượng (VPA)**: Ví dụ ở đỉnh có dấu hiệu phân kỳ âm, nếu giá lình sình/rướn lên mà **khối lượng tăng vọt** (xả mạnh không lên được) thì xác nhận rủi ro giảm cao; còn nếu giá giữ nền mà vol thấp (bên bán đã ngừng bán) thì chưa giảm ngay (`v2_3_4`).
     3. **Xác nhận từ Nến & Mẫu hình**: Xuất hiện cụm nến đảo chiều, gãy trendline giá/chỉ báo, hoặc kết hợp mẫu hình giá (`v1_3`, `v3_3_bg`, `v3_3_hd`).
     4. **Đồng thuận đa chỉ báo & Chia tiền 3 phần**: Cả **RSI + CCI** (hoặc thêm MACD cắt lên / BB thắt nút cổ chai) cùng xác nhận (`v3_3_hd`); nếu chưa chắc chắn 100%, áp dụng kỷ luật an toàn QMV: **chia vị thế làm 3 phần** giải ngân hoặc chốt lời từng bước (`v3_3_bg`).

---

## PHẦN 4: TOÀN BỘ VÍ DỤ THỰC TẾ VÀ GIẢI ĐÁP THẮC MẮC HỌC VIÊN TRONG `v3_3_hd` (KÈM ĐỐI CHIẾU VÍ DỤ & Q&A Ở `v1_3`, `v2_3_4`)

### 4.1. Chi tiết toàn bộ Case Studies & Giải đáp từng câu hỏi học viên trong `v3_3_hd`

#### A. Hai ví dụ cổ phiếu thực chiến giải phẫu Phân kỳ trong `v3_3_hd`
1. **Case Study 1 – Cổ phiếu SSI (`v3_3_hd`)**:
   - **Bối cảnh**: Giá SSI điều chỉnh đi xuống rồi tạo một vùng lình sình đi ngang tạo đáy.
   - **Phân tích**: So sánh vùng đáy lình sình hiện tại với vùng đáy quan trọng trước đó: **Đáy sau của giá SSI cao hơn đáy trước**, trong khi **đáy sau của chỉ báo lại thấp hơn đáy trước** -> Tạo **Phân kỳ Dương Ẩn (Hidden Bullish Divergence)**.
   - **Kết luận hành động**: Cho điểm mua đối với những ai bị lỡ nhịp tăng trước đó để "ăn nốt một đoạn tăng tiếp diễn"; tuy nhiên giảng viên lưu ý vì là phân kỳ dương ẩn (xuất hiện sau khi cổ phiếu đã tăng một nhịp lớn) nên dư địa thường ngắn hơn và có thể là đoạn cuối trước khi sang chu kỳ giảm. Ngoài ra thầy cũng chỉ thêm một đoạn ngắn hạn trên SSI nơi hai đáy giá sát ngang nhau nhưng chỉ báo hướng lên tạo phân kỳ dương thường cho lướt sóng ngắn.

2. **Case Study 2 – Cổ phiếu Hòa Phát (HPG) (`v3_3_hd`)**:
   - **Nhịp 1 (Giảm bình thường – Không phân kỳ)**: Trong xu thế giảm lớn của HPG, ở hai đáy đầu tiên, đáy sau của giá thấp hơn đáy trước và đáy sau của chỉ báo cũng thấp hơn đáy trước -> Cùng chiều bình thường, đứng ngoài.
   - **Nhịp 2 (Phân kỳ Dương Thường ở đáy đảo chiều)**: Tại cặp đáy kế tiếp sâu hơn, **đáy sau của giá HPG thấp hơn đáy trước**, nhưng **đáy sau của chỉ báo lại cao hơn một chút (hoặc ngang bằng)** -> Hình thành **Phân kỳ Dương Thường (Regular Bullish Divergence)**, cảnh báo đảo chiều xu hướng giảm sang nhịp tăng mới.
   - **Nhịp 3 (Phân kỳ Dương Ẩn tiếp diễn xu hướng tăng & Sự khác biệt giữa các chỉ báo)**: Khi HPG đã vào nhịp tăng, tại đáy điều chỉnh tiếp theo, **đáy sau của giá cao hơn đáy trước**, trên **Stochastic** hai đáy ngang bằng nhau (tạo **Phân kỳ dương ẩn** báo hiệu tăng tiếp một đoạn cuối), trong khi trên **CCI** thì tín hiệu không rõ ràng.
   - **Nhịp 4 (Tổ hợp BB Squeeze + Phân kỳ + MACD)**: Trên đồ thị minh họa trả lời bạn Lưu Hà, thầy chỉ ra vùng dải **Bollinger Bands thu hẹp** kết hợp **Phân kỳ dương ở đáy** và **đường MACD cắt lên** -> Hội tụ 3 tín hiệu củng cố niềm tin giá sẽ bùng nổ đi lên!

#### B. Tổng hợp đầy đủ 100% câu hỏi của từng học viên và lời giải của giảng viên trong `v3_3_hd`
1. **Đỗ Anh Tùng, Nga Nguyễn & Nguyễn Thu Phương – Hỏi về cách hiểu và phân biệt Phân kỳ âm / Phân kỳ dương**:
   - **Giải đáp**: Thầy lập bảng tổng kết 4 dạng (Dương thường, Dương ẩn, Âm thường, Âm ẩn) và rút gọn cốt lõi để học viên không bị rối: Cứ giá và chỉ báo không cùng chiều là Phân kỳ; xét ở **Đáy** (so các đáy) để canh **Mua** gọi là **Phân kỳ Dương**; xét ở **Đỉnh** (so các đỉnh) để canh **Bán** gọi là **Phân kỳ Âm**. Thường = Đảo chiều xu hướng cũ; Ẩn = Tiếp diễn xu hướng hiện tại thêm một nhịp.
2. **Đỗ Anh Tùng (Câu hỏi 2) – *"Khi xét phân kỳ mình nhìn cả 2 chỉ số CCI và RSI hay nhìn như thế nào?"***:
   - **Giải đáp**: Thầy quen dùng bộ đôi **RSI và CCI**. Nếu thấy phân kỳ đồng thời trên **cả 2 chỉ số** thì độ tin cậy cao hơn hẳn; nếu chỉ thấy trên 1 chỉ số thì độ tin cậy thấp hơn.
3. **Trung Kiên (và Nguyễn Thu Phương) – *"Thầy nói rõ hơn về 36 mẫu hình kỹ thuật nhắc trong bài giảng?"***:
   - **Giải đáp**: Bộ 36 mẫu hình thầy đã đăng trên Facebook Group QMV từ đầu năm (sẽ đăng lại vào thư viện) và sẽ học chi tiết ngay ở buổi học tiếp theo (Chủ đề 4: Mẫu hình kỹ thuật).
4. **Lưu Hà (Ý 1) – *"BB hẹp báo hiệu điều quan trọng sắp xảy ra — theo kinh nghiệm thầy đó thường là gì? BB hẹp thời gian dài hàm ý gì? BB rộng có hàm ý gì không?"***:
   - **Giải đáp**: BB hẹp cho thấy hai phe mua – bán giằng co cân bằng, lưỡng lự; chắc chắn sắp có biến động mạnh nhưng **chỉ nhìn BB thì không biết tăng hay giảm**, bắt buộc phải ghép thêm Phân kỳ RSI/CCI hoặc MACD cắt lên để đoán hướng nổ. Còn **BB rộng** không mang nhiều ý nghĩa dự báo vì nó chỉ rộng ra khi đã có xu hướng (chừng nào còn mở rộng là xu hướng còn khỏe; đang rộng mà co hẹp lại là xu hướng cũ chững lại chuẩn bị đổi pha).
5. **Lưu Hà (Ý 2) – *"Độ dốc / sự bất bình thường của đường RSI khác bình thường thế nào?"***:
   - **Giải đáp**: Độ dốc đo sự tương thích giữa đường giá và chỉ số; lệch nhau mạnh nhất chính là Phân kỳ. Gốc rễ là quy luật cung cầu: *bán nhiều mà giá không giảm thì khả năng tăng cao; mua nhiều mà giá không tăng thì khả năng giảm cao*.
6. **Hiệp – *"Em hay dùng Giá, Khối lượng và quy luật Cung cầu. Có nên kết hợp thêm chỉ số kỹ thuật không và thầy hay dùng chỉ báo/phương pháp nào nhất?"***:
   - **Giải đáp**: Giá – Khối lượng – Cung cầu là cốt lõi số 1, nhưng **nên kết hợp thêm** chỉ số kỹ thuật để có thêm thông tin xác thực. Thầy dùng kết hợp tổng thể (Cung cầu VPA + Kháng cự/Hỗ trợ + Vùng quá mua/bán + Phân kỳ trên **RSI & CCI**). Khi thực hành đủ nhiều, chỉ cần **liếc mắt vài giây** là tổng hợp xong toàn bộ bức tranh.
7. **Đỗ Anh Tùng (Câu hỏi 3) – *"Trong nhóm trung bình động (MA, BB, Ichimoku), thầy hay dùng cái nào nhất và cái nào hiệu quả nhất để em tập trung nghiên cứu?"***:
   - **Giải đáp**: Thầy hay dùng **MA** và **Bollinger Bands** — thậm chí chỉ cần bật **duy nhất Bollinger Bands** trên đồ thị giá là đủ (vì trục giữa BB đã là đường MA rồi), giúp đồ thị thoáng mắt không bị rối; bên dưới bật **RSI và CCI**. Ai thích chuyên sâu riêng đám mây Ichimoku cũng không sao.
8. **Nguyễn Thu Phương (Câu hỏi 2) – *"Các chỉ số di chuyển tương tự nhau, vậy nên chọn chỉ số như thế nào?"***:
   - **Giải đáp**: Chọn 2–3 chỉ số mà mình thấy thân thuộc, hiểu rõ bản chất nhất (như thầy chọn RSI & CCI; học viên có thể chọn MACD, Williams %R, CMF...), đem kiểm chứng trên dữ liệu quá khứ nhiều cổ phiếu. Khi thấy ổn định thì biến nó thành **nguyên tắc cố định của riêng mình**, không cần nhìn ngó xem người khác dùng chỉ báo gì.
9. **Đỗ Anh Tùng (Câu hỏi 4) – *"Khi phát hiện phân kỳ dương/âm trên CCI hoặc RSI thì bao lâu sẽ xảy ra tăng/giảm?"***:
   - **Giải đáp**: Không có mốc thời gian cố định ngay lập tức vì phân kỳ mới chỉ là **tín hiệu cảnh báo để quan sát kỹ**, tuyệt đối **cấm nhảy vào mua/bán ngay** khi mới chớm thấy phân kỳ! Phải kiểm tra tổ hợp: (1) Có nằm ở **Hỗ trợ/Kháng cự mạnh** không? (2) **Khối lượng** lúc đó thế nào? (3) **Nến** phản ứng ra sao? Nếu chưa có xác nhận bất thường từ các yếu tố đó thì giữ nguyên tắc: *"Cứ để nó chạy (let it run), nhưng phải quan sát sát sao"*.

### 4.2. Bổ sung các Case Studies thực chiến VN-Index & Hỏi đáp đặc sắc từ `v1_3` và `v2_3_4`
Để bức tranh HP4-3 không sót bất kỳ chi tiết thực chiến nào từ các phiên bản V1 và V2:
- **Case Study VN-Index vượt đỉnh lịch sử với Mẫu hình Phễu ngang mở rộng (`v1_3`, `v2_3_3`, `v2_3_4`)**:
  - Giảng viên kết hợp đường xu hướng, mẫu hình **Phễu ngang mở rộng** (~180 điểm biên độ -> cách đo target của dân trading trong ngày là cộng thêm đúng 180 điểm từ điểm breakout, xuất hiện cú rũ nảy lại đúng tại **điểm chạm số 3**), nến Doji thân ngắn (tăng chắc / chững lại kiểm tra cung cầu), Ichimoku (giá trên mây, mây chưa thu hẹp, Tenkan trên Kijun), và dòng tiền CMF + tiềm năng Phân kỳ âm RSI ở sóng 5 Elliott.
  - Phối hợp Vĩ mô & Vi mô ngành (`v1_3`, `v2_3_4`): PTKT không bao giờ thay thế được Phân tích Kinh tế & Chiến lược Thị trường! Thầy phân tích thêm Thông tư/Chỉ thị 03 của NHNN về trích lập dự phòng nợ Covid (ảnh hưởng lợi nhuận các ngân hàng nhỏ báo lãi ảo năm 2020) và kỳ vọng xử lý nghẽn lệnh HoSE để cảnh báo chu kỳ thận trọng/tiêu cực trong 2 tháng tới (hết Quý 2), dù ngắn hạn 1 tuần tới trên đồ thị kỹ thuật chưa gãy thì vẫn *"let it run"*.
- **Các câu hỏi thực chiến cực hay ở cuối `v1_3` (không có trong `v3_3_hd`)**:
  1. **Bạn Luis hỏi về nghẽn lệnh HoSE 16.000 tỷ làm méo chỉ số dòng tiền**: Thầy xác nhận khi hệ thống bị kẹt lệnh cơ học thì chỉ báo dòng tiền/PTKT bị méo mó, chỉ dùng tương đối.
  2. **Bạn Trường DX & câu hỏi về Cổ phiếu Trụ vs Cổ phiếu Thanh khoản thấp vs "Cổ phiếu Trà đá"**:
     - Cổ phiếu thanh khoản quá thấp hoặc mới lên sàn -> Không áp dụng tốt PTKT.
     - **Cổ phiếu Trụ (vốn hóa lớn)** -> Khối lượng giao dịch thực tế không nhiều mà bị chi phối bởi tổ chức -> Dùng PTKT **chưa chắc đã chính xác**!
     - **Cổ phiếu đại chúng, khối lượng giao dịch lớn (số lượng cổ phiếu khớp quan trọng hơn giá trị tiền), đặc biệt là "Cổ phiếu trà đá / cổ phiếu đầu cơ đánh đấm nhiều"** -> Phản ánh tâm lý đám đông rõ nhất nên **áp dụng PTKT lại cực kỳ hiệu quả** (vào trễ hơn đáy một chút, ra sớm hơn đỉnh một chút là thắng lớn!).
  3. **Bạn Duy Thiên hỏi Sóng Elliott, Fibonacci, Wyckoff thuộc nhóm chỉ báo nào?**: Thầy làm rõ đó là các **lý thuyết cấu trúc chu kỳ / luân chuyển lớp cổ phiếu** (thậm chí Fibonacci Retracement 38.2% - 50% - 61.8% bản chất cũng là ước lượng chia 100% thành **3 phần tâm lý**!), không phải là nhóm chỉ số kỹ thuật tính trực tiếp bằng công thức từ Giá & Lượng.