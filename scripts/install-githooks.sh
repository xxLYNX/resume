#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
chmod +x "$root/githooks/commit-msg"
cp "$root/githooks/commit-msg" "$root/.git/hooks/commit-msg"
echo "Installed commit-msg hook (strips Co-authored-by trailers)."
