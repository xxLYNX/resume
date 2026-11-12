#!/usr/bin/env bash
# Starts JobDesk for Opportunities browser acceptance (fixture posting fetch only).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PORT="${JOBDESK_E2E_PORT:-19547}"
WS="${JOBDESK_E2E_WORKSPACE:-$ROOT/target/e2e-opportunities-workspace}"

if [[ "${JOBDESK_E2E_FRESH_WS:-1}" == "1" ]]; then
  rm -rf "$WS"
fi
mkdir -p "$WS/preview"
if [[ ! -f "$WS/jobdesk.db" ]]; then
  cargo run -p jobdesk -q --manifest-path "$ROOT/Cargo.toml" \
    -- --root "$ROOT" --workspace "$WS" import
fi

exec cargo run -p jobdesk -q --manifest-path "$ROOT/Cargo.toml" -- \
  --root "$ROOT" --workspace "$WS" serve \
  --host 127.0.0.1 --port "$PORT" \
  --acceptance-fixtures "$ROOT/e2e/opportunities/fixtures/manifest.json"
