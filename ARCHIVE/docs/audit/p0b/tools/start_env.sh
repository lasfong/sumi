#!/usr/bin/env bash
# Phase 0b — start an isolated audit environment (temp DB, separate ports).
# Never touches backend/sumi.db.
#   bash docs/audit/p0b/tools/start_env.sh           # reuse existing audit DB if present
#   bash docs/audit/p0b/tools/start_env.sh --fresh   # recreate an empty audit DB
# Must be run OUTSIDE the agent sandbox (it binds localhost ports).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
OUT="$ROOT/test-results/p0b"
DB="$OUT/audit.db"
PY="$ROOT/.venv/bin/python"
BPORT=18200
FPORT=15300
unset HTTP_PROXY HTTPS_PROXY http_proxy https_proxy || true
export NO_PROXY="127.0.0.1,localhost" no_proxy="127.0.0.1,localhost"
mkdir -p "$OUT/logs"

bash "$ROOT/docs/audit/p0b/tools/stop_env.sh" >/dev/null 2>&1 || true

if [[ "${1:-}" == "--fresh" ]]; then rm -f "$DB"; fi
(cd "$ROOT/backend" && DATABASE_URL="sqlite:///$DB" "$PY" -m alembic upgrade head >"$OUT/logs/alembic.log" 2>&1)

SHA_BEFORE=$(shasum -a 256 "$ROOT/backend/sumi.db" 2>/dev/null | cut -d' ' -f1 || echo "absent")
echo "$SHA_BEFORE" >"$OUT/sumi_db_sha_before.txt"

cd "$ROOT/backend"
DATABASE_URL="sqlite:///$DB" CORS_ALLOWED_ORIGINS="http://127.0.0.1:$FPORT" \
  nohup "$PY" -m uvicorn app.main:app --host 127.0.0.1 --port $BPORT >"$OUT/logs/backend.log" 2>&1 &
echo $! >"$OUT/backend.pid"

cd "$ROOT/frontend"
"$PY" -c "
import os, subprocess
out_log = open('$OUT/logs/frontend.log', 'w')
env = dict(os.environ, CI='true', SUMI_API_TARGET='http://127.0.0.1:$BPORT')
p = subprocess.Popen(['node', '$ROOT/frontend/node_modules/vite/bin/vite.js', '--host', '127.0.0.1', '--port', '$FPORT', '--strictPort'], cwd='$ROOT/frontend', start_new_session=True, stdout=out_log, stderr=subprocess.STDOUT, env=env)
with open('$OUT/frontend.pid', 'w') as f:
    f.write(str(p.pid))
"

for _ in $(seq 1 80); do
  if curl --noproxy "*" -sf "http://127.0.0.1:$BPORT/api/health" >/dev/null && curl --noproxy "*" -sf "http://127.0.0.1:$FPORT" >/dev/null; then
    echo "READY frontend=http://127.0.0.1:$FPORT backend=http://127.0.0.1:$BPORT db=$DB"
    exit 0
  fi
  sleep 0.5
done
echo "FAILED to start; see $OUT/logs/" >&2
exit 1
