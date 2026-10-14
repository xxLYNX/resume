# O*NET catalog import

Jobdesk loads the skill **catalog** from an [O*NET Database](https://www.onetcenter.org/database.html) text bundle (29.x+). O*NET content is licensed under [CC BY 4.0](https://www.onetcenter.org/license.html); **do not commit** downloaded files to this repo (see [`docs/ONET_ATTRIBUTION.md`](../docs/ONET_ATTRIBUTION.md)).

## Fetch (recommended)

```bash
chmod +x scripts/fetch-onet.sh
./scripts/fetch-onet.sh
# default: data/onet/db_30_0_text/ (override with ONET_DB_VERSION=db_29_2_text)
```

## Required files

Point `--onet-dir` at the folder containing at least:

| File | Maps to |
|------|---------|
| `Technology Skills.txt` | `tool` nodes (deduped by **Example**) |
| `Software Skills.txt` | optional; same format as Technology Skills |
| `Skills.txt` | `competency` nodes (unique **Element ID** / **Element Name**) |

Tab-delimited rows; first line is the column header (`O*NET-SOC Code`, …).

## CLI

From repo root (default DB: `workspace/jobdesk.db`):

```bash
# Wipe catalog + all evidence links, then import O*NET
cargo run -p jobdesk -- skills import-catalog \
  --onet-dir /path/to/onet/db_29_2_text \
  --replace --yes

# Counts by kind
cargo run -p jobdesk -- skills catalog-stats
```

Optional: clear portfolio evidence only (keeps catalog):

```bash
cargo run -p jobdesk -- skills clear-links --yes
```

## After import

1. Hard-refresh the UI (`Ctrl+Shift+R`).
2. Re-tag portfolio entries via evidence chips (links are not migrated from the demo catalog).
3. Custom **markers** (`tag.*`) are not imported; create them in the UI as needed.

## Dev fixture

`data/catalog_seed.json` remains for tests and `jobdesk skills import`; fresh DBs still upsert it once at migration **v12**. Production-sized catalogs use `import-catalog --replace`.
