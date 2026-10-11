# Next milestone — F (skills catalog, provenance & framework refs)

**Active work:** [`MILESTONE_F_ROLLOUT.md`](MILESTONE_F_ROLLOUT.md) on branch **`dev/milestone-f`** (breaking changes OK; DB reset acceptable).

**Complete on `master`:** E, I, **J** (Portfolio, dates, themes, experience location, …).

```bash
git checkout master && git pull
git checkout dev/milestone-f
# if branch was stale:
git reset --hard master
```

---

## Milestone F — phased plan (v10)

| Phase | Deliverable |
|-------|-------------|
| **F.0** | Plan + branch aligned to `master` |
| **F.2a** | Schema v10 (tool/competency, element links, evidence-only PDF); portfolio grouping v1 |
| **F.2-cleanup** | Modular static UI; retire `skills.main`; schema v11 (smoke UI, then F.2b) |
| **F.2b** | Skill discovery (typeahead, payload scan, create tools) |
| **F.2c** | Relationships pane |
| **F.2d** | **Next:** cert/award/course on PDF/composition; portfolio UX polish |
| **F.3** | Framework import; tailoring hints |

Full checklists: [`MILESTONE_F_ROLLOUT.md`](MILESTONE_F_ROLLOUT.md).

---

## Milestone J (complete)

Merged via PR #1. Spec: [`MILESTONE_J_RESUME_LIBRARY.md`](MILESTONE_J_RESUME_LIBRARY.md).

---

## Milestone E (complete)

Content picker, themes, presets, applications UX — see [`ARCHITECTURE.md`](../ARCHITECTURE.md) Milestone E.

**Backlog:** submission history UI; phase/pipeline J.1; PDF cache; portfolio **G**; UniFFI **H**.
