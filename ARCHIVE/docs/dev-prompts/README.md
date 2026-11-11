# Sumi — DEV prompt entry point

> **Current authorization:** Phase 1 is reviewer-sealed. No legacy single-batch prompt is authorized next. Phase 2 must start through the new autonomous program workflow after baseline/integration ownership is resolved.

**Lưu trữ lịch sử Phase 1.** Chương trình mới bắt đầu tại `docs/dev-program/START.md`; không chạy tuần tự các prompt cũ. Các quy tắc giao việc bên dưới chỉ mô tả quy trình Phase 1 trước đây, được thay thế về điều phối bởi autonomous program.

## Cách giao việc

- Chat chỉ cần một câu dẫn tới prompt trong thư mục này.
- Mỗi prompt giao một kết quả kiểm chứng được, ghi rõ file được sửa, nguồn contract, test và điểm dừng.
- Spec chứa semantics; prompt chứa việc cần làm. Không chép lại toàn bộ spec vào prompt hoặc yêu cầu DEV đọc toàn bộ roadmap cho mỗi lượt.
- Reviewer đóng băng lựa chọn kiến trúc và expected results. DEV triển khai và trả evidence; không tự chọn lại semantics để làm test xanh.
- Giữ một ExecPlan xuyên suốt Phase 1: `docs/exec-plans/P1_SIGNAL_VOLUME_SPIKE.md`. Sau mỗi checkpoint, DEV cập nhật progress/evidence; reviewer kiểm tra code và test trước khi phát prompt tiếp.
- Các checkpoint là các phần của một vertical batch. Phase 1 chỉ hoàn thành sau integration, full gates, browser evidence và reviewer acceptance.
- Không có cam kết model nào không gây bug. Giảm rủi ro bằng phạm vi nhỏ, expected results độc lập, kiểm tra boundary và review thực tế.

## Gate dữ liệu Doraemon/provider

- Prompt của bất kỳ batch nào phụ thuộc dữ liệu Doraemon hoặc capability của provider phải yêu cầu DEV đọc `docs/research/DORAEMON_MARKET_DATA_AUDIT.md` và `docs/research/data_capability_matrix.csv` trước khi code.
- DEV chỉ kiểm tra freshness read-only cho đúng source/endpoint, field, symbol/universe và khoảng ngày mà batch sử dụng; không audit lại toàn bộ Doraemon.
- Nếu schema, freshness, coverage, semantics, access hoặc rights khác evidence đã ghi, DEV cập nhật audit và capability matrix, rồi dừng để reviewer xử lý nếu capability bắt buộc bị suy giảm hoặc chưa được xác minh.
- `P1R_02A_RENDER_IDENTITY.md` không dùng dữ liệu/capability Doraemon nên gate này không yêu cầu gọi provider trong batch hiện tại.

## Các lượt Phase 1

| Lượt | Kết quả | Điểm review |
| --- | --- | --- |
| 01 — M0 + M1 | ExecPlan và core Relative Volume/Volume Spike | Formula, quality/null, hash, future invariance |
| 02 — M2 | Isolated DSL binding và replay API | Negated unknown, strict schemas, no-future payload |
| 03 — M3 | Signal Inspector và mount tối thiểu | Race/stale response, rewind, explanation parity |
| 04 — M4 | Browser UAT, regression gates và handoff | Screenshots, incremental diff, DB invariant |

Chỉ phát hành prompt cho lượt kế tiếp khi reviewer đã xem evidence của lượt trước. Hiện có prompt lượt 01; lượt 02–04 sẽ được viết theo code thực tế sau review, không để DEV tự suy diễn từ bảng này.

## Quy tắc bảo toàn baseline

Áp dụng `Reviewer Adjudication` trong `docs/exec-plans/P0_BASELINE_FREEZE.md`. Giữ index và các thay đổi cũ. Không commit/stash/reset/cleanup. So sánh nội dung theo baseline và các delta đã được reviewer ghi nhận; không dùng tổng số untracked làm điều kiện fail duy nhất.

Hai file `docs/dev-prompts/README.md` và `docs/dev-prompts/P1_01_CORE.md` là tài liệu điều phối được reviewer thêm sau baseline. Chúng là delta dự kiến, không phải concurrent production drift. Không phục hồi các prompt cũ đang staged deletion.

Nếu DEV đã bắt đầu theo prompt chat trước: kiểm kê công việc đã làm, giữ nguyên và báo reviewer; không chạy lại, xóa hoặc đảo ngược code chỉ để khớp thứ tự mới.
