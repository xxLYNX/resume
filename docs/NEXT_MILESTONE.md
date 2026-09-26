# Milestone E — composition, themes, catalog UX

**In progress on branch `dev/ui-composition`.** **F** is reserved for [skills + framework refs](MILESTONE_F_SKILLS.md). Merge to `master` when stable.

## Priority themes

### Compositional editing (large)

Today: **draft composition** is JSON (`element_id` + `revision_id` per section). Goal: visual editor — pick sections, reorder, choose element + revision per row, without raw JSON. Same model under the hood; better for day-to-day use and for **presets**.

### Themes & manifest pinning

- Ship **theme bundles** as first-class (`theme.toml` + preamble + section templates).
- **`theme_bundles`** table or manifest registry: `id`, `version`, **files hash**.
- On **Submit**, set `theme_files_hash` on manifest v1 so “view as submitted” uses the same template bundle, not whatever changed on disk since apply.

### Application **Save** UX (pending)

- **Done (minor):** tracker button label **Save** (was “Save details”).
- **Still to design:** whether to combine tracker **Save** + **Save draft composition**, clarify **Submit** vs setting **phase/pipeline**, disable misleading “submitted pipeline” before first manifest. Track on UI dev branch.

---

## Explained backlog items

### Catalog UI: structured forms + `theme.toml`

**Structured forms:** `themes/classic/theme.toml` already lists sections and `optional_fields` (e.g. education `highlights`, `gpa`). The catalog pane should render **typed fields** per element kind from that metadata instead of one JSON blob — only show columns the theme can render (`{% if field %}`).

**File watch:** repo `content/` remains **import seed**. Optional watcher: external edit to YAML → notify UI or suggest `import --force`. Canonical data stays SQLite; no auto-overwrite of `content/`.

### Presets (E)

Saved **compositions** (section pins + theme id) **without** a posting row — like old `config.yaml` “profiles.” Clone preset → new application or quickfill.

**Skills table + NICE/ESCO:** **Milestone F**, not E — see [`MILESTONE_F_SKILLS.md`](MILESTONE_F_SKILLS.md).

---

## Also on the list (polish)

Submission history UI; PDF cache on submit; PDF.js / SSE; CLI `watch`; Go README cleanup. Portfolio → **G**; UniFFI → **H** in [`ARCHITECTURE.md`](../ARCHITECTURE.md).

---

## Branch workflow

```bash
git checkout -b dev/ui-composition   # example
# … UI work …
# merge when happy
```
