#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PYTHON_BIN="${SUMI_PYTHON:-$ROOT_DIR/.venv/bin/python}"
BACKEND_PORT="${SUMI_BATCH5_BACKEND_PORT:-18000}"
FRONTEND_PORT="${SUMI_BATCH5_FRONTEND_PORT:-15173}"
DURATION_SECONDS="${SUMI_BATCH5_DURATION_SECONDS:-1800}"
RUN_ID="$(date -u +%Y-%m-%dT%H-%M-%SZ)"
ARTIFACT_DIR="${SUMI_BATCH5_ARTIFACT_DIR:-$ROOT_DIR/test-results/batch5-hardening/$RUN_ID}"
RUNTIME_DIR="$(mktemp -d "${TMPDIR:-/tmp}/sumi-batch5.XXXXXX")"
FRESH_DB="$RUNTIME_DIR/fresh.db"
PRODUCTION_COPY_DB="$RUNTIME_DIR/production-format-copy.db"
BACKUP_DB="$ARTIFACT_DIR/backup/sumi.db"
RESTORED_DB="$ARTIFACT_DIR/restore/restored.db"
BASELINE="$ROOT_DIR/test-results/batch5-hardening/2026-07-19T01-05-23Z/product-uat/2026-07-19T01-05-26-343Z/results.json"
PRODUCTION_DB="$ROOT_DIR/backend/sumi.db"
BACKEND_PID=""
FRONTEND_PID=""

mkdir -p "$ARTIFACT_DIR/product-uat" "$ARTIFACT_DIR/backup" "$ARTIFACT_DIR/restore"

stop_services() {
  if [[ -n "$FRONTEND_PID" ]]; then kill "$FRONTEND_PID" 2>/dev/null || true; wait "$FRONTEND_PID" 2>/dev/null || true; FRONTEND_PID=""; fi
  if [[ -n "$BACKEND_PID" ]]; then kill "$BACKEND_PID" 2>/dev/null || true; wait "$BACKEND_PID" 2>/dev/null || true; BACKEND_PID=""; fi
}
cleanup() { stop_services; }
trap cleanup EXIT

start_services() {
  local database_path="$1"
  (
    cd "$ROOT_DIR/backend"
    DATABASE_URL="sqlite:///$database_path" CORS_ALLOWED_ORIGINS="http://127.0.0.1:$FRONTEND_PORT" \
      "$PYTHON_BIN" -m uvicorn app.main:app --host 127.0.0.1 --port "$BACKEND_PORT"
  ) >"$ARTIFACT_DIR/backend.log" 2>&1 &
  BACKEND_PID=$!
  (
    cd "$ROOT_DIR/frontend"
    SUMI_API_TARGET="http://127.0.0.1:$BACKEND_PORT" npm run dev -- --host 127.0.0.1 --port "$FRONTEND_PORT"
  ) >"$ARTIFACT_DIR/frontend.log" 2>&1 &
  FRONTEND_PID=$!
  for _ in {1..80}; do
    if curl --silent --fail "http://127.0.0.1:$BACKEND_PORT/api/health" >/dev/null \
      && curl --silent --fail "http://127.0.0.1:$FRONTEND_PORT" >/dev/null; then return; fi
    sleep 0.25
  done
  echo "Batch 5 services failed to start" >&2
  exit 1
}

PRODUCTION_SHA_BEFORE="$(shasum -a 256 "$PRODUCTION_DB" | awk '{print $1}')"
HEAD_VALUE="$(git -C "$ROOT_DIR" rev-parse HEAD)"
TAG_TARGET="$(git -C "$ROOT_DIR" rev-parse 'v2.0.0-rc2^{}')"

(
  cd "$ROOT_DIR/backend"
  DATABASE_URL="sqlite:///$FRESH_DB" "$PYTHON_BIN" -m alembic upgrade head
  DATABASE_URL="sqlite:///$FRESH_DB" "$PYTHON_BIN" scripts/seed_demo.py
) >"$ARTIFACT_DIR/fresh-migration.log" 2>&1
"$PYTHON_BIN" "$ROOT_DIR/backend/scripts/batch5_recovery_verify.py" --source "$FRESH_DB" --output "$ARTIFACT_DIR/fresh-migration.json" >/dev/null

