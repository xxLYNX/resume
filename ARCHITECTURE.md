# Architecture — local job-search workspace

Companion to [`CHECKLIST.md`](CHECKLIST.md). This doc is the **design source of truth**; checklist items map to **milestones** below. Check boxes as work lands.

---

## Product shape

- [x] Documented: single Rust binary, local-first, no required cloud
- [x] Default UX: `jobdesk serve` → browser on `127.0.0.1`
- [x] Workspace directory: SQLite DB + preview artifacts (manifests later)
- [x] CLI: `import`, `import --force`, `export`, `build`, `serve`

**Default workspace layout** (path configurable, e.g. `./workspace/`):

```
workspace/
  jobdesk.db
  themes/              # user overrides; defaults embedded or shipped beside binary
  artifacts/manifests/
  artifacts/pdf/       # optional cache
  export/              # YAML dumps for git / external editor
```

---

## Domain model (implementation checklist)

### Catalog

- [x] `elements` table: stable `id`, `kind`, `sort_order`, timestamps
- [x] `element_revisions` table: append-only `payload_json` per element (`rev_number`)
- [x] Element kinds: `header`, `objective`, `experience`, `education`, `project`, `skills`
- [x] Payload schemas: required shapes per `ElementKind` (`validate_payload` on save/import)
- [x] UI/rule: edit wording → **new revision** (import skips existing; `--force` appends)

### Skills (lightweight association) — **Milestone F**

- [ ] `skills` table: `id`, `label`, `aliases_json`, `tags_json`, `framework_refs_json`
- [ ] Resolver expands aliases for display; one canonical id internally
- [ ] Framework refs (O\*NET, NICE KSAs, ESCO URIs, etc.) — see [`docs/MILESTONE_F_SKILLS.md`](docs/MILESTONE_F_SKILLS.md)

### Themes & templates

- [x] Theme bundle: `theme.toml` + `preamble.tex` + `sections/*.jinja.tex` + `macros/` (see [`themes/CONTRACT.md`](themes/CONTRACT.md))
- [x] Snapshot + `theme_files_hash` on submit (workspace copy; not yet a DB registry)
- [x] jobdesk skips sections missing from resolved composition; resolve normalizes optional list fields on rows
- [x] Theme macros registered from `theme.toml` `[macros]` — optional scalars handled in theme toys, not jobdesk LaTeX
- [ ] `theme_bundles` DB registry (optional); build config fully driven by theme bundle only
- [ ] Versioned global filters/macros library for third-party themes (expand contract)

### Applications & submissions

- [x] `applications`: posting URL, company, title, source, **`phase`** + workflow **`status`**, tags, `draft_composition_json` (see [`docs/APPLICATION_LIFECYCLE.md`](docs/APPLICATION_LIFECYCLE.md))
- [x] `submissions`: immutable `manifest_path`, `content_hash`, optional `pdf_path`, `is_canonical`
- [x] Submit flow: resolve draft → write manifest (denormalized text + revision ids + theme hash)
- [x] Interview prep: pick application → render from **manifest** (read-only), PDF optional
- [x] `presets` table: saved compositions without posting URL
- [x] **Application content picker** UI: per application, enable/disable **sections (collections)** and **include toggles per catalog element**; preview/submit render only included elements
- [ ] **Submission history UI** (deferred post-E): list manifests per app; preview any past send
- [ ] **Future collections:** tools, certifications, awards as separate kinds/sections (deferred; skills blob covers certifications text today)

### Similarity & quickfill

- [x] Application tags (manual v1)
- [x] Similarity score: Jaccard on element ids + tag overlap + title tokens (`similarity` crate)
- [x] Quickfill: clone draft composition from top match (UI + API + `clone_from` on create)

---

## Submission manifest (schema v1)

- [x] `schema_version`
- [x] `submitted_at`, `theme_id`, `theme_files_hash`
- [x] `composition`: section order, `{ element_id, revision_id }` per item (element-level pin only)
- [x] `resolved`: denormalized MiniJinja contexts per section
- [x] `snapshot_hash` + `content_hash`: SHA-256 of canonical JSON

---

## Render pipeline

- [x] `resolve`: composition + catalog **or** manifest → section JSON for templates
- [x] `render_tex`: MiniJinja + theme → `resume.tex`
- [x] `render_pdf`: Tectonic / pdflatex detect + compile (concrete enum, not a trait yet)
- [x] Same pipeline for preview, CLI `build`, and manifest reconstruction via `jobdesk_build_aligned`

**Crate layout** (target):

| Crate | Responsibility | Status |
|-------|----------------|--------|
| `domain` | Types, composition, manifest, legacy config | [x] |
| `store` | SQLite, migrations, import/export | [x] |
| `resolve` | Catalog / manifest → render model | [x] default + manifest |
| `render_tex` | MiniJinja assembly | [x] legacy build |
| `render_pdf` | Tectonic / pdflatex | [x] detect + compile |
| `similarity` | Quickfill scoring | [x] |
| `server` | axum API + static UI | [x] preview v1 |
| `jobdesk` (cli) | clap entrypoint | [x] `import`, `build` |

---

## Local web UI

- [x] Left: catalog + application editors (JSON v1)
- [x] Right: PDF iframe preview, build log, hash, draft-vs-submission warning
- [ ] PDF.js (optional), SSE/WebSocket refresh
- [x] Application search/filters + “view as submitted” preview mode

