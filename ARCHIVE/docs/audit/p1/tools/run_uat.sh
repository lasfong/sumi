#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
export SUMI_FRONTEND_URL="http://127.0.0.1:15300"
export SUMI_BACKEND_URL="http://127.0.0.1:18200"
cd "$ROOT"
node "$ROOT/docs/audit/p1/tools/run_p1_uats.mjs"
