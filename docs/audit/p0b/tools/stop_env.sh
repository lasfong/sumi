#!/usr/bin/env bash
# Phase 0b — stop the audit environment started by start_env.sh and check sumi.db is untouched.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
OUT="$ROOT/test-results/p0b"
for name in backend frontend; do
  if [[ -f "$OUT/$name.pid" ]]; then
    pid=$(cat "$OUT/$name.pid")
    pkill -P "$pid" 2>/dev/null || true
    kill "$pid" 2>/dev/null || true
    rm -f "$OUT/$name.pid"
  fi
done
for port in 18200 15300; do
  lsof -ti tcp:$port 2>/dev/null | xargs kill 2>/dev/null || true
done
if [[ -f "$OUT/sumi_db_sha_before.txt" ]]; then
  before=$(cat "$OUT/sumi_db_sha_before.txt")
  after=$(shasum -a 256 "$ROOT/backend/sumi.db" 2>/dev/null | cut -d' ' -f1 || echo "absent")
  if [[ "$before" == "$after" ]]; then echo "sumi.db unchanged ($after)"; else echo "WARNING sumi.db CHANGED before=$before after=$after"; fi
fi
echo "stopped"
