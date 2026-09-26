# Milestone E — composition, themes, catalog UX

**Finish on branch `dev/ui-composition` before merge or Milestone F.** Skill **frameworks** (O\*NET, NICE, ESCO, …) are **F only** — [`MILESTONE_F_SKILLS.md`](MILESTONE_F_SKILLS.md).

## Application content picker (done)

Per posting, choose **what content** the resume contains (independent of **theme**):

1. **Section on/off** — whole blocks included or omitted.
2. **Per-element include** — catalog lists all elements in a collection; toggles + revision picker for included rows.

**Model:** `draft_composition` → `{ sections: [{ section_key, items: [{ element_id, revision_id }] }] }`. Omitted sections are skipped at render. **Submit** freezes the same shape in the manifest.

**Collections today:** header, objective, education, experience, projects, skills.

**Later (not blocking E):** separate catalog sections for **tools**, **certifications**, and **awards** — same picker pattern when kinds + theme exist.

Creating new catalog elements stays in the **Catalog** tab / `import`, not in the picker.

## Themes & manifest pinning (done)

- Theme bundles: `theme.toml` + preamble + sections + optional macros.
- **Submit** sets `theme_files_hash` and copies a snapshot under `workspace/artifacts/theme_snapshots/` for “view as submitted.”

## Presets (done)

Saved compositions without a posting row; clone to a new application or quickfill.

## Save UX (open)

Three actions today: **Save** (tracker fields), **Save composition** (draft pins), **Submit** (manifest). Help text exists; optional merge/stricter phase rules TBD — discuss before changing.

## E completion checklist

- [x] Theme snapshot + `theme_files_hash` on submit
- [x] Presets API + tab
- [x] Structured catalog fields + JSON toggle
- [x] Application content picker (section + element toggles, live draft preview)
- [ ] **Submission history** UI — list manifests per app; open any past send (today: latest canonical only in preview)
- [ ] Remaining **Save / phase** polish (if any)
- [ ] Smoke-tested `serve` end-to-end

**Dropped for jobdesk-only workflow:** file watch on repo `content/`. Canonical data is the DB; use **Catalog** / **`import`** / **`export`** instead of watching seed YAML.

**After E:** PDF cache on submit; PDF.js / SSE; optional CLI conveniences. Portfolio → **G**; UniFFI → **H**.
