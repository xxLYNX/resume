#!/usr/bin/env bash
# Live CrowdStrike Workday CXS ingest smoke (requires network).
# Uses an isolated workspace only — never touches workspace/jobdesk.db.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WS="$ROOT/target/milestone-n-workday-live-ws"
DB="$WS/jobdesk.db"
OWNER_DB="$ROOT/workspace/jobdesk.db"
PORT=3849
BASE="https://crowdstrike.wd5.myworkdayjobs.com/en-US/crowdstrikecareers"

URLS=(
  "$BASE/job/USA---Rancho-Cordova-CA/Data-Center-Technician--Santa-Clara--CA-_R30561"
  "$BASE/job/United-Kingdom---Remote/Technical-Support-Engineer---Cloud--Remote--GBR-_R29756"
  "$BASE/job/USA---Remote/Detection-Engineer--Falcon-Complete--Remote-_R30634"
  "$BASE/job/Canada---Remote-BC/Detection-Engineer--Falcon-Complete--Remote-_R30635"
  "$BASE/job/USA---Redmond-WA/Sr-Software-Engineer-II---Sensor--Linux-Guardian--Hybrid-_R30636-1"
  "$BASE/job/USA---Remote/Senior-Product-Security-Engineer--Application-Security--Remote-_R29568-1"
)

owner_posting_count() {
  if [[ -f "$OWNER_DB" ]]; then
    sqlite3 "$OWNER_DB" "SELECT COUNT(*) FROM postings;" 2>/dev/null || echo "missing"
  else
    echo "no-db"
  fi
}

rm -rf "$WS"
mkdir -p "$WS"
test ! -f "$DB"

# Migrate schema only in the isolated workspace (no catalog import).
cargo run -q -p jobdesk -- --root "$ROOT" --workspace "$WS" application list >/dev/null
test -f "$DB"

OWNER_BEFORE="$(owner_posting_count)"
echo "Owner postings before: $OWNER_BEFORE (db: $OWNER_DB)"
echo "Isolated workspace: $WS"

cargo build -q -p jobdesk
cargo run -q -p jobdesk -- --root "$ROOT" --workspace "$WS" serve --host 127.0.0.1 --port "$PORT" &
PID=$!
trap 'kill $PID 2>/dev/null || true' EXIT
for _ in 1 2 3 4 5 6 7 8 9 10; do
  if curl -sf "http://127.0.0.1:$PORT/api/health" >/dev/null; then
    break
  fi
  sleep 1
done
curl -sf "http://127.0.0.1:$PORT/api/health" >/dev/null

for u in "${URLS[@]}"; do
  echo "=== INGEST $u"
  curl -s -X POST "http://127.0.0.1:$PORT/api/opportunities/ingest" \
    -H 'Content-Type: application/json' \
    -d "$(printf '{"url":"%s"}' "$u")" | head -c 400
  echo
done

OWNER_AFTER="$(owner_posting_count)"
if [[ "$OWNER_BEFORE" != "$OWNER_AFTER" ]]; then
  echo "FAIL: owner posting count changed ($OWNER_BEFORE -> $OWNER_AFTER)" >&2
  exit 1
fi

echo "=== SUMMARY (isolated DB only)"
sqlite3 "$DB" "SELECT p.normalized_url,
  (SELECT outcome FROM posting_fetch_attempts a WHERE a.posting_id=p.id ORDER BY attempted_at DESC LIMIT 1),
  (SELECT retrieval_kind FROM posting_snapshots s WHERE s.posting_id=p.id ORDER BY fetched_at DESC LIMIT 1)
FROM postings p ORDER BY p.updated_at;"

echo "Applications in smoke ws: $(sqlite3 "$DB" "SELECT COUNT(*) FROM applications;") (must be 0)"
echo "Owner postings after: $OWNER_AFTER (unchanged)"
