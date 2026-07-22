#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

"$ROOT_DIR/scripts/verify-v2.sh"
"$ROOT_DIR/scripts/run-product-uat.sh"

echo "Sumi product verification passed."
