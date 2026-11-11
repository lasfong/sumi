# Prompt B01 - Transaction + FK (GAP-DB-02/03)

Doc: ADR-TX-001, DEV_BATCHES B01, `dependencies.py`, `db.py`,
write-paths import/sync/weekly/rollback.

Nhiem vu: atomic try/commit/except-rollback moi write; get_db rollback-on-error;
PRAGMA foreign_keys=ON listener.

Cam: doi schema/logic derive, mutate prod DB.

Verify: inject loi flush-commit van atomic; FK reject/cascade drawing; verify-v2.ps1.

STOP neu transaction ket.
