# 06. MASTER PROMPT KHỞI CHẠY PHIÊN PHÂN TÍCH, VIẾT SPEC, LẬP PLAN & IMPLEMENT TRÊN SUMI

> **Hướng dẫn sử dụng cho bạn:**
> Master Prompt dưới đây **ĐÃ BAO GỒM TRỌN VẸN CẢ 06 Ý TƯỞNG WORKSTATION (`M1..M5`) LẪN Ý TƯỞNG THỨ 07 LÀ `SUMI AGENT` (`M6..M7`)** thành một chỉnh thể thống nhất (để ngay từ Bước viết Spec, cấu trúc dữ liệu của `M1..M5` đã khớp sẵn 100% với 14 MCP tools của `Sumi Agent` ở `M6..M7`, không bao giờ phải đập đi sửa lại).
> - **Cách chạy khuyến nghị 1 (Kiểm soát chặt từng chặng):** Mở Session mới, gõ `/plan` kèm nội dung khung dưới để Agent viết trọn bộ `SUMI_QMV_V2_TECH_SPEC.md` + Kế hoạch `M1 -> M7`. Sau khi duyệt Plan, bạn có thể cho chạy theo 3 chặng gọn gàng: **Chặng A (`M1–M3`: Data, Indicators, Co-Pilot, Fox DNA & 9-Ball Radar)** $\rightarrow$ **Chặng B (`M4–M5`: Backtest Đa Lô `T+2`, Đảo Kho 6 Phần & Phòng Tập Dojo)** $\rightarrow$ **Chặng C (`M6–M7`: `Sumi Agent` ACP, Skills, Tự Luyện & Đấu Trường So Găng)**.
> - **Cách chạy khuyến nghị 2 (Tự động hóa sâu toàn trình):** Mở Session mới và dùng lệnh `/goal` (hoặc `/teamwork`) kèm Master Prompt dưới đây để Agent tự động triển khai và kiểm thử cuốn chiếu xuyên suốt từ `M1` đến `M7`.

---

