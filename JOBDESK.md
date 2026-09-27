# jobdesk CLI

Local workspace binary. Design and milestones: [`ARCHITECTURE.md`](ARCHITECTURE.md).

## Prerequisites

- Rust (`cargo`)
- PDF (optional): Tectonic or `pdflatex`
- **Git hooks (recommended):** `./scripts/install-githooks.sh` — strips unwanted `Co-authored-by` trailers from commits in this repo.

## Commands

From the repository root:

```bash
cargo run -p jobdesk -- import
cargo run -p jobdesk -- import --force   # append YAML as new revisions for existing elements

cargo run -p jobdesk -- export            # workspace/staging/content/
cargo run -p jobdesk -- export --mirror-repo-content   # also overwrite repo content/ (intentional)

cargo run -p jobdesk -- build
cargo run -p jobdesk -- build --pdf
cargo run -p jobdesk -- build --application <uuid>
cargo run -p jobdesk -- build --application <uuid> --preview-mode submitted

cargo run -p jobdesk -- serve
cargo run -p jobdesk -- serve --open
```

**Data:** `workspace/jobdesk.db` is canonical. `content/*.yaml` is import seed; use `import` / `export` when you want file round-trips.

**Applications** (tracker + composition + submit) are easiest in `serve`. CLI:

```bash
cargo run -p jobdesk -- application list
cargo run -p jobdesk -- application list --status interview --tag sec
cargo run -p jobdesk -- application add "Acme" "Engineer" --posting-url "https://…"
cargo run -p jobdesk -- application add "Acme" "Engineer" --clone-from <uuid>
cargo run -p jobdesk -- application show <uuid>
cargo run -p jobdesk -- application set-status <uuid> interview
cargo run -p jobdesk -- application update <uuid> --company "Acme"
cargo run -p jobdesk -- application submit <uuid>
cargo run -p jobdesk -- application delete <uuid>
cargo run -p jobdesk -- application export-ledger
cargo run -p jobdesk -- application export-ledger --output /path/to/ledger.pdf
```

**Phase vs workflow vs Submit:** [`docs/APPLICATION_LIFECYCLE.md`](docs/APPLICATION_LIFECYCLE.md). Only **Submit** freezes a manifest.

## Options

- `--root .` — project root
- `--workspace workspace` — DB and artifacts directory

## Tests

```bash
cargo test
```
