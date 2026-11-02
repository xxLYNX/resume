# Milestone F — execution roadmap

**Status:** merged to `master` (schema **v15**). Historical branch was `dev/milestone-f`.

**Policy:** **Single build path:** SQLite catalog → resolve → LaTeX/PDF (no parallel file-based content build).

**Legacy re-link checklist (human):** [`LEGACY_SKILLS_INVENTORY.md`](LEGACY_SKILLS_INVENTORY.md) — reads `content/skills.yaml` + element fields; not source of truth.

---

## North star (portfolio → PDF)

| Idea | Rule |
|------|------|
| **Portfolio** | Experience, project, education, course, certification, award entries are **anchors**. |
| **Evidence** | Each anchor links **tools**, **competencies**, and **other tags** via `element_skill_links` (three separate pickers in UI). |
| **Catalog** | `skill_nodes`: `tool` \| `competency` \| `tag` (`tag` = standards / niche labels — not tools or competencies). |
| **Graph** | `tool_competency_edges` + suggestions API (tools only → competencies). |
| **PDF** | **Projection**, not a second content model. **`toolbelt`**: theme **title** + deduped labels from all pinned evidence. **Project lines**: optional **`resume_tags`** from `CompositionItem.resume_tag_skill_ids` (subset of asserted links; chosen on **Resume** tab). |
| **Deprecated** | Payload `technologies` on projects; multi-bucket `skills.{languages, securityTools, …}`; composable `skills` / `skills.main`. O*NET fills catalog/suggestions only—not project subtitles. |

ATS/keywords: prefer narratives + linked tags on anchors, not a separate keyword blob.

### UI tabs (serve)

| Tab | Role |
|-----|------|
| **Portfolio** | Entry CRUD, evidence tags, skill relationship search; usage + link to Resume for project subheading tags. |
| **Resume** | Composition target (catalog default / preset / application), section picker, project PDF line tags, live PDF preview. |
| **Applications** | Posting tracker, hints, Save/Submit (resume draft read from Resume editor). |
| **Presets** | Named composition snapshots. |

**Manual check:** add evidence on a project → Resume tab → select application → toggle PDF line tags → preview shows `$|$ \emph{…}`; toolbelt still lists all pinned evidence.

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
| **F.4** | Done | O*NET catalog import, server search, scale-ready UI |

---

## F.4 — O*NET catalog (Phase 2)

- [x] `crates/onet_import`: Technology/Software Skills → tools; Skills.txt → competencies; SOC co-occurrence edges.
- [x] `jobdesk skills import-catalog --onet-dir … --replace --yes` (wipes catalog + evidence links when replacing).
- [x] `GET /api/skills/search`, `GET /api/skills/stats`, batch `GET /api/skills/nodes?ids=…`.
- [x] UI: search-driven evidence pickers (no full-catalog load).
- [x] DB **v14**: no auto-seed on empty catalog (`seed_skill_nodes_if_empty` no-op); v12 still upserts demo once on migrate for dev/tests.
- [x] Docs: [`data/ONET_IMPORT.md`](../data/ONET_IMPORT.md).

**Rollout on an existing workspace:**

```bash
cargo run -p jobdesk -- skills import-catalog --onet-dir /path/to/onet_text --replace --yes
```

Then hard-refresh the browser and re-apply evidence tags on portfolio entries.

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

- **DB:** v15 strips legacy `technologies` from project revision payloads; v14 O*NET empty-catalog policy; v13 posting/framework refs; v12 catalog upsert.
- **Import:** `jobdesk import` seeds header/objective/education/experience/projects from `content/` — not `skills.yaml`.
- **Build config:** `settings/jobdesk.yaml` lists **section templates only**; content always from DB resolve.
- **Serve:** `cargo run -p jobdesk -- serve` (DB typically `workspace/jobdesk.db`).
- **Reset evidence links:** `jobdesk skills clear-links --yes` (removes all asserted portfolio tags).
- **O*NET catalog:** `jobdesk skills import-catalog --onet-dir … --replace --yes` (see [`data/ONET_IMPORT.md`](../data/ONET_IMPORT.md)).

---

## Merge F → master (exit criteria) — done

1. Portfolio evidence model stable: three tag types, relationships pane, discovery.
2. PDF path is **toolbelt-only** for tag projection (classic + ats-classic themes).
3. Bundled catalog usable without manual seeding; import path documented.
4. F.3b framework hints shipped before merge.

---

## Maybe later — NICE KSA enrichment (not in F v1)

**NIST NICE** (Cybersecurity Workforce Framework, e.g. SP 800-181) supplies **work roles** and **KSAs** that complement **O\*NET** (general tools + competencies). Jobdesk today:

- **O\*NET:** bulk `tool` / `competency` import (`jobdesk skills import-catalog`).
- **Standards-style markers:** curated `tag.*` nodes + optional **`framework_refs`** ([`data/FRAMEWORK_REFS.md`](../data/FRAMEWORK_REFS.md), `jobdesk skills import-frameworks`).

**Possible follow-ups** (pick one depth; no commitment):

| Depth | Idea | Effort |
|-------|------|--------|
| **Light** | Curate dozens of NICE KSAs; add as `competency.*` or merge `framework_refs.nice` onto existing nodes via JSON import | Mostly data |
| **Medium** | `jobdesk skills import-nice --merge` (parse NIST publish bundle, stable ids, refs populated; do **not** replace O\*NET catalog) | Small crate + docs, dedup/overlap policy |
| **Heavy** | Application work-role picker, NICE↔tool graph, hierarchical UI | Product milestone |

Posting hints already scan `framework_refs` strings; enriching refs or adding NICE-labeled competencies improves cyber JD matching without changing the PDF projection model.
