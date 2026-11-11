# Prompt B00 - DB path absolute + guard CWD (GAP-DB-01)

Doc: AGENTS.md, TARGET_ARCHITECTURE_V3.md section 4, ADR-TX-001, DEV_BATCHES.md B00,
`backend/app/config.py`, `backend/alembic/env.py`.

Nhiem vu: DATABASE_URL default absolute neo theo file; startup assert khong tao DB
o CWD la; alembic env cung resolver; doc chay tu `backend/`.

Cam: sua service logic, migration data, mutate `backend/sumi.db` (temp-DB only).

Verify: khoi dong tu /tmp khong tao file; verify-v2.ps1; hash prod unchanged.

STOP neu tooling phu thuoc relative path. Ghi ExecPlan theo PLANS.md truoc code.
