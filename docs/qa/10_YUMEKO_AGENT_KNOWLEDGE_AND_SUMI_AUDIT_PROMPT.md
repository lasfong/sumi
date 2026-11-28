# MASTER PROMPT KIỂM TOÁN TOÀN DIỆN YUMEKO (`AGENT, TOOLS, SKILLS, KNOWLEDGE COMPOUNDING & SUMI BRIDGE`)

> **Hướng dẫn sử dụng:** Mở một phiên làm việc mới với Active Workspace là **`/Users/mizuhara/workspace/yumeko`** (khuyến nghị chạy kèm `/boost` hoặc `/goal` để Agent phân tích sâu đa luồng), sau đó dán nguyên văn khối Prompt dưới đây.

---

```markdown
Bạn đóng vai **Principal Agentic AI Systems Architect, Quantitative Research Platform Auditor & Knowledge Engineering Lead**.

Nhiệm vụ của bạn trong phiên này là thực hiện một cuộc **Kiểm toán Pháp y Toàn diện, Trung thực Tuyệt đối đến từng dòng mã nguồn & CSDL thực tế (`Forensic Code & Runtime Audit`)** đối với hệ thống **Yumeko (`v3.1.0`, 20 Rust crates + Tauri v2 + `apps/yumeko-web` + `knowledge_factory` + `memory` + `skills`)** tại `/Users/mizuhara/workspace/yumeko`, đồng thời đối chiếu mối nối với **Sumi Supercharts (`v3.0.0`)** tại `/Users/mizuhara/workspace/sumi`.

Bối cảnh: Trong quá trình phát triển và kiểm toán ứng dụng chị em **Sumi Supercharts** (đóng vai trò Bước 6 `KT01 Stock Confirmation Gate` trong quy trình 9 bước Top-Down QMV), chúng tôi phát hiện nhiều bài học xương máu về khoảng cách giữa **"kiến trúc trên tài liệu / unit test xanh"** và **"thực tế vận hành"** (cờ cấu hình chết, tính năng Rust viết xong nhưng trên UI hoặc CLI gọi hàm giả lập/template tĩnh, rò rỉ dữ liệu tương lai `Point-in-Time`, ghi đè mất dữ liệu khi đồng bộ đa máy Mac <-> Windows, tài liệu/skills lệch pha với code thật).

Tôi cần bạn kiểm tra xem **Yumeko hiện tại có thực sự vận hành như một Hệ điều hành Đầu tư Agentic có khả năng suy luận chuẩn QMV và tích lũy tri thức qua thời gian hay không**, hay cũng đang mắc các căn bệnh tương tự, đồng thời đối chiếu với **Best Practices thiết kế Agent/Tools/Skills và các kiến trúc GitHub Trending mới nhất (2025–2026)**.

---

## 1. NGUYÊN TẮC KIỂM TOÁN & RANH GIỚI AN TOÀN BẮT BUỘC (NON-NEGOTIABLE)

1. **Chế độ Chỉ Đọc (`100% Read-Only`) đối với Mã nguồn & Cơ sở dữ liệu:**
   - **KHÔNG sửa đổi bất kỳ tệp mã nguồn nào** trong `/Users/mizuhara/workspace/yumeko`, `/Users/mizuhara/workspace/sumi`, hay `/Users/mizuhara/workspace/Doraemi`.
   - Mọi truy vấn kiểm tra `/Users/mizuhara/workspace/yumeko/data/qmv-shared.sqlite` và `~/.local/share/yumeko/yumeko-app.sqlite` **BẮT BUỘC mở ở chế độ Chỉ Đọc** (`sqlite3 -readonly ...` hoặc `file:...?mode=ro` với `PRAGMA query_only = ON`).
2. **Bảo mật Tuyệt đối của Chủ sở hữu (`Owner Security Boundary`):**
   - **TUYỆT ĐỐI KHÔNG đọc, in, trích xuất hay chạm vào** `.env`, thư mục `secrets/`, OAuth tokens, Portainer, SSH, VPS hay R2 dưới bất kỳ hình thức nào.
3. **Kiểm chứng từ Code thật + Dữ liệu thật — KHÔNG chép lại từ tài liệu thiết kế:**
   - Mọi nhận định phải kèm dẫn chứng xác thực `file:line` và số lượng bản ghi thực tế trong SQLite / thư mục đĩa.
   - Không tô hồng: cái gì là template ghép chuỗi `format!()` trong Rust giả làm AI thì phải chỉ thẳng là template tĩnh; cái gì có bảng DB nhưng `0` dòng hoặc không bao giờ được đọc lại vào prompt thì phải chỉ thẳng là "bộ nhớ chết (`write-only graveyard`)".
4. **Đầu ra:** Trình bày báo cáo bằng **tiếng Việt** sắc bén, có bảng biểu đối chiếu trực quan, và chỉ ghi duy nhất 1 tệp báo cáo kiểm toán hoàn chỉnh tại `/Users/mizuhara/workspace/yumeko/docs/YUMEKO_AGENT_KNOWLEDGE_AND_SUMI_AUDIT_2026-11-28.md` (hoặc xuất Artifact nếu sandbox chặn ghi).

---

## 2. BẢY TRỤ CỘT BẮT BUỘC PHẢI ĐỌC CODE, ĐO ĐẠC & ĐỐI CHIẾU

Hãy chia các luồng nghiên cứu song song (`research` subagents) để kiểm tra tận gốc 7 trụ cột sau:

### Trụ cột 1 — Cơ chế Agent Thực tế vs. "Kịch bản Ghép chuỗi Tĩnh" (`Agent Runtime & ACP Orchestration`)
Đọc trực tiếp `crates/yumeko-agent-acp/src/lib.rs`, `crates/yumeko-app-service/src/`, `crates/yumeko-analysis/`, `crates/yumeko-report/`, `apps/yumeko-desktop/`, `apps/yumeko-web/`:
1. **Phân định rạch ròi "AI suy luận thật" vs. "Rust ghép chuỗi":** Lập bảng toàn bộ các tính năng mang danh Agent/AI trên UI và CLI của Yumeko (`Run 9-Step Daily Analysis`, `20-Part Report Generation`, `Scoped Ask / Chat Follow-up`, `Candidate Thesis`, `Portfolio Review`):
   - Tính năng nào **thực sự khởi chạy phiên `agy acp`**, truyền ngữ cảnh, gọi MCP tools và nhận suy luận từ mô hình ngôn ngữ?
   - Tính năng nào thực chất chỉ đang chạy hàm Rust nối chuỗi `format!()` (ví dụ kiểm tra kỹ `ask_scoped_followup` và các nhánh `local analytical fallback`) khiến người dùng tưởng đang hỏi đáp với Agent nhưng thực ra chỉ nhận văn mẫu cố định?
2. **Kiểm tra Lỗ hổng "Disposable SQLite Snapshot" trong `yumeko-agent-acp`:**
   - Theo `AGENTS.md`, `yumeko-agent-acp` khởi chạy `yumeko-mcp` trên một **bản chụp SQLite dùng một lần (`disposable SQLite snapshot`)**. Hãy truy vết từng dòng code trong `yumeko-agent-acp` và `yumeko-app-service`: Khi Agent gọi các tool có ghi trạng thái như `report_begin_draft`, `report_put_part`, `report_finalize_draft`, `memory_record_decision`, `yumeko_macro_upsert_observation` trên bản snapshot tạm đó, **dữ liệu có được merge/commit ngược về `~/.local/share/yumeko/yumeko-app.sqlite` thật không, hay bị xóa mất khi kết thúc phiên ACP, hoặc gây race condition?**
3. **Kiến trúc Suy luận so với chuẩn GitHub Trending (`TradingAgents`, `FinRobot`, Anthropic Building Effective Agents):**
   - Quy trình 9 bước Top-Down (`VM01 -> HP01 -> DN01 -> DM01 -> KT01 -> HP05`) hiện đang chạy theo kiểu 1 prompt khổng lồ đơn tuyến (`monolithic single-turn`), hay có chia vai chuyên biệt, có cơ chế **Phản biện Đối kháng (`Bull vs. Bear Evidence Debate` như `TauricResearch/TradingAgents`)**, và tách bạch rõ **"Software computes, Models reason, Systems verify" (`FinRobot`)** không?
   - Hệ thống kiểm duyệt đầu ra (`yumeko-report` / `yumeko-evidence` validators) chặn ảo giác số liệu, chặn giải mã bịa đặt các mã trạng thái QMV chưa chứng minh (`PriceTrend`, `DemandUp`), và chặn tư vấn đặt lệnh trực tiếp hoạt động chặt đến đâu?

---

### Trụ cột 2 — Thiết kế `MCP Tools` theo Chuẩn Agent Ergonomics (`Tool Design Best Practices`)
Đọc trực tiếp `crates/yumeko-mcp/src/lib.rs` và toàn bộ tệp cấu hình trong `mcp/`, `HERMES.md`, `AGENTS.md`:
1. Kiểm toán từng tool đang mở (`analysis_three_source_status`, `analysis_daily_prepare`, `analysis_input_chunk`, `report_begin_draft`, `report_put_part`, `report_get_draft_status`, `report_finalize_draft`, `memory_search_method`, `memory_record_decision`, `doraemi_sync_personal`, `qmv_slim_prepare`, `qmv_slim_collect`, `yumeko_macro_upsert_observation`, `yumeko_macro_web_search`) theo chuẩn **"Writing Effective Tools for AI Agents"** của Anthropic:
   - **Hiệu suất Token & Tỷ lệ Tín hiệu/Nhiễu (`Signal-to-Noise`):** `analysis_input_chunk` và các tool đọc dữ liệu trả về bảng tóm tắt tinh gọn hay xả hàng chục KB JSON thô làm loãng sự chú ý của mô hình (`context rot / lost-in-the-middle`)?
   - **Tính Poka-Yoke & Tự sửa lỗi (`Self-Correction Ergonomics`):** Khi `report_put_part` hoặc `report_finalize_draft` từ chối bản thảo do vi phạm quy tắc 20 phần, thông báo lỗi trả về có chỉ rõ chính xác trường/câu nào sai để Agent tự sửa ngay trong 1 lượt không?
2. **Độ lệch Cấu hình & Tài liệu (`Split-Brain Drift`):**
   - Đối chiếu `HERMES.md` (ví dụ các tên tool cũ `daily_assistant_preflight`, `daily_assistant_start_or_resume` có còn tồn tại không?), `AGENTS.md`, và các tệp `mcp/*.antigravity.json` vs `mcp/*.codex.toml`. Chỉ ra toàn bộ điểm lệch giữa các file hướng dẫn Agent và code Rust thực tế.

---

### Trụ cột 3 — Hệ thống `Skills` & Progressive Disclosure (`Skills Architecture Audit`)
Đọc toàn bộ `.agents/skills/*`, `skills/*`, `memory/skills/*`:
1. **Tại sao tồn tại 3 nơi chứa Skills rời rạc?**
   - Đối chiếu từng tệp trong `.agents/skills/` (`doraemi-analyst`, `yumeko-local-analysis`), `skills/` (`market_reading`, `risk_assessment`, `screening`, `symbol_analysis`, ...), và `memory/skills/`.
   - Những skill nào đang **thực sự được nạp** khi chạy `agy acp` / Antigravity / Codex? Những skill nào là "hóa thạch" từ thời Python `v0.3` (tháng 05/2026) chứa lệnh cũ hoặc mâu thuẫn với kiến trúc Rust `v3.1` hiện tại?
2. **Đánh giá theo chuẩn Agent Skills hiện đại (Claude Code / Antigravity Skills Spec 2025–2026):**
   - Có tuân thủ thiết kế **3 tầng tiết lộ dần (`Progressive Disclosure`)**: `YAML frontmatter` định tuyến chuẩn xác -> `SKILL.md` quy trình tinh gọn (`< 500` dòng) -> `references/` & scripts tất định tải lười (`lazy-loaded`) không?
   - Bộ skills hiện tại đã dạy Agent thực hành sắc bén chuỗi phương pháp QMV V7-2026 chưa, hay còn thiếu các skill cốt lõi (ví dụ: Skill đối chiếu định giá `DN01 CSTC`, Skill xác nhận kỹ thuật `KT01` cùng Sumi, Skill hậu kiểm luận điểm `Post-Mortem & Thesis Invalidation`)?

---

### Trụ cột 4 — Cơ chế Tích lũy Tri thức & Bộ nhớ Dài hạn (`Knowledge Compounding & Memory Audit`)
Đọc kỹ `crates/yumeko-memory/src/{lib.rs, compounding.rs, manifest_and_graph.rs}`, toàn bộ `knowledge_factory/` (`01_raw_sources` -> `07_rebuild_candidates`, `KNOWLEDGE_MANIFEST.json`, `OWNERSHIP.md`), thư mục `memory/` (`lessons`, `regime_journal`, `symbol_theses`), và truy vấn **Read-Only** các bảng bộ nhớ (`yr_*`) trong `~/.local/share/yumeko/yumeko-app.sqlite`:
1. **Đo đạc Thực chứng — Bộ nhớ đang "Sống & Sinh sôi" hay đang "Đóng băng"?**
   - Đếm số dòng thực tế trong từng bảng `yr_*` liên quan tới episodic memory, decisions, theses, procedural skills, chat sessions trong `yumeko-app.sqlite`, và đếm số tệp + ngày cập nhật mới nhất trong `memory/` và `knowledge_factory/01..07`.
   - Truy vết code `compounding.rs`: Hàm tích lũy tri thức (`compounding`) **được gọi tự động ở đâu trong luồng chạy thật**, hay mới chỉ nằm trong unit test / lệnh rời ít khi chạy?
2. **Kiểm tra Vòng lặp Phản tư Khép kín (`Closed-Loop Reflection & Outcome Scoring`):**
   - Khi Yumeko ghi nhận một luận điểm đầu tư (`symbol_theses` / `memory_record_decision`) tại ngày `T0`, hệ thống có cơ chế tự động **đối chiếu lại kết quả giá & BCTC thực tế ở `T+10 / T+20 / Quý sau`** để chấm điểm đúng/sai, rút ra bài học (`lessons`) và hạ trọng số các nhận định sai không? Hay chỉ ghi một chiều vào DB rồi bỏ đó?
3. **Đối chiếu với 3 Kiến trúc Bộ nhớ Agent Đỉnh cao trên GitHub Trending (2025–2026):**
   - **Mẫu "LLM Wiki" của Andrej Karpathy (`Stop Retrieving Raw Chunks, Start Compiling Knowledge` — `Ingest -> Compile -> Query -> Lint`):** `knowledge_factory` của Yumeko đã có khung `01_raw_sources -> 04_curated_memory / 05_curated_wiki`, nhưng luồng **Compile** từ `06_review_queue` / `07_rebuild_candidates` sang Wiki và luồng **Lint** chống trôi lệch tri thức có vận hành thật không? (Tại sao `05_curated_wiki` dừng từ tháng 5 trong khi `02_manifests` và `04_curated_memory` cập nhật tháng 11?).
   - **Đồ thị Tri thức Thời gian (`Temporal Knowledge Graph` kiểu `Zep / Graphiti`):** Trong đầu tư tài chính, mọi sự kiện và luận điểm đều có **thời hạn hiệu lực (`valid_from`, `valid_until`, `superseded_by`, `invalidation_trigger`)**. Kiểm tra `manifest_and_graph.rs` và bảng thesis xem Yumeko làm thế nào để ngăn một luận điểm cũ từ Quý trước (đã gãy cấu trúc hoặc đổi chu kỳ vĩ mô) lọt vào ngữ cảnh làm nhiễu báo cáo hôm nay?
   - **Cơ chế Truy hồi Lai (`Hybrid Retrieval`: `FTS5/BM25 + Graph Traversal + Summary Routing` kiểu `LightRAG / Letta`):** Kiểm tra thuật toán thực sự bên trong `memory_search_method` đang tìm kiếm bằng gì (`LIKE '%...%'`, `SQLite FTS5 BM25`, đồ thị liên kết, hay vector)? Chạy thử hoặc mô phỏng truy vấn tiếng Việt thực tế để đánh giá độ chính xác (`Precision / Recall`)!

---

### Trụ cột 5 — Quét các "Căn bệnh Kiểu Sumi" & Khả năng Chạy Đa Máy (`macOS + Windows Portability`)
Kiểm tra thẳng tay các lỗi hệ thống mà Sumi từng mắc phải:
1. **Khả năng mang sang máy khác (`Windows PC / Mac thứ 2`) mà không bị trói vào máy hiện tại:**
   - Grep toàn bộ kho mã (`crates/`, `apps/`, `scripts/`, `mcp/`) tìm đường dẫn cứng `/Users/mizuhara` hoặc giả định chỉ chạy trên macOS.
   - Khi cài Yumeko lên một máy Windows mới (`%LOCALAPPDATA%\yumeko`, không có tệp lịch sử `/Users/mizuhara/workspace/yumeko/data/qmv-shared.sqlite`), quy trình khởi tạo Clean-Slate + đồng bộ từ `Doraemi` (`/sync/v1/*`, `/personal/v1/*`) + đóng gói `scripts/build-windows-app.ps1` có thực sự khôi phục đầy đủ dữ liệu và tri thức không? Có nguy cơ LWW ghi đè mất dữ liệu giữa 2 máy hay lệch chuẩn hóa `SHA-256` giữa Python JSON và Rust `serde_json` (đặc biệt với số thực `float` / scientific notation) không?
2. **Độ vênh giữa `Rust Backend` và `WebView UI` (`apps/yumeko-web`):**
   - Liệt kê rõ các tính năng/bảng dữ liệu đã xây công phu trong 20 Rust crates nhưng chưa nối lên giao diện `apps/yumeko-web` (hoặc nối nửa vời), và ngược lại các nút trên UI đang gọi stub/mock.
3. **Tính toàn vẹn `Point-in-Time (as_of)`:** Có điểm nào khi chạy `./yumeko today --as-of YYYY-MM-DD` bị rò rỉ dữ liệu mới hơn ngày `as_of` (từ bộ nhớ `memory/`, `dnse`, `qmv`, hoặc `macro web search`) không?

---

### Trụ cột 6 — Kiểm toán Mối nối Hai Chiều `Yumeko <-> Sumi` tại Bước 6 (`KT01 Confirmation Gate`)
Đối chiếu `crates/yumeko-sumi-contract`, `crates/yumeko-analysis`, `crates/yumeko-import`, `crates/yumeko-mcp`, `apps/yumeko-web` với kho mã `/Users/mizuhara/workspace/sumi`:
1. Hiện Sumi `v3.0` đã đọc Read-Only `yumeko-app.sqlite`, mở `sumi-mcp` (5 tools định lượng) và xuất gói bằng chứng kỹ thuật `KT01GateVerdictV1` + `TechnicalEvidenceBundleV1` vào `~/.local/share/yumeko/imports/sumi/` kèm URI `sumi://inspect?symbol=...&as_of=...`.
2. **Ở chiều ngược lại trên Yumeko:**
   - Crate `yumeko-sumi-contract` hiện đang được gọi ở đâu trong Yumeko, hay đang nằm cô lập chưa ai gọi (`dead crate`)?
   - Bước 6 (`stock_confirmation_gate`) trong quy trình 9 bước của Yumeko hiện đang đánh giá kỹ thuật cổ phiếu bằng dữ liệu/logic nào? Đã đọc verdict `KT01` từ `~/.local/share/yumeko/imports/sumi/` hoặc gọi `sumi-mcp` chưa?
   - Giao diện `apps/yumeko-web` đã có nút mở nhanh biểu đồ mã cổ phiếu trên Sumi qua `sumi://inspect?...` chưa?
   - Kiểm tra độ khớp kiểu dữ liệu / định dạng JSON (`snake_case` vs `camelCase`, `Sha256Hex`, `Symbol`, `kVND` vs `VND`) giữa tệp Sumi xuất ra và bộ đọc hợp đồng của Yumeko để chỉ rõ mọi điểm lệch nếu nối trực tiếp.

---

### Trụ cột 7 — Bản Thiết kế Nâng cấp Đích (`Yumeko Next-Gen Blueprint & Priority Roadmap`)
Từ toàn bộ bằng chứng thực tế ở 6 trụ cột trên, hãy đề xuất **Kiến trúc Đích & Lộ trình Nâng cấp Ưu tiên (xếp hạng P0 -> P1 -> P2)** theo triết lý **Thực dụng, Tối giản, Không Phô trương (`Karpathy's Simplicity First` + `FinRobot Verification`)**:
1. Những lỗi/điểm đứt gãy nào cần vá ngay (`P0`) để Agent `agy acp`, bộ nhớ `yumeko-memory` và đồng bộ đa máy chạy thật 100%?
2. Thiết kế chuẩn hóa lại hệ thống **Skills + MCP Tools** như thế nào để vừa tiết kiệm ngữ cảnh tối đa, vừa đồng bộ giữa Antigravity / Codex / Hermes?
3. Thiết kế vòng lặp **Tích lũy Tri thức Tự động (`LLM Wiki Compiler + Temporal Thesis Graph + Post-Mortem Outcome Scoring`)** như thế nào để càng dùng lâu, Yumeko càng sắc bén và hiểu sâu phong cách đầu tư QMV của chủ sở hữu?
4. Thiết kế khép kín vòng lặp **`Yumeko (Bước 1–5: Vĩ mô, Ngành, Lọc QMV, Luận điểm) -> Sumi (Bước 6: Xác nhận biểu đồ đa khung KT01, AVWAP, AFC, Backtest T+2) -> Yumeko (Bước 7–9: Định giá, Tỷ trọng DM01, Quản trị rủi ro & Hậu kiểm)`** thành một dòng chảy liền mạch duy nhất trên cả macOS và Windows.
```
