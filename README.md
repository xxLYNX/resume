# jobdesk

Local-first job search workspace: **catalog** of resume facts (SQLite), **applications** per posting with tailored **composition**, LaTeX/PDF via **themes**, and immutable **submissions** for “what I actually sent.”

## Quick start

**Prerequisites:** Rust (`cargo`). Optional PDF: [Tectonic](https://tectonic-typesetting.github.io/) or TeX Live (`pdflatex`).

```bash
cargo run -p jobdesk -- import          # seed catalog from content/*.yaml (once)
cargo run -p jobdesk -- serve --open    # UI at http://127.0.0.1:3847
```

Canonical data lives in `workspace/jobdesk.db`. Repo `content/` is **import seed only** — edit the catalog in the UI (or `export` / re-import). See [`JOBDESK.md`](JOBDESK.md) for CLI commands.

## How it fits together

| Piece | Role |
|-------|------|
| **Catalog** | Elements (`header.main`, `experience.acme-2024`, …) with append-only **revisions** (JSON payloads) |
| **Composition** | Per application (or preset): which **sections** appear and which **elements + revisions** are included |
| **Theme** | `themes/<id>/` — `theme.toml`, preamble, MiniJinja section templates ([`themes/CONTRACT.md`](themes/CONTRACT.md)) |
| **Build** | Resolve composition → section JSON → `.tex` → PDF (`settings/jobdesk.yaml` lists sections/templates) |
| **Submit** | Freeze manifest + theme snapshot; preview **As submitted** from that snapshot |

**Collections today:** header, objective, education, experience, projects, skills. Separate **tools**, **certifications**, and **awards** collections are planned later (same picker pattern).

## Project layout

```
content/              Import seed YAML (optional; not auto-synced with DB)
settings/jobdesk.yaml Section order + template paths for builds
themes/classic/       Reference theme bundle
workspace/            jobdesk.db, preview artifacts, manifests (local)
crates/               Rust workspace (jobdesk CLI, server, resolve, render, store)
tests/fixtures/       Golden LaTeX fixture for regression tests
```

## Documentation

| Doc | Purpose |
|-----|---------|
| [`JOBDESK.md`](JOBDESK.md) | CLI reference |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | Schema, pipeline, milestones |
| [`docs/APPLICATION_LIFECYCLE.md`](docs/APPLICATION_LIFECYCLE.md) | Phase vs workflow vs Submit |
| [`docs/NEXT_MILESTONE.md`](docs/NEXT_MILESTONE.md) | Milestone F focus + E completion record |
| [`docs/MILESTONE_F_SKILLS.md`](docs/MILESTONE_F_SKILLS.md) | Skills catalog & framework refs (F) |
| [`docs/MILESTONE_I_APPLICATION_LEDGER.md`](docs/MILESTONE_I_APPLICATION_LEDGER.md) | Application ledger metadata + PDF export (I, complete) |
| [`themes/README.md`](themes/README.md) | Writing a theme (short) |
| [`CHECKLIST.md`](CHECKLIST.md) | Product goals and glossary |

## Tests

```bash
cargo test
```

Golden test: catalog resolve + classic theme matches `tests/fixtures/golden_resume.tex` (normalized whitespace).
