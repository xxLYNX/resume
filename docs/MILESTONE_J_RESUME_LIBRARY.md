# Milestone J — Resume library, editor & UX fixes

**Branch:** `dev/milestone-j` (from `master`).

**Priority:** Next active milestone — **before Milestone F (skills)**. F stays specified; do not start F until J is merged unless explicitly parallelizing.

---

## Why J now

Milestone E shipped **composition picker** and **revision editing**, but not **creating catalog elements** in the UI. Most day-to-day flow lives on **Applications**, while the **Catalog** tab only edits existing elements seeded via `import`. That gap blocks adding jobs, projects, etc. without YAML round-trips.

Separately, several **Applications / preview** bugs and **phase vs pipeline** inconsistencies need a focused pass. Preview fixes for “Last submit” landed on `master`; remaining items belong here.

---

## Product model (library vs application)

Canonical split (document in ARCHITECTURE + CHECKLIST; UI should match):

| Surface | Owns | Does not own |
|---------|------|----------------|
| **Resume library** (today: Catalog tab) | Global **elements** + **revisions** (facts: jobs, degrees, skills blob, …) | Per-posting composition, pipeline status |
| **Applications** | Posting metadata, **draft composition** (pins: element id + **revision id**), Submit → manifest | Creating catalog rows (except shortcuts that delegate to library) |
| **Presets** | Saved compositions without a posting row | Element payloads |
| **Submit** | Immutable manifest + theme snapshot | — |

**Composition pins revisions**, not “latest.” Editing library text creates a new revision; open applications keep old pins until the user changes revision in the picker or resets to catalog default.

**Preview sources**

- Library tab / no app selected: `default_composition` (all elements, **latest** revision each) — reference résumé.
- Application draft: pinned composition (live DOM when saved app open).
- Last submit: canonical manifest only when a submission exists.

Known gap (fix in J): **New application** form does not send unsaved `draft_composition` to preview — picker changes do not affect PDF until after Save.

---

## Scope

### 1. Bug fixes & polish (do first)

- [x] Preview **Last submit** when no submission — friendly empty state (server + UI), not build failure (`master`).
- [x] Preview mode dropdown — both options visible; validate instead of hiding disabled options (`master`).
- [x] **New application** — preview uses composition from picker before first Save.
- [ ] After library revision save, optional refresh of revision lists in open application picker (no auto-bump of pins).

### 2. Phase & pipeline (revisit — not blocking library work)

The two-layer model is defined in [`APPLICATION_LIFECYCLE.md`](APPLICATION_LIFECYCLE.md), but UI/Save/API behavior has **known inconsistencies** (e.g. phase/submitted workflow vs whether a manifest exists, filters vs detail state). **Defer deep redesign** until after library CRUD; track under [Revisit](#phase--pipeline-revisit).

- [ ] Audit: document concrete repro cases in this file or a short addendum.
- [ ] Align UI rules with lifecycle doc (stricter Save validation, clearer labels).
- [ ] Optional: disable pipeline labels until first Submit, or auto-set phase from submission count.

### 3. Library CRUD & editor UX

- [x] Store + API: `create_element(kind, id, initial_payload)` (element row + revision 1, validated).
- [x] `delete_element` when unused (not referenced by any application draft or preset composition).
- [ ] CLI: `jobdesk catalog add <kind> <id>` (or similar) for scripting.
- [x] UI: **New entry** inline form (like New application): resume fields + optional element id (auto-suggested from company/project name; not shown on PDF).
- [ ] UI copy: rename or subtitle Catalog → **Resume library** (tab label TBD; avoid duplicating composition editing here).
- [ ] Optional shortcut: after create, “Include in current application” when app detail is open.

### 4. Documentation

- [x] This spec + `NEXT_MILESTONE.md` + ARCHITECTURE milestone J.
- [ ] `JOBDESK.md` — catalog add command when CLI exists.
- [ ] CHECKLIST “Where to edit data” — create entries in library, not only import.

---

## Out of scope (J)

- Milestone **F** skills table / framework refs.
- New collection kinds (tools, certifications, awards).
- Submission history UI (list all manifests per app).
- Ledger PDF changes (Milestone I complete).

---

## Phase & pipeline revisit

**Status:** Not critical for library work; scheduled in J so it is not lost.

Issues observed in the wild (expand as found):

- Phase **submitted** or pipeline labels set **without** using **Submit** → “Last submit” empty but filters suggest otherwise.
- Phase/pipeline controls vs `submissions.length` can confuse users.
- Save may allow combinations that the lifecycle doc discourages.

**Intent later:** Single coherent story — **Submit** is the only action that creates a manifest and moves phase to submitted; pipeline labels are for tracking after that. UI should reflect submission count, not only phase field.

---

## Completion checklist

- [ ] Bug-fix tranche merged (new-app preview minimum).
- [ ] `create_element` + library “New entry” UI.
- [ ] Product model section reflected in ARCHITECTURE / CHECKLIST.
- [ ] Phase/pipeline audit notes captured; at least one UX guard or label fix OR explicit “deferred to J.1” with open issues listed.

---

## Relationship to other milestones

| Milestone | Relation |
|-----------|----------|
| **E** | Picker + revision editor — J adds **create** and clarifies library role. |
| **F** | Starts after J merge; skills migration may reuse library patterns. |
| **I** | Independent; complete on `master`. |

```bash
git checkout master
git pull
git checkout -b dev/milestone-j
```
