# Sumi autonomous DEV — bắt đầu tại đây

## Khởi chạy

Mở project `E:\Workspace\sumi` trong Antigravity. Dừng các session DEV khác đang ghi project. Gửi một lần:

```text
Đọc docs/dev-program/START.md và thực hiện Sumi autonomous DEV program. Tự điều phối implement–review–fix qua các milestone, lưu checkpoint và tiếp tục các việc đủ điều kiện; không yêu cầu tôi chuyển báo cáo giữa các agent.
```

Nếu tài khoản có Teamwork, dùng cùng câu trên sau `/teamwork-preview`, duyệt brief ban đầu của nền tảng một lần. Không dùng `/team` như tên lệnh mặc định.

## Những gì đã chuẩn bị

- `START.md`: brief, quyền hạn và vòng điều phối; là nguồn duy nhất cho cách chạy.
- `STATE.json`: checkpoint bền vững, không cần nạp toàn bộ lịch sử chat.
- `ROADMAP.md`: hàng đợi và cổng phụ thuộc; không thay thế functional spec.
- `scripts/verify-dev-program.mjs`: kiểm tra checkpoint/đường dẫn, chỉ đọc.
- Skill `sumi-dev`: điểm vào ngắn cho agent nhận diện chương trình.

Trạng thái ban đầu: **READY_FOR_BOOTSTRAP**. Phase 1 đã có reviewer seal; Phase 2 chưa triển khai. Bootstrap phải kiểm tra checkout hiện tại và khả năng independent review trước khi code. Đây không phải tuyên bố toàn bộ dự án sẵn sàng production.

## Thiết lập nên dùng

Ưu tiên native Teamwork nếu có. Cho worker dùng model Flash bạn đang có; không ghi cứng tên model chưa được IDE xác nhận. Nếu có lựa chọn, dùng model mạnh hơn cho orchestrator/reviewer ở causal execution, market rules, migrations và final seal. Nếu chỉ có Flash: vẫn tách context reviewer, khóa numerical oracle trước khi code, không bỏ quality gates. Không có bảo đảm zero-bug.

Giữ xác nhận cho thao tác phá hủy, truy cập secret, network/data export, cài dependency và publish. Không bật quyền chạy mọi lệnh vô điều kiện. Teamwork/skill không tự vượt quota, context limit hoặc permission của IDE; khi bị ngắt, gửi lại câu khởi chạy để tiếp tục từ checkpoint.

## Theo dõi thay vì chuyển báo cáo

Xem `STATE.json`, ExecPlan của batch và review tương ứng. Agent chỉ cần báo ngắn khi đạt milestone, gặp quyết định của owner hoặc kết thúc. Batch xanh tự chuyển tiếp. Nếu thiếu capability bắt buộc: đánh dấu blocked đúng nhánh, tiếp tục nhánh độc lập. Chỉ báo toàn bộ DONE khi không còn task/gate bắt buộc chưa đạt.

## Tương thích đã kiểm tra qua tài liệu, chưa chạy thực tế trong IDE

Google ghi nhận [Teamwork](https://antigravity.google/docs/teamwork/) trên gói trả phí của Antigravity 2.0 và CLI, có review độc lập và workspace riêng. Không suy ra mọi bản IDE đều có lệnh này. Chọn development integrity để giữ code/dependencies hiện hữu, cộng toàn bộ gates của Sumi; không chọn benchmark/from-scratch cho repo này.

[Skills](https://antigravity.google/docs/skills/) hỗ trợ `.agents/skills`. [IDE workflows](https://antigravity.google/docs/ide/workflows/) đang chuyển sang skills, nên không tạo thêm một hệ workflow cũ song song. Nếu skill không được nhận diện, câu đọc `START.md` vẫn dùng được. Nếu runtime không tạo được reviewer context độc lập, chương trình phải ghi NEEDS_REVIEW_CAPABILITY, không giả vờ tự đổi vai là independent review.
