# PRO-03 — Data Catalog and Import Quality

## Outcome
Users can see exactly which local Daily/Weekly histories exist and can preview, accept, repeat, or roll back an import without silent data loss.

## Acceptance boundary
`PRO-DATA-01`–`PRO-DATA-07` in the master plan and V3 acceptance criteria. Catalog provenance, pre-mutation preview, ambiguity rejection, idempotent manifest/checksum, deterministic Daily→Weekly provenance, adjustment isolation, and exact rollback are mandatory.

## Dependencies and risks
Build on the existing Symbol/Candle/import services. The current CafeF importer upserts conflicting candles, so conflict quarantine and transaction/backup design are required. Never mutate `backend/sumi.db`; tests use temporary databases.

## Entry / exit
Entry: PRO-00–PRO-02 approved and current ledger names PRO-03. Exit: malformed, ambiguous, duplicate, conflicting, out-of-order, adjusted, and rollback fixtures pass; full technical/product gates and retained evidence pass; DEV stops `IMPLEMENTED — REVIEW PENDING`.
