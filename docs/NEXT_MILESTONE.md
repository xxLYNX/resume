# Next milestone — F (skills catalog, provenance & framework refs)

**Active work:** [`MILESTONE_F_SKILLS.md`](MILESTONE_F_SKILLS.md) on branch **`dev/milestone-f`** only (may be rolled back; do not land on `master` until reviewed).

**Complete on `master`:** E, I, **J** (Portfolio, dates, themes, experience location, …).

```bash
git checkout master && git pull
git checkout dev/milestone-f
# if branch was stale:
git reset --hard master
```

---

## Milestone F — phased plan

See full north star, data model, and checklists in [`MILESTONE_F_SKILLS.md`](MILESTONE_F_SKILLS.md).

| Phase | Deliverable |
|-------|-------------|
| **F.0** | Plan + branch aligned to `master` |
| **F.1** | `skill_nodes` + `revision_skill_links` (asserted); UI pickers; resolver aggregation; PDF fallback |
| **F.2** | Hierarchy/inference; relationships pane; skills summary view |
| **F.3** | Framework import; tailoring hints |

**F.1 MVP checklist (start here after F.0):**

- [ ] Migrations + domain validation
- [ ] API + Portfolio skill admin + experience/project pickers
- [ ] Seed taxonomy (tools e.g. Ghidra)
- [ ] Resolver: skills from pinned element revisions
- [ ] Tests

---

## Milestone J (complete)

Merged via PR #1. Spec: [`MILESTONE_J_RESUME_LIBRARY.md`](MILESTONE_J_RESUME_LIBRARY.md).

---

## Milestone E (complete)

Summary in [`NEXT_MILESTONE.md`](NEXT_MILESTONE.md) history on `master` — content picker, themes, presets, applications UX.

**Backlog:** submission history UI; phase/pipeline J.1; PDF cache; portfolio **G**; UniFFI **H**.
