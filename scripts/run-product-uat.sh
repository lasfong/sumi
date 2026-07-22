#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PYTHON_BIN="${SUMI_PYTHON:-$ROOT_DIR/.venv/bin/python}"
BACKEND_PORT="${SUMI_UAT_BACKEND_PORT:-18000}"
FRONTEND_PORT="${SUMI_UAT_FRONTEND_PORT:-15173}"
RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/sumi-product-uat.XXXXXX")"
DATABASE_PATH="$RUN_DIR/sumi-uat.db"
ARTIFACT_DIR="${SUMI_PRODUCT_UAT_ARTIFACT_DIR:-$ROOT_DIR/test-results/product-uat}"
BACKEND_PID=""
FRONTEND_PID=""

cleanup() {
  if [[ -n "$FRONTEND_PID" ]]; then kill "$FRONTEND_PID" 2>/dev/null || true; fi
  if [[ -n "$BACKEND_PID" ]]; then kill "$BACKEND_PID" 2>/dev/null || true; fi
}
trap cleanup EXIT

if [[ ! -x "$PYTHON_BIN" ]]; then
  echo "Python environment not found: $PYTHON_BIN" >&2
  exit 1
fi

(
  cd "$ROOT_DIR/backend"
  DATABASE_URL="sqlite:///$DATABASE_PATH" \
    "$PYTHON_BIN" -m alembic upgrade head
  DATABASE_URL="sqlite:///$DATABASE_PATH" \
    "$PYTHON_BIN" scripts/seed_demo.py
)

(
  cd "$ROOT_DIR/backend"
  DATABASE_URL="sqlite:///$DATABASE_PATH" \
  CORS_ALLOWED_ORIGINS="http://127.0.0.1:$FRONTEND_PORT" \
    "$PYTHON_BIN" -m uvicorn app.main:app --host 127.0.0.1 --port "$BACKEND_PORT"
) >"$RUN_DIR/backend.log" 2>&1 &
BACKEND_PID=$!

(
  cd "$ROOT_DIR/frontend"
  SUMI_API_TARGET="http://127.0.0.1:$BACKEND_PORT" \
    npm run dev -- --host 127.0.0.1 --port "$FRONTEND_PORT"
) >"$RUN_DIR/frontend.log" 2>&1 &
FRONTEND_PID=$!

for _ in {1..40}; do
  if curl --silent --fail "http://127.0.0.1:$BACKEND_PORT/api/health" >/dev/null \
    && curl --silent --fail "http://127.0.0.1:$FRONTEND_PORT" >/dev/null; then
    break
  fi
  sleep 0.25
done

curl --silent --fail "http://127.0.0.1:$BACKEND_PORT/api/health" >/dev/null
curl --silent --fail "http://127.0.0.1:$FRONTEND_PORT" >/dev/null

cd "$ROOT_DIR"
SUMI_FRONTEND_URL="http://127.0.0.1:$FRONTEND_PORT" \
SUMI_BACKEND_URL="http://127.0.0.1:$BACKEND_PORT" \
SUMI_PRODUCT_UAT_ARTIFACT_DIR="$ARTIFACT_DIR" \
  node scripts/product-uat.mjs

echo "Runtime logs: $RUN_DIR"
