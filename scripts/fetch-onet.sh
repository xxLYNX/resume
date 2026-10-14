#!/usr/bin/env bash
# Download O*NET Database text files (CC BY 4.0). Do not commit the downloaded tree.
# See data/ONET_IMPORT.md and docs/ONET_ATTRIBUTION.md
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="${ONET_DB_VERSION:-db_30_0_text}"
BASE="https://www.onetcenter.org/dl_files/database/${VERSION}"
DEST="${ROOT}/data/onet/${VERSION}"
mkdir -p "$DEST"

fetch() {
  local name="$1"
  local url="${BASE}/${name// /%20}"
  echo "Fetching ${name}…"
  curl -fsSL "$url" -o "${DEST}/${name}"
}

fetch "Technology Skills.txt"
fetch "Skills.txt"
if curl -fsSI "${BASE}/Software%20Skills.txt" | head -1 | grep -q "200"; then
  fetch "Software Skills.txt"
else
  echo "Note: Software Skills.txt not published at ${BASE}; Technology Skills.txt is sufficient."
fi

echo "Done. Import with:"
echo "  cargo run -p jobdesk -- skills import-catalog --onet-dir ${DEST} --replace --yes"