```markdown
Bạn đóng vai **Principal Quantitative Systems Architect, Lead Rust/TypeScript Engineer & QMV Trading Workstation Designer**.

Nhiệm vụ của chúng ta là **nâng cấp toàn diện ứng dụng Sumi** (`/Users/mizuhara/workspace/sumi`) thành **Trạm Phân Tích, Tầm Soát Thực Chiến Hàng Ngày, Lò Luyện/Kiểm Chứng Hệ Thống Định Lượng VÀ Cộng Sự AI Tự Tiến Hóa Chuẩn 100% Phương Pháp QMV (`Sumi QMV Workstation v2.0 + Sumi Agent`)**.

---

## 1. BẮT BUỘC NẠP ĐẦY ĐỦ KHO TRI THỨC & ĐẶC TẢ TRƯỚC KHI HÀNH ĐỘNG

Toàn bộ tri thức bài giảng gốc QMV, công thức toán học chỉ báo, chiến lược giao dịch, kiểm toán dữ liệu, tài liệu yêu cầu nghiệp vụ (BRD) và thiết kế kiến trúc `Sumi Agent` đã được nghiên cứu sâu và lưu trữ sẵn ngay trong Sumi. Trước khi đề xuất thiết kế kỹ thuật hay sửa bất kỳ dòng code nào, bạn **BẮT BUỘC đọc kỹ bộ tài liệu sau**:

1. **Cổng Chỉ Mục Trung Tâm:**
   - `/Users/mizuhara/workspace/sumi/docs/qmv_research/00_QMV_KNOWLEDGE_MASTER_INDEX.md`
2. **Tài liệu Yêu cầu Nghiệp vụ Workstation (`M1..M5`) & Đặc tả Kiến trúc `Sumi Agent` (`M6..M7`):**
   - `/Users/mizuhara/workspace/sumi/docs/qmv_research/05_SUMI_QMV_WORKSTATION_BRD.md`
   - `/Users/mizuhara/workspace/sumi/docs/qmv_research/07_SUMI_AGENT_ARCHITECTURE_AND_SPEC.md`
   *(Tham khảo thêm cách triển khai `agy acp` chuẩn OpenMausBot đã vận hành thực tế tại `/Users/mizuhara/workspace/yumeko/crates/yumeko-agent-acp/src/lib.rs` ở chế độ Read-Only).*
3. **Đối chiếu Khoảng cách với Mã nguồn Sumi Hiện tại & 06 Bài kiểm thử Bất biến:**
   - `/Users/mizuhara/workspace/sumi/docs/qmv_research/04_SUMI_GAP_ANALYSIS_AND_IMPLEMENTATION_ROADMAP.md`
4. **Đặc tả Toán học 07 Chỉ báo Độc quyền QMV & 08 Chiến lược Chuẩn QMV (`S1..S8`) + Học thuyết Gốc:**
   - `/Users/mizuhara/workspace/sumi/docs/qmv_research/03_QMV_CUSTOM_INDICATORS_SPECIFICATION.md`
   - `/Users/mizuhara/workspace/sumi/docs/qmv_research/02_QMV_TRADING_STRATEGIES_PLAYBOOK.md`
   - `/Users/mizuhara/workspace/sumi/docs/qmv_research/01_QMV_TECHNICAL_ANALYSIS_COMPLETE_DOCTRINE.md`
   *(Khi cần tra cứu ví dụ thực tế hoặc quy tắc biên chi tiết của giảng viên QMV cho bất kỳ module/skill nào, hãy đọc thêm các tệp tương ứng trong `/Users/mizuhara/workspace/sumi/docs/qmv_research/lecture_digests/01..15*.md`).*

---

## 2. QUY TẮC KIẾN TRÚC & RÀNG BUỘC KỸ THUẬT BẤT BIẾN

1. **Tuyệt đối Read-Only với `/Users/mizuhara/workspace/yumeko`:**
   - Không sửa đổi, tạo mới hay xóa bất kỳ file/bảng dữ liệu nào bên trong Yumeko.
   - Khi xây dựng `YumekoReadOnlyBridge` trong `sumi-data` để đọc dữ liệu Vĩ mô `VM01`, Phân nhóm `Division`, Chỉ số Tài chính/`M-Score` và dữ liệu hiệu chỉnh QMV Blackbox từ `/Users/mizuhara/workspace/yumeko/data/qmv-shared.sqlite` & `~/.local/share/yumeko/yumeko-app.sqlite` (hoặc gói xuất từ `research_export`), bắt buộc mở kết nối với `SQLITE_OPEN_READ_ONLY` + `PRAGMA query_only = ON`, đồng thời đảm bảo **Graceful Standalone Fallback** (nếu không tìm thấy DB Yumeko, Sumi vẫn chạy mượt 100% bằng kho `sumi_market.db` và các thuật toán `OHLCV Proxy`).
2. **Bảo toàn Kiến trúc Lõi của Sumi (`wasm32`-clean, $O(N)$, `DEF-001` Causal):**
   - `crates/sumi-core` phải giữ nguyên tính chất thuần tính toán (`f64`/`i64`, không I/O, không `tokio`, biên dịch sạch cho `wasm32-unknown-unknown`).
   - Mọi chỉ báo, bộ đếm 3 chạm, sơ đồ nút `0..8`, `Z50_Safe`, `Fox DNA`, Bàn cờ 9 Quả Bóng và Sổ cái Đa Lô `T+2` phải **100% nhân quả (`Causal <= t`)** — kết quả tại phiên `t` trên lát cắt `candles[..=t]` phải trùng khớp từng bit với phần tử `t` khi tính trên toàn chuỗi `candles[..N]`, đảm bảo khi tua `Bar Replay` không bao giờ rò rỉ tương lai.
   - Tuân thủ nghiêm vi cấu trúc TTCK Việt Nam (`HOSE ±7% / HNX ±10% / UPCOM ±15%`, bước giá `10/50/100đ`, khóa thanh khoản Trần/Sàn cứng, lô `100`, mốc chuyển đổi lịch sử `2022-08-29` giữa `T+3` và `T+2`, phí `0.15%`, thuế bán `0.10%`).
3. **Giao diện Tinh gọn, Chuẩn Thẩm mỹ Workstation Chuyên nghiệp:**
   - Tôn trọng triết lý *"Đồ thị tối giản, sạch mắt"* của QMV: không bật chồng chéo mọi chỉ báo cùng lúc mà đóng gói thành các `Chart Templates` và tách thông tin chẩn đoán sang `QMV Co-Pilot Sidebar` (`RightWidgetBar`), `Top-Down 9-Ball Radar` (`StockScreenerTab`), `Dojo Debrief` (`ReplayControlBar` + `PaperBrokerTab`) và `Portfolio Lab` (`StrategyTesterTab`).

---

## 3. QUY TRÌNH LÀM VIỆC 4 BƯỚC BẮT BUỘC TRONG SESSION NÀY

Hãy thực hiện tuần tự, bài bản theo đúng 4 bước kỹ nghệ phần mềm sau:

### Bước 1 — Khảo sát Codebase Hiện tại & Viết Tài liệu Đặc tả Kỹ thuật Chi tiết (`Technical Specification`)
- Khảo sát kỹ cấu trúc hiện tại của `crates/sumi-core`, `crates/sumi-data`, `crates/sumi-desktop` (Tauri IPC commands & state) và `ui/src` (Zustand store, KLineChart v10 overlays/indicators, Dock tabs, Sidebar), đồng thời đối chiếu mẫu `agy acp` tại `/Users/mizuhara/workspace/yumeko/crates/yumeko-agent-acp/src/lib.rs` (Read-Only).
- Viết bản **Technical Specification tổng thể** lưu tại `/Users/mizuhara/workspace/sumi/docs/specs/SUMI_QMV_V2_TECH_SPEC.md`, định nghĩa rõ cho **cả 6 Ý tưởng Workstation (`M1..M5`) lẫn `Sumi Agent` (`M6..M7`)**:
  - Schema SQLite mở rộng trong `sumi_market.db`, `sumi_workspace.db` và `sumi_agent_memory.sqlite` (giá `Raw`, cache `Fox DNA`, cache đồng bộ Read-Only từ Yumeko, phiên tập `Dojo`, chỉ mục FTS5 bài giảng QMV, sổ cái Quyết định/Kết quả/Bài học `PIT-gated` và `.skill_ledger.jsonl`).
  - Các `struct`, `enum`, `trait` và chữ ký hàm chính xác trong `sumi-core`, `sumi-data` và 3 crate mới (`sumi-agent-memory`, `sumi-mcp` với 14 công cụ `< 6 KB`, `sumi-agent-acp` chuẩn OpenMausBot/Yumeko) cùng cấu trúc thư mục `agent_pack/` (`CONSTITUTION.md`, `steering/`, `skills/canonical/`, `skills/learned/`).
  - Danh sách Tauri commands mới/nâng cấp trong `sumi-desktop` (kèm web fallback trong `ui/src/api/webFallback.ts`).
  - Thiết kế state và kiến trúc component chi tiết trong `ui/src/` (7 Custom Indicators + 3 Templates, `9-Ball Radar`, `QMV Co-Pilot & Fox DNA Sidebar`, `1-Click Trade Planner`, `Trading Dojo`, `Multi-Tranche Portfolio Lab` và `Sumi Agent Drawer ⌘J` + `Human-vs-Agent Arena`).

### Bước 2 — Lập Kế hoạch Triển khai Phân kỳ (`Implementation Plan`) theo 7 Milestones (`M1 -> M7`)
- Lập kế hoạch triển khai chi tiết chia thành **7 Milestones độc lập, kiểm thử được ngay sau mỗi chặng** (bám sát `05_SUMI_QMV_WORKSTATION_BRD.md` cho `M1..M5` và `07_SUMI_AGENT_ARCHITECTURE_AND_SPEC.md` cho `M6..M7`):
  - **`M1` — Hạ tầng Dữ liệu & Cầu nối Read-Only Yumeko:** Sửa ánh xạ `Timeframe::M30` & `MN1`, bật nạp `CafeF.SolieuGD.Raw` (`Unadjusted`), công cụ Pre-warm `15m/30m/1W` cho rổ trọng điểm, và `YumekoReadOnlyBridge` (nạp `macro_pit`, `qmv_division_daily`, `fundamentals_pit` tính cổng `"Không được chết"` + `M-Score`, và dữ liệu hiệu chỉnh `Blackbox`).
  - **`M2` — Bộ Chỉ báo Cấp Cổ phiếu, Hồ sơ `Fox DNA` & `QMV Chart Co-Pilot`:** Triển khai `QMV_MERGED_VPA` (sửa lỗi nến Trần/Sàn, quy tắc `1/3`, ghép nến động, `SMA5 Vol`, chuỗi `T+3`, Cây thông Noel), `QMV_ZONE_3TOUCH`, `QMV_SMART_MONEY_CYCLE`, `QMV_REGIME_RIBBON_DIV`, `QMV_BLACKBOX_PROXY` (chống bẫy `Z-50`, `TMC/DSPI/MPICP/SP`, `0–5★ + Tốt hơn`), dịch vụ `FoxDnaProfiler`, 3 `Chart Templates` và tab `"QMV Co-Pilot"` trên `RightWidgetBar`.
  - **`M3` — Trạm Radar Top-Down, Nhiệt kế Độ rộng & Bàn cờ 9 Quả Bóng:** Triển khai `QMV_MARKET_BREADTH_BB` (`BBS20/50/200` + cảnh báo `BBS200 > 80%`), `QMV_9BALL_ROTATION` (ma trận `3×3` + cảnh báo Đỉnh `Trà đá` & `"Đổi gác" DT3 -> DT1`), lọc cổ phiếu Bo cung, và nâng cấp giao diện `StockScreenerTab`.
  - **`M4` — Sổ cái Backtest Đa Lô `T+2`, 08 Playbook `S1..S8` & Đảo Kho 6 Phần:** Nâng cấp `sumi-core::backtest` lên `Per-Tranche T+2 Lot Ledger` (`1/3–1/3–1/3`, `Cắt 1/2 cứu hộ`, cổng `RR > 1.0`, `T+3 Time-Stop`), cài đặt đủ 8 chiến lược `S1..S8`, trình giả lập `Đảo Kho 6 Phần`, bộ bật/tắt `Ablation` và nâng cấp `StrategyTesterTab`.
  - **`M5` — Trình Lập Kế Hoạch Lệnh 1-Click & Phòng Tập Mù `QMV Trading Dojo`:** Nâng cấp khung `Long R:R` kiểm cổng `RR > 1.0` + chia 3 lô `1/3` + nhắc chuông `T+3`, xây dựng bộ sinh đề thi lịch sử mù (`Scenario Packs`) và **Giám khảo Chấm điểm Kỷ luật QMV (`0–100đ`)** kết nối `ReplayControlBar` + `PaperBrokerTab`.
  - **`M6` — `Sumi Agent` Core (`agy acp` chuẩn OpenMausBot/Yumeko, Progressive `SKILL.md` & Chat Co-Pilot Vẽ Lên Chart):** Xây dựng 3 crate `sumi-agent-acp` (`oauth-personal`, đa model, `session/resume`, cô lập stream, Fail-Closed `PermissionPolicy`), `sumi-agent-memory` (FTS5 + 9 `Canonical QMV Skills` + Context Steering), `sumi-mcp` (chẩn đoán 6 bước, xếp hạng chiến lược `S1..S8` theo `Fox DNA`, đề xuất & vẽ kịch bản `RR > 1` trực tiếp lên biểu đồ, tự động Backtest/WFO) và ngăn trượt `Sumi Agent Drawer` (`⌘J`).
  - **`M7` — Tự Thực Hành Dojo, Đấu Trường `Bạn vs. Sumi Agent` & Tự Tiến Hóa Kỹ Năng Qua Cổng `WFO`:** Hoàn thiện vòng lặp tự giải đề Dojo mù cho Agent, bảng **Đấu trường Đối chiếu `Bạn vs. Sumi Agent`** (so sánh Backtest & giải đề Dojo song song trên cùng biểu đồ), và cơ chế tự đúc kết `Learned Skills` bắt buộc vượt cổng kiểm định `WFO 5-Fold` kèm sổ cái `skill_ledger.jsonl` hoàn tác 1 chạm.
- **Trình bày bản Tech Spec + Implementation Plan để tôi xem xét xác nhận** (hoặc nếu tôi yêu cầu chạy thẳng thì triển khai lần lượt từng Milestone `M1 -> M7`).

### Bước 3 — Thực thi Mã nguồn Sạch, Chặt chẽ Từng Milestone
- Viết code chuẩn mực, không để lại `TODO` hay mock giả lập hời hợt; giữ tương thích ngược với toàn bộ tính năng sẵn có của Sumi.

### Bước 4 — Kiểm thử Bất biến & Kiểm định Thực nghiệm
- Sau mỗi Milestone, chạy đầy đủ `cargo test --workspace`, kiểm tra `cargo check --target wasm32-unknown-unknown -p sumi-core`, và kiểm tra TypeScript/UI build (`npm run build` / `vitest` trong `ui/`).
- Bổ sung đầy đủ bộ kiểm thử bất biến cho cả Workstation lẫn Agent (`TEST-QMV-CAUSAL-PREFIX`, `TEST-QMV-VN-LIMIT-CANDLE`, `TEST-QMV-Z50-DEAD-SPAN`, `TEST-QMV-TRANCHE-T2-CUT-HALF`, `TEST-QMV-RR-AND-TIMESTOP`, `TEST-QMV-GUARD-CHANGE-CASH`, `TEST-QMV-ACP-FAIL-CLOSED-POLICY`, `TEST-QMV-AGENT-PIT-MEMORY`, `TEST-QMV-SKILL-WFO-GATE`) và chạy đối chiếu thực nghiệm trên `sumi_market.db`.
```
