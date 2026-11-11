# P1-01 — Signal core và kiểm chứng công thức

Thực hiện trong `E:\Workspace\sumi`. Đây là checkpoint M0 + M1 của Phase 1; xong thì dừng để session reviewer kiểm tra.

## Đọc trước

1. `docs/dev-prompts/README.md` và `AGENTS.md`; đọc các canonical sources mà AGENTS yêu cầu trước khi thiết kế behavior.
2. `PLANS.md`; phần `Reviewer Adjudication` của `docs/exec-plans/P0_BASELINE_FREEZE.md`.
3. Trong `docs/research/SUMI_FINAL_DEV_IMPLEMENTATION_PLAN.md`: O, P.3, P.4, P.5, P.6. Đọc trọn các mục này, dùng contract đã khóa; không cần nghiên cứu lại các phase khác.
4. Trong Master Spec: `SIG-VOL-001`, `SIG-VOL-002`, Signal Definition ở Appendix E.1. Nếu nguồn mâu thuẫn, báo vị trí và dừng phần phụ thuộc.

## Kết quả cần giao

Core thuần tính Relative Volume và Volume Spike, có registry/version, quality, reasons, params hash và test độc lập. Chưa nối API, DSL hoặc UI ở lượt này.

## File được tạo/sửa

- `docs/exec-plans/P1_SIGNAL_VOLUME_SPIKE.md`
- `backend/app/domain/signals/__init__.py`
- `backend/app/domain/signals/models.py`
- `backend/app/domain/signals/registry.py`
- `backend/app/domain/signals/volume.py`
- `backend/app/tests/test_signals.py`

Các đường dẫn còn lại chỉ đọc. Không thêm dependency, migration, network call hoặc thay đổi Git index. Nếu cần mở rộng danh sách, báo reviewer cùng lý do cụ thể.

## Thực hiện theo thứ tự

1. Kiểm tra HEAD/index và hash các file liên quan với baseline. Ghi delta đã biết theo README. Nếu có code Phase 1 từ lượt khác, báo hiện trạng và dừng trước khi sửa đè.
2. Tạo ExecPlan theo PLANS.md cho toàn bộ vertical batch, ghi rõ chỉ M0/M1 đang được thực hiện. Dẫn tới contract gốc và chốt interface core cụ thể, allowed/protected paths, test commands, rollback giữ nguyên công việc cũ. Không chép lại toàn bộ roadmap.
3. Viết expected-value tests từ P.5 trước: P1-OR-01..06, P1-OR-11; thêm strict parameter và duplicate/non-monotonic timestamp cases của P.4. Expected values phải là hằng số/tính tay, không gọi production function để tạo expected.
4. Implement core nhỏ nhất đủ pass. Không tạo plugin framework, universal signal DAG hoặc generic strategy engine. Core không import FastAPI/SQLAlchemy/service/DB; reuse dependency hiện có khi cần.
5. Chạy focused tests bằng Python trong `backend/.venv`, từ working directory `backend`, sau khi set temporary `DATABASE_URL` theo P.6. Lệnh focused: `.\.venv\Scripts\python.exe -m pytest app/tests/test_signals.py -q`. Ghi command thực tế, exit code và output. Không import app trên DB mặc định.
6. Rà diff với whitelist và contract; xác minh DB hash, index không đổi. Cập nhật ExecPlan với test evidence và các milestone chưa làm.

Contract trọng yếu cần tự kiểm tra trước khi bàn giao: baseline loại trừ current bar; đủ N prior bars mới tính; null khác false/zero; equality dùng `>=`; không làm tròn trước so sánh; params defaults cho hash giống explicit defaults; append/mutate future không đổi past. Chi tiết và expected arithmetic nằm ở P.4/P.5.

Nếu test fail, sửa implementation trong scope. Không giảm assertion, đổi expected hoặc bỏ quality guard. Nếu không thể sửa trong scope, ghi failure và dừng. Chưa chạy full browser gates ở checkpoint chỉ có core; các gate đó vẫn bắt buộc trước khi đóng Phase 1.

## Bàn giao rồi STOP

Trả tối đa một trang: đường dẫn ExecPlan; file tạo/sửa; bảng oracle → test → kết quả; lệnh/exit code; DB/index check; vấn đề còn lại. Dùng trạng thái `M1_READY_FOR_REVIEW` hoặc `M1_BLOCKED`, không tự ghi `PHASE_1_DONE`.

Không nối DSL/API/UI hoặc bắt đầu M2. Reviewer ở session chính sẽ đọc code/test và phát prompt kế tiếp.
