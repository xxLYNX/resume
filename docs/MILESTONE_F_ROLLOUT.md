# Milestone F — execution roadmap

**Branch:** `dev/milestone-f`  
**Policy:** Not production. Breaking changes OK. **Single build path:** SQLite catalog → resolve → LaTeX/PDF (no parallel file-based content build).

**Legacy re-link checklist (human):** [`LEGACY_SKILLS_INVENTORY.md`](LEGACY_SKILLS_INVENTORY.md) — reads `content/skills.yaml` + element fields; not source of truth.

---

## North star (portfolio → PDF)

| Idea | Rule |
|------|------|
| **Portfolio** | Experience, project, education, course, certification, award entries are **anchors**. |
| **Evidence** | Each anchor links **tools**, **competencies**, and **other tags** via `element_skill_links` (three separate pickers in UI). |
| **Catalog** | `skill_nodes`: `tool` \| `competency` \| `tag` (`tag` = standards / niche labels — not tools or competencies). |
| **Graph** | `tool_competency_edges` + suggestions API (tools only → competencies). |
| **PDF** | **Projection**, not a second content model. **`toolbelt`** section: theme-configurable **title** + flat **`tags: string[]`** (deduped labels from pinned evidence). |
| **Deprecated** | Multi-bucket `skills.{languages, securityTools, …}` template path; composable `skills` sections and `skills.main` elements removed. |

ATS/keywords: prefer narratives + linked tags on anchors, not a separate keyword blob.

---

## Phase map

| Phase | Status | Outcome |
|-------|--------|---------|
| **F.0** | Done | Branch + plan |
| **F.2a** | Done | v10 schema, element links, evidence resolver, portfolio grouping v1 |
| **F.2-cleanup** | Done | Modular UI; `skills.main` retired; v11; file-based build removed |
| **F.2b** | Done | Skill discovery UI |
| **F.2c** | Done | Relationships pane |
| **F.2d** | Done | Cert/award PDF sections; courses under education |
| **F.3a** | Done | `tag` kind, three pickers, **toolbelt** PDF, **`data/catalog_seed.json`** + v12 upsert |
| **F.3b** | Done | Framework refs merge, posting hints + preview |

---

## F.3a — catalog + toolbelt (complete)

- [x] Domain: `SkillKind::Tag` (`tag` / `other` aliases).
- [x] UI: Tools, Competencies, **Other tags** pickers; unmatched payload tokens → `tag.*` nodes.
- [x] Resolve: `inject_toolbelt_from_evidence` → `{ tags: [...] }`; skip composable `toolbelt` / `skills` keys.
- [x] Theme: `[sections.toolbelt]` with `title`; `toolbelt.jinja.tex`; `settings/jobdesk.yaml` uses `toolbelt` not `skills`.
- [x] DB **v12**: upsert bundled catalog from `data/catalog_seed.json` on migrate.
- [x] CLI: `jobdesk skills import <file>` still upserts arbitrary `SkillNode` JSON arrays.

---

## F.3b — frameworks & tailoring (complete)

- [x] `data/framework_refs_seed.json` + v13 merge; `jobdesk skills import-frameworks`.
- [x] `applications.posting_text`; `GET /api/applications/{id}/posting-hints` (read-only).
- [x] Applications UI: paste posting, analyze vs catalog + draft links.
- [x] Live preview: `POST …/posting-hints/preview` (unsaved posting + draft composition).
- [x] Read-only “suggested links”: hints flag **in posting, not on draft** (link manually in Portfolio).
- [x] External imports: any JSON file via `jobdesk skills import-frameworks` (see [`data/FRAMEWORK_REFS.md`](../data/FRAMEWORK_REFS.md)).

---

## F.2b — discovery (reference)

- Typeahead + filter lists; suggest tools from **technologies** / **highlights**.
- Unmatched → create **`tag.*`** (standards/niche); match known tools in catalog first.
- Tool selection → competency suggestions.

---

## Operations

- **DB:** v13 adds `posting_text` + framework ref seed merge; v12 catalog upsert; v11 dropped legacy skills elements. No wipe required.
- **Import:** `jobdesk import` seeds header/objective/education/experience/projects from `content/` — not `skills.yaml`.
- **Build config:** `settings/jobdesk.yaml` lists **section templates only**; content always from DB resolve.
- **Serve:** `cargo run -p jobdesk -- serve` (DB typically `workspace/jobdesk.db`).
- **Reset evidence links:** `jobdesk skills clear-links --yes` (removes all asserted portfolio tags).

---

## Merge F → master (exit criteria)

1. Portfolio evidence model stable: three tag types, relationships pane, discovery.
2. PDF path is **toolbelt-only** for tag projection (classic theme).
3. Bundled catalog usable without manual seeding; import path documented.
4. F.3b optional before merge if framework hints are required for your workflow.
