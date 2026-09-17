# Next milestone (prep) — composition, themes, catalog UX

Target work after **D** (tracker polish) and **phase/workflow** split. Use a **dev branch** for UI experiments; merge to `master` when stable.

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

### Data model: skills table + presets

**Skills table:** move from single `skills.main` blob to rows `{ id, label, aliases[], tags[] }` so compositions filter/tag skills and aliases match job keywords without duplicate strings.

**Presets:** saved **compositions** (section pins + theme id) **without** a posting row — like old `config.yaml` “profiles.” Clone preset → new application or quickfill; overlaps similarity but named and user-curated.

---

## Also on the list (E/F + polish)

See [`ARCHITECTURE.md`](../ARCHITECTURE.md) Milestones E–F: portfolio flags, static export, UniFFI optional; submission history UI; PDF cache on submit; PDF.js / SSE; CLI `watch`; Go README cleanup.

---

## Branch workflow

```bash
git checkout -b dev/ui-composition   # example
# … UI work …
# merge when happy
```
