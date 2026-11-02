# Product checklist

Living goals for **jobdesk**. Implementation status and milestones: [`ARCHITECTURE.md`](ARCHITECTURE.md).

## Goals

- [x] **Single source of truth** — SQLite catalog with stable element ids and append-only revisions
- [x] **Applications** — posting metadata + per-job **composition** (element + revision pins)
- [x] **Submissions** — immutable manifest (+ theme snapshot hash) for interview prep
- [x] **Presets** — reusable compositions without a posting row
- [ ] **Portfolio** surfaces (Milestone G)
- [x] **Local-first** — `jobdesk serve` + CLI; no required cloud

Non-goals: automated job-board apply; LLM cover letters as core product. Optional user-reviewed application Q&A drafts and coaching notes (Milestone K) may be stored locally; generative coaching runs via MCP in your editor, not as a required cloud service.

## Glossary

| Term | Meaning |
|------|---------|
| **Element** | One catalog record (`experience.acme-2024`, `project.foo`, …) |
| **Revision** | Append-only JSON payload for an element |
| **Composition** | Which sections and elements (with revision ids) belong on a resume for one application or preset |
| **Application** | A job posting you track + its draft composition |
| **Submission** | Frozen manifest (and optional PDF) at **Submit** time |
| **Theme** | LaTeX + MiniJinja bundle; independent of which elements you include |
| **Evidence** | Tools, competencies, and tags linked to portfolio anchors via `element_skill_links` |
| **Toolbelt** | PDF section projecting deduped evidence labels from pinned composition items |

## Collections

**Shipped element kinds:** `header`, `objective`, `education`, `experience`, `project`, `course`, `certification`, `award` (see `ElementKind` and theme section maps).

**Skills on PDF:** evidence-linked **toolbelt** + optional project line tags — not a separate composable `skills` blob.

**Milestones E, F, I & J:** complete on `master`. **Active:** **K** application intelligence ([`docs/MILESTONE_K_APPLICATION_INTELLIGENCE.md`](docs/MILESTONE_K_APPLICATION_INTELLIGENCE.md)); **G** portfolio export queued.

**Planned later:** separate **tools** collection kind (if split from evidence model); richer public portfolio (G).

## Where to edit data

| Task | Path |
|------|------|
| Edit existing element text | **Portfolio** tab in `serve` (new revision) |
| Add new jobs, projects, … | Portfolio **New entry** in `serve`, or `jobdesk catalog add … --payload file.json` |
| Seed / bulk YAML | `content/*.yaml` then `jobdesk import` |
| Backup / git-friendly dump | `jobdesk export` → `workspace/staging/content/` |
| Per-job resume content | **Resume** tab (target application) + Applications tracker |
| Posting vs catalog analysis | Applications → **Analyze**; CLI `jobdesk application analyze` |
| Layout / LaTeX | `themes/<id>/` |
