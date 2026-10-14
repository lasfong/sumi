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

VERIFY_TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/sumi-verify-v2.XXXXXX")"

cleanup() {
  unset DATABASE_URL || true
  if [[ -n "${VERIFY_TMP_DIR:-}" && -d "$VERIFY_TMP_DIR" ]]; then
    rm -rf "$VERIFY_TMP_DIR"
  fi
}
trap cleanup EXIT
trap 'cleanup; exit 130' INT
trap 'cleanup; exit 143' TERM
trap 'cleanup; exit 129' HUP

echo "[1/6] Backend tests (isolated database)"
VERIFY_DB="$VERIFY_TMP_DIR/sumi-verify.db"
VERIFY_DB_NATIVE="$VERIFY_DB"
if command -v cygpath >/dev/null 2>&1; then
  VERIFY_DB_NATIVE="$(cygpath -m "$VERIFY_DB")"
fi
export DATABASE_URL="sqlite:///$VERIFY_DB_NATIVE"
(cd "$ROOT_DIR/backend" && "$PYTHON_BIN" -m pytest -q)

echo "[2/6] Fresh database migration (distinct temporary database)"
MIGRATION_DB="$VERIFY_TMP_DIR/sumi-migration.db"
MIGRATION_DB_NATIVE="$MIGRATION_DB"
if command -v cygpath >/dev/null 2>&1; then
  MIGRATION_DB_NATIVE="$(cygpath -m "$MIGRATION_DB")"
fi
(cd "$ROOT_DIR/backend" && DATABASE_URL="sqlite:///$MIGRATION_DB_NATIVE" "$PYTHON_BIN" -m alembic upgrade head)

# DATABASE_URL remains exported to VERIFY_DB_NATIVE for stages 3-6 to guarantee
# that any subsequent tool, test, or smoke run cannot fall back to production DB.
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
