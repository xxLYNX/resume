# jobdesk (Rust)

Local job-search workspace CLI. See [`ARCHITECTURE.md`](ARCHITECTURE.md) for design and milestone checklists.

## Prerequisites

- Rust toolchain (`cargo`)
- PDF (optional): [Tectonic](https://tectonic-typesetting.github.io/) or TeX Live (`pdflatex`)

## Commands

From the repository root:

```bash
# Import content/*.yaml into workspace/jobdesk.db (skips existing elements)
cargo run -p jobdesk -- import
cargo run -p jobdesk -- import --force   # append file content as new revisions

# Export DB → workspace/staging/content/ (does not touch repo content/)
cargo run -p jobdesk -- export

# Optional: also mirror export into repo content/ (overwrites seed YAML)
cargo run -p jobdesk -- export --mirror-repo-content

# Build resume from SQLite + settings/jobdesk.yaml (requires import first)
cargo run -p jobdesk -- build

# Also compile PDF when a engine is installed
cargo run -p jobdesk -- build --pdf

# Local preview UI (http://127.0.0.1:3847)
cargo run -p jobdesk -- import   # required once for catalog pane
cargo run -p jobdesk -- serve
cargo run -p jobdesk -- serve --open
```

**Canonical data** lives in `workspace/jobdesk.db`. Repo `content/*.yaml` is **import seed only**; preview and `build` resolve sections from the catalog (no auto-sync to `content/`). Use `export` to inspect or backup YAML under `workspace/staging/content/`.

**Applications (Milestone C):** use the **Applications** tab in `serve` — create a job, edit draft composition, **Submit** freezes a manifest under `workspace/artifacts/manifests/`, switch preview to **As submitted** for interview prep.

See [`REVIEW.md`](REVIEW.md) for deferred polish (presets, quickfill, structured forms).

Options:

- `--root .` — project root (default: current directory)
- `--workspace workspace` — SQLite and artifacts directory (default: `workspace/`)

## Tests

```bash
cargo test
```

Golden LaTeX output is compared to `tests/fixtures/golden_resume.tex` (from the original Go builder) with whitespace normalization.

## Go builder

Unchanged: `go run main.go` reads **`config.yaml`** and **`templates/`** (Go `text/template`). jobdesk reads **`settings/jobdesk.yaml`** and **`themes/classic/`** (MiniJinja). The original theme under `templates/` is not modified by jobdesk work.
