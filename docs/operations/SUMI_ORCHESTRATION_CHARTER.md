# SUMI TRADING WORKSTATION — PROGRAM ORCHESTRATION CHARTER

- **Tài liệu quản trị:** `docs/operations/SUMI_ORCHESTRATION_CHARTER.md`
- **Thiết lập tại Session:** `SESSION-R03-20261111T105832+0700`
- **Phạm vi:** Điều phối chương trình phát triển Sumi Rust Desktop Workstation tại `/Users/mizuhara/workspace/sumi`
- **Ngôn ngữ giao tiếp với Product Owner:** Tiếng Việt (giữ nguyên tên kỹ thuật, file path, symbol, command, Evidence ID).

---

## 1. Mục đích và Nguyên tắc Điều phối

Tài liệu này thiết lập cơ chế vận hành bền vững cho **SUMI PROGRAM ORCHESTRATOR**, bảo đảm mọi session hoặc successor tiếp theo có thể **tự khởi động (self-bootstrap)** trực tiếp từ repository mà không yêu cầu Product Owner sao chép báo cáo hoặc tạo prompt cho từng ticket nhỏ.

Repository `/Users/mizuhara/workspace/sumi` là **Single Source of Truth**. Việc mã nguồn web TypeScript/Python cũ được lưu trữ nguyên trạng trong `ARCHIVE/` cùng tồn tại với ứng dụng Rust mới tại root là quyết định kiến trúc có chủ đích (`DEC-001`), tuyệt đối không tự ý xóa, di chuyển hoặc tách repository.

### Thứ tự ưu tiên nguồn sự thật khi có xung đột

