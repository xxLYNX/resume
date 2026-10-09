# Milestone J — Portfolio, editor & UX fixes

**Branch:** `dev/milestone-j` (from `master`).

**Priority:** Next active milestone — **before Milestone F (skills)**. F stays specified; do not start F until J is merged unless explicitly parallelizing.

---

## Why J now

Milestone E shipped **composition picker** and **revision editing**, but not **creating catalog elements** in the UI. Most day-to-day flow lives on **Applications**, while the **Portfolio** tab only edited existing elements seeded via `import`. That gap blocked adding jobs, projects, etc. without YAML round-trips.

Separately, several **Applications / preview** bugs and **phase vs pipeline** inconsistencies needed a focused pass. Preview fixes for “Last submit” landed on `master`; remaining items belong here.

---

## Product model (portfolio vs application)

Canonical split (document in ARCHITECTURE + CHECKLIST; UI should match):

| Surface | Owns | Does not own |
|---------|------|----------------|
| **Portfolio** tab | Global **elements** + **revisions** (facts: jobs, degrees, skills blob, …). Future: non-resume portfolio items. | Per-posting composition, pipeline status |
| **Applications** | Posting metadata, **draft composition** (pins: element id + **revision id**), Submit → manifest | Creating portfolio rows (except shortcuts that delegate to Portfolio) |
| **Presets** | Saved compositions without a posting row | Element payloads |
| **Submit** | Immutable manifest + theme snapshot | — |

**Composition pins revisions**, not “latest.” Editing portfolio text creates a new revision; open applications keep old pins until the user changes revision in the picker or resets to portfolio default.

**Preview sources**

- Portfolio tab: `default_composition` (all elements, **latest** revision each) — reference résumé.
- Application draft (Applications tab): pinned composition (live DOM when saved app open).
- Last submit: canonical manifest only when a submission exists.

---

## Scope

### 1. Bug fixes & polish (do first)

- [x] Preview **Last submit** when no submission — friendly empty state (server + UI), not build failure (`master`).
- [x] Preview mode dropdown — both options visible; validate instead of hiding disabled options (`master`).
- [x] **New application** — preview uses composition from picker before first Save.
- [x] After portfolio revision save, refresh revision lists in open application picker (no auto-bump of pins).

### 2. Phase & pipeline (revisit — not blocking portfolio work)

The two-layer model is defined in [`APPLICATION_LIFECYCLE.md`](APPLICATION_LIFECYCLE.md).

- [x] Audit: concrete cases documented below (phase/pipeline revisit section).
- [x] Align UI/rules: phase `submitted` only via **Submit** (Save rejects manual submitted phase before first manifest); UI disables submitted phase until Submit; help text on Applications tab.
- [ ] **Deferred to J.1:** auto-correct legacy rows with `phase=submitted` and zero submissions; filter/list UX vs submission count.

### 3. Portfolio CRUD & editor UX

- [x] Store + API: `create_element(kind, id, initial_payload)` (element row + revision 1, validated).
- [x] `delete_element` when unused (not referenced by any application draft or preset composition).
- [x] CLI: `jobdesk catalog add <kind> <id> --payload file.json`.
- [x] UI: **New entry** inline form (like New application): resume fields + optional element id (auto-suggested from company/project name; not shown on PDF).
- [x] UI copy: tab label **Portfolio** (API paths remain `/api/catalog/…`).
- [x] Optional shortcut: after create, “Include in open application draft” when an application is selected.

### 4. Documentation

- [x] This spec + `NEXT_MILESTONE.md` + ARCHITECTURE milestone J.
- [x] `JOBDESK.md` — `catalog add` command.
- [x] CHECKLIST “Where to edit data” — create entries in Portfolio, not only import.

---

## Out of scope (J)

- Milestone **F** skills table / framework refs.
- New collection kinds (tools, certifications, awards) — future Portfolio expansions beyond resume collections.
- Submission history UI (list all manifests per app).
- Ledger PDF changes (Milestone I complete).

---

## Phase & pipeline revisit

**Status:** Guardrails shipped in J; deeper redesign **J.1**.

Issues observed (repro / mitigation):

| Issue | Mitigation (J) |
|-------|----------------|
| Phase **submitted** set via Save without **Submit** → “Last submit” empty but filters suggest otherwise | Save rejects `phase=submitted` until `submissions.length > 0`; phase dropdown disables **submitted** until first Submit |
| Pipeline labels editable while `phase=draft` | Workflow select locked to `draft` while phase is draft (existing) |
| Legacy DB rows: `phase=submitted`, zero submissions | Detail meta warns; user should Save as draft or Submit — auto-migration deferred to J.1 |

**Intent (unchanged):** **Submit** creates the manifest and moves phase to submitted; pipeline labels track the job after that.

---

## Completion checklist

- [x] Bug-fix tranche (new-app preview).
- [x] `create_element` + Portfolio “New entry” UI.
- [x] Product model section reflected in ARCHITECTURE / CHECKLIST.
- [x] Phase/pipeline audit notes + UX guards (deeper work → J.1).

---

## Relationship to other milestones

| Milestone | Relation |
|-----------|----------|
| **E** | Picker + revision editor — J adds **create** and clarifies Portfolio role. |
| **F** | Starts after J merge; skills migration may reuse portfolio patterns. |
| **I** | Independent; complete on `master`. |

```bash
git checkout master
git pull
git checkout -b dev/milestone-j
```
