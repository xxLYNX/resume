#!/usr/bin/env bash
# Full Opportunities browser acceptance: isolated workspace, Playwright (Firefox + Chromium).
# NixOS: nix-shell e2e/opportunities/shell.nix --run ./scripts/run-opportunities-e2e.sh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ -z "${PLAYWRIGHT_BROWSERS_PATH:-}" ]] && command -v nix-build >/dev/null 2>&1; then
  if ! nix-build '<nixpkgs>' -A playwright-driver.browsers --no-out-link &>/dev/null; then
    echo "hint: on NixOS run: nix-shell e2e/opportunities/shell.nix --run $0"
  fi
fi

echo "Building jobdesk..."
cargo build -p jobdesk -q

WS="$ROOT/target/e2e-opportunities-workspace"
export JOBDESK_E2E_WORKSPACE="$WS"
export JOBDESK_E2E_FRESH_WS=1
export JOBDESK_E2E_PORT="${JOBDESK_E2E_PORT:-19547}"
if command -v fuser >/dev/null 2>&1; then
  fuser -k "${JOBDESK_E2E_PORT}/tcp" 2>/dev/null || true
fi

cd "$ROOT/e2e/opportunities"
if [[ -f package-lock.json ]]; then
  npm ci
else
  npm install
fi
if [[ -z "${PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD:-}" ]]; then
  npm run install-browsers
else
  echo "Using bundled browsers (PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1)"
fi
npm test

echo "Rust unit/integration sanity..."
cd "$ROOT"
cargo test --workspace -q