1. Chỉ thị trực tiếp của Product Owner.
2. Các quyết định đã được phê duyệt chính thức trong [`docs/DECISION_LOG.md`](file:///Users/mizuhara/workspace/sumi/docs/DECISION_LOG.md).
3. Master roadmap ([`docs/research/RESEARCH_TO_RUST_ROADMAP.md`](file:///Users/mizuhara/workspace/sumi/docs/research/RESEARCH_TO_RUST_ROADMAP.md)) và hồ sơ quản trị hiện hành ([`docs/PROJECT_CHARTER.md`](file:///Users/mizuhara/workspace/sumi/docs/PROJECT_CHARTER.md)).
4. Trạng thái thực thi và handoff ([`docs/EXECUTION_STATE.md`](file:///Users/mizuhara/workspace/sumi/docs/EXECUTION_STATE.md), [`docs/NEXT_SESSION_BRIEF.md`](file:///Users/mizuhara/workspace/sumi/docs/NEXT_SESSION_BRIEF.md), [`docs/sessions/SESSION_REGISTRY.md`](file:///Users/mizuhara/workspace/sumi/docs/sessions/SESSION_REGISTRY.md)).
5. Suy luận kỹ thuật của Agent (phải được kiểm chứng bằng mã nguồn thực tế hoặc bằng chứng thực thi trước khi nâng cấp thành kết luận).

---

## 2. Xác minh Khả năng Điều phối của Môi trường Antigravity

Đã kiểm tra thực tế trên runtime hiện hành (`antigravity-client v1.0.0`, macOS `arm64`):

### 2.1. Phân biệt các bề mặt Antigravity

| Bề mặt | Đặc điểm kiến trúc | Khả năng điều phối |
|---|---|---|
| **Antigravity IDE** | Môi trường tích hợp dựa trên VS Code (inline code lenses, tab supercomplete, sidebar chat/agent, `.agents/`). | Chạy Agent theo hội thoại trực tiếp trong IDE; hỗ trợ MCP và subagents tùy cấu hình phiên. |
| **Antigravity 2.0 App** | Ứng dụng desktop điều phối đa tiến trình (Chat Canvas, Scheduled Tasks, Auxiliary Pane cho Subagents, Background Tasks, Artifacts, Swarm Board). | Hỗ trợ đầy đủ cả Hierarchical Subagents (`define_subagent` / `invoke_subagent`) và Native Swarm (`swarm` tool / `/teamwork` UI). |
| **Antigravity CLI (`agy`)** | Giao diện dòng lệnh TUI cấu hình qua `~/.gemini/antigravity-cli/settings.json`. | Tương tác nhanh qua terminal, không có UI sidecar hay Swarm Overview trực quan. |

### 2.2. Các công cụ điều phối và nghiên cứu khả dụng thực tế trong phiên

1. **Hierarchical Subagents (`define_subagent`, `invoke_subagent`, `send_message`, `manage_subagents`) — `VERIFIED`:**
   - Hoạt động trực tiếp, cho phép Orchestrator định nghĩa các agent chuyên trách với quyền công cụ tách biệt (`EnableWriteTools`, `EnableMCPTools`), chạy song song bất đồng bộ với cơ chế đánh thức phản ứng (`reactive wakeup`), đồng thời giữ toàn quyền kiểm soát kế hoạch, tích hợp mã nguồn và nghiệm thu tại Orchestrator.
2. **Native Swarm / Teamwork Preview (`swarm` tool) — `VERIFIED (Preview & Quota-Heavy)`:**
   - Công cụ `swarm` phản hồi trực tiếp (`status: "no swarm yet; add a worker to start one"`). Tuy nhiên theo đặc tả kỹ thuật của `native_swarm`, các công cụ bảng tin (`post_message`, `read_board`, `swarm_wait`) chỉ xuất hiện ở lượt kế tiếp sau lệnh `swarm add` đầu tiên (bắt buộc ngắt lượt về phía người dùng) và tiêu tốn nhiều quota cho giao tiếp quảng bá.
   - **Quyết định điều phối:** Ưu tiên sử dụng **Hierarchical Subagents** (`sumi-researcher`, `sumi-critic`, `sumi-auditor`) dưới sự kiểm soát tập trung của Orchestrator cho các milestone kỹ thuật trong cùng một repository nhằm tránh xung đột ghi file song song và giảm báo cáo trung gian cho Product Owner; chỉ kích hoạt `swarm` với `workspace: "share"` khi có nhiều phân hệ hoàn toàn độc lập hoặc khi sắp hết context cần bàn giao phiên tự động.
3. **REA MCP (`rea-agents@6.8.0`, 229 tools) — `VERIFIED`:**
   - Kết nối trực tiếp qua `/opt/homebrew/bin/rea mcp`.
   - Khả dụng ngay không cần cài thêm công cụ: `binary_session`, `open_binary`, `close_binary`, `inspect_macho`, `list_architectures`, `trace_dylib_resolution`, `inspect_rust_binary`, `analyze_javascript_application`, `inspect_analysis_view`, `trace_application_feature`, `trace_javascript_semantics`, `compare_application_versions`, `compare_javascript_export_shapes`.
   - Giới hạn đã xác minh: Chưa cài đặt native decompiler (`Ghidra`, `Hopper`, `IDA`, `Kuna`) và chưa cấp quyền macOS TCC Screen Recording cho `capture_native_ui_scenario` / `observe_native_calls`.

---

## 3. Mô hình Vai trò Chuyên trách (Role Topology)

Orchestrator điều phối 4 vai trò độc lập trong vòng đời mỗi milestone:

1. **Program Orchestrator (Điều phối viên chính):**
   - Sở hữu kế hoạch tổng thể, quản lý ranh giới kiến trúc, phê duyệt nội bộ thiết kế kỹ thuật (trong phạm vi không vượt thẩm quyền Product Owner), tích hợp thay đổi và cập nhật hồ sơ quản trị.
2. **Research Agent (`sumi-researcher` — Read-Only):**
   - Đối chiếu trực tiếp các thuật toán với mã nguồn tham chiếu hợp pháp (`lightweight-charts v5.2.0` Apache 2.0, `TA-Lib` BSD, `ARCHIVE/` nội bộ) và bằng chứng REA MCP. Không sửa file.
3. **Engineering Execution (Orchestrator / Scoped Engineering Worker):**
   - Thực thi mã nguồn Rust sạch (`wasm32`-compatible cho tầng domain/viewport), viết unit tests + property/invariant tests, tích hợp giao diện `egui`/`eframe`, chạy benchmark định lượng. Chỉ một tác nhân được phép ghi vào một module tại một thời điểm.
4. **Critic / Challenger (`sumi-critic` — Read-Only Adversarial Review):**
   - Phản biện độc lập sau khi triển khai: tìm lỗi công thức biên (`NaN`, `Inf`, `0/1 bar`, kích thước `<= 1px`, log âm/0), rò rỉ hệ tọa độ (`logical` vs `pane-local` vs `screen`), coupling kiến trúc, test yếu hoặc regression.
5. **Independent Auditor (`sumi-auditor` — Verification-Only):**
   - Kiểm chứng độc lập các file thực tế thay đổi, thực thi lại các lệnh kiểm tra (`cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, benchmark), đối chiếu Evidence ID của REA, kiểm tra tuân thủ giới hạn Git/Workspace và gán nhãn bằng chứng nghiêm ngặt (`VERIFIED` / `INFERRED` / `UNVERIFIED` / `BLOCKED`).

---

## 4. Quy trình Tự khởi động (Cold-Start Bootstrap Protocol)

Bất kỳ Orchestrator hoặc successor mới nào khi tiếp nhận `/Users/mizuhara/workspace/sumi` PHẢI thực hiện tuần tự:

1. **Kiểm tra trạng thái Git và Workspace (Read-Only):**
   - Chạy `git branch --show-current`, `git rev-parse HEAD`, `git status --short --branch`, `git tag -l`.
   - Phân biệt rõ trạng thái dirty có sẵn từ Session 01 (`M .gitignore`, `687` đường dẫn `D` đã chuyển vào `ARCHIVE/`, và các thư mục `??` của dự án Rust mới) với những thay đổi phát sinh trong phiên hiện tại.
2. **Đọc bộ hồ sơ chính thức (Canonical Ledger):**
   - [`docs/EXECUTION_STATE.md`](file:///Users/mizuhara/workspace/sumi/docs/EXECUTION_STATE.md)
   - [`docs/NEXT_SESSION_BRIEF.md`](file:///Users/mizuhara/workspace/sumi/docs/NEXT_SESSION_BRIEF.md)
   - [`docs/DECISION_LOG.md`](file:///Users/mizuhara/workspace/sumi/docs/DECISION_LOG.md)
   - [`docs/sessions/SESSION_REGISTRY.md`](file:///Users/mizuhara/workspace/sumi/docs/sessions/SESSION_REGISTRY.md)
   - [`docs/evidence/EVIDENCE_INDEX.md`](file:///Users/mizuhara/workspace/sumi/docs/evidence/EVIDENCE_INDEX.md)
   - [`docs/research/RESEARCH_TO_RUST_ROADMAP.md`](file:///Users/mizuhara/workspace/sumi/docs/research/RESEARCH_TO_RUST_ROADMAP.md)
3. **Xác minh sức khỏe mã nguồn hiện tại:**
   - Chạy `export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH" && cargo test` trước khi sửa bất kỳ dòng code nào.
4. **Thực thi vòng lặp Milestone (Research Gate → Design → Implement → Benchmark & GUI Verify → Critic Review → Fix → Independent Audit → Ledger Update):**
   - Không tuyên bố hoàn thành từ cảm tính hoặc chỉ dựa trên `cargo check`.
   - Khi sắp chạm giới hạn context window: cập nhật đầy đủ `EXECUTION_STATE.md` và `NEXT_SESSION_BRIEF.md` trước khi dừng hoặc bàn giao successor.

---

## 5. Các Rào chắn Bất khả xâm phạm (Hard Guardrails)

1. **Cấm thao tác Git ghi/phá hủy khi chưa có lệnh trực tiếp của Product Owner:** Không `git commit`, `git tag`, `git reset`, `git clean`, `git checkout --`, `git stash`.
2. **Bảo toàn dữ liệu & lịch sử:** Không xóa bất kỳ file/thư mục nào; không chạm vào nội dung trong `ARCHIVE/` (chỉ đọc để nghiên cứu).
3. **Giới hạn pháp lý & Clean-Room:** Tuyệt đối không dùng REA hoặc bất kỳ công cụ nào để dịch ngược/trích xuất TradingView Desktop proprietary binaries/bundles (`DEC-006`). Chỉ nghiên cứu các nguồn hợp pháp đã phê duyệt (`lightweight-charts v5.2.0` Apache 2.0, `TA-Lib` BSD, `ARCHIVE/` của chính dự án, và binary Rust của Sumi).
4. **Kiểm soát Dependency & Chi phí:** Không tự ý thêm dependency lớn làm phình binary hoặc phá vỡ khả năng biên dịch `wasm32-unknown-unknown` của domain core; không phát sinh chi phí dịch vụ ngoài nếu chưa được Product Owner phê duyệt.
