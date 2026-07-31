#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Git Bash launches native Windows child processes with PID and path semantics
# that cannot reliably enforce this runner's cleanup and temporary-DB contract.
# Delegate to the equivalent PowerShell runner on Windows.
if command -v cygpath >/dev/null 2>&1 && command -v powershell.exe >/dev/null 2>&1; then
  powershell.exe -NoProfile -ExecutionPolicy Bypass \
    -File "$(cygpath -w "$ROOT_DIR/scripts/run-product-uat.ps1")"
  exit $?
fi

PYTHON_BIN="${SUMI_PYTHON:-$ROOT_DIR/.venv/bin/python}"
BACKEND_PORT="${SUMI_UAT_BACKEND_PORT:-18000}"
FRONTEND_PORT="${SUMI_UAT_FRONTEND_PORT:-15173}"
RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/sumi-product-uat.XXXXXX")"
DATABASE_PATH="$RUN_DIR/sumi-uat.db"
DATABASE_PATH_NATIVE="$DATABASE_PATH"
if command -v cygpath >/dev/null 2>&1; then
  DATABASE_PATH_NATIVE="$(cygpath -m "$DATABASE_PATH")"
fi
ARTIFACT_DIR="${SUMI_PRODUCT_UAT_ARTIFACT_DIR:-$ROOT_DIR/test-results/product-uat}"
BACKEND_PID=""
FRONTEND_PID=""
if command -v cygpath >/dev/null 2>&1; then
  export MSYS2_ENV_CONV_EXCL="DATABASE_URL;CORS_ALLOWED_ORIGINS;SUMI_API_TARGET;SUMI_FRONTEND_URL;SUMI_BACKEND_URL;SUMI_UAT_DATABASE_PATH;SUMI_PRODUCTION_DATABASE_PATH"
fi

cleanup() {
  if command -v cygpath >/dev/null 2>&1; then
    if [[ -n "$FRONTEND_PID" ]]; then taskkill //PID "$FRONTEND_PID" //T //F >/dev/null 2>&1 || true; fi
    if [[ -n "$BACKEND_PID" ]]; then taskkill //PID "$BACKEND_PID" //T //F >/dev/null 2>&1 || true; fi
  else
    if [[ -n "$FRONTEND_PID" ]]; then kill "$FRONTEND_PID" 2>/dev/null || true; fi
    if [[ -n "$BACKEND_PID" ]]; then kill "$BACKEND_PID" 2>/dev/null || true; fi
  fi
}
trap cleanup EXIT

if [[ ! -x "$PYTHON_BIN" ]]; then
  echo "Python environment not found: $PYTHON_BIN" >&2
  exit 1
fi

(
  cd "$ROOT_DIR/backend"
  DATABASE_URL="sqlite:///$DATABASE_PATH_NATIVE" \
    "$PYTHON_BIN" -m alembic upgrade head
  DATABASE_URL="sqlite:///$DATABASE_PATH_NATIVE" \
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
SUMI_UAT_DATABASE_PATH="$DATABASE_PATH" \
  node scripts/product-uat.mjs

echo "Runtime logs: $RUN_DIR"
