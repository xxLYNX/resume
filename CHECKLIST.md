# Product checklist

Living goals for **jobdesk**. Implementation status and milestones: [`ARCHITECTURE.md`](ARCHITECTURE.md).

## Goals

- [x] **Single source of truth** — SQLite catalog with stable element ids and append-only revisions
- [x] **Applications** — posting metadata + per-job **composition** (element + revision pins)
- [x] **Submissions** — immutable manifest (+ theme snapshot hash) for interview prep
- [x] **Presets** — reusable compositions without a posting row
- [ ] **Portfolio** surfaces (Milestone G)
- [x] **Local-first** — `jobdesk serve` + CLI; no required cloud

Non-goals: automated job-board apply; LLM cover letters as core product.

## Glossary

| Term | Meaning |
|------|---------|
| **Element** | One catalog record (`experience.acme-2024`, `project.foo`, …) |
| **Revision** | Append-only JSON payload for an element |
| **Composition** | Which sections and elements (with revision ids) belong on a resume for one application or preset |
| **Application** | A job posting you track + its draft composition |
| **Submission** | Frozen manifest (and optional PDF) at **Submit** time |
| **Theme** | LaTeX + MiniJinja bundle; independent of which elements you include |

## Collections

**Shipped:** `header`, `objective`, `education`, `experience`, `project`, `skills` (see `ElementKind` and `themes/classic/theme.toml`).

**Milestone E & I:** complete on `master`. **Active:** **J** — Portfolio tab + create elements + UX fixes ([`docs/MILESTONE_J_RESUME_LIBRARY.md`](docs/MILESTONE_J_RESUME_LIBRARY.md)). **Next:** **F** — skills + framework refs.

**Planned later:** separate **tools**, **certifications**, **awards** collections.

## Where to edit data

| Task | Path |
|------|------|
| Edit existing element text | **Portfolio** tab in `serve` (new revision) |
| Add new jobs, projects, … | Portfolio **New entry** in `serve`, or `jobdesk catalog add … --payload file.json` |
| Seed / bulk YAML | `content/*.yaml` then `jobdesk import` |
| Backup / git-friendly dump | `jobdesk export` → `workspace/staging/content/` |
| Per-job resume content | Applications tab → content picker |
| Layout / LaTeX | `themes/<id>/` |
