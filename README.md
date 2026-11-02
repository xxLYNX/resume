# jobdesk

Local-first job search workspace: **catalog** of resume facts (SQLite), **applications** per posting with tailored **composition**, LaTeX/PDF via **themes**, and immutable **submissions** for “what I actually sent.”

## Source repository

**Primary remote:** [Forgejo](http://192.168.50.18:3000/epsil0n/resume) — clone, push, and [open issues](http://192.168.50.18:3000/epsil0n/resume/issues) there. Details: [`docs/FORGEJO_MIRROR.md`](docs/FORGEJO_MIRROR.md).

**GitHub** ([xxLYNX/resume](https://github.com/xxLYNX/resume)) is a **read-only mirror** for visibility; do not use GitHub Issues for this project.

## Quick start

**Prerequisites:** Rust (`cargo`). Optional PDF: [Tectonic](https://tectonic-typesetting.github.io/) or TeX Live (`pdflatex`).

```bash
cargo run -p jobdesk -- import          # seed catalog from content/*.yaml (once)
cargo run -p jobdesk -- serve --open    # UI at http://127.0.0.1:3847
```

Canonical data lives in `workspace/jobdesk.db`. Repo `content/` is **import seed only** — edit the catalog in the UI (or `export` / re-import). See [`JOBDESK.md`](JOBDESK.md) for CLI commands.

**MCP (Cursor / agents):** `cargo run -p jobdesk -- mcp serve --workspace workspace/` exposes scoped tools over your local DB (no network by default).

## How it fits together

| Piece | Role |
|-------|------|
| **Catalog** | Elements (`header.main`, `experience.acme-2024`, …) with append-only **revisions** (JSON payloads) |
| **Evidence** | Tools, competencies, tags linked to portfolio entries; **toolbelt** on PDF |
| **Composition** | Per application (or preset): sections + **elements + revisions** + project line tags |
| **Theme** | `themes/<id>/` — `theme.toml`, preamble, MiniJinja section templates ([`themes/CONTRACT.md`](themes/CONTRACT.md)) |
| **Build** | Resolve composition → section JSON → `.tex` → PDF |
| **Submit** | Freeze manifest + theme snapshot; preview **As submitted** from that snapshot |
| **Analyze** | Posting vs catalog/evidence/composition report (Applications tab or `jobdesk application analyze`) |

**Element kinds:** header, objective, education, experience, project, course, certification, award.

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
| [`docs/NEXT_MILESTONE.md`](docs/NEXT_MILESTONE.md) | Active milestone (K) + backlog |
| [`docs/MILESTONE_K_APPLICATION_INTELLIGENCE.md`](docs/MILESTONE_K_APPLICATION_INTELLIGENCE.md) | Analyze + MCP (K) |
| [`docs/MILESTONE_F_ROLLOUT.md`](docs/MILESTONE_F_ROLLOUT.md) | Skills catalog & evidence (F, complete) |
| [`docs/MILESTONE_J_RESUME_LIBRARY.md`](docs/MILESTONE_J_RESUME_LIBRARY.md) | Portfolio CRUD, Resume tab (J, complete) |
| [`docs/MILESTONE_I_APPLICATION_LEDGER.md`](docs/MILESTONE_I_APPLICATION_LEDGER.md) | Application ledger + PDF export (I) |
| [`docs/FORGEJO_MIRROR.md`](docs/FORGEJO_MIRROR.md) | Forgejo primary + GitHub mirror |
| [`themes/README.md`](themes/README.md) | Writing a theme (short) |
| [`CHECKLIST.md`](CHECKLIST.md) | Product goals and glossary |

## Tests

```bash
cargo test
```

Golden test: catalog resolve + classic theme matches `tests/fixtures/golden_resume.tex` (normalized whitespace).
