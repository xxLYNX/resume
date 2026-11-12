#!/usr/bin/env bash
# Run Opportunities Playwright suite N times (default 5); exit non-zero on any failure.
set -euo pipefail
N="${1:-5}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export JOBDESK_E2E_WORKSPACE="$ROOT/target/e2e-opportunities-workspace"
export JOBDESK_E2E_FRESH_WS=1
export JOBDESK_E2E_PORT="${JOBDESK_E2E_PORT:-19547}"
if command -v fuser >/dev/null 2>&1; then
  fuser -k "${JOBDESK_E2E_PORT}/tcp" 2>/dev/null || true
fi
cd "$ROOT"
echo "Determinism check: $N consecutive e2e runs (workspace=$JOBDESK_E2E_WORKSPACE)"
for i in $(seq 1 "$N"); do
  echo "=== run $i/$N ==="
  if command -v nix-shell >/dev/null 2>&1 && [[ -f e2e/opportunities/shell.nix ]]; then
    nix-shell e2e/opportunities/shell.nix --run \
      "cd e2e/opportunities && npx playwright test" || exit 1
  else
    cd e2e/opportunities && npx playwright test || exit 1
  fi
  fuser -k "${JOBDESK_E2E_PORT}/tcp" 2>/dev/null || true
  sleep 1
done
echo "All $N runs passed."
