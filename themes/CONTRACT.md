# Theme contract (jobdesk)

Third-party themes target **jobdesk** only. Section templates are MiniJinja; content comes from resolved catalog/composition JSON.

## Mental model

- **Catalog:** each **element** is one record (struct-like JSON). List collections (experience, projects, …) are many elements; singleton sections (header, objective) are usually one element each.
- **Composition:** per application, which **sections** appear and which **elements + revisions** are included. Sparse fields on an element are normal.
- **Render:** jobdesk resolves composition → **section contexts** (JSON) → MiniJinja section templates → one `.tex` → PDF.

Theme authors work with **section contexts** and **macros** shipped in the theme bundle. They do not implement composition logic or SQLite.

## What jobdesk guarantees

1. **Omitted section** — If a section is not in the resolved payload for this build, jobdesk **does not run** that section’s template (no empty `\section{}` unless the template is invoked with an empty list by design).
2. **Normalized list fields** — For documented row types, missing optional **arrays** (e.g. `details`, `highlights`) are treated as `[]` before render so `{%- for %}` loops are safe.
3. **Stable section keys** — Resolver keys match theme section names (`projects`, `experience`, …). List sections receive `{ "<key>": [ row, … ] }`; singleton sections receive the row object at the root (header, objective) or documented nesting (`skills`).
4. **Macro registry** — `theme.toml` `[macros]` maps logical names → files. jobdesk loads macros into the MiniJinja environment **before** section templates. Sections use `{% from "macro_name" import … %}`.

## What themes own

1. **Presentation** — LaTeX layout, `\section` titles, spacing, which fields appear.
2. **Optional scalars** — Use theme **macros** (or filters we add later) for patterns like “title | technologies emphasis”. Authors should not rely on empty strings looking correct.
3. **`optional_fields` in `theme.toml`** — Documents which element fields the catalog UI may omit; aligns with macros, not a substitute for them.
4. **`month_display` in `theme.toml`** — `abbrev` (e.g. `Dec.`) or `numeric` (e.g. `12`). jobdesk owns month tokens; the theme picks the style. Catalog date previews and the `format_date_range` filter honor this.
5. **Experience `location`** — optional `{ city, state }` on experience rows; templates render company + location via `format_experience_company_line`.

## Expandable “toys” for theme authors

jobdesk may add **versioned** macro libraries and MiniJinja **filters** (e.g. join lines, format date ranges) declared in `theme.toml`. Themes pin a `contract_version` and import only what they need. New toys are additive; existing themes keep working.

## Success criterion for classic

The reference **classic** theme must reproduce the historical resume appearance on the golden fixture when **composition-selected `resume_tags`** are present on project rows (from evidence links, not payload `technologies`), while supporting omitted sections (content picker) and empty resume-line tags without broken LaTeX.

Project list rows may include **`resume_tags`** (comma-separated string, resolve-injected from `CompositionItem.resume_tag_skill_ids`).

## Bundle layout

```
themes/<id>/
  theme.toml          # id, sections, optional_fields, [macros]
  preamble.tex
  macros/*.jinja.tex  # shared building blocks
  sections/*.jinja.tex
```

Build config (`settings/jobdesk.yaml`) currently lists section template paths; long term this merges into the theme bundle only.
