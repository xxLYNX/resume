# Next milestone — F (skills & framework refs)

**Milestone E is complete** on `dev/ui-composition` (merge to `master`, then branch for F). E shipped composition/themes/catalog UX; see [Milestone E (complete)](#milestone-e-complete) below.

**Active work:** [`MILESTONE_F_SKILLS.md`](MILESTONE_F_SKILLS.md) — skills table, aliases, tags, `framework_refs` (O\*NET, NICE, ESCO, …).

**Milestone I is complete** on `master` — application metadata + ledger PDF export ([`MILESTONE_I_APPLICATION_LEDGER.md`](MILESTONE_I_APPLICATION_LEDGER.md)).

```bash
git checkout master
git checkout -b dev/milestone-f   # if not already on F branch
```

---

## Milestone F — start here

- [ ] Skills table + aliases + tags
- [ ] `framework_refs` + optional import enrichment
- [ ] Composition/resolver uses skill ids + per-job alias choice

Not F (later): separate **tools**, **certifications**, **awards** catalog collections (same content-picker pattern as E when theme + kinds exist).

---

## Milestone E (complete)

Delivered on branch `dev/ui-composition`:

| Area | Outcome |
|------|---------|
| **Content picker** | Section on/off + per-element include + revision; live draft preview; render skips omitted sections |
| **Themes** | `theme.toml`, contract, macro loader, snapshot + `theme_files_hash` on submit |
| **Catalog UX** | Structured fields from theme metadata; JSON toggle |
| **Presets** | Saved compositions; clone to application / quickfill |
| **Applications UI** | List/detail flow, unified Save, preview cache (no redundant PDF work) |
| **Product** | jobdesk-only; legacy Go builder removed |

**Deferred (post-E, not blocking F):**

- **Submission history UI** — list past manifests per application; preview any send (today: count + latest canonical + **Last submit** preview mode).
- **Phase/pipeline UI polish** — optional stricter rules before first manifest ([`APPLICATION_LIFECYCLE.md`](APPLICATION_LIFECYCLE.md) already defines behavior).

**Dropped:** file watch on repo `content/` (canonical data is SQLite; `import` / `export` / Catalog UI).

**After E (backlog, any milestone):** PDF cache on submit; PDF.js / SSE; portfolio → **G**; UniFFI → **H**.
