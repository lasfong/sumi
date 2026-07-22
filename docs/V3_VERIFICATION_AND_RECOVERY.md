# V3 verification and local recovery procedure

This procedure is local-only. Never test recovery against `backend/sumi.db` in place.

## Verification

Run from the repository root:

```bash
git diff --check
cd backend && ../.venv/bin/python -m pytest -q
cd ../frontend && npm test -- --run && npm run lint && npm run build
cd .. && ./scripts/verify-v2.sh
./scripts/run-product-uat.sh
./scripts/verify-product.sh
node scripts/batch5-evidence-negative-selftest.mjs
./scripts/run-batch5-hardening.sh
shasum -a 256 backend/sumi.db
git rev-parse HEAD
git rev-parse 'v2.0.0-rc2^{}'
```

For an unattended canonical run on macOS, prevent system sleep without changing the runner contract:

```bash
caffeinate -dimsu ./scripts/run-batch5-hardening.sh
```

The canonical result must retain 277 unique passing IDs: the unchanged returned 272-ID baseline plus exactly five blocking `batch5.closure.*` IDs. The negative selftest must report all 20 cases passing, and the final manifest must be `pass: true`.

The Batch 5 runner creates a fresh database, validates a copied production-format database, executes the real browser workflow, stops services, creates a SQLite online backup inside the artifact root at `restore/restored.db`, starts against that restored path, imports the browser-local workspace bundle and compares semantic state. The manifest stores bundle-relative canonical evidence paths and verifies every file is a regular non-symlink inside the artifact root with matching SHA-256.

## Manual backup

1. Stop Sumi backend/frontend processes that use the database.
2. Copy `backend/sumi.db` to a new dated directory, never over an existing backup.
3. Export relevant Sumi localStorage entries as JSON. Database-only backup does not include indicator/drawing/magnet browser state.
4. Record SHA-256 values for the source database, backup database and workspace JSON.
5. Keep code/version provenance with the bundle.

## Restore rehearsal

1. Copy the backup database to a new temporary restore path.
2. Point `DATABASE_URL` at that new path; never point a rehearsal at `backend/sumi.db`.
3. If a legacy production-format copy has the required application tables but no `alembic_version`, validate the schema first, then stamp and upgrade the copy only. Do not stamp the production file in place.
4. Start local services against the restored path.
5. Import the workspace JSON before opening Replay.
6. Verify session index, decisions/orders/executions/trades, journal entries, indicator document, drawing document and localhost-only runtime.

The reproducible implementation is `scripts/run-batch5-hardening.sh`; its manifest records the exact subcommands, comparison results, containment checks and post-write hash verification. The sealed canonical example is `test-results/batch5-hardening/2026-07-22T10-06-17Z/manifest.json`.

## Rollback boundary

V3 schema-v1 browser documents and newer opaque records are not promised readable by RC2. A code rollback must preserve the newer backup bundle and must not destructively rewrite it. `v2.0.0-rc2` remains protected and unchanged.
