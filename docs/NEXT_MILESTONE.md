# Milestone E — composition, themes, catalog UX

**Finish on branch `dev/ui-composition` before merge or Milestone F.** External skill **frameworks** (O\*NET, NICE, ESCO, …) are **F only** — see [`MILESTONE_F_SKILLS.md`](MILESTONE_F_SKILLS.md).

## Priority themes

### Application content picker (core E UX — not done)

When you apply to a **specific posting**, you choose **what content** that resume contains. That decision is **per application**. It may match a previous application exactly (same elements and revisions); **theme** is independent and can change without redoing content choices.

Two levels of choice:

1. **Section / collection included or omitted** — top-level blocks you want on the compiled resume for this application (e.g. header, objective, education, experience, projects, skills; target model also **tools**, **certifications**, **awards** as separate collections when catalog + theme support them).
2. **Which elements inside each collection** — the catalog holds many elements per collection (e.g. 10 projects). For this application you **include only the elements you consider relevant**; excluded elements stay in the catalog but **do not appear in preview or submit output**. Inclusion is at **element** level (whole job, whole project, whole education row — fields and bullets live on that element’s pinned **revision**), not ad-hoc bullet picking across elements.

The UI goal: for each enabled section, show the **full catalog list** for that collection with **include toggles** (and revision picker where wording matters), not only “add a row from a dropdown.”

**Data model (unchanged):** `draft_composition` → sections with `items: [{ element_id, revision_id }, …]`; omitted sections or empty `items` mean that block is not rendered. Submit freezes the same shape in the manifest.

**Today vs target collections**

| Collection (target) | In catalog / classic theme today |
|---------------------|----------------------------------|
| header, objective, education, experience, projects | Yes (`ElementKind` + import) |
| skills | Yes (often one element `skills.main`) |
| tools, certifications, awards | **Not yet** — may be new kinds/sections + theme; picker UX should extend the same pattern when they exist |

**Done (v1, interim):** row-based composition editor (add/remove/reorder, element + revision selects; JSON in details). **Replace/augment with the content picker** above.

**Not this feature:** O\*NET / NICE / ESCO keyword frameworks (**Milestone F**). **Creating** new catalog elements (new job row) remains import seed and/or catalog forms — separate from include toggles.

### Compositional editing (implementation note)

Presets and quickfill copy a saved **content set** (composition), not a theme. Visual editor work continues under **Application content picker**.

### Themes & manifest pinning

- Ship **theme bundles** as first-class (`theme.toml` + preamble + section templates).
- **`theme_bundles`** table or manifest registry: `id`, `version`, **files hash**.
- On **Submit**, set `theme_files_hash` on manifest v1 so “view as submitted” uses the same template bundle, not whatever changed on disk since apply.

### Application **Save** UX

- **Done (minor):** tracker button label **Save**; help text for Save vs Save composition vs Submit.
- **Still E:** optional single-action save, stricter phase/pipeline rules before first manifest (if we keep separate buttons).

---

## Explained backlog items

### Catalog UI: structured forms + `theme.toml`

**Structured forms:** `themes/classic/theme.toml` already lists sections and `optional_fields` (e.g. education `highlights`, `gpa`). The catalog pane should render **typed fields** per element kind from that metadata instead of one JSON blob — only show columns the theme can render (`{% if field %}`).

**File watch:** repo `content/` remains **import seed**. Optional watcher: external edit to YAML → notify UI or suggest `import --force`. Canonical data stays SQLite; no auto-overwrite of `content/`.

### Presets (E)

Saved **compositions** (section pins + theme id) **without** a posting row — like old `config.yaml` “profiles.” Clone preset → new application or quickfill.

**Skills catalog + external frameworks:** **Milestone F** — canonical skill ids, aliases, tags, `framework_refs` (O\*NET, NICE, ESCO, …). Not E.

---

## E completion checklist (merge when all checked)

- [x] Theme snapshot + `theme_files_hash` on submit
- [x] Presets API + tab; clone preset → application
- [x] Structured catalog fields + JSON toggle
- [x] Visual composition editor (row-based)
- [x] **Application content picker** — section on/off + per-element include toggles (all theme/catalog collections today)
- [ ] **Submission history** UI (list manifests for app; open “as submitted”)
- [ ] **File watch** on repo `content/` → suggest `import` / `--force` (no auto-overwrite DB)
- [ ] Remaining Save/phase UX polish (if any after inclusion UI)
- [ ] Smoke-tested `serve` path end-to-end

**After E (master or small fixes):** PDF cache on submit; PDF.js / SSE; CLI `watch`; Go README cleanup. Portfolio → **G**; UniFFI → **H**.

---

## Branch workflow

```bash
git checkout -b dev/ui-composition   # example
# … UI work …
# merge when happy
```
