# Next milestone — J (Portfolio, editor & UX fixes)

**Active work:** [`MILESTONE_J_RESUME_LIBRARY.md`](MILESTONE_J_RESUME_LIBRARY.md) on branch **`dev/milestone-j`**.

**Then:** Milestone **F** (skills & framework refs) — [`MILESTONE_F_SKILLS.md`](MILESTONE_F_SKILLS.md). Do not treat F as active until J is merged to `master` (unless you explicitly parallelize).

**Complete on `master`:** E (composition/themes/catalog UX), I (application metadata + ledger PDF).

```bash
git checkout master
git pull
git checkout -b dev/milestone-j
```

---

## Milestone J — start here

**Order of work**

1. Remaining **Applications / preview** fixes (new-application preview from picker).
2. **Portfolio CRUD** — create elements in UI/API (experience, education, project, …).
3. **Docs & naming** — Portfolio vs application model in UI copy.
4. **Phase & pipeline revisit** — audit and tighten UI/rules ([`APPLICATION_LIFECYCLE.md`](APPLICATION_LIFECYCLE.md)); not blocking steps 1–3.

See checklist in [`MILESTONE_J_RESUME_LIBRARY.md`](MILESTONE_J_RESUME_LIBRARY.md).

---

## Milestone F — queued (after J)

- [ ] Skills table + aliases + tags
- [ ] `framework_refs` + optional import enrichment
- [ ] Composition/resolver uses skill ids + per-job alias choice

Not F (later): separate **tools**, **certifications**, **awards** catalog collections.

Branch when starting F: `dev/milestone-f` (rebase or recreate from `master` after J merge).

---

## Milestone E (complete)

Delivered and merged to `master` (formerly `dev/ui-composition`):

| Area | Outcome |
|------|---------|
| **Content picker** | Section on/off + per-element include + revision; live draft preview; render skips omitted sections |
| **Themes** | `theme.toml`, contract, macro loader, snapshot + `theme_files_hash` on submit |
| **Catalog UX** | Structured fields from theme metadata; JSON toggle |
| **Presets** | Saved compositions; clone to application / quickfill |
| **Applications UI** | List/detail flow, unified Save, preview cache (no redundant PDF work) |
| **Product** | jobdesk-only; legacy Go builder removed |

**Deferred (picked up in J or later):**

- **Submission history UI** — list past manifests per application; preview any send.
- **Phase/pipeline UX** — revisit in Milestone J ([lifecycle doc](APPLICATION_LIFECYCLE.md) vs UI behavior).

**Dropped:** file watch on repo `content/` (canonical data is SQLite; `import` / `export` / Catalog UI).

**Backlog (any milestone):** PDF cache on submit; PDF.js / SSE; portfolio → **G**; UniFFI → **H**.
