#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PYTHON_BIN="${SUMI_PYTHON:-$ROOT_DIR/.venv/bin/python}"
RUN_BROWSER_SMOKE="${SUMI_BROWSER_SMOKE:-0}"
if command -v cygpath >/dev/null 2>&1; then
  export MSYS2_ENV_CONV_EXCL="DATABASE_URL"
fi

if [[ ! -x "$PYTHON_BIN" ]]; then
  echo "Python environment not found: $PYTHON_BIN" >&2
  echo "Create the Python 3.12 environment and install backend/requirements.txt." >&2
  exit 1
fi

echo "[1/6] Backend tests"
(cd "$ROOT_DIR/backend" && "$PYTHON_BIN" -m pytest -q)

echo "[2/6] Fresh database migration"
MIGRATION_DB="$(mktemp "${TMPDIR:-/tmp}/sumi-migration.XXXXXX.db")"
trap 'rm -f "$MIGRATION_DB"' EXIT
MIGRATION_DB_NATIVE="$MIGRATION_DB"
if command -v cygpath >/dev/null 2>&1; then
  MIGRATION_DB_NATIVE="$(cygpath -m "$MIGRATION_DB")"
fi
(cd "$ROOT_DIR/backend" && DATABASE_URL="sqlite:///$MIGRATION_DB_NATIVE" "$PYTHON_BIN" -m alembic upgrade head)

echo "[3/6] Frontend lint"
(cd "$ROOT_DIR/frontend" && npm run lint)

echo "[4/6] Frontend tests"
(cd "$ROOT_DIR/frontend" && npm test)

echo "[5/6] Frontend production build"
(cd "$ROOT_DIR/frontend" && npm run build)

if [[ "$RUN_BROWSER_SMOKE" == "1" ]]; then
  echo "[6/6] Browser smoke"
  echo "Backend and frontend must already be running."
  (cd "$ROOT_DIR/frontend" && npm run smoke:browser)
else
  echo "[6/6] Browser smoke skipped (set SUMI_BROWSER_SMOKE=1 to enable)"
fi

echo "Sumi V2 verification passed."
