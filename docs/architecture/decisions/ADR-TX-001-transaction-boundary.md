# ADR-TX-001 — Transaction Boundary (rollback everywhere + FK + absolute DB path)

**Status:** Accepted 2026-10-09.
**Context:** Write-path commit/flush trần không rollback; `get_db` chỉ close; FK không pragma; DB URL tương đối sinh file ma root 0B; alembic env thiếu 8 models; preview commit thiếu confirm (GAP-DB-01/02/03/05/10/14, NEW-DB-02/03).
**Decision:** Mọi write `try/commit/except-rollback`; `get_db` rollback-on-error; `PRAGMA foreign_keys=ON`; DB path absolute neo file + startup assert + xóa ma sau backup; env import full models; migration guard `_has_column`; preview tách commit/temp.
**Alternatives:** Chỉ bọc vài path chính — loại: lỗi giữa flush→commit vẫn mở transaction.
**Verification:** Inject lỗi derive vẫn atomic; FK reject/cascade test; khởi động từ CWD lạ không tạo file.