---

## Milestone A — Render engine

LaTeX output locked to a golden fixture; catalog resolve + classic theme.

- [x] Cargo workspace scaffold
- [x] `themes/classic/` MiniJinja section templates + preamble + `theme.toml`
- [x] `jobdesk build`: `settings/jobdesk.yaml` + SQLite resolve → `.tex`
- [x] `jobdesk build --pdf`: Tectonic or pdflatex when available
- [x] `jobdesk import`: seed YAML → SQLite elements + revision 1
- [x] Golden test: normalized `.tex` matches `tests/fixtures/golden_resume.tex`

```bash
cargo run -p jobdesk -- import
cargo run -p jobdesk -- build
cargo run -p jobdesk -- build --pdf
```

---

## Milestone B — Minimal UI

Recommended order (each step is usable on its own):

1. [x] **`serve` + preview API** — localhost, `POST /api/preview` runs current `settings/jobdesk.yaml` build + PDF; right pane iframe PDF (debounce on load + manual rebuild).
2. [x] **Catalog read API** — list elements/revisions from SQLite; left pane browser.
3. [x] **Catalog write** — save revision (validated) + preview rebuild from SQLite (no repo `content/` sync).
4. [x] **2-pane polish** — structured catalog forms from `theme.toml` (optional fields by kind).

**Foundation:** `resolve` crate + default composition for preview/build; YAML export to `workspace/staging/content/` only. Per-job composition and applications remain Milestone C.

- [x] `jobdesk serve` on localhost (default port 3847)
- [x] 2-pane: catalog JSON editor + preview PDF
- [x] Applications tab (tracker + draft composition)

---

## Milestone C — Applications + submit

- [x] Application CRUD + draft composition editor (JSON)
- [x] Submit → `workspace/artifacts/manifests/*.json` + DB row
- [x] View as submitted from manifest (preview mode)

---

## Milestone D — Tracker polish

- [x] Search, filters, status workflow (UI + `GET /api/applications?q&status&tag`)
- [x] Tags + quickfill (Jaccard similarity crate; clone on create / quickfill API)
- [x] YAML export to staging (`jobdesk export`); import seed from repo `content/`
- [x] CLI: `jobdesk application list|add|show|set-status`

---

## Milestone E — Composition, themes, catalog UX

**Complete** (merged from `dev/ui-composition`). Summary: [`docs/NEXT_MILESTONE.md`](docs/NEXT_MILESTONE.md#milestone-e-complete).

- [x] Interim composition editor (element + revision rows; JSON in details)
- [x] **Application content picker** — section/collection on-off + include toggles for each catalog element in that collection (content choice per posting; theme separate)
- [x] Theme bundles + `theme_files_hash` on submit (snapshot under `workspace/artifacts/theme_snapshots/`)
- [x] Catalog structured forms from `theme.toml` metadata (JSON toggle remains)
- [x] Presets (saved compositions without a job row)
- [x] Application Save UX — one **Save** for tracker + composition; **Submit** freezes manifest
- [x] Live draft preview + preview dedupe (skip PDF when document unchanged)
- [x] Applications list/detail UI (New application flow, Close back to list)

**Deferred post-E:** submission history UI (list/open past manifests); optional phase/pipeline UI polish.

---

## Milestone F — Skills catalog & framework refs

**Current milestone.** Spec: [`docs/MILESTONE_F_SKILLS.md`](docs/MILESTONE_F_SKILLS.md). Track progress in [`docs/NEXT_MILESTONE.md`](docs/NEXT_MILESTONE.md).

- [ ] Skills table + aliases + tags
- [ ] `framework_refs` (NICE, ESCO, …) + optional import enrichment
- [ ] Composition/resolver uses skill ids + per-job alias choice

---

## Milestone G — Portfolio

- [ ] `show_on_portfolio` on elements
- [ ] Second theme or static HTML export

---

## Milestone H — UniFFI (optional)

- [ ] Thin API over domain + store for native shells

---

## Milestone I — Application ledger & PDF export

**Branch:** `dev/application-ledger` (parallel to F). Spec: [`docs/MILESTONE_I_APPLICATION_LEDGER.md`](docs/MILESTONE_I_APPLICATION_LEDGER.md).

- [x] Application fields: employment type, work mode, location, compensation (hourly/annual, range)
- [x] Ledger PDF export to user Downloads folder (CLI + UI)
- [x] Domain validation (location when on-site/hybrid; salary conversion display rules)

---

## Import seed

- [x] Map `content/*.yaml` list items → stable element ids (`jobdesk import`)
- [ ] `settings/jobdesk.yaml` section order → default preset (optional)

---

## Cross-reference

| Doc | Role |
|-----|------|
| `CHECKLIST.md` | Product goals, glossary, phased todos |
| `ARCHITECTURE.md` | Structure, schema, milestones (this file) |

---

## Changelog

| Date | Change |
|------|--------|
| 2026-09-17 | Initial architecture + milestone checklists |
| 2026-09-17 | Milestone A: Rust workspace, jobdesk import/build, classic theme |
| 2026-09-17 | Milestone B step 1: `serve`, preview API, embedded UI |
| 2026-09-26 | Removed legacy Go builder; jobdesk-only docs and product surface |
| 2026-09-26 | Milestone E complete: content picker, theme contract, unified Save, preview cache, Applications UX |