cp "$PRODUCTION_DB" "$PRODUCTION_COPY_DB"
(
  cd "$ROOT_DIR/backend"
  "$PYTHON_BIN" scripts/batch5_recovery_verify.py --source "$PRODUCTION_COPY_DB" --require-application-schema --output "$ARTIFACT_DIR/production-copy-pre-snapshot.json"
  if ! "$PYTHON_BIN" -c 'import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); sys.exit(0 if c.execute("SELECT 1 FROM sqlite_master WHERE type=\"table\" AND name=\"alembic_version\"").fetchone() else 1)' "$PRODUCTION_COPY_DB"; then
    DATABASE_URL="sqlite:///$PRODUCTION_COPY_DB" "$PYTHON_BIN" -m alembic stamp head
  fi
  DATABASE_URL="sqlite:///$PRODUCTION_COPY_DB" "$PYTHON_BIN" -m alembic upgrade head
  "$PYTHON_BIN" scripts/batch5_recovery_verify.py --source "$PRODUCTION_COPY_DB" --output "$ARTIFACT_DIR/production-copy-snapshot.json"
) >"$ARTIFACT_DIR/production-copy-migration.log" 2>&1

start_services "$FRESH_DB"
(
  cd "$ROOT_DIR"
  SUMI_FRONTEND_URL="http://127.0.0.1:$FRONTEND_PORT" SUMI_BACKEND_URL="http://127.0.0.1:$BACKEND_PORT" \
  SUMI_PRODUCT_UAT_ARTIFACT_DIR="$ARTIFACT_DIR/product-uat" SUMI_BATCH5_DURATION_SECONDS="$DURATION_SECONDS" \
    node scripts/batch5-hardening-uat.mjs
) >"$ARTIFACT_DIR/hardening.log" 2>&1
stop_services

PRODUCT_RESULTS="$(find "$ARTIFACT_DIR/product-uat" -name results.json -type f | sort | tail -1)"
WORKSPACE_EXPORT="$(find "$ARTIFACT_DIR/product-uat" -name workspace-export.json -type f | sort | tail -1)"
if [[ -z "$PRODUCT_RESULTS" || -z "$WORKSPACE_EXPORT" ]]; then echo "Hardening evidence is incomplete" >&2; exit 1; fi

"$PYTHON_BIN" "$ROOT_DIR/backend/scripts/batch5_recovery_verify.py" --source "$FRESH_DB" --backup "$BACKUP_DB" --output "$ARTIFACT_DIR/database-recovery.json" >"$ARTIFACT_DIR/database-recovery.log" 2>&1
cp "$BACKUP_DB" "$RESTORED_DB"
"$PYTHON_BIN" "$ROOT_DIR/backend/scripts/batch5_recovery_verify.py" --source "$RESTORED_DB" --output "$ARTIFACT_DIR/restored-snapshot.json" >"$ARTIFACT_DIR/restored-snapshot.log" 2>&1

start_services "$RESTORED_DB"
(
  cd "$ROOT_DIR"
  SUMI_FRONTEND_URL="http://127.0.0.1:$FRONTEND_PORT" SUMI_BACKEND_URL="http://127.0.0.1:$BACKEND_PORT" \
  SUMI_BATCH5_WORKSPACE_EXPORT="$WORKSPACE_EXPORT" SUMI_BATCH5_RESTORE_ARTIFACT_DIR="$ARTIFACT_DIR" \
    node scripts/batch5-restore-uat.mjs
) >"$ARTIFACT_DIR/restore.log" 2>&1
stop_services

node "$ROOT_DIR/scripts/batch5-evidence-negative-selftest.mjs" >"$ARTIFACT_DIR/negative-selftest.json"
node "$ROOT_DIR/scripts/batch5-evidence-audit.mjs" "$BASELINE" "$PRODUCT_RESULTS" "$ARTIFACT_DIR/baseline-audit.json" >"$ARTIFACT_DIR/baseline-audit.log" 2>&1
SUMI_BATCH5_ARTIFACT_DIR="$ARTIFACT_DIR" SUMI_BATCH5_PRODUCTION_DB="$PRODUCTION_DB" \
SUMI_BATCH5_PRODUCTION_SHA_BEFORE="$PRODUCTION_SHA_BEFORE" SUMI_BATCH5_PRODUCT_RESULTS="$PRODUCT_RESULTS" \
SUMI_BATCH5_WORKSPACE_EXPORT="$WORKSPACE_EXPORT" SUMI_BATCH5_BACKUP_DB="$BACKUP_DB" SUMI_BATCH5_RESTORED_DB="$RESTORED_DB" \
SUMI_BATCH5_HEAD="$HEAD_VALUE" SUMI_BATCH5_TAG_TARGET="$TAG_TARGET" node "$ROOT_DIR/scripts/batch5-manifest.mjs" >"$ARTIFACT_DIR/manifest.log" 2>&1

echo "Batch 5 hardening evidence: $ARTIFACT_DIR"
