# Job search workspace — running checklist

Living plan for evolving this repo from a LaTeX resume builder into a **local-first job-search workspace**: resume generation (one component), application tracking (“backwards ATS”), skills catalog, and eventually portfolio surfaces. Update this file as items ship or priorities change.

---

## Goals (what “done enough” looks like)

- [ ] **Single source of truth** for career facts (jobs, education, projects, skills, bullets) with stable IDs—not duplicated YAML per variant.
- [ ] **Applications** with posting URL + composition (entry + **revision** ids per element); optional **presets**; **submissions** store immutable **manifest** (reconstruct resume for interview prep); PDF optional.
- [ ] **Application log** that records where you applied, which profile/PDF hash you used, and status—without requiring job boards to integrate.
- [ ] **Optional portfolio** views derived from the same catalog (web static site or simple local preview).
- [ ] **Free, self-hosted, no vendor lock-in**; paid services optional, not required.

Non-goals for early phases:

- [ ] Documented: no automated apply/submit to job boards (fragile, ToS, low ROI).
- [ ] Documented: no LLM-generated cover letters as a core feature (see [Cover letters & ATS](#cover-letters--ats-notes)).

---

## Architecture decisions (record choices here)

| Topic | Options | Decision | Notes |
|-------|---------|----------|-------|
| Core language | Go (current), Rust (rewrite) | **Rust** | See [Rust rewrite](#rust-rewrite--uniffi) |
| Career catalog on disk | YAML files, JSON, SQLite, hybrid | **SQLite canonical + YAML/JSON export/import** | Edit in external editor via export, or UI writes DB |
| Resume PDF engine | system `pdflatex`, Tectonic, both | **Tectonic primary** | See [PDF engine](#pdf-engine-pdflatex-vs-tectonic); optional dev fallback |
| UI (if any) | CLI only, TUI, local web, desktop/mobile via UniFFI | **Local web app** | 2-pane: edit left, PDF preview right; CLI for scripts/CI |
| Templates | LaTeX (current), HTML→PDF later | **LaTeX + MiniJinja** | Optional fields per element; see [LaTeX templating](#latex-templating-rust) |

---

## Glossary: catalog, revisions, presets, applications, submissions

You are not missing anything obvious: **the thing that matters for a job posting is the application + what you actually submitted**, not an abstract “profile.”

### Mental model (bottom line)

| Concept | What it is | Required? |
|---------|------------|-----------|
| **Catalog entry** | Stable identity for a fact (`project.homelab`, `experience.affinity-2024`) | Yes |
| **Entry revision** | Wording snapshot for that entry (`project.homelab` rev 1, rev 2, …) | Yes, for your interview use case |
| **Application** | A posting you applied to (URL, company, title, dates, status) + **which elements/revisions** were chosen for that attempt | Yes |
| **Submission** | **Immutable** record of the resume as sent (PDF + resolved manifest + hash)—frozen at apply time | Yes |
| **Preset** (optional; was called “profile”) | Reusable selection template (“infosec one-pager”) to **copy into** a new application without re-clicking every entry | No |

**Profile** in earlier drafts = **preset**. It is **not** the same as an application. You might never name presets if you prefer to configure each application from scratch; presets only save time when many postings share the same shape.

### Application (what you assumed — correct)

An **application** row is roughly:

- **Posting identity:** URL (primary), plus company, title, location, source (LinkedIn, Greenhouse, etc.).
- **Resume composition (content for this posting):** which **sections/collections** appear, and which **catalog elements** are included in each (each pin is **entry id + revision id** — whole element, not bullet-level). Same content can be reused across applications; **theme** is chosen separately and does not drive inclusion.
- **Theme** id used for that build (layout/templates only).
- **Lifecycle:** draft → submitted → rejected/interview/offer/withdrawn.
- **Link to submission(s):** you may submit twice (e.g. corrected typo)—each submit creates a new **submission** row; mark one as canonical for “what they saw.”

```yaml
# Illustrative — stored in SQLite, not necessarily hand-edited YAML
application:
  id: app-2026-03-acme
  posting_url: "https://careers.acme.example/jobs/123"
  company: Acme Corp
  title: Security Engineer
  status: interview
  composition:
    theme: classic-resume
    sections:
      - experience
        items:
          - entry: experience.affinity-2024
            revision: 2          # whole element pinned; no bullet-level forks
      - projects
        items:
          - entry: project.homelab
            revision: 3
  submissions:
    - id: sub-001
      submitted_at: 2026-03-10T14:22:00Z
      manifest: artifacts/sub-001.resolved.json   # required: enough to reconstruct
      pdf: artifacts/sub-001.pdf                  # optional: cache/print only
      content_hash: sha256:…
```

### Entry revisions (rewording without losing history)

- **`project.homelab`** is the identity (“this project exists in my career”).
- **Revisions** are numbered or id’d snapshots of text (`rev-1`, `rev-2`, or `homelab-2026-03-phrasing`).
- Editing for a new application usually means **create new revision**, not overwrite (overwrite only if nothing was ever submitted with the old text).
- Query: “all applications that included `project.homelab`” → join on entry id (any revision).
- Query: “what wording did Acme see?” → read **submission manifest**, not live catalog.

Rule of thumb: **submissions are immutable**; **catalog revisions are append-only**; **applications point at specific revision ids** at submit time.

### Submission record (interview prep — reconstruct, don’t require PDF)

A **content hash of today’s live catalog is not enough** after you keep editing. You need a frozen **submission** that can **re-render the same resume** without guessing.

**Primary artifact: resolved manifest** (JSON). Pick application → load its canonical submission → resolve from manifest → preview in UI (same pipeline as live draft). No need to open a stored PDF unless you want to print or keep a byte-identical file.

At submit time, persist:

1. **Resolved manifest** (required): composition (entry id + **element revision** id per item), theme id, **theme/template bundle version** (or hash of template files used), skill alias choices, and **denormalized text** (exact bullet strings as sent—so you can read interview prep even if a revision row were ever corrupted).
2. **Content hash** of the manifest (integrity; “draft vs last submission” warning).
3. **PDF** (optional): cache of last render for printing; can always regenerate from manifest + current Tectonic (layout may shift slightly if the engine/fonts change—text comes from manifest).

**Reconstruction requirements:**

- Revisions are **append-only** (never edit rev 3 in place after submit).
- Submission pins **one revision per included element** (whole project, whole job, whole education row)—**not** bullet-level pins.
- Templates/themes are **versioned** or manifest stores **template bundle hash** so re-render doesn’t silently pick up today’s edited `.tex`.

UI: Application log search → “View as submitted” → read-only preview from manifest (PDF export button if desired).

### Similar applications & quickfill

- [ ] Store optional **tags/category** on applications (e.g. `offensive-security`, `backend`, `fed-contractor`) for human + fuzzy matching.
- [ ] **Similarity:** compare compositions (Jaccard on entry ids, same theme, overlapping skill tags) and/or posting title keywords.
- [ ] **Quickfill:** “New application → start from similar” copies composition + revisions from best match; user adjusts before submit.
- [ ] Show hint: “~85% overlap with Application X (Company Y)” when drafting.

### Preset (“profile”) — optional convenience

Same shape as `composition` + theme on an application, but **without** a posting URL—saved as `preset: infosec-one-pager` to clone when a new job looks similar.

Today’s `config.yaml` is a **preset** that includes whole section files with no per-entry/revision picking. Phase 2 replaces that with explicit entry + revision ids.

---

## PDF engine: pdflatex vs Tectonic

You do **not** need both for normal operation. Pick **one primary** for releases; an optional second engine is only for developer fallback or exotic LaTeX packages.

| | **pdflatex** (TeX Live) | **Tectonic** |
|---|------------------------|--------------|
| **Install** | Large system package (`texlive-*`) | Single binary; fetches deps into a cache |
| **Cross-platform ship** | Hard to bundle; “install TeX Live first” | Fits “download our CLI and it works” |
| **Package coverage** | Very broad (anything on CTAN) | Large but not identical; edge packages may fail |
| **Speed** | Mature; often fast after warm cache | Good; first run may download files |
| **Reproducibility** | Depends on user’s TeX Live version | Bundled versions + cache → more predictable |
| **Your repo today** | What `main.go` calls | Not wired yet |

**Recommendation:** **Tectonic as the only engine users need** in packaged builds. Optional: if a template uses a package Tectonic lacks, either add it to the vendored bundle or document “devs may use `pdflatex` locally” until the theme is adjusted— not “ship both and pick at random.”

**Why people mention both:** developers already have TeX Live and use `pdflatex` while iterating; CI/release uses Tectonic. That’s convenience, not a product requirement.

---

## LaTeX templating (Rust)

Your templates today use Go **`text/template`** (e.g. `{{- range .education}}` in `templates/education.tex`). In Rust you pick a **runtime** engine if templates stay user-editable files on disk, or a **compile-time** engine if templates are fixed when you build the app.

| Engine | Style | Best for | Tradeoffs |
|--------|--------|----------|-----------|
| **MiniJinja** | Jinja2-like (`{% for %}`, `{{ }}`) | User-editable `.tex` on disk | Closest mental model to Jinja; actively used for doc gen; syntax differs slightly from Go templates—migration pass on templates |
| **Tera** | Jinja2-like | Same as above | Mature in Rust; similar to MiniJinja—compare ergonomics and compile times |
| **Askama** | HTML-ish, compile-time | Fixed templates in crate | Fast, type-safe; ** awkward for “drop a .tex file in templates/”** without rebuild |
| **Handlebars** | Mustache-style | Simple substitution | Weaker for complex section logic |

**Decision:** **MiniJinja** for user-editable `.tex` on disk. One-time migration from Go `text/template` to Jinja `{% for %}` / `{% if %}`.

**Optional fields (do not assume every element field exists):**

- Catalog schema: fields like `gpa`, `highlights`, `link` are **optional** per entry type; validation is per **entry kind**, not global.
- Render model passes missing keys as empty / absent; templates **must** guard: `{% if entry.gpa %}…{% endif %}`.
- Themes may omit entire sections or use alternate section templates that expect different shapes—resolver only passes data; template decides visibility.
- Document per-theme **field usage** so the UI doesn’t force GPA when the active education template ignores it.

**CLI vs browser:** the same `render-latex` crate runs from CLI or from the local server after a save—one implementation.

---

## Local UI: browser vs terminal split-pane

**Browser (local server)** is the right default for edit + PDF preview:

- PDF preview in-pane (**PDF.js** or `<iframe>`) is standard.
- Monaco or CodeMirror for YAML/Markdown; forms later for structured edits.
- **WebSocket or SSE**: save → rebuild → push new PDF URL/hash to the right pane.
- Bind to `127.0.0.1` only; no auth needed if not exposed.

**Terminal 2-pane** is possible but a poor primary UX:

- TUI editors (ratatui) don’t render PDF; you’d split tmux with an external viewer or a terminal image protocol—fragile across OS/terminals.
- You’d still maintain preview logic separately unless the TUI shells out to the same server.

**Practical split:** one **local web UI** for editing and preview; thin **CLI** (`build`, `export`, `application add`) for scripts and CI. Skip a full TUI unless you want offline keyboard-only use later.

Suggested flow:

1. `jobdesk serve` (name TBD) opens `http://127.0.0.1:PORT`.
2. Left: file tree or tabs (catalog export paths, profile YAML, optional raw template).
3. Right: PDF preview + “last build” log + export hash.
4. Optional: “Open in $EDITOR” for users who prefer Neovim for long prose; UI reloads on file watch.

---

## Easy to miss (add to roadmap)

- [ ] **Schema migrations** for SQLite (`refinery`, `sqlx migrate`) once catalog lives in DB.
- [ ] **Backup story**: periodic `export yaml` / git commit of exports; document “DB is canonical, git is backup.”
- [ ] **Conflict rule**: UI vs hand-edited export—last-write-wins or “import overwrites DB” prompt.
- [ ] **Debounced rebuild** in UI (300–500 ms) so typing doesn’t spawn 10 Tectonic runs.
- [ ] **Build errors in UI**: surface LaTeX log tail, not only “failed.”
- [ ] **Privacy**: application tracker may hold salary, recruiter emails—local-only, encrypt-at-rest optional later.
- [ ] **Objective/summary per profile** (short text variants), not only entry filtering.
- [ ] **Accessibility of PDF output** (tags) — low priority for most resumes but exists.
- [ ] **Font licensing** if themes ship embedded fonts (OFPL/GUST for TeX fonts is usually fine; verify per theme).

---

## Go → Rust: does anything major change?

**No meaningful change to the product options.** Rust mainly affects *how* you ship and maintain the core, not *what* you can build.

| Capability | Go (today) | Rust | Comment |
|------------|----------|------|---------|
| YAML/JSON config | `yaml.v3` | `serde` + `serde_yaml` / `serde_json` | Equivalent |
| SQLite tracker | `database/sql`, `modernc.org/sqlite` | `rusqlite`, `sqlx` | Rust ecosystem is strong |
| LaTeX templating | `text/template` | `minijinja`, `tera`, `askama` | Pick one; minijinja is close to Jinja ergonomics |
| Watch / rebuild | fsnotify (add) | `notify` crate | Equivalent |
| Embed assets | `embed` | `rust-embed` / `include_str!` | Equivalent |
| Ship one CLI binary | Straightforward | Straightforward | Rust compile times longer; runtime fine |
| Vendored PDF (Tectonic) | Subprocess | Subprocess or `tectonic` crate | Tectonic is Rust-native—nice alignment |
| Shared core for future apps | Cgo / separate processes | **UniFFI** → Kotlin, Swift, Python | Main *new* option with Rust |

**What Rust does *not* magically solve:** job boards still won’t expose a unified API; you still need hashes + manual “I applied here” records; portfolio and resume still need a presentation layer.

**Suggested Rust crate layout (sketch):**

```
crates/
  catalog/      # IDs, entries, skills graph, validation
  profile/      # resolve profile → render model
  render-latex/ # templates → .tex
  render-pdf/   # invoke tectonic/pdflatex
  tracker/      # applications, exports, hash linkage
  cli/          # binary entrypoint
  uniffi-bindings/  # optional; thin API over catalog + profile + tracker
```

---

## Data storage: SQLite vs JSON vs YAML

Use **different stores for different jobs** (hybrid is OK):

| Use case | YAML/JSON on disk | SQLite |
|----------|-------------------|--------|
| Human-edited resume chunks | Excellent (git diff, hand edit) | Poor |
| Application tracker (many rows, queries) | Awkward | Excellent |
| Skills catalog + aliases | Good as files; fine in DB | Good if you want queries (“all tags offensive”) |
| Profile definitions | Good (one file per job target) | Optional |
| “What PDF did I send?” | Needs discipline | **Hash + FK to application row** |

**Recommended path:**

1. [ ] **Phase A:** Keep YAML content during migration; Rust loader validates and assigns stable IDs.
2. [ ] **Phase B:** Introduce SQLite for `applications`, `exports` (pdf path, content hash, profile id, timestamp).
3. [ ] **Phase C:** Optional migration of catalog into SQLite with YAML export for git-friendly backups—or keep catalog as YAML forever and only track apps in DB.

Content hash for sync (minimal spec):

- [ ] Define hash input: canonical JSON of **resolved submission manifest** (element revision ids + denormalized text + theme/template bundle version).
- [ ] Store on **submission** row: `sha256`, `application_id`, `manifest_path` (required), `pdf_path` (optional cache), `submitted_at`.
- [ ] UI: draft composition may drift; warn if live draft hash ≠ last submission hash for that application.

---

## Cover letters & ATS (notes)

- [ ] **ATS (employer-side)** mostly parses structure and keywords; it is not a reliable “LLM detector” in the general case.
- [ ] **Humans** often recognize generic cover letters; **LLM-polished** prose can hurt if it sounds boilerplate or misstates the role.
- [ ] If cover letters are added later: treat them as **structured snippets** (paragraphs you own) merged per application—not auto-generated defaults.

---

## Skills association (lightweight catalog)

- [ ] Schema: `id`, `label`, `aliases[]`, `tags[]`, optional `framework_refs` (e.g. NICE KSAs).
- [ ] Compositions/presets filter by `tags` and emit **posting-friendly labels** (alias chosen for JD keywords).
- [ ] De-dupe rule: one canonical id; never store “pentest” and “penetration testing” as separate skills in the catalog.
- [ ] Optional later: import NICE/ESCO mappings as enrichment, not as v1 blocker.

---

## Phase 0 — Inventory & migration prep (current Go repo)

- [ ] Wire `-config` flag in Go (docs still mention it) **or** fix EXAMPLES/README to say `config.yaml` only.
- [x] Pre–Milestone C review — [`REVIEW.md`](REVIEW.md); import/export/build polish.
- [ ] Export current content into a **manifest** listing implicit IDs (file + index → stable id).
- [ ] Capture sample outputs: `resume.tex`, PDF, for diff testing after rewrite.
- [ ] Decide repo name / scope (resume-only repo vs monorepo for tracker + portfolio).

---

## Phase 1 — Rust core: parity with today

- [x] Cargo workspace (`jobdesk` binary, see `ARCHITECTURE.md`).
- [x] Load `settings/jobdesk.yaml` + section YAML (Go keeps `config.yaml`).
- [x] Render LaTeX via `themes/classic/` MiniJinja templates; optional fields use `{% if %}`.
- [x] Invoke PDF engine (auto-detect tectonic/pdflatex) via `build --pdf`.
- [x] CLI: `import`, `build` — `clean`/`validate`/`serve` still TODO.
- [x] Integration test: golden compare normalized `.tex`.
- [ ] MSRV policy documented.
- [ ] `clean`, `validate` parity with Go CLI.

---

## Phase 2 — Catalog, revisions, compositions (resume component “done right”)

- [ ] Stable entry IDs for education, experience, projects, bullets, skills.
- [ ] **Append-only revisions** per **element** (one revision = full wording of that project/job/education row); UI to “fork wording” → new revision.
- [ ] **Composition**: sections, entry + element revision selection, skill tag filters, theme id (used by applications and optional presets).
- [ ] Resolver produces render model (templates unchanged or slowly migrated).
- [ ] Draft PDF preview from current composition; **submission** writes immutable manifest (+ optional PDF cache).
- [ ] “View as submitted” re-renders from manifest (interview prep path).
- [ ] Themes: preamble + section template set; one theme per composition (section-level theme override deferred).

---

## Phase 3 — Application tracker (“backwards ATS”)

- [ ] SQLite schema: companies, applications, stages, notes, links.
- [ ] Application: posting URL, company, title, status, composition (entry + revision ids).
- [ ] Link application → one or more **submission** rows (manifest + hash; optional pdf).
- [ ] Similar-composition hints + quickfill from past applications (tags + entry-set overlap).
- [ ] CLI: `application add`, `list`, `status`, `show <id>`.
- [ ] Optional: import CSV; export for backup.
- [ ] No requirement to integrate with LinkedIn/Indeed APIs v1.

---

## Phase 4 — Local web UI & packaging

- [x] `serve`: bind localhost, 2-pane shell + PDF preview (iframe; PDF.js optional later).
- [x] Debounced rebuild on load + manual rebuild; error tail in left pane.
- [ ] File watch → auto-rebuild; WebSocket/SSE optional.
- [x] Catalog editor in left pane (list, revision picker, JSON editor, save → export).
- [ ] `watch` mode still available headless for non-UI users.
- [ ] Document Tectonic bundling per OS/arch; CI builds release artifacts.
- [ ] Embed default templates in binary; override with user dir.

---

## Phase 5 — Portfolio (optional surface)

- [ ] Define subset of catalog flagged `portfolio: true`.
- [ ] Static site generator (Rust: `leptos`/`axum` build, or simple HTML templates).
- [ ] Share deploy story (GitHub Pages, self-host)—documentation only unless automated.

---

## Phase 6 — UniFFI (optional, when a native UI is worth it)

- [ ] Extract `catalog`, `profile`, `tracker` APIs behind UniFFI-safe types (no strings leaking invariants).
- [ ] Generate bindings for chosen target (Swift/Kotlin/Python).
- [ ] Thin shell app: view applications, trigger build, open PDF—logic stays in Rust.
- [ ] CI job to verify bindings compile.

---

## Open questions

- [ ] Monorepo vs split: `resume-engine` crate vs this repo name?
- [x] Git-only catalog vs DB-only with export—**SQLite canonical; edit via UI or exported YAML in external editor, then import/sync**
- [x] Primary UX—**local browser 2-pane**; CLI for automation
- [ ] Portfolio: public repo of content or private catalog + public render?
- [ ] License for any bundled Tectonic/fonts?

---

## Changelog (checklist meta)

| Date | Change |
|------|--------|
| 2026-09-17 | Initial checklist: scope, Rust/UniFFI, storage, phases 0–6 |
| 2026-09-17 | Glossary (profiles), PDF/templating/UI notes, “easy to miss” items |
| 2026-09-17 | Reframe: application + submission + entry revisions; preset optional |
| 2026-09-17 | Reconstruct-from-manifest; element-level pins; MiniJinja; optional template fields; quickfill |
