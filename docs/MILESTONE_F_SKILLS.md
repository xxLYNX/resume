# Milestone F — Skills catalog, provenance & framework refs

**Branch:** `dev/milestone-f` (from `master` after Milestone J merge).

**Isolation:** All F work lands on this branch only until merge. The branch may be discarded; keep phases small and mergeable so rollback is cheap.

**Depends on:** J complete on `master` (Portfolio CRUD, composition pins, structured element payloads).

---

## North star

Skills are not only a single `skills.main` blob. They are:

1. A **curated catalog** of canonical ids (tools like Ghidra, activities like reverse engineering, purposes like malware analysis).
2. **Evidence links** from specific **element revisions** (especially experience and project) via controlled **what / with / for** axes.
3. **Narrative “why”** stays in highlights/objective text — not tags.
4. An **aggregated skills view** (Portfolio + PDF) derived from linked evidence, with optional per-application alias choice when composing.
5. **Framework refs** (NICE, O\*NET, ESCO, …) on catalog nodes for external vocabulary and future JD matching — not automatic résumé claims.

**Tailoring:** Applications already pin elements + revisions. F adds “what this work demonstrates” so matching a posting is evidence-based, not a static keyword list.

---

## Design principles

| Principle | Rule |
|-----------|------|
| **Curated catalog** | Users pick from autocomplete; synonyms map to one id (`ghidra` → `tool.ghidra`). No user-created hierarchy edges in v1. |
| **Asserted vs inferred** | Store both; UI shows inferred ancestors faded; opt-out on inferred rows. Inference conservative; **with** (tools) safer than **what** (activities). |
| **Revision-scoped links** | Links attach to **revision_id** so composition pins stay meaningful when text changes. |
| **Separate application tags** | Job tracker tags ≠ skill ids. |
| **PDF vs Portfolio** | Classic theme may keep compact skill lines; Portfolio gets richer relationship views later. |

---

## Vocabulary axes

| Axis | Meaning | Examples | Inference |
|------|---------|----------|-----------|
| **with** | Tool, language, platform | Ghidra, Rust, Kubernetes | Narrow upward (Ghidra → disassembler category) — optional F.2 |
| **what** | Activity / competency demonstrated | Reverse engineering, code review | Shallow tree; inferred off by default or “suggested” — F.2+ |
| **for** | Purpose / domain (leaf-heavy) | Malware analysis, performance tuning | No upward inference |

Catalog row shape (evolves from early F):

```text
skill_nodes(
  id,              -- e.g. tool.ghidra, activity.reverse-engineering
  kind,            -- with | what | for
  label,
  parent_id,       -- nullable; F.2+
  aliases_json,
  buckets_json,    -- optional grouping for PDF (security, cloud, …)
  framework_refs_json
)
```

**Ghidra** is a **with** leaf (or mid-tree) node; framework refs might map it to external taxonomies without implying the user “knows” every parent skill.

---

## Evidence links (provenance)

```text
revision_skill_links(
  revision_id,
  skill_id,
  axis,            -- with | what | for (must match skill_nodes.kind)
  source           -- asserted | inferred
)
```

- **v1 (F.1):** `asserted` only; experience + project revisions in UI.
- **F.2:** materialize inferred links from `parent_id` walk; user can delete inferred rows per revision.

Do **not** embed large tag arrays inside YAML payloads as the source of truth — junction table + catalog enables queries and the relationships pane.

---

## Aggregation & composition

| Mechanism | Behavior |
|-----------|----------|
| **Portfolio skills summary** | Dedupe skill ids from all latest revisions (or from a dedicated “skills profile” preset) → grouped labels for display/export. |
| **Composition** | Unchanged: pin elements + revisions per section. Skills **section** can mode: `legacy_blob` \| `from_pinned_elements` \| `manual_ids` (phase in). |
| **Resolver** | Build render context: skill lines for theme from chosen mode + alias overrides on composition (posting-friendly strings). |
| **Application tailoring** | Future: filter/highlight elements whose linked skills intersect posting keywords or framework refs. |

---

## UI vision (phased)

| Phase | UI |
|-------|-----|
| **F.1** | Portfolio: skill catalog CRUD (list/edit node, aliases, framework refs JSON). Experience/project editor: what/with/for pickers on revision save. |
| **F.2** | **Relationships pane:** element ↔ skills (table); reverse lookup (“all work using Ghidra”); optional inferred rows faded. |
| **F.3** | Graph or faceted filters; framework lens (show NICE/O\*NET on nodes); application-side “overlap with posting” hints. |

Relationships pane reads the same tables as the resolver — no second source of truth.

---

## Framework refs

Stored on `skill_nodes.framework_refs_json` (see original sketch). **Import** bundled or user mapping files in F.1 optional slice.

- **Does:** map internal id ↔ external codes for search, display, future JD tools.
- **Does not:** auto-add skills to revisions or rewrite bullets from NICE text in F.

---

## Phased delivery

Work **only on `dev/milestone-f`**. Merge each phase to `master` when stable; later phases can be dropped if the branch is rolled back.

### F.0 — Plan & branch hygiene (this doc)

- [x] Reset `dev/milestone-f` to post-J `master`
- [x] Plan through north star (this file)
- [ ] Update `NEXT_MILESTONE.md` / `ARCHITECTURE.md` on branch (F active, phased checklist)

### F.1 — Catalog + asserted links (MVP)

**Goal:** Canonical skills exist; experience/project revisions assert with/what/for; PDF still works.

- [ ] Migration: `skill_nodes`, `revision_skill_links`
- [ ] Domain validation: kinds, ids, link axis matches node kind
- [ ] Store + API: CRUD skill nodes; GET/PUT links for a revision
- [ ] Import: seed starter taxonomy (security/tools subset incl. Ghidra, Rust, …) from JSON in repo
- [ ] Portfolio UI: skill admin view; pickers on experience/project forms
- [ ] Resolver: `from_pinned_elements` aggregates asserted skills → classic `skills` template shape (keep `skills.main` fallback)
- [ ] CLI: optional `jobdesk skills import` for taxonomy file
- [ ] Tests: domain + store link rules; resolver dedupe

**Exit:** User links Ghidra to a project revision; pinned composition includes that project; build shows Ghidra under appropriate PDF group.

### F.2 — Inference, aggregation polish, relationships pane

- [ ] `parent_id` + closure or recursive read; generate `inferred` links
- [ ] UI: toggle inferred; opt-out per link
- [ ] Relationships pane (table + reverse query)
- [ ] Portfolio “skills summary” view (aggregated, not only PDF)
- [ ] Composition: `manual_ids` + alias override per application

### F.3 — Framework enrichment & tailoring aids

- [ ] Import enrichment files into `framework_refs`
- [ ] UI: framework lens on catalog nodes
- [ ] Application hints: keyword overlap with linked skills (read-only v1)

### Explicitly not F

- Separate **tools / certifications / awards** collections (future Portfolio expansions)
- Full ontology editor for users
- LLM JD parsing
- Public portfolio site (G)

---

## Migration from today

1. Keep `skills.main` element until F.1 resolver path is default.
2. Optional script: parse existing blob lines → suggest `skill_nodes` + links (manual confirm).
3. No breaking change to composition JSON until `skills section mode` is opt-in.

---

## Rollback strategy

- F lives on `dev/milestone-f` until a phase merges.
- F.1 migrations should be **additive** (new tables only); dropping branch before merge leaves `master` untouched.
- After merge, rollback = revert merge commit on `master` (new tables unused if code reverted).

---

## Relationship to other milestones

| Milestone | Relation |
|-----------|----------|
| **J** | Portfolio CRUD + revision pins — required foundation |
| **G** | Public portfolio can reuse relationships pane + aggregation |
| **J.1** | Application phase/pipeline — independent |

```bash
git checkout master && git pull
git checkout dev/milestone-f   # reset to master if stale
git reset --hard master        # only when (re)starting F
```
